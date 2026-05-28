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
use std::str::FromStr;

pub mod geometry;
pub mod library;
pub mod render;
pub mod worker;

pub const MOLECULE_ADT_TAG: &str = "chem.molecule.adt";
pub const MOLECULE_ADT_SKILL: &str = "chem.molecule.adt.v1";
pub const DFT_REQUEST_TAG: &str = "chem.dft.request";
pub const DFT_REQUEST_SKILL: &str = "chem.dft.request.v1";

/// Canonical element coverage for the MolADT family.
///
/// This crate is the **single source of truth** for atomic-symbol coverage
/// across every language binding (Rust, Python `literature_synthesis`, Haskell
/// `MolADT-Bayes`). The coverage is periods 1–6 in full plus Th and U from
/// the actinides (85 elements). Declared in atomic-number (Z) order so the
/// derived `Ord` sorts by Z, which is what every downstream consumer expects.
///
/// Extending coverage is a one-place change: append a variant in Z order,
/// add arms to the four exhaustive matches below, and append a row to
/// [`MOLADT_ELEMENT_MANIFEST`]. The Python and Haskell mirrors validate
/// themselves against the manifest at test time.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum AtomicSymbol {
    // Period 1
    H,  He,
    // Period 2
    Li, Be, B,  C,  N,  O,  F,  Ne,
    // Period 3
    Na, Mg, Al, Si, P,  S,  Cl, Ar,
    // Period 4
    K,  Ca, Sc, Ti, V,  Cr, Mn, Fe, Co, Ni, Cu, Zn,
    Ga, Ge, As, Se, Br, Kr,
    // Period 5
    Rb, Sr, Y,  Zr, Nb, Mo, Tc, Ru, Rh, Pd, Ag, Cd,
    In, Sn, Sb, Te, I,  Xe,
    // Period 6
    Cs, Ba, La, Ce, Pr, Nd, Pm, Sm, Eu, Gd, Tb, Dy,
    Ho, Er, Tm, Yb, Lu,
    Hf, Ta, W,  Re, Os, Ir, Pt, Au, Hg,
    Tl, Pb, Bi,
    // Selected actinides
    Th, U,
}

/// Error returned when a string cannot be parsed as an [`AtomicSymbol`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseAtomicSymbolError {
    symbol: String,
}

impl ParseAtomicSymbolError {
    /// The offending input, with surrounding whitespace stripped.
    #[must_use]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

impl Display for ParseAtomicSymbolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported atomic symbol {:?}", self.symbol)
    }
}

impl std::error::Error for ParseAtomicSymbolError {}

impl AtomicSymbol {
    /// Parse the textual form of an atomic symbol (e.g. `"Cu"`).
    ///
    /// Surrounding whitespace is trimmed. Case is **significant** —
    /// chemistry convention requires the canonical capitalisation (`"Cu"`,
    /// not `"CU"` or `"cu"`).
    #[must_use]
    pub fn from_symbol(symbol: &str) -> Option<Self> {
        // Linear scan over the manifest keeps the lookup table and the
        // exhaustive matches in lock-step — there is no separate hash table
        // to maintain.
        let trimmed = symbol.trim();
        MOLADT_ELEMENT_MANIFEST
            .iter()
            .find(|(_, _, _, s)| *s == trimmed)
            .map(|(sym, _, _, _)| *sym)
    }

