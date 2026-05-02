/**
 * Load + parse the six signed `chem.dft.result` artifacts from
 * `public/artifacts/` and surface them as typed records the dashboard
 * can render.
 *
 * Every artifact JSON in `public/artifacts/` is the canonical signed
 * `Artifact` shape produced by `chimiaclaw-cli` on the Rust side. The
 * payload bytes are inline-hex-encoded under
 * `payload.location.Inline.bytes_hex`; we hex-decode and JSON-parse
 * them here at module load time so pages can consume typed objects.
 *
 * No fetching, no network, no API. The Vercel build embeds these
 * JSONs into the bundle and renders them server-side.
 */
import "server-only"

import { readFileSync, readdirSync } from "node:fs"
import { join } from "node:path"

const ARTIFACTS_DIR = join(process.cwd(), "public", "artifacts")
const ORBITALS_DIR = join(process.cwd(), "public", "orbitals")

// ---------------- Canonical `chem.*` artifact shape ---------------- //

export interface SignedArtifact {
  id: string
  content_hash: string
  skill: string
  agent: string
  topic: string
  input_fingerprint: string
  output_cid: string | null
  parent_artifact_ids: string[]
  schema_tags: string[]
  payload: {
    hash: string
    encoding: "Json" | "Cbor" | "Bytes"
    location: { Inline: { bytes_hex: string } } | { External: { cid: string } }
  } | null
  created_at_unix: number
  signing_public_key: string
  signature: string
}

// ---------------- chem.dft.result payload ---------------- //

export interface DftResultPayload {
  schema_tag: "chem.dft.result"
  request_id: string
  molecule_id: string
  functional: string
  basis_set: string
  backend: string
  total_charge: number
  multiplicity: number
  energy_hartree: number
  orbitals: {
    homo_hartree: number
    lumo_hartree: number
    gap_hartree: number
    gap_ev: number
  } | null
  dipole: {
    x_debye: number
    y_debye: number
    z_debye: number
    magnitude_debye: number
  } | null
  convergence: {
    converged: boolean
    n_cycles: number
    final_gradient_norm: number | null
    scf_threshold: number | null
  }
  timings: {
    wall_seconds: number
    cpu_seconds: number | null
  }
  requested_properties: string[]
  provenance: {
    source_kind: string
    source_ref: string
    host: string | null
    pyscf_version: string | null
    skala_version: string | null
    dispersion: string | null
    notes: string[]
  }
  orbital_densities?: Array<{
    label: "HOMO" | "LUMO" | "TOTAL_DENSITY" | string
    sha256: string
    bytes: number
    grid_resolution: number
    local_path: string
  }>
}

// ---------------- chem.molecule.adt payload (subset) ---------------- //

export interface MoleculeAdtPayload {
  molecule_id: string
  name: string
  atoms: Record<
    string,
    {
      atom_id: number
      attributes: { symbol: string; atomic_number: number; atomic_weight: number }
      coordinate: { x_angstrom: number; y_angstrom: number; z_angstrom: number }
      formal_charge: number
    }
  >
  provenance: {
    source_kind: string
    source_ref: string
    notes: string[]
  }
  projections?: {
    canonical_smiles?: string | null
    inchi?: string | null
    inchikey?: string | null
  }
}

// ---------------- chem.dft.request payload (subset) ---------------- //

export interface DftRequestPayload {
  request_id: string
  molecule: {
    molecule_id: string
    molecule_name: string
    molecular_formula: string
    molecule_artifact_id: string | null
    molecule_payload_hash: string | null
    canonical_smiles: string | null
  }
  total_charge: number
  multiplicity: number
  method: {
    functional: string
    basis_set: string
    backend: string
    dispersion: string | null
    grid_level: number | null
  }
  job_kind: string
  requested_properties: string[]
  worker_hint: string | null
}

// ---------------- Cube PNG metadata ---------------- //

export interface OrbitalCubePng {
  label: "HOMO" | "LUMO" | "TOTAL_DENSITY"
  pngPublicPath: string // path under /public, e.g. "/orbitals/MOLADT.WATER.001_HOMO_....png"
  cubeSha256Prefix: string
}

// ---------------- The fully-resolved record a page renders ---------------- //

export interface DftRecord {
  resultArtifactId: string
  requestArtifactId: string
  moleculeArtifactId: string
  result: DftResultPayload
  request: DftRequestPayload
  molecule: MoleculeAdtPayload
  cubes: OrbitalCubePng[]
  rawSignedArtifact: SignedArtifact
}

// ---------------- Loader ---------------- //

function decodeInlinePayload<T>(artifact: SignedArtifact): T {
  if (!artifact.payload) {
    throw new Error(`artifact ${artifact.id} has no payload`)
  }
  const loc = artifact.payload.location
  if (!("Inline" in loc)) {
    throw new Error(`artifact ${artifact.id} payload is not inline`)
  }
  const hex = loc.Inline.bytes_hex
  const bytes = Buffer.from(hex, "hex")
  return JSON.parse(bytes.toString("utf-8")) as T
}

