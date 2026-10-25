/** Durable fulfillment for ONE gateway process per ledger. Multiple processes
 * require a transactional database; this module deliberately provides no claim
 * of cross-process locking or exactly-once delivery over a network. */
import { mkdir, open, readFile, realpath, lstat } from "node:fs/promises";
import { constants } from "node:fs";
import { createHash, randomBytes } from "node:crypto";
import path from "node:path";

export type PaidSkuId = "moladt.geometry" | "dft.cached_result" | "dft.skala_cached" | "dft.pack" | "dft.credit";
export interface FulfillmentToken {
  token: string;
  sku_id: PaidSkuId;
  created_at_unix: number;
  expires_at_unix: number;
  stripe_ref: string;
  label?: string;
  smiles?: string;
  pack_id?: string;
  remaining_redemptions?: number;
  issued_redemptions?: number;
  used_at_unix?: number;
  used_for?: string;
}
export interface FulfillmentStore { path: string; ttlSeconds: number }
const queues = new Map<string, Promise<unknown>>();

export function parseSkuId(raw: unknown): PaidSkuId {
  if (typeof raw === "string" && ["moladt.geometry", "dft.cached_result", "dft.skala_cached", "dft.pack", "dft.credit"].includes(raw)) return raw as PaidSkuId;
  throw new Error("invalid_fulfillment_sku");
}

function positiveInteger(value: number, name: string) {
  if (!Number.isSafeInteger(value) || value <= 0) throw new Error(`invalid_${name}`);
}

async function canonicalStore(storePath: string): Promise<string> {
  const resolved = path.resolve(storePath);
  await mkdir(path.dirname(resolved), { recursive: true, mode: 0o700 });
  return path.join(await realpath(path.dirname(resolved)), path.basename(resolved));
}

async function transaction<T>(store: FulfillmentStore, action: (file: string, rows: Map<string, FulfillmentToken>) => Promise<T>): Promise<T> {
  const file = await canonicalStore(store.path);
  const previous = queues.get(file) ?? Promise.resolve();
  const operation = previous.catch(() => {}).then(async () => action(file, await loadRows(file)));
  queues.set(file, operation);
  try { return await operation; }
  finally { if (queues.get(file) === operation) queues.delete(file); }
}

function validateRow(row: FulfillmentToken): void {
  if (!row || typeof row !== "object" || !/^ft_[0-9a-f]{48}$/.test(row.token)) throw new Error("corrupt_fulfillment_ledger");
  parseSkuId(row.sku_id);
  positiveInteger(row.created_at_unix, "created_at");
  positiveInteger(row.expires_at_unix, "expires_at");
  if (row.expires_at_unix <= row.created_at_unix || typeof row.stripe_ref !== "string" || !row.stripe_ref.trim()) throw new Error("corrupt_fulfillment_ledger");
  for (const field of [row.label, row.smiles, row.pack_id, row.used_for]) if (field !== undefined && typeof field !== "string") throw new Error("corrupt_fulfillment_ledger");
  if (row.used_at_unix !== undefined) positiveInteger(row.used_at_unix, "used_at");
  if (row.sku_id === "dft.credit") {
    positiveInteger(row.issued_redemptions!, "issued_redemptions");
    if (!Number.isSafeInteger(row.remaining_redemptions) || row.remaining_redemptions! < 0 || row.remaining_redemptions! > row.issued_redemptions!) throw new Error("corrupt_fulfillment_credits");
  } else if (row.remaining_redemptions !== undefined || row.issued_redemptions !== undefined) throw new Error("invalid_credit_sku");
  if (row.sku_id === "dft.pack" && !row.pack_id) throw new Error("pack_id_required");
}

function grant(row: FulfillmentToken) {
  return JSON.stringify([row.sku_id, row.stripe_ref, row.label ?? null, row.smiles ?? null, row.pack_id ?? null, row.issued_redemptions ?? null]);
}

function validTransition(previous: FulfillmentToken, next: FulfillmentToken) {
  if (grant(previous) !== grant(next) || previous.token !== next.token || previous.created_at_unix !== next.created_at_unix || previous.expires_at_unix !== next.expires_at_unix ||
    (previous.used_at_unix !== undefined && JSON.stringify(previous) !== JSON.stringify(next)) ||
    (previous.remaining_redemptions !== undefined && next.remaining_redemptions! > previous.remaining_redemptions)) {
    throw new Error("invalid_fulfillment_transition");
  }
}

