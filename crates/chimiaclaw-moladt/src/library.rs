//! Curated SMILES \u2192 [`MoleculeAdt`] library.
//!
//! Real SMILES \u2192 3D embedding requires RDKit or OpenBabel and a force-field
//! optimization step that does not belong inside this crate. As a hackathon
//! bridge, we keep a small hand-curated table of common ORD substrates with
//! deterministic schematic geometries so that downstream DFT workers always
//! receive a payload-bound MolADT rather than a bare SMILES string.
//!
//! Every fixture is flagged in [`MoleculeProvenance::source_kind`] as a
//! schematic (not a re-optimized geometry), so a real DFT worker is expected
//! to either accept the schematic and re-optimize internally, or refuse the
//! job and ask for a worker-side geometry pass.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Atom, AtomicSymbol, BondingSystem, Coordinate, Edge, MoleculeAdt, MoleculeProjections,
    MoleculeProvenance,
};

/// Resolve a SMILES string to a curated [`MoleculeAdt`] if available.
///
/// SMILES inputs are normalized by trimming whitespace; matching is exact.
/// Returns `None` for SMILES that are not yet in the library.
#[must_use]
pub fn resolve_smiles(smiles: &str) -> Option<MoleculeAdt> {
    let key = smiles.trim();
    match key {
        "O" => Some(water()),
        "Brc1ccccc1" => Some(bromobenzene()),
        "OB(O)c1ccccc1" | "B(O)(O)c1ccccc1" => Some(phenylboronic_acid()),
        "c1ccc(-c2ccccc2)cc1" | "c1ccccc1-c1ccccc1" => Some(biphenyl()),
        "Cc1ccccc1" => Some(toluene()),
        "CO" => Some(methanol()),
        "CCO" => Some(ethanol()),
        "CC(=O)O" => Some(acetic_acid()),
        "N" => Some(ammonia()),
        "c1ccccc1" | "C1=CC=CC=C1" => Some(benzene()),
        _ => None,
    }
}

/// Identify SMILES strings that are explicitly not safe to silently submit to
/// a DFT engine without an external geometry pre-pass.
///
/// These are typically multi-component salts, transition-metal complexes, or
/// other structures where naive SMILES interpretation would produce a wrong
/// MolADT.
#[must_use]
pub fn is_known_unsafe_for_dft(smiles: &str) -> bool {
    let key = smiles.trim();
    key.contains('.')
        || key.contains("[Pd")
        || key.contains("[Ni")
        || key.contains("[Pt")
        || key.contains("[K+]")
        || key.contains("[Na+]")
        || key.contains("[Li+]")
        || key.contains("[Cs+]")
}

#[must_use]
fn build_molecule(
    molecule_id: &str,
    name: &str,
    canonical_smiles: &str,
    inchi: Option<&str>,
    inchikey: Option<&str>,
    atom_specs: &[(u32, AtomicSymbol, Coordinate)],
    bond_specs: &[(u32, u32)],
    aromatic_ring: Option<&[u32]>,
) -> MoleculeAdt {
    let mut atoms = BTreeMap::new();
    for (id, symbol, coord) in atom_specs {
        atoms.insert(*id, Atom::new(*id, symbol.clone(), coord.clone()));
    }
    let local_bonds: BTreeSet<Edge> = bond_specs
        .iter()
        .copied()
        .map(|(a, b)| Edge::new(a, b))
        .collect();
    let mut systems = Vec::new();
    if let Some(ring) = aromatic_ring {
        let mut member_edges = BTreeSet::new();
        for window in ring.windows(2) {
            member_edges.insert(Edge::new(window[0], window[1]));
        }
        if let (Some(first), Some(last)) = (ring.first(), ring.last()) {
            if first != last {
                member_edges.insert(Edge::new(*first, *last));
            }
        }
        systems.push(BondingSystem {
            system_id: 1,
            shared_electrons: 6,
            member_edges,
            tag: Some("aromatic_ring".to_string()),
        });
    }
    MoleculeAdt {
        molecule_id: molecule_id.to_string(),
        name: name.to_string(),
        atoms,
        local_bonds,
        systems,
        provenance: MoleculeProvenance {
            source_kind: "schematic-curated".to_string(),
            source_ref: format!("chimiaclaw-moladt::library::{molecule_id}"),
            notes: vec![
                "Coordinates are deterministic schematic placeholders for artifact demos."
                    .to_string(),
                "DFT workers must apply their own geometry optimization before energy is trusted."
                    .to_string(),
            ],
        },
        projections: MoleculeProjections {
            canonical_smiles: Some(canonical_smiles.to_string()),
            inchi: inchi.map(str::to_string),
            inchikey: inchikey.map(str::to_string),
        },
    }
}

