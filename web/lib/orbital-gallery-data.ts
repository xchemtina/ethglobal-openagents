/**
 * Real Olympus cube-derived orbital PNGs (PBE/def2-tzvp gallery).
 * Assets under public/orbitals/ — signed chem.dft.result artifacts are source of truth.
 */

export type OrbitalKind = "homo" | "lumo" | "density";

export interface GalleryMolecule {
  id: string;
  label: string;
  formula: string;
  artifactId: string;
  energyHa: number;
  gapEv: number;
  method: string;
  images: Record<OrbitalKind, string>;
}

export const ORBITAL_GALLERY: GalleryMolecule[] = [
  {
    id: "water",
    label: "Water",
    formula: "H₂O",
    artifactId: "art_3d5c1283b1a8f79f",
    energyHa: -76.376421,
    gapEv: 6.964,
    method: "PBE/def2-tzvp · Olympus",
    images: {
      homo: "/orbitals/water_homo.png",
      lumo: "/orbitals/water_lumo.png",
      density: "/orbitals/water_density.png",
    },
  },
  {
    id: "methanol",
    label: "Methanol",
    formula: "CH₃OH",
    artifactId: "art_563825a02d8ea8a3",
    energyHa: -115.626291,
    gapEv: 6.025,
    method: "PBE/def2-tzvp · Olympus",
    images: {
      homo: "/orbitals/methanol_homo.png",
      lumo: "/orbitals/methanol_lumo.png",
      density: "/orbitals/methanol_density.png",
    },
  },
  {
    id: "benzene",
    label: "Benzene",
    formula: "C₆H₆",
    artifactId: "art_87a648cd3b5f6490",
    energyHa: -232.018795,
    gapEv: 5.129,
    method: "PBE/def2-tzvp · Olympus",
    images: {
      homo: "/orbitals/benzene_homo.png",
      lumo: "/orbitals/benzene_lumo.png",
      density: "/orbitals/benzene_density.png",
    },
  },
  {
    id: "propylene-glycol",
    label: "Propylene glycol",
    formula: "C₃H₈O₂",
    artifactId: "art_c1d9cf319fc537e2",
    energyHa: -269.351584,
    gapEv: 6.226,
    method: "PBE/def2-tzvp · Olympus",
    images: {
      homo: "/orbitals/propylene_glycol_homo.png",
      lumo: "/orbitals/propylene_glycol_lumo.png",
      density: "/orbitals/propylene_glycol_density.png",
    },
  },
  {
    id: "caprylic-acid",
    label: "Caprylic acid (C8)",
    formula: "C₈H₁₆O₂",
    artifactId: "art_b4002fedd3e69f20",
    energyHa: -464.544963,
    gapEv: 5.257,
    method: "PBE/def2-tzvp · Olympus",
    images: {
      homo: "/orbitals/caprylic_acid_homo.png",
      lumo: "/orbitals/caprylic_acid_lumo.png",
      density: "/orbitals/caprylic_acid_density.png",
    },
  },
  {
    id: "capric-acid",
    label: "Capric acid (C10)",
    formula: "C₁₀H₂₀O₂",
    artifactId: "art_5d1b8812735b2611",
    energyHa: -543.084109,
    gapEv: 5.389,
    method: "PBE/def2-tzvp · Olympus",
    images: {
      homo: "/orbitals/capric_acid_homo.png",
      lumo: "/orbitals/capric_acid_lumo.png",
      density: "/orbitals/capric_acid_density.png",
    },
  },
];