    /// All 85 supported symbols in Z order.
    #[must_use]
    pub fn all() -> &'static [AtomicSymbol] {
        ATOMIC_SYMBOLS_IN_Z_ORDER
    }

    /// Canonical textual representation. Matches the enum constructor name
    /// and the wire-format value in the Python/Haskell mirrors.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::H => "H",   Self::He => "He",
            Self::Li => "Li", Self::Be => "Be", Self::B => "B",  Self::C => "C",
            Self::N => "N",   Self::O => "O",   Self::F => "F",  Self::Ne => "Ne",
            Self::Na => "Na", Self::Mg => "Mg", Self::Al => "Al", Self::Si => "Si",
            Self::P => "P",   Self::S => "S",   Self::Cl => "Cl", Self::Ar => "Ar",
            Self::K => "K",   Self::Ca => "Ca", Self::Sc => "Sc", Self::Ti => "Ti",
            Self::V => "V",   Self::Cr => "Cr", Self::Mn => "Mn", Self::Fe => "Fe",
            Self::Co => "Co", Self::Ni => "Ni", Self::Cu => "Cu", Self::Zn => "Zn",
            Self::Ga => "Ga", Self::Ge => "Ge", Self::As => "As", Self::Se => "Se",
            Self::Br => "Br", Self::Kr => "Kr",
            Self::Rb => "Rb", Self::Sr => "Sr", Self::Y => "Y",   Self::Zr => "Zr",
            Self::Nb => "Nb", Self::Mo => "Mo", Self::Tc => "Tc", Self::Ru => "Ru",
            Self::Rh => "Rh", Self::Pd => "Pd", Self::Ag => "Ag", Self::Cd => "Cd",
            Self::In => "In", Self::Sn => "Sn", Self::Sb => "Sb", Self::Te => "Te",
            Self::I => "I",   Self::Xe => "Xe",
            Self::Cs => "Cs", Self::Ba => "Ba", Self::La => "La", Self::Ce => "Ce",
            Self::Pr => "Pr", Self::Nd => "Nd", Self::Pm => "Pm", Self::Sm => "Sm",
            Self::Eu => "Eu", Self::Gd => "Gd", Self::Tb => "Tb", Self::Dy => "Dy",
            Self::Ho => "Ho", Self::Er => "Er", Self::Tm => "Tm", Self::Yb => "Yb",
            Self::Lu => "Lu",
            Self::Hf => "Hf", Self::Ta => "Ta", Self::W => "W",   Self::Re => "Re",
            Self::Os => "Os", Self::Ir => "Ir", Self::Pt => "Pt", Self::Au => "Au",
            Self::Hg => "Hg",
            Self::Tl => "Tl", Self::Pb => "Pb", Self::Bi => "Bi",
            Self::Th => "Th", Self::U => "U",
        }
    }

    /// Atomic number (Z).
    #[must_use]
    pub const fn atomic_number(&self) -> u8 {
        match self {
            Self::H => 1,   Self::He => 2,
            Self::Li => 3,  Self::Be => 4,  Self::B => 5,   Self::C => 6,
            Self::N => 7,   Self::O => 8,   Self::F => 9,   Self::Ne => 10,
            Self::Na => 11, Self::Mg => 12, Self::Al => 13, Self::Si => 14,
            Self::P => 15,  Self::S => 16,  Self::Cl => 17, Self::Ar => 18,
            Self::K => 19,  Self::Ca => 20, Self::Sc => 21, Self::Ti => 22,
            Self::V => 23,  Self::Cr => 24, Self::Mn => 25, Self::Fe => 26,
            Self::Co => 27, Self::Ni => 28, Self::Cu => 29, Self::Zn => 30,
            Self::Ga => 31, Self::Ge => 32, Self::As => 33, Self::Se => 34,
            Self::Br => 35, Self::Kr => 36,
            Self::Rb => 37, Self::Sr => 38, Self::Y => 39,  Self::Zr => 40,
            Self::Nb => 41, Self::Mo => 42, Self::Tc => 43, Self::Ru => 44,
            Self::Rh => 45, Self::Pd => 46, Self::Ag => 47, Self::Cd => 48,
            Self::In => 49, Self::Sn => 50, Self::Sb => 51, Self::Te => 52,
            Self::I => 53,  Self::Xe => 54,
            Self::Cs => 55, Self::Ba => 56, Self::La => 57, Self::Ce => 58,
            Self::Pr => 59, Self::Nd => 60, Self::Pm => 61, Self::Sm => 62,
            Self::Eu => 63, Self::Gd => 64, Self::Tb => 65, Self::Dy => 66,
            Self::Ho => 67, Self::Er => 68, Self::Tm => 69, Self::Yb => 70,
            Self::Lu => 71,
            Self::Hf => 72, Self::Ta => 73, Self::W => 74,  Self::Re => 75,
            Self::Os => 76, Self::Ir => 77, Self::Pt => 78, Self::Au => 79,
            Self::Hg => 80,
            Self::Tl => 81, Self::Pb => 82, Self::Bi => 83,
            Self::Th => 90, Self::U => 92,
        }
    }

    /// CIAAW standard atomic weight (g/mol). Synthetic / radioactive
    /// elements (Tc, Pm, Th, U) use the mass number of the most stable or
    /// most common isotope.
    #[must_use]
    pub const fn default_atomic_weight(&self) -> f64 {
        match self {
            Self::H => 1.008,    Self::He => 4.0026,
            Self::Li => 6.94,    Self::Be => 9.0122,  Self::B => 10.811,
            Self::C => 12.011,   Self::N => 14.007,   Self::O => 15.999,
            Self::F => 18.998,   Self::Ne => 20.180,
            Self::Na => 22.990,  Self::Mg => 24.305,  Self::Al => 26.982,
            Self::Si => 28.085,  Self::P => 30.974,   Self::S => 32.065,
            Self::Cl => 35.453,  Self::Ar => 39.948,
            Self::K => 39.098,   Self::Ca => 40.078,  Self::Sc => 44.956,
            Self::Ti => 47.867,  Self::V => 50.942,   Self::Cr => 51.996,
            Self::Mn => 54.938,  Self::Fe => 55.845,  Self::Co => 58.933,
            Self::Ni => 58.693,  Self::Cu => 63.546,  Self::Zn => 65.38,
            Self::Ga => 69.723,  Self::Ge => 72.630,  Self::As => 74.922,
            Self::Se => 78.971,  Self::Br => 79.904,  Self::Kr => 83.798,
            Self::Rb => 85.468,  Self::Sr => 87.62,   Self::Y => 88.906,
            Self::Zr => 91.224,  Self::Nb => 92.906,  Self::Mo => 95.95,
            Self::Tc => 98.0,    Self::Ru => 101.07,  Self::Rh => 102.906,
            Self::Pd => 106.42,  Self::Ag => 107.868, Self::Cd => 112.414,
            Self::In => 114.818, Self::Sn => 118.710, Self::Sb => 121.760,
            Self::Te => 127.60,  Self::I => 126.904,  Self::Xe => 131.293,
            Self::Cs => 132.905, Self::Ba => 137.327, Self::La => 138.905,
            Self::Ce => 140.116, Self::Pr => 140.908, Self::Nd => 144.242,
            Self::Pm => 145.0,   Self::Sm => 150.36,  Self::Eu => 151.964,
            Self::Gd => 157.25,  Self::Tb => 158.925, Self::Dy => 162.500,
            Self::Ho => 164.930, Self::Er => 167.259, Self::Tm => 168.934,
            Self::Yb => 173.045, Self::Lu => 174.967,
            Self::Hf => 178.49,  Self::Ta => 180.948, Self::W => 183.84,
            Self::Re => 186.207, Self::Os => 190.23,  Self::Ir => 192.217,
            Self::Pt => 195.084, Self::Au => 196.967, Self::Hg => 200.592,
            Self::Tl => 204.38,  Self::Pb => 207.2,   Self::Bi => 208.980,
            Self::Th => 232.038, Self::U => 238.029,
        }
    }
}

