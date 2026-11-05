# Fist Commerce — Soroban GraphQL playground (optional)

**Not the system-of-record indexer.**

Production / product indexing for Stellar runs in **FistCommerceBackend**:

- Celery task `metrics_handler.tasks.index_soroban_events_task`
- Persists to Django `IndexedEvents` (`chain_id=1397037394`)
- RPC: `SOROBAN_TESTNET_RPC_URL` (QuikNode or SDF)

This Node service is an optional local GraphQL playground only. You do **not** need it for BE indexing.

```bash
# Backend (authoritative)
# SOROBAN_TESTNET_INDEXING_ENABLED=true
# celery worker + beat → index_soroban_events_task

# Optional local playground only:
npm install && npm run dev
```

See `FistCommerceBackend/metrics_handler/soroban_indexer.py` and `utils/soroban_rpc.py`.
