# Fist Commerce — Trade Credit Contracts (Soroban)

**Fist Commerce** Stellar/Soroban smart contracts: a limited feasibility prototype of the trade-credit financing path, plus Track B modules that mirror FistCommerce EVM edge-case logic for Testnet.

## What this prototype demonstrates

- Investor allowlist + FISTUSD (SAC) deposit into `TradeCreditPool`
- Synthetic receivable register → risk approve → dual attestation gate → fund/disburse
- Partial and final repayment until Settled
- AttestationRegistry without storing PII on-chain

## What it does not demonstrate

- Production lending / Mainnet / real capital
- Circle-issued USDC (test asset is **FISTUSD**, not USDC)
- Full production security review or threat monitoring

## Toolchain

Recorded at implement time (see `Cargo.lock` and `deployments/testnet.json` after deploy):

- Rust stable + `wasm32v1-none`
- `soroban-sdk` 29.0.1
- Stellar CLI 30.0.1

## Build & test

```bash
cargo test --workspace
stellar contract build --package attestation-registry
stellar contract build --package trade-credit-pool
```

## Deploy (Testnet)

```bash
./scripts/deploy-testnet.sh
./scripts/run-demo-flow.sh
```

Contract IDs, Wasm hashes, and explorer links: [`deployments/testnet.json`](deployments/testnet.json), [`docs/testnet-evidence.md`](docs/testnet-evidence.md).

## Docs

- [architecture.md](docs/architecture.md)
- [trust-model.md](docs/trust-model.md)
- [scope.md](docs/scope.md)
- [testnet-evidence.md](docs/testnet-evidence.md)

## Contributors (roles)

- Timilehin Olowu — Soroban/Rust Lead
- King Favour — Stellar Full-Stack Integration
- Jaffar Keikei — Stellar Integration and QA
- Tochukwu Samuel — Backend and Evidence

## Track B / SCF note

Track B (withdraw queue, NAV, two-step payout, pauses, allocator, multisig, indexer, gated FE) is **expanded self-funded feasibility** beyond the minimal pre-submission brief. SCF funding is not requested retroactively for this work. Remaining production-grade items (USDC, Mainnet, hardening, pilot capital) stay for SCF.
