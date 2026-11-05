import { readFileSync } from "node:fs";
import { rpc } from "@stellar/stellar-sdk";

async function main() {
  const dep = JSON.parse(readFileSync("../deployments/testnet.json", "utf8"));
  const server = new rpc.Server(
    process.env.STELLAR_RPC_URL || "https://soroban-testnet.stellar.org",
    { allowHttp: true },
  );
  const latest = await server.getLatestLedger();
  const latestNum =
    typeof latest === "number" ? latest : Number((latest as { sequence: number }).sequence);
  const start = Math.max(1, latestNum - 15_000);
  const events = await server.getEvents({
    filters: [
      { type: "contract", contractIds: [dep.trade_credit_pool.contract_id] },
      { type: "contract", contractIds: [dep.attestation_registry.contract_id] },
    ],
    startLedger: start,
    limit: 8,
  } as never);
  const list = (events as { events?: unknown[] }).events ?? [];
  console.log("count", list.length, "latest", latestNum, "start", start);
  for (const ev of list.slice(0, 5)) {
    console.log("---");
    console.log(JSON.stringify(ev, null, 2).slice(0, 1500));
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
