/**
 * Load the AiZynthFinder route-search summary and the B3LYP precursor
 * follow-up summary from `public/retrosynthesis/`.
 *
 * These are static demo artifacts copied from `demo/retrosynthesis/` and
 * `demo/dft/b3lyp-precursors-1-5/`; no network requests are made.
 */
import "server-only"

import { readFileSync } from "node:fs"
import { join } from "node:path"

const RETROSYNTHESIS_DIR = join(process.cwd(), "public", "retrosynthesis")

export interface AiZynthRoute {
  reaction_smiles: string
  template_hash: string
  classification: string
  policy_probability: number
  policy_probability_rank: number
  policy_name: string
  precursor_smiles: string[]
  scores: Record<string, number>
  route_metadata: {
    created_at_iteration: number
    is_solved: boolean
  }
}

export interface AiZynthTarget {
  target_label: string
  target_smiles: string
  is_solved: boolean
  search_time_seconds: number
  first_solution_time_seconds: number
  first_solution_iteration: number
  number_of_nodes: number
  number_of_routes: number
  number_of_solved_routes: number
  top_score: number
  number_of_steps: number
  number_of_precursors: number
  number_of_precursors_in_stock: number
  precursors_in_stock: string
  precursors_not_in_stock: string
  precursors_availability: string
  top_route: AiZynthRoute
}

export interface AiZynthSummary {
  schema_tag: "chem.retrosynth.aizynthfinder.route_search.summary"
  run_id: string
  execution_node: string
  tool: {
    name: string
    version: string
    policy: string
    filter: string
    stock: string
    config_path_remote: string
  }
  source_files: {
    routes_json_gz: string
    targets_smi: string
  }
  targets: AiZynthTarget[]
  b3lyp_dft_candidate_precursors: Array<{
    smiles: string
    from_targets: string[]
  }>
  hackathon_chain: string[]
}

export interface B3lypCompletedResult {
  label: string
  artifact_id: string
  request_artifact_id: string
  path: string
  molecule_id: string
  functional: string
  basis_set: string
  energy_hartree: number
  gap_ev: number
  dipole_debye: number
  wall_seconds: number
  converged: boolean
  source_kind: string
}

export interface B3lypSummary {
  schema_tag: "chem.dft.b3lyp_precursor_batch.summary"
  run_id: string
  execution_node: string
  method: {
    functional: string
    basis_set: string
    backend: string
  }
  completed_results: B3lypCompletedResult[]
  blocked_candidates: Array<{
    smiles: string
    reason: string
  }>
  notes: string[]
}

export interface RetrosynthesisDashboardData {
  routeSummary: AiZynthSummary
  dftSummary: B3lypSummary
  stats: {
    targets: number
    solvedTargets: number
    unsolvedTargets: number
    completedDft: number
    blockedDft: number
  }
}

function readJson<T>(filename: string): T {
  const raw = readFileSync(join(RETROSYNTHESIS_DIR, filename), "utf-8")
  return JSON.parse(raw) as T
}

export function loadRetrosynthesisDashboard(): RetrosynthesisDashboardData {
  const routeSummary = readJson<AiZynthSummary>("aizynth-targets-1-5.summary.json")
  const dftSummary = readJson<B3lypSummary>("b3lyp-precursors-1-5.summary.json")
  const solvedTargets = routeSummary.targets.filter((target) => target.is_solved).length

  return {
    routeSummary,
    dftSummary,
    stats: {
      targets: routeSummary.targets.length,
      solvedTargets,
      unsolvedTargets: routeSummary.targets.length - solvedTargets,
      completedDft: dftSummary.completed_results.length,
      blockedDft: dftSummary.blocked_candidates.length,
    },
  }
}
