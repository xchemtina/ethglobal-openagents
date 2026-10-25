export type X402Mode = "free" | "stub" | "live";

export interface GatewayConfig {
  port: number;
  host: string;
  corsOrigin: string;
  mode: X402Mode;
  payTo: string;
  network: string;
  facilitatorUrl: string;
  cliPath: string;
  revenueLogPath: string;
  /** Directory of signed chem.dft.result JSON artifacts (demo/dft by default). */
  dftCacheDir: string;
  /** MolADT SKU price in USDC micros ($0.01 default). */
  moladtPriceMicros: number;
  moladtPriceDisplay: string;
  /** Cached DFT SKU price in USDC micros ($0.05 default). */
  dftCachedPriceMicros: number;
  dftCachedPriceDisplay: string;
  /** Skala-1.1 premium cached SKU (default $20). */
  dftSkalaPriceMicros: number;
  dftSkalaPriceDisplay: string;
  /** Human rails */
  stripeSecretKey: string | null;
  stripePublishableKey: string | null;
  /** Dashboard Payment Links (buy.stripe.com/...) per SKU */
  stripePaymentLinkMoladt: string | null;
  stripePaymentLinkDftCached: string | null;
  /** Stripe webhook signing secret (whsec_…) from CLI listen or Dashboard */
  stripeWebhookSecret: string | null;
  /** Append-only fulfillment token log */
  fulfillmentLogPath: string;
  /** Fulfillment token TTL seconds (default 24h) */
  fulfillmentTtlSeconds: number;
  revolutPayTo: string | null;
  revolutPaymentLink: string | null;
  revolutNote: string | null;
  /** Unstoppable Domains human/crypto rail (e.g. chimiadao.dao) */
  udDomain: string | null;
  /** Resolved or operator-pinned EVM receive address for UD domain */
  udEvmAddress: string | null;
  /** Preferred chain CAIP-2 or label (eip155:8453 = Base mainnet) */
  udPreferredNetwork: string;
  /** Token symbol buyers should send (USDC default) */
  udToken: string;
  /** Optional UD Partner API key for live resolve */
  unstoppableApiKey: string | null;
  /** Shared secret for operator mint of fulfillment tokens (Stripe paid emails). */
  operatorFulfillSecret: string | null;
  publicBaseUrl: string;
  rateLimitMax: number;
  rateLimitWindowMs: number;
  rateLimitMaxKeys: number;
  moladtTimeoutMs: number;
  moladtMaxOutputBytes: number;
  moladtMaxConcurrent: number;
  /** Append-only agent DFT job board */
  jobLogPath: string;
  /** Max heavy atoms accepted into public queue */
  jobMaxAtoms: number;
  /** Pack price as fraction of unit×count (0.8 = 20% off) */
  packDiscount: number;
  /** Credit bundle discount vs singles */
  creditDiscount: number;
}

function env(name: string, fallback?: string): string {
  const value = process.env[name] ?? fallback;
  if (value === undefined) {
    throw new Error(`missing required env ${name}`);
  }
  return value;
}

function positiveInteger(name: string, fallback: string, maximum = Number.MAX_SAFE_INTEGER): number {
  const value = Number(env(name, fallback));
  if (!Number.isSafeInteger(value) || value <= 0 || value > maximum) {
    throw new Error(`${name} must be a positive integer no greater than ${maximum}`);
  }
  return value;
}

function parseMode(raw: string): X402Mode {
  if (raw === "free" || raw === "stub" || raw === "live") {
    return raw;
  }
  throw new Error(`X402_MODE must be free|stub|live, got: ${raw}`);
}

