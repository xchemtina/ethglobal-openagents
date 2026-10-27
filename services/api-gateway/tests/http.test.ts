import test from "node:test";
import assert from "node:assert/strict";
import type { AddressInfo } from "node:net";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { once } from "node:events";
import { gzipSync } from "node:zlib";
import { createApp } from "../src/app.js";
import { loadConfig } from "../src/config.js";

async function serve(config = loadConfig()) {
  const app = await createApp(config);
  const server = app.listen(0, "127.0.0.1");
  await once(server, "listening");
  const base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  return { base, close: () => new Promise<void>((resolve, reject) => { server.closeAllConnections(); server.close(error => error ? reject(error) : resolve()); }) };
}

test("HTTP live gateway refuses forged payments without executing a worker", async () => {
  const gateway = await serve({ ...loadConfig(), mode: "live", cliPath: "/missing/worker" });
  try {
    for (const signature of ["", "STUB", "forged"]) {
      const response = await fetch(`${gateway.base}/v1/moladt`, { method: "POST", headers: { "Content-Type": "application/json", "PAYMENT-SIGNATURE": signature }, body: JSON.stringify({ smiles: "O" }) });
      assert.equal(response.status, 503);
      assert.equal((await response.json()).error, "live_payment_unavailable");
    }
    const cached = await fetch(`${gateway.base}/v1/dft/cached?label=water`, { headers: { "PAYMENT-SIGNATURE": "forged" } });
    assert.equal(cached.status, 503);
  } finally { await gateway.close(); }
});

test("HTTP cache lookup, spoofed limits, JSON errors and failed log writes remain bounded", async () => {
  const dir = await mkdtemp(path.join(os.tmpdir(), "gateway-http-"));
  const config = { ...loadConfig(), mode: "free" as const, revenueLogPath: dir, stripeSecretKey: "test-key", stripePaymentLinkMoladt: null, stripePaymentLinkDftCached: null, revolutPayTo: "operator-test" };
  const gateway = await serve(config);
  try {
    const wrong = await fetch(`${gateway.base}/v1/dft/cached?smiles=C1CCCCC1`);
    assert.equal(wrong.status, 404);
    const ambiguous = await fetch(`${gateway.base}/v1/dft/cached?label=water&smiles=CO`);
    assert.equal(ambiguous.status, 400);
    const failed = await fetch(`${gateway.base}/v1/dft/cached?label=water`);
    assert.equal(failed.status, 500);
    assert.deepEqual(await failed.json(), { error: "internal_error", message: "The request could not be completed." });
    const checkout = await fetch(`${gateway.base}/v1/checkout`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ sku_id: "moladt.geometry", method: "revolut" }) });
    assert.equal(checkout.status, 500);
    assert.equal((await checkout.json()).error, "internal_error");
    const minimum = await fetch(`${gateway.base}/v1/checkout`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ sku_id: "moladt.geometry", method: "stripe_checkout" }) });
    assert.equal(minimum.status, 422);
    assert.equal((await minimum.json()).amount_cents, 1);
    const malformed = await fetch(`${gateway.base}/v1/checkout`, { method: "POST", headers: { "Content-Type": "application/json" }, body: "{bad" });
    assert.equal(malformed.status, 400);
    assert.equal((await malformed.json()).error, "invalid_request");
    assert.equal((await fetch(`${gateway.base}/health`)).status, 200);
  } finally { await gateway.close(); await rm(dir, { recursive: true, force: true }); }
  const limited = await serve({ ...loadConfig(), rateLimitMax: 1 });
  try {
    assert.equal((await fetch(`${limited.base}/health`, { headers: { "X-Forwarded-For": "1.1.1.1" } })).status, 200);
    assert.equal((await fetch(`${limited.base}/health`, { headers: { "X-Forwarded-For": "2.2.2.2" } })).status, 429);
  } finally { await limited.close(); }
});

test("compressed trailing bytes and invalid checkout fields cannot terminate the gateway", async () => {
  const gateway = await serve();
  try {
    // GHSA-27r3-jxmh-p828: an invalid tail must not become an unhandled decoder error.
    const payload = new Uint8Array(Buffer.concat([gzipSync("{}"), Buffer.from("invalid gzip tail")]));
    const compressed = await fetch(`${gateway.base}/v1/checkout`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "Content-Encoding": "gzip" },
      body: payload,
    });
    assert.equal(compressed.status, 400);
    await compressed.arrayBuffer();
    for (const body of [{ sku_id: [] }, { sku_id: null }, { sku_id: "moladt.geometry", method: "unknown" }]) {
      const invalid = await fetch(`${gateway.base}/v1/checkout`, {
        method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body),
      });
      assert.equal(invalid.status, 400);
      const response = await invalid.json();
      assert.equal(response.error, "invalid body");
      assert.ok(Object.keys(response.details.fieldErrors).length > 0);
    }
    assert.equal((await fetch(`${gateway.base}/health`)).status, 200);
  } finally { await gateway.close(); }
});
