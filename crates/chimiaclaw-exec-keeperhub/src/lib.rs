//! KeeperHub execution adapter.
//!
//! Default builds expose typed artifacts only. Compile with `--features live` to
//! enable a REST client for KeeperHub workflow execution.

use chimiaclaw_artifact::{
    Artifact, ArtifactDraft, ArtifactError, ArtifactId, ArtifactSigner, PayloadRef,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

pub const KEEPERHUB_SCHEDULED_TAG: &str = "exec.keeperhub.scheduled";
pub const KEEPERHUB_COMPLETED_TAG: &str = "exec.keeperhub.completed";
pub const KEEPERHUB_FAILED_TAG: &str = "exec.keeperhub.failed";
pub const KEEPERHUB_SCHEDULED_SKILL: &str = "exec.keeperhub.scheduled.v1";
pub const KEEPERHUB_COMPLETED_SKILL: &str = "exec.keeperhub.completed.v1";
pub const KEEPERHUB_FAILED_SKILL: &str = "exec.keeperhub.failed.v1";
pub const DEFAULT_KEEPERHUB_BASE_URL: &str = "https://app.keeperhub.com";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub job_artifact: ArtifactId,
    pub keeper_job_id: String,
}

pub trait JobScheduler {
    fn schedule(&self, job_artifact: &ArtifactId) -> Result<ScheduledJob, String>;
    fn status(&self, keeper_job_id: &str) -> Result<String, String>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum KeeperHubExecutionState {
    Submitted,
    Running,
    Succeeded,
    Failed,
    Unknown(String),
}

impl KeeperHubExecutionState {
    #[must_use]
    pub fn from_status(status: &str) -> Self {
        match status.to_ascii_lowercase().as_str() {
            "submitted" | "queued" | "pending" => Self::Submitted,
            "running" | "in_progress" | "processing" => Self::Running,
            "succeeded" | "success" | "completed" | "complete" => Self::Succeeded,
            "failed" | "error" | "errored" | "cancelled" | "canceled" => Self::Failed,
            other => Self::Unknown(other.to_string()),
        }
    }

    #[must_use]
    pub fn is_terminal_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }

