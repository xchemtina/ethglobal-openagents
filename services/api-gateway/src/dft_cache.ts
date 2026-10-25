import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

export interface DftCacheEntry {
  artifact_id: string;
  label: string;
  aliases: string[];
  smiles: string | null;
  molecule_id: string | null;
  molecule_name: string | null;
  functional: string | null;
  basis_set: string | null;
  energy_hartree: number | null;
  gap_ev: number | null;
  dipole_debye: number | null;
  topic: string | null;
  source_path: string;
  result_payload: Record<string, unknown>;
  request_payload: Record<string, unknown> | null;
  artifact: Record<string, unknown>;
}

export interface DftCacheIndexItem {
  label: string;
  aliases: string[];
  artifact_id: string;
  smiles: string | null;
  molecule_id: string | null;
  functional: string | null;
  basis_set: string | null;
  energy_hartree: number | null;
  gap_ev: number | null;
  dipole_debye: number | null;
}

interface ArtifactEnvelope {
  id?: string;
  topic?: string;
  parent_artifact_ids?: string[];
  schema_tags?: string[];
  payload?: {
    encoding?: string;
    location?: {
      Inline?: { bytes_hex?: string };
    };
  };
  [key: string]: unknown;
}

const COMMON_ALIASES: Record<string, string[]> = {
  water: ["h2o"],
  methanol: ["ch3oh", "meoh"],
  ethanol: ["etoh", "ch3ch2oh"],
  acetone: ["ch3coch3"],
  toluene: ["phme"],
  pyridine: [],
  benzene: ["c6h6"],
  // C3-SP Sn seals (CLI labeled from NC3Sn_*_opt.xyz — keep buyable under honest names)
  "nc3sn h cage opt": [
    "stannatrane h cage c3sp",
    "stannatrane-h-cage-c3sp",
    "nc3sn_h_cage_c3sp",
  ],
  "nc3sn cl cage opt": [
    "stannatrane cl cage c3sp",
    "stannatrane-cl-cage-c3sp",
  ],
  "nc3sn me cage opt": [
    "stannatrane me cage c3sp",
    "stannatrane-me-cage-c3sp",
  ],
  "propylene glycol": [
    "propyleneglycol",
    "propane-1,2-diol",
    "1,2-propanediol",
    "pg",
  ],
  "caprylic acid": ["c8", "octanoic acid", "octanoic"],
  "capric acid": ["c10", "decanoic acid", "decanoic"],
};

function normalizeKey(value: string): string {
  return value.trim().toLowerCase().replace(/[_-]+/g, " ").replace(/\s+/g, " ");
}

function decodeInlineJson(
  artifact: ArtifactEnvelope,
): Record<string, unknown> | null {
  const hex = artifact.payload?.location?.Inline?.bytes_hex;
  if (!hex) {
    return null;
  }
  try {
    const json = Buffer.from(hex, "hex").toString("utf8");
    return JSON.parse(json) as Record<string, unknown>;
  } catch {
    return null;
  }
}

function slugFromTopic(topic: string | undefined): string | null {
  if (!topic) {
    return null;
  }
  // e.g. "DFT result REQ.MOLADT.DFT.WATER.001 (pbe/def2-tzvp)"
  const match = topic.match(/DFT\.(?:[A-Z0-9_]+\.)?([A-Z0-9]+)\./i);
  if (match?.[1]) {
    return match[1].toLowerCase().replace(/_/g, " ");
  }
  const waterish = topic.match(/\b(WATER|METHANOL|BENZENE)\b/i);
  if (waterish?.[1]) {
    return waterish[1].toLowerCase();
  }
  return null;
}

