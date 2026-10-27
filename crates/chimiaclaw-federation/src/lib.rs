//! Offline evidence exchange. Trust comes from the separately maintained local
//! policy, not a public key embedded in a package. No network or chain is used.
pub mod files;
mod model;
mod strict;
pub use model::*;

use chimiaclaw_artifact::{ArtifactDraft, ArtifactId, ArtifactSigner, PayloadEncoding, PayloadRef};
use chimiaclaw_posyn::{verify_portable, VerifiedBundle};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Error(pub &'static str);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// A local secret, intentionally lacking Debug/Serialize. CLI keys are generated
/// from the operating system CSPRNG; `from_seed` restores a locally stored key.
pub struct LocalIdentity {
    identity: Identity,
    seed: [u8; 32],
}
impl LocalIdentity {
    /// Restore an identity from a private seed. New CLI installations use OS randomness.
    /// # Errors
    /// Rejects invalid identity IDs.
    pub fn from_seed(id: String, kind: IdentityKind, seed: [u8; 32]) -> Result<Self> {
        if !identifier(&id, 100) {
            return Err(Error("invalid_identity_id"));
        }
        let public_key = ArtifactSigner::from_seed(seed).public_key_hex();
        Ok(Self {
            identity: Identity {
                id,
                kind,
                public_key,
            },
            seed,
        })
    }
    #[must_use]
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    fn signer(&self) -> ArtifactSigner {
        ArtifactSigner::from_seed(self.seed)
    }
}

#[must_use]
pub fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}
fn identifier(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c))
}
fn hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn digest(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(hex_digest)
}
/// Decode a strict bounded policy document; time/authority checks happen separately.
/// # Errors
/// Rejects duplicate, unknown, malformed or oversized JSON.
pub fn decode_policy(bytes: &[u8]) -> Result<TrustPolicy> {
    strict::decode(bytes, MAX_POLICY_BYTES)
}
/// Decode a strict bounded package, including imported artifact fields.
/// # Errors
/// Rejects duplicate, unknown, malformed or oversized JSON.
pub fn decode_envelope(bytes: &[u8]) -> Result<Envelope> {
    strict::decode(bytes, MAX_PACKAGE_BYTES)
}

impl TrustPolicy {
    /// Validate local trust, distinct human roles, scope and freshness bounds.
    /// # Errors
    /// Rejects malformed, expired, ambiguous or revoked policy participants.
    pub fn validate(&self, now: u64) -> Result<()> {
        if self.schema != POLICY_SCHEMA
            || !identifier(&self.policy_id, 100)
            || self.observed_at_unix > now
            || self.expires_at_unix <= now
            || self.expires_at_unix <= self.observed_at_unix
        {
            return Err(Error("invalid_or_expired_policy"));
        }
        if !digest(&self.expected_posyn_root, "blake3:")
            || !identifier(&self.current_scope.scope_id, 200)
            || self.current_scope.revision == 0
            || !digest(&self.current_scope.scope_sha256, "sha256:")
        {
            return Err(Error("invalid_scope_or_root"));
        }
        if self.trusted_identities.len() > 256
            || self.revoked_keys.len() > 1024
            || self.revoked_delegations.len() > 1024
        {
            return Err(Error("policy_limit"));
        }
        let mut ids = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for identity in &self.trusted_identities {
            if !identifier(&identity.id, 100)
                || !hex_digest(&identity.public_key)
                || !ids.insert(&identity.id)
                || !keys.insert(&identity.public_key)
            {
                return Err(Error("invalid_or_duplicate_identity"));
            }
        }
        if self.revoked_keys.iter().any(|v| !hex_digest(v))
            || self
                .revoked_delegations
                .iter()
                .any(|v| !digest(v, "blake3:"))
        {
            return Err(Error("invalid_revocation"));
        }
        let scope = &self.current_scope;
        let roles = [
            &scope.coordinator_id,
            &scope.producer_id,
            &scope.reviewer_id,
        ];
        if roles.iter().collect::<BTreeSet<_>>().len() != 3 {
            return Err(Error("distinct_human_roles_required"));
        }
        for id in roles {
            if self.identity(id)?.kind != IdentityKind::Human {
                return Err(Error("human_role_required"));
            }
        }
        Ok(())
    }
    fn identity(&self, id: &str) -> Result<&Identity> {
        let found = self
            .trusted_identities
            .iter()
            .find(|identity| identity.id == id)
            .ok_or(Error("untrusted_identity"))?;
        if self.revoked_keys.contains(&found.public_key) {
            return Err(Error("revoked_key"));
        }
        Ok(found)
    }
}

