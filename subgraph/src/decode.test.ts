import { decodeEvent } from "./decode.js";

function assert(cond: unknown, msg: string): asserts cond {
  if (!cond) throw new Error(msg);
}

const deposit = decodeEvent({
  id: "1",
  ledger: 100,
  ledgerClosedAt: "2026-01-01T00:00:00Z",
  transactionHash: "abc",
  topic: [{ symbol: "deposited" }],
  value: { investor: "GABC", amount: "1000" },
});
assert(deposit?.table === "Deposit", "deposit table");
assert(deposit.entity.user === "GABC", "deposit user");
assert(deposit.entity.amount === "1000", "deposit amount");

const funded = decodeEvent({
  id: "2",
  ledger: 101,
  ledgerClosedAt: "2026-01-01T00:01:00Z",
  transactionHash: "def",
  topic: [{ symbol: "receivable_funded" }],
  value: { receivable_id: "FIST-DEMO-1", amount: "5" },
});
assert(funded?.table === "ReceivableFunded", "funded table");
assert(funded.entity.receivableId === "FIST-DEMO-1", "funded id");

const att = decodeEvent({
  id: "3",
  ledger: 102,
  ledgerClosedAt: "2026-01-01T00:02:00Z",
  transactionHash: "ghi",
  topic: [{ symbol: "attestation_issued" }],
  value: { subject_id: "deadbeef", attestation_type: "1", issuer: "GISSUER" },
});
assert(att?.table === "AttestationIssued", "att table");
assert(att.entity.issuer === "GISSUER", "att issuer");

console.log("decode.test.ts OK");
