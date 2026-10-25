import cors from "cors";
import express from "express";
import { z } from "zod";
import { buildCatalog } from "./catalog.js";
import type { GatewayConfig } from "./config.js";
import { asyncRoute, errorHandler } from "./http_errors.js";
import {
  creditBundlePriceMicros,
  mintCreditToken,
  mintPackToken,
  PACK_SKU,
} from "./credits.js";
import {
  findDftEntry,
  indexItems,
  loadDftCache,
  type DftCacheEntry,
} from "./dft_cache.js";
import {
  getFulfillmentToken,
  mintFulfillmentToken,
  parseSkuId,
  publicFulfillmentView,
  type FulfillmentStore,
} from "./fulfillment.js";
import {
  createDftJob,
  getDftJob,
  listRecentJobs,
  resolveQuery,
  type JobStore,
} from "./jobs.js";
import { extractArtifactId, runMoladtApi, MoladtExecutionError } from "./moladt.js";
import { buildOpenApi } from "./openapi.js";
import {
  collectPackEntries,
  getPackDefinition,
  listResolvedPacks,
  PACK_DEFINITIONS,
  resolvePack,
} from "./packs.js";
import {
  createStripeCheckout,
  paymentMethods,
  resolveUnstoppableWallet,
  revolutInstructions,
  unstoppableInstructions,
} from "./payments.js";
import { authorizePayment } from "./payment.js";
import { rateLimit } from "./rate_limit.js";
import { appendRevenueEvent } from "./revenue.js";
import { processStripeWebhookRequest } from "./stripe_webhook.js";

export async function createApp(config: GatewayConfig) {
const catalog = buildCatalog(config);
const fulfillmentStore: FulfillmentStore = {
  path: config.fulfillmentLogPath,
  ttlSeconds: config.fulfillmentTtlSeconds,
};
const jobStore: JobStore = { path: config.jobLogPath };
const app = express();

const dftCache: DftCacheEntry[] = await loadDftCache(config.dftCacheDir);

app.use(
  cors({
    origin: config.corsOrigin === "*" ? true : config.corsOrigin,
  }),
);

// Stripe needs the raw body for signature verification — register before json().
app.post(
  "/v1/webhooks/stripe",
  express.raw({ type: "application/json", limit: "1mb" }),
  asyncRoute(async (req, res) => {
    const rawBody = Buffer.isBuffer(req.body)
      ? req.body
      : Buffer.from(typeof req.body === "string" ? req.body : "", "utf8");
    const result = await processStripeWebhookRequest({
      rawBody,
      signatureHeader: req.header("stripe-signature") ?? undefined,
      webhookSecret: config.stripeWebhookSecret,
      store: fulfillmentStore,
      prices: {
        "moladt.geometry": Math.round(config.moladtPriceMicros / 10000),
        "dft.cached_result": Math.round(config.dftCachedPriceMicros / 10000),
        "dft.skala_cached": Math.round(config.dftSkalaPriceMicros / 10000),
      },
    });

    // The durable fulfillment ledger is the payment receipt. Replayed webhook
    // deliveries must not append additional apparent revenue events.

    res.status(result.status).json(result.body);
  }),
);

app.use(express.json({ limit: "1mb" }));
app.use(
  rateLimit({
    windowMs: config.rateLimitWindowMs,
    max: config.rateLimitMax,
    maxKeys: config.rateLimitMaxKeys,
  }),
);
app.use((_req, res, next) => {
  res.setHeader("X-Content-Type-Options", "nosniff");
  res.setHeader("X-Chimia-Gateway", "chimiaclaw-api-gateway/0.1");
  res.setHeader("X-Chimia-Payment-Mode", config.mode);
  next();
});

app.get("/health", asyncRoute(async (_req, res) => {
  const ud = await resolveUnstoppableWallet(config);
  const methods = paymentMethods(config, ud);
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
    unstoppable_domain: config.udDomain,
    stripe_webhook: config.stripeWebhookSecret ? "configured" : "unconfigured",
    deployable_hint: methods.deployable,
  });
}));