impl Display for AtomicSymbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AtomicSymbol {
    type Err = ParseAtomicSymbolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_symbol(s).ok_or_else(|| ParseAtomicSymbolError {
            symbol: s.trim().to_string(),
        })
    }
}

/// Canonical element manifest. Each row is `(symbol, atomic number,
/// standard atomic weight, textual representation)` and is the authoritative
/// data the Python and Haskell mirrors validate against.
pub const MOLADT_ELEMENT_MANIFEST: &[(AtomicSymbol, u8, f64, &str)] = &[
    (AtomicSymbol::H,  1,   1.008,   "H"),
    (AtomicSymbol::He, 2,   4.0026,  "He"),
    (AtomicSymbol::Li, 3,   6.94,    "Li"),
    (AtomicSymbol::Be, 4,   9.0122,  "Be"),
    (AtomicSymbol::B,  5,  10.811,   "B"),
    (AtomicSymbol::C,  6,  12.011,   "C"),
    (AtomicSymbol::N,  7,  14.007,   "N"),
    (AtomicSymbol::O,  8,  15.999,   "O"),
    (AtomicSymbol::F,  9,  18.998,   "F"),
    (AtomicSymbol::Ne, 10, 20.180,   "Ne"),
    (AtomicSymbol::Na, 11, 22.990,   "Na"),
    (AtomicSymbol::Mg, 12, 24.305,   "Mg"),
    (AtomicSymbol::Al, 13, 26.982,   "Al"),
    (AtomicSymbol::Si, 14, 28.085,   "Si"),
    (AtomicSymbol::P,  15, 30.974,   "P"),
    (AtomicSymbol::S,  16, 32.065,   "S"),
    (AtomicSymbol::Cl, 17, 35.453,   "Cl"),
    (AtomicSymbol::Ar, 18, 39.948,   "Ar"),
    (AtomicSymbol::K,  19, 39.098,   "K"),
    (AtomicSymbol::Ca, 20, 40.078,   "Ca"),
    (AtomicSymbol::Sc, 21, 44.956,   "Sc"),
    (AtomicSymbol::Ti, 22, 47.867,   "Ti"),
    (AtomicSymbol::V,  23, 50.942,   "V"),
    (AtomicSymbol::Cr, 24, 51.996,   "Cr"),
    (AtomicSymbol::Mn, 25, 54.938,   "Mn"),
    (AtomicSymbol::Fe, 26, 55.845,   "Fe"),
    (AtomicSymbol::Co, 27, 58.933,   "Co"),
    (AtomicSymbol::Ni, 28, 58.693,   "Ni"),
    (AtomicSymbol::Cu, 29, 63.546,   "Cu"),
    (AtomicSymbol::Zn, 30, 65.38,    "Zn"),
    (AtomicSymbol::Ga, 31, 69.723,   "Ga"),
    (AtomicSymbol::Ge, 32, 72.630,   "Ge"),
    (AtomicSymbol::As, 33, 74.922,   "As"),
    (AtomicSymbol::Se, 34, 78.971,   "Se"),
    (AtomicSymbol::Br, 35, 79.904,   "Br"),
    (AtomicSymbol::Kr, 36, 83.798,   "Kr"),
    (AtomicSymbol::Rb, 37, 85.468,   "Rb"),
    (AtomicSymbol::Sr, 38, 87.62,    "Sr"),
    (AtomicSymbol::Y,  39, 88.906,   "Y"),
    (AtomicSymbol::Zr, 40, 91.224,   "Zr"),
    (AtomicSymbol::Nb, 41, 92.906,   "Nb"),
    (AtomicSymbol::Mo, 42, 95.95,    "Mo"),
    (AtomicSymbol::Tc, 43, 98.0,     "Tc"),
    (AtomicSymbol::Ru, 44, 101.07,   "Ru"),
    (AtomicSymbol::Rh, 45, 102.906,  "Rh"),
    (AtomicSymbol::Pd, 46, 106.42,   "Pd"),
    (AtomicSymbol::Ag, 47, 107.868,  "Ag"),
    (AtomicSymbol::Cd, 48, 112.414,  "Cd"),
    (AtomicSymbol::In, 49, 114.818,  "In"),
    (AtomicSymbol::Sn, 50, 118.710,  "Sn"),
    (AtomicSymbol::Sb, 51, 121.760,  "Sb"),
    (AtomicSymbol::Te, 52, 127.60,   "Te"),
    (AtomicSymbol::I,  53, 126.904,  "I"),
    (AtomicSymbol::Xe, 54, 131.293,  "Xe"),
    (AtomicSymbol::Cs, 55, 132.905,  "Cs"),
    (AtomicSymbol::Ba, 56, 137.327,  "Ba"),
    (AtomicSymbol::La, 57, 138.905,  "La"),
    (AtomicSymbol::Ce, 58, 140.116,  "Ce"),
    (AtomicSymbol::Pr, 59, 140.908,  "Pr"),
    (AtomicSymbol::Nd, 60, 144.242,  "Nd"),
    (AtomicSymbol::Pm, 61, 145.0,    "Pm"),
    (AtomicSymbol::Sm, 62, 150.36,   "Sm"),
    (AtomicSymbol::Eu, 63, 151.964,  "Eu"),
    (AtomicSymbol::Gd, 64, 157.25,   "Gd"),
    (AtomicSymbol::Tb, 65, 158.925,  "Tb"),
    (AtomicSymbol::Dy, 66, 162.500,  "Dy"),
    (AtomicSymbol::Ho, 67, 164.930,  "Ho"),
    (AtomicSymbol::Er, 68, 167.259,  "Er"),
    (AtomicSymbol::Tm, 69, 168.934,  "Tm"),
    (AtomicSymbol::Yb, 70, 173.045,  "Yb"),
    (AtomicSymbol::Lu, 71, 174.967,  "Lu"),
    (AtomicSymbol::Hf, 72, 178.49,   "Hf"),
    (AtomicSymbol::Ta, 73, 180.948,  "Ta"),
    (AtomicSymbol::W,  74, 183.84,   "W"),
    (AtomicSymbol::Re, 75, 186.207,  "Re"),
    (AtomicSymbol::Os, 76, 190.23,   "Os"),
    (AtomicSymbol::Ir, 77, 192.217,  "Ir"),
    (AtomicSymbol::Pt, 78, 195.084,  "Pt"),
    (AtomicSymbol::Au, 79, 196.967,  "Au"),
    (AtomicSymbol::Hg, 80, 200.592,  "Hg"),
    (AtomicSymbol::Tl, 81, 204.38,   "Tl"),
    (AtomicSymbol::Pb, 82, 207.2,    "Pb"),
    (AtomicSymbol::Bi, 83, 208.980,  "Bi"),
    (AtomicSymbol::Th, 90, 232.038,  "Th"),
    (AtomicSymbol::U,  92, 238.029,  "U"),
];

