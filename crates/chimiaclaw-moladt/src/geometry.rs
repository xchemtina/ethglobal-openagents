//! Deterministic geometry guesser for [`MoleculeAdt`] payloads.
//!
//! When a MolADT carries connectivity (`local_bonds`) but no trustworthy 3D
//! coordinates, [`guess_coordinates`] places every atom by a deterministic
//! breadth-first walk seeded at the smallest-id atom: each new atom is
//! positioned at the parent atom plus a unit direction times the parent-child
//! covalent-radius sum, then a small fixed number of cheap spring/repulsion
//! iterations relaxes obvious overlaps. Covalent radii are the Cordero (2008)
//! values for single bonds.
//!
//! The result is good enough for visualization, XYZ scaffolding, and "is the
//! topology sane?" sanity checks. It is **not** a substitute for an MMFF/UFF
//! geometry pass before DFT energies are trusted; the function annotates
//! [`MoleculeProvenance::source_kind`] to make that explicit.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{AtomicSymbol, Coordinate, Edge, MolAdtError, MoleculeAdt};

/// Cordero 2008 single-bond covalent radii in Angstrom.
///
/// Hand-tuned values cover the elements most common in the demo molecules.
/// Every other supported [`AtomicSymbol`] falls through to a conservative
/// 1.40 Å default, which keeps geometry guessers from crashing on
/// transition-metal or lanthanide atoms while still producing a usable bond
/// estimate. Replace the default with literature values per element as
/// chemistry demand grows.
#[must_use]
pub fn covalent_radius_angstrom(symbol: &AtomicSymbol) -> f64 {
    match symbol {
        AtomicSymbol::H => 0.31,
        AtomicSymbol::B => 0.84,
        AtomicSymbol::C => 0.76,
        AtomicSymbol::N => 0.71,
        AtomicSymbol::O => 0.66,
        AtomicSymbol::F => 0.57,
        AtomicSymbol::Na => 1.66,
        AtomicSymbol::P => 1.07,
        AtomicSymbol::S => 1.05,
        AtomicSymbol::Cl => 1.02,
        AtomicSymbol::Fe => 1.32,
        AtomicSymbol::Br => 1.20,
        AtomicSymbol::I => 1.39,
        _ => 1.40,
    }
}

/// Knobs that control the geometry guesser.
#[derive(Clone, Debug)]
pub struct GeometryGuessOptions {
    /// Number of cheap spring/repulsion relaxation iterations to run.
    pub relaxation_iterations: usize,
    /// Multiplicative step toward the target bond length each iteration.
    pub bond_step: f64,
    /// Multiplicative step away from non-bonded neighbours each iteration.
    pub repulsion_step: f64,
    /// Neighbours within this many Angstroms below the covalent contact
    /// distance contribute a soft repulsion term during relaxation.
    pub repulsion_padding: f64,
}

impl Default for GeometryGuessOptions {
    fn default() -> Self {
        Self {
            relaxation_iterations: 200,
            bond_step: 0.25,
            repulsion_step: 0.10,
            repulsion_padding: 0.30,
        }
    }
}