/** Lookup a Stripe-issued fulfillment token (no secrets beyond the token itself). */
app.get("/v1/fulfillment/:token", asyncRoute(async (req, res) => {
  const token = req.params.token?.trim() ?? "";
  if (!token.startsWith("ft_")) {
    res.status(400).json({ error: "invalid_token" });
    return;
  }
  const row = await getFulfillmentToken(fulfillmentStore, token);
  if (!row) {
    res.status(404).json({ error: "not_found" });
    return;
  }
  res.json({ ok: true, fulfillment: publicFulfillmentView(row) });
}));

/**
 * Operator mint after Stripe / USDC payment (no webhook required).
 * Header: X-Operator-Secret: $OPERATOR_FULFILL_SECRET
 * Body: {
 *   sku_id: moladt.geometry | dft.cached_result | dft.pack | dft.credit,
 *   label?, smiles?, pack_id?, redemptions?, stripe_ref?
 * }
 */
app.post("/v1/operator/mint-fulfillment", asyncRoute(async (req, res) => {
  if (!config.operatorFulfillSecret) {
    res.status(503).json({
      error: "operator_mint_disabled",
      message: "Set OPERATOR_FULFILL_SECRET on the gateway to enable.",
    });
    return;
  }
  const secret = req.header("x-operator-secret") ?? "";
  if (secret !== config.operatorFulfillSecret) {
    res.status(401).json({ error: "unauthorized" });
    return;
  }
  const parsed = z.object({
    sku_id: z.enum(["moladt.geometry", "dft.cached_result", "dft.skala_cached", "dft.pack", "dft.credit"]),
    stripe_ref: z.string().min(1).max(200), label: z.string().min(1).max(512).optional(),
    smiles: z.string().min(1).max(512).optional(), pack_id: z.string().min(1).max(100).optional(),
    redemptions: z.number().int().min(1).max(100).optional(),
  }).safeParse(req.body);
  if (!parsed.success) { res.status(400).json({ error: "invalid_mint_request" }); return; }
  const body = parsed.data;
  const skuId = parseSkuId(body.sku_id);
  const stripeRef = body.stripe_ref.trim();
  if (!stripeRef) { res.status(400).json({ error: "payment_reference_required" }); return; }

  let token;
  if (skuId === "dft.credit") {
    const n = body.redemptions;
    if (n === undefined) { res.status(400).json({ error: "redemptions_required" }); return; }
    token = await mintCreditToken(fulfillmentStore, {
      redemptions: n,
      stripeRef,
    });
  } else if (skuId === "dft.pack") {
    const packId = body.pack_id?.trim();
    if (!packId || !getPackDefinition(packId)) {
      res.status(400).json({
        error: "pack_id_required",
        known: listResolvedPacks(dftCache, config.dftCachedPriceMicros).map(
          (p) => p.pack_id,
        ),
      });
      return;
    }
    token = await mintPackToken(fulfillmentStore, { packId, stripeRef });
  } else {
    token = await mintFulfillmentToken(fulfillmentStore, {
      skuId,
      stripeRef,
      label: body.label?.trim() || undefined,
      smiles: body.smiles?.trim() || undefined,
    });
  }

  const amountMicros =
    token.sku_id === "moladt.geometry"
      ? config.moladtPriceMicros
      : token.sku_id === "dft.skala_cached"
        ? config.dftSkalaPriceMicros
        : token.sku_id === "dft.credit"
          ? creditBundlePriceMicros(
              config.dftCachedPriceMicros,
              token.remaining_redemptions ?? 5,
              config.creditDiscount,
            )
          : config.dftCachedPriceMicros;

  await appendRevenueEvent(config.revenueLogPath, {
    at_unix: Math.floor(Date.now() / 1000),
    sku_id: token.sku_id,
    mode: "operator_mint_fulfillment",
    amount_usdc_micros: amountMicros,
    price_display:
      token.sku_id === "moladt.geometry"
        ? config.moladtPriceDisplay
        : token.sku_id === "dft.skala_cached"
          ? config.dftSkalaPriceDisplay
          : token.sku_id === "dft.credit"
            ? `${token.remaining_redemptions} credits`
            : config.dftCachedPriceDisplay,
    pay_to: "stripe",
    network: "fiat:stripe",
    result_artifact_id: token.token,
    smiles: token.smiles,
    label: token.label ?? token.pack_id,
    payment_signature_ref: token.stripe_ref,
  });
  res.json({
    ok: true,
    fulfillment: publicFulfillmentView(token),
    redeem: {
      header: `PAYMENT-SIGNATURE: fulfill:${token.token}`,
      dft_example:
        token.sku_id === "dft.skala_cached"
          ? token.label
            ? `GET /v1/dft/skala?label=${encodeURIComponent(token.label)}`
            : "GET /v1/dft/skala?label=skala%20adamantane"
          : token.label
            ? `GET /v1/dft/cached?label=${encodeURIComponent(token.label)}`
            : "GET /v1/dft/cached?label=water",
      pack_example: token.pack_id
        ? `GET /v1/dft/packs/${token.pack_id}`
        : "GET /v1/dft/packs/solvents.pbe",
      credit_note:
        token.sku_id === "dft.credit"
          ? `Multi-use: ${token.remaining_redemptions} credits (classical=1, Skala=4)`
          : undefined,
      moladt_example: "POST /v1/moladt",
    },
  });
}));

