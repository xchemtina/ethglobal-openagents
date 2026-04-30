//! ORD-like reaction to MolADT translation.
//!
//! Pulls SMILES strings out of an [`OrdLikeReaction`] and resolves them through
//! the curated `chimiaclaw_moladt::library`. Substrates whose SMILES are not
//! yet in the library, or whose SMILES are flagged as unsafe to silently
//! interpret (multi-component salts, transition metal complexes, charged
//! cations), are recorded as skipped entries instead of being silently
//! mis-translated.

use std::collections::BTreeSet;

use chimiaclaw_moladt::{library, MoleculeAdt};
use serde::{Deserialize, Serialize};

use crate::{OrdInput, OrdLikeReaction};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrdSubstrateRole {
    Reactant,
    Auxiliary,
    Product,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SkipReason {
    NotInLibrary,
    UnsafeForDirectDft,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResolvedSubstrate {
    pub label: String,
    pub smiles: String,
    pub role: OrdSubstrateRole,
    pub roles_in_reaction: BTreeSet<String>,
    pub molecule: MoleculeAdt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkippedSubstrate {
    pub label: String,
    pub smiles: String,
    pub role: OrdSubstrateRole,
    pub roles_in_reaction: BTreeSet<String>,
    pub reason: SkipReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrdMoladtTranslation {
    pub reaction_name: String,
    pub resolved: Vec<ResolvedSubstrate>,
    pub skipped: Vec<SkippedSubstrate>,
}

impl OrdMoladtTranslation {
    #[must_use]
    pub fn dft_ready(&self) -> bool {
        !self.resolved.is_empty()
    }

    #[must_use]
    pub fn unique_resolved_smiles(&self) -> Vec<&str> {
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        for entry in &self.resolved {
            if seen.insert(entry.smiles.as_str()) {
                order.push(entry.smiles.as_str());
            }
        }
        order
    }
}

/// Translate every input/auxiliary/product substrate in `ord` to a curated
/// `MoleculeAdt`, returning resolved entries plus an explicit list of skips.
#[must_use]
pub fn translate_reaction(ord: &OrdLikeReaction) -> OrdMoladtTranslation {
    let mut resolved = Vec::new();
    let mut skipped = Vec::new();
    for input in &ord.inputs {
        push_entry(
            input,
            OrdSubstrateRole::Reactant,
            &mut resolved,
            &mut skipped,
        );
    }
    for auxiliary in &ord.auxiliary_samples {
        push_entry(
            auxiliary,
            OrdSubstrateRole::Auxiliary,
            &mut resolved,
            &mut skipped,
        );
    }
    for product in &ord.products {
        push_entry(
            product,
            OrdSubstrateRole::Product,
            &mut resolved,
            &mut skipped,
        );
    }
    OrdMoladtTranslation {
        reaction_name: ord.name.clone(),
        resolved,
        skipped,
    }
}

fn push_entry(
    input: &OrdInput,
    role: OrdSubstrateRole,
    resolved: &mut Vec<ResolvedSubstrate>,
    skipped: &mut Vec<SkippedSubstrate>,
) {
    let mut roles_in_reaction = BTreeSet::new();
    if let Some(role_label) = input.role.clone() {
        roles_in_reaction.insert(role_label);
    }
    if library::is_known_unsafe_for_dft(&input.smiles) {
        skipped.push(SkippedSubstrate {
            label: input.label.clone(),
            smiles: input.smiles.clone(),
            role,
            roles_in_reaction,
            reason: SkipReason::UnsafeForDirectDft,
        });
        return;
    }
    match library::resolve_smiles(&input.smiles) {
        Some(molecule) => resolved.push(ResolvedSubstrate {
            label: input.label.clone(),
            smiles: input.smiles.clone(),
            role,
            roles_in_reaction,
            molecule,
        }),
        None => skipped.push(SkippedSubstrate {
            label: input.label.clone(),
            smiles: input.smiles.clone(),
            role,
            roles_in_reaction,
            reason: SkipReason::NotInLibrary,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo_suzuki_ord_like;

    #[test]
    fn demo_suzuki_translates_open_substrates_and_skips_metal_catalyst() {
        let ord = demo_suzuki_ord_like();
        let translation = translate_reaction(&ord);
        assert_eq!(translation.reaction_name, ord.name);
        assert!(translation.dft_ready());
        let resolved_smiles = translation.unique_resolved_smiles();
        assert!(resolved_smiles.contains(&"Brc1ccccc1"));
        assert!(resolved_smiles.contains(&"Cc1ccccc1"));
        assert!(translation
            .skipped
            .iter()
            .any(|entry| entry.reason == SkipReason::UnsafeForDirectDft
                && entry.smiles.contains(".Pd")));
        assert!(translation
            .skipped
            .iter()
            .any(|entry| entry.reason == SkipReason::UnsafeForDirectDft
                && entry.smiles.contains("[K+]")));
    }
}
