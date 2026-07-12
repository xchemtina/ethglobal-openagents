import cors from "cors";
import express from "express";
import { z } from "zod";
import { buildCatalog } from "./catalog.js";
import { loadConfig } from "./config.js";
import {
  findDftEntry,
  indexItems,
  loadDftCache,
  type DftCacheEntry,
} from "./dft_cache.js";
import { extractArtifactId, runMoladtApi } from "./moladt.js";
import { buildOpenApi } from "./openapi.js";
import {
  createStripeCheckout,
  paymentMethods,
  revolutInstructions,
} from "./payments.js";
import { paymentGate } from "./payment.js";
import { rateLimit } from "./rate_limit.js";
import { appendRevenueEvent } from "./revenue.js";

const config = loadConfig();
const catalog = buildCatalog(config);
const app = express();

/** Loaded once at boot; restart gateway after adding new demo DFT artifacts. */
let dftCache: DftCacheEntry[] = [];
loadDftCache(config.dftCacheDir)
  .then((entries) => {
    dftCache = entries;
    // eslint-disable-next-line no-console
    console.log(
      JSON.stringify({
        event: "dft_cache_loaded",
        dir: config.dftCacheDir,
        count: entries.length,
        labels: entries.map((e) => e.label),
      }),
    );
  })
  .catch((error: unknown) => {
    // eslint-disable-next-line no-console
    console.error(
      JSON.stringify({
        event: "dft_cache_load_failed",
        dir: config.dftCacheDir,
        message: error instanceof Error ? error.message : String(error),
      }),
    );
  });

app.use(
  cors({
    origin: config.corsOrigin === "*" ? true : config.corsOrigin,
  }),
);
app.use(express.json({ limit: "1mb" }));
app.use(
  rateLimit({
    windowMs: config.rateLimitWindowMs,
    max: config.rateLimitMax,
  }),
);
app.use((_req, res, next) => {
  res.setHeader("X-Content-Type-Options", "nosniff");
  res.setHeader("X-Chimia-Gateway", "chimiaclaw-api-gateway/0.1");
  res.setHeader("X-Chimia-Payment-Mode", config.mode);
  next();
});

app.get("/health", (_req, res) => {
  const methods = paymentMethods(config);
  res.json({
    ok: true,
    service: "chimiaclaw-api-gateway",
    mode: config.mode,
    network: config.network,
    pay_to: config.payTo,
    payment_methods: methods.methods.map((m) => ({
      id: m.id,
      status: m.status,
    })),
    deployable_hint: methods.deployable,
  });
});

/** Free discovery surface — agents and the website read this. */
app.get("/v1/catalog", (_req, res) => {
  res.json(catalog);
});

app.get("/v1/payment-methods", (_req, res) => {
  res.json(paymentMethods(config));
});

app.get("/openapi.json", (_req, res) => {
  res.json(buildOpenApi(config));
});

app.get("/v1/openapi.json", (_req, res) => {
  res.json(buildOpenApi(config));
});

const checkoutBody = z.object({
  sku_id: z.enum(["moladt.geometry", "dft.cached_result"]),
  /** stripe = Payment Link if set, else Checkout Session API; revolut = manual */
  method: z.enum(["stripe", "revolut", "stripe_checkout"]).default("stripe"),
  success_url: z.string().url().optional(),
  cancel_url: z.string().url().optional(),
  smiles: z.string().optional(),
  label: z.string().optional(),
});

