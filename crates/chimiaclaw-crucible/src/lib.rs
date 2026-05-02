//! Discourse primitives for SciCrucible.
//!
//! This crate hosts the canonical signed-artifact shapes that turn the
//! SciCrucible dashboard's voting / peer-review flow into auditable
//! ChimiaClaw artifacts. The first artifact type is
//! [`crucible.review.vote`](VOTE_SCHEMA_TAG) — a single, signed vote on a
//! specific artifact, bound to that artifact's content hash so the vote is
//! invalidated by tampering.
//!
//! No network, no on-chain coupling. The artifact merely records a discourse
//! event in a way that is verifiable, content-addressed, and parented to the
//! object of discussion.

#![allow(clippy::module_name_repetitions)]

use chimiaclaw_artifact::{
    Artifact, ArtifactDraft, ArtifactError, ArtifactId, ArtifactSigner, PayloadRef,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

pub const VOTE_SCHEMA_TAG: &str = "crucible.review.vote";
pub const VOTE_SKILL: &str = "crucible.review.vote.v1";

/// Maximum byte length of a freeform rationale string, applied at validation
/// time so reviewers cannot stuff a manifesto into a single signed vote.
pub const MAX_RATIONALE_BYTES: usize = 4096;

/// Closed enumeration of vote outcomes.
///
/// Closed on purpose: a discourse substrate that lets every reviewer invent
/// their own outcome label is not a substrate, it is a comment thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum VoteKind {
    Approve,
    Reject,
    Abstain,
    RequestRevision,
}

impl VoteKind {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::Abstain => "abstain",
            Self::RequestRevision => "request-revision",
        }
    }
}

impl Display for VoteKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Layered identity for the voter.
///
/// At least one of the three fields must be present at validation time.
/// Multiple may be set — for example, a researcher who has both an ORCID
/// and an ENS-resolved Ethereum address.
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct VoterIdentity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ens_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eth_address: Option<String>,
}

