//! ASKCOS retrosynthesis adapter for ChimiaClaw.
//!
//! This crate is the Rust side of the
//! `skills/scienceclaw-port/workers/retrosynth/askcos-retro` worker. It runs
//! the worker via the `CHIMIACLAW_ASKCOS_COMMAND` environment variable,
//! validates the worker's JSON output against [`AskcosTemplateSuggestions`],
//! and seals the result as a signed `chem.retrosynth.template_suggestions`
//! artifact ready to feed `apps/retroquoter`.
//!
//! By design, this crate refuses to invoke a live ASKCOS endpoint unless the
//! caller has configured the worker boundary. It does **not** provide a
//! "scraper fallback" because that would invite fabricated routes into the
//! signed retrosynthesis graph.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Stdio};

use chimiaclaw_artifact::{Artifact, ArtifactDraft, ArtifactError, ArtifactSigner, PayloadRef};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};

pub const ASKCOS_TEMPLATE_SUGGESTIONS_TAG: &str = "chem.retrosynth.template_suggestions";
pub const ASKCOS_TEMPLATE_SUGGESTIONS_SKILL: &str = "chem.retrosynth.askcos.template_relevance.v1";
pub const ASKCOS_WORKER_ENV: &str = "CHIMIACLAW_ASKCOS_COMMAND";

/// Output schema produced by the `askcos-retro` worker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AskcosTemplateSuggestions {
    pub schema_tag: String,
    pub target_smiles: String,
    pub endpoint: String,
    pub template_sets: Vec<String>,
    pub top_k: u32,
    pub seed: u64,
    pub proposals: Vec<AskcosProposal>,
    pub provenance: AskcosProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<AskcosCacheRecord>,
}

/// Disk-cache record emitted by the worker. `hit = true` means the proposals
/// were served from cache; `hit = false` means the worker called the live
/// endpoint and (unless `--no-cache` was set) wrote the response to disk.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AskcosCacheRecord {
    pub hit: bool,
    pub key: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AskcosProposal {
    pub template_set: String,
    pub request: serde_json::Value,
    pub response: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AskcosProvenance {
    pub source_kind: String,
    pub source_ref: String,
    pub notes: Vec<String>,
}

/// Errors emitted by the adapter.
#[derive(Debug)]
pub enum AskcosError {
    NotConfigured,
    Spawn(String),
    Stdin(String),
    NonZeroExit {
        status_code: Option<i32>,
        stderr: String,
    },
    NonUtf8Output(String),
    Json(String),
    Schema(String),
    Artifact(ArtifactError),
}

impl std::fmt::Display for AskcosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(
                f,
                "{ASKCOS_WORKER_ENV} is not set; refusing to invoke ASKCOS"
            ),
            Self::Spawn(message) => write!(f, "spawn worker: {message}"),
            Self::Stdin(message) => write!(f, "write SMILES to worker stdin: {message}"),
            Self::NonZeroExit {
                status_code,
                stderr,
            } => write!(
                f,
                "worker exited with status {:?}: {}",
                status_code,
                stderr.trim()
            ),
            Self::NonUtf8Output(message) => write!(f, "worker stdout not utf-8: {message}"),
            Self::Json(message) => {
                write!(
                    f,
                    "worker stdout not valid AskcosTemplateSuggestions JSON: {message}"
                )
            }
            Self::Schema(message) => write!(f, "worker output failed schema check: {message}"),
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
        }
    }
}

impl std::error::Error for AskcosError {}

impl From<ArtifactError> for AskcosError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