app.post("/v1/checkout", async (req, res) => {
  const parsed = checkoutBody.safeParse(req.body ?? {});
  if (!parsed.success) {
    res.status(400).json({
      error: "invalid body",
      details: parsed.error.flatten(),
    });
    return;
  }
  const { sku_id, method, smiles, label } = parsed.data;
  const amountCents =
    sku_id === "moladt.geometry"
      ? Math.round(config.moladtPriceMicros / 10_000)
      : Math.round(config.dftCachedPriceMicros / 10_000);
  const priceDisplay =
    sku_id === "moladt.geometry"
      ? config.moladtPriceDisplay
      : config.dftCachedPriceDisplay;
  const productName =
    sku_id === "moladt.geometry"
      ? "ChimiaClaw MolADT geometry"
      : "ChimiaClaw cached DFT result";
  const reference = `chimia-${sku_id}-${Date.now().toString(36)}`;

  if (method === "revolut") {
    const instr = revolutInstructions(config, {
      skuId: sku_id,
      amountDisplay: priceDisplay,
      reference,
    });
    if (!instr.pay_to && !instr.payment_link) {
      res.status(503).json({
        error: "revolut_unconfigured",
        message: "Set REVOLUT_PAY_TO or REVOLUT_PAYMENT_LINK",
      });
      return;
    }
    await appendRevenueEvent(config.revenueLogPath, {
      at_unix: Math.floor(Date.now() / 1000),
      sku_id,
      mode: "revolut_intent",
      amount_usdc_micros:
        sku_id === "moladt.geometry"
          ? config.moladtPriceMicros
          : config.dftCachedPriceMicros,
      price_display: priceDisplay,
      pay_to: config.revolutPayTo ?? "revolut",
      network: "fiat:revolut",
      result_artifact_id: reference,
      smiles,
      label,
      payment_signature_ref: reference,
    });
    res.json({
      ok: true,
      method: "revolut",
      status: "awaiting_transfer",
      instructions: instr,
      next: "Operator confirms transfer, then fulfills SKU offline or via free/stub API.",
    });
    return;
  }

  // Prefer Dashboard Payment Links when configured (simplest for you)
  const paymentLink =
    method === "stripe"
      ? sku_id === "moladt.geometry"
        ? config.stripePaymentLinkMoladt
        : config.stripePaymentLinkDftCached
      : null;

  if (method === "stripe" && paymentLink) {
    await appendRevenueEvent(config.revenueLogPath, {
      at_unix: Math.floor(Date.now() / 1000),
      sku_id,
      mode: "stripe_payment_link",
      amount_usdc_micros:
        sku_id === "moladt.geometry"
          ? config.moladtPriceMicros
          : config.dftCachedPriceMicros,
      price_display: priceDisplay,
      pay_to: "stripe",
      network: "fiat:stripe",
      result_artifact_id: reference,
      smiles,
      label,
      payment_signature_ref: paymentLink,
    });
    res.json({
      ok: true,
      method: "stripe_payment_link",
      url: paymentLink,
      reference,
      next: "Customer pays on Stripe-hosted page. Operator fulfills SKU (or use webhook later).",
    });
    return;
  }

  // API Checkout Session (requires secret key)
  if (!config.stripeSecretKey) {
    res.status(503).json({
      error: "stripe_unconfigured",
      message:
        "Create a Payment Link in Stripe Dashboard and set STRIPE_PAYMENT_LINK_MOLADT / STRIPE_PAYMENT_LINK_DFT_CACHED, or set STRIPE_SECRET_KEY for Checkout API. Or method=revolut / x402 stub.",
      dashboard_create:
        "https://dashboard.stripe.com/acct_1PsRXKLWiL2XLIJa/payment-links/create",
      payment_methods: "/v1/payment-methods",
    });
    return;
  }
  try {
    const success =
      parsed.data.success_url ??
      `${config.publicBaseUrl}/v1/catalog?checkout=success`;
    const cancel =
      parsed.data.cancel_url ??
      `${config.publicBaseUrl}/v1/catalog?checkout=cancel`;
    const session = await createStripeCheckout(config, {
      skuId: sku_id,
      amountCents: Math.max(amountCents, 50), // Stripe min often $0.50 for cards
      productName,
      successUrl: success,
      cancelUrl: cancel,
      metadata: {
        smiles: smiles ?? "",
        label: label ?? "",
        reference,
      },
    });
    await appendRevenueEvent(config.revenueLogPath, {
      at_unix: Math.floor(Date.now() / 1000),
      sku_id,
      mode: "stripe_checkout",
      amount_usdc_micros:
        sku_id === "moladt.geometry"
          ? config.moladtPriceMicros
          : config.dftCachedPriceMicros,
      price_display: priceDisplay,
      pay_to: "stripe",
      network: "fiat:stripe",
      result_artifact_id: session.id,
      smiles,
      label,
      payment_signature_ref: session.id,
    });
    res.json({
      ok: true,
      method: "stripe_checkout",
      checkout_session_id: session.id,
      url: session.url,
    });
  } catch (error) {
    res.status(502).json({
      error: "stripe_failed",
      message: error instanceof Error ? error.message : String(error),
    });
  }
});

