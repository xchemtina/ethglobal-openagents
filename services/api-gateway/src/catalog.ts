import type { GatewayConfig } from "./config.js";

export interface ServiceSku {
  sku_id: string;
  service_kind: string;
  title: string;
  description: string;
  price_usdc_micros: number;
  price_display: string;
  http_method: string;
  path: string;
  produces_schema_tags: string[];
  estimated_latency_seconds: number;
  network: string;
  mime_type: string;
  status: "live" | "coming_soon";
}

export interface PublicCatalog {
  catalog_id: string;
  version: string;
  provider_ens: string;
  pay_to: string;
  network: string;
  facilitator_url: string;
  mode: string;
  maturity: string;
  skus: ServiceSku[];
  agent_notes: string[];
  human_notes: string[];
}

export function buildCatalog(config: GatewayConfig): PublicCatalog {
  const network = config.network;
  return {
    catalog_id: "chimia.x402.catalog.v1",
    version: "0.1.0",
    provider_ens: "market.chimiaclaw.eth",
    pay_to: config.payTo,
    network,
    facilitator_url: config.facilitatorUrl,
    mode: config.mode,
    maturity: "scaffold-ready",
    skus: [
      {
        sku_id: "moladt.geometry",
        service_kind: "moladt",
        title: "MolADT geometry",
        description:
          "SMILES → signed chem.molecule.adt (curated library or RDKit worker)",
        price_usdc_micros: config.moladtPriceMicros,
        price_display: config.moladtPriceDisplay,
        http_method: "POST",
        path: "/v1/moladt",
        produces_schema_tags: ["chem.molecule.adt"],
        estimated_latency_seconds: 5,
        network,
        mime_type: "application/json",
        status: "live",
      },
      {
        sku_id: "literature.synthesis",
        service_kind: "literature",
        title: "Literature synthesis",
        description: "Query → signed science.literature.synthesis artifact",
        price_usdc_micros: 100_000,
        price_display: "$0.10",
        http_method: "POST",
        path: "/v1/literature",
        produces_schema_tags: ["science.literature.synthesis"],
        estimated_latency_seconds: 60,
        network,
        mime_type: "application/json",
        status: "coming_soon",
      },
      {
        sku_id: "dft.cached_result",
        service_kind: "dft",
        title: "Cached DFT result",
        description:
          "Retrieve a previously computed signed chem.dft.result by label, SMILES, or artifact id (see GET /v1/dft/index free)",
        price_usdc_micros: config.dftCachedPriceMicros,
        price_display: config.dftCachedPriceDisplay,
        http_method: "GET",
        path: "/v1/dft/cached",
        produces_schema_tags: ["chem.dft.result"],
        estimated_latency_seconds: 2,
        network,
        mime_type: "application/json",
        status: "live",
      },
      {
        sku_id: "dft.live_small",
        service_kind: "dft",
        title: "Live small-molecule DFT",
        description:
          "Operator-capped live DFT (Modal H100/A10G path via chimiaclaw-dft-modal; not free-run; atom/time/spend guards)",
        price_usdc_micros: 2_500_000,
        price_display: "$2.50",
        http_method: "POST",
        path: "/v1/dft/live",
        produces_schema_tags: ["chem.dft.result"],
        estimated_latency_seconds: 600,
        network,
        mime_type: "application/json",
        status: "coming_soon",
      },
    ],
    agent_notes: [
      "Unauthenticated calls to paid routes receive HTTP 402 with payment requirements.",
      "Retry with PAYMENT-SIGNATURE after settling USDC via x402.",
      "In stub mode, send header PAYMENT-SIGNATURE: stub (no funds moved).",
      "Successful responses include a signed ChimiaClaw artifact as the payload of record.",
    ],
    human_notes: [
      "This catalog powers the public website and agent discovery.",
      "pay_to must be a DAO-controlled Base address before enabling live mode.",
      "Website is a projection; signed artifacts remain the source of truth.",
    ],
  };
}