function looksLikeSmiles(name: string): boolean {
  // crude: has digits in weird places or only lower organic chars with parens
  if (/[=\(\)\[\]#@]/.test(name)) return true;
  if (/^[a-z0-9\\\/]+$/i.test(name) && /[0-9]/.test(name) && name.length > 8)
    return true;
  return false;
}

function isWeakLabel(label: string): boolean {
  const l = label.trim().toLowerCase();
  return (
    !l ||
    l === "unknown" ||
    l === "rdkit" ||
    l.startsWith("rdkit") ||
    l.startsWith("moladt.rdkit") ||
    l.startsWith("[") ||
    looksLikeSmiles(l)
  );
}

function labelFromPath(sourcePath: string): string | null {
  // overnight-full-out/dft/propylene-glycol-dihexanoate/chem_dft_result...
  const parts = sourcePath.split(/[/\\]/).filter(Boolean);
  for (let i = parts.length - 1; i >= 0; i--) {
    const p = parts[i];
    if (p.startsWith("chem_dft_result") || p.endsWith(".json")) continue;
    if (p === "dft" || p === "demo" || p === "results") continue;
    if (p.length < 2) continue;
    return p.replace(/[_-]+/g, " ").toLowerCase();
  }
  return null;
}

function labelFromSources(
  resultPayload: Record<string, unknown>,
  requestPayload: Record<string, unknown> | null,
  topic: string | undefined,
  sourcePath: string,
): string {
  const molecule =
    (requestPayload?.molecule as Record<string, unknown> | undefined) ?? {};
  const name = molecule.molecule_name;
  if (
    typeof name === "string" &&
    name.length > 0 && name !== molecule.canonical_smiles &&
    !looksLikeSmiles(name) &&
    !isWeakLabel(name)
  ) {
    return name.toLowerCase().replace(/[_-]+/g, " ");
  }

  const moleculeId =
    (typeof resultPayload.molecule_id === "string" &&
      resultPayload.molecule_id) ||
    (typeof molecule.molecule_id === "string" && molecule.molecule_id) ||
    null;
  if (moleculeId) {
    if (/WATER/i.test(moleculeId)) return "water";
    if (/METHANOL/i.test(moleculeId)) return "methanol";
    if (/BENZENE/i.test(moleculeId)) return "benzene";
    if (/DNIAPMSPPWPWGF/i.test(moleculeId)) return "propylene glycol";
    if (/WWZKQHOCKIZLMA/i.test(moleculeId)) return "caprylic acid";
    if (/GHVNFZFCNZKVNT/i.test(moleculeId)) return "capric acid";
    // MOLADT.OVERNIGHT.label or MOLADT.CAGE.label
    const overnight = moleculeId.match(
      /MOLADT\.(?:OVERNIGHT|CAGE)\.([A-Za-z0-9_\-]+)/i,
    );
    if (overnight?.[1]) {
      return overnight[1].replace(/[_-]+/g, " ").toLowerCase();
    }
  }

  const fromTopic = slugFromTopic(topic);
  if (fromTopic && !isWeakLabel(fromTopic)) {
    return fromTopic;
  }

  const fromPath = labelFromPath(sourcePath);
  if (fromPath && !isWeakLabel(fromPath)) {
    return fromPath;
  }

  if (typeof name === "string" && name.length > 0 && name !== molecule.canonical_smiles && !isWeakLabel(name)) {
    return name.toLowerCase();
  }

  if (moleculeId && !/RDKIT/i.test(moleculeId)) {
    return moleculeId.toLowerCase();
  }

  return fromPath || "unknown";
}

function entryQuality(entry: DftCacheEntry): number {
  let score = 0;
  // Prefer curated gallery path
  if (entry.source_path.includes(`${path.sep}demo${path.sep}dft${path.sep}`))
    score += 100;
  if (entry.source_path.includes("overnight-science-out")) score += 40;
  if (entry.source_path.includes("overnight-full-out")) score += 30;
  if (!isWeakLabel(entry.label)) score += 20;
  if (entry.energy_hartree != null) score += 5;
  if (entry.gap_ev != null) score += 5;
  if (entry.functional?.toLowerCase().includes("b3lyp")) score += 2;
  return score;
}

function dedupeByLabel(entries: DftCacheEntry[]): DftCacheEntry[] {
  const best = new Map<string, DftCacheEntry>();
  for (const entry of entries) {
    if (isWeakLabel(entry.label)) {
      // keep weak labels only if unique artifact and no better name
      // skip pure rdkit dumps from storefront index
      continue;
    }
    const key = normalizeKey(entry.label);
    const prev = best.get(key);
    if (!prev || entryQuality(entry) > entryQuality(prev)) {
      best.set(key, entry);
    }
  }
  return [...best.values()].sort((a, b) => a.label.localeCompare(b.label));
}

function buildAliases(label: string): string[] {
  return [...new Set([
    label,
    normalizeKey(label),
    label.replace(/\s+/g, ""),
    label.replace(/\s+/g, "-"),
    ...(COMMON_ALIASES[label] ?? []),
  ])];
}

async function loadJson(filePath: string): Promise<ArtifactEnvelope | null> {
  try {
    const raw = await readFile(filePath, "utf8");
    return JSON.parse(raw) as ArtifactEnvelope;
  } catch {
    return null;
  }
}

async function listJsonFilesRecursive(root: string): Promise<string[]> {
  const out: string[] = [];
  async function walk(dir: string, depth: number): Promise<void> {
    if (depth > 6) return;
    let names: string[];
    try {
      names = await readdir(dir);
    } catch {
      return;
    }
    for (const name of names) {
      if (name.startsWith(".")) continue;
      const full = path.join(dir, name);
      try {
        const st = await stat(full);
        if (st.isDirectory()) {
          await walk(full, depth + 1);
        } else if (name.endsWith(".json")) {
          out.push(full);
        }
      } catch {
        // unreadable entry
      }
    }
  }
  await walk(root, 0);
  return out;
}

/**
 * Scan one or more directories (colon-separated) for signed chem.dft.result artifacts.
 */
export async function loadDftCache(cacheDir: string): Promise<DftCacheEntry[]> {
  const roots = cacheDir
    .split(/[:;]/)
    .map((s) => s.trim())
    .filter(Boolean);
  const allJson: string[] = [];
  for (const root of roots) {
    const files = await listJsonFilesRecursive(root);
    allJson.push(...files);
  }

  const resultFiles = allJson.filter((p) => {
    const base = path.basename(p);
    return base.startsWith("chem_dft_result.") && base.endsWith(".json");
  });
  const byId = new Map<string, string>();
  for (const full of allJson) {
    const match = path.basename(full).match(/art_[0-9a-f]+/i);
    if (match) {
      byId.set(match[0], full);
    }
  }

  const entries: DftCacheEntry[] = [];
  const seenArtifacts = new Set<string>();

  for (const sourcePath of resultFiles) {
    const artifact = await loadJson(sourcePath);
    if (!artifact?.id) continue;
    if (seenArtifacts.has(artifact.id)) continue;
    if (
      Array.isArray(artifact.schema_tags) &&
      !artifact.schema_tags.includes("chem.dft.result")
    ) {
      continue;
    }
    seenArtifacts.add(artifact.id);

    const resultPayload = decodeInlineJson(artifact);
    if (!resultPayload) continue;

    let requestPayload: Record<string, unknown> | null = null;
    for (const parentId of artifact.parent_artifact_ids ?? []) {
      const parentPath = byId.get(parentId);
      if (!parentPath) continue;
      const parent = await loadJson(parentPath);
      if (!parent) continue;
      const decoded = decodeInlineJson(parent);
      if (decoded?.molecule || decoded?.request_id) {
        requestPayload = decoded;
        break;
      }
    }

    const molecule =
      (requestPayload?.molecule as Record<string, unknown> | undefined) ?? {};
    const smiles =
      (typeof molecule.canonical_smiles === "string" &&
        molecule.canonical_smiles) ||
      null;
    const moleculeName =
      (typeof molecule.molecule_name === "string" && molecule.molecule_name) ||
      null;
    const moleculeId =
      (typeof resultPayload.molecule_id === "string" &&
        resultPayload.molecule_id) ||
      (typeof molecule.molecule_id === "string" && molecule.molecule_id) ||
      null;

    const label = labelFromSources(
      resultPayload,
      requestPayload,
      artifact.topic,
      sourcePath,
    );
    const orbitals =
      (resultPayload.orbitals as Record<string, unknown> | undefined) ?? {};
    const dipole =
      (resultPayload.dipole as Record<string, unknown> | undefined) ?? {};
    const convergence =
      (resultPayload.convergence as Record<string, unknown> | undefined) ?? {};
    // Do not sell unconverged SCFs from the cashier cache.
    if (convergence.converged === false) {
      continue;
    }

    const entry: DftCacheEntry = {
      artifact_id: artifact.id,
      label,
      aliases: buildAliases(label),
      smiles,
      molecule_id: moleculeId,
      molecule_name: moleculeName,
      functional:
        (typeof resultPayload.functional === "string" &&
          resultPayload.functional) ||
        null,
      basis_set:
        (typeof resultPayload.basis_set === "string" &&
          resultPayload.basis_set) ||
        null,
      energy_hartree:
        typeof resultPayload.energy_hartree === "number"
          ? resultPayload.energy_hartree
          : null,
      gap_ev:
        typeof orbitals.gap_ev === "number" ? orbitals.gap_ev : null,
      dipole_debye:
        typeof dipole.magnitude_debye === "number"
          ? dipole.magnitude_debye
          : null,
      topic: typeof artifact.topic === "string" ? artifact.topic : null,
      source_path: sourcePath,
      result_payload: resultPayload,
      request_payload: requestPayload,
      artifact: artifact as Record<string, unknown>,
    };
    entries.push(entry);
  }

  return dedupeByLabel(entries);
}

export function indexItems(entries: DftCacheEntry[]): DftCacheIndexItem[] {
  return entries.map((entry) => ({
    label: entry.label,
    aliases: entry.aliases.filter(
      (alias) => alias !== entry.artifact_id && alias !== entry.label,
    ),
    artifact_id: entry.artifact_id,
    smiles: entry.smiles,
    molecule_id: entry.molecule_id,
    functional: entry.functional,
    basis_set: entry.basis_set,
    energy_hartree: entry.energy_hartree,
    gap_ev: entry.gap_ev,
    dipole_debye: entry.dipole_debye,
  }));
}

/**
 * Resolve a query token (label, smiles, artifact id, alias) to one cache entry.
 */
export function findDftEntry(
  entries: DftCacheEntry[],
  query: string,
  kind: "auto" | "smiles" | "id" = "auto",
): DftCacheEntry | null {
  const raw = query.trim();
  if (!raw) return null;
  // Exact comparisons come first, and typed lookups never fall back to names.
  if (kind !== "id") {
    const exact = entries.find((entry) => entry.smiles === raw);
    if (exact) return exact;
  }
  if (kind !== "smiles") {
    const exact = entries.find((entry) => entry.artifact_id === raw || entry.molecule_id === raw);
    if (exact) return exact;
  }
  if (kind !== "auto") return null;
  const key = normalizeKey(raw);
  const compact = key.replace(/\s+/g, "");
  return entries.find((entry) => [entry.label, ...entry.aliases].some((alias) => {
    // Also protect callers with legacy entries containing identifier aliases.
    if (alias === entry.smiles || alias === entry.molecule_id || alias === entry.artifact_id) return false;
    const normalized = normalizeKey(alias);
    return normalized === key || normalized.replace(/\s+/g, "") === compact;
  })) ?? null;
}
