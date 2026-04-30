//! Portable Molecular ADT payloads for ChimiaClaw.
//!
//! This crate mirrors the cross-language subset of the adjacent Haskell
//! `MolADT-Bayes` molecule representation without vendoring that project.  The
//! canonical artifact format is JSON so Rust, Haskell, Python, and remote DFT
//! workers can agree on the same signed payload bytes.

use chimiaclaw_artifact::{
    blake3_hex, canonical_bytes, Artifact, ArtifactDraft, ArtifactError, ArtifactId,
    ArtifactSigner, PayloadRef,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

pub mod geometry;
pub mod library;
pub mod render;
pub mod worker;

pub const MOLECULE_ADT_TAG: &str = "chem.molecule.adt";
pub const MOLECULE_ADT_SKILL: &str = "chem.molecule.adt.v1";
pub const DFT_REQUEST_TAG: &str = "chem.dft.request";
pub const DFT_REQUEST_SKILL: &str = "chem.dft.request.v1";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum AtomicSymbol {
    H,
    B,
    C,
    N,
    O,
    F,
    Na,
    P,
    S,
    Cl,
    Fe,
    Br,
    I,
}

impl AtomicSymbol {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::H => "H",
            Self::B => "B",
            Self::C => "C",
            Self::N => "N",
            Self::O => "O",
            Self::F => "F",
            Self::Na => "Na",
            Self::P => "P",
            Self::S => "S",
            Self::Cl => "Cl",
            Self::Fe => "Fe",
            Self::Br => "Br",
            Self::I => "I",
        }
    }

    #[must_use]
    pub const fn atomic_number(&self) -> u8 {
        match self {
            Self::H => 1,
            Self::B => 5,
            Self::C => 6,
            Self::N => 7,
            Self::O => 8,
            Self::F => 9,
            Self::Na => 11,
            Self::P => 15,
            Self::S => 16,
            Self::Cl => 17,
            Self::Fe => 26,
            Self::Br => 35,
            Self::I => 53,
        }
    }

    #[must_use]
    pub const fn default_atomic_weight(&self) -> f64 {
        match self {
            Self::H => 1.008,
            Self::B => 10.81,
            Self::C => 12.011,
            Self::N => 14.007,
            Self::O => 15.999,
            Self::F => 18.998,
            Self::Na => 22.990,
            Self::P => 30.974,
            Self::S => 32.06,
            Self::Cl => 35.45,
            Self::Fe => 55.845,
            Self::Br => 79.904,
            Self::I => 126.904,
        }
    }
}

impl Display for AtomicSymbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElementAttributes {
    pub symbol: AtomicSymbol,
    pub atomic_number: u8,
    pub atomic_weight: f64,
}

