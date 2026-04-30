//! 0G Storage adapter.
//!
//! The default artifact store remains a local in-memory mock. Compile with
//! `--features live` to enable an operator-provided 0G upload command wrapper.
//! The wrapper receives non-secret metadata on stdin and reads secrets from
//! environment variables, avoiding private keys in process arguments.

use chimiaclaw_artifact::{
    Artifact, ArtifactDraft, ArtifactError, ArtifactId, ArtifactSigner, ArtifactStore,
    ArtifactStoreError, InMemoryArtifactStore, PayloadRef,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub const ZEROG_UPLOAD_TAG: &str = "storage.zerog.upload";
pub const ZEROG_UPLOAD_SKILL: &str = "storage.zerog.upload.v1";
pub const DEFAULT_ZEROG_RPC_URL: &str = "https://evmrpc-testnet.0g.ai";
pub const DEFAULT_ZEROG_INDEXER_URL: &str = "https://indexer-storage-testnet-turbo.0g.ai";

pub struct ZeroGArtifactStore {
    inner: InMemoryArtifactStore,
    pub endpoint: String,
}

impl ZeroGArtifactStore {
    #[must_use]
    pub fn mocked(endpoint: impl Into<String>) -> Self {
        Self {
            inner: InMemoryArtifactStore::new(),
            endpoint: endpoint.into(),
        }
    }
}

impl ArtifactStore for ZeroGArtifactStore {
    fn put(&mut self, artifact: Artifact) -> Result<(), ArtifactStoreError> {
        self.inner.put(artifact)
    }
    fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, ArtifactStoreError> {
        self.inner.get(id)
    }
    fn children_of(&self, id: &ArtifactId) -> Result<Vec<Artifact>, ArtifactStoreError> {
        self.inner.children_of(id)
    }
    fn all(&self) -> Result<Vec<Artifact>, ArtifactStoreError> {
        self.inner.all()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ZeroGUploadRequest {
    pub file_path: PathBuf,
    pub network: String,
    pub rpc_url_env: String,
    pub indexer_url: String,
    pub private_key_env: String,
    pub expected_replica: u8,
    pub finality_required: bool,
}

impl ZeroGUploadRequest {
    #[must_use]
    pub fn galileo_turbo(file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
            network: "0g-galileo-turbo".to_string(),
            rpc_url_env: "ZEROG_RPC_URL".to_string(),
            indexer_url: DEFAULT_ZEROG_INDEXER_URL.to_string(),
            private_key_env: "ZEROG_PRIVATE_KEY".to_string(),
            expected_replica: 1,
            finality_required: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ZeroGUploadReceipt {
    pub network: String,
    pub indexer_url: String,
    pub root_hashes: Vec<String>,
    pub tx_hashes: Vec<String>,
    pub uploaded_at_unix: u64,
    pub audit_notes: Vec<String>,
}

impl ZeroGUploadReceipt {
    #[must_use]
    pub fn storage_uri(&self) -> Option<String> {
        match self.root_hashes.as_slice() {
            [] => None,
            [root] => Some(format!("zg://{root}")),
            roots => Some(format!("zg://{}", roots.join(","))),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ZeroGUploadAnchor {
    pub source_artifact_id: ArtifactId,
    pub source_payload_hash: Option<String>,
    pub storage_uri: Option<String>,
    pub receipt: ZeroGUploadReceipt,
}

#[derive(Debug)]
pub enum ZeroGError {
    MissingEnv(String),
    Io(String),
    Command(String),
    Parse(String),
    Artifact(ArtifactError),
}

impl Display for ZeroGError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnv(name) => write!(f, "missing required environment variable {name}"),
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Command(error) => write!(f, "command error: {error}"),
            Self::Parse(error) => write!(f, "parse error: {error}"),
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
        }
    }
}

impl std::error::Error for ZeroGError {}

impl From<ArtifactError> for ZeroGError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

pub fn upload_anchor_artifact(
    source: &Artifact,
    receipt: ZeroGUploadReceipt,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, ZeroGError> {
    let anchor = ZeroGUploadAnchor {
        source_artifact_id: source.id.clone(),
        source_payload_hash: source.payload.as_ref().map(|payload| payload.hash.clone()),
        storage_uri: receipt.storage_uri(),
        receipt,
    };
    ArtifactDraft {
        skill: SkillId(ZEROG_UPLOAD_SKILL.to_string()),
        agent,
        topic: format!("0G upload anchor for {}", source.id.0),
        input_fingerprint: format!(
            "artifact:{}:payload:{:?}",
            source.id.0, anchor.source_payload_hash
        ),
        output_cid: anchor.storage_uri.clone(),
        parent_artifact_ids: vec![source.id.clone()],
        schema_tags: BTreeSet::from([SchemaTag(ZEROG_UPLOAD_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(&anchor)?),
    }
    .seal(signer, created_at_unix)
    .map_err(ZeroGError::Artifact)
}

#[cfg(feature = "live")]
mod live {
    use super::{
        ZeroGError, ZeroGUploadReceipt, ZeroGUploadRequest, DEFAULT_ZEROG_INDEXER_URL,
        DEFAULT_ZEROG_RPC_URL,
    };
    use serde::{Deserialize, Serialize};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    #[derive(Clone, Debug)]
    pub struct ZeroGCommandUploader {
        program: PathBuf,
        program_args: Vec<String>,
        rpc_url: String,
        indexer_url: String,
        private_key_env: String,
    }

    impl ZeroGCommandUploader {
        pub fn from_env() -> Result<Self, ZeroGError> {
            let raw = std::env::var("ZEROG_UPLOAD_COMMAND")
                .map_err(|_| ZeroGError::MissingEnv("ZEROG_UPLOAD_COMMAND".to_string()))?;
            let mut tokens = raw.split_whitespace();
            let program = tokens
                .next()
                .ok_or_else(|| ZeroGError::MissingEnv("ZEROG_UPLOAD_COMMAND".to_string()))?;
            let program_args: Vec<String> = tokens.map(str::to_string).collect();
            let rpc_url = std::env::var("ZEROG_RPC_URL")
                .unwrap_or_else(|_| DEFAULT_ZEROG_RPC_URL.to_string());
            let indexer_url = std::env::var("ZEROG_INDEXER_URL")
                .unwrap_or_else(|_| DEFAULT_ZEROG_INDEXER_URL.to_string());
            if std::env::var("ZEROG_PRIVATE_KEY").is_err() {
                return Err(ZeroGError::MissingEnv("ZEROG_PRIVATE_KEY".to_string()));
            }
            Ok(Self {
                program: PathBuf::from(program),
                program_args,
                rpc_url,
                indexer_url,
                private_key_env: "ZEROG_PRIVATE_KEY".to_string(),
            })
        }

        pub fn upload_file(
            &self,
            file_path: impl AsRef<Path>,
            uploaded_at_unix: u64,
        ) -> Result<ZeroGUploadReceipt, ZeroGError> {
            let mut request = ZeroGUploadRequest::galileo_turbo(file_path.as_ref());
            request.indexer_url = self.indexer_url.clone();
            request.finality_required = false;
            let input = CommandUploadInput {
                file_path: request.file_path.display().to_string(),
                network: request.network.clone(),
                rpc_url: self.rpc_url.clone(),
                indexer_url: self.indexer_url.clone(),
                private_key_env: self.private_key_env.clone(),
                expected_replica: request.expected_replica,
                finality_required: request.finality_required,
            };
            let input_json =
                serde_json::to_vec(&input).map_err(|error| ZeroGError::Parse(error.to_string()))?;
            let mut child = Command::new(&self.program)
                .args(&self.program_args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| ZeroGError::Io(error.to_string()))?;
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| ZeroGError::Io("could not open upload command stdin".to_string()))?;
            stdin
                .write_all(&input_json)
                .map_err(|error| ZeroGError::Io(error.to_string()))?;
            drop(child.stdin.take());
            let output = child
                .wait_with_output()
                .map_err(|error| ZeroGError::Io(error.to_string()))?;
            if !output.status.success() {
                return Err(ZeroGError::Command(
                    String::from_utf8_lossy(&output.stderr).into(),
                ));
            }
            let parsed: CommandUploadOutput = serde_json::from_slice(&output.stdout)
                .map_err(|error| ZeroGError::Parse(error.to_string()))?;
            let root_hashes = parsed
                .root_hashes
                .or_else(|| parsed.root_hash.map(|root| vec![root]))
                .ok_or_else(|| ZeroGError::Parse("upload output missing root_hash".to_string()))?;
            let tx_hashes = parsed
                .tx_hashes
                .or_else(|| parsed.tx_hash.map(|tx| vec![tx]))
                .unwrap_or_default();
            Ok(ZeroGUploadReceipt {
                network: request.network,
                indexer_url: self.indexer_url.clone(),
                root_hashes,
                tx_hashes,
                uploaded_at_unix,
                audit_notes: vec![
                    "0G upload executed by operator-provided command wrapper".to_string(),
                    "private key was expected in environment, not process arguments".to_string(),
                ],
            })
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct CommandUploadInput {
        file_path: String,
        network: String,
        rpc_url: String,
        indexer_url: String,
        private_key_env: String,
        expected_replica: u8,
        finality_required: bool,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
    struct CommandUploadOutput {
        root_hash: Option<String>,
        root_hashes: Option<Vec<String>>,
        tx_hash: Option<String>,
        tx_hashes: Option<Vec<String>>,
    }
}

#[cfg(feature = "live")]
pub use live::ZeroGCommandUploader;
