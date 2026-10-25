import type { Request, Response, NextFunction } from "express";
import type { GatewayConfig } from "./config.js";
import {
  extractFulfillToken,
  redeemFulfillmentToken,
  type FulfillmentStore,
} from "./fulfillment.js";

export interface PaymentRequirement {
  scheme: string;
  network: string;
  maxAmountRequired: string;
  resource: string;
  description: string;
  mimeType: string;
  payTo: string;
  maxTimeoutSeconds: number;
  asset: string;
  extra: {
    name: string;
    version: string;
    sku_id: string;
    price_display: string;
    mode: string;
  };
}

export interface PaymentGateOptions {
  skuId: string;
  priceDisplay: string;
  priceMicros: number;
  description: string;
  resourcePath: string;
}

/** Authorize only after validating the requested resource. Fulfillment is a
 * separate paid rail; live x402 headers are never accepted as evidence. */
export async function authorizePayment(
  req: Request, res: Response, config: GatewayConfig, options: PaymentGateOptions,
  fulfillmentStore?: FulfillmentStore, target: { label?: string; smiles?: string; packId?: string } = {},
): Promise<boolean> {
  if (config.mode === "free") {
    res.locals.x402 = { mode: "free", verified: false, paymentSignature: null };
    return true;
  }
  const signature = req.header("PAYMENT-SIGNATURE") ?? req.header("X-PAYMENT") ?? "";
  const token = extractFulfillToken(signature);
  if (token && fulfillmentStore) {
    const result = await redeemFulfillmentToken(fulfillmentStore, token, options.skuId, options.resourcePath, target);
    if (!result.ok) {
      res.status(402).json({ error: "Payment Required", fulfillment: result.reason });
      return false;
    }
    res.locals.x402 = { mode: result.row.sku_id === "dft.credit" ? "agent_credit" : "stripe_fulfillment", verified: true, paymentSignature: `fulfill:${token}`, fulfillment_token: token, remaining_redemptions: result.row.remaining_redemptions };
    return true;
  }
  if (config.mode === "stub") {
    if (signature.toLowerCase() === "stub") {
      res.locals.x402 = { mode: "stub", verified: false, paymentSignature: "stub" };
      return true;
    }
    send402(res, config, options);
    return false;
  }
  res.status(503).json({ error: "live_payment_unavailable", message: "Live x402 verification and settlement are not implemented. A valid separately issued fulfillment token is required." });
  return false;
}

export function paymentGate(config: GatewayConfig, options: PaymentGateOptions, fulfillmentStore?: FulfillmentStore) {
  return (req: Request, res: Response, next: NextFunction): void => {
    void authorizePayment(req, res, config, options, fulfillmentStore).then(allowed => { if (allowed) next(); }).catch(next);
  };
}

function send402(
  res: Response,
  config: GatewayConfig,
  options: PaymentGateOptions,
): void {
  const requirement: PaymentRequirement = {
    scheme: "exact",
    network: config.network,
    maxAmountRequired: String(options.priceMicros),
    resource: options.resourcePath,
    description: options.description,
    mimeType: "application/json",
    payTo: config.payTo,
    maxTimeoutSeconds: 600,
    asset: "USDC",
    extra: {
      name: "chimiaclaw-x402",
      version: "0.1.0",
      sku_id: options.skuId,
      price_display: options.priceDisplay,
      mode: config.mode,
    },
  };

  // x402 clients read PAYMENT-REQUIRED. Header values must be ASCII-safe
  // (Node rejects arrows / non-ISO-8859-1 characters in setHeader).
  const headerRequirement = {
    ...requirement,
    description: asciiHeader(requirement.description),
    extra: {
      ...requirement.extra,
      name: asciiHeader(requirement.extra.name),
      sku_id: asciiHeader(requirement.extra.sku_id),
      price_display: asciiHeader(requirement.extra.price_display),
      mode: asciiHeader(requirement.extra.mode),
    },
  };
  res.setHeader("PAYMENT-REQUIRED", JSON.stringify(headerRequirement));
  res.setHeader("Content-Type", "application/json");
  res.status(402).json({
    error: "Payment Required",
    x402Version: 1,
    accepts: [requirement],
    how_to_pay: {
      stub: "Retry with header: PAYMENT-SIGNATURE: stub",
      credit: "Operator-minted multi-use: PAYMENT-SIGNATURE: fulfill:ft_… (dft.credit)",
      fulfill: "One-shot after Stripe: PAYMENT-SIGNATURE: fulfill:ft_…",
      live: "Unavailable until verification and settlement are implemented",
      resolve: "GET /v1/dft/resolve?q=<smiles|label> — free discovery",
      packs: "GET /v1/dft/packs — sealed series (agent-preferred)",
      jobs: "POST /v1/dft/jobs — queue if not in cache",
      docs: "https://docs.x402.org/getting-started/quickstart-for-buyers",
      catalog: "/v1/catalog",
    },
  });
}

function asciiHeader(value: string): string {
  return value.replace(/[^\x20-\x7E]/g, " ");
}