fn sign(identity: &LocalIdentity, statement: &Statement, now: u64) -> Result<Envelope> {
    let parents = match statement {
        Statement::Review {
            evidence_artifact_id,
            ..
        } => vec![ArtifactId(evidence_artifact_id.clone())],
        _ => Vec::new(),
    };
    let artifact = ArtifactDraft {
        skill: SkillId("federation.exchange.v1".into()),
        agent: AgentId(identity.identity.id.clone()),
        topic: format!("federation.{}", statement.kind()),
        input_fingerprint: statement.scope().scope_sha256.clone(),
        output_cid: None,
        parent_artifact_ids: parents,
        schema_tags: BTreeSet::from([SchemaTag(format!("federation.{}.v1", statement.kind()))]),
        payload: Some(
            PayloadRef::inline_json(statement).map_err(|_| Error("payload_encoding_failed"))?,
        ),
    }
    .seal(&identity.signer(), now)
    .map_err(|_| Error("signing_failed"))?;
    Ok(Envelope {
        schema: PACKAGE_SCHEMA.into(),
        artifact,
    })
}

/// Strictly parse and verify the artifact envelope and its trusted signer before
/// interpreting the statement. Returning a Statement alone grants no authority.
fn statement(envelope: &Envelope, policy: &TrustPolicy, now: u64) -> Result<Statement> {
    if envelope.schema != PACKAGE_SCHEMA {
        return Err(Error("invalid_package_schema"));
    }
    let artifact = &envelope.artifact;
    let identity = policy.identity(&artifact.agent.0)?;
    if identity.public_key != artifact.signing_public_key {
        return Err(Error("signer_key_mismatch"));
    }
    artifact
        .verify()
        .map_err(|_| Error("invalid_artifact_signature"))?;
    if artifact.created_at_unix > now
        || artifact.output_cid.is_some()
        || artifact.skill.0 != "federation.exchange.v1"
        || artifact
            .payload
            .as_ref()
            .is_none_or(|p| p.encoding != PayloadEncoding::Json)
    {
        return Err(Error("invalid_artifact_metadata"));
    }
    let bytes = artifact
        .inline_payload_bytes()
        .map_err(|_| Error("invalid_inline_payload"))?
        .ok_or(Error("inline_payload_required"))?;
    artifact
        .verify_payload_bytes(&bytes)
        .map_err(|_| Error("payload_hash_mismatch"))?;
    let statement: Statement = strict::decode(&bytes, MAX_PACKAGE_BYTES / 2)?;
    let expected_parents = match &statement {
        Statement::Review {
            evidence_artifact_id,
            ..
        } => vec![ArtifactId(evidence_artifact_id.clone())],
        _ => Vec::new(),
    };
    if statement.scope() != &policy.current_scope {
        return Err(Error("stale_or_wrong_scope"));
    }
    if artifact.topic != format!("federation.{}", statement.kind())
        || artifact.schema_tags
            != BTreeSet::from([SchemaTag(format!("federation.{}.v1", statement.kind()))])
        || artifact.parent_artifact_ids != expected_parents
        || artifact.input_fingerprint != policy.current_scope.scope_sha256
    {
        return Err(Error("invalid_artifact_metadata"));
    }
    Ok(statement)
}

fn verify_delegation(envelope: &Envelope, policy: &TrustPolicy, now: u64) -> Result<String> {
    if policy
        .revoked_delegations
        .contains(&format!("blake3:{}", envelope.artifact.content_hash))
    {
        return Err(Error("revoked_delegation"));
    }
    let Statement::Delegation {
        owner_id,
        delegate_id,
        permission,
        expires_at_unix,
        ..
    } = statement(envelope, policy, now)?
    else {
        return Err(Error("delegation_required"));
    };
    if owner_id != policy.current_scope.producer_id
        || owner_id != envelope.artifact.agent.0
        || policy.identity(&owner_id)?.kind != IdentityKind::Human
        || policy.identity(&delegate_id)?.kind != IdentityKind::Ai
        || permission != "produce_evidence"
        || expires_at_unix <= now
        || expires_at_unix <= envelope.artifact.created_at_unix
        || expires_at_unix > policy.expires_at_unix
    {
        return Err(Error("invalid_or_expired_delegation"));
    }
    Ok(delegate_id)
}

fn verify_evidence(envelope: &Envelope, policy: &TrustPolicy, now: u64) -> Result<()> {
    let Statement::Evidence {
        bundle,
        delegation,
        expected_posyn_root,
        ..
    } = statement(envelope, policy, now)?
    else {
        return Err(Error("evidence_required"));
    };
    let signer = policy.identity(&envelope.artifact.agent.0)?;
    match (&signer.kind, delegation) {
        (IdentityKind::Human, None) if signer.id == policy.current_scope.producer_id => {}
        (IdentityKind::Ai, Some(delegation)) => {
            if verify_delegation(&delegation, policy, now)? != signer.id {
                return Err(Error("delegation_identity_mismatch"));
            }
            if delegation.artifact.created_at_unix > envelope.artifact.created_at_unix {
                return Err(Error("evidence_predates_delegation"));
            }
        }
        _ => return Err(Error("producer_authority_required")),
    }
    if expected_posyn_root != policy.expected_posyn_root {
        return Err(Error("expected_root_mismatch"));
    }
    verify_portable(&bundle, &policy.expected_posyn_root)
        .map_err(|_| Error("invalid_posyn_bundle"))?;
    Ok(())
}