impl From<AtomicSymbol> for ElementAttributes {
    fn from(symbol: AtomicSymbol) -> Self {
        Self {
            atomic_number: symbol.atomic_number(),
            atomic_weight: symbol.default_atomic_weight(),
            symbol,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub x_angstrom: f64,
    pub y_angstrom: f64,
    pub z_angstrom: f64,
}

impl Coordinate {
    #[must_use]
    pub const fn new(x_angstrom: f64, y_angstrom: f64, z_angstrom: f64) -> Self {
        Self {
            x_angstrom,
            y_angstrom,
            z_angstrom,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Atom {
    pub atom_id: u32,
    pub attributes: ElementAttributes,
    pub coordinate: Coordinate,
    pub formal_charge: i32,
    pub shells: Vec<ElectronShell>,
}

impl Atom {
    #[must_use]
    pub fn new(atom_id: u32, symbol: AtomicSymbol, coordinate: Coordinate) -> Self {
        Self {
            atom_id,
            attributes: symbol.into(),
            coordinate,
            formal_charge: 0,
            shells: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElectronShell {
    pub principal_quantum_number: u8,
    pub subshells: Vec<SubShell>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubShell {
    pub label: String,
    pub orbitals: Vec<OrbitalAnnotation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitalAnnotation {
    pub label: String,
    pub electron_count: u8,
    pub orientation: Option<Coordinate>,
    pub hybrid_components: Vec<HybridComponent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HybridComponent {
    pub coefficient: f64,
    pub orbital: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Edge {
    pub a: u32,
    pub b: u32,
}

impl Edge {
    #[must_use]
    pub const fn new(i: u32, j: u32) -> Self {
        if i <= j {
            Self { a: i, b: j }
        } else {
            Self { a: j, b: i }
        }
    }

    #[must_use]
    pub const fn contains(&self, atom_id: u32) -> bool {
        self.a == atom_id || self.b == atom_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BondingSystem {
    pub system_id: u32,
    pub shared_electrons: u32,
    pub member_edges: BTreeSet<Edge>,
    pub tag: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoleculeProvenance {
    pub source_kind: String,
    pub source_ref: String,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoleculeProjections {
    pub canonical_smiles: Option<String>,
    pub inchi: Option<String>,
    pub inchikey: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoleculeAdt {
    pub molecule_id: String,
    pub name: String,
    pub atoms: BTreeMap<u32, Atom>,
    pub local_bonds: BTreeSet<Edge>,
    pub systems: Vec<BondingSystem>,
    pub provenance: MoleculeProvenance,
    pub projections: MoleculeProjections,
}

impl MoleculeAdt {
    pub fn validate(&self) -> Result<(), MolAdtError> {
        if self.atoms.is_empty() {
            return Err(MolAdtError::EmptyMolecule);
        }
        for (atom_id, atom) in &self.atoms {
            if atom.atom_id != *atom_id {
                return Err(MolAdtError::AtomIdMismatch {
                    map_key: *atom_id,
                    atom_id: atom.atom_id,
                });
            }
        }
        for edge in &self.local_bonds {
            self.validate_edge(edge)?;
        }
        for system in &self.systems {
            if system.shared_electrons == 0 {
                return Err(MolAdtError::InvalidBondingSystem {
                    system_id: system.system_id,
                    reason: "shared_electrons must be positive".to_string(),
                });
            }
            if system.member_edges.is_empty() {
                return Err(MolAdtError::InvalidBondingSystem {
                    system_id: system.system_id,
                    reason: "member_edges must not be empty".to_string(),
                });
            }
            for edge in &system.member_edges {
                self.validate_edge(edge)?;
            }
        }
        Ok(())
    }

    fn validate_edge(&self, edge: &Edge) -> Result<(), MolAdtError> {
        if edge.a == edge.b {
            return Err(MolAdtError::SelfBond(edge.a));
        }
        if !self.atoms.contains_key(&edge.a) || !self.atoms.contains_key(&edge.b) {
            return Err(MolAdtError::UnknownAtomInEdge {
                a: edge.a,
                b: edge.b,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn total_formal_charge(&self) -> i32 {
        self.atoms.values().map(|atom| atom.formal_charge).sum()
    }

    #[must_use]
    pub fn formula(&self) -> String {
        let mut counts: BTreeMap<AtomicSymbol, u32> = BTreeMap::new();
        for atom in self.atoms.values() {
            *counts.entry(atom.attributes.symbol.clone()).or_default() += 1;
        }
        let mut parts = Vec::new();
        for preferred in [AtomicSymbol::C, AtomicSymbol::H] {
            if let Some(count) = counts.remove(&preferred) {
                parts.push(format_formula_part(&preferred, count));
            }
        }
        parts.extend(
            counts
                .into_iter()
                .map(|(symbol, count)| format_formula_part(&symbol, count)),
        );
        parts.join("")
    }

    pub fn payload_hash(&self) -> Result<String, MolAdtError> {
        canonical_bytes(self)
            .map(|bytes| blake3_hex(&bytes))
            .map_err(MolAdtError::Artifact)
    }

    pub fn to_xyz(&self) -> Result<String, MolAdtError> {
        self.validate()?;
        let mut lines = vec![
            self.atoms.len().to_string(),
            format!(
                "{} | molecule_id={} | formula={}",
                self.name,
                self.molecule_id,
                self.formula()
            ),
        ];
        for atom in self.atoms.values() {
            lines.push(format!(
                "{} {:.8} {:.8} {:.8}",
                atom.attributes.symbol,
                atom.coordinate.x_angstrom,
                atom.coordinate.y_angstrom,
                atom.coordinate.z_angstrom
            ));
        }
        Ok(lines.join("\n"))
    }

    pub fn to_pyscf_atom_block(&self) -> Result<String, MolAdtError> {
        self.validate()?;
        Ok(self
            .atoms
            .values()
            .map(|atom| {
                format!(
                    "{} {:.8} {:.8} {:.8}",
                    atom.attributes.symbol,
                    atom.coordinate.x_angstrom,
                    atom.coordinate.y_angstrom,
                    atom.coordinate.z_angstrom
                )
            })
            .collect::<Vec<_>>()
            .join("; "))
    }

    /// Write the XYZ projection of this molecule to disk.
    pub fn write_xyz_to(&self, path: impl AsRef<std::path::Path>) -> Result<(), MolAdtError> {
        let xyz = self.to_xyz()?;
        std::fs::write(path, xyz).map_err(|error| MolAdtError::Io(error.to_string()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DftMoleculeRef {
    pub molecule_id: String,
    pub molecule_name: String,
    pub molecular_formula: String,
    pub molecule_artifact_id: Option<ArtifactId>,
    pub molecule_payload_hash: Option<String>,
    pub canonical_smiles: Option<String>,
}

impl DftMoleculeRef {
    #[must_use]
    pub fn unbound(molecule: &MoleculeAdt) -> Self {
        Self {
            molecule_id: molecule.molecule_id.clone(),
            molecule_name: molecule.name.clone(),
            molecular_formula: molecule.formula(),
            molecule_artifact_id: None,
            molecule_payload_hash: None,
            canonical_smiles: molecule.projections.canonical_smiles.clone(),
        }
    }

    #[must_use]
    pub fn with_artifact(mut self, artifact: &Artifact) -> Self {
        self.molecule_artifact_id = Some(artifact.id.clone());
        self.molecule_payload_hash = artifact
            .payload
            .as_ref()
            .map(|payload| payload.hash.clone());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DftBackend {
    PyScf,
    Gpu4PyScf,
    Ase,
    ExternalCommand,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DftMethodSpec {
    pub functional: String,
    pub basis_set: String,
    pub backend: DftBackend,
    pub dispersion: Option<String>,
    pub grid_level: Option<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DftJobKind {
    SinglePoint,
    GeometryOptimization,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DftRequest {
    pub request_id: String,
    pub molecule: DftMoleculeRef,
    pub total_charge: i32,
    pub multiplicity: u8,
    pub method: DftMethodSpec,
    pub job_kind: DftJobKind,
    pub requested_properties: Vec<String>,
    pub worker_hint: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum MolAdtError {
    EmptyMolecule,
    AtomIdMismatch { map_key: u32, atom_id: u32 },
    SelfBond(u32),
    UnknownAtomInEdge { a: u32, b: u32 },
    InvalidBondingSystem { system_id: u32, reason: String },
    Artifact(ArtifactError),
    Io(String),
    Worker(String),
}

impl Display for MolAdtError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyMolecule => write!(f, "molecule must contain at least one atom"),
            Self::AtomIdMismatch { map_key, atom_id } => write!(
                f,
                "atom map key {map_key} does not match embedded atom_id {atom_id}"
            ),
            Self::SelfBond(atom_id) => write!(f, "atom {atom_id} is bonded to itself"),
            Self::UnknownAtomInEdge { a, b } => {
                write!(f, "edge references unknown atom(s): {a}-{b}")
            }
            Self::InvalidBondingSystem { system_id, reason } => {
                write!(f, "invalid bonding system {system_id}: {reason}")
            }
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
            Self::Io(message) => write!(f, "io error: {message}"),
            Self::Worker(message) => write!(f, "smiles worker error: {message}"),
        }
    }
}

impl std::error::Error for MolAdtError {}

impl From<ArtifactError> for MolAdtError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

pub fn molecule_artifact(
    molecule: &MoleculeAdt,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, MolAdtError> {
    molecule.validate()?;
    ArtifactDraft {
        skill: SkillId(MOLECULE_ADT_SKILL.to_string()),
        agent,
        topic: format!("MolADT molecule {}", molecule.name),
        input_fingerprint: format!("moladt:{}:{}", molecule.molecule_id, molecule.formula()),
        output_cid: None,
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(MOLECULE_ADT_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(molecule)?),
    }
    .seal(signer, created_at_unix)
    .map_err(MolAdtError::Artifact)
}

pub fn dft_request_artifact(
    request: &DftRequest,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, MolAdtError> {
    let parent_artifact_ids = request
        .molecule
        .molecule_artifact_id
        .clone()
        .into_iter()
        .collect();
    ArtifactDraft {
        skill: SkillId(DFT_REQUEST_SKILL.to_string()),
        agent,
        topic: format!("DFT request {}", request.request_id),
        input_fingerprint: format!(
            "dft:{}:{}:{}",
            request.request_id, request.molecule.molecule_id, request.method.functional
        ),
        output_cid: None,
        parent_artifact_ids,
        schema_tags: BTreeSet::from([SchemaTag(DFT_REQUEST_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(request)?),
    }
    .seal(signer, created_at_unix)
    .map_err(MolAdtError::Artifact)
}

#[must_use]
pub fn demo_ferrocene_moladt() -> MoleculeAdt {
    let mut atoms = BTreeMap::new();
    atoms.insert(
        1,
        Atom::new(1, AtomicSymbol::Fe, Coordinate::new(0.0, 0.0, 0.0)),
    );

    let carbon_radius = 1.42_f64;
    let hydrogen_radius = 2.48_f64;
    let top_z = 1.67_f64;
    let bottom_z = -1.67_f64;
    let top_h_z = 1.92_f64;
    let bottom_h_z = -1.92_f64;
    for i in 0..5 {
        let angle = std::f64::consts::FRAC_PI_2 + f64::from(i) * std::f64::consts::TAU / 5.0;
        let carbon_id = 2 + i;
        let hydrogen_id = 7 + i;
        atoms.insert(
            carbon_id,
            Atom::new(
                carbon_id,
                AtomicSymbol::C,
                Coordinate::new(
                    carbon_radius * angle.cos(),
                    carbon_radius * angle.sin(),
                    top_z,
                ),
            ),
        );
        atoms.insert(
            hydrogen_id,
            Atom::new(
                hydrogen_id,
                AtomicSymbol::H,
                Coordinate::new(
                    hydrogen_radius * angle.cos(),
                    hydrogen_radius * angle.sin(),
                    top_h_z,
                ),
            ),
        );
    }
    for i in 0..5 {
        let angle = std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI / 5.0
            + f64::from(i) * std::f64::consts::TAU / 5.0;
        let carbon_id = 12 + i;
        let hydrogen_id = 17 + i;
        atoms.insert(
            carbon_id,
            Atom::new(
                carbon_id,
                AtomicSymbol::C,
                Coordinate::new(
                    carbon_radius * angle.cos(),
                    carbon_radius * angle.sin(),
                    bottom_z,
                ),
            ),
        );
        atoms.insert(
            hydrogen_id,
            Atom::new(
                hydrogen_id,
                AtomicSymbol::H,
                Coordinate::new(
                    hydrogen_radius * angle.cos(),
                    hydrogen_radius * angle.sin(),
                    bottom_h_z,
                ),
            ),
        );
    }

    let mut local_bonds = BTreeSet::new();
    for i in 0..5 {
        let top_c = 2 + i;
        let top_c_next = 2 + ((i + 1) % 5);
        let top_h = 7 + i;
        let bottom_c = 12 + i;
        let bottom_c_next = 12 + ((i + 1) % 5);
        let bottom_h = 17 + i;
        local_bonds.insert(Edge::new(top_c, top_c_next));
        local_bonds.insert(Edge::new(top_c, top_h));
        local_bonds.insert(Edge::new(bottom_c, bottom_c_next));
        local_bonds.insert(Edge::new(bottom_c, bottom_h));
    }

    let top_ring_edges = (0..5)
        .map(|i| Edge::new(2 + i, 2 + ((i + 1) % 5)))
        .collect::<BTreeSet<_>>();
    let bottom_ring_edges = (0..5)
        .map(|i| Edge::new(12 + i, 12 + ((i + 1) % 5)))
        .collect::<BTreeSet<_>>();
    let top_haptic_edges = (0..5).map(|i| Edge::new(1, 2 + i)).collect::<BTreeSet<_>>();
    let bottom_haptic_edges = (0..5)
        .map(|i| Edge::new(1, 12 + i))
        .collect::<BTreeSet<_>>();

    MoleculeAdt {
        molecule_id: "MOLADT.FERROCENE.001".to_string(),
        name: "ferrocene schematic MolADT".to_string(),
        atoms,
        local_bonds,
        systems: vec![
            BondingSystem {
                system_id: 1,
                shared_electrons: 6,
                member_edges: top_ring_edges,
                tag: Some("top_cp_pi_ring".to_string()),
            },
            BondingSystem {
                system_id: 2,
                shared_electrons: 6,
                member_edges: bottom_ring_edges,
                tag: Some("bottom_cp_pi_ring".to_string()),
            },
            BondingSystem {
                system_id: 3,
                shared_electrons: 6,
                member_edges: top_haptic_edges,
                tag: Some("eta5_top_cp_fe".to_string()),
            },
            BondingSystem {
                system_id: 4,
                shared_electrons: 6,
                member_edges: bottom_haptic_edges,
                tag: Some("eta5_bottom_cp_fe".to_string()),
            },
        ],
        provenance: MoleculeProvenance {
            source_kind: "deterministic-fixture".to_string(),
            source_ref: "MolADT-Bayes-inspired ferrocene schematic".to_string(),
            notes: vec![
                "Coordinates are a deterministic unoptimized sandwich geometry for artifact demos"
                    .to_string(),
                "A live DFT worker should replace this with conformer/geometry provenance before trusting energies"
                    .to_string(),
            ],
        },
        projections: MoleculeProjections {
            canonical_smiles: Some("[Fe]12(C=CC=C1)(C=CC=C2)".to_string()),
            inchi: None,
            inchikey: None,
        },
    }
}

fn format_formula_part(symbol: &AtomicSymbol, count: u32) -> String {
    if count == 1 {
        symbol.to_string()
    } else {
        format!("{symbol}{count}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer() -> ArtifactSigner {
        ArtifactSigner::from_seed([77; 32])
    }

    #[test]
    fn ferrocene_fixture_validates_and_projects() {
        let molecule = demo_ferrocene_moladt();
        molecule.validate().expect("valid ferrocene fixture");
        assert_eq!(molecule.atoms.len(), 21);
        assert_eq!(molecule.formula(), "C10H10Fe");
        let xyz = molecule.to_xyz().expect("xyz");
        assert!(xyz.contains("Fe 0.00000000 0.00000000 0.00000000"));
        let pyscf = molecule.to_pyscf_atom_block().expect("pyscf");
        assert!(pyscf.starts_with("Fe 0.00000000 0.00000000 0.00000000"));
    }

    #[test]
    fn molecule_artifact_is_payload_bound() {
        let molecule = demo_ferrocene_moladt();
        let artifact = molecule_artifact(
            &molecule,
            AgentId("operator.chimiaclaw.eth".to_string()),
            &signer(),
            1,
        )
        .expect("molecule artifact");
        artifact.verify().expect("artifact verifies");
        artifact
            .verify_payload_value(&molecule)
            .expect("payload binding");
        assert!(artifact
            .schema_tags
            .contains(&SchemaTag(MOLECULE_ADT_TAG.to_string())));
    }

    #[test]
    fn dft_request_artifact_links_to_molecule_artifact() {
        let molecule = demo_ferrocene_moladt();
        let molecule_artifact = molecule_artifact(
            &molecule,
            AgentId("operator.chimiaclaw.eth".to_string()),
            &signer(),
            1,
        )
        .expect("molecule artifact");
        let request = DftRequest {
            request_id: "REQ.DFT.FERROCENE.001".to_string(),
            molecule: DftMoleculeRef::unbound(&molecule).with_artifact(&molecule_artifact),
            total_charge: molecule.total_formal_charge(),
            multiplicity: 1,
            method: DftMethodSpec {
                functional: "skala-1.1".to_string(),
                basis_set: "def2-tzvp".to_string(),
                backend: DftBackend::PyScf,
                dispersion: Some("dftd3".to_string()),
                grid_level: Some(3),
            },
            job_kind: DftJobKind::SinglePoint,
            requested_properties: vec!["total_energy".to_string()],
            worker_hint: Some("CHIMIACLAW_DFT_COMMAND".to_string()),
        };
        let request_artifact = dft_request_artifact(
            &request,
            AgentId("operator.chimiaclaw.eth".to_string()),
            &signer(),
            2,
        )
        .expect("request artifact");
        request_artifact.verify().expect("request verifies");
        assert!(request_artifact.has_parent(&molecule_artifact.id));
    }
}
