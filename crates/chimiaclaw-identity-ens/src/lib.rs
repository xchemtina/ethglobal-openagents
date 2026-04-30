//! ENS identity adapter.
//!
//! The default build stays offline and dependency-light. Compile with
//! `--features live` to enable read-only ENS JSON-RPC resolution.

use chimiaclaw_artifact::{Artifact, ArtifactDraft, ArtifactError, ArtifactSigner, PayloadRef};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId, StrategySetId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

pub const ENS_RESOLUTION_TAG: &str = "identity.ens.resolution";
pub const ENS_VERIFICATION_TAG: &str = "identity.ens.verification";
pub const ENS_PUBLICATION_TAG: &str = "identity.ens.publication";
pub const ENS_RESOLUTION_SKILL: &str = "identity.ens.resolution.v1";
pub const ENS_VERIFICATION_SKILL: &str = "identity.ens.verification.v1";
pub const ENS_PUBLICATION_SKILL: &str = "identity.ens.publication.v1";
pub const ENS_PUBLISHER_COMMAND_ENV: &str = "CHIMIACLAW_ENS_PUBLISH_COMMAND";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StaticIdentityResolver {
    records: BTreeMap<AgentId, EnsAgentRecord>,
}

impl StaticIdentityResolver {
    #[must_use]
    pub fn new(records: impl IntoIterator<Item = EnsAgentRecord>) -> Self {
        Self {
            records: records
                .into_iter()
                .map(|record| (record.agent.clone(), record))
                .collect(),
        }
    }
}

