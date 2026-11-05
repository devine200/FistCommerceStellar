#!/usr/bin/env bash
# Fist Commerce — fund roles assumed; issue FISTUSD, deploy SAC + contracts, initialize, write manifest.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="${HOME}/.local/bin:${PATH}"
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Library/Developer/CommandLineTools}"

NETWORK="${NETWORK:-testnet}"
NETWORK_PASSPHRASE="${NETWORK_PASSPHRASE:-Test SDF Network ; September 2015}"
DEPLOYER="${STELLAR_IDENTITY:-fist-deployer}"
ISSUER_ID="${FISTUSD_ISSUER_IDENTITY:-fist-issuer}"
INVESTOR_ID="${INVESTOR_IDENTITY:-fist-investor}"
MERCHANT_ID="${MERCHANT_IDENTITY:-fist-merchant}"
ASSET_CODE="${FISTUSD_CODE:-FISTUSD}"

DEPLOYER_G="$(stellar keys address "$DEPLOYER")"
ISSUER_G="$(stellar keys address "$ISSUER_ID")"
INVESTOR_G="$(stellar keys address "$INVESTOR_ID")"
MERCHANT_G="$(stellar keys address "$MERCHANT_ID")"
ASSET="${ASSET_CODE}:${ISSUER_G}"

echo "Deployer: $DEPLOYER_G"
echo "Issuer:   $ISSUER_G"
echo "Investor: $INVESTOR_G"
echo "Merchant: $MERCHANT_G"
echo "Asset:    $ASSET"

echo "==> Trustlines for investor + merchant"
stellar tx new change-trust --source "$INVESTOR_ID" --network "$NETWORK" --line "$ASSET" || true
stellar tx new change-trust --source "$MERCHANT_ID" --network "$NETWORK" --line "$ASSET" || true

echo "==> Mint classic FISTUSD to investor (creates asset supply)"
# 1_000_000 FISTUSD in stroops (7 decimals): 1e6 * 1e7 = 1e13
MINT_STROOPS="${MINT_STROOPS:-10000000000000}"
stellar tx new payment \
  --source "$ISSUER_ID" \
  --network "$NETWORK" \
  --destination "$INVESTOR_G" \
  --asset "$ASSET" \
  --amount "$MINT_STROOPS"

echo "==> Deploy FISTUSD SAC"
SAC_OUT="$(stellar contract asset deploy --asset "$ASSET" --source "$ISSUER_ID" --network "$NETWORK" --alias fistusd 2>&1 | tee /tmp/fist-sac-deploy.log)"
echo "$SAC_OUT"
SAC_ID="$(stellar contract id asset --asset "$ASSET" --network "$NETWORK")"
echo "SAC: $SAC_ID"

echo "==> Build Wasm"
stellar contract build --package attestation-registry
stellar contract build --package trade-credit-pool
stellar contract build --package protocol-controller || true
stellar contract build --package receivable-allocator || true
stellar contract build --package payout-router || true
stellar contract build --package receivable-token || true
stellar contract build --package fist-multisig || true

find_wasm() {
  local name="$1"
  local p
  for p in \
    "$ROOT/target/wasm32v1-none/release/${name}.wasm" \
    "$ROOT/target/wasm32-unknown-unknown/release/${name}.wasm" \
    "/var/folders/"*/*/cursor-sandbox-cache/*/cargo-target/wasm32v1-none/release/"${name}.wasm"
  do
    # shellcheck disable=SC2086
    if compgen -G "$p" > /dev/null 2>&1; then
      compgen -G "$p" | head -1
      return 0
    fi
  done
  # last resort search
  find "$ROOT/target" -name "${name}.wasm" 2>/dev/null | head -1
}

WASM_REG="$(find_wasm attestation_registry)"
WASM_POOL="$(find_wasm trade_credit_pool)"
echo "Registry wasm: $WASM_REG"
echo "Pool wasm:     $WASM_POOL"
[[ -f "$WASM_REG" && -f "$WASM_POOL" ]] || { echo "Missing wasm" >&2; exit 1; }

echo "==> Deploy AttestationRegistry"
REG_ID="$(stellar contract deploy --wasm "$WASM_REG" --source "$DEPLOYER" --network "$NETWORK" --alias fist-attestation-registry | tail -1 | tr -d '[:space:]')"
echo "Registry: $REG_ID"

