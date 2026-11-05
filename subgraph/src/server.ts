import express from "express";
import { buildSchema } from "graphql";
import { createHandler } from "graphql-http/lib/use/express";
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { list } from "./store.js";
import { pollOnce } from "./poller.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const typeDefs = readFileSync(resolve(__dirname, "../schema.graphql"), "utf8");

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

app.get("/health", (_req, res) => res.json({ ok: true, project: "Fist Commerce" }));

const port = Number(process.env.PORT ?? 4000);
app.listen(port, () => {
  console.log(`Fist Commerce Soroban subgraph GraphQL on :${port}/graphql`);
});

// background poll
let cursor: string | undefined;
setInterval(() => {
  pollOnce(cursor)
    .then((c) => {
      cursor = c;
    })
    .catch((e) => console.error("poll error", e));
}, Number(process.env.POLL_MS ?? 15_000));
