/**
 * Local smoke test (no funds):
 * 1. GET /v1/catalog
 * 2. POST /v1/moladt without payment → expect 402 in stub mode
 * 3. POST /v1/moladt with PAYMENT-SIGNATURE: stub → expect signed artifact
 * 4. GET /v1/dft/index → free label list
 * 5. GET /v1/dft/cached unpaid → 402 in stub
 * 6. GET /v1/dft/cached?label=water with stub payment → signed result
 *
 * Usage:
 *   X402_MODE=stub CHIMIACLAW_CLI=../../target/debug/chimiaclaw-cli npm run smoke
 * (gateway must already be running on PORT)
 */

const base = process.env.SMOKE_BASE_URL ?? "http://127.0.0.1:4021";

async function main(): Promise<void> {
  const catalogRes = await fetch(`${base}/v1/catalog`);
  if (!catalogRes.ok) {
    throw new Error(`catalog failed: ${catalogRes.status}`);
  }
  const catalog = (await catalogRes.json()) as {
    skus: Array<{ sku_id: string; status: string }>;
  };
  console.log("catalog skus:", catalog.skus.length);
  const dftSku = catalog.skus.find((s) => s.sku_id === "dft.cached_result");
  if (!dftSku || dftSku.status !== "live") {
    throw new Error("dft.cached_result should be live in catalog");
  }

  const unpaid = await fetch(`${base}/v1/moladt`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ smiles: "O", no_worker: true }),
  });
  console.log("moladt unpaid status:", unpaid.status);
  if (unpaid.status !== 402 && unpaid.status !== 200) {
    throw new Error(`unexpected unpaid status ${unpaid.status}`);
  }

  const paid = await fetch(`${base}/v1/moladt`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "PAYMENT-SIGNATURE": "stub",
    },
    body: JSON.stringify({ smiles: "O", no_worker: true }),
  });
  const paidBody = (await paid.json()) as {
    ok?: boolean;
    result_artifact_id?: string;
    error?: string;
    message?: string;
  };
  console.log("moladt paid status:", paid.status);
  if (!paid.ok) {
    throw new Error(
      `paid moladt failed: ${paidBody.error ?? paid.status} ${paidBody.message ?? ""}`,
    );
  }
  console.log("moladt result_artifact_id:", paidBody.result_artifact_id);

  const indexRes = await fetch(`${base}/v1/dft/index`);
  if (!indexRes.ok) {
    throw new Error(`dft index failed: ${indexRes.status}`);
  }
  const indexBody = (await indexRes.json()) as {
    count: number;
    items: Array<{ label: string }>;
  };
  console.log("dft index count:", indexBody.count);
  if (indexBody.count < 1) {
    throw new Error("dft cache is empty — set DFT_CACHE_DIR to demo/dft");
  }

  const dftUnpaid = await fetch(`${base}/v1/dft/cached?label=water`);
  console.log("dft unpaid status:", dftUnpaid.status);
  if (dftUnpaid.status !== 402 && dftUnpaid.status !== 200) {
    throw new Error(`unexpected dft unpaid status ${dftUnpaid.status}`);
  }

  const dftPaid = await fetch(`${base}/v1/dft/cached?label=water`, {
    headers: { "PAYMENT-SIGNATURE": "stub" },
  });
  const dftBody = (await dftPaid.json()) as {
    ok?: boolean;
    result_artifact_id?: string;
    label?: string;
    error?: string;
  };
  console.log("dft paid status:", dftPaid.status);
  if (!dftPaid.ok || !dftBody.ok) {
    throw new Error(
      `paid dft failed: ${dftBody.error ?? dftPaid.status}`,
    );
  }
  console.log(
    "dft result_artifact_id:",
    dftBody.result_artifact_id,
    "label:",
    dftBody.label,
  );

  // Agent-buyable path: resolve → packs → jobs
  const resolveRes = await fetch(`${base}/v1/dft/resolve?q=water`);
  const resolveBody = (await resolveRes.json()) as {
    status?: string;
    buy?: { path?: string };
  };
  console.log("resolve water:", resolveBody.status);
  if (resolveBody.status !== "in_cache") {
    throw new Error("water should resolve as in_cache");
  }

  const packsRes = await fetch(`${base}/v1/dft/packs`);
  if (!packsRes.ok) throw new Error(`packs list failed: ${packsRes.status}`);
  const packsBody = (await packsRes.json()) as {
    packs: Array<{ pack_id: string; status: string; member_count_available: number }>;
  };
  console.log("packs:", packsBody.packs.length);
  const buyable = packsBody.packs.find((p) => p.member_count_available > 0);
  if (buyable) {
    const packUnpaid = await fetch(`${base}/v1/dft/packs/${buyable.pack_id}`);
    console.log("pack unpaid:", packUnpaid.status);
    const packPaid = await fetch(`${base}/v1/dft/packs/${buyable.pack_id}`, {
      headers: { "PAYMENT-SIGNATURE": "stub" },
    });
    const packJson = (await packPaid.json()) as {
      ok?: boolean;
      results?: unknown[];
    };
    console.log("pack paid:", packPaid.status, "results:", packJson.results?.length);
    if (!packPaid.ok || !packJson.ok) {
      throw new Error(`pack buy failed for ${buyable.pack_id}`);
    }
  }

  const jobRes = await fetch(`${base}/v1/dft/jobs`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ smiles: "CCO", method: "pbe", basis: "def2-tzvp" }),
  });
  const jobBody = (await jobRes.json()) as {
    job?: { job_id: string; status: string };
  };
  console.log("job ethanol:", jobBody.job?.status, jobBody.job?.job_id);
  if (!jobBody.job?.job_id) {
    throw new Error("job create failed");
  }

  const creditsRes = await fetch(`${base}/v1/dft/credits`);
  if (!creditsRes.ok) throw new Error("credits endpoint failed");
  console.log("smoke ok (agent loop)");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
