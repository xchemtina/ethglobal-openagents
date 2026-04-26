//! DAG conflict resolution and pruning hooks.

use chimiaclaw_artifact::{Artifact, ArtifactId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationDecision {
    Keep(ArtifactId),
    Prune(ArtifactId),
    NeedsReview(Vec<ArtifactId>),
}

pub trait ArtifactMutator {
    fn decide(&self, conflicting: &[Artifact]) -> MutationDecision;
}

#[derive(Default)]
pub struct ConservativeMutator;

impl ArtifactMutator for ConservativeMutator {
    fn decide(&self, conflicting: &[Artifact]) -> MutationDecision {
        match conflicting {
            [] => MutationDecision::NeedsReview(Vec::new()),
            [artifact] => MutationDecision::Keep(artifact.id.clone()),
            many => MutationDecision::NeedsReview(
                many.iter().map(|artifact| artifact.id.clone()).collect(),
            ),
        }
    }
}
