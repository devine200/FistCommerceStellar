/**
 * Map Soroban RPC getEvents envelopes → GraphQL entities.
 * Tolerates SDK JSON shapes and stringified diagnostic topics.
 */
import type { Entity } from "./db.js";

export type RawEvent = {
  id?: string;
  type?: string;
  ledger?: number | string;
  ledgerClosedAt?: string;
  contractId?: string;
  transactionHash?: string;
  txHash?: string;
  eventIndex?: number | string;
  topic?: unknown[];
  value?: unknown;
  inSuccessfulContractCall?: boolean;
};

function asPlain(v: unknown): unknown {
  if (v == null) return v;
  if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") return v;
  if (Array.isArray(v)) return v.map(asPlain);
  if (typeof v === "object") {
    const o = v as Record<string, unknown>;
    // stellar-sdk / RPC JSON often wraps as { string: "..." } | { symbol: "..." } | { address: "..." }
    if ("string" in o && Object.keys(o).length === 1) return o.string;
    if ("symbol" in o && Object.keys(o).length === 1) return o.symbol;
    if ("address" in o && Object.keys(o).length === 1) return o.address;
    if ("bytes" in o && Object.keys(o).length === 1) return String(o.bytes);
    if ("i128" in o) return String((o.i128 as { toString?: () => string })?.toString?.() ?? o.i128);
    if ("u64" in o) return String(o.u64);
    if ("u32" in o) return String(o.u32);
    const out: Record<string, unknown> = {};
    for (const [k, val] of Object.entries(o)) out[k] = asPlain(val);
    return out;
  }
  return String(v);
}

function topicStrings(topic: unknown[] | undefined): string[] {
  if (!topic?.length) return [];
  return topic.map((t) => {
    const p = asPlain(t);
    if (typeof p === "string") return p;
    return JSON.stringify(p);
  });
}

function eventName(topics: string[]): string {
  // Convention: first topic is symbol / name (Deposited, receivable_funded, …)
  const t0 = (topics[0] ?? "").replace(/^"|"$/g, "");
  return t0;
}

function pickField(value: unknown, keys: string[]): string {
  const plain = asPlain(value);
  if (plain == null) return "";
  if (typeof plain !== "object") return String(plain);
  const obj = plain as Record<string, unknown>;
  for (const k of keys) {
    if (obj[k] != null && obj[k] !== "") return String(asPlain(obj[k]));
  }
  // nested map style
  for (const k of keys) {
    const found = Object.entries(obj).find(([ek]) => ek.toLowerCase() === k.toLowerCase());
    if (found) return String(asPlain(found[1]));
  }
  return "";
}

function baseMeta(ev: RawEvent): Pick<
  Entity,
  "id" | "blockNumber" | "blockTimestamp" | "transactionHash"
> {
  const tx = String(ev.transactionHash ?? ev.txHash ?? "");
  const id = String(ev.id ?? `${tx}-${ev.eventIndex ?? "0"}`);
  return {
    id,
    blockNumber: String(ev.ledger ?? ""),
    blockTimestamp: String(ev.ledgerClosedAt ?? ""),
    transactionHash: tx,
  };
}

export type Decoded =
  | { table: "Deposit"; entity: Entity }
  | { table: "Withdrawal"; entity: Entity }
  | { table: "ReceivableFunded"; entity: Entity }
  | { table: "PaymentRecorded"; entity: Entity }
  | { table: "AttestationIssued"; entity: Entity }
  | { table: "AttestationRevoked"; entity: Entity }
  | null;

export function decodeEvent(ev: RawEvent): Decoded {
  const topics = topicStrings(ev.topic);
  const name = eventName(topics).toLowerCase();
  const meta = baseMeta(ev);
  const value = ev.value;

  // Also match joined topic blob for soft diagnostic formats
  const blob = topics.join(" ").toLowerCase();

  if (
    name.includes("deposited") ||
    name === "deposit" ||
    blob.includes("deposited")
  ) {
    return {
      table: "Deposit",
      entity: {
        ...meta,
        user:
          pickField(value, ["investor", "user", "from"]) ||
          topics[1] ||
          "unknown",
        amount: pickField(value, ["amount", "units"]) || "0",
      },
    };
  }

  if (name.includes("withdraw") || blob.includes("withdraw")) {
    return {
      table: "Withdrawal",
      entity: {
        ...meta,
        user: pickField(value, ["investor", "user", "to"]) || topics[1] || "unknown",
        amount: pickField(value, ["amount", "units"]) || "0",
      },
    };
  }

  if (
    name.includes("receivable_funded") ||
    name.includes("receivablefunded") ||
    name.includes("funded") ||
    blob.includes("receivable_funded") ||
    blob.includes("receivablefunded")
  ) {
    // Avoid matching "Deposited" etc.
    if (name.includes("deposit")) return null;
    return {
      table: "ReceivableFunded",
      entity: {
        ...meta,
        receivableId:
          pickField(value, ["receivable_id", "receivableId", "id"]) ||
          topics[1] ||
          "unknown",
      },
    };
  }

  if (
    name.includes("repay") ||
    name.includes("payment") ||
    blob.includes("repaid") ||
    blob.includes("payment")
  ) {
    return {
      table: "PaymentRecorded",
      entity: {
        ...meta,
        receivableId:
          pickField(value, ["receivable_id", "receivableId"]) || topics[1] || "unknown",
        payment: pickField(value, ["amount", "payment", "repaid"]) || "0",
      },
    };
  }

  if (
    name.includes("attestation_issued") ||
    name.includes("attestationissued") ||
    blob.includes("attestation_issued")
  ) {
    return {
      table: "AttestationIssued",
      entity: {
        ...meta,
        subjectId:
          pickField(value, ["subject_id", "subjectId"]) || topics[1] || "unknown",
        attestationType: String(
          pickField(value, ["attestation_type", "attestationType"]) ||
            topics[2] ||
            "unknown",
        ),
        issuer: pickField(value, ["issuer"]) || topics[3] || "unknown",
      },
    };
  }

  if (
    name.includes("attestation_revoked") ||
    name.includes("attestationrevoked") ||
    blob.includes("attestation_revoked")
  ) {
    return {
      table: "AttestationRevoked",
      entity: {
        ...meta,
        subjectId:
          pickField(value, ["subject_id", "subjectId"]) || topics[1] || "unknown",
        attestationType: String(
          pickField(value, ["attestation_type", "attestationType"]) ||
            topics[2] ||
            "unknown",
        ),
      },
    };
  }

  return null;
}
