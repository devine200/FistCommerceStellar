#!/usr/bin/env bash
# Live Testnet attestation-gate demo: fund fails without attestations, then succeeds.
# Uses Stellar CLI keystore identities only — never prints or commits secrets.
set -euo pipefail

export PATH="${HOME}/.local/bin:${PATH}"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Library/Developer/CommandLineTools}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEPLOYMENTS="${DEPLOYMENTS_FILE:-$ROOT/deployments/testnet.json}"
NETWORK="${NETWORK:-testnet}"
ADMIN_ID="${STELLAR_IDENTITY:-fist-deployer}"
INVESTOR_ID="${INVESTOR_IDENTITY:-fist-investor}"
ISSUER_ID="${FISTUSD_ISSUER_IDENTITY:-fist-issuer}"
MERCHANT_ID_KEY="${MERCHANT_IDENTITY:-fist-merchant}"

command -v jq >/dev/null || { echo "jq required" >&2; exit 1; }
command -v stellar >/dev/null || { echo "stellar CLI required" >&2; exit 1; }
command -v openssl >/dev/null || { echo "openssl required" >&2; exit 1; }

REG="$(jq -r .attestation_registry.contract_id "$DEPLOYMENTS")"
POOL="$(jq -r .trade_credit_pool.contract_id "$DEPLOYMENTS")"
SAC="$(jq -r .test_asset.sac_contract_id "$DEPLOYMENTS")"
ADMIN_G="$(stellar keys address "$ADMIN_ID")"
INVESTOR_G="$(stellar keys address "$INVESTOR_ID")"
ISSUER_G="$(stellar keys address "$ISSUER_ID")"
MERCHANT_G="$(stellar keys address "$MERCHANT_ID_KEY")"

RUN_TS="$(date +%s)"
RECEIVABLE_ID="FIST-DEMO-${RUN_TS}"
# Unique merchant subject per run so prior Testnet attestations cannot short-circuit the gate.
MERCHANT_SUBJECT_STR="MERCHANT-DEMO-${RUN_TS}"
# Opaque subject: SHA-256 of UTF-8 merchant id string (matches contract Bytes::from(String) for ASCII)
SUBJECT_HEX="$(printf '%s' "$MERCHANT_SUBJECT_STR" | openssl dgst -sha256 -hex | awk '{print $NF}')"
EVIDENCE_HEX="$(openssl rand -hex 32)"
EXPIRES=$(( $(date +%s) + 86400 * 365 ))
ADVANCE=100000000000   # 10_000 * 1e7
REPAY=105000000000
DEPOSIT_AMT=200000000000
MATURITY=$(( $(date +%s) + 86400 * 30 ))

op_id() { openssl rand -hex 32; }

echo "Project: Fist Commerce — live attestation gate"
echo "Pool=$POOL Registry=$REG SAC=$SAC"
echo "Receivable=$RECEIVABLE_ID MerchantSubject=$MERCHANT_SUBJECT_STR"
echo "SubjectHex=$SUBJECT_HEX"

mkdir -p /tmp/fist-e2e-logs

echo "==> Approve investor"
stellar contract invoke --id "$POOL" --source "$ADMIN_ID" --network "$NETWORK" -- \
  set_investor_status --investor "$INVESTOR_G" --approved true

echo "==> Deposit (investor signs)"
stellar contract invoke --id "$POOL" --source "$INVESTOR_ID" --network "$NETWORK" -- \
  deposit --investor "$INVESTOR_G" --amount "$DEPOSIT_AMT" --operation_id "$(op_id)"

echo "==> Register receivable"
stellar contract invoke --id "$POOL" --source "$ADMIN_ID" --network "$NETWORK" -- \
  register_receivable \
  --receivable_id "$RECEIVABLE_ID" \
  --merchant_id "$MERCHANT_SUBJECT_STR" \
  --merchant_payout_address "$MERCHANT_G" \
  --advance_amount "$ADVANCE" \
  --contractual_repayment_amount "$REPAY" \
  --maturity_timestamp "$MATURITY"

echo "==> Approve receivable"
stellar contract invoke --id "$POOL" --source "$ADMIN_ID" --network "$NETWORK" -- \
  approve_receivable --receivable_id "$RECEIVABLE_ID"

echo "==> Funding attempt 1 (expect AttestationMissing)"
set +e
stellar contract invoke --id "$POOL" --source "$ADMIN_ID" --network "$NETWORK" -- \
  fund_receivable --receivable_id "$RECEIVABLE_ID" --operation_id "$(op_id)" \
  2>&1 | tee /tmp/fist-e2e-logs/fund-attempt-1.log
FUND1=${PIPESTATUS[0]}
set -e
if [[ "$FUND1" -eq 0 ]]; then
  echo "UNEXPECTED: fund succeeded without attestations" >&2
  exit 1
fi
grep -qiE 'AttestationMissing|Error\(Contract, #12\)|#12' /tmp/fist-e2e-logs/fund-attempt-1.log \
  || echo "(note) fund failed as expected; check log for contract error #12"

echo "==> Issue MerchantApproved attestation"
stellar contract invoke --id "$REG" --source "$ISSUER_ID" --network "$NETWORK" -- \
  issue_attestation \
  --issuer "$ISSUER_G" \
  --subject_id "$SUBJECT_HEX" \
  --attestation_type 1 \
  --evidence_commitment "$EVIDENCE_HEX" \
  --expires_at "$EXPIRES"

echo "==> Funding attempt 2 (expect still missing DocumentsVerified)"
set +e
stellar contract invoke --id "$POOL" --source "$ADMIN_ID" --network "$NETWORK" -- \
  fund_receivable --receivable_id "$RECEIVABLE_ID" --operation_id "$(op_id)" \
  2>&1 | tee /tmp/fist-e2e-logs/fund-attempt-2.log
FUND2=${PIPESTATUS[0]}
set -e
if [[ "$FUND2" -eq 0 ]]; then
  echo "UNEXPECTED: fund succeeded with only MerchantApproved" >&2
  exit 1
fi

echo "==> Issue DocumentsVerified attestation"
EVIDENCE2="$(openssl rand -hex 32)"
stellar contract invoke --id "$REG" --source "$ISSUER_ID" --network "$NETWORK" -- \
  issue_attestation \
  --issuer "$ISSUER_G" \
  --subject_id "$SUBJECT_HEX" \
  --attestation_type 2 \
  --evidence_commitment "$EVIDENCE2" \
  --expires_at "$EXPIRES"

echo "==> Funding attempt 3 (expect success)"
stellar contract invoke --id "$POOL" --source "$ADMIN_ID" --network "$NETWORK" -- \
  fund_receivable --receivable_id "$RECEIVABLE_ID" --operation_id "$(op_id)" \
  2>&1 | tee /tmp/fist-e2e-logs/fund-success.log

echo "ATTESTATION_GATE_PASS receivable=$RECEIVABLE_ID"
echo "Logs under /tmp/fist-e2e-logs/"
