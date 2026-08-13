use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

fn deserialize_required_nullable<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

pub const DOSSIER_SCHEMA: &str = "science.discovery.dossier.v1";
pub const DISCOVERY_SKILL: &str = "science.discovery.dossier.v1";
pub const MAX_DOSSIER_BYTES: usize = 1_048_576;
pub const MAX_JSON_DEPTH: usize = 32;
pub const MAX_JSON_NODES: usize = 20_000;

pub(crate) const MAX_SOURCES: usize = 64;
pub(crate) const MAX_CLAIMS: usize = 128;
pub(crate) const MAX_SUPPORT_PER_CLAIM: usize = 8;
pub(crate) const MAX_SUPPORT_TOTAL: usize = 512;
pub(crate) const MAX_HYPOTHESES: usize = 32;
pub(crate) const MAX_TESTS: usize = 64;
pub(crate) const MAX_CONFLICTS: usize = 32;
pub(crate) const MAX_OPEN_QUESTIONS: usize = 32;
pub(crate) const MAX_AUTHORS: usize = 32;
pub(crate) const MAX_PARENT_IDS: usize = 128;
pub(crate) const MAX_AGENT_IDS: usize = 16;
pub(crate) const MAX_MODEL_IDS: usize = 16;
pub(crate) const MAX_ID_BYTES: usize = 128;
pub(crate) const MAX_TEXT_BYTES: usize = 4096;
pub(crate) const MAX_SUMMARY_BYTES: usize = 8192;
pub(crate) const MAX_EVIDENCE_BYTES: usize = 1000;
pub(crate) const MAX_EVIDENCE_TOTAL: usize = 65_536;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryDossier {
    pub schema: String,
    pub dossier_id: String,
    pub title: String,
    pub research_question: String,
    pub summary: String,
    pub sources: Vec<Source>,
    pub claims: Vec<Claim>,
    pub hypotheses: Vec<Hypothesis>,
    pub proposed_tests: Vec<ProposedTest>,
    pub conflicts: Vec<Conflict>,
    pub open_questions: Vec<String>,
    pub provenance: Provenance,
    pub governance: Governance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub kind: SourceKind,
    pub locator: String,
    pub title: String,
    pub authors: Vec<String>,
    pub year: i64,
    pub license: String,
    pub retrieved_at_unix: i64,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub content_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SourceKind {
    #[serde(rename = "doi")]
    Doi,
    #[serde(rename = "arxiv")]
    Arxiv,
    #[serde(rename = "https")]
    Https,
    #[serde(rename = "openagents-artifact")]
    OpenagentsArtifact,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub status: ClaimStatus,
    pub support: Vec<Support>,
    pub uncertainty: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimStatus {
    #[serde(rename = "reported")]
    Reported,
    #[serde(rename = "inferred")]
    Inferred,
    #[serde(rename = "conflicting")]
    Conflicting,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Support {
    pub source_id: String,
    pub locator: String,
    pub evidence_span: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hypothesis {
    pub id: String,
    pub statement: String,
    pub rationale_claim_ids: Vec<String>,
    pub prediction: String,
    pub falsifier: String,
    pub uncertainty: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedTest {
    pub id: String,
    pub hypothesis_id: String,
    pub objective: String,
    pub method_summary: String,
    pub positive_signal: String,
    pub falsifying_signal: String,
    pub safety_review_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conflict {
    pub id: String,
    pub claim_ids: Vec<String>,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub producer: String,
    pub producer_version: String,
    pub parent_artifact_ids: Vec<String>,
    pub agent_ids: Vec<String>,
    pub model_ids: Vec<String>,
    pub prompt_sha256: String,
    pub generated_at_unix: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Governance {
    pub report_only: bool,
    pub requires_human_decision: bool,
    pub decision: String,
    pub authority: DossierAuthority,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)] // Exact wire schema: every capability must be explicit.
pub struct DossierAuthority {
    pub database_write: bool,
    pub dispatch: bool,
    pub execution: bool,
    pub publication: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryError {
    pub code: &'static str,
    pub at: String,
}

impl DiscoveryError {
    pub(crate) fn new(code: &'static str, at: impl Into<String>) -> Self {
        Self {
            code,
            at: at.into(),
        }
    }
}

impl Display for DiscoveryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.code, self.at)
    }
}

impl std::error::Error for DiscoveryError {}
