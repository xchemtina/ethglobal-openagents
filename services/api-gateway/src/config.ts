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
  /** Human rails */
  stripeSecretKey: string | null;
  stripePublishableKey: string | null;
  /** Dashboard Payment Links (buy.stripe.com/...) per SKU */
  stripePaymentLinkMoladt: string | null;
  stripePaymentLinkDftCached: string | null;
  revolutPayTo: string | null;
  revolutPaymentLink: string | null;
  revolutNote: string | null;
  publicBaseUrl: string;
  rateLimitMax: number;
  rateLimitWindowMs: number;
  rateLimitMaxKeys: number;
  moladtTimeoutMs: number;
  moladtMaxOutputBytes: number;
  moladtMaxConcurrent: number;
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
    dftCacheDir: env("DFT_CACHE_DIR", "../../demo/dft"),
    moladtPriceMicros: 10_000,
    moladtPriceDisplay: "$0.01",
    dftCachedPriceMicros: 50_000,
    dftCachedPriceDisplay: "$0.05",
    stripeSecretKey: process.env.STRIPE_SECRET_KEY?.trim() || null,
    stripePublishableKey: process.env.STRIPE_PUBLISHABLE_KEY?.trim() || null,
    stripePaymentLinkMoladt:
      process.env.STRIPE_PAYMENT_LINK_MOLADT?.trim() || null,
    stripePaymentLinkDftCached:
      process.env.STRIPE_PAYMENT_LINK_DFT_CACHED?.trim() || null,
    revolutPayTo: process.env.REVOLUT_PAY_TO?.trim() || null,
    revolutPaymentLink: process.env.REVOLUT_PAYMENT_LINK?.trim() || null,
    revolutNote: process.env.REVOLUT_NOTE?.trim() || null,
    publicBaseUrl: env("PUBLIC_BASE_URL", "http://127.0.0.1:4021"),
    rateLimitMax: positiveInteger("RATE_LIMIT_MAX", "120"),
    rateLimitWindowMs: positiveInteger("RATE_LIMIT_WINDOW_MS", "60000", 2_147_483_647),
    rateLimitMaxKeys: positiveInteger("RATE_LIMIT_MAX_KEYS", "10000"),
    moladtTimeoutMs: positiveInteger("MOLADT_TIMEOUT_MS", "30000", 2_147_483_647),
    moladtMaxOutputBytes: positiveInteger("MOLADT_MAX_OUTPUT_BYTES", "4194304"),
    moladtMaxConcurrent: positiveInteger("MOLADT_MAX_CONCURRENT", "4"),
  };
}
