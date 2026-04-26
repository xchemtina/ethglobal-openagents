//! Optimization abstractions for MSSP, switchers, and future active-learning methods.

use chimiaclaw_artifact::{Artifact, ArtifactId};
use chimiaclaw_schema::{StrategySet, StrategySetId};

pub trait Population {
    fn members(&self) -> &[ArtifactId];
}

pub trait Fitness {
    fn evaluate(&self, artifact: &Artifact) -> f64;
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateSpec {
    pub parent_m: ArtifactId,
    pub parent_f: ArtifactId,
    pub dominance_z: f64,
    pub note: String,
}

pub trait Crossover {
    fn cross(&self, m: &Artifact, f: &Artifact, z: f64) -> CandidateSpec;
}

pub trait Tournament {
    fn select(&self, population: &dyn Population, k: usize) -> Vec<ArtifactId>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeCtx {
    pub node_id: String,
    pub active_constraints: Vec<String>,
}

pub trait Switcher {
    fn elect(&self, sets: &[StrategySet], ctx: &NodeCtx) -> Option<StrategySetId>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryPopulation {
    pub artifacts: Vec<ArtifactId>,
}

impl Population for InMemoryPopulation {
    fn members(&self) -> &[ArtifactId] {
        &self.artifacts
    }
}

#[derive(Default)]
pub struct FirstKSelection;

impl Tournament for FirstKSelection {
    fn select(&self, population: &dyn Population, k: usize) -> Vec<ArtifactId> {
        population.members().iter().take(k).cloned().collect()
    }
}

#[derive(Default)]
pub struct DominanceCrossover;

impl Crossover for DominanceCrossover {
    fn cross(&self, m: &Artifact, f: &Artifact, z: f64) -> CandidateSpec {
        CandidateSpec {
            parent_m: m.id.clone(),
            parent_f: f.id.clone(),
            dominance_z: z,
            note: "Phase 0 deterministic placeholder; payload blending is schema-specific"
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_k_selection_is_deterministic() {
        let population = InMemoryPopulation {
            artifacts: vec![ArtifactId("a".into()), ArtifactId("b".into())],
        };
        let selected = FirstKSelection.select(&population, 1);
        assert_eq!(selected, vec![ArtifactId("a".into())]);
    }
}
