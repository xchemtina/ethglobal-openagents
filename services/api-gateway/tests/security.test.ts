import test from "node:test";
import assert from "node:assert/strict";
import type { Request, Response } from "express";
import { paymentGate } from "../src/payment.js";
import { loadConfig } from "../src/config.js";
import { paymentMethods, createStripeCheckout } from "../src/payments.js";
import { buildCatalog } from "../src/catalog.js";
import { rateLimit } from "../src/rate_limit.js";
import { findDftEntry, loadDftCache, type DftCacheEntry } from "../src/dft_cache.js";

function response() {
  const state = { status: 200, body: null as any, headers: {} as Record<string, string>, locals: {} as Record<string, unknown> };
  const res = {
    locals: state.locals,
    setHeader(name: string, value: string) { state.headers[name] = value; return res; },
    status(status: number) { state.status = status; return res; },
    json(body: unknown) { state.body = body; return res; },
  } as unknown as Response;
  return { state, res };
}
const paymentOptions = { skuId: "moladt.geometry", priceDisplay: "$0.01", priceMicros: 10000, description: "SMILES → MolADT", resourcePath: "/v1/moladt" };

test("live mode refuses missing, arbitrary, and mixed-case stub signatures", () => {
  const config = { ...loadConfig(), mode: "live" as const };
  for (const signature of [undefined, "", "stub", "STUB", "StUb", "arbitrary-unverified-payload"]) {
    const { state, res } = response();
    let dispatched = false;
    paymentGate(config, paymentOptions)({ header: () => signature } as unknown as Request, res, () => { dispatched = true; });
    assert.equal(dispatched, false);
    assert.equal(state.status, 503);
    assert.equal(state.body.error, "live_payment_unavailable");
    assert.equal(state.locals.x402, undefined);
  }
  const methods = paymentMethods(config);
  assert.equal(methods.agent_default, "unavailable");
  assert.equal(methods.methods[0].status, "unavailable");
  assert.equal(methods.methods[0].details.live_verification, false);
  assert.equal(buildCatalog(config).skus.find(s => s.sku_id === "moladt.geometry")?.status, "unavailable");
});

test("free/stub modes are explicit and unpaid challenge is ASCII-safe", () => {
  for (const [mode, signature, expected] of [["free", undefined, true], ["stub", "stub", true], ["stub", "STUB", true], ["stub", "no", false]] as const) {
    const { state, res } = response();
    let dispatched = false;
    paymentGate({ ...loadConfig(), mode }, paymentOptions)({ header: () => signature } as unknown as Request, res, () => { dispatched = true; });
    assert.equal(dispatched, expected);
    if (!expected) {
      assert.equal(state.status, 402);
      assert.match(state.headers["PAYMENT-REQUIRED"], /^[\x20-\x7e]*$/);
    }
  }
});

function request(ip: string, forwarded = ip) {
  return { ip, socket: { remoteAddress: ip }, headers: { "x-forwarded-for": forwarded } } as unknown as Request;
}

test("rate limit ignores spoofed forwarded headers and expires exactly at the deadline", () => {
  let now = 1000;
  const limiter = rateLimit({ max: 1, windowMs: 1000, now: () => now });
  const first = response();
  let calls = 0;
  limiter(request("127.0.0.1", "fake-a"), first.res, () => { calls++; });
  const second = response();
  limiter(request("127.0.0.1", "fake-b"), second.res, () => { calls++; });
  assert.equal(calls, 1);
  assert.equal(second.state.status, 429);
  assert.equal(second.state.headers["Retry-After"], "1");
  now = 2000;
  limiter(request("127.0.0.1"), response().res, () => { calls++; });
  assert.equal(calls, 2);
});

test("rate limiter bounds active identities and reclaims expired buckets", () => {
  let now = 0;
  const limiter = rateLimit({ max: 2, windowMs: 1000, maxKeys: 2, now: () => now });
  let accepted = 0;
  for (const ip of ["a", "b", "c"]) limiter(request(ip), response().res, () => { accepted++; });
  assert.equal(accepted, 2);
  const blocked = response();
  limiter(request("d"), blocked.res, () => assert.fail("must reject over-capacity identity"));
  assert.equal(blocked.state.body.error, "rate_limit_capacity");
  now = 1000;
  limiter(request("d"), response().res, () => { accepted++; });
  assert.equal(accepted, 3);
  for (const max of [0, -1, NaN, Infinity, 0.5]) assert.throws(() => rateLimit({ max, windowMs: 1000 }));
});

test("configuration rejects nonpositive and nonfinite limits", () => {
  const original = process.env.MOLADT_TIMEOUT_MS;
  try {
    for (const value of ["0", "-1", "NaN", "Infinity", "0.1"]) {
      process.env.MOLADT_TIMEOUT_MS = value;
      assert.throws(loadConfig, /MOLADT_TIMEOUT_MS/);
    }
  } finally {
    if (original === undefined) delete process.env.MOLADT_TIMEOUT_MS;
    else process.env.MOLADT_TIMEOUT_MS = original;
  }
});

test("SMILES preserve case, charges and bond syntax; labels remain friendly", () => {
  const benzene = { artifact_id: "art_0123456789abcdef", molecule_id: "MOL.BENZENE", label: "benzene", aliases: ["benzene", "c6h6", "c1ccccc1"], smiles: "c1ccccc1" } as DftCacheEntry;
  assert.equal(findDftEntry([benzene], "C1CCCCC1"), null);
  assert.equal(findDftEntry([benzene], "c1ccccc1"), benzene);
  assert.equal(findDftEntry([benzene], "BENZENE"), benzene);
  assert.equal(findDftEntry([benzene], "benzene", "smiles"), null);
  assert.equal(findDftEntry([benzene], "mol.benzene", "id"), null);
  for (const smiles of ["[O-]", "C-C", "C/C=C/C"]) {
    const entry = { ...benzene, label: "example", aliases: [smiles], smiles };
    assert.equal(findDftEntry([entry], smiles), entry);
    assert.equal(findDftEntry([entry], smiles.replace(/[-/]/g, "")), null);
  }
});

test("real cache loader keeps SMILES outside normalized human aliases", async () => {
  const entries = await loadDftCache("../../demo/dft");
  assert.ok(entries.length >= 6);
  const benzene = entries.find(e => e.label === "benzene");
  assert.ok(benzene);
  assert.equal(findDftEntry(entries, "C1CCCCC1"), null);
  assert.equal(findDftEntry(entries, "c1ccccc1", "smiles"), benzene);
  for (const entry of entries) if (entry.smiles) assert.ok(!entry.aliases.includes(entry.smiles));
});

test("Checkout API rejects subminimum prices before contacting Stripe", async () => {
  await assert.rejects(createStripeCheckout({ ...loadConfig(), stripeSecretKey: "test-key" }, {
    skuId: "moladt.geometry", amountCents: 1, productName: "MolADT", successUrl: "https://example.com/success", cancelUrl: "https://example.com/cancel",
  }), /checkout_amount_below_minimum/);
});
