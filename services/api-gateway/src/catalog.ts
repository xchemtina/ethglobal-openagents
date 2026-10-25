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
  status: "live" | "unavailable" | "coming_soon";
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
        status: config.mode === "live" ? "unavailable" : "live",
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
        title: "Cached DFT result (classical)",
        description:
          "PBE/B3LYP sealed chem.dft.result by label (GET /v1/dft/index). Premium Skala: dft.skala_cached.",
        price_usdc_micros: config.dftCachedPriceMicros,
        price_display: config.dftCachedPriceDisplay,
        http_method: "GET",
        path: "/v1/dft/cached",
        produces_schema_tags: ["chem.dft.result"],
        estimated_latency_seconds: 2,
        network,
        mime_type: "application/json",
        status: config.mode === "live" ? "unavailable" : "live",
      },
      {
        sku_id: "dft.skala_cached",
        service_kind: "dft",
        title: "Skala-1.1 cached DFT",
        description:
          "Microsoft Skala-1.1 signed chem.dft.result (premium). Labels skala-* from GET /v1/dft/index?tier=skala. GET /v1/dft/skala?label=…",
        price_usdc_micros: config.dftSkalaPriceMicros,
        price_display: config.dftSkalaPriceDisplay,
        http_method: "GET",
        path: "/v1/dft/skala",
        produces_schema_tags: ["chem.dft.result"],
        estimated_latency_seconds: 2,
        network,
        mime_type: "application/json",
        status: config.mode === "live" ? "unavailable" : "live",
      },
      {
        sku_id: "dft.pack",
        service_kind: "dft",
        title: "Sealed DFT pack (series)",
        description:
          "Same-protocol multi-molecule pack — solvents, cages, Skala flagship. GET /v1/dft/packs then buy /v1/dft/packs/:id",
        price_usdc_micros: Math.round(config.dftCachedPriceMicros * 3 * 0.8),
        price_display: "from pack list",
        http_method: "GET",
        path: "/v1/dft/packs/{pack_id}",
        produces_schema_tags: ["chem.dft.result", "chem.dft.pack"],
        estimated_latency_seconds: 3,
        network,
        mime_type: "application/json",
        status: config.mode === "live" ? "unavailable" : "live",
      },
      {
        sku_id: "dft.credit",
        service_kind: "dft",
        title: "Agent DFT credits (multi-use)",
        description:
          "N redemptions against GET /v1/dft/cached without re-minting per molecule. Operator mints dft.credit tokens.",
        price_usdc_micros: Math.round(config.dftCachedPriceMicros * 5 * 0.85),
        price_display: "bundle (see /v1/dft/credits)",
        http_method: "GET",
        path: "/v1/dft/cached",
        produces_schema_tags: ["chem.dft.result"],
        estimated_latency_seconds: 2,
        network,
        mime_type: "application/json",
        status: config.mode === "live" ? "unavailable" : "live",
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
      "Preferred loop: GET /v1/dft/resolve?q=… → buy classical ($5) or Skala ($20).",
      "Skala tier: GET /v1/dft/index?tier=skala then GET /v1/dft/skala?label=skala-…",
      "Packs: GET /v1/dft/packs (skala.flagship.v1 is the premium series).",
      "Unauthenticated paid routes return HTTP 402 with PAYMENT-REQUIRED.",
      "Stub: PAYMENT-SIGNATURE: stub. Credits/fulfill: PAYMENT-SIGNATURE: fulfill:ft_…",
      "We never invent orbitals or energies — not_in_cache means queue or pick another label.",
    ],
    human_notes: [
      "Classical sealed DFT $5 · Skala-1.1 premium $20 · packs listed on /v1/dft/packs.",
      "Human checkout: Stripe Payment Links via POST /v1/checkout method=stripe.",
      "After pay, operator mints ft_… — agents prefer dft.credit or Skala fulfill tokens.",
      "Live custom DFT stays operator-gated. Signed chem.dft.result is source of truth.",
    ],
  };
}