impl IdentityResolver for StaticIdentityResolver {
    fn resolve(&self, agent: &AgentId) -> Option<EnsAgentRecord> {
        self.records.get(agent).cloned()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsResolvedTextRecord {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsResolution {
    pub agent: AgentId,
    pub ens_name: String,
    pub registry_address: String,
    pub resolver_address: Option<String>,
    pub resolved_address: Option<String>,
    pub text_records: Vec<EnsResolvedTextRecord>,
    pub resolved_at_unix: u64,
}

impl EnsResolution {
    #[must_use]
    pub fn text_record(&self, key: &str) -> Option<&str> {
        self.text_records
            .iter()
            .find(|record| record.key == key)
            .and_then(|record| record.value.as_deref())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsVerificationExpectation {
    pub agent: AgentId,
    pub ens_name: String,
    pub expected_address: Option<String>,
    pub expected_axl_peer_id: Option<String>,
    pub expected_head_artifact_cid: Option<String>,
    pub required_text_records: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsVerificationReport {
    pub agent: AgentId,
    pub ens_name: String,
    pub verified: bool,
    pub mismatches: Vec<String>,
    pub resolution: EnsResolution,
}

#[derive(Debug)]
pub enum EnsError {
    MissingEnv(String),
    Http(String),
    Rpc(String),
    Abi(String),
    Artifact(ArtifactError),
}

impl Display for EnsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnv(name) => write!(f, "missing required environment variable {name}"),
            Self::Http(error) => write!(f, "http error: {error}"),
            Self::Rpc(error) => write!(f, "rpc error: {error}"),
            Self::Abi(error) => write!(f, "abi error: {error}"),
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
        }
    }
}

impl std::error::Error for EnsError {}

impl From<ArtifactError> for EnsError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

#[must_use]
pub fn verify_resolution(
    expectation: &EnsVerificationExpectation,
    resolution: &EnsResolution,
) -> EnsVerificationReport {
    let mut mismatches = Vec::new();
    if expectation.agent != resolution.agent {
        mismatches.push(format!(
            "agent mismatch: expected {}, got {}",
            expectation.agent.0, resolution.agent.0
        ));
    }
    if expectation.ens_name != resolution.ens_name {
        mismatches.push(format!(
            "ens_name mismatch: expected {}, got {}",
            expectation.ens_name, resolution.ens_name
        ));
    }
    if let Some(expected_address) = &expectation.expected_address {
        let actual = resolution
            .resolved_address
            .as_deref()
            .unwrap_or("<missing>");
        if !eq_address(expected_address, actual) {
            mismatches.push(format!(
                "address mismatch: expected {expected_address}, got {actual}"
            ));
        }
    }
    if let Some(expected_axl_peer_id) = &expectation.expected_axl_peer_id {
        let actual = resolution
            .text_record("chimiaclaw.axl.peer_id")
            .unwrap_or("<missing>");
        if actual != expected_axl_peer_id {
            mismatches.push(format!(
                "chimiaclaw.axl.peer_id mismatch: expected {expected_axl_peer_id}, got {actual}"
            ));
        }
    }
    if let Some(expected_head_artifact_cid) = &expectation.expected_head_artifact_cid {
        let actual = resolution
            .text_record("chimiaclaw.head_artifact.cid")
            .unwrap_or("<missing>");
        if actual != expected_head_artifact_cid {
            mismatches.push(format!(
                "chimiaclaw.head_artifact.cid mismatch: expected {expected_head_artifact_cid}, got {actual}"
            ));
        }
    }
    for (key, expected) in &expectation.required_text_records {
        let actual = resolution.text_record(key).unwrap_or("<missing>");
        if actual != expected {
            mismatches.push(format!("{key} mismatch: expected {expected}, got {actual}"));
        }
    }
    EnsVerificationReport {
        agent: expectation.agent.clone(),
        ens_name: expectation.ens_name.clone(),
        verified: mismatches.is_empty(),
        mismatches,
        resolution: resolution.clone(),
    }
}

pub fn resolution_artifact(
    resolution: &EnsResolution,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, EnsError> {
    ArtifactDraft {
        skill: SkillId(ENS_RESOLUTION_SKILL.to_string()),
        agent: resolution.agent.clone(),
        topic: format!("ENS resolution for {}", resolution.ens_name),
        input_fingerprint: format!(
            "ens:{}@{}",
            resolution.ens_name, resolution.registry_address
        ),
        output_cid: None,
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(ENS_RESOLUTION_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(resolution)?),
    }
    .seal(signer, created_at_unix)
    .map_err(EnsError::Artifact)
}

/// Single text-record outcome reported by the ENS publisher worker.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsPublishedTextRecord {
    pub key: String,
    pub value: String,
    pub status: EnsPublishStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_gas_price: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnsPublishStatus {
    Published,
    Unchanged,
    DryRun,
    Failed,
}

/// Full output from the `ens-publish-text-records` worker.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsPublication {
    pub schema_tag: String,
    pub ens_name: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub controller_address: String,
    pub registry_owner: String,
    pub records: Vec<EnsPublishedTextRecord>,
    pub started_at_unix: u64,
    pub completed_at_unix: u64,
    pub dry_run: bool,
    pub provenance: EnsPublicationProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnsPublicationProvenance {
    pub source_kind: String,
    pub source_ref: String,
    pub notes: Vec<String>,
}

impl EnsPublication {
    /// True iff every record landed at its desired value (published or already
    /// matching) without any failure.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        !self.records.is_empty()
            && self.records.iter().all(|record| {
                matches!(
                    record.status,
                    EnsPublishStatus::Published | EnsPublishStatus::Unchanged
                )
            })
    }

    /// Build an [`EnsVerificationExpectation`] from the records this publication
    /// committed to, so the caller can hand it straight to
    /// [`verify_resolution`] after running [`crate::live::LiveEnsResolver`].
    #[must_use]
    pub fn verification_expectation(&self, agent: AgentId) -> EnsVerificationExpectation {
        EnsVerificationExpectation {
            agent,
            ens_name: self.ens_name.clone(),
            expected_address: None,
            expected_axl_peer_id: self
                .records
                .iter()
                .find(|record| record.key == "chimiaclaw.axl.peer_id")
                .map(|record| record.value.clone()),
            expected_head_artifact_cid: self
                .records
                .iter()
                .find(|record| record.key == "chimiaclaw.head_artifact.cid")
                .map(|record| record.value.clone()),
            required_text_records: self
                .records
                .iter()
                .map(|record| (record.key.clone(), record.value.clone()))
                .collect(),
        }
    }
}

/// Sign the publisher's output as a `identity.ens.publication` artifact.
pub fn publication_artifact(
    publication: &EnsPublication,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, EnsError> {
    if publication.schema_tag != ENS_PUBLICATION_TAG {
        return Err(EnsError::Rpc(format!(
            "worker emitted schema_tag={:?}, expected {:?}",
            publication.schema_tag, ENS_PUBLICATION_TAG
        )));
    }
    ArtifactDraft {
        skill: SkillId(ENS_PUBLICATION_SKILL.to_string()),
        agent,
        topic: format!(
            "ENS text-record publication for {} ({} records)",
            publication.ens_name,
            publication.records.len()
        ),
        input_fingerprint: format!(
            "ens-publication:{}@{}:{}",
            publication.ens_name,
            publication.chain_id,
            publication.records.len()
        ),
        output_cid: None,
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(ENS_PUBLICATION_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(publication)?),
    }
    .seal(signer, created_at_unix)
    .map_err(EnsError::Artifact)
}

pub fn verification_artifact(
    report: &EnsVerificationReport,
    signer: &ArtifactSigner,
    parent_resolution_artifact: Option<chimiaclaw_artifact::ArtifactId>,
    created_at_unix: u64,
) -> Result<Artifact, EnsError> {
    let parent_artifact_ids = parent_resolution_artifact.into_iter().collect();
    ArtifactDraft {
        skill: SkillId(ENS_VERIFICATION_SKILL.to_string()),
        agent: report.agent.clone(),
        topic: format!("ENS verification for {}", report.ens_name),
        input_fingerprint: format!("ens-verification:{}", report.ens_name),
        output_cid: None,
        parent_artifact_ids,
        schema_tags: BTreeSet::from([SchemaTag(ENS_VERIFICATION_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(report)?),
    }
    .seal(signer, created_at_unix)
    .map_err(EnsError::Artifact)
}

fn eq_address(a: &str, b: &str) -> bool {
    a.trim_start_matches("0x")
        .eq_ignore_ascii_case(b.trim_start_matches("0x"))
}

#[cfg(feature = "live")]
mod live {
    use super::{
        EnsError, EnsPublication, EnsResolution, EnsResolvedTextRecord, ENS_PUBLISHER_COMMAND_ENV,
    };
    use chimiaclaw_schema::AgentId;
    use reqwest::blocking::Client;
    use serde_json::Value;
    use sha3::{Digest, Keccak256};
    use std::ffi::OsStr;
    use std::process::{Command, Stdio};

    const MAINNET_ENS_REGISTRY: &str = "0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e";
    const SELECTOR_RESOLVER: &str = "0178b8bf";
    const SELECTOR_ADDR: &str = "3b3b57de";
    const SELECTOR_TEXT: &str = "59d1d43c";

    #[derive(Clone, Debug)]
    pub struct LiveEnsResolver {
        rpc_url: String,
        registry_address: String,
        client: Client,
    }

    impl LiveEnsResolver {
        #[must_use]
        pub fn new(rpc_url: impl Into<String>) -> Self {
            Self::with_registry(rpc_url, MAINNET_ENS_REGISTRY)
        }

        #[must_use]
        pub fn with_registry(
            rpc_url: impl Into<String>,
            registry_address: impl Into<String>,
        ) -> Self {
            Self {
                rpc_url: rpc_url.into(),
                registry_address: registry_address.into(),
                client: Client::new(),
            }
        }

        pub fn from_env() -> Result<Self, EnsError> {
            let rpc_url = std::env::var("ENS_RPC_URL")
                .map_err(|_| EnsError::MissingEnv("ENS_RPC_URL".to_string()))?;
            Ok(Self::new(rpc_url))
        }

        pub fn resolve(
            &self,
            agent: AgentId,
            ens_name: &str,
            text_keys: &[String],
            resolved_at_unix: u64,
        ) -> Result<EnsResolution, EnsError> {
            let node = namehash(ens_name);
            let resolver_data = format!("0x{SELECTOR_RESOLVER}{}", hex::encode(node));
            let resolver_raw = self.eth_call(&self.registry_address, &resolver_data)?;
            let resolver_address = decode_address_word(&resolver_raw)?;
            let (resolved_address, text_records) = if let Some(resolver) = resolver_address.as_ref()
            {
                let addr_data = format!("0x{SELECTOR_ADDR}{}", hex::encode(node));
                let resolved_address = decode_address_word(&self.eth_call(resolver, &addr_data)?)?;
                let mut records = Vec::with_capacity(text_keys.len());
                for key in text_keys {
                    let text_data = encode_text_call(&node, key);
                    let value = decode_string_return(&self.eth_call(resolver, &text_data)?)?;
                    records.push(EnsResolvedTextRecord {
                        key: key.clone(),
                        value: if value.is_empty() { None } else { Some(value) },
                    });
                }
                (resolved_address, records)
            } else {
                (
                    None,
                    text_keys
                        .iter()
                        .map(|key| EnsResolvedTextRecord {
                            key: key.clone(),
                            value: None,
                        })
                        .collect(),
                )
            };

            Ok(EnsResolution {
                agent,
                ens_name: ens_name.to_string(),
                registry_address: self.registry_address.clone(),
                resolver_address,
                resolved_address,
                text_records,
                resolved_at_unix,
            })
        }

        fn eth_call(&self, to: &str, data: &str) -> Result<String, EnsError> {
            let request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "eth_call",
                "params": [
                    {
                        "to": to,
                        "data": data,
                    },
                    "latest"
                ]
            });
            let response: Value = self
                .client
                .post(&self.rpc_url)
                .json(&request)
                .send()
                .map_err(|error| EnsError::Http(error.to_string()))?
                .json()
                .map_err(|error| EnsError::Http(error.to_string()))?;
            if let Some(error) = response.get("error") {
                return Err(EnsError::Rpc(error.to_string()));
            }
            response
                .get("result")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .ok_or_else(|| EnsError::Rpc(format!("missing result in response {response}")))
        }
    }

    /// Configuration for invoking the external ENS publisher worker.
    /// command line is read from the `CHIMIACLAW_ENS_PUBLISH_COMMAND`
    /// environment variable; the worker reads the private key from the
    /// environment as well so it never appears in process arguments.
    #[derive(Clone, Debug)]
    pub struct EnsPublisherCommandConfig {
        program: String,
        args: Vec<String>,
    }

    impl EnsPublisherCommandConfig {
        pub fn from_env() -> Result<Self, EnsError> {
            let raw = std::env::var(ENS_PUBLISHER_COMMAND_ENV)
                .map_err(|_| EnsError::MissingEnv(ENS_PUBLISHER_COMMAND_ENV.to_string()))?;
            let mut tokens = raw.split_whitespace();
            let program = tokens
                .next()
                .ok_or_else(|| EnsError::MissingEnv(ENS_PUBLISHER_COMMAND_ENV.to_string()))?;
            Ok(Self {
                program: program.to_string(),
                args: tokens.map(str::to_string).collect(),
            })
        }

        pub fn invoke<S: AsRef<OsStr>>(
            &self,
            ens_name: S,
            records: &[(String, String)],
            dry_run: bool,
            allow_mainnet: bool,
        ) -> Result<EnsPublication, EnsError> {
            let mut command = Command::new(&self.program);
            command.args(&self.args).arg("--ens").arg(ens_name.as_ref());
            for (key, value) in records {
                command.arg("--record").arg(format!("{key}={value}"));
            }
            if dry_run {
                command.arg("--dry-run");
            }
            if allow_mainnet {
                command.arg("--allow-mainnet");
            }
            let mut child = command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| EnsError::Http(format!("spawn ens publisher: {error}")))?;
            // Drop stdin explicitly so the worker doesn't block on it.
            drop(child.stdin.take());
            let output = child
                .wait_with_output()
                .map_err(|error| EnsError::Http(format!("wait ens publisher: {error}")))?;
            if !output.status.success() {
                return Err(EnsError::Rpc(format!(
                    "ens publisher exited with status {:?}: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            let stdout = String::from_utf8(output.stdout)
                .map_err(|error| EnsError::Abi(format!("worker stdout not utf-8: {error}")))?;
            let publication: EnsPublication = serde_json::from_str(stdout.trim())
                .map_err(|error| EnsError::Abi(format!("worker stdout not valid JSON: {error}")))?;
            Ok(publication)
        }
    }

    pub fn default_text_keys() -> Vec<String> {
        [
            "chimiaclaw.profile.cid",
            "chimiaclaw.capabilities",
            "chimiaclaw.settlement.endpoint",
            "chimiaclaw.axl.peer_id",
            "chimiaclaw.head_artifact.cid",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    fn namehash(name: &str) -> [u8; 32] {
        let mut node = [0u8; 32];
        if name.is_empty() {
            return node;
        }
        for label in name
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .split('.')
            .rev()
        {
            let label_hash = Keccak256::digest(label.as_bytes());
            let mut bytes = [0u8; 64];
            bytes[..32].copy_from_slice(&node);
            bytes[32..].copy_from_slice(&label_hash);
            node.copy_from_slice(&Keccak256::digest(bytes));
        }
        node
    }

    fn decode_address_word(raw: &str) -> Result<Option<String>, EnsError> {
        let bytes = decode_hex(raw)?;
        if bytes.len() < 32 {
            return Err(EnsError::Abi(format!(
                "expected at least 32 bytes for address word, got {}",
                bytes.len()
            )));
        }
        let word = &bytes[bytes.len() - 32..];
        let address = &word[12..32];
        if address.iter().all(|byte| *byte == 0) {
            Ok(None)
        } else {
            Ok(Some(format!("0x{}", hex::encode(address))))
        }
    }

    fn encode_text_call(node: &[u8; 32], key: &str) -> String {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&hex::decode(SELECTOR_TEXT).expect("selector hex"));
        bytes.extend_from_slice(node);
        bytes.extend_from_slice(&word_from_usize(64));
        bytes.extend_from_slice(&word_from_usize(key.len()));
        bytes.extend_from_slice(key.as_bytes());
        while bytes.len() % 32 != 0 {
            bytes.push(0);
        }
        format!("0x{}", hex::encode(bytes))
    }

    fn decode_string_return(raw: &str) -> Result<String, EnsError> {
        let bytes = decode_hex(raw)?;
        if bytes.is_empty() {
            return Ok(String::new());
        }
        if bytes.len() < 64 {
            return Err(EnsError::Abi(format!(
                "expected dynamic string return, got {} bytes",
                bytes.len()
            )));
        }
        let offset = usize_from_word(&bytes[..32])?;
        if bytes.len() < offset + 32 {
            return Err(EnsError::Abi(
                "string offset exceeds return data".to_string(),
            ));
        }
        let len = usize_from_word(&bytes[offset..offset + 32])?;
        let start = offset + 32;
        let end = start + len;
        if bytes.len() < end {
            return Err(EnsError::Abi(
                "string length exceeds return data".to_string(),
            ));
        }
        String::from_utf8(bytes[start..end].to_vec())
            .map_err(|error| EnsError::Abi(error.to_string()))
    }

    fn decode_hex(raw: &str) -> Result<Vec<u8>, EnsError> {
        hex::decode(raw.trim_start_matches("0x")).map_err(|error| EnsError::Abi(error.to_string()))
    }

    fn word_from_usize(value: usize) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[24..32].copy_from_slice(&(value as u64).to_be_bytes());
        word
    }

    fn usize_from_word(word: &[u8]) -> Result<usize, EnsError> {
        if word.len() != 32 {
            return Err(EnsError::Abi(format!(
                "expected 32-byte word, got {}",
                word.len()
            )));
        }
        if word[..24].iter().any(|byte| *byte != 0) {
            return Err(EnsError::Abi(
                "cannot decode ABI word larger than u64".to_string(),
            ));
        }
        let mut value = [0u8; 8];
        value.copy_from_slice(&word[24..32]);
        Ok(u64::from_be_bytes(value) as usize)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn ens_namehash_matches_known_eth_root() {
            assert_eq!(
                hex::encode(namehash("eth")),
                "93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae"
            );
        }

        #[test]
        fn text_call_starts_with_expected_selector() {
            let call = encode_text_call(&[0u8; 32], "chimiaclaw.profile.cid");
            assert!(call.starts_with("0x59d1d43c"));
        }
    }
}

#[cfg(feature = "live")]
pub use live::{default_text_keys, EnsPublisherCommandConfig, LiveEnsResolver};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_publication() -> EnsPublication {
        EnsPublication {
            schema_tag: ENS_PUBLICATION_TAG.to_string(),
            ens_name: "dft.service.chimiadao.eth".to_string(),
            chain_id: 11155111, // Sepolia
            rpc_url: "https://YOUR_SEPOLIA_RPC".to_string(),
            controller_address: "0x0000000000000000000000000000000000000abc".to_string(),
            registry_owner: "0x0000000000000000000000000000000000000abc".to_string(),
            records: vec![
                EnsPublishedTextRecord {
                    key: "chimiaclaw.profile.cid".to_string(),
                    value: "zg://demo-profile-root".to_string(),
                    status: EnsPublishStatus::Published,
                    previous_value: Some(String::new()),
                    tx_hash: Some("0xabc".to_string()),
                    block_number: Some(123_456),
                    gas_used: Some(48_000),
                    effective_gas_price: Some(1_000_000_000),
                    error: None,
                },
                EnsPublishedTextRecord {
                    key: "chimiaclaw.axl.peer_id".to_string(),
                    value: "axl-dft-demo-peer".to_string(),
                    status: EnsPublishStatus::Unchanged,
                    previous_value: Some("axl-dft-demo-peer".to_string()),
                    tx_hash: None,
                    block_number: None,
                    gas_used: None,
                    effective_gas_price: None,
                    error: None,
                },
            ],
            started_at_unix: 100,
            completed_at_unix: 110,
            dry_run: false,
            provenance: EnsPublicationProvenance {
                source_kind: "ens-publisher-setText".to_string(),
                source_ref: "test-fixture".to_string(),
                notes: vec!["controller: 0x0...abc".to_string()],
            },
        }
    }

    #[test]
    fn publication_artifact_is_payload_bound_and_consistent() {
        let publication = sample_publication();
        assert!(publication.is_consistent());
        let signer = ArtifactSigner::from_seed([91; 32]);
        let agent = AgentId("dft.service.chimiadao.eth".to_string());
        let artifact =
            publication_artifact(&publication, agent, &signer, 200).expect("signed publication");
        artifact.verify().expect("artifact verifies");
        artifact
            .verify_payload_value(&publication)
            .expect("payload binding holds");
        assert!(artifact
            .schema_tags
            .contains(&SchemaTag(ENS_PUBLICATION_TAG.to_string())));
    }

    #[test]
    fn publication_drives_verification_expectation() {
        let publication = sample_publication();
        let agent = AgentId("dft.service.chimiadao.eth".to_string());
        let expectation = publication.verification_expectation(agent.clone());
        assert_eq!(
            expectation.expected_axl_peer_id.as_deref(),
            Some("axl-dft-demo-peer")
        );
        assert_eq!(expectation.required_text_records.len(), 2);
    }

    #[test]
    fn worker_json_round_trips_through_ens_publication() {
        let raw = r#"{
            "schema_tag": "identity.ens.publication",
            "ens_name": "dft.service.chimiadao.eth",
            "chain_id": 11155111,
            "rpc_url": "https://YOUR_SEPOLIA_RPC",
            "controller_address": "0xabc",
            "registry_owner": "0xabc",
            "records": [
                {"key": "chimiaclaw.profile.cid", "value": "zg://demo", "status": "published", "previous_value": "", "tx_hash": "0x1", "block_number": 1, "gas_used": 48000, "effective_gas_price": 1000000000}
            ],
            "started_at_unix": 1,
            "completed_at_unix": 2,
            "dry_run": false,
            "provenance": {"source_kind": "ens-publisher-setText", "source_ref": "ref", "notes": []}
        }"#;
        let parsed: EnsPublication = serde_json::from_str(raw).expect("parses worker output");
        assert_eq!(parsed.records.len(), 1);
        assert!(matches!(
            parsed.records[0].status,
            EnsPublishStatus::Published
        ));
    }

    #[test]
    fn empty_records_publication_is_not_consistent() {
        let mut publication = sample_publication();
        publication.records.clear();
        assert!(!publication.is_consistent());
    }

    #[test]
    fn verification_reports_missing_text_record() {
        let expectation = EnsVerificationExpectation {
            agent: AgentId("dft.service.chimiaclaw.eth".to_string()),
            ens_name: "dft.service.chimiaclaw.eth".to_string(),
            expected_address: None,
            expected_axl_peer_id: None,
            expected_head_artifact_cid: None,
            required_text_records: BTreeMap::from([(
                "chimiaclaw.profile.cid".to_string(),
                "zg://catalog/demo".to_string(),
            )]),
        };
        let resolution = EnsResolution {
            agent: expectation.agent.clone(),
            ens_name: expectation.ens_name.clone(),
            registry_address: "0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e".to_string(),
            resolver_address: None,
            resolved_address: None,
            text_records: Vec::new(),
            resolved_at_unix: 1,
        };
        let report = verify_resolution(&expectation, &resolution);
        assert!(!report.verified);
        assert_eq!(report.mismatches.len(), 1);
    }
}
