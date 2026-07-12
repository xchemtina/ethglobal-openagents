/**
 * ChimiaClaw / x402 API gateway client.
 * Contract: docs/X402.md + web/README.contract.md
 */

export type GatewayMode = "free" | "stub" | "live" | string;

export interface GatewayHealth {
  ok: boolean;
  service?: string;
  mode?: GatewayMode;
  network?: string;
  pay_to?: string;
  payment_methods?: Array<{ id: string; status: string }>;
  deployable_hint?: boolean;
}

export interface CatalogSku {
  sku_id: string;
  service_kind: string;
  title: string;
  description: string;
  price_usdc_micros: number;
  price_display: string;
  http_method: string;
  path: string;
  status: "live" | "coming_soon" | string;
  produces_schema_tags?: string[];
}

export interface PublicCatalog {
  catalog_id: string;
  version: string;
  provider_ens: string;
  pay_to: string;
  network: string;
  mode: string;
  skus: CatalogSku[];
  agent_notes?: string[];
  human_notes?: string[];
}

export interface DftIndexItem {
  label: string;
  artifact_id: string;
  smiles: string | null;
  energy_hartree: number | null;
  gap_ev: number | null;
  functional: string | null;
  basis_set: string | null;
}

export interface DftIndex {
  ok: boolean;
  count: number;
  price_display?: string;
  items: DftIndexItem[];
}

export function apiBase(): string {
  return (
    process.env.NEXT_PUBLIC_API_BASE?.replace(/\/$/, "") ||
    "http://127.0.0.1:4021"
  );
}

async function getJson<T>(path: string, init?: RequestInit): Promise<T> {
  const url = `${apiBase()}${path.startsWith("/") ? path : `/${path}`}`;
  const res = await fetch(url, {
    ...init,
    headers: {
      accept: "application/json",
      ...(init?.headers ?? {}),
    },
    // browser calls to local gateway
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`${path} → HTTP ${res.status}`);
  }
  return (await res.json()) as T;
}

export function fetchHealth(): Promise<GatewayHealth> {
  return getJson<GatewayHealth>("/health");
}

export function fetchCatalog(): Promise<PublicCatalog> {
  return getJson<PublicCatalog>("/v1/catalog");
}

export function fetchDftIndex(): Promise<DftIndex> {
  return getJson<DftIndex>("/v1/dft/index");
}

/** Stub-paid MolADT call for demo UI (no real funds). */
export async function fetchMoladtStub(smiles: string): Promise<unknown> {
  const url = `${apiBase()}/v1/moladt`;
  const res = await fetch(url, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "PAYMENT-SIGNATURE": "stub",
    },
    body: JSON.stringify({ smiles, no_worker: true }),
  });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new Error(
      (body as { message?: string; error?: string }).message ||
        (body as { error?: string }).error ||
        `moladt HTTP ${res.status}`,
    );
  }
  return body;
}

export function displayMode(): string {
  return process.env.NEXT_PUBLIC_X402_MODE || "stub";
}

export function displayPayTo(): string {
  return process.env.NEXT_PUBLIC_PAY_TO || "0xYourDaoTreasuryOnBase";
}
