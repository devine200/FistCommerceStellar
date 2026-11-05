/** In-memory store for local/demo; swap for Postgres in production deploy. */

export type Entity = Record<string, string>;

const tables: Record<string, Entity[]> = {
  Deposit: [],
  Withdrawal: [],
  ReceivableFunded: [],
  PaymentRecorded: [],
  AttestationIssued: [],
  AttestationRevoked: [],
};

export function upsert(table: string, entity: Entity) {
  if (!tables[table]) tables[table] = [];
  const idx = tables[table].findIndex((e) => e.id === entity.id);
  if (idx >= 0) tables[table][idx] = entity;
  else tables[table].push(entity);
}

export function list(table: string, first = 100, skip = 0): Entity[] {
  return (tables[table] ?? []).slice(skip, skip + first);
}

export function allTables() {
  return tables;
}
