//! ERC-7857 iNFT capability token placeholder.

use chimiaclaw_schema::{AgentId, CapabilityDescriptor};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct INftPublicMetadata {
    pub token_id: u64,
    pub agent: AgentId,
    pub capability_fingerprint: String,
    pub head_artifact_cid: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedAgentState {
    pub ciphertext_cid: String,
    pub attestation: String,
}

pub trait CapabilityToken {
    fn mint(&self, descriptor: &CapabilityDescriptor) -> Result<INftPublicMetadata, String>;
    fn encrypted_state(&self, token_id: u64) -> Result<Option<EncryptedAgentState>, String>;
}
