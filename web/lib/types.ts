/** Core artifact types matching the Rust backend schema */

export interface Artifact {
  id: string
  artifactType: ArtifactType
  hash: string
  timestamp: string
  parentIds: string[]
  metadata: Record<string, unknown>
}

export type ArtifactType =
  | 'smiles'
  | 'mol_adt'
  | 'dft_request'
  | 'dft_result'
  | 'uniswap_quote'
  | '0g_anchor'

export interface DFTRequest {
  moleculeId: string
  method: 'B3LYP' | 'PBE0' | 'M06-2X' | 'wB97X-D'
  basisSet: 'def2-SVP' | 'def2-TZVP' | 'cc-pVDZ' | 'cc-pVTZ'
  charge: number
  multiplicity: number
  solvent?: string
}

export interface DFTResult {
  id: string
  requestId: string
  energy: number // Hartrees
  dipole: [number, number, number]
  homo: number // eV
  lumo: number // eV
  computeTimeMs: number
  operatorAddress: string
  signature: string
  anchorTxHash?: string
}

export interface SettlementQuote {
  id: string
  artifactIds: string[]
  priceUsdc: number
  validUntil: string
  escrowAddress: string
  status: 'pending' | 'accepted' | 'settled' | 'expired'
}

export interface AgentIdentity {
  ensName: string
  address: string
  publicKey: string
  capabilities: string[]
  registeredAt: string
}

/** Challenge system types */

export type ChallengeStatus = 'pending' | 'computing' | 'verified' | 'disputed' | 'resolved'

export interface Challenge {
  id: string
  originalArtifactId: string
  originalResult: DFTResult
  challengerAddress: string
  bondAmount: number // USDC
  status: ChallengeStatus
  challengeResult?: DFTResult
  discrepancyThreshold: number // Hartrees
  createdAt: string
  resolvedAt?: string
  winner?: 'original' | 'challenger'
}

/** Pipeline step for visualization */

export interface PipelineStep {
  id: string
  label: string
  description: string
  icon: 'molecule' | 'cpu' | 'lock' | 'coins' | 'anchor' | 'check'
  status?: 'pending' | 'active' | 'complete'
}

/** Molecule data for ticker and families */

export interface Molecule {
  name: string
  smiles: string
  artifactId: string
  family?: string
}

export interface MoleculeFamily {
  name: string
  description: string
  count: number
  molecules: Molecule[]
}
