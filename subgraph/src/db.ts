/**
 * Postgres-backed store for the Soroban direct indexer.
 * Falls back to in-memory when DATABASE_URL is unset (local demo only).
 */
import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import pg from "pg";

export type Entity = Record<string, string>;

const __dirname = dirname(fileURLToPath(import.meta.url));

const TABLE_MAP: Record<string, string> = {
  Deposit: "deposits",
  Withdrawal: "withdrawals",
  ReceivableFunded: "receivable_fundeds",
  PaymentRecorded: "payment_recordeds",
  AttestationIssued: "attestation_issueds",
  AttestationRevoked: "attestation_revokeds",
};

const COLUMNS: Record<string, string[]> = {
  deposits: ["id", "user", "amount", "blockNumber", "blockTimestamp", "transactionHash"],
  withdrawals: ["id", "user", "amount", "blockNumber", "blockTimestamp", "transactionHash"],
  receivable_fundeds: [
    "id",
    "receivableId",
    "blockNumber",
    "blockTimestamp",
    "transactionHash",
  ],
  payment_recordeds: [
    "id",
    "receivableId",
    "payment",
    "blockNumber",
    "blockTimestamp",
    "transactionHash",
  ],
  attestation_issueds: [
    "id",
    "subjectId",
    "attestationType",
    "issuer",
    "blockTimestamp",
    "transactionHash",
  ],
  attestation_revokeds: [
    "id",
    "subjectId",
    "attestationType",
    "blockTimestamp",
    "transactionHash",
  ],
};

/** In-memory fallback */
const memTables: Record<string, Entity[]> = {
  Deposit: [],
  Withdrawal: [],
  ReceivableFunded: [],
  PaymentRecorded: [],
  AttestationIssued: [],
  AttestationRevoked: [],
};
const memState: Record<string, string> = {};

let pool: pg.Pool | null = null;
let migrated = false;
/** Session held while poll advisory lock is owned (pool-safe). */
let lockClient: pg.PoolClient | null = null;

export function usingPostgres(): boolean {
  return Boolean((process.env.DATABASE_URL || "").trim());
}

export function getPool(): pg.Pool | null {
  const url = (process.env.DATABASE_URL || "").trim();
  if (!url) return null;
  if (!pool) {
    pool = new pg.Pool({ connectionString: url, max: 4 });
  }
  return pool;
}

export async function migrate(): Promise<void> {
  const p = getPool();
  if (!p || migrated) return;
  const sql = readFileSync(resolve(__dirname, "../migrations/001_init.sql"), "utf8");
  await p.query(sql);
  migrated = true;
}

export async function getState(key: string): Promise<string | undefined> {
  const p = getPool();
  if (!p) return memState[key];
  await migrate();
  const r = await p.query<{ value: string }>(
    "SELECT value FROM indexer_state WHERE key = $1",
    [key],
  );
  return r.rows[0]?.value;
}

export async function setState(key: string, value: string): Promise<void> {
  const p = getPool();
  if (!p) {
    memState[key] = value;
    return;
  }
  await migrate();
  await p.query(
    `INSERT INTO indexer_state (key, value, updated_at) VALUES ($1, $2, NOW())
     ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()`,
    [key, value],
  );
}

/** Try to take a session-level advisory lock. Returns false if another worker holds it. */
export async function tryPollLock(): Promise<boolean> {
  const p = getPool();
  if (!p) return true;
  await migrate();
  if (lockClient) return true;
  const client = await p.connect();
  try {
    const r = await client.query<{ locked: boolean }>(
      "SELECT pg_try_advisory_lock($1) AS locked",
      [0x4153544c], // 'ASTL'
    );
    if (!r.rows[0]?.locked) {
      client.release();
      return false;
    }
    lockClient = client;
    return true;
  } catch (e) {
    client.release();
    throw e;
  }
}

export async function releasePollLock(): Promise<void> {
  if (!lockClient) return;
  try {
    await lockClient.query("SELECT pg_advisory_unlock($1)", [0x4153544c]);
  } finally {
    lockClient.release();
    lockClient = null;
  }
}

export async function upsert(table: string, entity: Entity): Promise<void> {
  const p = getPool();
  if (!p) {
    if (!memTables[table]) memTables[table] = [];
    const idx = memTables[table].findIndex((e) => e.id === entity.id);
    if (idx >= 0) memTables[table][idx] = entity;
    else memTables[table].push(entity);
    return;
  }
  await migrate();
  const sqlTable = TABLE_MAP[table];
  if (!sqlTable) throw new Error(`Unknown entity table ${table}`);
  const cols = COLUMNS[sqlTable];
  const values = cols.map((c) => entity[c] ?? "");
  const placeholders = cols.map((_, i) => `$${i + 1}`).join(", ");
  const quoted = cols.map((c) => (c === "user" ? '"user"' : `"${c}"`)).join(", ");
  const updates = cols
    .filter((c) => c !== "id")
    .map((c) => {
      const q = c === "user" ? '"user"' : `"${c}"`;
      return `${q} = EXCLUDED.${q}`;
    })
    .join(", ");
  await p.query(
    `INSERT INTO ${sqlTable} (${quoted}) VALUES (${placeholders})
     ON CONFLICT (id) DO UPDATE SET ${updates}`,
    values,
  );
}

export async function list(
  table: string,
  first = 100,
  skip = 0,
): Promise<Entity[]> {
  const p = getPool();
  if (!p) {
    return (memTables[table] ?? []).slice(skip, skip + first);
  }
  await migrate();
  const sqlTable = TABLE_MAP[table];
  if (!sqlTable) return [];
  const r = await p.query<Entity>(
    `SELECT * FROM ${sqlTable} ORDER BY "blockTimestamp" DESC NULLS LAST LIMIT $1 OFFSET $2`,
    [first, skip],
  );
  return r.rows.map((row) => {
    const out: Entity = {};
    for (const [k, v] of Object.entries(row)) out[k] = v == null ? "" : String(v);
    return out;
  });
}

export async function healthSnapshot(): Promise<{
  backend: "postgres" | "memory";
  cursor?: string;
  lastLedger?: string;
  lastPollAt?: string;
  counts: Record<string, number>;
}> {
  const p = getPool();
  if (!p) {
    const counts: Record<string, number> = {};
    for (const [k, v] of Object.entries(memTables)) counts[k] = v.length;
    return {
      backend: "memory",
      cursor: memState.events_cursor,
      lastLedger: memState.last_ledger,
      lastPollAt: memState.last_poll_at,
      counts,
    };
  }
  await migrate();
  const cursor = await getState("events_cursor");
  const lastLedger = await getState("last_ledger");
  const lastPollAt = await getState("last_poll_at");
  const counts: Record<string, number> = {};
  for (const [entity, sqlTable] of Object.entries(TABLE_MAP)) {
    const r = await p.query<{ n: string }>(`SELECT COUNT(*)::text AS n FROM ${sqlTable}`);
    counts[entity] = Number(r.rows[0]?.n ?? 0);
  }
  return { backend: "postgres", cursor, lastLedger, lastPollAt, counts };
}

export async function closeDb(): Promise<void> {
  if (pool) {
    await pool.end();
    pool = null;
    migrated = false;
  }
}