/// Verify under the exact policy document bytes supplied by the local operator.
/// Review verification always requires the original evidence package as well.
/// # Errors
/// Rejects invalid signatures, stale scope, untrusted/revoked identities, invalid
/// bundles, delegated authority violations and mismatched review targets.
pub fn verify_package(
    envelope: &Envelope,
    policy_bytes: &[u8],
    now: u64,
    evidence: Option<&Envelope>,
) -> Result<VerificationReceipt> {
    let policy = decode_policy(policy_bytes)?;
    policy.validate(now)?;
    let statement = statement(envelope, &policy, now)?;
    match &statement {
        Statement::Evidence { .. } => {
            if evidence.is_some() {
                return Err(Error("unexpected_evidence_argument"));
            }
            verify_evidence(envelope, &policy, now)?;
        }
        Statement::Delegation { .. } => {
            if evidence.is_some() {
                return Err(Error("unexpected_evidence_argument"));
            }
            verify_delegation(envelope, &policy, now)?;
        }
        Statement::Review {
            evidence_content_hash,
            evidence_artifact_id,
            scientific_review,
            ..
        } => {
            let signer = policy.identity(&envelope.artifact.agent.0)?;
            if signer.kind != IdentityKind::Human
                || signer.id != policy.current_scope.reviewer_id
                || scientific_review != "not_established"
            {
                return Err(Error("human_reviewer_required"));
            }
            let evidence = evidence.ok_or(Error("review_requires_evidence"))?;
            verify_evidence(evidence, &policy, now)?;
            if evidence_content_hash != &format!("blake3:{}", evidence.artifact.content_hash)
                || evidence_artifact_id != &evidence.artifact.id.0
            {
                return Err(Error("review_target_mismatch"));
            }
            if evidence.artifact.created_at_unix > envelope.artifact.created_at_unix {
                return Err(Error("review_predates_evidence"));
            }
        }
    }
    Ok(VerificationReceipt {
        schema: "openagents.federation.verification.v1".into(),
        statement_type: statement.kind().into(),
        artifact_id: envelope.artifact.id.0.clone(),
        content_hash: format!("blake3:{}", envelope.artifact.content_hash),
        signer_id: envelope.artifact.agent.0.clone(),
        scope: policy.current_scope,
        expected_posyn_root: policy.expected_posyn_root,
        policy_sha256: sha256(policy_bytes),
        policy_observed_at_unix: policy.observed_at_unix,
        verified_at_unix: now,
        trust_state: "local_snapshot".into(),
        scientific_review: "not_established".into(),
        review_decision: match statement {
            Statement::Review { decision, .. } => Some(decision),
            _ => None,
        },
        publication_authorized: false,
        encrypted: false,
    })
}

/// Sign evidence under the producer's authority and current local policy.
/// # Errors
/// Rejects invalid bundles, policy or signer/delegation authority.
pub fn create_evidence(
    identity: &LocalIdentity,
    policy: &TrustPolicy,
    bundle: VerifiedBundle,
    delegation: Option<Envelope>,
    audience: Audience,
    now: u64,
) -> Result<Envelope> {
    policy.validate(now)?;
    let envelope = sign(
        identity,
        &Statement::Evidence {
            scope: policy.current_scope.clone(),
            audience,
            expected_posyn_root: policy.expected_posyn_root.clone(),
            bundle,
            delegation: delegation.map(Box::new),
        },
        now,
    )?;
    verify_evidence(&envelope, policy, now)?;
    Ok(envelope)
}

/// Delegate evidence production to one pinned AI identity for this exact scope.
/// # Errors
/// Rejects nonproducer owners, non-AI delegates and invalid time bounds.
pub fn create_delegation(
    identity: &LocalIdentity,
    policy: &TrustPolicy,
    delegate_id: String,
    expires_at_unix: u64,
    now: u64,
) -> Result<Envelope> {
    policy.validate(now)?;
    let envelope = sign(
        identity,
        &Statement::Delegation {
            scope: policy.current_scope.clone(),
            owner_id: identity.identity.id.clone(),
            delegate_id,
            permission: "produce_evidence".into(),
            expires_at_unix,
        },
        now,
    )?;
    verify_delegation(&envelope, policy, now)?;
    Ok(envelope)
}

/// Sign a human review decision referring to the exact verified evidence.
/// # Errors
/// Rejects invalid evidence, policy or any signer except the named human reviewer.
pub fn create_review(
    identity: &LocalIdentity,
    policy_bytes: &[u8],
    evidence: &Envelope,
    decision: ReviewDecision,
    now: u64,
) -> Result<Envelope> {
    let policy = decode_policy(policy_bytes)?;
    policy.validate(now)?;
    verify_evidence(evidence, &policy, now)?;
    let envelope = sign(
        identity,
        &Statement::Review {
            scope: policy.current_scope.clone(),
            evidence_content_hash: format!("blake3:{}", evidence.artifact.content_hash),
            evidence_artifact_id: evidence.artifact.id.0.clone(),
            decision,
            scientific_review: "not_established".into(),
        },
        now,
    )?;
    verify_package(&envelope, policy_bytes, now, Some(evidence))?;
    Ok(envelope)
}
