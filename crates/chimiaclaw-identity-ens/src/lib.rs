//! ENS identity adapter placeholder.

use chimiaclaw_schema::{AgentId, StrategySetId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnsAgentRecord {
    pub agent: AgentId,
    pub address: String,
    pub axl_peer_id: String,
    pub head_artifact_cid: Option<String>,
    pub active_strategy_sets: Vec<StrategySetId>,
}

pub trait IdentityResolver {
    fn resolve(&self, agent: &AgentId) -> Option<EnsAgentRecord>;
}
