# Testnet evidence

Project: **Fist Commerce**  
Demo receivable: `FIST-DEMO-001` · Merchant: `MERCHANT-DEMO-001` · Advance: 10,000 FISTUSD · Contractual repayment: 10,500 FISTUSD

Successful transactions must link to a Testnet explorer. For failed/rejected invocations, paste reproducible command output — **do not invent transaction hashes**.

| Step | Contract | Function | Transaction / Evidence | Result |
|------|----------|----------|------------------------|--------|
| Deploy registry | AttestationRegistry | Deploy | _pending_ | Success |
| Deploy pool | TradeCreditPool | Deploy | _pending_ | Success |
| Approve investor | TradeCreditPool | set_investor_status | _pending_ | Success |
| Deposit | TradeCreditPool | deposit | _pending_ | Success |
| Register receivable | TradeCreditPool | register_receivable | _pending_ | Success |
| Approve receivable | TradeCreditPool | approve_receivable | _pending_ | Success |
| Funding attempt 1 | TradeCreditPool | fund_receivable | See `/tmp/fist-fund-attempt-1.log` or CI artifact | Rejected: missing attestations |
| Merchant attestation | AttestationRegistry | issue_attestation | _pending_ | Success |
| Funding attempt 2 | TradeCreditPool | fund_receivable | See `/tmp/fist-fund-attempt-2.log` | Rejected: missing document verification |
| Document attestation | AttestationRegistry | issue_attestation | _pending_ | Success |
| Fund receivable | TradeCreditPool | fund_receivable | _pending_ | Success |
| Partial repayment | TradeCreditPool | repay | _pending_ | Success |
| Final repayment | TradeCreditPool | repay | _pending_ | Settled |

Fill explorer links after `./scripts/deploy-testnet.sh` and `./scripts/run-demo-flow.sh` (or the TypeScript demo client) against live Testnet.
