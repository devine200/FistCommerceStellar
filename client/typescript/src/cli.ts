#!/usr/bin/env node
/**
 * Fist Commerce — minimal TypeScript demo client (brief §9).
 * Uses @stellar/stellar-sdk. Prefer generated bindings via:
 *   npx @stellar/stellar-sdk generate --wasm <path> --output src/generated/...
 */
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import {
  Contract,
  Networks,
  rpc,
  TransactionBuilder,
  Account,
  Keypair,
  nativeToScVal,
  scValToNative,
  xdr,
} from "@stellar/stellar-sdk";

const __dirname = dirname(fileURLToPath(import.meta.url));

type Deployments = {
  network_passphrase: string;
  attestation_registry: { contract_id: string };
  trade_credit_pool: { contract_id: string };
  test_asset: { sac_contract_id: string; code: string };
};

function loadDeployments(): Deployments {
  const path = resolve(__dirname, "../../../deployments/testnet.json");
  return JSON.parse(readFileSync(path, "utf8"));
}

function usage(): never {
  console.log(`Fist Commerce Stellar demo client

Commands:
  accounting              Read pool accounting
  receivable <id>         Read receivable state
  approve-investor <G...> Admin approve investor
  deposit <G...> <amt> <opHex>
  register <id> <merchantId> <merchantG> <advance> <repay> <maturity>
  approve-receivable <id>
  issue-attestation <issuerG> <subjectHex> <type> <commitmentHex> <expires>
  fund <id> <opHex>
  repay <id> <payerG> <amt> <opHex>

Env: STELLAR_SECRET (signer), STELLAR_RPC_URL (default Testnet RPC)
`);
  process.exit(1);
}

async function main() {
  const [cmd, ...args] = process.argv.slice(2);
  if (!cmd) usage();

  const dep = loadDeployments();
  const rpcUrl = process.env.STELLAR_RPC_URL ?? "https://soroban-testnet.stellar.org";
  const server = new rpc.Server(rpcUrl, { allowHttp: rpcUrl.startsWith("http://") });
  const pool = new Contract(dep.trade_credit_pool.contract_id);
  const registry = new Contract(dep.attestation_registry.contract_id);

  const secret = process.env.STELLAR_SECRET;
  const kp = secret ? Keypair.fromSecret(secret) : null;

  async function invoke(contract: Contract, fn: string, params: xdr.ScVal[]) {
    if (!kp) throw new Error("STELLAR_SECRET required for invoke");
    const account = await server.getAccount(kp.publicKey());
    const tx = new TransactionBuilder(account as Account, {
      fee: "100000",
      networkPassphrase: dep.network_passphrase || Networks.TESTNET,
    })
      .addOperation(contract.call(fn, ...params))
      .setTimeout(120)
      .build();
    const prepared = await server.prepareTransaction(tx);
    prepared.sign(kp);
    const send = await server.sendTransaction(prepared);
    console.log("send:", send.status, send.hash);
    if (send.status === "ERROR") {
      console.error(send);
      process.exit(1);
    }
    // poll
    for (let i = 0; i < 30; i++) {
      await new Promise((r) => setTimeout(r, 2000));
      const r = await server.getTransaction(send.hash);
      if (r.status === "SUCCESS") {
        console.log("confirmed:", send.hash);
        return r;
      }
      if (r.status === "FAILED") {
        console.error(r);
        process.exit(1);
      }
    }
    throw new Error("timeout waiting for " + send.hash);
  }

  switch (cmd) {
    case "accounting": {
      const simAccount = kp?.publicKey() ?? Keypair.random().publicKey();
      const account = await server.getAccount(simAccount).catch(async () => {
        // simulation-only: build with random and simulate
        return { accountId: simAccount, sequenceNumber: "0" } as unknown as Account;
      });
      const tx = new TransactionBuilder(
        new Account(simAccount, "0"),
        { fee: "100000", networkPassphrase: dep.network_passphrase || Networks.TESTNET },
      )
        .addOperation(pool.call("get_pool_accounting"))
        .setTimeout(30)
        .build();
      const sim = await server.simulateTransaction(tx);
      console.log(JSON.stringify(sim, null, 2));
      break;
    }
    case "receivable": {
      const id = args[0] ?? "FIST-DEMO-001";
      const simAccount = Keypair.random().publicKey();
      const tx = new TransactionBuilder(new Account(simAccount, "0"), {
        fee: "100000",
        networkPassphrase: dep.network_passphrase || Networks.TESTNET,
      })
        .addOperation(pool.call("get_receivable", nativeToScVal(id, { type: "string" })))
        .setTimeout(30)
        .build();
      const sim = await server.simulateTransaction(tx);
      console.log(JSON.stringify(sim, null, 2));
      break;
    }
    case "approve-investor": {
      await invoke(pool, "set_investor_status", [
        nativeToScVal(args[0], { type: "address" }),
        nativeToScVal(true, { type: "bool" }),
      ]);
      break;
    }
    case "deposit": {
      await invoke(pool, "deposit", [
        nativeToScVal(args[0], { type: "address" }),
        nativeToScVal(BigInt(args[1]), { type: "i128" }),
        nativeToScVal(Buffer.from(args[2], "hex")),
      ]);
      break;
    }
    case "fund": {
      await invoke(pool, "fund_receivable", [
        nativeToScVal(args[0], { type: "string" }),
        nativeToScVal(Buffer.from(args[1], "hex")),
      ]);
      break;
    }
    case "repay": {
      await invoke(pool, "repay", [
        nativeToScVal(args[0], { type: "string" }),
        nativeToScVal(args[1], { type: "address" }),
        nativeToScVal(BigInt(args[2]), { type: "i128" }),
        nativeToScVal(Buffer.from(args[3], "hex")),
      ]);
      break;
    }
    default:
      usage();
  }

  void registry;
  void scValToNative;
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
