#!/usr/bin/env bash
# Fist Commerce — 16-step FIST-DEMO-001 synthetic Testnet demonstration.
# Failed funding attempts must leave reproducible command output (no invented hashes).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEPLOYMENTS="${DEPLOYMENTS_FILE:-$ROOT/deployments/testnet.json}"
NETWORK="${NETWORK:-testnet}"
IDENTITY="${STELLAR_IDENTITY:-fist-deployer}"
EVIDENCE="$ROOT/docs/testnet-evidence.md"

if [[ ! -f "$DEPLOYMENTS" ]]; then
  echo "Missing $DEPLOYMENTS — run ./scripts/deploy-testnet.sh first" >&2
  exit 1
fi

command -v jq >/dev/null || { echo "jq required" >&2; exit 1; }
command -v stellar >/dev/null || { echo "stellar CLI required" >&2; exit 1; }

REG="$(jq -r .attestation_registry.contract_id "$DEPLOYMENTS")"
POOL="$(jq -r .trade_credit_pool.contract_id "$DEPLOYMENTS")"
SAC="$(jq -r .test_asset.sac_contract_id "$DEPLOYMENTS")"

RECEIVABLE_ID="FIST-DEMO-001"
MERCHANT_ID="MERCHANT-DEMO-001"
ADVANCE=100000000000  # 10000 with 7 decimals typical; adjust to asset
REPAY_TOTAL=105000000000
PARTIAL=50000000000

echo "Project: Fist Commerce"
echo "Receivable: $RECEIVABLE_ID Merchant: $MERCHANT_ID"
echo "Pool=$POOL Registry=$REG SAC=$SAC"
echo
echo "This script expects contracts initialized and roles/accounts funded."
echo "Steps 8 and 10 intentionally fail without both attestations."
echo

run_step() {
  local title="$1"; shift
  echo "==> $title"
  if "$@"; then
    echo "OK: $title"
  else
    echo "FAILED (expected for attestation-gate steps): $title" >&2
    return 1
  fi
}

# Placeholders — operators fill identity addresses via env
ADMIN="${ADMIN_ADDR:?set ADMIN_ADDR}"
INVESTOR="${INVESTOR_ADDR:?set INVESTOR_ADDR}"
ORIGINATOR="${ORIGINATOR_ADDR:?set ORIGINATOR_ADDR}"
RISK="${RISK_MANAGER_ADDR:?set RISK_MANAGER_ADDR}"
POOL_MGR="${POOL_MANAGER_ADDR:?set POOL_MANAGER_ADDR}"
MERCHANT="${MERCHANT_ADDR:?set MERCHANT_ADDR}"
ISSUER="${ISSUER_ADDR:?set ISSUER_ADDR}"

echo "1-3: Asset/SAC/contracts assumed deployed (see deployments/testnet.json)"
echo "4: Approve investor"
stellar contract invoke --id "$POOL" --source "$IDENTITY" --network "$NETWORK" -- \
  set_investor_status --investor "$INVESTOR" --approved true

echo "5: Deposit"
stellar contract invoke --id "$POOL" --source "$IDENTITY" --network "$NETWORK" -- \
  deposit --investor "$INVESTOR" --amount "$ADVANCE" --operation_id "$(openssl rand -hex 16)"

echo "6: Register receivable"
MATURITY=$(( $(date +%s) + 86400 * 30 ))
stellar contract invoke --id "$POOL" --source "$IDENTITY" --network "$NETWORK" -- \
  register_receivable \
  --receivable_id "$RECEIVABLE_ID" \
  --merchant_id "$MERCHANT_ID" \
  --merchant_payout_address "$MERCHANT" \
  --advance_amount "$ADVANCE" \
  --contractual_repayment_amount "$REPAY_TOTAL" \
  --maturity_timestamp "$MATURITY"

echo "7: Approve receivable"
stellar contract invoke --id "$POOL" --source "$IDENTITY" --network "$NETWORK" -- \
  approve_receivable --receivable_id "$RECEIVABLE_ID"

echo "8: Funding attempt 1 (expect reject — missing attestations)"
set +e
stellar contract invoke --id "$POOL" --source "$IDENTITY" --network "$NETWORK" -- \
  fund_receivable --receivable_id "$RECEIVABLE_ID" --operation_id "$(openssl rand -hex 16)" \
  2>&1 | tee /tmp/fist-fund-attempt-1.log
set -e

echo "9: Issue MerchantApproved attestation (operator supplies subject_id bytes)"
echo "   Use client/typescript demo to issue attestations with hashed merchant id."

echo "10: Funding attempt 2 (expect reject — missing DocumentsVerified)"
set +e
stellar contract invoke --id "$POOL" --source "$IDENTITY" --network "$NETWORK" -- \
  fund_receivable --receivable_id "$RECEIVABLE_ID" --operation_id "$(openssl rand -hex 16)" \
  2>&1 | tee /tmp/fist-fund-attempt-2.log
set -e

echo "11-16: Issue DocumentsVerified, fund, partial repay, final repay — see client/typescript"
echo "Append explorer links to $EVIDENCE after successful txs."
echo "Demo flow scaffolding complete."
