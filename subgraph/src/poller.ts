/**
 * Cursor-based Soroban RPC getEvents poller (Stellar-only direct indexer).
 * Not SubQuery / The Graph — persists via db.ts (Postgres or memory).
 */
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { rpc } from "@stellar/stellar-sdk";
import {
  getState,
  setState,
  upsert,
  tryPollLock,
  releasePollLock,
} from "./db.js";
import { decodeEvent, type RawEvent } from "./decode.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

type Deployments = {
  attestation_registry: { contract_id: string };
  trade_credit_pool: { contract_id: string };
  track_b?: Record<string, string>;
};

function loadDep(): Deployments {
  const custom = (process.env.DEPLOYMENTS_PATH || "").trim();
  const path = custom
    ? custom
    : resolve(__dirname, "../../deployments/testnet.json");
  // Docker compose mounts repo deployments at /deployments
  const fallback = "/deployments/testnet.json";
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return JSON.parse(readFileSync(fallback, "utf8"));
  }
}

function contractIds(dep: Deployments): string[] {
  const ids = [
    dep.trade_credit_pool?.contract_id,
    dep.attestation_registry?.contract_id,
  ];
  // Optional Track B later via INDEX_TRACK_B=1
  if (process.env.INDEX_TRACK_B === "1" && dep.track_b) {
    ids.push(...Object.values(dep.track_b));
  }
  return ids.filter((c): c is string => Boolean(c) && !c.startsWith("C..."));
}

export type PollResult = {
  cursor?: string;
  ingested: number;
  skipped: number;
  lockedOut?: boolean;
};

let polling = false;

export async function pollOnce(): Promise<PollResult> {
  if (polling) return { ingested: 0, skipped: 0, lockedOut: true };
  polling = true;

  const locked = await tryPollLock();
  if (!locked) {
    polling = false;
    return { ingested: 0, skipped: 0, lockedOut: true };
  }

  try {
    const dep = loadDep();
    const rpcUrl =
      process.env.STELLAR_RPC_URL ?? "https://soroban-testnet.stellar.org";
    const server = new rpc.Server(rpcUrl, { allowHttp: true });
    const contracts = contractIds(dep);

    if (contracts.length === 0) {
      console.warn(
        "No real contract IDs in deployments/testnet.json — skip poll",
      );
      return { ingested: 0, skipped: 0 };
    }

    let cursor = await getState("events_cursor");
    const startLedgerEnv = Number(process.env.START_LEDGER ?? 0) || undefined;
    // Lookback on cold start — keep inside provider retention (QuikNode/SDF ~few days).
    const lookback = Number(process.env.START_LEDGER_LOOKBACK ?? 10_000);

    const filters = contracts.map((contractId) => ({
      type: "contract" as const,
      contractIds: [contractId],
    }));
    const limit = Number(process.env.EVENT_LIMIT ?? 100);

    async function fetchPage(startLedger?: number, pageCursor?: string) {
      const req: Record<string, unknown> = { filters, limit };
      if (pageCursor) req.cursor = pageCursor;
      else if (startLedger) req.startLedger = startLedger;
      return server.getEvents(req as never);
    }

    let startLedger: number | undefined;
    if (!cursor) {
      startLedger = startLedgerEnv;
      if (!startLedger) {
        const latest = await server.getLatestLedger();
        const latestNum =
          typeof latest === "number"
            ? latest
            : Number(
                (latest as { sequence?: number }).sequence ??
                  (latest as { ledger?: number }).ledger ??
                  0,
              );
        startLedger = Math.max(1, latestNum - lookback);
        console.log(
          `cold start: latestLedger≈${latestNum} startLedger=${startLedger} (lookback=${lookback})`,
        );
      }
    }

    let events;
    try {
      events = await fetchPage(startLedger, cursor);
    } catch (e) {
      const msg =
        e instanceof Error
          ? e.message
          : typeof e === "object" && e && "message" in e
            ? String((e as { message: unknown }).message)
            : String(e);
      const m = msg.match(/ledger range:\s*(\d+)\s*-\s*(\d+)/i);
      if (m && !cursor) {
        const minL = Number(m[1]);
        const maxL = Number(m[2]);
        startLedger = Math.min(Math.max(startLedger ?? minL, minL), maxL);
        console.warn(
          `adjusting startLedger to provider window → ${startLedger} (${minL}-${maxL})`,
        );
        events = await fetchPage(startLedger, undefined);
      } else {
        throw e;
      }
    }

    const payload = events as {
      events?: RawEvent[];
      cursor?: string;
      latestLedger?: number;
    };

    let ingested = 0;
    let skipped = 0;
    for (const ev of payload.events ?? []) {
      const decoded = decodeEvent(ev);
      if (!decoded) {
        skipped += 1;
        continue;
      }
      await upsert(decoded.table, decoded.entity);
      ingested += 1;
    }

    if (payload.cursor) {
      await setState("events_cursor", payload.cursor);
      cursor = payload.cursor;
    }
    if (payload.latestLedger != null) {
      await setState("last_ledger", String(payload.latestLedger));
    }
    await setState("last_poll_at", new Date().toISOString());

    if (ingested || skipped) {
      console.log(
        `poll: ingested=${ingested} skipped=${skipped} cursor=${cursor ?? ""}`,
      );
    }

    return { cursor, ingested, skipped };
  } finally {
    await releasePollLock().catch(() => undefined);
    polling = false;
  }
}

export async function runPollLoop(signal?: AbortSignal): Promise<void> {
  const ms = Number(process.env.POLL_MS ?? 30_000);
  let backoff = ms;

  const tick = async () => {
    if (signal?.aborted) return;
    try {
      await pollOnce();
      backoff = ms;
    } catch (e) {
      console.error("poll error", e);
      backoff = Math.min(backoff * 2, 5 * 60_000);
    }
  };

  await tick();
  const handle = setInterval(() => {
    void tick();
  }, backoff);

  // Re-arm interval when backoff changes after errors — simple: use fixed POLL_MS and sleep in tick on error
  signal?.addEventListener("abort", () => clearInterval(handle));
}

if (import.meta.url === `file://${process.argv[1]}`) {
  pollOnce().then((r) => {
    console.log("poll done", r);
  });
}
