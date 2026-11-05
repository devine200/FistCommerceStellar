-- Fist Commerce Soroban direct indexer (not The Graph / SubQuery).

CREATE TABLE IF NOT EXISTS indexer_state (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS deposits (
  id TEXT PRIMARY KEY,
  "user" TEXT NOT NULL,
  amount TEXT NOT NULL,
  "blockNumber" TEXT NOT NULL,
  "blockTimestamp" TEXT NOT NULL,
  "transactionHash" TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS withdrawals (
  id TEXT PRIMARY KEY,
  "user" TEXT NOT NULL,
  amount TEXT NOT NULL,
  "blockNumber" TEXT NOT NULL,
  "blockTimestamp" TEXT NOT NULL,
  "transactionHash" TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS receivable_fundeds (
  id TEXT PRIMARY KEY,
  "receivableId" TEXT NOT NULL,
  "blockNumber" TEXT NOT NULL,
  "blockTimestamp" TEXT NOT NULL,
  "transactionHash" TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS payment_recordeds (
  id TEXT PRIMARY KEY,
  "receivableId" TEXT NOT NULL,
  payment TEXT NOT NULL,
  "blockNumber" TEXT NOT NULL,
  "blockTimestamp" TEXT NOT NULL,
  "transactionHash" TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS attestation_issueds (
  id TEXT PRIMARY KEY,
  "subjectId" TEXT NOT NULL,
  "attestationType" TEXT NOT NULL,
  issuer TEXT NOT NULL,
  "blockTimestamp" TEXT NOT NULL,
  "transactionHash" TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS attestation_revokeds (
  id TEXT PRIMARY KEY,
  "subjectId" TEXT NOT NULL,
  "attestationType" TEXT NOT NULL,
  "blockTimestamp" TEXT NOT NULL,
  "transactionHash" TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_deposits_ts ON deposits ("blockTimestamp" DESC);
CREATE INDEX IF NOT EXISTS idx_receivable_fundeds_ts ON receivable_fundeds ("blockTimestamp" DESC);
CREATE INDEX IF NOT EXISTS idx_attestation_issueds_ts ON attestation_issueds ("blockTimestamp" DESC);
