import type { DFTResult, Challenge, ChallengeStatus } from '@/lib/types'

/** Sample artifacts for demo purposes - will be replaced with real data */
export const SAMPLE_ARTIFACTS: DFTResult[] = [
  {
    id: 'dft-001',
    requestId: 'req-001',
    energy: -347.892341,
    dipole: [0.234, -0.567, 0.891],
    homo: -6.23,
    lumo: -1.45,
    computeTimeMs: 45230,
    operatorAddress: '0x7a23...8f91',
    signature: '0x1234...abcd',
    anchorTxHash: '0xabc123...def456',
  },
  {
    id: 'dft-002',
    requestId: 'req-002',
    energy: -521.445678,
    dipole: [0.112, 0.334, -0.221],
    homo: -5.89,
    lumo: -1.12,
    computeTimeMs: 67890,
    operatorAddress: '0x8b34...9g02',
    signature: '0x5678...efgh',
    anchorTxHash: '0xdef789...ghi012',
  },
  {
    id: 'dft-003',
    requestId: 'req-003',
    energy: -189.223456,
    dipole: [-0.445, 0.667, 0.223],
    homo: -7.12,
    lumo: -2.34,
    computeTimeMs: 23450,
    operatorAddress: '0x9c45...0h13',
    signature: '0x9012...ijkl',
  },
]

/** Challenge bond amount in USDC */
export const CHALLENGE_BOND_AMOUNT = 50

/** Discrepancy threshold in Hartrees */
export const DISCREPANCY_THRESHOLD = 0.001

/** Sample active challenges for demo */
export const SAMPLE_CHALLENGES: Challenge[] = [
  {
    id: 'chal-001',
    originalArtifactId: 'dft-001',
    originalResult: SAMPLE_ARTIFACTS[0],
    challengerAddress: '0xdef...456',
    bondAmount: 50,
    status: 'computing',
    discrepancyThreshold: DISCREPANCY_THRESHOLD,
    createdAt: new Date(Date.now() - 3600000).toISOString(),
  },
]

export function getChallengeStatusLabel(status: ChallengeStatus): string {
  const labels: Record<ChallengeStatus, string> = {
    pending: 'Pending Bond',
    computing: 'Re-computing',
    verified: 'Verified',
    disputed: 'Disputed',
    resolved: 'Resolved',
  }
  return labels[status]
}

export function getChallengeStatusColor(status: ChallengeStatus): string {
  const colors: Record<ChallengeStatus, string> = {
    pending: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
    computing: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
    verified: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30',
    disputed: 'bg-red-500/20 text-red-400 border-red-500/30',
    resolved: 'bg-muted text-muted-foreground border-muted',
  }
  return colors[status]
}
