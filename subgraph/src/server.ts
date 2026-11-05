/**
 * Fist Commerce — Soroban direct event indexer GraphQL API.
 * Stellar-only; EVM chains continue to use The Graph / SubQuery elsewhere.
 */
import express from "express";
import { buildSchema } from "graphql";
import { createHandler } from "graphql-http/lib/use/express";
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { list, healthSnapshot, migrate, usingPostgres } from "./db.js";
import { pollOnce } from "./poller.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const typeDefs = readFileSync(resolve(__dirname, "../schema.graphql"), "utf8")
  // The Graph `@entity` directives are not valid for graphql-js; strip for local API.
  .replace(/@entity(\([^)]*\))?/g, "");

const schema = buildSchema(typeDefs);

const root = {
  deposits: ({ first, skip }: { first?: number; skip?: number }) =>
    list("Deposit", first, skip),
  withdrawals: ({ first, skip }: { first?: number; skip?: number }) =>
    list("Withdrawal", first, skip),
  receivableFundeds: ({ first, skip }: { first?: number; skip?: number }) =>
    list("ReceivableFunded", first, skip),
  paymentRecordeds: ({ first, skip }: { first?: number; skip?: number }) =>
    list("PaymentRecorded", first, skip),
  attestationIssueds: ({ first, skip }: { first?: number; skip?: number }) =>
    list("AttestationIssued", first, skip),
};

const app = express();
app.all(
  "/graphql",
  createHandler({
    schema,
    rootValue: root,
  }),
);

app.get("/health", async (_req, res) => {
  try {
    const snap = await healthSnapshot();
    res.json({
      ok: true,
      project: "Fist Commerce",
      indexer: "soroban-direct",
      ...snap,
    });
  } catch (e) {
    res.status(500).json({
      ok: false,
      project: "Fist Commerce",
      error: e instanceof Error ? e.message : String(e),
    });
  }
});

const port = Number(process.env.PORT ?? 4000);
const pollMs = Number(process.env.POLL_MS ?? 30_000);

async function main() {
  if (usingPostgres()) {
    await migrate();
    console.log("Postgres migrations applied");
  } else {
    console.warn(
      "DATABASE_URL unset — using in-memory store (demo only; data lost on restart)",
    );
  }

  app.listen(port, () => {
    console.log(
      `Fist Commerce Soroban indexer GraphQL on :${port}/graphql (poll ${pollMs}ms)`,
    );
  });

  let backoff = pollMs;
  const tick = async () => {
    try {
      await pollOnce();
      backoff = pollMs;
    } catch (e) {
      console.error("poll error", e);
      backoff = Math.min(Math.max(backoff * 2, pollMs), 5 * 60_000);
    } finally {
      setTimeout(tick, backoff);
    }
  };
  // Initial poll shortly after boot
  setTimeout(tick, 1_000);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