/// Replace the molecule's coordinates with a deterministic guess derived from
/// its connectivity (`local_bonds`). The molecule must contain at least one
/// atom; isolated atoms (with no incident bond) are placed deterministically
/// in a fan around the existing layout.
pub fn guess_coordinates(
    molecule: &mut MoleculeAdt,
    options: &GeometryGuessOptions,
) -> Result<(), MolAdtError> {
    if molecule.atoms.is_empty() {
        return Err(MolAdtError::EmptyMolecule);
    }

    let adjacency = adjacency_map(molecule);
    let mut placed: BTreeMap<u32, [f64; 3]> = BTreeMap::new();
    let root_id = *molecule
        .atoms
        .keys()
        .next()
        .expect("non-empty molecule yields a root id");
    placed.insert(root_id, [0.0, 0.0, 0.0]);

    let mut visited = BTreeSet::new();
    visited.insert(root_id);
    let mut queue = VecDeque::new();
    queue.push_back(root_id);
    while let Some(parent_id) = queue.pop_front() {
        let parent_pos = placed[&parent_id];
        let parent_symbol = molecule.atoms[&parent_id].attributes.symbol.clone();
        let parent_radius = covalent_radius_angstrom(&parent_symbol);
        let neighbours = adjacency.get(&parent_id).cloned().unwrap_or_default();
        let placed_neighbour_directions: Vec<[f64; 3]> = neighbours
            .iter()
            .filter(|other| placed.contains_key(other))
            .map(|other| unit_vector(parent_pos, placed[other]))
            .collect();
        let mut placed_dirs = placed_neighbour_directions;
        let mut new_neighbours: Vec<u32> = neighbours
            .iter()
            .copied()
            .filter(|other| !visited.contains(other))
            .collect();
        new_neighbours.sort_unstable();
        for child_id in new_neighbours {
            visited.insert(child_id);
            let child_symbol = molecule.atoms[&child_id].attributes.symbol.clone();
            let child_radius = covalent_radius_angstrom(&child_symbol);
            let bond_length = parent_radius + child_radius;
            let direction = pick_direction(&placed_dirs, child_id);
            let position = [
                parent_pos[0] + direction[0] * bond_length,
                parent_pos[1] + direction[1] * bond_length,
                parent_pos[2] + direction[2] * bond_length,
            ];
            placed.insert(child_id, position);
            placed_dirs.push(direction);
            queue.push_back(child_id);
        }
    }

    // Place any disconnected atoms in a deterministic fan offset from the
    // origin so the relaxation step has somewhere to start.
    let mut isolated_index = 0_u32;
    let isolated_radius = 4.0_f64;
    for atom_id in molecule.atoms.keys().copied().collect::<Vec<_>>() {
        if !placed.contains_key(&atom_id) {
            let angle = std::f64::consts::TAU * f64::from(isolated_index) / 12.0_f64.max(1.0_f64);
            let position = [
                isolated_radius * angle.cos(),
                isolated_radius * angle.sin(),
                0.0,
            ];
            placed.insert(atom_id, position);
            isolated_index = isolated_index.saturating_add(1);
        }
    }

    relax(&mut placed, molecule, options);

    for (atom_id, atom) in molecule.atoms.iter_mut() {
        let pos = placed[atom_id];
        atom.coordinate = Coordinate::new(pos[0], pos[1], pos[2]);
    }

    let note =
        "Coordinates are a deterministic covalent-radii BFS embedding with light spring relaxation; \
         re-optimize with MMFF/UFF or a real DFT geometry pass before trusting energies.";
    if !molecule.provenance.notes.iter().any(|line| line == note) {
        molecule.provenance.notes.push(note.to_string());
    }
    molecule.provenance.source_kind = "geometry-guess-covalent-radii".to_string();
    molecule.validate()?;
    Ok(())
}

fn adjacency_map(molecule: &MoleculeAdt) -> BTreeMap<u32, Vec<u32>> {
    let mut map: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for atom_id in molecule.atoms.keys().copied() {
        map.entry(atom_id).or_default();
    }
    for edge in &molecule.local_bonds {
        map.entry(edge.a).or_default().push(edge.b);
        map.entry(edge.b).or_default().push(edge.a);
    }
    for neighbours in map.values_mut() {
        neighbours.sort_unstable();
        neighbours.dedup();
    }
    map
}

fn unit_vector(origin: [f64; 3], target: [f64; 3]) -> [f64; 3] {
    let dx = target[0] - origin[0];
    let dy = target[1] - origin[1];
    let dz = target[2] - origin[2];
    let len = (dx * dx + dy * dy + dz * dz).sqrt();
    if len < f64::EPSILON {
        [1.0, 0.0, 0.0]
    } else {
        [dx / len, dy / len, dz / len]
    }
}

fn pick_direction(occupied: &[[f64; 3]], child_id: u32) -> [f64; 3] {
    let candidates = canonical_directions();
    let mut best = candidates[0];
    let mut best_score = f64::NEG_INFINITY;
    let phase = f64::from(child_id % 12);
    for (index, candidate) in candidates.iter().enumerate() {
        let mut min_dot = f64::INFINITY;
        for direction in occupied {
            let dot = candidate[0] * direction[0]
                + candidate[1] * direction[1]
                + candidate[2] * direction[2];
            if dot < min_dot {
                min_dot = dot;
            }
        }
        if occupied.is_empty() {
            min_dot = 0.0;
        }
        let bias = f64::from(index as u32).mul_add(0.0, phase) * 1.0e-6;
        let score = -min_dot + bias;
        if score > best_score {
            best_score = score;
            best = *candidate;
        }
    }
    best
}

fn canonical_directions() -> [[f64; 3]; 12] {
    let frac_inv_sqrt_2 = 1.0 / std::f64::consts::SQRT_2;
    [
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [frac_inv_sqrt_2, frac_inv_sqrt_2, 0.0],
        [-frac_inv_sqrt_2, frac_inv_sqrt_2, 0.0],
        [frac_inv_sqrt_2, -frac_inv_sqrt_2, 0.0],
        [-frac_inv_sqrt_2, -frac_inv_sqrt_2, 0.0],
        [frac_inv_sqrt_2, 0.0, frac_inv_sqrt_2],
        [-frac_inv_sqrt_2, 0.0, -frac_inv_sqrt_2],
    ]
}