/** Free discovery surface — agents and the website read this. */
app.get("/v1/catalog", (_req, res) => {
  res.json(catalog);
});

app.get("/v1/payment-methods", asyncRoute(async (_req, res) => {
  const ud = await resolveUnstoppableWallet(config);
  res.json(paymentMethods(config, ud));
}));

/** Compact storefront snapshot for the website / operator dashboard. */
app.get("/v1/storefront", asyncRoute(async (_req, res) => {
  const ud = await resolveUnstoppableWallet(config);
  const methods = paymentMethods(config, ud);
  const stripe = methods.methods.find((m) => m.id === "stripe");
  const unstoppable = methods.methods.find((m) => m.id === "unstoppable");
  res.json({
    ok: true,
    service: "chimiaclaw-api-gateway",
    public_base_url: config.publicBaseUrl,
    mode: config.mode,
    human_rails: ["stripe", "unstoppable"],
    stripe_status: stripe?.status ?? "unconfigured",
    unstoppable_status: unstoppable?.status ?? "unconfigured",
    unstoppable: {
      domain: config.udDomain,
      pay_to_address: ud.evm_address,
      token: config.udToken,
      network: config.udPreferredNetwork,
      profile_url: ud.profile_url,
    },
    prices: {
      moladt: config.moladtPriceDisplay,
      dft_cached: config.dftCachedPriceDisplay,
      dft_skala: config.dftSkalaPriceDisplay,
    },
    skus: catalog.skus
      .filter((s) => s.status === "live")
      .map((s) => ({
        sku_id: s.sku_id,
        title: s.title,
        price_display: s.price_display,
        path: s.path,
        method: s.http_method,
      })),
    dft_cache: {
      count: dftCache.length,
      classical: dftCache.filter(
        (e) =>
          !e.label.includes("skala") &&
          !(e.functional || "").toLowerCase().includes("skala"),
      ).length,
      skala: dftCache.filter(
        (e) =>
          e.label.includes("skala") ||
          (e.functional || "").toLowerCase().includes("skala"),
      ).length,
      labels: dftCache.map((e) => e.label).sort(),
      free_index: "/v1/dft/index",
      free_skala_index: "/v1/dft/index?tier=skala",
      buy_classical: "GET /v1/dft/cached?label=<label>",
      buy_skala: "GET /v1/dft/skala?label=<skala-…>",
      resolve: "GET /v1/dft/resolve?q=<label|smiles>",
      packs: "GET /v1/dft/packs",
    },
    checkout:
      "POST /v1/checkout { sku_id: moladt.geometry|dft.cached_result|dft.skala_cached, method: 'stripe' | 'unstoppable' }",
    agent_loop: [
      "GET /v1/dft/resolve?q=…",
      "GET /v1/dft/packs · GET /v1/dft/credits",
      "buy path from resolve (classical $5 or Skala $20)",
      "miss → POST /v1/dft/jobs",
    ],
    fulfill_after_pay: {
      operator_mint: "POST /v1/operator/mint-fulfillment",
      webhook: "POST /v1/webhooks/stripe",
      redeem_header: "PAYMENT-SIGNATURE: fulfill:ft_…",
      credit_note: "classical=1 credit · Skala-1.1=4 credits",
    },
    honesty: methods.honesty,
  });
}));

app.get("/openapi.json", (_req, res) => {
  res.json(buildOpenApi(config));
});

