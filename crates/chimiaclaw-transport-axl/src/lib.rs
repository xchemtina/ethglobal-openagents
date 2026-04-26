//! Gensyn AXL transport placeholder.

use chimiaclaw_schema::{AgentId, SchemaTag};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AxlPeer {
    pub agent: AgentId,
    pub peer_id: String,
    pub localhost_endpoint: String,
}

pub trait Transport {
    fn subscribe(&self, tag: &SchemaTag) -> Result<(), String>;
    fn publish(&self, tag: &SchemaTag, payload: &[u8]) -> Result<(), String>;
}

#[derive(Clone, Debug, Default)]
pub struct MockAxlTransport;

impl Transport for MockAxlTransport {
    fn subscribe(&self, _tag: &SchemaTag) -> Result<(), String> {
        Ok(())
    }
    fn publish(&self, _tag: &SchemaTag, _payload: &[u8]) -> Result<(), String> {
        Ok(())
    }
}