#[must_use]
pub fn water() -> MoleculeAdt {
    build_molecule(
        "MOLADT.WATER.001",
        "water",
        "O",
        Some("InChI=1S/H2O/h1H2"),
        Some("XLYOFNOQVPJJNP-UHFFFAOYSA-N"),
        &[
            (1, AtomicSymbol::O, Coordinate::new(0.000, 0.000, 0.000)),
            (2, AtomicSymbol::H, Coordinate::new(0.757, 0.586, 0.000)),
            (3, AtomicSymbol::H, Coordinate::new(-0.757, 0.586, 0.000)),
        ],
        &[(1, 2), (1, 3)],
        None,
    )
}

#[must_use]
pub fn ammonia() -> MoleculeAdt {
    build_molecule(
        "MOLADT.AMMONIA.001",
        "ammonia",
        "N",
        Some("InChI=1S/H3N/h1H3"),
        Some("QGZKDVFQNNGYKY-UHFFFAOYSA-N"),
        &[
            (1, AtomicSymbol::N, Coordinate::new(0.000, 0.000, 0.000)),
            (2, AtomicSymbol::H, Coordinate::new(0.939, 0.000, -0.341)),
            (3, AtomicSymbol::H, Coordinate::new(-0.470, 0.814, -0.341)),
            (4, AtomicSymbol::H, Coordinate::new(-0.470, -0.814, -0.341)),
        ],
        &[(1, 2), (1, 3), (1, 4)],
        None,
    )
}

#[must_use]
pub fn methanol() -> MoleculeAdt {
    build_molecule(
        "MOLADT.METHANOL.001",
        "methanol",
        "CO",
        Some("InChI=1S/CH4O/c1-2/h2H,1H3"),
        Some("OKKJLVBELUTLKV-UHFFFAOYSA-N"),
        &[
            (1, AtomicSymbol::C, Coordinate::new(0.000, 0.000, 0.000)),
            (2, AtomicSymbol::O, Coordinate::new(1.420, 0.000, 0.000)),
            (3, AtomicSymbol::H, Coordinate::new(-0.363, 1.029, 0.000)),
            (4, AtomicSymbol::H, Coordinate::new(-0.363, -0.514, 0.891)),
            (5, AtomicSymbol::H, Coordinate::new(-0.363, -0.514, -0.891)),
            (6, AtomicSymbol::H, Coordinate::new(1.778, 0.929, 0.000)),
        ],
        &[(1, 2), (1, 3), (1, 4), (1, 5), (2, 6)],
        None,
    )
}

