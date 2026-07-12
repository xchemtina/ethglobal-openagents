import type { Request, Response, NextFunction } from "express";
import type { GatewayConfig } from "./config.js";

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

/**
 * Minimal x402-compatible gate for free/stub modes.
 * Live mode can be upgraded to official @x402/express middleware when packages are installed.
 */
export function paymentGate(config: GatewayConfig, options: PaymentGateOptions) {
  return (req: Request, res: Response, next: NextFunction): void => {
    if (config.mode === "free") {
      res.locals.x402 = {
        mode: "free",
        verified: true,
        paymentSignature: null,
      };
      next();
      return;
    }

    const signature =
      (req.header("PAYMENT-SIGNATURE") ??
        req.header("payment-signature") ??
        req.header("X-PAYMENT") ??
        "") ||
      "";

    if (config.mode === "stub") {
      if (signature === "stub" || signature.toLowerCase() === "stub") {
        res.locals.x402 = {
          mode: "stub",
          verified: true,
          paymentSignature: "stub",
        };
        next();
        return;
      }
      send402(res, config, options);
      return;
    }

    // live mode — without full @x402 middleware, require an explicit signature header
    // and mark verification as "header-present" for operator follow-up.
    // Prefer installing @x402/express and wiring facilitator verification.
    if (!signature || signature === "stub") {
      send402(res, config, options);
      return;
    }

    res.locals.x402 = {
      mode: "live",
      verified: true,
      paymentSignature: signature.slice(0, 256),
      verification: "header-present-operator-must-enable-facilitator",
    };
    next();
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
      live: "Use an x402-compatible client / wallet; settle USDC then retry with PAYMENT-SIGNATURE",
      docs: "https://docs.x402.org/getting-started/quickstart-for-buyers",
      catalog: "/v1/catalog",
    },
  });
}

function asciiHeader(value: string): string {
  return value.replace(/[^\x20-\x7E]/g, " ");
}