fn relax(
    placed: &mut BTreeMap<u32, [f64; 3]>,
    molecule: &MoleculeAdt,
    options: &GeometryGuessOptions,
) {
    let bonds: Vec<(u32, u32, f64)> = molecule
        .local_bonds
        .iter()
        .map(|edge| {
            let target = covalent_radius_angstrom(&molecule.atoms[&edge.a].attributes.symbol)
                + covalent_radius_angstrom(&molecule.atoms[&edge.b].attributes.symbol);
            (edge.a, edge.b, target)
        })
        .collect();
    let bonded_pairs: BTreeSet<Edge> = molecule.local_bonds.iter().cloned().collect();
    let atom_ids: Vec<u32> = placed.keys().copied().collect();
    for _ in 0..options.relaxation_iterations {
        // Bond springs.
        for (a, b, target) in &bonds {
            let pa = placed[a];
            let pb = placed[b];
            let dx = pb[0] - pa[0];
            let dy = pb[1] - pa[1];
            let dz = pb[2] - pa[2];
            let len = (dx * dx + dy * dy + dz * dz).sqrt().max(f64::EPSILON);
            let delta = (len - target) * options.bond_step;
            let nx = dx / len;
            let ny = dy / len;
            let nz = dz / len;
            let pa_mut = placed.get_mut(a).expect("a placed");
            pa_mut[0] += nx * delta;
            pa_mut[1] += ny * delta;
            pa_mut[2] += nz * delta;
            let pb_mut = placed.get_mut(b).expect("b placed");
            pb_mut[0] -= nx * delta;
            pb_mut[1] -= ny * delta;
            pb_mut[2] -= nz * delta;
        }
        // Soft non-bonded repulsion.
        for i in 0..atom_ids.len() {
            for j in (i + 1)..atom_ids.len() {
                let id_a = atom_ids[i];
                let id_b = atom_ids[j];
                if bonded_pairs.contains(&Edge::new(id_a, id_b)) {
                    continue;
                }
                let pa = placed[&id_a];
                let pb = placed[&id_b];
                let dx = pb[0] - pa[0];
                let dy = pb[1] - pa[1];
                let dz = pb[2] - pa[2];
                let len_sq = dx * dx + dy * dy + dz * dz;
                let len = len_sq.sqrt().max(f64::EPSILON);
                let target = covalent_radius_angstrom(&molecule.atoms[&id_a].attributes.symbol)
                    + covalent_radius_angstrom(&molecule.atoms[&id_b].attributes.symbol)
                    + options.repulsion_padding;
                if len < target {
                    let push = (target - len) * options.repulsion_step;
                    let nx = dx / len;
                    let ny = dy / len;
                    let nz = dz / len;
                    let pa_mut = placed.get_mut(&id_a).expect("a placed");
                    pa_mut[0] -= nx * push;
                    pa_mut[1] -= ny * push;
                    pa_mut[2] -= nz * push;
                    let pb_mut = placed.get_mut(&id_b).expect("b placed");
                    pb_mut[0] += nx * push;
                    pb_mut[1] += ny * push;
                    pb_mut[2] += nz * push;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;

    #[test]
    fn bonded_atoms_are_close_to_target_distance() {
        let mut molecule = library::water();
        // Wipe coordinates to force the guess to do real work.
        for atom in molecule.atoms.values_mut() {
            atom.coordinate = Coordinate::new(0.0, 0.0, 0.0);
        }
        guess_coordinates(&mut molecule, &GeometryGuessOptions::default()).expect("guessed");
        for edge in &molecule.local_bonds {
            let pa = &molecule.atoms[&edge.a].coordinate;
            let pb = &molecule.atoms[&edge.b].coordinate;
            let dx = pb.x_angstrom - pa.x_angstrom;
            let dy = pb.y_angstrom - pa.y_angstrom;
            let dz = pb.z_angstrom - pa.z_angstrom;
            let length = (dx * dx + dy * dy + dz * dz).sqrt();
            let target = covalent_radius_angstrom(&molecule.atoms[&edge.a].attributes.symbol)
                + covalent_radius_angstrom(&molecule.atoms[&edge.b].attributes.symbol);
            assert!(
                (length - target).abs() < 0.10,
                "bond {edge:?} length {length:.3} far from target {target:.3}"
            );
        }
        assert_eq!(
            molecule.provenance.source_kind,
            "geometry-guess-covalent-radii"
        );
    }

    #[test]
    fn empty_molecule_errors() {
        let molecule = MoleculeAdt {
            molecule_id: "empty".into(),
            name: "empty".into(),
            atoms: BTreeMap::new(),
            local_bonds: BTreeSet::new(),
            systems: Vec::new(),
            provenance: crate::MoleculeProvenance {
                source_kind: "test".into(),
                source_ref: "test".into(),
                notes: Vec::new(),
            },
            projections: crate::MoleculeProjections {
                canonical_smiles: None,
                inchi: None,
                inchikey: None,
            },
        };
        let mut clone = molecule;
        let result = guess_coordinates(&mut clone, &GeometryGuessOptions::default());
        assert_eq!(result, Err(MolAdtError::EmptyMolecule));
    }
}
