/**
 * Real Olympus / ChimiaClaw evidence for ticker + demos.
 * Sourced from demo/olympus-dft-inventory + signed gallery artifacts.
 * Prefer live GET /v1/dft/index when the gateway is up.
 */

export interface EvidenceMolecule {
  name: string;
  formula?: string;
  artifactId: string;
  energyHa?: number;
  method?: string;
  cubes?: boolean;
}

/** Cube-backed PBE/def2-tzvp gallery (host=Olympus.local). */
export const GALLERY_EVIDENCE: EvidenceMolecule[] = [
  {
    name: "Water",
    formula: "H₂O",
    artifactId: "art_3d5c1283b1a8f79f",
    energyHa: -76.376421,
    method: "PBE/def2-tzvp",
    cubes: true,
  },
  {
    name: "Methanol",
    formula: "CH₃OH",
    artifactId: "art_563825a02d8ea8a3",
    energyHa: -115.626291,
    method: "PBE/def2-tzvp",
    cubes: true,
  },
  {
    name: "Benzene",
    formula: "C₆H₆",
    artifactId: "art_87a648cd3b5f6490",
    energyHa: -232.018795,
    method: "PBE/def2-tzvp",
    cubes: true,
  },
  {
    name: "Propylene glycol",
    formula: "C₃H₈O₂",
    artifactId: "art_c1d9cf319fc537e2",
    energyHa: -269.351584,
    method: "PBE/def2-tzvp",
    cubes: true,
  },
  {
    name: "Caprylic acid (C8)",
    formula: "C₈H₁₆O₂",
    artifactId: "art_b4002fedd3e69f20",
    energyHa: -464.544963,
    method: "PBE/def2-tzvp",
    cubes: true,
  },
  {
    name: "Capric acid (C10)",
    formula: "C₁₀H₂₀O₂",
    artifactId: "art_5d1b8812735b2611",
    energyHa: -543.084109,
    method: "PBE/def2-tzvp",
    cubes: true,
  },
];

/** Overnight Ge scalar (PBE/def2-svp, no cubes). */
export const GE_SCALAR_EVIDENCE: EvidenceMolecule[] = [
  {
    name: "Germane",
    formula: "GeH₄",
    artifactId: "art_72799a3871d01929",
    energyHa: -2078.792573,
    method: "PBE/def2-svp",
  },
  {
    name: "Methylgermane",
    formula: "CH₃GeH₃",
    artifactId: "art_691b9179ea649f38",
    energyHa: -2118.028742,
    method: "PBE/def2-svp",
  },
  {
    name: "Cyclopropylgermane",
    artifactId: "art_69972a470fe7966c",
    energyHa: -2195.248871,
    method: "PBE/def2-svp",
  },
  {
    name: "Adamantylgermane",
    artifactId: "art_bb3490ecb173f082",
    energyHa: -2467.519022,
    method: "PBE/def2-svp",
  },
  {
    name: "Germatrane",
    artifactId: "art_d1a3d12e5978be50",
    energyHa: -2810.512862,
    method: "PBE/def2-svp",
  },
];

/** Sn single-points from Ge→Sn batch (Olympus, raw worker JSON). */
export const SN_BATCH_EVIDENCE: EvidenceMolecule[] = [
  { name: "NC₃Sn–H", artifactId: "NC3Sn_H", energyHa: -2066.416353, method: "PBE/def2-svp · Ge→Sn start" },
  { name: "NC₃Sn–Cl", artifactId: "NC3Sn_Cl", energyHa: -2525.525087, method: "PBE/def2-svp · Ge→Sn start" },
  { name: "C3 NC₃Sn–H", artifactId: "C3_NC3Sn_H", energyHa: -2065.504249, method: "PBE/def2-svp · Ge→Sn start" },
  { name: "C3 NC₃Sn–Cl", artifactId: "C3_NC3Sn_Cl", energyHa: -2524.628703, method: "PBE/def2-svp · Ge→Sn start" },
  { name: "Ad–SnH₃", artifactId: "Ad_SnH3", energyHa: -2047.533908, method: "PBE/def2-svp · Ge→Sn start" },
  { name: "Ad–SnCl₃", artifactId: "Ad_SnCl3", energyHa: -3425.198750, method: "PBE/def2-svp · Ge→Sn start" },
  { name: "Ad–SnMe₃", artifactId: "Ad_SnMe3", energyHa: -2169.969544, method: "PBE/def2-svp · Ge→Sn start" },
];

export function formatEnergy(e?: number): string | undefined {
  if (e === undefined || Number.isNaN(e)) return undefined;
  return `${e.toFixed(2)} Ha`;
}

export function defaultTickerMolecules(): EvidenceMolecule[] {
  return [...GALLERY_EVIDENCE, ...GE_SCALAR_EVIDENCE, ...SN_BATCH_EVIDENCE];
}
