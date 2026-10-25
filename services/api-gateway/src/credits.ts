/**
 * Multi-use agent credits — mint N redemptions for GET /v1/dft/cached.
 */

import {
  mintFulfillmentToken,
  type FulfillmentStore,
  type FulfillmentToken,
} from "./fulfillment.js";

export const CREDIT_SKU = "dft.credit" as const;
export const PACK_SKU = "dft.pack" as const;

export async function mintCreditToken(
  store: FulfillmentStore,
  opts: {
    redemptions: number;
    stripeRef: string;
    ttlSeconds?: number;
  },
): Promise<FulfillmentToken> {
  if (!Number.isSafeInteger(opts.redemptions) || opts.redemptions < 1 || opts.redemptions > 100) throw new Error("invalid_redemptions");
  const n = opts.redemptions;
  return mintFulfillmentToken(store, {
    skuId: CREDIT_SKU,
    stripeRef: opts.stripeRef,
    ttlSeconds: opts.ttlSeconds,
    remaining_redemptions: n,
  });
}

export async function mintPackToken(
  store: FulfillmentStore,
  opts: {
    packId: string;
    stripeRef: string;
    ttlSeconds?: number;
  },
): Promise<FulfillmentToken> {
  return mintFulfillmentToken(store, {
    skuId: PACK_SKU,
    stripeRef: opts.stripeRef,
    packId: opts.packId,
    ttlSeconds: opts.ttlSeconds,
  });
}

/** Catalog price for N credits (slight bulk discount). */
export function creditBundlePriceMicros(
  unitMicros: number,
  redemptions: number,
  discount = 0.85,
): number {
  const n = Math.max(1, redemptions);
  return Math.round(n * unitMicros * discount);
}