function readArtifact(filename: string): SignedArtifact {
  const raw = readFileSync(join(ARTIFACTS_DIR, filename), "utf-8")
  return JSON.parse(raw) as SignedArtifact
}

/** Friendly molecule display name, derived from molecule_id. */
function displayName(moleculeId: string): string {
  if (moleculeId.startsWith("MOLADT.RDKIT.")) {
    // RDKit-resolved: InChIKey-derived id. Hand-map the three known.
    const inchi = moleculeId.replace("MOLADT.RDKIT.", "").split("_")[0]
    if (inchi.startsWith("DNIAPMSPPWPWGF")) return "propylene glycol"
    if (inchi.startsWith("WWZKQHOCKIZLMA")) return "caprylic acid (C8)"
    if (inchi.startsWith("GHVNFZFCNZKVNT")) return "capric acid (C10)"
    return inchi
  }
  // Curated library: MOLADT.<NAME>.001
  const name = moleculeId.split(".")[1]?.toLowerCase() ?? moleculeId
  return name
}

/** Find the three cube PNGs for a given molecule_id. */
function cubesFor(moleculeId: string): OrbitalCubePng[] {
  const allPngs = readdirSync(ORBITALS_DIR).filter((f) => f.endsWith(".png"))
  // Filenames look like:
  //   MOLADT.WATER.001_HOMO_<sha-prefix>.png
  //   MOLADT.WATER.001_TOTAL_DENSITY_<sha-prefix>.png
  const prefix = `${moleculeId}_`
  const matches = allPngs.filter((f) => f.startsWith(prefix))
  const labelOrder: Record<OrbitalCubePng["label"], number> = {
    HOMO: 0,
    LUMO: 1,
    TOTAL_DENSITY: 2,
  }
  return matches
    .map((filename) => {
      const stem = filename.replace(/\.png$/, "")
      const tail = stem.slice(prefix.length) // e.g. "HOMO_40b..." or "TOTAL_DENSITY_99c..."
      let label: OrbitalCubePng["label"]
      let shaPrefix: string
      if (tail.startsWith("TOTAL_DENSITY_")) {
        label = "TOTAL_DENSITY"
        shaPrefix = tail.slice("TOTAL_DENSITY_".length)
      } else if (tail.startsWith("HOMO_")) {
        label = "HOMO"
        shaPrefix = tail.slice("HOMO_".length)
      } else if (tail.startsWith("LUMO_")) {
        label = "LUMO"
        shaPrefix = tail.slice("LUMO_".length)
      } else {
        // Unknown layout — skip.
        return null
      }
      return {
        label,
        pngPublicPath: `/orbitals/${filename}`,
        cubeSha256Prefix: shaPrefix,
      } as OrbitalCubePng
    })
    .filter((c): c is OrbitalCubePng => c !== null)
    .sort((a, b) => labelOrder[a.label] - labelOrder[b.label])
}

/** Load every chem.dft.result artifact + matching parents. */
export function loadAllDftRecords(): DftRecord[] {
  const files = readdirSync(ARTIFACTS_DIR)
  const resultFiles = files.filter((f) => f.startsWith("chem_dft_result."))
  const records: DftRecord[] = []
  for (const resultFile of resultFiles) {
    const resultArtifact = readArtifact(resultFile)
    const result = decodeInlinePayload<DftResultPayload>(resultArtifact)

    const requestArtifactId = resultArtifact.parent_artifact_ids[0]
    const requestFile = `chem_dft_request.${requestArtifactId}.json`
    const requestArtifact = readArtifact(requestFile)
    const request = decodeInlinePayload<DftRequestPayload>(requestArtifact)

    const moleculeArtifactId =
      request.molecule.molecule_artifact_id ??
      requestArtifact.parent_artifact_ids[0]
    const moleculeFile = `chem_molecule_adt.${moleculeArtifactId}.json`
    const moleculeArtifact = readArtifact(moleculeFile)
    const molecule = decodeInlinePayload<MoleculeAdtPayload>(moleculeArtifact)

    const cubes = cubesFor(result.molecule_id)

    records.push({
      resultArtifactId: resultArtifact.id,
      requestArtifactId: requestArtifact.id,
      moleculeArtifactId: moleculeArtifact.id,
      result,
      request,
      molecule,
      cubes,
      rawSignedArtifact: resultArtifact,
    })
  }
  // Sort by energy ascending (lowest = "smaller molecule" first feels nicer
  // for the gallery; not a chemistry claim).
  records.sort((a, b) => a.result.energy_hartree - b.result.energy_hartree)
  return records
}

/** Get one DFT record by its result artifact id. */
export function getDftRecord(resultArtifactId: string): DftRecord | undefined {
  return loadAllDftRecords().find(
    (r) => r.resultArtifactId === resultArtifactId,
  )
}

/** Friendly molecule display name for a record. */
export function recordDisplayName(record: DftRecord): string {
  return displayName(record.result.molecule_id)
}