/// Convenience static parallel to [`MOLADT_ELEMENT_MANIFEST`] containing
/// just the symbols in Z order, useful when callers only need to iterate
/// the supported elements without their associated metadata.
const ATOMIC_SYMBOLS_IN_Z_ORDER: &[AtomicSymbol] = &[
    AtomicSymbol::H,  AtomicSymbol::He,
    AtomicSymbol::Li, AtomicSymbol::Be, AtomicSymbol::B,  AtomicSymbol::C,
    AtomicSymbol::N,  AtomicSymbol::O,  AtomicSymbol::F,  AtomicSymbol::Ne,
    AtomicSymbol::Na, AtomicSymbol::Mg, AtomicSymbol::Al, AtomicSymbol::Si,
    AtomicSymbol::P,  AtomicSymbol::S,  AtomicSymbol::Cl, AtomicSymbol::Ar,
    AtomicSymbol::K,  AtomicSymbol::Ca, AtomicSymbol::Sc, AtomicSymbol::Ti,
    AtomicSymbol::V,  AtomicSymbol::Cr, AtomicSymbol::Mn, AtomicSymbol::Fe,
    AtomicSymbol::Co, AtomicSymbol::Ni, AtomicSymbol::Cu, AtomicSymbol::Zn,
    AtomicSymbol::Ga, AtomicSymbol::Ge, AtomicSymbol::As, AtomicSymbol::Se,
    AtomicSymbol::Br, AtomicSymbol::Kr,
    AtomicSymbol::Rb, AtomicSymbol::Sr, AtomicSymbol::Y,  AtomicSymbol::Zr,
    AtomicSymbol::Nb, AtomicSymbol::Mo, AtomicSymbol::Tc, AtomicSymbol::Ru,
    AtomicSymbol::Rh, AtomicSymbol::Pd, AtomicSymbol::Ag, AtomicSymbol::Cd,
    AtomicSymbol::In, AtomicSymbol::Sn, AtomicSymbol::Sb, AtomicSymbol::Te,
    AtomicSymbol::I,  AtomicSymbol::Xe,
    AtomicSymbol::Cs, AtomicSymbol::Ba, AtomicSymbol::La, AtomicSymbol::Ce,
    AtomicSymbol::Pr, AtomicSymbol::Nd, AtomicSymbol::Pm, AtomicSymbol::Sm,
    AtomicSymbol::Eu, AtomicSymbol::Gd, AtomicSymbol::Tb, AtomicSymbol::Dy,
    AtomicSymbol::Ho, AtomicSymbol::Er, AtomicSymbol::Tm, AtomicSymbol::Yb,
    AtomicSymbol::Lu,
    AtomicSymbol::Hf, AtomicSymbol::Ta, AtomicSymbol::W,  AtomicSymbol::Re,
    AtomicSymbol::Os, AtomicSymbol::Ir, AtomicSymbol::Pt, AtomicSymbol::Au,
    AtomicSymbol::Hg,
    AtomicSymbol::Tl, AtomicSymbol::Pb, AtomicSymbol::Bi,
    AtomicSymbol::Th, AtomicSymbol::U,
];

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
    fn manifest_is_z_sorted_and_round_trips_through_from_symbol() {
        // The manifest is the cross-language source of truth, so every row
        // must be discoverable by symbol and the column data must agree with
        // the per-element helper methods.
        let mut last_z = 0_u8;
        for (symbol, z, weight, text) in MOLADT_ELEMENT_MANIFEST {
            assert_eq!(symbol.as_str(), *text, "as_str mismatch for Z={z}");
            assert_eq!(symbol.atomic_number(), *z, "atomic_number mismatch for {text}");
            assert!(
                (symbol.default_atomic_weight() - *weight).abs() < 1e-9,
                "atomic_weight mismatch for {text}"
            );
            assert_eq!(
                AtomicSymbol::from_symbol(text),
                Some(*symbol),
                "from_symbol round-trip failed for {text}"
            );
            assert!(
                *z >= last_z,
                "manifest is not sorted by Z at element {text} (z={z}, previous z={last_z})"
            );
            last_z = *z;
        }
    }

    #[test]
    fn all_returns_complete_z_sorted_list() {
        let symbols = AtomicSymbol::all();
        assert_eq!(symbols.len(), MOLADT_ELEMENT_MANIFEST.len());
        for (lhs, rhs) in symbols.iter().zip(MOLADT_ELEMENT_MANIFEST.iter()) {
            assert_eq!(*lhs, rhs.0);
        }
        // Cu, Sn, Ge, Au, U specifically are addressable from the public API.
        for sym in [
            AtomicSymbol::Cu,
            AtomicSymbol::Sn,
            AtomicSymbol::Ge,
            AtomicSymbol::Au,
            AtomicSymbol::U,
        ] {
            assert!(symbols.contains(&sym), "missing {sym:?}");
        }
    }

    #[test]
    fn from_str_trims_whitespace_and_rejects_unknown() {
        assert_eq!("  Cu ".parse::<AtomicSymbol>().unwrap(), AtomicSymbol::Cu);
        let err = "Xx".parse::<AtomicSymbol>().unwrap_err();
        assert_eq!(err.symbol(), "Xx");
        assert!(format!("{err}").contains("unsupported atomic symbol"));
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