/// Invoke the ASKCOS worker for `target_smiles` and parse the response.
pub fn invoke_worker(target_smiles: &str) -> Result<AskcosTemplateSuggestions, AskcosError> {
    let command_line = std::env::var(ASKCOS_WORKER_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or(AskcosError::NotConfigured)?;
    let mut tokens = command_line.split_whitespace();
    let program = tokens.next().ok_or(AskcosError::NotConfigured)?;
    let args: Vec<&str> = tokens.collect();
    invoke_worker_command(program, &args, target_smiles)
}

/// Lower-level entry point used by tests.
pub fn invoke_worker_command<S: AsRef<OsStr>>(
    program: S,
    args: &[&str],
    target_smiles: &str,
) -> Result<AskcosTemplateSuggestions, AskcosError> {
    let mut child = Command::new(program.as_ref())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| AskcosError::Spawn(error.to_string()))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| AskcosError::Stdin("worker stdin not available".to_string()))?;
        stdin
            .write_all(target_smiles.trim().as_bytes())
            .map_err(|error| AskcosError::Stdin(error.to_string()))?;
        if !target_smiles.ends_with('\n') {
            let _ = stdin.write_all(b"\n");
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|error| AskcosError::Spawn(error.to_string()))?;
    if !output.status.success() {
        return Err(AskcosError::NonZeroExit {
            status_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| AskcosError::NonUtf8Output(error.to_string()))?;
    let suggestions: AskcosTemplateSuggestions = serde_json::from_str(stdout.trim())
        .map_err(|error| AskcosError::Json(error.to_string()))?;
    if suggestions.schema_tag != ASKCOS_TEMPLATE_SUGGESTIONS_TAG {
        return Err(AskcosError::Schema(format!(
            "worker emitted schema_tag={:?}, expected {:?}",
            suggestions.schema_tag, ASKCOS_TEMPLATE_SUGGESTIONS_TAG
        )));
    }
    if suggestions.proposals.is_empty() {
        return Err(AskcosError::Schema(
            "worker returned no proposals; refusing to sign empty retrosynthesis suggestions"
                .to_string(),
        ));
    }
    Ok(suggestions)
}

/// Sign a `chem.retrosynth.template_suggestions` artifact for the worker
/// output. The caller controls the agent identity, signer, and timestamp.
pub fn template_suggestions_artifact(
    suggestions: &AskcosTemplateSuggestions,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, AskcosError> {
    ArtifactDraft {
        skill: SkillId(ASKCOS_TEMPLATE_SUGGESTIONS_SKILL.to_string()),
        agent,
        topic: format!(
            "ASKCOS retrosynthesis suggestions for {}",
            suggestions.target_smiles
        ),
        input_fingerprint: format!(
            "askcos:{}:{}:{}",
            suggestions.target_smiles,
            suggestions.endpoint,
            suggestions.template_sets.join("+")
        ),
        output_cid: None,
        parent_artifact_ids: Vec::new(),
        schema_tags: BTreeSet::from([SchemaTag(ASKCOS_TEMPLATE_SUGGESTIONS_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(suggestions)?),
    }
    .seal(signer, created_at_unix)
    .map_err(AskcosError::Artifact)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_suggestions() -> AskcosTemplateSuggestions {
        AskcosTemplateSuggestions {
            schema_tag: ASKCOS_TEMPLATE_SUGGESTIONS_TAG.to_string(),
            target_smiles: "O=Cc1ccccc1".to_string(),
            endpoint: "http://duck.olympus.local:9410".to_string(),
            template_sets: vec!["reaxys".to_string(), "pistachio".to_string()],
            top_k: 5,
            seed: 2025,
            proposals: vec![AskcosProposal {
                template_set: "reaxys".to_string(),
                request: serde_json::json!({"smiles": "O=Cc1ccccc1", "template_set": "reaxys", "top_k": 5}),
                response: serde_json::json!([{"precursors": "c1ccc(C=O)cc1", "score": 0.81}]),
            }],
            provenance: AskcosProvenance {
                source_kind: "askcos-template-relevance".to_string(),
                source_ref: "test-fixture".to_string(),
                notes: vec!["fixture provenance".to_string()],
            },
            cache: None,
        }
    }

    #[test]
    fn signs_template_suggestions_as_payload_bound_artifact() {
        let suggestions = sample_suggestions();
        let signer = ArtifactSigner::from_seed([88; 32]);
        let artifact = template_suggestions_artifact(
            &suggestions,
            AgentId("retrosynth.askcos.chimiaclaw.eth".to_string()),
            &signer,
            42,
        )
        .expect("signed artifact");
        artifact.verify().expect("artifact verifies");
        artifact
            .verify_payload_value(&suggestions)
            .expect("payload binding holds");
        assert!(artifact
            .schema_tags
            .contains(&SchemaTag(ASKCOS_TEMPLATE_SUGGESTIONS_TAG.to_string())));
    }

    #[test]
    fn missing_env_returns_not_configured() {
        let previous = std::env::var(ASKCOS_WORKER_ENV).ok();
        std::env::remove_var(ASKCOS_WORKER_ENV);
        let err = invoke_worker("O=Cc1ccccc1").expect_err("worker not configured");
        assert!(matches!(err, AskcosError::NotConfigured));
        if let Some(previous) = previous {
            std::env::set_var(ASKCOS_WORKER_ENV, previous);
        }
    }

    #[test]
    fn cache_record_round_trips_through_signed_artifact() {
        let mut suggestions = sample_suggestions();
        suggestions.cache = Some(AskcosCacheRecord {
            hit: true,
            key: "abcdef0123456789".to_string(),
            path: "/tmp/chimiaclaw-askcos-cache/ab/abcdef0123456789.json".to_string(),
        });
        let signer = ArtifactSigner::from_seed([89; 32]);
        let artifact = template_suggestions_artifact(
            &suggestions,
            AgentId("retrosynth.askcos.chimiaclaw.eth".to_string()),
            &signer,
            7,
        )
        .expect("signed artifact");
        artifact.verify().expect("artifact verifies");
        artifact
            .verify_payload_value(&suggestions)
            .expect("payload binding holds");
    }

    #[test]
    fn missing_cache_field_deserializes_to_none() {
        let raw = r#"{
            "schema_tag": "chem.retrosynth.template_suggestions",
            "target_smiles": "O=Cc1ccccc1",
            "endpoint": "http://duck.olympus.local:9410",
            "template_sets": ["reaxys"],
            "top_k": 5,
            "seed": 2025,
            "proposals": [{"template_set": "reaxys", "request": {}, "response": []}],
            "provenance": {"source_kind": "askcos-template-relevance", "source_ref": "f", "notes": []}
        }"#;
        let parsed: AskcosTemplateSuggestions =
            serde_json::from_str(raw).expect("parses without cache field");
        assert!(parsed.cache.is_none());
    }
}
