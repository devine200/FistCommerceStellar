# Architecture

## Track A (brief submission surface)

```text
Investor --deposit FISTUSD SAC--> TradeCreditPool
Originator --register_receivable--> TradeCreditPool
RiskManager --approve_receivable--> TradeCreditPool
PoolManager --fund_receivable--> TradeCreditPool --transfer--> Merchant
TradeCreditPool --is_attestation_valid x2--> AttestationRegistry
Payer --repay--> TradeCreditPool
```

Statuses: `Registered → Approved → Funded → PartiallyRepaid → Settled` (+ `Cancelled`).

Settlement asset: **FISTUSD** via Stellar Asset Contract only (never branded as USDC / Circle).

## Track B (FistCommerce EVM-parity edge cases)

```text
Investors <-> TradeCreditPool (NAV / withdraw / loss)
TradeCreditPool <-> ReceivableAllocator (risk tiers, exposure)
TradeCreditPool <-> PayoutRouter (two-step payout / repay)
AttestationRegistry gates + verification lifecycle
ProtocolController pauses
ReceivableToken binds loan docs (not settlement)
FistMultisig threshold admin
Subgraph poller -> GraphQL -> Backend Celery / FE history
```

## Frontend

Selecting **Soroban Testnet** enables only capabilities that exist on deployed Soroban modules; EVM-only screens are hidden.
