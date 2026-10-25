/**
 * Stripe webhook verification + fulfillment minting.
 * No Stripe SDK — HMAC per https://stripe.com/docs/webhooks/signatures
 */

import { createHmac, timingSafeEqual } from "node:crypto";
import type { FulfillmentStore, FulfillmentToken } from "./fulfillment.js";
import { mintFulfillmentToken, parseSkuId } from "./fulfillment.js";

const DEFAULT_TOLERANCE_SECONDS = 300;

export interface StripeWebhookResult {
  ok: boolean;
  status: number;
  body: Record<string, unknown>;
  token?: FulfillmentToken;
}

export function verifyStripeSignature(
  rawBody: Buffer,
  signatureHeader: string | undefined,
  secret: string,
  toleranceSeconds = DEFAULT_TOLERANCE_SECONDS,
): { ok: true; timestamp: number } | { ok: false; reason: string } {
  if (!signatureHeader) {
    return { ok: false, reason: "missing_stripe_signature" };
  }
  if (!secret) {
    return { ok: false, reason: "missing_webhook_secret" };
  }

  const parts = signatureHeader.split(",").map((p) => p.trim());
  let timestamp: number | null = null;
  const v1: string[] = [];
  for (const part of parts) {
    const [k, v] = part.split("=");
    if (k === "t" && v) {
      timestamp = Number(v);
    } else if (k === "v1" && v) {
      v1.push(v);
    }
  }
  if (timestamp === null || !Number.isSafeInteger(timestamp) || timestamp <= 0 || v1.length === 0) {
    return { ok: false, reason: "malformed_signature_header" };
  }

  const now = Math.floor(Date.now() / 1000);
  if (Math.abs(now - timestamp) > toleranceSeconds) {
    return { ok: false, reason: "timestamp_outside_tolerance" };
  }

  const expected = createHmac("sha256", secret).update(`${timestamp}.`, "utf8").update(rawBody).digest("hex");
  const expectedBuf = Buffer.from(expected, "utf8");

  let matched = false;
  for (const sig of v1) {
    const got = Buffer.from(sig, "utf8");
    if (got.length === expectedBuf.length && timingSafeEqual(got, expectedBuf)) {
      matched = true;
      break;
    }
  }
  if (!matched) {
    return { ok: false, reason: "signature_mismatch" };
  }
  return { ok: true, timestamp };
}

export interface StripeEventLike {
  id?: string;
  type?: string;
  data?: { object?: Record<string, unknown> };
}
export type WebhookPrices = Record<"moladt.geometry" | "dft.cached_result" | "dft.skala_cached", number>;
const DEFAULT_PRICES: WebhookPrices = { "moladt.geometry": 100, "dft.cached_result": 500, "dft.skala_cached": 2000 };

/** Only paid Checkout Sessions grant entitlements. A payment-intent event cannot
 * independently grant another token for the same payment. Packs/credits require
 * operator minting until a priced Checkout contract exists for those SKUs. */
export async function handleStripeEvent(store: FulfillmentStore, event: StripeEventLike, prices: WebhookPrices = DEFAULT_PRICES): Promise<StripeWebhookResult> {
  if (!event || typeof event !== "object" || Array.isArray(event) || typeof event.type !== "string") {
    return { ok: false, status: 400, body: { error: "invalid_stripe_event" } };
  }
  const type = event.type;
  if (!["checkout.session.completed", "checkout.session.async_payment_succeeded"].includes(type)) {
    return { ok: true, status: 200, body: { received: true, ignored: true, type } };
  }
  const obj = event.data?.object;
  if (!obj || !/^evt_[A-Za-z0-9_]+$/.test(event.id ?? "") || typeof obj.id !== "string" || !/^cs_[A-Za-z0-9_]+$/.test(obj.id) || obj.object !== "checkout.session" || obj.mode !== "payment") {
    return { ok: false, status: 400, body: { error: "invalid_checkout_session" } };
  }
  if (obj.payment_status === "unpaid") {
    return { ok: true, status: 200, body: { received: true, deferred: true, type } };
  }
  const meta = obj.metadata;
  if (obj.payment_status !== "paid" || !meta || typeof meta !== "object" || Array.isArray(meta)) {
    return { ok: false, status: 400, body: { error: "paid_checkout_metadata_required" } };
  }
  const metadata = meta as Record<string, unknown>;
  let skuId;
  try { skuId = parseSkuId(metadata.sku_id); } catch {
    return { ok: false, status: 400, body: { error: "invalid_fulfillment_sku" } };
  }
  if (!(skuId in prices)) return { ok: false, status: 422, body: { error: "automatic_fulfillment_unsupported_sku" } };
  const expected = prices[skuId as keyof WebhookPrices];
  if (!Number.isSafeInteger(expected) || expected <= 0 || obj.currency !== "usd" || !Number.isSafeInteger(obj.amount_total) || obj.amount_total !== expected) {
    return { ok: false, status: 400, body: { error: "checkout_amount_mismatch" } };
  }
  const optionalText = (value: unknown): string | undefined => {
    if (value === undefined || value === "") return undefined;
    if (typeof value !== "string" || value.length > 512 || !value.trim()) throw new Error("invalid_target");
    return value.trim();
  };
  let label: string | undefined;
  let smiles: string | undefined;
  try { label = optionalText(metadata.label); smiles = optionalText(metadata.smiles); } catch {
    return { ok: false, status: 400, body: { error: "invalid_checkout_target" } };
  }
  if ((skuId === "moladt.geometry" && !smiles) || (skuId !== "moladt.geometry" && !label && !smiles)) {
    return { ok: false, status: 400, body: { error: "checkout_target_required" } };
  }
  try {
    const token = await mintFulfillmentToken(store, { skuId, stripeRef: obj.id, label, smiles });
    return { ok: true, status: 200, token, body: {
      received: true, fulfilled: true, type, event_id: event.id, stripe_ref: obj.id,
      sku_id: token.sku_id, fulfillment_token: token.token, expires_at_unix: token.expires_at_unix,
      how_to_redeem: `PAYMENT-SIGNATURE: fulfill:${token.token}`,
    } };
  } catch (error) {
    if ((error as Error).message === "fulfillment_idempotency_conflict") return { ok: false, status: 409, body: { error: "fulfillment_idempotency_conflict" } };
    throw error;
  }
}

export async function processStripeWebhookRequest(opts: {
  rawBody: Buffer;
  signatureHeader: string | undefined;
  webhookSecret: string | null;
  store: FulfillmentStore;
  prices?: WebhookPrices;
}): Promise<StripeWebhookResult> {
  if (!opts.webhookSecret) {
    {
      return {
        ok: false,
        status: 503,
        body: {
          error: "stripe_webhook_unconfigured",
          message: "Set STRIPE_WEBHOOK_SECRET (from `stripe listen` or Dashboard).",
        },
      };
    }
  } else {
    const verified = verifyStripeSignature(
      opts.rawBody,
      opts.signatureHeader,
      opts.webhookSecret,
    );
    if (!verified.ok) {
      return {
        ok: false,
        status: 400,
        body: { error: "invalid_signature", reason: verified.reason },
      };
    }
  }

  let event: StripeEventLike;
  try {
    event = JSON.parse(opts.rawBody.toString("utf8")) as StripeEventLike;
  } catch {
    return {
      ok: false,
      status: 400,
      body: { error: "invalid_json" },
    };
  }

  return handleStripeEvent(opts.store, event, opts.prices);
}