app.get("/.well-known/x402", (_req, res) => {
  res.json({
    protocol: "x402",
    version: 1,
    provider: "ChimiaDAO / ChimiaClaw",
    catalog: "/v1/catalog",
    mode: config.mode,
    network: config.network,
    pay_to: config.payTo,
    facilitator_url: config.facilitatorUrl,
    skus: catalog.skus.map((s) => ({
      sku_id: s.sku_id,
      path: s.path,
      method: s.http_method,
      price: s.price_display,
      status: s.status,
    })),
  });
});

const moladtBody = z.object({
  smiles: z.string().min(1).max(512),
  no_worker: z.boolean().optional(),
});

app.post(
  "/v1/moladt",
  paymentGate(config, {
    skuId: "moladt.geometry",
    priceDisplay: config.moladtPriceDisplay,
    priceMicros: config.moladtPriceMicros,
    description: "SMILES to signed chem.molecule.adt",
    resourcePath: "/v1/moladt",
  }),
  async (req, res) => {
    const parsed = moladtBody.safeParse(req.body ?? {});
    if (!parsed.success) {
      res.status(400).json({
        error: "invalid body",
        expected: { smiles: "string", no_worker: "optional boolean" },
        details: parsed.error.flatten(),
      });
      return;
    }

    try {
      const result = await runMoladtApi(config, parsed.data.smiles, {
        noWorker: parsed.data.no_worker ?? false,
      });
      const artifactId = extractArtifactId(result);
      const x402 = res.locals.x402 as
        | { mode: string; paymentSignature?: string | null }
        | undefined;

      await appendRevenueEvent(config.revenueLogPath, {
        at_unix: Math.floor(Date.now() / 1000),
        sku_id: "moladt.geometry",
        mode: config.mode,
        amount_usdc_micros: config.moladtPriceMicros,
        price_display: config.moladtPriceDisplay,
        pay_to: config.payTo,
        network: config.network,
        result_artifact_id: artifactId,
        smiles: parsed.data.smiles,
        payment_signature_ref: x402?.paymentSignature ?? null,
      });

      res.json({
        ok: true,
        sku_id: "moladt.geometry",
        settlement: {
          method: "X402HttpPayment",
          mode: config.mode,
          amount_usdc_micros: config.moladtPriceMicros,
          price_display: config.moladtPriceDisplay,
          pay_to: config.payTo,
          network: config.network,
        },
        result,
        result_artifact_id: artifactId,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const status = message.includes("not in curated library") ? 422 : 500;
      res.status(status).json({
        error: "moladt execution failed",
        message,
        hint: "Build the CLI with `cargo build -p chimiaclaw-cli` and set CHIMIACLAW_CLI",
      });
    }
  },
);

app.post("/v1/literature", (_req, res) => {
  res.status(501).json({
    error: "coming_soon",
    sku_id: "literature.synthesis",
    message: "Literature SKU is catalogued; paid execution lands next sprint.",
    catalog: "/v1/catalog",
  });
});

/** Free discovery — which signed DFT results are sellable from cache. */
app.get("/v1/dft/index", (_req, res) => {
  res.json({
    ok: true,
    sku_id: "dft.cached_result",
    cache_dir: config.dftCacheDir,
    count: dftCache.length,
    price_display: config.dftCachedPriceDisplay,
    price_usdc_micros: config.dftCachedPriceMicros,
    how_to_buy:
      "GET /v1/dft/cached?label=<label|smiles|artifact_id> with payment headers",
    items: indexItems(dftCache),
  });
});

app.get(
  "/v1/dft/cached",
  paymentGate(config, {
    skuId: "dft.cached_result",
    priceDisplay: config.dftCachedPriceDisplay,
    priceMicros: config.dftCachedPriceMicros,
    description: "Cached signed chem.dft.result by label/SMILES/id",
    resourcePath: "/v1/dft/cached",
  }),
  async (req, res) => {
    const q =
      (typeof req.query.label === "string" && req.query.label) ||
      (typeof req.query.smiles === "string" && req.query.smiles) ||
      (typeof req.query.id === "string" && req.query.id) ||
      (typeof req.query.q === "string" && req.query.q) ||
      "";

    if (!q) {
      res.status(400).json({
        error: "missing query",
        expected: {
          label: "water | methanol | benzene | ...",
          smiles: "O | CO | c1ccccc1 | ...",
          id: "art_...",
        },
        free_index: "/v1/dft/index",
      });
      return;
    }

    const entry = findDftEntry(dftCache, q);
    if (!entry) {
      res.status(404).json({
        error: "not_in_cache",
        query: q,
        available: dftCache.map((e) => e.label),
        free_index: "/v1/dft/index",
      });
      return;
    }

    const x402 = res.locals.x402 as
      | { mode: string; paymentSignature?: string | null }
      | undefined;

    await appendRevenueEvent(config.revenueLogPath, {
      at_unix: Math.floor(Date.now() / 1000),
      sku_id: "dft.cached_result",
      mode: config.mode,
      amount_usdc_micros: config.dftCachedPriceMicros,
      price_display: config.dftCachedPriceDisplay,
      pay_to: config.payTo,
      network: config.network,
      result_artifact_id: entry.artifact_id,
      smiles: entry.smiles ?? undefined,
      label: entry.label,
      payment_signature_ref: x402?.paymentSignature ?? null,
    });

    res.json({
      ok: true,
      sku_id: "dft.cached_result",
      settlement: {
        method: "X402HttpPayment",
        mode: config.mode,
        amount_usdc_micros: config.dftCachedPriceMicros,
        price_display: config.dftCachedPriceDisplay,
        pay_to: config.payTo,
        network: config.network,
      },
      query: q,
      label: entry.label,
      smiles: entry.smiles,
      molecule_id: entry.molecule_id,
      summary: {
        functional: entry.functional,
        basis_set: entry.basis_set,
        energy_hartree: entry.energy_hartree,
        gap_ev: entry.gap_ev,
        dipole_debye: entry.dipole_debye,
        topic: entry.topic,
      },
      result_payload: entry.result_payload,
      request_payload: entry.request_payload,
      artifact: entry.artifact,
      result_artifact_id: entry.artifact_id,
      notes: [
        "Returned artifact is the signed chem.dft.result already computed offline.",
        "Cube files are content-addressed by SHA-256 inside orbital_densities[]; bytes are not re-hosted here.",
        "This is a paid cache lookup, not a live SCF run.",
      ],
    });
  },
);

app.post("/v1/dft/live", (_req, res) => {
  // Live SCF is deliberately not free-run on the public cashier.
  // Path: CHIMIACLAW_DFT_COMMAND → chimiaclaw-dft-modal --mode modal
  // (see docs/MODAL_DFT.md). Enable only after Modal deploy + spend caps.
  res.status(501).json({
    error: "operator_gated",
    sku_id: "dft.live_small",
    message:
      "Live DFT is not free-run on the public API. Wire Modal via skills/scienceclaw-port/workers/dft_modal, set CHIMIACLAW_DFT_LIVE_OPERATOR=1, then enable this route behind payment + caps.",
    docs: "docs/MODAL_DFT.md",
    use_instead: {
      cached: "GET /v1/dft/cached?label=water",
      index: "GET /v1/dft/index",
    },
    catalog: "/v1/catalog",
  });
});

app.listen(config.port, config.host, () => {
  // eslint-disable-next-line no-console
  console.log(
    JSON.stringify({
      event: "gateway_listen",
      url: `http://${config.host}:${config.port}`,
      mode: config.mode,
      network: config.network,
      pay_to: config.payTo,
      catalog: "/v1/catalog",
      moladt: "POST /v1/moladt",
      dft_index: "GET /v1/dft/index",
      dft_cached: "GET /v1/dft/cached?label=water",
    }),
  );
});
