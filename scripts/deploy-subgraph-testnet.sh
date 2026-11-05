#!/usr/bin/env bash
# OPTIONAL local GraphQL playground only — NOT the product indexer.
# Authoritative Stellar indexing: FistCommerceBackend Celery
#   metrics_handler.tasks.index_soroban_events_task → IndexedEvents.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/subgraph"
echo "WARNING: This starts an optional Node playground, not the BE indexer." >&2

if [[ -f "$ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT/.env"
  set +a
fi
if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

export STELLAR_RPC_URL="${STELLAR_RPC_URL:-${SOROBAN_TESTNET_RPC_URL:-https://soroban-testnet.stellar.org}}"
export PORT="${PORT:-4000}"
export POLL_MS="${POLL_MS:-30000}"
export DATABASE_URL="${DATABASE_URL:-postgres://fist:fist@postgres:5432/stellar_indexer}"

docker compose up -d --build

echo "Soroban direct indexer GraphQL: http://localhost:${PORT}/graphql"
echo "Health: http://localhost:${PORT}/health"
echo "Set SOROBAN_TESTNET_SUBGRAPH_ENDPOINT=http://localhost:${PORT}/graphql in FistCommerceBackend"
echo "RPC configured (token redacted if QuikNode)."
