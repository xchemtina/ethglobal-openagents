import { readdir, readFile } from "node:fs/promises";
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
  water: ["h2o", "o", "moladt.water.001"],
  methanol: ["ch3oh", "meoh", "co", "moladt.methanol.001"],
  benzene: ["c6h6", "c1ccccc1", "moladt.benzene.001"],
  "propylene glycol": [
    "propyleneglycol",
    "propane-1,2-diol",
    "1,2-propanediol",
    "occ(o)c",
    "pg",
  ],
  "caprylic acid": ["c8", "octanoic acid", "cccccccc(=o)o", "octanoic"],
  "capric acid": ["c10", "decanoic acid", "cccccccccc(=o)o", "decanoic"],
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

function labelFromSources(
  resultPayload: Record<string, unknown>,
  requestPayload: Record<string, unknown> | null,
  topic: string | undefined,
): string {
  const molecule =
    (requestPayload?.molecule as Record<string, unknown> | undefined) ?? {};
  const name = molecule.molecule_name;
  if (typeof name === "string" && name.length > 0 && !/[=\(\)]/.test(name)) {
    // Prefer human names; reject SMILES-looking "names".
    return name.toLowerCase();
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
  }

  const fromTopic = slugFromTopic(topic);
  if (fromTopic) {
    return fromTopic;
  }

  if (typeof name === "string" && name.length > 0) {
    return name.toLowerCase();
  }

  return (
    (typeof resultPayload.molecule_id === "string" &&
      resultPayload.molecule_id.toLowerCase()) ||
    "unknown"
  );
}

function buildAliases(
  label: string,
  smiles: string | null,
  moleculeId: string | null,
  artifactId: string,
): string[] {
  const aliases = new Set<string>();
  aliases.add(label);
  aliases.add(normalizeKey(label));
  aliases.add(label.replace(/\s+/g, ""));
  aliases.add(label.replace(/\s+/g, "-"));
  aliases.add(artifactId);
  if (smiles) {
    aliases.add(smiles);
    aliases.add(smiles.toLowerCase());
  }
  if (moleculeId) {
    aliases.add(moleculeId);
    aliases.add(moleculeId.toLowerCase());
  }
  for (const extra of COMMON_ALIASES[label] ?? []) {
    aliases.add(extra);
  }
  return [...aliases];
}

async function loadJson(filePath: string): Promise<ArtifactEnvelope | null> {
  try {
    const raw = await readFile(filePath, "utf8");
    return JSON.parse(raw) as ArtifactEnvelope;
  } catch {
    return null;
  }
}

/**
 * Scan a directory for signed chem.dft.result artifacts and index by label/smiles/id.
 */
export async function loadDftCache(cacheDir: string): Promise<DftCacheEntry[]> {
  let names: string[];
  try {
    names = await readdir(cacheDir);
  } catch {
    return [];
  }

  const resultFiles = names.filter(
    (name) =>
      name.startsWith("chem_dft_result.") && name.endsWith(".json"),
  );
  const byId = new Map<string, string>();
  for (const name of names) {
    if (!name.endsWith(".json")) continue;
    const match = name.match(/art_[0-9a-f]+/i);
    if (match) {
      byId.set(match[0], path.join(cacheDir, name));
    }
  }

  const entries: DftCacheEntry[] = [];

  for (const name of resultFiles) {
    const sourcePath = path.join(cacheDir, name);
    const artifact = await loadJson(sourcePath);
    if (!artifact?.id) continue;
    if (
      Array.isArray(artifact.schema_tags) &&
      !artifact.schema_tags.includes("chem.dft.result")
    ) {
      continue;
    }

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

    const label = labelFromSources(resultPayload, requestPayload, artifact.topic);
    const orbitals =
      (resultPayload.orbitals as Record<string, unknown> | undefined) ?? {};
    const dipole =
      (resultPayload.dipole as Record<string, unknown> | undefined) ?? {};

    const entry: DftCacheEntry = {
      artifact_id: artifact.id,
      label,
      aliases: buildAliases(label, smiles, moleculeId, artifact.id),
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

  entries.sort((a, b) => a.label.localeCompare(b.label));
  return entries;
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
): DftCacheEntry | null {
  const raw = query.trim();
  if (!raw) return null;
  const key = normalizeKey(raw);
  const compact = key.replace(/\s+/g, "");

  for (const entry of entries) {
    if (entry.artifact_id === raw || entry.artifact_id === key) {
      return entry;
    }
    if (normalizeKey(entry.label) === key) {
      return entry;
    }
    for (const alias of entry.aliases) {
      const a = normalizeKey(alias);
      if (a === key || a.replace(/\s+/g, "") === compact) {
        return entry;
      }
      if (alias === raw) {
        return entry;
      }
    }
    if (entry.smiles && entry.smiles === raw) {
      return entry;
    }
  }
  return null;
}