export function loadConfig(): GatewayConfig {
  const mode = parseMode(env("X402_MODE", "stub"));
  const payTo = env("CHIMIA_X402_PAY_TO", "0xChimiaDAOTreasuryPlaceholder0000000000");
  if (mode === "live" && payTo.includes("Placeholder")) {
    throw new Error(
      "live mode requires a real CHIMIA_X402_PAY_TO treasury address (not the placeholder)",
    );
  }

  return {
    port: positiveInteger("PORT", "4021", 65535),
    host: env("HOST", "0.0.0.0"),
    corsOrigin: env("CORS_ORIGIN", "*"),
    mode,
    payTo,
    network: env("X402_NETWORK", "eip155:84532"),
    facilitatorUrl: env("X402_FACILITATOR_URL", "https://x402.org/facilitator"),
    cliPath: env("CHIMIACLAW_CLI", "../../target/debug/chimiaclaw-cli"),
    revenueLogPath: env("REVENUE_LOG_PATH", "./data/revenue.jsonl"),
    // Colon-separated dirs allowed (e.g. demo/dft:demo/overnight-science-out/dft)
    dftCacheDir: env("DFT_CACHE_DIR", "../../demo/dft"),
    // Human-facing catalog defaults match Stripe Payment Link scale (≥ $0.50 card floor).
    // Override via env; agent x402 can still use stub without moving funds.
    moladtPriceMicros: positiveInteger("MOLADT_PRICE_MICROS", "1000000"),
    moladtPriceDisplay: env("MOLADT_PRICE_DISPLAY", "$1.00"),
    dftCachedPriceMicros: positiveInteger("DFT_CACHED_PRICE_MICROS", "5000000"),
    dftCachedPriceDisplay: env("DFT_CACHED_PRICE_DISPLAY", "$5.00"),
    dftSkalaPriceMicros: positiveInteger("DFT_SKALA_PRICE_MICROS", "20000000"),
    dftSkalaPriceDisplay: env("DFT_SKALA_PRICE_DISPLAY", "$20.00"),
    stripeSecretKey: process.env.STRIPE_SECRET_KEY?.trim() || null,
    stripePublishableKey: process.env.STRIPE_PUBLISHABLE_KEY?.trim() || null,
    stripePaymentLinkMoladt:
      process.env.STRIPE_PAYMENT_LINK_MOLADT?.trim() || null,
    stripePaymentLinkDftCached:
      process.env.STRIPE_PAYMENT_LINK_DFT_CACHED?.trim() || null,
    stripeWebhookSecret: process.env.STRIPE_WEBHOOK_SECRET?.trim() || null,
    fulfillmentLogPath: env(
      "FULFILLMENT_LOG_PATH",
      "./data/fulfillment-tokens.jsonl",
    ),
    fulfillmentTtlSeconds: positiveInteger("FULFILLMENT_TTL_SECONDS", String(24 * 60 * 60)),
    revolutPayTo: process.env.REVOLUT_PAY_TO?.trim() || null,
    revolutPaymentLink: process.env.REVOLUT_PAYMENT_LINK?.trim() || null,
    revolutNote: process.env.REVOLUT_NOTE?.trim() || null,
    udDomain: process.env.UD_DOMAIN?.trim() || "chimiadao.dao",
    udEvmAddress: process.env.UD_EVM_ADDRESS?.trim() || null,
    udPreferredNetwork: env("UD_PREFERRED_NETWORK", "eip155:8453"),
    udToken: env("UD_TOKEN", "USDC"),
    unstoppableApiKey: process.env.UNSTOPPABLE_API_KEY?.trim() || null,
    operatorFulfillSecret:
      process.env.OPERATOR_FULFILL_SECRET?.trim() || null,
    publicBaseUrl: env("PUBLIC_BASE_URL", "http://127.0.0.1:4021"),
    rateLimitMax: positiveInteger("RATE_LIMIT_MAX", "120"),
    rateLimitWindowMs: positiveInteger("RATE_LIMIT_WINDOW_MS", "60000", 2147483647),
    rateLimitMaxKeys: positiveInteger("RATE_LIMIT_MAX_KEYS", "10000"),
    moladtTimeoutMs: positiveInteger("MOLADT_TIMEOUT_MS", "30000", 2147483647),
    moladtMaxOutputBytes: positiveInteger("MOLADT_MAX_OUTPUT_BYTES", "4194304"),
    moladtMaxConcurrent: positiveInteger("MOLADT_MAX_CONCURRENT", "4"),
    jobLogPath: env("DFT_JOB_LOG_PATH", "./data/dft-jobs.jsonl"),
    jobMaxAtoms: positiveInteger("DFT_JOB_MAX_ATOMS", "40"),
    packDiscount: Number(env("DFT_PACK_DISCOUNT", "0.8")),
    creditDiscount: Number(env("DFT_CREDIT_DISCOUNT", "0.85")),
  };
}