echo "==> Deploy TradeCreditPool"
POOL_ID="$(stellar contract deploy --wasm "$WASM_POOL" --source "$DEPLOYER" --network "$NETWORK" --alias fist-trade-credit-pool | tail -1 | tr -d '[:space:]')"
echo "Pool: $POOL_ID"

# Track B deploys (best-effort)
deploy_pkg() {
  local pkg="$1" alias="$2"
  local wasm
  wasm="$(find_wasm "$pkg")"
  if [[ -n "$wasm" && -f "$wasm" ]]; then
    stellar contract deploy --wasm "$wasm" --source "$DEPLOYER" --network "$NETWORK" --alias "$alias" | tail -1 | tr -d '[:space:]'
  else
    echo "pending"
  fi
}

PC_ID="$(deploy_pkg protocol_controller fist-protocol-controller)"
RA_ID="$(deploy_pkg receivable_allocator fist-receivable-allocator)"
PR_ID="$(deploy_pkg payout_router fist-payout-router)"
RT_ID="$(deploy_pkg receivable_token fist-receivable-token)"
MS_ID="$(deploy_pkg fist_multisig fist-multisig)"

echo "==> Initialize AttestationRegistry"
stellar contract invoke --id "$REG_ID" --source "$DEPLOYER" --network "$NETWORK" -- \
  initialize --admin "$DEPLOYER_G"

echo "==> Authorize issuers (MerchantApproved=1 DocumentsVerified=2 as enum discriminants)"
stellar contract invoke --id "$REG_ID" --source "$DEPLOYER" --network "$NETWORK" -- \
  authorize_issuer --attestation_type 1 --issuer "$ISSUER_G"
stellar contract invoke --id "$REG_ID" --source "$DEPLOYER" --network "$NETWORK" -- \
  authorize_issuer --attestation_type 2 --issuer "$ISSUER_G"

echo "==> Initialize TradeCreditPool (deployer holds all Track A roles)"
stellar contract invoke --id "$POOL_ID" --source "$DEPLOYER" --network "$NETWORK" -- \
  initialize \
  --admin "$DEPLOYER_G" \
  --pool_manager "$DEPLOYER_G" \
  --originator "$DEPLOYER_G" \
  --risk_manager "$DEPLOYER_G" \
  --settlement_asset "$SAC_ID" \
  --attestation_registry "$REG_ID"

STELLAR_CLI_VERSION="$(stellar version 2>/dev/null | head -1 || echo unknown)"
SOROBAN_SDK_VERSION="$(grep -A2 'name = "soroban-sdk"' Cargo.lock | grep version | head -1 | sed 's/.*"\(.*\)"/\1/' || echo 29.0.1)"
SOURCE_COMMIT="$(git rev-parse HEAD 2>/dev/null || echo unknown)"
REG_HASH="$(shasum -a 256 "$WASM_REG" | awk '{print $1}')"
POOL_HASH="$(shasum -a 256 "$WASM_POOL" | awk '{print $1}')"

cat > "$ROOT/deployments/testnet.json" <<EOF
{
  "project": "Fist Commerce",
  "network": "$NETWORK",
  "network_passphrase": "$NETWORK_PASSPHRASE",
  "stellar_cli_version": "$STELLAR_CLI_VERSION",
  "soroban_sdk_version": "$SOROBAN_SDK_VERSION",
  "source_commit": "$SOURCE_COMMIT",
  "accounts": {
    "admin": "$DEPLOYER_G",
    "issuer": "$ISSUER_G",
    "investor": "$INVESTOR_G",
    "merchant": "$MERCHANT_G"
  },
  "test_asset": {
    "code": "$ASSET_CODE",
    "issuer": "$ISSUER_G",
    "sac_contract_id": "$SAC_ID"
  },
  "attestation_registry": {
    "contract_id": "$REG_ID",
    "wasm_hash": "$REG_HASH",
    "deployment_transaction": "deployed-via-setup-testnet"
  },
  "trade_credit_pool": {
    "contract_id": "$POOL_ID",
    "wasm_hash": "$POOL_HASH",
    "deployment_transaction": "deployed-via-setup-testnet"
  },
  "subgraph_url": "${SUBGRAPH_URL:-http://localhost:4000/graphql}",
  "track_b": {
    "protocol_controller": "$PC_ID",
    "receivable_allocator": "$RA_ID",
    "payout_router": "$PR_ID",
    "receivable_token": "$RT_ID",
    "fist_multisig": "$MS_ID"
  }
}
EOF

echo "Wrote deployments/testnet.json"
cat "$ROOT/deployments/testnet.json"
