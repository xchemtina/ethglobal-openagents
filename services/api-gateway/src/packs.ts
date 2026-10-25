/**
 * Sealed DFT packs — same-protocol series agents actually want to buy.
 * Built from the live cache; missing members are reported, not invented.
 */

import type { DftCacheEntry } from "./dft_cache.js";
import { findDftEntry } from "./dft_cache.js";

export interface PackDefinition {
  pack_id: string;
  title: string;
  description: string;
  /** Preferred labels; first match wins per member via findDftEntry */
  members: string[];
  /** Human/agent why this is scarce vs free local DFT */
  why_buy: string;
  protocol_hint: string;
}

export interface ResolvedPackMember {
  query: string;
  found: boolean;
  label?: string;
  artifact_id?: string;
  energy_hartree?: number | null;
  gap_ev?: number | null;
  functional?: string | null;
  basis_set?: string | null;
}

export interface ResolvedPack {
  pack_id: string;
  title: string;
  description: string;
  why_buy: string;
  protocol_hint: string;
  member_count_requested: number;
  member_count_available: number;
  complete: boolean;
  price_usdc_micros: number;
  price_display: string;
  unit_price_usdc_micros: number;
  discount_vs_singles: string;
  members: ResolvedPackMember[];
  status: "live" | "partial" | "empty";
}

/** Curated packs — expand as shelf grows. */
export const PACK_DEFINITIONS: PackDefinition[] = [
  {
    pack_id: "solvents.pbe",
    title: "Solvent set (PBE)",
    description:
      "Common solvents under one protocol — energy, gap, dipole when present.",
    members: [
      "water",
      "methanol",
      "acetone",
      "toluene",
      "pyridine",
      "benzene",
      "propylene glycol",
    ],
    why_buy:
      "Same functional/basis across a solvent ladder; useful for relative comparisons without re-running.",
    protocol_hint: "PBE / def2-TZVP (Olympus gallery / sealed cache)",
  },
  {
    pack_id: "aromatics.pbe",
    title: "Aromatics mini-set",
    description: "Benzene family reference set for agent baselines.",
    members: ["benzene", "toluene", "pyridine"],
    why_buy: "Cheap baseline pack for aromatic SMILES → property checks.",
    protocol_hint: "PBE / def2-TZVP sealed",
  },
  {
    pack_id: "fatty.acids.c8c10",
    title: "Caprylic / capric acids",
    description: "C8 and C10 carboxylic acids from sealed gallery.",
    members: ["caprylic acid", "capric acid"],
    why_buy: "Lipid-chain pair at fixed protocol.",
    protocol_hint: "PBE / def2-TZVP sealed",
  },
  {
    pack_id: "atrane.cages.c3opt",
    title: "Atrane cages (C3 + geomopt preferred)",
    description:
      "Closed atrane cages; prefers *-c3opt; Sn may be *-c3sp if geomopt failed.",
    members: [
      "germatrane h cage c3opt",
      "germatrane cl cage c3opt",
      "stannatrane h cage c3opt",
      "stannatrane cl cage c3opt",
      "stannatrane me cage c3opt",
      "stannatrane h cage c3sp",
      "stannatrane cl cage c3sp",
      "stannatrane me cage c3sp",
      // actual cashier labels from xyz-derived MolADT ids
      "nc3sn h cage opt",
      "nc3sn cl cage opt",
      "nc3sn me cage opt",
      // Last resort: pre-opt sealed cages
      "germatrane h cage",
      "germatrane cl cage",
      "stannatrane h cage",
      "stannatrane cl cage",
      "stannatrane me cage",
    ],
    why_buy:
      "Scarce geometry: closed cages, not open strands. Prefer C3-optimized (Ge); Sn C3-SP is honest when opt failed.",
    protocol_hint: "C3-cleaned; geomopt when converged; PBE sealed",
  },
  {
    pack_id: "weekly.pilot.v1",
    title: "Weekly pilot organics v1",
    description: "Small organics from weekly-drop pilot (same protocol batch).",
    members: [
      "aniline",
      "butyric acid",
      "dimethyl ether",
      "methylamine",
      "n-butanol",
      "n-hexanol",
      "n-propanol",
      "propionic acid",
    ],
    why_buy: "Agent-friendly batch of everyday organics under one seal pass.",
    protocol_hint: "Weekly pilot Olympus SCF",
  },
  {
    pack_id: "pg.diester.ladder",
    title: "Propylene glycol diester ladder",
    description: "PG diester chain-length series when sealed.",
    members: [
      "propylene glycol",
      "propylene glycol dibutyrate",
      "propylene glycol dihexanoate",
      "propylene glycol dioctanoate",
      "propylene glycol didecanoate",
      "propylene glycol dilauroate",
    ],
    why_buy: "Homologous series for structure–property trends.",
    protocol_hint: "PBE sealed where present",
  },
  {
    pack_id: "skala.flagship.v1",
    title: "Skala-1.1 flagship set",
    description:
      "Premium Microsoft Skala-1.1 SPs (cages, Ad–Ge, tip proxies). Sealed as skala-* labels.",
    members: [
      "skala adamantane",
      "skala ad geh3",
      "skala ad gecl3",
      "skala ad geme3",
      "skala ad germatrane",
      "skala ad snh3",
      "skala ad sncl3",
      "skala ad snme3",
      "skala me3ge c2h",
      "skala h3si c2h",
      "skala germatrane h cage c3opt",
      "skala germatrane cl cage c3opt",
      "skala germatrane me cage",
      "skala silatrane h cage",
      "skala silatrane cl cage",
      "skala silatrane me cage",
      "skala stannatrane h cage",
      "skala stannatrane cl cage",
      "skala stannatrane me cage",
    ],
    why_buy:
      "Hybrid-level XC story; signed chem.dft.result with source_kind pyscf-skala-1.1.",
    protocol_hint: "Skala-1.1 · Olympus",
  },
  {
    pack_id: "skala.atrane.core",
    title: "Skala atrane cages (Ge/Si/Sn)",
    description: "H/Cl/Me cage ladder at Skala-1.1 where sealed.",
    members: [
      "skala germatrane h cage c3opt",
      "skala germatrane cl cage c3opt",
      "skala germatrane me cage",
      "skala silatrane h cage",
      "skala silatrane cl cage",
      "skala silatrane me cage",
      "skala stannatrane h cage",
      "skala stannatrane cl cage",
      "skala stannatrane me cage",
    ],
    why_buy: "Scarce cage series under one premium XC.",
    protocol_hint: "Skala-1.1 sealed",
  },
  {
    pack_id: "skala.ad_ge.v1",
    title: "Skala Ad–Ge / Ad–Sn ladder",
    description:
      "Adamantane + Ad–Ge/Sn X3 + germatrane + closed-shell tip proxies.",
    members: [
      "skala adamantane",
      "skala ad geh3",
      "skala ad gecl3",
      "skala ad geme3",
      "skala ad germatrane",
      "skala ad snh3",
      "skala ad sncl3",
      "skala ad snme3",
      "skala me3ge c2h",
      "skala h3si c2h",
    ],
    why_buy: "Paper-aligned main-group ladder at Skala-1.1 (closed-shell only).",
    protocol_hint: "Skala-1.1 · arXiv-inspired tip proxies",
  },
];

