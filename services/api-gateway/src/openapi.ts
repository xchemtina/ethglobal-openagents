import type { GatewayConfig } from "./config.js";

/** Minimal OpenAPI 3 surface for agents. */
export function buildOpenApi(config: GatewayConfig): Record<string, unknown> {
  const server = config.publicBaseUrl.replace(/\/$/, "");
  return {
    openapi: "3.0.3",
    info: {
      title: "ChimiaClaw / DFT.xyz API gateway",
      version: "0.1.0",
      description:
        "Agentic science cashier. Primary path: x402 HTTP 402. Humans may use Stripe or Revolut when configured.",
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
      "/v1/payment-methods": {
        get: {
          summary: "x402 + Stripe + Revolut rails",
          responses: { "200": { description: "Methods + honesty notes" } },
        },
      },
      "/.well-known/x402": {
        get: {
          summary: "x402 discovery document",
          responses: { "200": { description: "Discovery" } },
        },
      },
      "/v1/dft/index": {
        get: {
          summary: "Cached DFT labels (free)",
          responses: { "200": { description: "Index" } },
        },
      },
      "/v1/moladt": {
        post: {
          summary: "SMILES → signed MolADT (paid)",
          parameters: [
            {
              name: "PAYMENT-SIGNATURE",
              in: "header",
              required: false,
              schema: { type: "string" },
              description: "stub in stub mode; real x402 payload in live mode",
            },
          ],
          requestBody: {
            required: true,
            content: {
              "application/json": {
                schema: {
                  type: "object",
                  required: ["smiles"],
                  properties: {
                    smiles: { type: "string" },
                    no_worker: { type: "boolean" },
                  },
                },
              },
            },
          },
          responses: {
            "200": { description: "Paid result" },
            "402": { description: "Payment Required" },
          },
        },
      },
      "/v1/dft/cached": {
        get: {
          summary: "Cached signed DFT result (paid)",
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
          },
        },
      },
      "/v1/checkout": {
        post: {
          summary: "Human checkout (Stripe or Revolut instructions)",
          responses: { "200": { description: "Checkout payload" } },
        },
      },
    },
  };
}
