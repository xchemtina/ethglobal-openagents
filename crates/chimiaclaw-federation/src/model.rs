use chimiaclaw_artifact::Artifact;
use chimiaclaw_posyn::VerifiedBundle;
use serde::{Deserialize, Serialize};

pub const POLICY_SCHEMA: &str = "openagents.federation.policy.v1";
pub const PACKAGE_SCHEMA: &str = "openagents.federation.package.v1";
pub const MAX_PACKAGE_BYTES: usize = 48 * 1024 * 1024;
pub const MAX_POLICY_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityKind {
    Human,
    Ai,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub id: String,
    pub kind: IdentityKind,
    pub public_key: String,
}

/// Scope IDs are opaque and must include the source handoff identity when used
/// with `PoX`. A new handoff must not inherit acceptance of the old one.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub scope_id: String,
    pub revision: u64,
    pub scope_sha256: String,
    pub coordinator_id: String,
    pub producer_id: String,
    pub reviewer_id: String,
}

/// Local administrator input, never inferred from a received package. Public
/// keys and the expected root must be confirmed over a separate trusted channel.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustPolicy {
    pub schema: String,
    pub policy_id: String,
    pub observed_at_unix: u64,
    pub expires_at_unix: u64,
    pub current_scope: Scope,
    pub expected_posyn_root: String,
    pub trusted_identities: Vec<Identity>,
    pub revoked_keys: Vec<String>,
    pub revoked_delegations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    Private,
    Synthetic,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Reviewed,
    CorrectionsRequired,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub schema: String,
    pub artifact: Artifact,
}

/// All fields below are inside the signed artifact payload. Human review is a
/// review of the evidence package, never a claim of scientific admission.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "statement_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Statement {
    Evidence {
        scope: Scope,
        audience: Audience,
        expected_posyn_root: String,
        bundle: VerifiedBundle,
        delegation: Option<Box<Envelope>>,
    },
    Delegation {
        scope: Scope,
        owner_id: String,
        delegate_id: String,
        permission: String,
        expires_at_unix: u64,
    },
    Review {
        scope: Scope,
        evidence_content_hash: String,
        evidence_artifact_id: String,
        decision: ReviewDecision,
        scientific_review: String,
    },
}

impl Statement {
    #[must_use]
    pub fn scope(&self) -> &Scope {
        match self {
            Self::Evidence { scope, .. }
            | Self::Delegation { scope, .. }
            | Self::Review { scope, .. } => scope,
        }
    }
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Evidence { .. } => "evidence",
            Self::Delegation { .. } => "delegation",
            Self::Review { .. } => "review",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationReceipt {
    pub schema: String,
    pub statement_type: String,
    pub artifact_id: String,
    pub content_hash: String,
    pub signer_id: String,
    pub scope: Scope,
    pub expected_posyn_root: String,
    pub policy_sha256: String,
    pub policy_observed_at_unix: u64,
    pub verified_at_unix: u64,
    pub trust_state: String,
    pub scientific_review: String,
    pub review_decision: Option<ReviewDecision>,
    pub publication_authorized: bool,
    pub encrypted: bool,
}
