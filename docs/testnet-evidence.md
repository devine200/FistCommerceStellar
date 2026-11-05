# Testnet evidence

Project: **Fist Commerce**  
Demo receivable: `FIST-DEMO-001` · Merchant: `MERCHANT-DEMO-001` · Advance: 10,000 FISTUSD · Contractual repayment: 10,500 FISTUSD

Successful transactions must link to a Testnet explorer. For failed/rejected invocations, paste reproducible command output — **do not invent transaction hashes**.

## Live deployment (2026-11-05)

| Resource | ID / Link |
|----------|-----------|
| Admin | `GBG2BACHNIJJ65PGRXTZQLFPP4VZJUWZ362MNZWKNE65BEXJKII4FTZE` |
| Issuer | `GCRGIQHPFQWSRB27AKKY4C33MOY4JPMYTIAE7R4HWHWZF24CCK3AKFI3` |
| FISTUSD SAC | `CAPA5QMLHIWOZO7N3REUVHXYQRUAOW32S6B3BZ7VTVQSDPGQH4DJ6GPQ` |
| AttestationRegistry | [`CDALKH7ZYF2EAWBVUIVLXUNKVXTGWJY5VLVFIVOB2LGPEH6OFGJD4VOM`](https://lab.stellar.org/r/testnet/contract/CDALKH7ZYF2EAWBVUIVLXUNKVXTGWJY5VLVFIVOB2LGPEH6OFGJD4VOM) |
| TradeCreditPool | [`CABUICVVHPHD5BF5J3ID3BH4OJMTEKPQQKOPHRKQYRKER3MAFDS4T43V`](https://lab.stellar.org/r/testnet/contract/CABUICVVHPHD5BF5J3ID3BH4OJMTEKPQQKOPHRKQYRKER3MAFDS4T43V) |
| ProtocolController | `CBIKSAYJBXMN3276JJNZW3ASNM53YP65SIF7I7FJMRAM4PEMR7NP3CQO` |
| ReceivableAllocator | `CCCIT3V4G5SZHD4ECSK3TXMSC42PP5ZYZYVYTTJYOGINLTO5YINJOUFK` |
| PayoutRouter | `CCKXHHQJ6EH472CRNZKBACWSW4TOINWTJ726CPB4HIOW6TISD22SLUWW` |
| ReceivableToken | `CDRNNS6WPG6Y4ITUFRLCFTBMML2VC55KUIRNOFIGUCWVLHVEZQFXCP4X` |
| FistMultisig | `CAIGNJJWQ5DZYP6LLHPSXOYR5QKBG74OHQJ7VQNWJPUNPIJNNCYVLY45` |

| Step | Contract | Function | Transaction / Evidence | Result |
|------|----------|----------|------------------------|--------|
| Deploy registry wasm | AttestationRegistry | upload | [d10a3c3d…](https://stellar.expert/explorer/testnet/tx/d10a3c3d0895df12c493aca36862ba13400eae9a7b73adeb2160ef455e66bab4) | Success |
| Deploy registry | AttestationRegistry | Deploy | [b3a41d03…](https://stellar.expert/explorer/testnet/tx/b3a41d03bd008c14d001241730ac94ca0a791627af8bd22500d35e95038ee562) | Success |
| Deploy pool | TradeCreditPool | Deploy | [d0fe9780…](https://stellar.expert/explorer/testnet/tx/d0fe9780a0e39d5cc913c31baa30cc9dcc9d420dfe62af234d58a9b190925ed9) | Success |
| Initialize registry | AttestationRegistry | initialize | [6b8661ad…](https://stellar.expert/explorer/testnet/tx/6b8661adefd5b7ec2c50cf3944fca1a89da309e2ac284dd8c1e5c7ea4da32e77) | Success |
| Authorize MerchantApproved | AttestationRegistry | authorize_issuer | [e72eda1c…](https://stellar.expert/explorer/testnet/tx/e72eda1cc4a56dde9ea2b52f406c40ff48611892efd1a6d487553fee65d00809) | Success |
| Authorize DocumentsVerified | AttestationRegistry | authorize_issuer | [712661dc…](https://stellar.expert/explorer/testnet/tx/712661dc06130e94b4c80afc24204666d089dc7a114699fbde492bf6be82c73b) | Success |
| Approve investor | TradeCreditPool | set_investor_status | [31966447…](https://stellar.expert/explorer/testnet/tx/31966447cdf86f681a20ac379b3b04c46bc05cdb4c43d7d122a95bbdd25d16de) | Success |
| Init protocol controller | ProtocolController | initialize | [fb239f1a…](https://stellar.expert/explorer/testnet/tx/fb239f1ae8b674189e7f13c6ddd057b80feed5731d9e1f0f38db0e78015664e3) | Success |
| Init multisig | FistMultisig | initialize | [c5c20b94…](https://stellar.expert/explorer/testnet/tx/c5c20b94301a4a5db40b6fc7c292eaacf216063ac2c36cd2668b36639f6da53d) | Success |
| Deposit | TradeCreditPool | deposit | [b1a470a1…](https://stellar.expert/explorer/testnet/tx/b1a470a195636163a85dcd4125a24753d5e2f9929c57c4379cf820c4f8095941) | Success |
| Register receivable | TradeCreditPool | register_receivable | [22193463…](https://stellar.expert/explorer/testnet/tx/221934630e77154dcbfc38502cfe9c33fa2c6c833fe6c49b3cc7ec7b65e5092e) | Success (`FIST-DEMO-1793882666`) |
| Approve receivable | TradeCreditPool | approve_receivable | [9690ecb8…](https://stellar.expert/explorer/testnet/tx/9690ecb82b12b7e8d0ba77c998b8b78a0a33c38d50edd47f27612a6427ca24cc) | Success |
| Funding attempt 1 | TradeCreditPool | fund_receivable | `/tmp/fist-e2e-logs/fund-attempt-1.log` — `Error(Contract, #12)` AttestationMissing | Rejected |
| Merchant attestation | AttestationRegistry | issue_attestation | [bd9995f9…](https://stellar.expert/explorer/testnet/tx/bd9995f9785b1de527c2ad7f48c72a6c3ec2fed7251e3169b336e2980720eb44) | Success (type=1) |
| Funding attempt 2 | TradeCreditPool | fund_receivable | `/tmp/fist-e2e-logs/fund-attempt-2.log` — still `#12` (DocumentsVerified false) | Rejected |
| Document attestation | AttestationRegistry | issue_attestation | [ab183dcd…](https://stellar.expert/explorer/testnet/tx/ab183dcdf6d16a1b74e4a4240661791b6d20b9f199ac931a37cef9bfa5139962) | Success (type=2) |
| Fund receivable | TradeCreditPool | fund_receivable | [1d539c14…](https://stellar.expert/explorer/testnet/tx/1d539c1465ac7a09430fa5add926ae8062a8d431f82329db38d8cb261eae9335) | Success — `ATTESTATION_GATE_PASS` |

Manifest: [`deployments/testnet.json`](../deployments/testnet.json)  
Live script: `./scripts/run-live-attestation-gate.sh`
