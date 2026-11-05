/**
 * Cursor-based Soroban RPC getEvents poller.
 * Maps contract events into schema-compatible entities for FE/BE.
 */
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { rpc } from "@stellar/stellar-sdk";
import { upsert } from "./store.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

type Deployments = {
  attestation_registry: { contract_id: string };
  trade_credit_pool: { contract_id: string };
};

function loadDep(): Deployments {
  return JSON.parse(
    readFileSync(resolve(__dirname, "../../deployments/testnet.json"), "utf8"),
  );
}

export async function pollOnce(cursor?: string) {
  const dep = loadDep();
  const rpcUrl = process.env.STELLAR_RPC_URL ?? "https://soroban-testnet.stellar.org";
  const server = new rpc.Server(rpcUrl, { allowHttp: true });
  const contracts = [
    dep.trade_credit_pool.contract_id,
    dep.attestation_registry.contract_id,
  ].filter((c) => c && !c.startsWith("C..."));

  if (contracts.length === 0) {
    console.warn("No real contract IDs in deployments/testnet.json — skip poll");
    return cursor;
  }

  const startLedger = Number(process.env.START_LEDGER ?? 0) || undefined;
  const events = await server.getEvents({
    filters: contracts.map((contractId) => ({ type: "contract", contractIds: [contractId] })),
    startLedger,
    cursor,
    limit: 100,
  } as never);

  // Soft-parse: store raw event envelopes for operators; typed mapping fills as topics stabilize.
  const payload = events as { events?: Array<Record<string, unknown>>; cursor?: string };
  for (const ev of payload.events ?? []) {
    const id = String(ev.id ?? `${ev.txHash}-${ev.eventIndex}`);
    const tx = String(ev.txHash ?? "");
    const ts = String(ev.ledgerClosedAt ?? Date.now());
    const topic0 = JSON.stringify((ev as { topic?: unknown[] }).topic?.[0] ?? "");
    if (topic0.includes("Deposited") || topic0.includes("deposit")) {
      upsert("Deposit", {
        id,
        user: "unknown",
        amount: "0",
        blockNumber: String(ev.ledger ?? ""),
        blockTimestamp: ts,
        transactionHash: tx,
      });
    } else if (topic0.includes("Funded") || topic0.includes("ReceivableFunded")) {
      upsert("ReceivableFunded", {
        id,
        receivableId: "unknown",
        blockNumber: String(ev.ledger ?? ""),
        blockTimestamp: ts,
        transactionHash: tx,
      });
    } else if (topic0.includes("Repaid") || topic0.includes("Payment")) {
      upsert("PaymentRecorded", {
        id,
        receivableId: "unknown",
        payment: "0",
        blockNumber: String(ev.ledger ?? ""),
        blockTimestamp: ts,
        transactionHash: tx,
      });
    } else if (topic0.includes("AttestationIssued")) {
      upsert("AttestationIssued", {
        id,
        subjectId: "unknown",
        attestationType: "unknown",
        issuer: "unknown",
        blockTimestamp: ts,
        transactionHash: tx,
      });
    }
  }

  return payload.cursor ?? cursor;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  pollOnce().then((c) => {
    console.log("poll done, cursor=", c);
  });
}
