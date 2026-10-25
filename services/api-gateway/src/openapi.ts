import type { GatewayConfig } from "./config.js";

/** Minimal OpenAPI 3 surface for agents. */
export function buildOpenApi(config: GatewayConfig): Record<string, unknown> {
  const server = config.publicBaseUrl.replace(/\/$/, "");
  return {
    openapi: "3.0.3",
    info: {
      title: "ChimiaClaw agent DFT cashier",
      version: "0.2.0",
      description:
        "Agent-buyable sealed DFT. Preferred loop: resolve → packs/credits → cached buy; else queue job. No invented results.",
    },
    servers: [{ url: server }],
    paths: {
      "/health": {
        get: {
          summary: "Liveness + payment mode",
          responses: { "200": { description: "OK" } },
        },
      },
      "/v1/catalog": {
        get: {
          summary: "SKU catalog (free)",
          responses: { "200": { description: "Catalog JSON" } },
        },
      },
      "/v1/dft/index": {
        get: {
          summary: "Cached DFT labels (free)",
          parameters: [
            {
              name: "tier",
              in: "query",
              required: false,
              schema: { type: "string", enum: ["classical", "skala", ""] },
              description: "Filter: classical | skala | omit for all",
            },
          ],
          responses: { "200": { description: "Index" } },
        },
      },
      "/v1/dft/resolve": {
        get: {
          summary: "Free: is this query buyable right now?",
          description:
            "Returns tier (classical|skala), buy path, price, credit_cost (1 classical / 4 Skala).",
          parameters: [
            {
              name: "q",
              in: "query",
              required: true,
              schema: { type: "string" },
              description: "label | SMILES | artifact id",
            },
          ],
          responses: {
            "200": {
              description: "in_cache with buy path, or not_in_cache with job template",
            },
          },
        },
      },
      "/v1/dft/skala": {
        get: {
          summary: "Paid: Skala-1.1 sealed chem.dft.result ($20 / 4 credits)",
          parameters: [
            {
              name: "label",
              in: "query",
              required: true,
              schema: { type: "string" },
            },
            {
              name: "PAYMENT-SIGNATURE",
              in: "header",
              schema: { type: "string" },
              description: "stub | fulfill:ft_… | x402",
            },
          ],
          responses: {
            "200": { description: "Signed Skala result" },
            "402": { description: "Payment Required" },
            "503": { description: "Live x402 unavailable" },
            "404": { description: "Not in cache" },
          },
        },
      },
      "/v1/dft/packs": {
        get: {
          summary: "Free: sealed same-protocol packs (agent-preferred SKUs)",
          responses: { "200": { description: "Pack list + prices" } },
        },
      },
      "/v1/dft/packs/{pack_id}": {
        get: {
          summary: "Paid: download all available signed results in pack",
          parameters: [
            {
              name: "pack_id",
              in: "path",
              required: true,
              schema: { type: "string" },
            },
            {
              name: "PAYMENT-SIGNATURE",
              in: "header",
              schema: { type: "string" },
            },
          ],
          responses: {
            "200": { description: "Pack results" },
            "402": { description: "Payment Required" },
            "503": { description: "Live x402 unavailable" },
          },
        },
      },
      "/v1/dft/credits": {
        get: {
          summary: "Free: multi-use credit bundle pricing + agent loop",
          responses: { "200": { description: "Credit table" } },
        },
      },
      "/v1/dft/jobs": {
        post: {
          summary:
            "Free: request DFT — cache hit → buy path; else queue for offline seal",
          requestBody: {
            content: {
              "application/json": {
                schema: {
                  type: "object",
                  properties: {
                    smiles: { type: "string" },
                    label: { type: "string" },
                    xyz: { type: "string" },
                    method: { type: "string" },
                    basis: { type: "string" },
                    geomopt: { type: "boolean" },
                    charge: { type: "integer" },
                    multiplicity: { type: "integer" },
                  },
                },
              },
            },
          },
          responses: {
            "200": { description: "Job created (available_now | queued)" },
            "400": { description: "Rejected" },
          },
        },
      },
      "/v1/dft/jobs/{job_id}": {
        get: {
          summary: "Poll job status",
          parameters: [
            {
              name: "job_id",
              in: "path",
              required: true,
              schema: { type: "string" },
            },
          ],
          responses: { "200": { description: "Job" }, "404": { description: "Missing" } },
        },
      },
      "/v1/dft/cached": {
        get: {
          summary:
            "Paid: classical sealed chem.dft.result ($5 / 1 credit). Skala labels → use /v1/dft/skala",
          parameters: [
            { name: "label", in: "query", schema: { type: "string" } },
            {
              name: "PAYMENT-SIGNATURE",
              in: "header",
              schema: { type: "string" },
            },
          ],
          responses: {
            "200": { description: "Paid result" },
            "402": { description: "Payment Required" },
            "503": { description: "Live x402 unavailable" },
          },
        },
      },
      "/v1/moladt": {
        post: {
          summary: "SMILES → signed MolADT (paid)",
          responses: {
            "200": { description: "Paid result" },
            "402": { description: "Payment Required" },
            "503": { description: "Live x402 unavailable" },
          },
        },
      },
      "/v1/checkout": {
        post: {
          summary: "Human checkout (Stripe / Unstoppable)",
          responses: { "200": { description: "Checkout payload" } },
        },
      },
      "/v1/operator/mint-fulfillment": {
        post: {
          summary:
            "Mint fulfill / pack / multi-use credit token (X-Operator-Secret)",
          responses: { "200": { description: "Token" } },
        },
      },
      "/v1/fulfillment/{token}": {
        get: {
          summary: "Inspect fulfillment / credit token",
          parameters: [
            {
              name: "token",
              in: "path",
              required: true,
              schema: { type: "string" },
            },
          ],
          responses: { "200": { description: "Status" } },
        },
      },
    },
    "x-agent-buy-loop": {
      mode: config.mode,
      steps: [
        "GET /v1/dft/resolve?q=<smiles|label>",
        "if in_cache: GET buy.path with PAYMENT-SIGNATURE",
        "if pack better: GET /v1/dft/packs then GET /v1/dft/packs/:id paid",
        "if not_in_cache: POST /v1/dft/jobs → poll",
        "prefer dft.credit multi-use tokens for repeated singles",
      ],
    },
  };
}