#[must_use]
pub fn ethanol() -> MoleculeAdt {
    build_molecule(
        "MOLADT.ETHANOL.001",
        "ethanol",
        "CCO",
        Some("InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3"),
        Some("LFQSCWFLJHTTHZ-UHFFFAOYSA-N"),
        &[
            (1, AtomicSymbol::C, Coordinate::new(-1.230, 0.000, 0.000)),
            (2, AtomicSymbol::C, Coordinate::new(0.000, 0.770, 0.000)),
            (3, AtomicSymbol::O, Coordinate::new(1.230, 0.000, 0.000)),
            (4, AtomicSymbol::H, Coordinate::new(-1.230, -0.630, 0.890)),
            (5, AtomicSymbol::H, Coordinate::new(-1.230, -0.630, -0.890)),
            (6, AtomicSymbol::H, Coordinate::new(-2.130, 0.620, 0.000)),
            (7, AtomicSymbol::H, Coordinate::new(0.000, 1.400, 0.890)),
            (8, AtomicSymbol::H, Coordinate::new(0.000, 1.400, -0.890)),
            (9, AtomicSymbol::H, Coordinate::new(2.000, 0.560, 0.000)),
        ],
        &[
            (1, 2),
            (2, 3),
            (1, 4),
            (1, 5),
            (1, 6),
            (2, 7),
            (2, 8),
            (3, 9),
        ],
        None,
    )
}

#[must_use]
pub fn acetic_acid() -> MoleculeAdt {
    build_molecule(
        "MOLADT.ACETIC_ACID.001",
        "acetic acid",
        "CC(=O)O",
        Some("InChI=1S/C2H4O2/c1-2(3)4/h1H3,(H,3,4)"),
        Some("QTBSBXVTEAMEQO-UHFFFAOYSA-N"),
        &[
            (1, AtomicSymbol::C, Coordinate::new(-1.222, 0.000, 0.000)),
            (2, AtomicSymbol::C, Coordinate::new(0.000, 0.770, 0.000)),
            (3, AtomicSymbol::O, Coordinate::new(0.000, 1.997, 0.000)),
            (4, AtomicSymbol::O, Coordinate::new(1.232, 0.150, 0.000)),
            (5, AtomicSymbol::H, Coordinate::new(-1.222, -0.629, 0.890)),
            (6, AtomicSymbol::H, Coordinate::new(-1.222, -0.629, -0.890)),
            (7, AtomicSymbol::H, Coordinate::new(-2.115, 0.620, 0.000)),
            (8, AtomicSymbol::H, Coordinate::new(2.000, 0.700, 0.000)),
        ],
        &[(1, 2), (2, 3), (2, 4), (1, 5), (1, 6), (1, 7), (4, 8)],
        None,
    )
}

#[must_use]
pub fn benzene() -> MoleculeAdt {
    let radius = 1.397_f64;
    let h_radius = 2.479_f64;
    let mut atom_specs = Vec::new();
    let mut bond_specs = Vec::new();
    let mut ring = Vec::new();
    for i in 0..6 {
        let angle = std::f64::consts::PI / 6.0 + f64::from(i) * std::f64::consts::TAU / 6.0;
        let c_id = 1 + i;
        let h_id = 7 + i;
        atom_specs.push((
            c_id,
            AtomicSymbol::C,
            Coordinate::new(radius * angle.cos(), radius * angle.sin(), 0.0),
        ));
        atom_specs.push((
            h_id,
            AtomicSymbol::H,
            Coordinate::new(h_radius * angle.cos(), h_radius * angle.sin(), 0.0),
        ));
        bond_specs.push((c_id, h_id));
        bond_specs.push((c_id, 1 + ((i + 1) % 6)));
        ring.push(c_id);
    }
    build_molecule(
        "MOLADT.BENZENE.001",
        "benzene",
        "c1ccccc1",
        Some("InChI=1S/C6H6/c1-2-4-6-5-3-1/h1-6H"),
        Some("UHOVQNZJYSORNB-UHFFFAOYSA-N"),
        &atom_specs,
        &bond_specs,
        Some(&ring),
    )
}

