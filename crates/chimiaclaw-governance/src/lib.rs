//! Governance artifacts and read-only governor primitives.

use chimiaclaw_artifact::ArtifactId;
use chimiaclaw_reputation::ReputationDomain;
use chimiaclaw_schema::AgentId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalKind {
    ParameterChange,
    TreasurySpend,
    SkillRegistryUpdate,
    ContractUpgrade,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalArtifact {
    pub artifact_id: ArtifactId,
    pub proposer: AgentId,
    pub kind: ProposalKind,
    pub cid: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VoteArtifact {
    pub artifact_id: ArtifactId,
    pub voter: AgentId,
    pub proposal: ArtifactId,
    pub domain: ReputationDomain,
    pub weight: f64,
    pub support: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionArtifact {
    pub artifact_id: ArtifactId,
    pub proposal: ArtifactId,
    pub target: String,
    pub calldata_hash: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadOnlyTally {
    pub support_weight: f64,
    pub against_weight: f64,
    pub quorum: f64,
}

impl ReadOnlyTally {
    #[must_use]
    pub fn passes(&self) -> bool {
        self.support_weight >= self.quorum && self.support_weight > self.against_weight
    }
}
