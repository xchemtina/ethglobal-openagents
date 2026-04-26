//! DFT daemon reference swarm profiles.

pub const SUBMITTER_AGENT: &str = "submitter.dft.eth";
pub const WORKER_AGENT_PREFIX: &str = "worker";
pub const MINTER_AGENT: &str = "minter.dft.eth";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DftJobRequest {
    pub smiles: String,
    pub level_of_theory: String,
    pub basis_set: String,
}
