/**
 * Agent DFT job board — request what you need; buy if sealed; queue if not.
 * Does not invent results. Live SCF stays operator-gated.
 */

import { appendFile, mkdir, readFile } from "node:fs/promises";
import { randomBytes } from "node:crypto";
import path from "node:path";
import type { DftCacheEntry } from "./dft_cache.js";
import { findDftEntry } from "./dft_cache.js";

export type JobStatus =
  | "available_now"
  | "queued"
  | "rejected"
  | "fulfilled_offline";

export interface DftJob {
  job_id: string;
  created_at_unix: number;
  status: JobStatus;
  /** SMILES and/or label and/or xyz snippet */
  smiles?: string;
  label?: string;
  xyz?: string;
  charge: number;
  multiplicity: number;
  method: string;
  basis: string;
  geomopt: boolean;
  notes?: string;
  /** If cache hit */
  cache_label?: string;
  cache_artifact_id?: string;
  /** How to buy if available */
  buy?: {
    sku_id: string;
    path: string;
    price_display: string;
  };
  reject_reason?: string;
}

export interface JobStore {
  path: string;
}

async function loadJobs(storePath: string): Promise<Map<string, DftJob>> {
  const cache = new Map<string, DftJob>();
  let text: string;
  try { text = await readFile(storePath, "utf8"); } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return cache;
    throw error;
  }
  if (text && !text.endsWith("\n")) throw new Error("incomplete_job_ledger");
  for (const line of text.split("\n")) {
    if (!line) continue;
    const row = JSON.parse(line) as DftJob;
    if (!row || !/^job_[a-f0-9]{24}$/.test(row.job_id)) throw new Error("corrupt_job_ledger");
    cache.set(row.job_id, row);
  }
  return cache;
}
const jobQueues = new Map<string, Promise<void>>();
async function appendJob(storePath: string, job: DftJob): Promise<void> {
  const key = path.resolve(storePath);
  const previous = jobQueues.get(key) ?? Promise.resolve();
  const operation = previous.catch(() => {}).then(async () => {
    await mkdir(path.dirname(key), { recursive: true, mode: 0o700 });
    await appendFile(key, `${JSON.stringify(job)}\n`, { encoding: "utf8", mode: 0o600 });
  });
  jobQueues.set(key, operation);
  try { await operation; } finally { if (jobQueues.get(key) === operation) jobQueues.delete(key); }
}

function newJobId(): string {
  return `job_${randomBytes(12).toString("hex")}`;
}

function atomCountFromXyz(xyz: string): number | null {
  const lines = xyz.trim().split(/\r?\n/);
  if (lines.length < 3) return null;
  const n = Number(lines[0].trim());
  return Number.isFinite(n) && n > 0 ? n : null;
}

export interface CreateJobInput {
  smiles?: string;
  label?: string;
  xyz?: string;
  charge?: number;
  multiplicity?: number;
  method?: string;
  basis?: string;
  geomopt?: boolean;
  notes?: string;
}

/** True if cache entry is premium Skala-1.1 shelf. */
export function isSkalaEntry(entry: DftCacheEntry): boolean {
  return (
    entry.label.toLowerCase().includes("skala") ||
    (entry.functional || "").toLowerCase().includes("skala")
  );
}

/**
 * Resolve request against cache; queue if missing and not too large/ill-formed.
 */
export async function createDftJob(
  store: JobStore,
  dftCache: DftCacheEntry[],
  input: CreateJobInput,
  opts: {
    priceDisplay: string;
    maxAtomsQueued: number;
    /** Skala unit price when cache hit is premium */
    skalaPriceDisplay?: string;
  },
): Promise<DftJob> {


  const smiles = input.smiles?.trim() || undefined;
  const label = input.label?.trim() || undefined;
  const xyz = input.xyz?.trim() || undefined;

  if (!smiles && !label && !xyz) {
    const job: DftJob = {
      job_id: newJobId(),
      created_at_unix: Math.floor(Date.now() / 1000),
      status: "rejected",
      charge: input.charge ?? 0,
      multiplicity: input.multiplicity ?? 1,
      method: input.method ?? "pbe",
      basis: input.basis ?? "def2-tzvp",
      geomopt: Boolean(input.geomopt),
      reject_reason: "Provide smiles, label, or xyz",
    };
    await appendJob(store.path, job);
    return job;
  }

  // Cache hit?
  const query = label || smiles || "";
  const candidate = query ? findDftEntry(dftCache, query, smiles && !label ? "smiles" : "auto") : null;
  // A stored calculation with another requested method/basis is not a cache hit.
  const entry = candidate && (!input.method || candidate.functional?.toLowerCase() === input.method.toLowerCase()) && (!input.basis || candidate.basis_set?.toLowerCase() === input.basis.toLowerCase()) && !input.geomopt && !input.xyz && (!input.smiles || candidate.smiles === input.smiles.trim()) && input.charge === undefined && input.multiplicity === undefined ? candidate : null;
  if (entry) {
    const skala = isSkalaEntry(entry);
    const price =
      skala && opts.skalaPriceDisplay
        ? opts.skalaPriceDisplay
        : opts.priceDisplay;
    const sku = skala ? "dft.skala_cached" : "dft.cached_result";
    const path = skala
      ? `/v1/dft/skala?label=${encodeURIComponent(entry.label)}`
      : `/v1/dft/cached?label=${encodeURIComponent(entry.label)}`;
    const job: DftJob = {
      job_id: newJobId(),
      created_at_unix: Math.floor(Date.now() / 1000),
      status: "available_now",
      smiles,
      label: label ?? entry.label,
      xyz,
      charge: input.charge ?? 0,
      multiplicity: input.multiplicity ?? 1,
      method: input.method ?? entry.functional ?? "pbe",
      basis: input.basis ?? entry.basis_set ?? "def2-tzvp",
      geomopt: Boolean(input.geomopt),
      notes: input.notes,
      cache_label: entry.label,
      cache_artifact_id: entry.artifact_id,
      buy: {
        sku_id: sku,
        path,
        price_display: price,
      },
    };
    await appendJob(store.path, job);
    return job;
  }

  // Size guard for queue (xyz only)
  if (xyz) {
    const n = atomCountFromXyz(xyz);
    if (n != null && n > opts.maxAtomsQueued) {
      const job: DftJob = {
        job_id: newJobId(),
        created_at_unix: Math.floor(Date.now() / 1000),
        status: "rejected",
        smiles,
        label,
        xyz: xyz.slice(0, 500),
        charge: input.charge ?? 0,
        multiplicity: input.multiplicity ?? 1,
        method: input.method ?? "pbe",
        basis: input.basis ?? "def2-tzvp",
        geomopt: Boolean(input.geomopt),
        notes: input.notes,
        reject_reason: `xyz has ${n} atoms; public queue max is ${opts.maxAtomsQueued}`,
      };
      await appendJob(store.path, job);
      return job;
    }
  }

  const job: DftJob = {
    job_id: newJobId(),
    created_at_unix: Math.floor(Date.now() / 1000),
    status: "queued",
    smiles,
    label,
    xyz: xyz ? xyz.slice(0, 200_000) : undefined,
    charge: input.charge ?? 0,
    multiplicity: input.multiplicity ?? 1,
    method: (input.method ?? "pbe").toLowerCase(),
    basis: (input.basis ?? "def2-tzvp").toLowerCase(),
    geomopt: Boolean(input.geomopt),
    notes: input.notes,
  };
  await appendJob(store.path, job);
  return job;
}

