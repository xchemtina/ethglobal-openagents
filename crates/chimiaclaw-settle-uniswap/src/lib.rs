//! Uniswap settlement adapter placeholder.

use chimiaclaw_artifact::ArtifactId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementIntent {
    pub source_artifact: ArtifactId,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: u128,
    pub recipient: String,
}

pub trait Settlement {
    fn quote(&self, intent: &SettlementIntent) -> Result<u128, String>;
    fn prepare(&self, intent: &SettlementIntent) -> Result<String, String>;
}
