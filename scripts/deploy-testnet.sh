#!/usr/bin/env bash
# Fist Commerce — deploy Track A contracts + FISTUSD SAC to Stellar Testnet.
# Does not print or write secret seeds.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export PATH="${HOME}/.local/bin:${PATH}"

NETWORK="${NETWORK:-testnet}"
NETWORK_PASSPHRASE="${NETWORK_PASSPHRASE:-Test SDF Network ; September 2015}"
DEPLOYMENTS_FILE="${DEPLOYMENTS_FILE:-$ROOT/deployments/testnet.json}"

if ! command -v stellar >/dev/null; then
  echo "stellar CLI not found. Install from https://github.com/stellar/stellar-cli/releases" >&2
  exit 1
fi

STELLAR_CLI_VERSION="$(stellar version 2>/dev/null | head -1 || echo unknown)"
SOROBAN_SDK_VERSION="$(grep -A2 'name = "soroban-sdk"' Cargo.lock | grep version | head -1 | sed 's/.*"\(.*\)"/\1/' || echo 29.0.1)"
SOURCE_COMMIT="$(git rev-parse HEAD 2>/dev/null || echo unknown)"

echo "==> Building contracts"
stellar contract build --package attestation-registry
stellar contract build --package trade-credit-pool

WASM_REG="$ROOT/target/wasm32v1-none/release/attestation_registry.wasm"
WASM_POOL="$ROOT/target/wasm32v1-none/release/trade_credit_pool.wasm"
# Fallback path if build uses wasm32-unknown-unknown naming
[[ -f "$WASM_REG" ]] || WASM_REG="$ROOT/target/wasm32-unknown-unknown/release/attestation_registry.wasm"
[[ -f "$WASM_POOL" ]] || WASM_POOL="$ROOT/target/wasm32-unknown-unknown/release/trade_credit_pool.wasm"

if [[ ! -f "$WASM_REG" || ! -f "$WASM_POOL" ]]; then
  echo "Wasm artifacts missing. Run: stellar contract build" >&2
  ls -la "$ROOT/target"/*/release/*.wasm 2>/dev/null || true
  exit 1
fi

echo "==> Ensure network + identity (uses local stellar config; secrets never written to deployments/)"
stellar network add --global "$NETWORK" \
  --rpc-url "${STELLAR_RPC_URL:-${SOROBAN_TESTNET_RPC_URL:-https://soroban-testnet.stellar.org}}" \
  --network-passphrase "$NETWORK_PASSPHRASE" 2>/dev/null || true

IDENTITY="${STELLAR_IDENTITY:-fist-deployer}"
if ! stellar keys ls 2>/dev/null | grep -q "$IDENTITY"; then
  echo "Create identity first: stellar keys generate $IDENTITY --network $NETWORK --fund" >&2
  exit 1
fi

echo "==> Deploy AttestationRegistry"
REG_OUT="$(stellar contract deploy --wasm "$WASM_REG" --source "$IDENTITY" --network "$NETWORK")"
REG_ID="$(echo "$REG_OUT" | tail -1 | tr -d '[:space:]')"
REG_HASH="$(shasum -a 256 "$WASM_REG" | awk '{print $1}')"

echo "==> Deploy TradeCreditPool"
POOL_OUT="$(stellar contract deploy --wasm "$WASM_POOL" --source "$IDENTITY" --network "$NETWORK")"
POOL_ID="$(echo "$POOL_OUT" | tail -1 | tr -d '[:space:]')"
POOL_HASH="$(shasum -a 256 "$WASM_POOL" | awk '{print $1}')"

# FISTUSD: require pre-issued SAC contract id via env for reproducibility
FISTUSD_CODE="${FISTUSD_CODE:-FISTUSD}"
FISTUSD_ISSUER="${FISTUSD_ISSUER:-}"
FISTUSD_SAC="${FISTUSD_SAC_CONTRACT_ID:-}"

mkdir -p "$(dirname "$DEPLOYMENTS_FILE")"
cat > "$DEPLOYMENTS_FILE" <<EOF
{
  "project": "Fist Commerce",
  "network": "$NETWORK",
  "network_passphrase": "$NETWORK_PASSPHRASE",
  "stellar_cli_version": "$STELLAR_CLI_VERSION",
  "soroban_sdk_version": "$SOROBAN_SDK_VERSION",
  "source_commit": "$SOURCE_COMMIT",
  "test_asset": {
    "code": "$FISTUSD_CODE",
    "issuer": "${FISTUSD_ISSUER:-G...}",
    "sac_contract_id": "${FISTUSD_SAC:-C...}"
  },
  "attestation_registry": {
    "contract_id": "$REG_ID",
    "wasm_hash": "$REG_HASH",
    "deployment_transaction": "see explorer for deploy of $REG_ID"
  },
  "trade_credit_pool": {
    "contract_id": "$POOL_ID",
    "wasm_hash": "$POOL_HASH",
    "deployment_transaction": "see explorer for deploy of $POOL_ID"
  },
  "subgraph_url": "${SUBGRAPH_URL:-}"
}
EOF

echo "Wrote $DEPLOYMENTS_FILE"
echo "Registry: $REG_ID"
echo "Pool:     $POOL_ID"
echo "Initialize contracts and issue FISTUSD separately; see scripts/run-demo-flow.sh"