#[must_use]
pub fn toluene() -> MoleculeAdt {
    let mut molecule = benzene();
    molecule.molecule_id = "MOLADT.TOLUENE.001".to_string();
    molecule.name = "toluene".to_string();
    molecule.projections = MoleculeProjections {
        canonical_smiles: Some("Cc1ccccc1".to_string()),
        inchi: Some("InChI=1S/C7H8/c1-7-5-3-2-4-6-7/h2-6H,1H3".to_string()),
        inchikey: Some("YXFVVABEGXRONW-UHFFFAOYSA-N".to_string()),
    };
    let methyl_carbon = (atom_id_after(&molecule), AtomicSymbol::C);
    let methyl_carbon_id = methyl_carbon.0;
    let ring_anchor_id = 1_u32;
    let ring_anchor_coord = molecule
        .atoms
        .get(&ring_anchor_id)
        .expect("ring anchor present")
        .coordinate
        .clone();
    let displacement = Coordinate::new(
        ring_anchor_coord.x_angstrom * 1.524 / 1.397,
        ring_anchor_coord.y_angstrom * 1.524 / 1.397,
        0.0,
    );
    let methyl_carbon_coord = Coordinate::new(
        ring_anchor_coord.x_angstrom + (displacement.x_angstrom - ring_anchor_coord.x_angstrom),
        ring_anchor_coord.y_angstrom + (displacement.y_angstrom - ring_anchor_coord.y_angstrom),
        0.0,
    );
    molecule.atoms.insert(
        methyl_carbon_id,
        Atom::new(methyl_carbon_id, methyl_carbon.1, methyl_carbon_coord),
    );
    let methyl_h_base = methyl_carbon_id + 1;
    for (i, offset) in [
        (0.000, 0.000, 1.090),
        (0.000, 1.030, -0.350),
        (0.000, -1.030, -0.350),
    ]
    .iter()
    .enumerate()
    {
        let h_id = methyl_h_base + i as u32;
        let coord = Coordinate::new(
            molecule.atoms[&methyl_carbon_id].coordinate.x_angstrom + offset.0,
            molecule.atoms[&methyl_carbon_id].coordinate.y_angstrom + offset.1,
            offset.2,
        );
        molecule
            .atoms
            .insert(h_id, Atom::new(h_id, AtomicSymbol::H, coord));
        molecule
            .local_bonds
            .insert(Edge::new(methyl_carbon_id, h_id));
    }
    // Replace the H originally bonded to ring atom 1 (atom id 7 in benzene) with the methyl carbon.
    let original_anchor_h = 7_u32;
    molecule.atoms.remove(&original_anchor_h);
    molecule
        .local_bonds
        .remove(&Edge::new(ring_anchor_id, original_anchor_h));
    molecule
        .local_bonds
        .insert(Edge::new(ring_anchor_id, methyl_carbon_id));
    molecule
}

#[must_use]
pub fn bromobenzene() -> MoleculeAdt {
    let mut molecule = benzene();
    molecule.molecule_id = "MOLADT.BROMOBENZENE.001".to_string();
    molecule.name = "bromobenzene".to_string();
    molecule.projections = MoleculeProjections {
        canonical_smiles: Some("Brc1ccccc1".to_string()),
        inchi: Some("InChI=1S/C6H5Br/c7-6-4-2-1-3-5-6/h1-5H".to_string()),
        inchikey: Some("QARVLSVVCXYDNA-UHFFFAOYSA-N".to_string()),
    };
    let ring_anchor_id = 1_u32;
    let original_anchor_h = 7_u32;
    let ring_coord = molecule
        .atoms
        .get(&ring_anchor_id)
        .expect("ring anchor")
        .coordinate
        .clone();
    let bromine_id = atom_id_after(&molecule);
    let bromine_coord = Coordinate::new(
        ring_coord.x_angstrom * (1.397 + 1.910) / 1.397,
        ring_coord.y_angstrom * (1.397 + 1.910) / 1.397,
        0.0,
    );
    molecule.atoms.insert(
        bromine_id,
        Atom::new(bromine_id, AtomicSymbol::Br, bromine_coord),
    );
    molecule.atoms.remove(&original_anchor_h);
    molecule
        .local_bonds
        .remove(&Edge::new(ring_anchor_id, original_anchor_h));
    molecule
        .local_bonds
        .insert(Edge::new(ring_anchor_id, bromine_id));
    molecule
}