export async function getDftJob(
  store: JobStore,
  jobId: string,
): Promise<DftJob | null> {

  return (await loadJobs(store.path)).get(jobId) ?? null;
}

export async function listRecentJobs(
  store: JobStore,
  limit = 50,
): Promise<DftJob[]> {

  return [...(await loadJobs(store.path)).values()]
    .sort((a, b) => b.created_at_unix - a.created_at_unix)
    .slice(0, limit);
}

export interface ResolvePriceOpts {
  classicalPriceDisplay: string;
  classicalPriceMicros: number;
  skalaPriceDisplay: string;
  skalaPriceMicros: number;
  /** Credits spent per Skala single (default 4 = $20 / $5) */
  skalaCreditCost?: number;
}

/** Free resolve: what can I buy for this query right now? */
export function resolveQuery(
  dftCache: DftCacheEntry[],
  query: string,
  priceDisplayOrOpts: string | ResolvePriceOpts,
  priceMicros?: number,
): Record<string, unknown> {
  const opts: ResolvePriceOpts =
    typeof priceDisplayOrOpts === "string"
      ? {
          classicalPriceDisplay: priceDisplayOrOpts,
          classicalPriceMicros: priceMicros ?? 5_000_000,
          skalaPriceDisplay: "$20.00",
          skalaPriceMicros: 20_000_000,
          skalaCreditCost: 4,
        }
      : {
          skalaCreditCost: 4,
          ...priceDisplayOrOpts,
        };

  const q = query.trim();
  if (!q) {
    return { ok: false, error: "empty_query" };
  }
  const entry = findDftEntry(dftCache, q);
  if (entry) {
    const skala = isSkalaEntry(entry);
    const priceDisplay = skala
      ? opts.skalaPriceDisplay
      : opts.classicalPriceDisplay;
    const priceUsdc = skala
      ? opts.skalaPriceMicros
      : opts.classicalPriceMicros;
    const sku = skala ? "dft.skala_cached" : "dft.cached_result";
    const path = skala
      ? `/v1/dft/skala?label=${encodeURIComponent(entry.label)}`
      : `/v1/dft/cached?label=${encodeURIComponent(entry.label)}`;
    return {
      ok: true,
      status: "in_cache",
      query: q,
      label: entry.label,
      tier: skala ? "skala" : "classical",
      artifact_id: entry.artifact_id,
      summary: {
        functional: entry.functional,
        basis_set: entry.basis_set,
        energy_hartree: entry.energy_hartree,
        gap_ev: entry.gap_ev,
      },
      buy: {
        sku_id: sku,
        method: "GET",
        path,
        price_display: priceDisplay,
        price_usdc_micros: priceUsdc,
        payment: "PAYMENT-SIGNATURE: stub | fulfill:ft_… | x402 signature",
        credit_cost: skala ? (opts.skalaCreditCost ?? 4) : 1,
      },
      agent_loop: [
        `GET ${path} without payment → 402`,
        "Settle (stub / credit / fulfill / x402)",
        "Retry with PAYMENT-SIGNATURE → signed chem.dft.result",
      ],
    };
  }
  return {
    ok: true,
    status: "not_in_cache",
    query: q,
    buy: null,
    next: {
      submit_job: {
        method: "POST",
        path: "/v1/dft/jobs",
        body: {
          smiles: q.includes("=") || q.length < 40 ? q : undefined,
          label: q,
          method: "pbe",
          basis: "def2-tzvp",
          geomopt: false,
        },
      },
      browse: "/v1/dft/index",
      packs: "/v1/dft/packs",
      skala_index: "/v1/dft/index?tier=skala",
    },
    notes: [
      "We do not invent DFT. Queue a job or buy a sealed pack of related molecules.",
      "Queued jobs are operator-fulfilled offline; poll GET /v1/dft/jobs/:id",
      "Skala-1.1 premium labels use GET /v1/dft/skala ($20 or 4 credits).",
    ],
  };
}
