/**
 * Human payment rails: Stripe Checkout + Revolut wallet instructions.
 * Agents keep x402 (stub/live). These are for operators and human buyers.
 */

import type { GatewayConfig } from "./config.js";

export interface PaymentMethodsResponse {
  deployable: boolean;
  honesty: string[];
  methods: PaymentMethod[];
  agent_default: "x402_stub" | "unavailable" | "free";
}

export interface PaymentMethod {
  id: "x402" | "stripe" | "revolut";
  status: "unavailable" | "configured" | "stub" | "unconfigured";
  title: string;
  description: string;
  details: Record<string, string | boolean | null>;
}

export function paymentMethods(config: GatewayConfig): PaymentMethodsResponse {
  const stripeCheckoutConfigured = Boolean(config.stripeSecretKey);
  const stripeCheckoutAvailable = stripeCheckoutConfigured && (
    config.moladtPriceMicros >= 500_000 || config.dftCachedPriceMicros >= 500_000
  );
  const stripeLinksConfigured = Boolean(
    config.stripePaymentLinkMoladt || config.stripePaymentLinkDftCached,
  );
  const stripeConfigured = stripeCheckoutAvailable || stripeLinksConfigured;
  const revolutConfigured = Boolean(
    config.revolutPayTo || config.revolutPaymentLink,
  );

  const methods: PaymentMethod[] = [
    {
      id: "x402",
      status:
        config.mode === "live"
          ? "unavailable"
          : config.mode === "stub"
            ? "stub"
            : "configured",
      title: "x402 (agents)",
      description:
        "Free/stub integration only. Live x402 is unavailable until verification and settlement are implemented.",
      details: {
        mode: config.mode,
        pay_to: config.payTo,
        network: config.network,
        facilitator_url: config.facilitatorUrl,
        live_verification: false,
        live_settlement: false,
      },
    },
    {
      id: "stripe",
      status: stripeConfigured ? "configured" : "unconfigured",
      title: "Stripe (Payment Links or Checkout)",
      description: stripeConfigured
        ? "Card payments via Dashboard Payment Links and/or API Checkout sessions."
        : "Create Payment Links in Stripe Dashboard, or set STRIPE_SECRET_KEY for Checkout API.",
      details: {
        configured: stripeConfigured,
        checkout_api: stripeCheckoutAvailable,
        checkout_api_key_configured: stripeCheckoutConfigured,
        checkout_minimum_usd: "0.50",
        payment_links: stripeLinksConfigured,
        publishable_key_set: Boolean(config.stripePublishableKey),
        payment_link_moladt: config.stripePaymentLinkMoladt,
        payment_link_dft_cached: config.stripePaymentLinkDftCached,
        checkout: "/v1/checkout",
        dashboard_create:
          "https://dashboard.stripe.com/acct_1PsRXKLWiL2XLIJa/payment-links/create",
      },
    },
    {
      id: "revolut",
      status: revolutConfigured ? "configured" : "unconfigured",
      title: "Revolut (personal wallet)",
      description: revolutConfigured
        ? "Manual transfer to operator Revolut; include SKU in reference. Fulfillment is operator-gated."
        : "Set REVOLUT_PAY_TO and/or REVOLUT_PAYMENT_LINK.",
      details: {
        pay_to: config.revolutPayTo,
        payment_link: config.revolutPaymentLink,
        note: config.revolutNote,
      },
    },
  ];

  const humanReady = stripeConfigured || revolutConfigured;
  return {
    deployable: humanReady || config.mode === "stub" || config.mode === "free",
    honesty: [
      "Stub x402 never moves funds; live mode returns 503 and accepts no payment.",
      "Stripe Checkout API rejects these micro-priced SKUs below its configured $0.50 floor; no silent price increase.",
      "Stripe charges when a Payment Link is paid or STRIPE_SECRET_KEY Checkout completes (operator still fulfills SKU until webhook).",
      "Revolut is manual settlement — do not auto-deliver high-cost SKUs without operator confirmation.",
      "Live DFT remains operator-gated regardless of payment rail.",
    ],
    methods,
    agent_default:
      config.mode === "live"
        ? "unavailable"
        : config.mode === "free"
          ? "free"
          : "x402_stub",
  };
}

export async function createStripeCheckout(
  config: GatewayConfig,
  opts: {
    skuId: string;
    amountCents: number;
    productName: string;
    successUrl: string;
    cancelUrl: string;
    metadata?: Record<string, string>;
  },
): Promise<{ url: string; id: string }> {
  if (!config.stripeSecretKey) {
    throw new Error("STRIPE_SECRET_KEY not configured");
  }
  if (!Number.isSafeInteger(opts.amountCents) || opts.amountCents < 50) {
    throw new Error("checkout_amount_below_minimum: configure an explicitly priced bundle of at least $0.50");
  }
  const params = new URLSearchParams();
  params.set("mode", "payment");
  params.set("success_url", opts.successUrl);
  params.set("cancel_url", opts.cancelUrl);
  params.set("line_items[0][price_data][currency]", "usd");
  params.set(
    "line_items[0][price_data][product_data][name]",
    opts.productName,
  );
  params.set(
    "line_items[0][price_data][unit_amount]",
    String(opts.amountCents),
  );
  params.set("line_items[0][quantity]", "1");
  params.set("metadata[sku_id]", opts.skuId);
  if (opts.metadata) {
    for (const [k, v] of Object.entries(opts.metadata)) {
      params.set(`metadata[${k}]`, v);
    }
  }

  const res = await fetch("https://api.stripe.com/v1/checkout/sessions", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${config.stripeSecretKey}`,
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body: params.toString(),
    signal: AbortSignal.timeout(15_000),
  });
  const body = (await res.json()) as {
    id?: string;
    url?: string;
    error?: { message?: string };
  };
  if (!res.ok || !body.url || !body.id) {
    throw new Error(
      body.error?.message || `Stripe checkout failed HTTP ${res.status}`,
    );
  }
  return { url: body.url, id: body.id };
}

export function revolutInstructions(
  config: GatewayConfig,
  opts: { skuId: string; amountDisplay: string; reference: string },
): Record<string, string | null> {
  return {
    sku_id: opts.skuId,
    amount: opts.amountDisplay,
    pay_to: config.revolutPayTo,
    payment_link: config.revolutPaymentLink,
    reference: opts.reference,
    note:
      config.revolutNote ||
      "Include the reference string in the payment description. Operator confirms before manual fulfillment of the selected artifact SKU.",
  };
}