#[must_use]
pub fn phenylboronic_acid() -> MoleculeAdt {
    let mut molecule = benzene();
    molecule.molecule_id = "MOLADT.PHENYLBORONIC_ACID.001".to_string();
    molecule.name = "phenylboronic acid".to_string();
    molecule.projections = MoleculeProjections {
        canonical_smiles: Some("OB(O)c1ccccc1".to_string()),
        inchi: Some("InChI=1S/C6H7BO2/c8-7(9)6-4-2-1-3-5-6/h1-5,8-9H".to_string()),
        inchikey: Some("HXITXNWTGFUOAU-UHFFFAOYSA-N".to_string()),
    };
    let ring_anchor_id = 1_u32;
    let original_anchor_h = 7_u32;
    let ring_coord = molecule
        .atoms
        .get(&ring_anchor_id)
        .expect("ring anchor")
        .coordinate
        .clone();
    let boron_id = atom_id_after(&molecule);
    let boron_coord = Coordinate::new(
        ring_coord.x_angstrom * (1.397 + 1.560) / 1.397,
        ring_coord.y_angstrom * (1.397 + 1.560) / 1.397,
        0.0,
    );
    molecule.atoms.insert(
        boron_id,
        Atom::new(boron_id, AtomicSymbol::B, boron_coord.clone()),
    );
    molecule.atoms.remove(&original_anchor_h);
    molecule
        .local_bonds
        .remove(&Edge::new(ring_anchor_id, original_anchor_h));
    molecule
        .local_bonds
        .insert(Edge::new(ring_anchor_id, boron_id));
    let oxygen_left_id = atom_id_after(&molecule);
    let oxygen_left = Coordinate::new(boron_coord.x_angstrom + 1.360, boron_coord.y_angstrom, 0.0);
    molecule.atoms.insert(
        oxygen_left_id,
        Atom::new(oxygen_left_id, AtomicSymbol::O, oxygen_left.clone()),
    );
    molecule
        .local_bonds
        .insert(Edge::new(boron_id, oxygen_left_id));
    let oxygen_right_id = atom_id_after(&molecule);
    let oxygen_right = Coordinate::new(boron_coord.x_angstrom, boron_coord.y_angstrom + 1.360, 0.0);
    molecule.atoms.insert(
        oxygen_right_id,
        Atom::new(oxygen_right_id, AtomicSymbol::O, oxygen_right.clone()),
    );
    molecule
        .local_bonds
        .insert(Edge::new(boron_id, oxygen_right_id));
    let h_left_id = atom_id_after(&molecule);
    molecule.atoms.insert(
        h_left_id,
        Atom::new(
            h_left_id,
            AtomicSymbol::H,
            Coordinate::new(oxygen_left.x_angstrom + 0.960, oxygen_left.y_angstrom, 0.0),
        ),
    );
    molecule
        .local_bonds
        .insert(Edge::new(oxygen_left_id, h_left_id));
    let h_right_id = atom_id_after(&molecule);
    molecule.atoms.insert(
        h_right_id,
        Atom::new(
            h_right_id,
            AtomicSymbol::H,
            Coordinate::new(
                oxygen_right.x_angstrom,
                oxygen_right.y_angstrom + 0.960,
                0.0,
            ),
        ),
    );
    molecule
        .local_bonds
        .insert(Edge::new(oxygen_right_id, h_right_id));
    molecule
}