    #[must_use]
    pub fn is_terminal_failure(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeeperHubWorkflowInput {
    pub workflow_id: String,
    pub input: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeeperHubScheduledExecution {
    pub workflow_id: String,
    pub execution_id: String,
    pub state: KeeperHubExecutionState,
    pub submitted_at_unix: u64,
    pub base_url: String,
    pub raw_response: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeeperHubExecutionStatus {
    pub execution_id: String,
    pub state: KeeperHubExecutionState,
    pub checked_at_unix: u64,
    pub tx_hash: Option<String>,
    pub block_explorer_url: Option<String>,
    pub raw_response: serde_json::Value,
}

#[derive(Debug)]
pub enum KeeperHubError {
    MissingEnv(String),
    Http(String),
    Api(String),
    Parse(String),
    Artifact(ArtifactError),
}

impl Display for KeeperHubError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnv(name) => write!(f, "missing required environment variable {name}"),
            Self::Http(error) => write!(f, "http error: {error}"),
            Self::Api(error) => write!(f, "api error: {error}"),
            Self::Parse(error) => write!(f, "parse error: {error}"),
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
        }
    }
}

impl std::error::Error for KeeperHubError {}

impl From<ArtifactError> for KeeperHubError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

pub fn scheduled_artifact(
    scheduled: &KeeperHubScheduledExecution,
    parent_artifact_id: Option<ArtifactId>,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, KeeperHubError> {
    ArtifactDraft {
        skill: SkillId(KEEPERHUB_SCHEDULED_SKILL.to_string()),
        agent,
        topic: format!("KeeperHub workflow scheduled: {}", scheduled.workflow_id),
        input_fingerprint: format!("keeperhub:workflow:{}", scheduled.workflow_id),
        output_cid: Some(format!("keeperhub://execution/{}", scheduled.execution_id)),
        parent_artifact_ids: parent_artifact_id.into_iter().collect(),
        schema_tags: BTreeSet::from([SchemaTag(KEEPERHUB_SCHEDULED_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(scheduled)?),
    }
    .seal(signer, created_at_unix)
    .map_err(KeeperHubError::Artifact)
}

pub fn status_artifact(
    status: &KeeperHubExecutionStatus,
    scheduled_artifact_id: Option<ArtifactId>,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, KeeperHubError> {
    let tag = if status.state.is_terminal_success() {
        KEEPERHUB_COMPLETED_TAG
    } else if status.state.is_terminal_failure() {
        KEEPERHUB_FAILED_TAG
    } else {
        KEEPERHUB_SCHEDULED_TAG
    };
    let skill = if status.state.is_terminal_success() {
        KEEPERHUB_COMPLETED_SKILL
    } else if status.state.is_terminal_failure() {
        KEEPERHUB_FAILED_SKILL
    } else {
        KEEPERHUB_SCHEDULED_SKILL
    };
    ArtifactDraft {
        skill: SkillId(skill.to_string()),
        agent,
        topic: format!("KeeperHub execution status: {}", status.execution_id),
        input_fingerprint: format!("keeperhub:execution:{}", status.execution_id),
        output_cid: status.block_explorer_url.clone().or_else(|| {
            status
                .tx_hash
                .as_ref()
                .map(|hash| format!("keeperhub://tx/{hash}"))
        }),
        parent_artifact_ids: scheduled_artifact_id.into_iter().collect(),
        schema_tags: BTreeSet::from([SchemaTag(tag.to_string())]),
        payload: Some(PayloadRef::inline_json(status)?),
    }
    .seal(signer, created_at_unix)
    .map_err(KeeperHubError::Artifact)
}

#[cfg(feature = "live")]
mod live {
    use super::{
        KeeperHubError, KeeperHubExecutionState, KeeperHubExecutionStatus,
        KeeperHubScheduledExecution, DEFAULT_KEEPERHUB_BASE_URL,
    };
    use reqwest::blocking::Client;
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
    use serde_json::Value;

    #[derive(Clone, Debug)]
    pub struct KeeperHubClient {
        base_url: String,
        api_key: String,
        client: Client,
    }

    impl KeeperHubClient {
        #[must_use]
        pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
            Self {
                base_url: base_url.into().trim_end_matches('/').to_string(),
                api_key: api_key.into(),
                client: Client::new(),
            }
        }

        pub fn from_env() -> Result<Self, KeeperHubError> {
            let api_key = std::env::var("KEEPERHUB_API_KEY")
                .map_err(|_| KeeperHubError::MissingEnv("KEEPERHUB_API_KEY".to_string()))?;
            let base_url = std::env::var("KEEPERHUB_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_KEEPERHUB_BASE_URL.to_string());
            Ok(Self::new(base_url, api_key))
        }

        pub fn execute_workflow(
            &self,
            workflow_id: &str,
            input: Value,
            submitted_at_unix: u64,
        ) -> Result<KeeperHubScheduledExecution, KeeperHubError> {
            let url = format!("{}/api/workflows/{workflow_id}/execute", self.base_url);
            let body = serde_json::json!({ "input": input });
            let response = self
                .client
                .post(url)
                .headers(self.headers()?)
                .json(&body)
                .send()
                .map_err(|error| KeeperHubError::Http(error.to_string()))?;
            let status = response.status();
            let value: Value = response
                .json()
                .map_err(|error| KeeperHubError::Http(error.to_string()))?;
            if !status.is_success() {
                return Err(KeeperHubError::Api(value.to_string()));
            }
            let execution_id = first_string(
                &value,
                &["execution_id", "executionId", "run_id", "runId", "id"],
            )
            .ok_or_else(|| KeeperHubError::Parse(format!("missing execution id in {value}")))?;
            let state = first_string(&value, &["status", "state"])
                .map(|status| KeeperHubExecutionState::from_status(&status))
                .unwrap_or(KeeperHubExecutionState::Submitted);
            Ok(KeeperHubScheduledExecution {
                workflow_id: workflow_id.to_string(),
                execution_id,
                state,
                submitted_at_unix,
                base_url: self.base_url.clone(),
                raw_response: value,
            })
        }

        pub fn execution_status(
            &self,
            execution_id: &str,
            checked_at_unix: u64,
        ) -> Result<KeeperHubExecutionStatus, KeeperHubError> {
            let url = format!("{}/api/executions/{execution_id}", self.base_url);
            let response = self
                .client
                .get(url)
                .headers(self.headers()?)
                .send()
                .map_err(|error| KeeperHubError::Http(error.to_string()))?;
            let status = response.status();
            let value: Value = response
                .json()
                .map_err(|error| KeeperHubError::Http(error.to_string()))?;
            if !status.is_success() {
                return Err(KeeperHubError::Api(value.to_string()));
            }
            let state = first_string(&value, &["status", "state"])
                .map(|status| KeeperHubExecutionState::from_status(&status))
                .unwrap_or_else(|| KeeperHubExecutionState::Unknown("missing-status".to_string()));
            Ok(KeeperHubExecutionStatus {
                execution_id: execution_id.to_string(),
                state,
                checked_at_unix,
                tx_hash: first_string(
                    &value,
                    &["tx_hash", "txHash", "transaction_hash", "transactionHash"],
                ),
                block_explorer_url: first_string(
                    &value,
                    &[
                        "block_explorer_url",
                        "blockExplorerUrl",
                        "explorer_url",
                        "explorerUrl",
                    ],
                ),
                raw_response: value,
            })
        }

        fn headers(&self) -> Result<HeaderMap, KeeperHubError> {
            let mut headers = HeaderMap::new();
            let bearer = format!("Bearer {}", self.api_key);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&bearer)
                    .map_err(|error| KeeperHubError::Http(error.to_string()))?,
            );
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            Ok(headers)
        }
    }

    fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(text) = value.get(*key).and_then(Value::as_str) {
                return Some(text.to_string());
            }
            if let Some(text) = value
                .get("data")
                .and_then(|data| data.get(*key))
                .and_then(Value::as_str)
            {
                return Some(text.to_string());
            }
        }
        None
    }
}

#[cfg(feature = "live")]
pub use live::KeeperHubClient;