function microsToDisplay(micros: number): string {
  const dollars = micros / 1_000_000;
  return `$${dollars.toFixed(dollars >= 1 ? 2 : 2)}`;
}

/**
 * Pack price: per-available-member × unit price × discount (default 0.8).
 * Incomplete packs still sell what is present (honest partial).
 */
export function resolvePack(
  def: PackDefinition,
  cache: DftCacheEntry[],
  unitPriceMicros: number,
  discount = 0.8,
): ResolvedPack {
  const seenArtifacts = new Set<string>();
  const members: ResolvedPackMember[] = [];

  for (const query of def.members) {
    const entry = findDftEntry(cache, query);
    if (!entry) {
      members.push({ query, found: false });
      continue;
    }
    if (seenArtifacts.has(entry.artifact_id)) {
      // Alias/fallback already included under another query
      members.push({
        query,
        found: true,
        label: entry.label,
        artifact_id: entry.artifact_id,
        energy_hartree: entry.energy_hartree,
        gap_ev: entry.gap_ev,
        functional: entry.functional,
        basis_set: entry.basis_set,
      });
      continue;
    }
    seenArtifacts.add(entry.artifact_id);
    members.push({
      query,
      found: true,
      label: entry.label,
      artifact_id: entry.artifact_id,
      energy_hartree: entry.energy_hartree,
      gap_ev: entry.gap_ev,
      functional: entry.functional,
      basis_set: entry.basis_set,
    });
  }

  const uniqueAvailable = [...seenArtifacts].length;
  const requestedUnique = new Set(def.members.map((m) => m.toLowerCase())).size;
  // For cages pack with fallbacks, "complete" means ≥3 unique cage artifacts
  const complete =
    def.pack_id === "atrane.cages.c3opt"
      ? uniqueAvailable >= 3
      : uniqueAvailable >= Math.min(def.members.length, requestedUnique) &&
        uniqueAvailable > 0;

  const priceMicros = Math.max(
    0,
    Math.round(uniqueAvailable * unitPriceMicros * discount),
  );

  let status: ResolvedPack["status"] = "empty";
  if (uniqueAvailable === 0) status = "empty";
  else if (complete) status = "live";
  else status = "partial";

  const fullSingles = uniqueAvailable * unitPriceMicros;
  const saved = fullSingles - priceMicros;

  return {
    pack_id: def.pack_id,
    title: def.title,
    description: def.description,
    why_buy: def.why_buy,
    protocol_hint: def.protocol_hint,
    member_count_requested: def.members.length,
    member_count_available: uniqueAvailable,
    complete,
    price_usdc_micros: priceMicros,
    price_display: microsToDisplay(priceMicros),
    unit_price_usdc_micros: unitPriceMicros,
    discount_vs_singles:
      uniqueAvailable > 0
        ? `${Math.round(discount * 100)}% of singles (save ${microsToDisplay(saved)})`
        : "n/a",
    members,
    status,
  };
}

export function listResolvedPacks(
  cache: DftCacheEntry[],
  unitPriceMicros: number,
): ResolvedPack[] {
  return PACK_DEFINITIONS.map((d) => resolvePack(d, cache, unitPriceMicros));
}

export function getPackDefinition(packId: string): PackDefinition | null {
  return PACK_DEFINITIONS.find((p) => p.pack_id === packId) ?? null;
}

export function collectPackEntries(
  def: PackDefinition,
  cache: DftCacheEntry[],
): DftCacheEntry[] {
  const out: DftCacheEntry[] = [];
  const seen = new Set<string>();
  for (const query of def.members) {
    const entry = findDftEntry(cache, query);
    if (!entry || seen.has(entry.artifact_id)) continue;
    seen.add(entry.artifact_id);
    out.push(entry);
  }
  return out;
}