async function loadRows(file: string): Promise<Map<string, FulfillmentToken>> {
  const rows = new Map<string, FulfillmentToken>();
  const refs = new Map<string, string>();
  let text: string;
  try {
    const info = await lstat(file);
    if (!info.isFile() || info.isSymbolicLink()) throw new Error("unsafe_fulfillment_ledger");
    if (info.size > 64 * 1024 * 1024) throw new Error("fulfillment_ledger_requires_archival");
    text = await readFile(file, "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return rows;
    throw error;
  }
  if (text && !text.endsWith("\n")) throw new Error("incomplete_fulfillment_ledger");
  for (const line of text.split("\n")) {
    if (!line) continue;
    const row = JSON.parse(line) as FulfillmentToken;
    validateRow(row);
    if (refs.has(row.stripe_ref) && refs.get(row.stripe_ref) !== row.token) throw new Error("duplicate_fulfillment_payment");
    const previous = rows.get(row.token);
    if (previous) validTransition(previous, row);
    refs.set(row.stripe_ref, row.token);
    rows.set(row.token, row);
  }
  return rows;
}

async function appendDurable(file: string, row: FulfillmentToken): Promise<void> {
  validateRow(row);
  const handle = await open(file, constants.O_WRONLY | constants.O_APPEND | constants.O_CREAT | constants.O_NOFOLLOW, 0o600);
  try { await handle.writeFile(`${JSON.stringify(row)}\n`, "utf8"); await handle.sync(); }
  finally { await handle.close(); }
  const directory = await open(path.dirname(file), constants.O_RDONLY);
  try { await directory.sync(); } finally { await directory.close(); }
}

export function newTokenId(): string { return `ft_${randomBytes(24).toString("hex")}`; }

export async function mintFulfillmentToken(store: FulfillmentStore, opts: {
  skuId: PaidSkuId; stripeRef: string; label?: string; smiles?: string; packId?: string;
  remaining_redemptions?: number; ttlSeconds?: number;
}): Promise<FulfillmentToken> {
  const ttl = opts.ttlSeconds ?? store.ttlSeconds;
  positiveInteger(ttl, "fulfillment_ttl");
  if (typeof opts.stripeRef !== "string" || !opts.stripeRef.trim()) throw new Error("payment_reference_required");
  return transaction(store, async (file, rows) => {
    const now = Math.floor(Date.now() / 1000);
    const row: FulfillmentToken = {
      token: newTokenId(), sku_id: parseSkuId(opts.skuId), created_at_unix: now, expires_at_unix: now + ttl,
      stripe_ref: opts.stripeRef, label: opts.label, smiles: opts.smiles, pack_id: opts.packId,
      remaining_redemptions: opts.remaining_redemptions, issued_redemptions: opts.remaining_redemptions,
    };
    validateRow(row);
    const existing = [...rows.values()].find(r => r.stripe_ref === opts.stripeRef);
    if (existing) {
      if (grant(existing) !== grant(row)) throw new Error("fulfillment_idempotency_conflict");
      return structuredClone(existing); // Do not replenish used credits or expiry on replay.
    }
    await appendDurable(file, row);
    return row;
  });
}

export async function updateFulfillmentToken(store: FulfillmentStore, row: FulfillmentToken): Promise<void> {
  await transaction(store, async (file, rows) => {
    const previous = rows.get(row.token);
    if (!previous) throw new Error("unknown_fulfillment_token");
    validateRow(row);
    validTransition(previous, row);
    await appendDurable(file, row);
  });
}

export async function getFulfillmentToken(store: FulfillmentStore, token: string): Promise<FulfillmentToken | null> {
  return transaction(store, async (_file, rows) => structuredClone(rows.get(token) ?? null));
}

export function publicFulfillmentView(row: FulfillmentToken): Record<string, unknown> {
  const expired = Math.floor(Date.now() / 1000) >= row.expires_at_unix;
  const used = row.used_at_unix !== undefined;
  return {
    token: row.token, sku_id: row.sku_id,
    status: expired ? "expired" : used ? "used" : row.sku_id === "dft.credit" && !row.remaining_redemptions ? "exhausted" : "ready",
    expires_at_unix: row.expires_at_unix, label: row.label ?? null, smiles: row.smiles ?? null,
    pack_id: row.pack_id ?? null, remaining_redemptions: row.remaining_redemptions ?? null,
    how_to_redeem: { header: `PAYMENT-SIGNATURE: fulfill:${row.token}`, dft_cached: "GET /v1/dft/cached?label=<label> (1 credit)", dft_skala: "GET /v1/dft/skala?label=<label> (4 credits)", dft_pack: "GET /v1/dft/packs/<pack_id>" },
  };
}

export async function redeemFulfillmentToken(store: FulfillmentStore, token: string, skuId: string, usedFor: string,
  target: { label?: string; smiles?: string; packId?: string } = {},
): Promise<{ ok: true; row: FulfillmentToken } | { ok: false; reason: "missing" | "expired" | "used" | "sku_mismatch" | "exhausted" | "target_mismatch" }> {
  return transaction(store, async (file, rows) => {
    const row = rows.get(token);
    if (!row) return { ok: false, reason: "missing" };
    const now = Math.floor(Date.now() / 1000);
    if (now >= row.expires_at_unix) return { ok: false, reason: "expired" };
    if ((row.label && row.label !== target.label) || (row.smiles && row.smiles !== target.smiles) || (row.pack_id && row.pack_id !== target.packId)) return { ok: false, reason: "target_mismatch" };
    let updated: FulfillmentToken;
    if (row.sku_id === "dft.credit" && ["dft.cached_result", "dft.skala_cached"].includes(skuId)) {
      const remaining = row.remaining_redemptions ?? 0;
      const cost = skuId === "dft.skala_cached" ? 4 : 1;
      if (remaining < cost || row.used_at_unix !== undefined) return { ok: false, reason: "exhausted" };
      updated = { ...row, remaining_redemptions: remaining - cost, used_at_unix: remaining === cost ? now : undefined, used_for: usedFor };
    } else {
      if (row.used_at_unix !== undefined) return { ok: false, reason: "used" };
      if (row.sku_id !== skuId) return { ok: false, reason: "sku_mismatch" };
      updated = { ...row, used_at_unix: now, used_for: usedFor };
    }
    await appendDurable(file, updated);
    return { ok: true, row: updated };
  });
}

export function extractFulfillToken(signature: string): string | null {
  const raw = signature.trim().replace(/^fulfill:/, "").trim();
  return /^ft_[0-9a-f]{48}$/.test(raw) ? raw : null;
}
export function stripeRefHash(ref: string): string { return createHash("sha256").update(ref).digest("hex").slice(0, 16); }
