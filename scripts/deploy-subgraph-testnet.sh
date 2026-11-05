#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/subgraph"
docker compose up -d --build
echo "Soroban subgraph GraphQL: http://localhost:4000/graphql"
echo "Set SOROBAN_TESTNET_SUBGRAPH_ENDPOINT=http://localhost:4000/graphql in FistCommerceBackend"
