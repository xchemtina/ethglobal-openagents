/**
 * Human payment rails: Stripe Checkout + Revolut + Unstoppable Domains wallet.
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
  id: "x402" | "stripe" | "revolut" | "unstoppable";
  status: "unavailable" | "configured" | "stub" | "unconfigured" | "partial";
  title: string;
  description: string;
  details: Record<string, string | boolean | number | null>;
}

export interface UdResolvedWallet {
  domain: string;
  evm_address: string | null;
  source: "env" | "unstoppable_api" | "none";
  records: Record<string, string>;
  profile_url: string;
}

/** Resolve UD multi-chain records when API key present; else use UD_EVM_ADDRESS. */
export async function resolveUnstoppableWallet(
  config: GatewayConfig,
): Promise<UdResolvedWallet> {
  const domain = config.udDomain || "chimiadao.dao";
  const profile_url = `https://ud.me/${domain}`;
  const records: Record<string, string> = {};

  if (config.udEvmAddress) {
    return {
      domain,
      evm_address: config.udEvmAddress,
      source: "env",
      records: { "crypto.ETH.address": config.udEvmAddress },
      profile_url,
    };
  }

  if (config.unstoppableApiKey) {
    try {
      const url = `https://api.unstoppabledomains.com/resolve/domains/${encodeURIComponent(domain)}`;
      const res = await fetch(url, {
        headers: {
          Authorization: `Bearer ${config.unstoppableApiKey}`,
          Accept: "application/json",
        },
      });
      if (res.ok) {
        const body = (await res.json()) as {
          records?: Record<string, string>;
          meta?: { domain?: string };
        };
        const rec = body.records || {};
        Object.assign(records, rec);
        const addr =
          rec["crypto.ETH.address"] ||
          rec["token.EVM.address"] ||
          rec["crypto.MATIC.version.ERC20.address"] ||
          rec["crypto.USDC.version.ERC20.address"] ||
          rec["crypto.ETH.version.ERC20.address"] ||
          null;
        if (addr) {
          return {
            domain,
            evm_address: addr,
            source: "unstoppable_api",
            records: rec,
            profile_url,
          };
        }
      }
    } catch {
      /* fall through */
    }
  }

  return {
    domain,
    evm_address: null,
    source: "none",
    records,
    profile_url,
  };
}

export function paymentMethods(
  config: GatewayConfig,
  ud?: UdResolvedWallet | null,
): PaymentMethodsResponse {
  const stripeCheckoutConfigured = Boolean(config.stripeSecretKey);
  const stripeLinksConfigured = Boolean(
    config.stripePaymentLinkMoladt || config.stripePaymentLinkDftCached,
  );
  const stripeConfigured = stripeCheckoutConfigured || stripeLinksConfigured;
  const revolutConfigured = Boolean(
    config.revolutPayTo || config.revolutPaymentLink,
  );
  const udDomain = config.udDomain || "chimiadao.dao";
  const udAddr = ud?.evm_address || config.udEvmAddress;
  const udConfigured = Boolean(udDomain && udAddr);
  const udPartial = Boolean(udDomain && !udAddr);

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
        "Free/stub integration and separately issued fulfillment tokens. Live x402 verification and settlement are unavailable.",
      details: {
        mode: config.mode,
        pay_to: config.payTo,
        network: config.network,
        facilitator_url: config.facilitatorUrl,
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
        checkout_api: stripeCheckoutConfigured,
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
      id: "unstoppable",
      status: udConfigured
        ? "configured"
        : udPartial
          ? "partial"
          : "unconfigured",
      title: `Unstoppable · ${udDomain}`,
      description: udConfigured
        ? `Send ${config.udToken} to the EVM address resolved for ${udDomain}. Include the checkout reference. Operator mints fulfill token after on-chain confirmation.`
        : `Domain ${udDomain} is wired — set UD_EVM_ADDRESS (from UD wallet) or UNSTOPPABLE_API_KEY to resolve live.`,
      details: {
        domain: udDomain,
        profile_url: ud?.profile_url || `https://ud.me/${udDomain}`,
        pay_to_domain: udDomain,
        pay_to_address: udAddr,
        token: config.udToken,
        preferred_network: config.udPreferredNetwork,
        resolve_source: ud?.source || (config.udEvmAddress ? "env" : "none"),
        checkout: "POST /v1/checkout { method: 'unstoppable', sku_id }",
      },
    },
    {
      id: "revolut",
      status: revolutConfigured ? "configured" : "unconfigured",
      title: "Revolut (personal wallet)",
      description: revolutConfigured
        ? "Manual transfer to operator Revolut; include SKU in reference. Fulfillment is operator-gated."
        : "Set REVOLUT_PAY_TO and/or REVOLUT_PAYMENT_LINK. Off by default (privacy).",
      details: {
        pay_to: config.revolutPayTo,
        payment_link: config.revolutPaymentLink,
        note: config.revolutNote,
      },
    },
  ];

  const humanReady =
    stripeConfigured || revolutConfigured || udConfigured || udPartial;
  return {
    deployable: humanReady || config.mode === "stub" || config.mode === "free",
    honesty: [
      "Stub x402 never moves funds; live x402 rejects unverifiable headers.",
      "Stripe: Payment Link / Checkout charge; POST /v1/webhooks/stripe mints a one-shot fulfillment token redeemable as PAYMENT-SIGNATURE: fulfill:ft_…",
      `Unstoppable Domains (${udDomain}): crypto transfer to domain wallet — operator confirms on-chain, then mints fulfill token. Not auto-settled yet.`,
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
  if (!Number.isSafeInteger(opts.amountCents) || opts.amountCents < 50) throw new Error("checkout_amount_below_minimum");
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
    signal: AbortSignal.timeout(15000),
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
      "Include the reference string in the payment description. Operator confirms before delivery of live compute.",
  };
}

export function unstoppableInstructions(
  config: GatewayConfig,
  ud: UdResolvedWallet,
  opts: {
    skuId: string;
    amountDisplay: string;
    amountMicros: number;
    reference: string;
    label?: string;
    smiles?: string;
  },
): Record<string, string | number | null> {
  const amountToken = (opts.amountMicros / 1_000_000).toFixed(2);
  const addr = ud.evm_address;
  return {
    sku_id: opts.skuId,
    label: opts.label ?? null,
    smiles: opts.smiles ?? null,
    amount_display: opts.amountDisplay,
    amount_token: amountToken,
    token: config.udToken,
    network: config.udPreferredNetwork,
    pay_to_domain: ud.domain,
    pay_to_address: addr,
    profile_url: ud.profile_url,
    reference: opts.reference,
    memo: opts.reference,
    resolve_source: ud.source,
    ethereum_uri: addr
      ? `ethereum:${addr}@${config.udPreferredNetwork.replace("eip155:", "")}`
      : null,
    note: `Send ${amountToken} ${config.udToken} on ${config.udPreferredNetwork} to ${ud.domain}${
      addr ? ` (${addr})` : ""
    }. Put reference "${opts.reference}" in the tx memo/note if your wallet supports it. Email operator with tx hash; they mint PAYMENT-SIGNATURE: fulfill:ft_…`,
  };
}