#[must_use]
pub fn biphenyl() -> MoleculeAdt {
    let mut atom_specs = Vec::new();
    let mut bond_specs = Vec::new();
    let radius = 1.397_f64;
    let h_radius = 2.479_f64;
    let inter_ring = 1.480_f64;
    let mut left_ring = Vec::new();
    let mut right_ring = Vec::new();
    for i in 0..6 {
        let angle = std::f64::consts::PI / 6.0 + f64::from(i) * std::f64::consts::TAU / 6.0;
        let cx_left = radius * angle.cos();
        let cy = radius * angle.sin();
        let c_left = 1 + i;
        atom_specs.push((c_left, AtomicSymbol::C, Coordinate::new(cx_left, cy, 0.0)));
        bond_specs.push((c_left, 1 + ((i + 1) % 6)));
        left_ring.push(c_left);
        if i != 0 {
            let h_left = 13 + i;
            atom_specs.push((
                h_left,
                AtomicSymbol::H,
                Coordinate::new(h_radius * angle.cos(), h_radius * angle.sin(), 0.0),
            ));
            bond_specs.push((c_left, h_left));
        }
        let cx_right = inter_ring + radius + radius * (-angle).cos();
        let cy_right = radius * (-angle).sin();
        let c_right = 7 + i;
        atom_specs.push((
            c_right,
            AtomicSymbol::C,
            Coordinate::new(cx_right, cy_right, 0.0),
        ));
        bond_specs.push((c_right, 7 + ((i + 1) % 6)));
        right_ring.push(c_right);
        if i != 0 {
            let h_right = 19 + i;
            atom_specs.push((
                h_right,
                AtomicSymbol::H,
                Coordinate::new(
                    inter_ring + radius + h_radius * (-angle).cos(),
                    h_radius * (-angle).sin(),
                    0.0,
                ),
            ));
            bond_specs.push((c_right, h_right));
        }
    }
    bond_specs.push((1, 7));
    build_molecule(
        "MOLADT.BIPHENYL.001",
        "biphenyl",
        "c1ccc(-c2ccccc2)cc1",
        Some("InChI=1S/C12H10/c1-3-7-11(8-4-1)12-9-5-2-6-10-12/h1-10H"),
        Some("ZUOUZKKEUPVFJK-UHFFFAOYSA-N"),
        &atom_specs,
        &bond_specs,
        Some(&left_ring),
    )
}

#[must_use]
fn atom_id_after(molecule: &MoleculeAdt) -> u32 {
    molecule
        .atoms
        .keys()
        .copied()
        .max()
        .map_or(1, |max| max + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_resolves_curated_smiles() {
        for smiles in [
            "O",
            "Brc1ccccc1",
            "OB(O)c1ccccc1",
            "c1ccc(-c2ccccc2)cc1",
            "Cc1ccccc1",
            "CO",
            "CCO",
            "CC(=O)O",
            "N",
            "c1ccccc1",
        ] {
            let molecule = resolve_smiles(smiles).expect("curated molecule resolves");
            molecule
                .validate()
                .unwrap_or_else(|error| panic!("library entry {smiles} validates: {error:?}"));
        }
    }

    #[test]
    fn library_flags_unsafe_smiles() {
        assert!(is_known_unsafe_for_dft("O=C([O-])[O-].[K+].[K+]"));
        assert!(is_known_unsafe_for_dft("P(c1ccccc1)(c1ccccc1)c1ccccc1.Pd"));
        assert!(!is_known_unsafe_for_dft("Cc1ccccc1"));
    }

    #[test]
    fn benzene_has_six_carbons_and_six_hydrogens() {
        let molecule = benzene();
        let carbon_count = molecule
            .atoms
            .values()
            .filter(|atom| atom.attributes.symbol == AtomicSymbol::C)
            .count();
        let hydrogen_count = molecule
            .atoms
            .values()
            .filter(|atom| atom.attributes.symbol == AtomicSymbol::H)
            .count();
        assert_eq!(carbon_count, 6);
        assert_eq!(hydrogen_count, 6);
        assert_eq!(molecule.formula(), "C6H6");
    }

    #[test]
    fn bromobenzene_has_one_bromine_and_five_hydrogens() {
        let molecule = bromobenzene();
        let bromines = molecule
            .atoms
            .values()
            .filter(|atom| atom.attributes.symbol == AtomicSymbol::Br)
            .count();
        let hydrogens = molecule
            .atoms
            .values()
            .filter(|atom| atom.attributes.symbol == AtomicSymbol::H)
            .count();
        assert_eq!(bromines, 1);
        assert_eq!(hydrogens, 5);
        assert_eq!(molecule.formula(), "C6H5Br");
    }
}
