import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import type { AddressInfo } from "node:net";
import { once } from "node:events";
import path from "node:path";
import os from "node:os";
import { createApp } from "../src/app.js";
import { loadConfig } from "../src/config.js";
import { mintCreditToken, mintPackToken } from "../src/credits.js";
import { getFulfillmentToken } from "../src/fulfillment.js";

async function fixture(t: any, overrides: Partial<ReturnType<typeof loadConfig>> = {}) {
  const dir = await mkdtemp(path.join(os.tmpdir(), "gateway-main-http-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const requestId = "art_1111111111111111", resultId = "art_2222222222222222";
  const encode = (payload: unknown) => ({ location: { Inline: { bytes_hex: Buffer.from(JSON.stringify(payload)).toString("hex") } } });
  await writeFile(path.join(dir, `chem_dft_request.${requestId}.json`), JSON.stringify({ id: requestId, payload: encode({ molecule: { canonical_smiles: "c1ccccc1", molecule_name: "benzene" } }) }));
  await writeFile(path.join(dir, `chem_dft_result.${resultId}.json`), JSON.stringify({ id: resultId, schema_tags: ["chem.dft.result"], parent_artifact_ids: [requestId], payload: encode({ molecule_id: "mol.benzene", functional: "pbe", basis_set: "def2-tzvp" }) }));
  await writeFile(path.join(dir, "chem_dft_result.art_3333333333333333.json"), JSON.stringify({ id: "art_3333333333333333", schema_tags: ["chem.dft.result"], payload: encode({ molecule_id: "MOLADT.OVERNIGHT.skala_adamantane", functional: "Skala-1.1", basis_set: "def2-tzvp" }) }));
  const config = { ...loadConfig(), mode: "live" as const, payTo: "test", dftCacheDir: dir, fulfillmentLogPath: path.join(dir, "tokens.jsonl"), revenueLogPath: path.join(dir, "revenue.jsonl"), jobLogPath: path.join(dir,"jobs.jsonl"), unstoppableApiKey: null, stripeSecretKey: null, operatorFulfillSecret: "operator-test", ...overrides };
  const app = await createApp(config); const server = app.listen(0, "127.0.0.1"); await once(server,"listening");
  t.after(() => new Promise<void>(resolve => { server.closeAllConnections(); server.close(() => resolve()); }));
  return { config, base: `http://127.0.0.1:${(server.address() as AddressInfo).port}`, store: { path: config.fulfillmentLogPath, ttlSeconds: 3600 } };
}

test("live headers fail closed for singles, packs and MolADT; valid tokens retain separate fulfillment", async t => {
  const { base, store } = await fixture(t);
  for (const route of ["/v1/dft/cached?label=benzene", "/v1/dft/packs/aromatics.pbe"]) {
    for (const signature of ["STUB", "forged"]) assert.equal((await fetch(base+route,{ headers: { "PAYMENT-SIGNATURE":signature } })).status,503);
  }
  assert.equal((await fetch(base+"/v1/moladt", { method:"POST",headers:{"Content-Type":"application/json","PAYMENT-SIGNATURE":"forged"},body:JSON.stringify({smiles:"O"}) })).status,503);
  const credit = await mintCreditToken(store,{redemptions:1,stripeRef:"credit"});
  const headers={"PAYMENT-SIGNATURE":`fulfill:${credit.token}`};
  assert.equal((await fetch(base+"/v1/dft/cached?smiles=C1CCCCC1",{headers})).status,404);
  assert.equal((await getFulfillmentToken(store,credit.token))?.remaining_redemptions,1);
  const responses=await Promise.all([fetch(base+"/v1/dft/cached?smiles=c1ccccc1",{headers}),fetch(base+"/v1/dft/cached?smiles=c1ccccc1",{headers})]);
  assert.deepEqual(responses.map(r=>r.status).sort(),[200,402]);
  const pack=await mintPackToken(store,{packId:"aromatics.pbe",stripeRef:"pack"});
  assert.equal((await fetch(base+"/v1/dft/packs/solvents.pbe",{headers:{"PAYMENT-SIGNATURE":`fulfill:${pack.token}`}})).status,402);
  assert.equal((await getFulfillmentToken(store,pack.token))?.used_at_unix,undefined);
});

test("operator rejects unknown/default SKU and malformed credits", async t => {
  const {base}=await fixture(t);
  for(const body of [{stripe_ref:"x"},{sku_id:"typo",stripe_ref:"x"},{sku_id:"dft.credit",redemptions:1.5,stripe_ref:"x"}]) {
    assert.equal((await fetch(base+"/v1/operator/mint-fulfillment",{method:"POST",headers:{"Content-Type":"application/json","X-Operator-Secret":"operator-test"},body:JSON.stringify(body)})).status,400);
  }
});


test("checkout validates targets before payment instructions and premium never gets the classical link", async t => {
  const { base } = await fixture(t, { stripePaymentLinkDftCached: "https://buy.stripe.com/classical-test", stripePaymentLinkMoladt: "https://buy.stripe.com/moladt-test" });
  const checkout = (body: unknown) => fetch(base + "/v1/checkout", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  for (const body of [
    { sku_id: "moladt.geometry" },
    { sku_id: "dft.cached_result" },
    { sku_id: "dft.cached_result", label: "" },
    { sku_id: "dft.cached_result", label: "absent" },
    { sku_id: "dft.cached_result", label: "benzene", smiles: "C1CCCCC1" },
    { sku_id: "dft.cached_result", label: "skala adamantane" },
    { sku_id: "dft.cached_result", label: "x".repeat(513) },
  ]) {
    const response = await checkout(body);
    assert.ok([400, 404].includes(response.status), JSON.stringify(body));
    assert.equal((await response.json()).url, undefined);
  }
  const valid = await checkout({ sku_id: "dft.cached_result", label: "BENZENE" });
  assert.equal(valid.status, 200);
  assert.equal((await valid.json()).url, "https://buy.stripe.com/classical-test");
  const premium = await checkout({ sku_id: "dft.skala_cached", label: "skala adamantane" });
  assert.equal(premium.status, 503);
  assert.equal((await premium.json()).url, undefined);
});