impl VoterIdentity {
    #[must_use]
    pub fn from_orcid(orcid: impl Into<String>) -> Self {
        Self {
            orcid: Some(orcid.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn from_ens(ens_name: impl Into<String>) -> Self {
        Self {
            ens_name: Some(ens_name.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn from_eth_address(eth_address: impl Into<String>) -> Self {
        Self {
            eth_address: Some(eth_address.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.orcid.is_none() && self.ens_name.is_none() && self.eth_address.is_none()
    }
}

/// How / where the vote was submitted. Included in the signed payload so
/// downstream auditors can distinguish web-UI votes from API-direct votes.
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct VoteProvenance {
    /// e.g. `"scicrucible-web"`, `"chimiaclaw-cli"`, `"agent.peer-review"`.
    pub source_kind: String,
    /// Free-form ref: route path, session id, agent task id.
    pub source_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// A single, signed vote on a target artifact.
///
/// The signed bytes commit to `target_content_hash`, so a vote made against
/// a particular SCF result is invalidated the moment that result's bytes are
/// tampered with. The vote artifact's `parent_artifact_ids` references the
/// target by id, so consumers can walk the discourse graph in either
/// direction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewVote {
    /// Stable id chosen by the issuing client. Must be non-empty.
    pub vote_id: String,
    pub voter_identity: VoterIdentity,
    pub target_artifact_id: ArtifactId,
    /// Blake3 hex digest of the target artifact's canonical payload bytes.
    pub target_content_hash: String,
    pub target_schema_tag: SchemaTag,
    pub kind: VoteKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    pub issued_at_unix: u64,
    pub provenance: VoteProvenance,
    /// Schema-tag self-reference, embedded in the payload so deserializers
    /// can dispatch without an out-of-band envelope. Always
    /// [`VOTE_SCHEMA_TAG`].
    #[serde(default = "default_vote_schema_tag")]
    pub schema_tag: String,
}

fn default_vote_schema_tag() -> String {
    VOTE_SCHEMA_TAG.to_string()
}

impl ReviewVote {
    /// Construct a vote with sensible defaults, then call [`Self::validate`].
    #[must_use]
    pub fn new(
        vote_id: impl Into<String>,
        voter_identity: VoterIdentity,
        target_artifact_id: ArtifactId,
        target_content_hash: impl Into<String>,
        target_schema_tag: SchemaTag,
        kind: VoteKind,
        issued_at_unix: u64,
        provenance: VoteProvenance,
    ) -> Self {
        Self {
            vote_id: vote_id.into(),
            voter_identity,
            target_artifact_id,
            target_content_hash: target_content_hash.into(),
            target_schema_tag,
            kind,
            rationale: None,
            issued_at_unix,
            provenance,
            schema_tag: VOTE_SCHEMA_TAG.to_string(),
        }
    }

    #[must_use]
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Validate the vote payload. Returns the first error encountered.
    pub fn validate(&self) -> Result<(), CrucibleError> {
        if self.vote_id.trim().is_empty() {
            return Err(CrucibleError::EmptyVoteId);
        }
        if self.voter_identity.is_empty() {
            return Err(CrucibleError::MissingVoterIdentity);
        }
        if self.target_content_hash.len() != 64
            || !self
                .target_content_hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err(CrucibleError::InvalidContentHash {
                actual: self.target_content_hash.clone(),
            });
        }
        if self.target_schema_tag.0.trim().is_empty() {
            return Err(CrucibleError::EmptyTargetSchemaTag);
        }
        if let Some(r) = &self.rationale {
            if r.len() > MAX_RATIONALE_BYTES {
                return Err(CrucibleError::RationaleTooLong {
                    max: MAX_RATIONALE_BYTES,
                    actual: r.len(),
                });
            }
        }
        if self.schema_tag != VOTE_SCHEMA_TAG {
            return Err(CrucibleError::WrongSchemaTag {
                expected: VOTE_SCHEMA_TAG,
                actual: self.schema_tag.clone(),
            });
        }
        if self.provenance.source_kind.trim().is_empty() {
            return Err(CrucibleError::EmptyProvenanceSourceKind);
        }
        Ok(())
    }
}

/// Build a signed `crucible.review.vote` artifact from a validated payload.
///
/// The artifact's `parent_artifact_ids` will contain the target artifact id,
/// so the vote can be walked back to its target through the artifact graph.
pub fn vote_artifact(
    vote: &ReviewVote,
    agent: AgentId,
    signer: &ArtifactSigner,
    created_at_unix: u64,
) -> Result<Artifact, CrucibleError> {
    vote.validate()?;
    let parent_artifact_ids = vec![vote.target_artifact_id.clone()];
    let input_fingerprint = format!(
        "vote:{}:{}:{}",
        vote.vote_id,
        vote.target_artifact_id.0,
        vote.kind.as_str()
    );
    let topic = format!(
        "Vote {} on artifact {}",
        vote.kind.as_str(),
        vote.target_artifact_id.0
    );
    ArtifactDraft {
        skill: SkillId(VOTE_SKILL.to_string()),
        agent,
        topic,
        input_fingerprint,
        output_cid: None,
        parent_artifact_ids,
        schema_tags: BTreeSet::from([SchemaTag(VOTE_SCHEMA_TAG.to_string())]),
        payload: Some(PayloadRef::inline_json(vote)?),
    }
    .seal(signer, created_at_unix)
    .map_err(CrucibleError::Artifact)
}

#[derive(Debug)]
pub enum CrucibleError {
    EmptyVoteId,
    MissingVoterIdentity,
    InvalidContentHash {
        actual: String,
    },
    EmptyTargetSchemaTag,
    EmptyProvenanceSourceKind,
    RationaleTooLong {
        max: usize,
        actual: usize,
    },
    WrongSchemaTag {
        expected: &'static str,
        actual: String,
    },
    Artifact(ArtifactError),
}

impl Display for CrucibleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyVoteId => write!(f, "vote_id must be non-empty"),
            Self::MissingVoterIdentity => write!(
                f,
                "voter_identity must include at least one of orcid, ens_name, eth_address"
            ),
            Self::InvalidContentHash { actual } => write!(
                f,
                "target_content_hash must be a 64-char hex Blake3 digest, got {actual:?}"
            ),
            Self::EmptyTargetSchemaTag => write!(f, "target_schema_tag must be non-empty"),
            Self::EmptyProvenanceSourceKind => {
                write!(f, "provenance.source_kind must be non-empty")
            }
            Self::RationaleTooLong { max, actual } => {
                write!(f, "rationale is {actual} bytes, max is {max}")
            }
            Self::WrongSchemaTag { expected, actual } => {
                write!(f, "schema_tag must be {expected:?}, got {actual:?}")
            }
            Self::Artifact(error) => write!(f, "artifact error: {error:?}"),
        }
    }
}

impl std::error::Error for CrucibleError {}

impl From<ArtifactError> for CrucibleError {
    fn from(value: ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_content_hash() -> String {
        // 64 hex chars
        "a".repeat(64)
    }

    fn baseline_vote() -> ReviewVote {
        ReviewVote::new(
            "vote_001",
            VoterIdentity::from_orcid("0000-0001-2345-6789"),
            ArtifactId("art_3d5c1283b1a8f79f".into()),
            fake_content_hash(),
            SchemaTag("chem.dft.result".into()),
            VoteKind::Approve,
            1_730_000_000,
            VoteProvenance {
                source_kind: "scicrucible-web".into(),
                source_ref: "/dft/art_3d5c1283b1a8f79f".into(),
                user_agent: None,
                notes: Vec::new(),
            },
        )
    }

    #[test]
    fn validates_baseline_vote() {
        baseline_vote().validate().expect("baseline vote valid");
    }

    #[test]
    fn rejects_empty_vote_id() {
        let mut v = baseline_vote();
        v.vote_id = String::new();
        assert!(matches!(v.validate(), Err(CrucibleError::EmptyVoteId)));
    }

    #[test]
    fn rejects_missing_identity() {
        let mut v = baseline_vote();
        v.voter_identity = VoterIdentity::default();
        assert!(matches!(
            v.validate(),
            Err(CrucibleError::MissingVoterIdentity)
        ));
    }

    #[test]
    fn rejects_bad_content_hash() {
        let mut v = baseline_vote();
        v.target_content_hash = "deadbeef".into();
        assert!(matches!(
            v.validate(),
            Err(CrucibleError::InvalidContentHash { .. })
        ));
    }

    #[test]
    fn rejects_long_rationale() {
        let mut v = baseline_vote();
        v.rationale = Some("x".repeat(MAX_RATIONALE_BYTES + 1));
        assert!(matches!(
            v.validate(),
            Err(CrucibleError::RationaleTooLong { .. })
        ));
    }

    #[test]
    fn json_roundtrip_preserves_schema_tag() {
        let v = baseline_vote();
        let bytes = serde_json::to_vec(&v).unwrap();
        let parsed: ReviewVote = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.schema_tag, VOTE_SCHEMA_TAG);
        assert_eq!(parsed, v);
    }

    #[test]
    fn signed_artifact_carries_target_as_parent() {
        let v = baseline_vote();
        let signer = ArtifactSigner::from_seed([7u8; 32]);
        let artifact = vote_artifact(
            &v,
            AgentId("agent.scicrucible".into()),
            &signer,
            1_730_000_001,
        )
        .expect("seal");
        assert_eq!(
            artifact.parent_artifact_ids,
            vec![v.target_artifact_id.clone()]
        );
        assert!(artifact.schema_tags.iter().any(|t| t.0 == VOTE_SCHEMA_TAG));
        // Decode payload and confirm round-trip.
        let bytes = artifact
            .payload
            .as_ref()
            .unwrap()
            .inline_bytes()
            .unwrap()
            .unwrap();
        let parsed: ReviewVote = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn vote_artifact_rejects_invalid_payload() {
        let mut v = baseline_vote();
        v.vote_id.clear();
        let signer = ArtifactSigner::from_seed([7u8; 32]);
        let err = vote_artifact(&v, AgentId("a".into()), &signer, 1).unwrap_err();
        assert!(matches!(err, CrucibleError::EmptyVoteId));
    }
}
