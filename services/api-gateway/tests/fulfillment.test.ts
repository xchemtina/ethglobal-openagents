import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, readFile, appendFile, symlink } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { mintFulfillmentToken, redeemFulfillmentToken, getFulfillmentToken, updateFulfillmentToken, parseSkuId, type FulfillmentStore } from "../src/fulfillment.js";
import { mintCreditToken, mintPackToken } from "../src/credits.js";

async function fixture(t: any) {
  const dir = await mkdtemp(path.join(os.tmpdir(), "fulfillment-test-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  return { dir, store: { path: path.join(dir, "tokens.jsonl"), ttlSeconds: 3600 } as FulfillmentStore };
}

test("concurrent duplicate payment mints one durable token and restart preserves redemption", async t => {
  const { store } = await fixture(t);
  const opts = { skuId: "dft.cached_result" as const, stripeRef: "cs_same", label: "water" };
  const rows = await Promise.all(Array.from({ length: 20 }, () => mintFulfillmentToken(store, opts)));
  assert.equal(new Set(rows.map(r => r.token)).size, 1);
  assert.equal((await readFile(store.path, "utf8")).trim().split("\n").length, 1);
  const results = await Promise.all(Array.from({ length: 20 }, () => redeemFulfillmentToken(store, rows[0].token, "dft.cached_result", "/cache", { label: "water" })));
  assert.equal(results.filter(r => r.ok).length, 1);
  const replay = await mintFulfillmentToken(store, opts);
  assert.ok(replay.used_at_unix);
  const script = `import { getFulfillmentToken } from ${JSON.stringify(new URL('../src/fulfillment.ts', import.meta.url).href)}; console.log(JSON.stringify(await getFulfillmentToken(${JSON.stringify(store)}, ${JSON.stringify(replay.token)})));`;
  const persisted = JSON.parse(execFileSync(process.execPath, ["--import", "tsx", "--input-type=module", "-e", script], { encoding: "utf8" }));
  assert.equal(persisted.used_at_unix, replay.used_at_unix);
});

test("simultaneous multi-credit redemption cannot overspend or replenish on mint replay", async t => {
  const { store } = await fixture(t);
  const token = await mintCreditToken(store, { redemptions: 5, stripeRef: "paid-credit" });
  const results = await Promise.all(Array.from({ length: 8 }, () => redeemFulfillmentToken(store, token.token, "dft.skala_cached", "/skala")));
  assert.equal(results.filter(r => r.ok).length, 1);
  assert.equal((await getFulfillmentToken(store, token.token))?.remaining_redemptions, 1);
  assert.equal((await mintCreditToken(store, { redemptions: 5, stripeRef: "paid-credit" })).remaining_redemptions, 1);
  assert.equal((await redeemFulfillmentToken(store, token.token, "dft.cached_result", "/cached")).ok, true);
  assert.equal((await redeemFulfillmentToken(store, token.token, "dft.cached_result", "/cached")).ok, false);
});

test("stores are isolated under concurrent first load, and returned objects cannot mutate ledger", async t => {
  const { dir, store } = await fixture(t);
  const second = { ...store, path: path.join(dir, "second.jsonl") };
  const [a, b] = await Promise.all([mintCreditToken(store, { redemptions: 2, stripeRef: "ref" }), mintCreditToken(second, { redemptions: 3, stripeRef: "ref" })]);
  assert.notEqual(a.token, b.token);
  assert.equal(await getFulfillmentToken(store, b.token), null);
  a.remaining_redemptions = 100;
  assert.equal((await getFulfillmentToken(store, a.token))?.remaining_redemptions, 2);
  await assert.rejects(updateFulfillmentToken(store, a), /corrupt_fulfillment_credits/);
});

test("replay conflicts, invalid SKU and fractional/NaN credits fail closed", async t => {
  const { store } = await fixture(t);
  await mintCreditToken(store, { redemptions: 5, stripeRef: "ref" });
  await assert.rejects(mintCreditToken(store, { redemptions: 10, stripeRef: "ref" }), /idempotency_conflict/);
  for (const redemptions of [NaN, 0, -1, 1.5, 101]) await assert.rejects(mintCreditToken(store, { redemptions, stripeRef: "bad" }), /invalid_redemptions/);
  assert.throws(() => parseSkuId(undefined), /invalid_fulfillment_sku/);
  assert.throws(() => parseSkuId("typo"), /invalid_fulfillment_sku/);
});

test("pack and molecule target mismatch never consumes an entitlement", async t => {
  const { store } = await fixture(t);
  const pack = await mintPackToken(store, { packId: "solvents.pbe", stripeRef: "pack" });
  assert.equal((await redeemFulfillmentToken(store, pack.token, "dft.pack", "/pack", { packId: "skala.flagship.v1" })).ok, false);
  assert.equal((await redeemFulfillmentToken(store, pack.token, "dft.pack", "/pack", { packId: "solvents.pbe" })).ok, true);
  const molecule = await mintFulfillmentToken(store, { skuId: "moladt.geometry", stripeRef: "molecule", smiles: "O" });
  assert.equal((await redeemFulfillmentToken(store, molecule.token, "moladt.geometry", "/moladt", { smiles: "CO" })).ok, false);
  assert.equal((await getFulfillmentToken(store, molecule.token))?.used_at_unix, undefined);
});

test("corrupt, partial and symlink ledgers fail closed instead of resetting balances", async t => {
  const { dir, store } = await fixture(t);
  const row = await mintCreditToken(store, { redemptions: 2, stripeRef: "paid" });
  await appendFile(store.path, '{"partial":');
  await assert.rejects(getFulfillmentToken(store, row.token), /incomplete_fulfillment_ledger/);
  await appendFile(store.path, '\n');
  await assert.rejects(mintCreditToken(store, { redemptions: 2, stripeRef: "other" }));
  const alias = path.join(dir, "alias.jsonl");
  await symlink(store.path, alias);
  await assert.rejects(getFulfillmentToken({ ...store, path: alias }, row.token), /unsafe_fulfillment_ledger/);
});