app.get("/v1/openapi.json", (_req, res) => {
  res.json(buildOpenApi(config));
});

const checkoutBody = z.object({
  sku_id: z.enum([
    "moladt.geometry",
    "dft.cached_result",
    "dft.skala_cached",
  ]),
  /** stripe | unstoppable (chimiadao.dao) | revolut | stripe_checkout */
  method: z
    .enum(["stripe", "revolut", "stripe_checkout", "unstoppable"])
    .default("stripe"),
  success_url: z.string().url().optional(),
  cancel_url: z.string().url().optional(),
  smiles: z.string().trim().min(1).max(512).optional(),
  label: z.string().trim().min(1).max(512).optional(),
});

function checkoutPricing(sku_id: string): {
  amountCents: number;
  amountMicros: number;
  priceDisplay: string;
  productName: string;
} {
  if (sku_id === "moladt.geometry") {
    return {
      amountCents: Math.round(config.moladtPriceMicros / 10_000),
      amountMicros: config.moladtPriceMicros,
      priceDisplay: config.moladtPriceDisplay,
      productName: "ChimiaClaw MolADT geometry",
    };
  }
  if (sku_id === "dft.skala_cached") {
    return {
      amountCents: Math.round(config.dftSkalaPriceMicros / 10_000),
      amountMicros: config.dftSkalaPriceMicros,
      priceDisplay: config.dftSkalaPriceDisplay,
      productName: "ChimiaClaw Skala-1.1 DFT result",
    };
  }
  return {
    amountCents: Math.round(config.dftCachedPriceMicros / 10_000),
    amountMicros: config.dftCachedPriceMicros,
    priceDisplay: config.dftCachedPriceDisplay,
    productName: "ChimiaClaw cached DFT result",
  };
}

