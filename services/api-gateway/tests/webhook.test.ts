import test from "node:test";
import assert from "node:assert/strict";
import { createHmac } from "node:crypto";
import { mkdtemp, rm, readFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { handleStripeEvent, processStripeWebhookRequest, type StripeEventLike } from "../src/stripe_webhook.js";

function event(): StripeEventLike { return { id: "evt_test", type: "checkout.session.completed", data: { object: { object: "checkout.session", id: "cs_test", mode: "payment", payment_status: "paid", currency: "usd", amount_total: 500, metadata: { sku_id: "dft.cached_result", label: "water" } } } }; }
async function fixture(t: any) {
  const dir = await mkdtemp(path.join(os.tmpdir(), "webhook-test-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  return { path: path.join(dir, "tokens.jsonl"), ttlSeconds: 3600 };
}

test("signed webhook replay and async event for one checkout issue exactly one token", async t => {
  const store = await fixture(t);
  const rawBody = Buffer.from(JSON.stringify(event()));
  const timestamp = Math.floor(Date.now() / 1000);
  const secret = "test-whsec";
  const signatureHeader = `t=${timestamp},v1=${createHmac("sha256", secret).update(`${timestamp}.`).update(rawBody).digest("hex")}`;
  const results = await Promise.all(Array.from({ length: 10 }, () => processStripeWebhookRequest({ store, rawBody, signatureHeader, webhookSecret: secret })));
  assert.ok(results.every(r => r.status === 200));
  const asyncResult = await handleStripeEvent(store, { ...event(), id: "evt_async", type: "checkout.session.async_payment_succeeded" });
  assert.equal(new Set([...results, asyncResult].map(r => r.token?.token)).size, 1);
  assert.equal((await readFile(store.path, "utf8")).trim().split("\n").length, 1);
  const intent = await handleStripeEvent(store, { ...event(), type: "payment_intent.succeeded" });
  assert.equal(intent.body.ignored, true);
});

test("missing signature, wrong signature, stale timestamp and modified body reject", async t => {
  const store = await fixture(t);
  for (const signatureHeader of [undefined, "t=1,v1=abc", `t=${Math.floor(Date.now()/1000)},v1=${"0".repeat(64)}`]) {
    assert.equal((await processStripeWebhookRequest({ store, rawBody: Buffer.from(JSON.stringify(event())), signatureHeader, webhookSecret: "secret" })).status, 400);
  }
  assert.equal((await processStripeWebhookRequest({ store, rawBody: Buffer.from("{}"), webhookSecret: null })).status, 503);
});

test("missing paid status, invalid metadata, wrong amount/currency and unsupported automatic grants never mint", async t => {
  const store = await fixture(t);
  for (const patch of [ { payment_status: undefined }, { payment_status: "no_payment_required" }, { mode: "subscription" }, { id: undefined }, { amount_total: 1 }, { currency: "eur" }, { metadata: {} }, { metadata: { sku_id: "typo" } }, { metadata: { sku_id: "dft.cached_result" } }, { metadata: { sku_id: "dft.credit", redemptions: "100" } } ]) {
    const value = event(); Object.assign(value.data!.object!, patch);
    const result = await handleStripeEvent(store, value);
    assert.equal(result.ok, false, JSON.stringify(patch));
    assert.equal(result.token, undefined);
  }
  await assert.rejects(readFile(store.path), { code: "ENOENT" });
  const unpaid = event(); unpaid.data!.object!.payment_status = "unpaid";
  assert.equal((await handleStripeEvent(store, unpaid)).body.deferred, true);
});

test("same checkout cannot grant a different product on replay", async t => {
  const store = await fixture(t);
  await handleStripeEvent(store, event());
  const changed = event(); changed.data!.object!.metadata = { sku_id: "dft.cached_result", label: "benzene" };
  assert.equal((await handleStripeEvent(store, changed)).status, 409);
});