app.post("/v1/checkout", asyncRoute(async (req, res) => {
  const parsed = checkoutBody.safeParse(req.body ?? {});
  if (!parsed.success) {
    res.status(400).json({
      error: "invalid body",
      details: parsed.error.flatten(),
    });
    return;
  }
  const { sku_id, method } = parsed.data;
  let { smiles, label } = parsed.data;
  // Validate a deliverable target BEFORE returning any payment instructions.
  if (sku_id === "moladt.geometry") {
    if (!smiles) { res.status(400).json({ error: "checkout_smiles_required" }); return; }
    label = undefined;
  } else {
    if (!label && !smiles) { res.status(400).json({ error: "checkout_target_required" }); return; }
    const premium = sku_id === "dft.skala_cached";
    const shelf = dftCache.filter(entry => (entry.label.toLowerCase().includes("skala") || (entry.functional ?? "").toLowerCase().includes("skala")) === premium);
    const byLabel = label ? findDftEntry(shelf, label) : null;
    const bySmiles = smiles ? findDftEntry(shelf, smiles, "smiles") : null;
    if ((label && !byLabel) || (smiles && !bySmiles)) { res.status(404).json({ error: "checkout_target_not_available" }); return; }
    if (byLabel && bySmiles && byLabel.artifact_id !== bySmiles.artifact_id) { res.status(400).json({ error: "checkout_target_mismatch" }); return; }
    const entry = byLabel ?? bySmiles!;
    label = entry.label;
    smiles = entry.smiles ?? undefined;
  }
  const { amountCents, amountMicros, priceDisplay, productName } =
    checkoutPricing(sku_id);
  const reference = `chimia-${sku_id}-${Date.now().toString(36)}`;

  if (method === "unstoppable") {
    const ud = await resolveUnstoppableWallet(config);
    if (!config.udDomain) {
      res.status(503).json({
        error: "unstoppable_unconfigured",
        message: "Set UD_DOMAIN (default chimiadao.dao)",
      });
      return;
    }
    const instr = unstoppableInstructions(config, ud, {
      skuId: sku_id,
      amountDisplay: priceDisplay,
      amountMicros,
      reference,
      label,
      smiles,
    });
    if (!instr.pay_to_address) {
      res.status(503).json({
        error: "unstoppable_address_missing",
        message:
          "Domain is set but no EVM address resolved. Set UD_EVM_ADDRESS from the Unstoppable wallet for chimiadao.dao, or UNSTOPPABLE_API_KEY to resolve.",
        domain: config.udDomain,
        profile_url: ud.profile_url,
        next: "UD dashboard → domain → crypto addresses → copy ETH/EVM address into UD_EVM_ADDRESS",
      });
      return;
    }
    await appendRevenueEvent(config.revenueLogPath, {
      at_unix: Math.floor(Date.now() / 1000),
      sku_id,
      mode: "unstoppable_intent",
      amount_usdc_micros: amountMicros,
      price_display: priceDisplay,
      pay_to: String(instr.pay_to_address),
      network: config.udPreferredNetwork,
      result_artifact_id: reference,
      smiles,
      label,
      payment_signature_ref: reference,
    });
    res.json({
      ok: true,
      method: "unstoppable",
      status: "awaiting_onchain_transfer",
      domain: config.udDomain,
      instructions: instr,
      next: "Buyer sends USDC (or configured token) to the domain wallet. Operator verifies tx, then POST /v1/operator/mint-fulfillment and returns fulfill:ft_…",
    });
    return;
  }

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
      amount_usdc_micros: amountMicros,
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
        : sku_id === "dft.cached_result"
          ? config.stripePaymentLinkDftCached
          : null
      : null;

  if (method === "stripe" && paymentLink) {
    await appendRevenueEvent(config.revenueLogPath, {
      at_unix: Math.floor(Date.now() / 1000),
      sku_id,
      mode: "stripe_payment_link",
      amount_usdc_micros: amountMicros,
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
  if (amountCents < 50) {
    res.status(422).json({ error: "checkout_amount_below_minimum", amount_cents: amountCents, minimum_cents: 50 }); return;
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
      amountCents: amountCents, // Stripe min often $0.50 for cards
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
      amount_usdc_micros: amountMicros,
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
  } catch {
    res.status(502).json({
      error: "stripe_failed",
      message: "Checkout creation or logging failed.",
    });
  }
}));

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
  asyncRoute(async (req, res) => {
    const parsed = moladtBody.safeParse(req.body ?? {});
    if (!parsed.success) {
      res.status(400).json({
        error: "invalid body",
        expected: { smiles: "string", no_worker: "optional boolean" },
        details: parsed.error.flatten(),
      });
      return;
    }

    if (!await authorizePayment(req, res, config, { skuId: "moladt.geometry", priceDisplay: config.moladtPriceDisplay, priceMicros: config.moladtPriceMicros, description: "SMILES to signed MolADT", resourcePath: "/v1/moladt" }, fulfillmentStore, { smiles: parsed.data.smiles })) return;
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
      const status = error instanceof MoladtExecutionError ? error.status : 500;
      res.status(status).json({
        error: "moladt execution failed",
        message: error instanceof MoladtExecutionError ? message : "MolADT execution or logging failed.",
        hint: "Build the CLI with `cargo build -p chimiaclaw-cli` and set CHIMIACLAW_CLI",
      });
    }
  }),
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
app.get("/v1/dft/index", (req, res) => {
  const tier =
    typeof req.query.tier === "string" ? req.query.tier.toLowerCase() : "";
  const isSkala = (e: (typeof dftCache)[0]) =>
    e.label.includes("skala") ||
    (e.functional || "").toLowerCase().includes("skala");
  let items = dftCache;
  if (tier === "skala") items = dftCache.filter(isSkala);
  else if (tier === "classical") items = dftCache.filter((e) => !isSkala(e));
  res.json({
    ok: true,
    sku_id:
      tier === "skala" ? "dft.skala_cached" : "dft.cached_result",
    cache_dir: config.dftCacheDir,
    count: items.length,
    total_cache: dftCache.length,
    price_display:
      tier === "skala"
        ? config.dftSkalaPriceDisplay
        : config.dftCachedPriceDisplay,
    price_usdc_micros:
      tier === "skala"
        ? config.dftSkalaPriceMicros
        : config.dftCachedPriceMicros,
    skala_price_display: config.dftSkalaPriceDisplay,
    classical_price_display: config.dftCachedPriceDisplay,
    how_to_buy:
      tier === "skala"
        ? "GET /v1/dft/skala?label=<skala-…> with payment headers"
        : "GET /v1/dft/cached?label=… (classical $5) or /v1/dft/skala?label=… (Skala $20)",
    agent_preferred: {
      resolve: "GET /v1/dft/resolve?q=<smiles|label>",
      packs: "GET /v1/dft/packs",
      skala_index: "GET /v1/dft/index?tier=skala",
      jobs: "POST /v1/dft/jobs",
      credits: "GET /v1/dft/credits",
    },
    items: indexItems(items).map((it) => ({
      ...it,
      tier:
        it.label.includes("skala") ||
        (it.functional || "").toLowerCase().includes("skala")
          ? "skala"
          : "classical",
    })),
  });
});

/**
 * Free agent resolve — "would I buy this right now?"
 * q= label | SMILES | artifact id
 */
app.get("/v1/dft/resolve", (req, res) => {
  const q =
    (typeof req.query.q === "string" && req.query.q) ||
    (typeof req.query.label === "string" && req.query.label) ||
    (typeof req.query.smiles === "string" && req.query.smiles) ||
    "";
  res.json(
    resolveQuery(dftCache, q, {
      classicalPriceDisplay: config.dftCachedPriceDisplay,
      classicalPriceMicros: config.dftCachedPriceMicros,
      skalaPriceDisplay: config.dftSkalaPriceDisplay,
      skalaPriceMicros: config.dftSkalaPriceMicros,
      skalaCreditCost: 4,
    }),
  );
});

/** Free: sealed packs an agent should prefer over random singles. */
app.get("/v1/dft/packs", (_req, res) => {
  const packs = PACK_DEFINITIONS.map((def) => {
    const unit = def.pack_id.startsWith("skala.")
      ? config.dftSkalaPriceMicros
      : config.dftCachedPriceMicros;
    const p = resolvePack(def, dftCache, unit, config.packDiscount);
    return {
      ...p,
      unit_tier: def.pack_id.startsWith("skala.") ? "skala" : "classical",
      members: p.members.map((m) => ({
        query: m.query,
        found: m.found,
        label: m.label,
        artifact_id: m.artifact_id,
      })),
    };
  });
  res.json({
    ok: true,
    sku_id: PACK_SKU,
    unit_price_display: config.dftCachedPriceDisplay,
    skala_unit_price_display: config.dftSkalaPriceDisplay,
    discount: config.packDiscount,
    how_to_buy:
      "GET /v1/dft/packs/:pack_id with PAYMENT-SIGNATURE (stub | fulfill pack token | x402)",
    packs,
  });
});

/**
 * Paid pack download — all available signed results in the pack.
 * Price is dynamic from available members × unit × discount.
 */
app.get("/v1/dft/packs/:pack_id", asyncRoute(async (req, res) => {
  const packId = req.params.pack_id?.trim() ?? "";
  const def = getPackDefinition(packId);
  if (!def) {
    res.status(404).json({
      error: "unknown_pack",
      pack_id: packId,
      list: "/v1/dft/packs",
    });
    return;
  }
  const unit = def.pack_id.startsWith("skala.")
    ? config.dftSkalaPriceMicros
    : config.dftCachedPriceMicros;
  const resolved = resolvePack(def, dftCache, unit, config.packDiscount);
  if (resolved.member_count_available === 0) {
    res.status(404).json({
      error: "pack_empty",
      pack: resolved,
      message: "No sealed members yet — try again after Olympus seal.",
    });
    return;
  }

  if (!await authorizePayment(req, res, config, { skuId: PACK_SKU, priceDisplay: resolved.price_display, priceMicros: resolved.price_usdc_micros, description: `Pack ${packId}`, resourcePath: `/v1/dft/packs/${packId}` }, fulfillmentStore, { packId })) return;

  const entries = collectPackEntries(def, dftCache);
  const x402 = res.locals.x402 as
    | { mode?: string; paymentSignature?: string | null }
    | undefined;

  await appendRevenueEvent(config.revenueLogPath, {
    at_unix: Math.floor(Date.now() / 1000),
    sku_id: PACK_SKU,
    mode: config.mode,
    amount_usdc_micros: resolved.price_usdc_micros,
    price_display: resolved.price_display,
    pay_to: config.payTo,
    network: config.network,
    result_artifact_id: entries.map((e) => e.artifact_id).join(","),
    label: packId,
    payment_signature_ref: x402?.paymentSignature ?? null,
  });

  res.json({
    ok: true,
    sku_id: PACK_SKU,
    pack: resolved,
    results: entries.map((entry) => ({
      label: entry.label,
      artifact_id: entry.artifact_id,
      smiles: entry.smiles,
      summary: {
        functional: entry.functional,
        basis_set: entry.basis_set,
        energy_hartree: entry.energy_hartree,
        gap_ev: entry.gap_ev,
        dipole_debye: entry.dipole_debye,
      },
      result_payload: entry.result_payload,
      artifact: entry.artifact,
    })),
    notes: [
      "Pack is a paid multi-artifact download — same sealed cache, one protocol story.",
      "Missing members are omitted, not fabricated.",
    ],
  });
}));

/** Free: how agent credit bundles work + price table. */
app.get("/v1/dft/credits", (_req, res) => {
  const unit = config.dftCachedPriceMicros;
  const bundles = [3, 5, 10, 20].map((n) => ({
    redemptions: n,
    price_usdc_micros: creditBundlePriceMicros(
      unit,
      n,
      config.creditDiscount,
    ),
    price_display: `$${(creditBundlePriceMicros(unit, n, config.creditDiscount) / 1e6).toFixed(2)}`,
    vs_singles: `${Math.round(config.creditDiscount * 100)}% of ${n}×${config.dftCachedPriceDisplay}`,
  }));
  res.json({
    ok: true,
    sku_id: "dft.credit",
    unit_price_display: config.dftCachedPriceDisplay,
    discount: config.creditDiscount,
    bundles,
    how_it_works: [
      "Human or agent pays for a bundle (Stripe / USDC).",
      "Operator mints: POST /v1/operator/mint-fulfillment { sku_id: 'dft.credit', redemptions: N }",
      "Classical: GET /v1/dft/cached?label=… costs 1 credit",
      "Skala-1.1: GET /v1/dft/skala?label=… costs 4 credits",
      "Spends decrement remaining_redemptions until 0.",
    ],
    spend_table: {
      classical_single: 1,
      skala_single: 4,
      note: "4 classical credits ≈ one Skala-1.1 single at list parity",
    },
    agent_loop: {
      resolve: "GET /v1/dft/resolve?q=benzene",
      buy_classical: "GET /v1/dft/cached?label=benzene + fulfill token",
      buy_skala: "GET /v1/dft/skala?label=skala%20adamantane + fulfill token (4 credits)",
      inspect: "GET /v1/fulfillment/ft_…",
    },
  });
});

/**
 * Free job board — request a structure; cache hit → buy path; else queue.
 * Does not run live DFT on the public path.
 */
app.post("/v1/dft/jobs", asyncRoute(async (req, res) => {
  const body = (req.body ?? {}) as {
    smiles?: string;
    label?: string;
    xyz?: string;
    charge?: number;
    multiplicity?: number;
    method?: string;
    basis?: string;
    geomopt?: boolean;
    notes?: string;
  };
  const job = await createDftJob(jobStore, dftCache, body, {
    priceDisplay: config.dftCachedPriceDisplay,
    skalaPriceDisplay: config.dftSkalaPriceDisplay,
    maxAtomsQueued: config.jobMaxAtoms,
  });
  const status = job.status === "rejected" ? 400 : 200;
  res.status(status).json({
    ok: job.status !== "rejected",
    job,
    next:
      job.status === "available_now"
        ? {
            buy: job.buy,
            payment: "PAYMENT-SIGNATURE: stub | fulfill:ft_… | x402",
          }
        : job.status === "queued"
          ? {
              poll: `GET /v1/dft/jobs/${job.job_id}`,
              honesty:
                "Queued for operator offline seal — not real-time SCF on this host.",
            }
          : { fix: job.reject_reason },
  });
}));

app.get("/v1/dft/jobs/:job_id", asyncRoute(async (req, res) => {
  const job = await getDftJob(jobStore, req.params.job_id ?? "");
  if (!job) {
    res.status(404).json({ error: "not_found" });
    return;
  }
  res.json({ ok: true, job });
}));

/** Operator view of recent queue (secret required). */
app.get("/v1/operator/dft-jobs", asyncRoute(async (req, res) => {
  if (!config.operatorFulfillSecret) {
    res.status(503).json({ error: "operator_disabled" });
    return;
  }
  if ((req.header("x-operator-secret") ?? "") !== config.operatorFulfillSecret) {
    res.status(401).json({ error: "unauthorized" });
    return;
  }
  const jobs = await listRecentJobs(jobStore, 100);
  res.json({ ok: true, count: jobs.length, jobs });
}));

function serveCachedDft(
  skuId: "dft.cached_result" | "dft.skala_cached",
  priceDisplay: string,
  priceMicros: number,
  resourcePath: string,
  requireSkala: boolean,
) {
  return [
    asyncRoute(async (req: express.Request, res: express.Response) => {
      const fields = ["label", "smiles", "id", "q"] as const;
      const supplied = fields.filter(key => typeof req.query[key] === "string" && req.query[key]);
      if (supplied.length > 1) { res.status(400).json({ error: "ambiguous_query" }); return; }
      const field = supplied[0];
      const q = field ? req.query[field] as string : "";

      if (!q) {
        res.status(400).json({
          error: "missing query",
          expected: { label: "…", smiles: "…", id: "art_..." },
          free_index: requireSkala
            ? "/v1/dft/index?tier=skala"
            : "/v1/dft/index",
        });
        return;
      }

      const entry = findDftEntry(dftCache, q, field === "smiles" ? "smiles" : field === "id" ? "id" : "auto");
      if (!entry) {
        res.status(404).json({
          error: "not_in_cache",
          query: q,
          free_index: "/v1/dft/index",
        });
        return;
      }

      const isSkala =
        entry.label.includes("skala") ||
        (entry.functional || "").toLowerCase().includes("skala");
      if (requireSkala && !isSkala) {
        res.status(400).json({
          error: "not_skala_tier",
          message: "Use GET /v1/dft/cached for classical labels, or pick skala-*",
          label: entry.label,
          free_index: "/v1/dft/index?tier=skala",
        });
        return;
      }
      if (!requireSkala && isSkala) {
        res.status(400).json({
          error: "skala_requires_premium_sku",
          message: "Skala labels are sold via GET /v1/dft/skala?label=… ($20)",
          label: entry.label,
          buy: `/v1/dft/skala?label=${encodeURIComponent(entry.label)}`,
        });
        return;
      }

      if (!await authorizePayment(req, res, config, { skuId, priceDisplay, priceMicros, description: "Cached signed DFT result", resourcePath }, fulfillmentStore, { label: field === "label" ? q : entry.label, smiles: entry.smiles ?? undefined })) return;

      const x402 = res.locals.x402 as
        | { mode: string; paymentSignature?: string | null }
        | undefined;

      await appendRevenueEvent(config.revenueLogPath, {
        at_unix: Math.floor(Date.now() / 1000),
        sku_id: skuId,
        mode: config.mode,
        amount_usdc_micros: priceMicros,
        price_display: priceDisplay,
        pay_to: config.payTo,
        network: config.network,
        result_artifact_id: entry.artifact_id,
        smiles: entry.smiles ?? undefined,
        label: entry.label,
        payment_signature_ref: x402?.paymentSignature ?? null,
      });

      res.json({
        ok: true,
        sku_id: skuId,
        tier: requireSkala ? "skala" : "classical",
        settlement: {
          method: "X402HttpPayment",
          mode: config.mode,
          amount_usdc_micros: priceMicros,
          price_display: priceDisplay,
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
          requireSkala
            ? "Skala-1.1 premium sealed result (not a live SCF)."
            : "Classical sealed result (not a live SCF).",
          "Signed chem.dft.result is the payload of record.",
        ],
      });
    }),
  ];
}

app.get(
  "/v1/dft/cached",
  ...(serveCachedDft(
    "dft.cached_result",
    config.dftCachedPriceDisplay,
    config.dftCachedPriceMicros,
    "/v1/dft/cached",
    false,
  ) as Parameters<typeof app.get>[1][]),
);

app.get(
  "/v1/dft/skala",
  ...(serveCachedDft(
    "dft.skala_cached",
    config.dftSkalaPriceDisplay,
    config.dftSkalaPriceMicros,
    "/v1/dft/skala",
    true,
  ) as Parameters<typeof app.get>[1][]),
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

app.use(errorHandler);
return app;
}
