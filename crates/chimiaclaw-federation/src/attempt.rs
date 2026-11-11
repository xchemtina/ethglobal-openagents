//! Prospective, signed terminal-attempt statements. This is an evidence binding,
//! not an execution grant, a trusted clock, or proof that a calculation occurred.
//! Original exchange v1 packages and their claim limits remain unchanged.
use crate::{decode_policy, digest, identifier, sha256, statement, strict, verify_evidence};
use crate::{Envelope, Error, LocalIdentity, Result, Scope, Statement, TrustPolicy};
use chimiaclaw_artifact::{Artifact, ArtifactDraft, ArtifactId, PayloadEncoding, PayloadRef};
use chimiaclaw_posyn::{parse_manifest, Role};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PACKAGE_SCHEMA: &str = "openagents.terminal-attempt.package.v1";
pub const EXPECTATION_SCHEMA: &str = "openagents.terminal-attempt.expectation.v1";
pub const RECEIPT_SCHEMA: &str = "openagents.terminal-attempt.receipt.v1";
pub const BINDING_SCHEMA: &str = "openagents.terminal-attempt.binding.v1";
pub const FAILURE_RECORD_SCHEMA: &str = "openagents.terminal-attempt.failure-record.v1";
pub const MAX_RECORD_BYTES: usize = 1024 * 1024;
pub const MAX_ATTEMPT_PACKAGE_BYTES: usize = 4 * 1024 * 1024;
const SKILL: &str = "federation.terminal-attempt.v1";

/// Separately held recipient input. Never accept this from the sender as its own
/// authorization. References identify records; this verifier grants no execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expectation {
    pub schema: String,
    pub job_id: String,
    pub attempt_id: String,
    pub authorization_sha256: String,
    pub inputs_sha256: String,
    pub environment_sha256: String,
    pub event_sha256: String,
    pub receipt_sha256: String,
    pub expected_outputs: Vec<String>,
    pub not_before_unix: u64,
    pub expires_at_unix: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    WorkerFailed,
    MissingOutput,
    InvalidOutput,
    TaskTimeout,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputIdentity {
    pub name: String,
    pub blake3: String,
    pub size_bytes: u64,
}

/// An explicitly failed terminal report may occupy the required `PoSyn` v1
/// result role. It is retained evidence, never a produced scientific output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailureRecord {
    pub schema: String,
    pub job_id: String,
    pub attempt_id: String,
    pub outcome: Outcome,
    pub failure_code: FailureCode,
    pub missing_outputs: Vec<String>,
}

/// Typed terminal summary. Its exact bytes are separately hashed; outputs plus
/// any explicitly retained failure record must equal the original result roles.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalReceipt {
    pub schema: String,
    pub scope: Scope,
    pub job_id: String,
    pub attempt_id: String,
    pub authorization_sha256: String,
    pub inputs_sha256: String,
    pub environment_sha256: String,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    pub outcome: Outcome,
    pub failure_code: Option<FailureCode>,
    pub outputs: Vec<OutputIdentity>,
    pub missing_outputs: Vec<String>,
    pub retained_failure_record: Option<OutputIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub schema: String,
    pub scope: Scope,
    pub expectation_sha256: String,
    pub event_sha256: String,
    pub receipt_sha256: String,
    pub evidence_content_hash: String,
    pub evidence_artifact_id: String,
    pub expected_posyn_root: String,
    pub receipt: TerminalReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub schema: String,
    pub artifact: Artifact,
}

/// All context is supplied explicitly. No referenced path is opened or executed.
pub struct Context<'a> {
    pub policy_bytes: &'a [u8],
    pub expectation_bytes: &'a [u8],
    pub receipt_bytes: &'a [u8],
    pub event_bytes: &'a [u8],
    pub evidence: &'a Envelope,
    pub now: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub schema: String,
    pub artifact_id: String,
    pub content_hash: String,
    pub signer_id: String,
    pub scope: Scope,
    pub job_id: String,
    pub attempt_id: String,
    pub outcome: Outcome,
    pub failure_code: Option<FailureCode>,
    pub policy_sha256: String,
    pub expectation_sha256: String,
    pub receipt_sha256: String,
    pub event_sha256: String,
    pub evidence_content_hash: String,
    pub expected_posyn_root: String,
    pub verified_at_unix: u64,
    pub trust_state: String,
    pub verification_scope: String,
    pub execution: String,
    pub authorization_reference_verified: bool,
    pub execution_authorized: bool,
    pub scientific_review: String,
    pub human_awareness: String,
    pub publication_authorized: bool,
}

/// Strict bounded package decoding; this alone does not verify any claims.
/// # Errors
/// Rejects oversized, duplicate, unknown or noncanonical fields.
pub fn decode_package(bytes: &[u8]) -> Result<Package> {
    strict::decode(bytes, MAX_ATTEMPT_PACKAGE_BYTES)
}

fn names(values: &[String]) -> Result<BTreeSet<&str>> {
    if values.len() > 128
        || values
            .iter()
            .any(|name| !identifier(name, 128) || name == "." || name == "..")
    {
        return Err(Error("invalid_output_names"));
    }
    let out: BTreeSet<_> = values.iter().map(String::as_str).collect();
    if out.len() != values.len() {
        return Err(Error("duplicate_output_name"));
    }
    Ok(out)
}

fn expectation(context: &Context<'_>, policy: &TrustPolicy) -> Result<Expectation> {
    let expected: Expectation = strict::decode(context.expectation_bytes, MAX_RECORD_BYTES)?;
    if expected.schema != EXPECTATION_SCHEMA
        || !identifier(&expected.job_id, 128)
        || !identifier(&expected.attempt_id, 128)
        || expected.not_before_unix > context.now
        || expected.expires_at_unix <= context.now
        || expected.expires_at_unix <= expected.not_before_unix
        || expected.expires_at_unix > policy.expires_at_unix
        || expected.expected_outputs.is_empty()
    {
        return Err(Error("invalid_or_expired_attempt_expectation"));
    }
    for hash in [
        &expected.authorization_sha256,
        &expected.inputs_sha256,
        &expected.environment_sha256,
        &expected.event_sha256,
        &expected.receipt_sha256,
    ] {
        if !digest(hash, "sha256:") {
            return Err(Error("invalid_attempt_digest"));
        }
    }
    names(&expected.expected_outputs)?;
    if context.event_bytes.is_empty() || context.event_bytes.len() > MAX_RECORD_BYTES {
        return Err(Error("invalid_event_bytes"));
    }
    if sha256(context.event_bytes) != expected.event_sha256
        || sha256(context.receipt_bytes) != expected.receipt_sha256
    {
        return Err(Error("event_or_receipt_digest_mismatch"));
    }
    check_observation(context.event_bytes, &expected)?;
    Ok(expected)
}

// This profile joins the existing observer v1 record without granting authority
// to its operator-reported fields. Future event profiles need explicit adapters.
fn check_observation(bytes: &[u8], expected: &Expectation) -> Result<()> {
    let event: serde_json::Value = strict::decode(bytes, MAX_RECORD_BYTES)?;
    let Some(object) = event.as_object() else {
        return Err(Error("observer_event_required"));
    };
    let keys = [
        "schema",
        "workstream_id",
        "receipt_sha256",
        "source_host",
        "source_session",
        "owner_last_reported",
        "gate_reported",
        "next_step_proposed",
        "recipients",
        "assurances",
    ];
    if object.len() != keys.len()
        || keys.iter().any(|k| !object.contains_key(*k))
        || event["schema"] != "openagents.workstream.observation.v1"
        || event["workstream_id"] != expected.job_id
        || event["source_session"] != expected.attempt_id
        || event["receipt_sha256"] != expected.receipt_sha256
        || event["assurances"]
            != serde_json::json!({
                "record_kind": "unsigned_local_observation", "receipt_claims": "unverified",
                "owner_status": "last_reported_only", "source_identity": "operator_reported_not_authenticated",
                "gate_authority": "existing_board", "human_awareness": "unknown", "independent_review": "absent",
                "execution_authorized": false, "lease_authority": false, "scientific_admission": false,
                "publication_authorized": false
            })
        || !["open", "paused", "unknown"].contains(&event["gate_reported"].as_str().unwrap_or(""))
        || event["source_host"]
            .as_str()
            .is_none_or(|v| !identifier(v, 128))
        || event["owner_last_reported"]
            .as_str()
            .is_none_or(|v| !identifier(v, 128))
        || event["next_step_proposed"]
            .as_str()
            .is_none_or(|v| v.is_empty() || v.len() > 4096)
    {
        return Err(Error("observer_event_mismatch"));
    }
    let recipients: Vec<String> = serde_json::from_value(event["recipients"].clone())
        .map_err(|_| Error("invalid_event_recipients"))?;
    if recipients.is_empty() || recipients.len() > 64 {
        return Err(Error("invalid_event_recipients"));
    }
    names(&recipients)?;
    Ok(())
}

fn checked_receipt(
    context: &Context<'_>,
    policy: &TrustPolicy,
    expected: &Expectation,
) -> Result<TerminalReceipt> {
    let receipt: TerminalReceipt = strict::decode(context.receipt_bytes, MAX_RECORD_BYTES)?;
    if receipt.schema != RECEIPT_SCHEMA
        || receipt.scope != policy.current_scope
        || receipt.job_id != expected.job_id
        || receipt.attempt_id != expected.attempt_id
        || receipt.authorization_sha256 != expected.authorization_sha256
        || receipt.inputs_sha256 != expected.inputs_sha256
        || receipt.environment_sha256 != expected.environment_sha256
        || receipt.started_at_unix < expected.not_before_unix
        || receipt.finished_at_unix < receipt.started_at_unix
        || receipt.finished_at_unix > context.now
        || receipt.finished_at_unix >= expected.expires_at_unix
        || receipt.finished_at_unix > context.evidence.artifact.created_at_unix
    {
        return Err(Error("attempt_receipt_mismatch"));
    }
    match (&receipt.outcome, &receipt.failure_code) {
        (Outcome::Succeeded, None)
            if !receipt.outputs.is_empty() && receipt.missing_outputs.is_empty() => {}
        (Outcome::TimedOut, Some(FailureCode::TaskTimeout))
        | (Outcome::Cancelled, Some(FailureCode::Cancelled))
        | (
            Outcome::Failed,
            Some(
                FailureCode::WorkerFailed | FailureCode::MissingOutput | FailureCode::InvalidOutput,
            ),
        ) => {}
        _ => return Err(Error("invalid_terminal_outcome")),
    }
    check_results(context, policy, expected, &receipt)?;
    Ok(receipt)
}

fn check_results(
    context: &Context<'_>,
    policy: &TrustPolicy,
    expected: &Expectation,
    receipt: &TerminalReceipt,
) -> Result<()> {
    let output_names: Vec<_> = receipt.outputs.iter().map(|v| v.name.clone()).collect();
    let present = names(&output_names)?;
    let missing = names(&receipt.missing_outputs)?;
    let required = names(&expected.expected_outputs)?;
    if !present.is_disjoint(&missing)
        || present.union(&missing).copied().collect::<BTreeSet<_>>() != required
    {
        return Err(Error("output_membership_mismatch"));
    }
    let Statement::Evidence { bundle, .. } = statement(context.evidence, policy, context.now)?
    else {
        return Err(Error("evidence_required"));
    };
    let manifest = parse_manifest(&bundle).map_err(|_| Error("invalid_posyn_bundle"))?;
    let results: Vec<_> = manifest
        .payload
        .artifacts
        .iter()
        .filter(|v| v.role == Role::Result)
        .collect();
    // This reserved typed report is evidence of a failed attempt. Even a newly
    // pinned expectation may not relabel its bytes as a scientific output.
    for output in &receipt.outputs {
        if let Some(object) = bundle.objects.iter().find(|v| v.digest == output.blake3) {
            let raw = hex::decode(&object.bytes_hex).map_err(|_| Error("invalid_output_bytes"))?;
            if serde_json::from_slice::<serde_json::Value>(&raw)
                .is_ok_and(|v| v["schema"] == FAILURE_RECORD_SCHEMA)
            {
                return Err(Error("failure_record_is_not_a_produced_output"));
            }
        }
    }
    let mut retained = receipt.outputs.clone();
    if let Some(failure) = &receipt.retained_failure_record {
        if receipt.outcome == Outcome::Succeeded || !receipt.outputs.is_empty() {
            return Err(Error("failure_record_requires_no_output_failure"));
        }
        names(std::slice::from_ref(&failure.name))?;
        if required.contains(failure.name.as_str()) {
            return Err(Error("failure_record_is_not_an_expected_output"));
        }
        let bytes = bundle
            .objects
            .iter()
            .find(|v| v.digest == failure.blake3)
            .ok_or(Error("missing_failure_record"))?;
        let raw = hex::decode(&bytes.bytes_hex).map_err(|_| Error("invalid_failure_record"))?;
        let record: FailureRecord = strict::decode(&raw, MAX_RECORD_BYTES)?;
        if record.schema != FAILURE_RECORD_SCHEMA
            || record.job_id != receipt.job_id
            || record.attempt_id != receipt.attempt_id
            || record.outcome != receipt.outcome
            || Some(record.failure_code) != receipt.failure_code
            || record.missing_outputs != receipt.missing_outputs
        {
            return Err(Error("failure_record_mismatch"));
        }
        retained.push(failure.clone());
    }
    if results.len() != retained.len() {
        return Err(Error("result_inventory_mismatch"));
    }
    for output in &retained {
        if !digest(&output.blake3, "blake3:")
            || !results.iter().any(|v| {
                v.name == output.name
                    && v.digest == output.blake3
                    && v.size_bytes == output.size_bytes
            })
        {
            return Err(Error("output_identity_mismatch"));
        }
    }
    Ok(())
}

fn validated(context: &Context<'_>) -> Result<(TrustPolicy, Binding)> {
    let policy = decode_policy(context.policy_bytes)?;
    policy.validate(context.now)?;
    verify_evidence(context.evidence, &policy, context.now)?;
    let expected = expectation(context, &policy)?;
    let receipt = checked_receipt(context, &policy, &expected)?;
    let binding = Binding {
        schema: BINDING_SCHEMA.into(),
        scope: policy.current_scope.clone(),
        expectation_sha256: sha256(context.expectation_bytes),
        event_sha256: sha256(context.event_bytes),
        receipt_sha256: sha256(context.receipt_bytes),
        evidence_content_hash: format!("blake3:{}", context.evidence.artifact.content_hash),
        evidence_artifact_id: context.evidence.artifact.id.0.clone(),
        expected_posyn_root: policy.expected_posyn_root.clone(),
        receipt,
    };
    Ok((policy, binding))
}

/// Sign a prospective terminal binding as the original authorized evidence
/// signer. Existing `produce_evidence` delegation grants only evidence signing.
/// # Errors
/// Rejects changed context, unauthorized signers or invalid terminal records.
pub fn create(identity: &LocalIdentity, context: &Context<'_>) -> Result<Package> {
    let (_, binding) = validated(context)?;
    if identity.identity().id != context.evidence.artifact.agent.0
        || identity.identity().public_key != context.evidence.artifact.signing_public_key
    {
        return Err(Error("attempt_requires_evidence_signer"));
    }
    let artifact = ArtifactDraft {
        skill: SkillId(SKILL.into()),
        agent: AgentId(identity.identity().id.clone()),
        topic: SKILL.into(),
        input_fingerprint: binding.scope.scope_sha256.clone(),
        output_cid: None,
        parent_artifact_ids: vec![ArtifactId(binding.evidence_artifact_id.clone())],
        schema_tags: BTreeSet::from([SchemaTag(BINDING_SCHEMA.into())]),
        payload: Some(
            PayloadRef::inline_json(&binding).map_err(|_| Error("payload_encoding_failed"))?,
        ),
    }
    .seal(&identity.signer(), context.now)
    .map_err(|_| Error("signing_failed"))?;
    let package = Package {
        schema: PACKAGE_SCHEMA.into(),
        artifact,
    };
    verify(&package, context)?;
    Ok(package)
}

/// Independently verify exact bytes and current local policy. Reverification is
/// read-only and idempotent. A different expected attempt refuses replay; callers
/// use the returned content hash with their durable observer for deduplication.
/// # Errors
/// Rejects bad signatures, stale/revoked policies/delegations, mismatched original
/// evidence, exact-byte changes and inconsistent result or failure inventories.
pub fn verify(package: &Package, context: &Context<'_>) -> Result<Verification> {
    let (policy, expected) = validated(context)?;
    let artifact = &package.artifact;
    let identity = policy.identity(&artifact.agent.0)?;
    if package.schema != PACKAGE_SCHEMA
        || artifact.signing_public_key != identity.public_key
        || artifact.agent != context.evidence.artifact.agent
        || artifact.created_at_unix > context.now
        || artifact.created_at_unix < context.evidence.artifact.created_at_unix
        || artifact.skill.0 != SKILL
        || artifact.topic != SKILL
        || artifact.input_fingerprint != policy.current_scope.scope_sha256
        || artifact.output_cid.is_some()
        || artifact.parent_artifact_ids != vec![context.evidence.artifact.id.clone()]
        || artifact.schema_tags != BTreeSet::from([SchemaTag(BINDING_SCHEMA.into())])
        || artifact
            .payload
            .as_ref()
            .is_none_or(|p| p.encoding != PayloadEncoding::Json)
    {
        return Err(Error("invalid_attempt_metadata"));
    }
    artifact
        .verify()
        .map_err(|_| Error("invalid_artifact_signature"))?;
    let payload = artifact
        .inline_payload_bytes()
        .map_err(|_| Error("invalid_inline_payload"))?
        .ok_or(Error("inline_payload_required"))?;
    artifact
        .verify_payload_bytes(&payload)
        .map_err(|_| Error("payload_hash_mismatch"))?;
    let binding: Binding = strict::decode(&payload, MAX_RECORD_BYTES)?;
    if binding != expected {
        return Err(Error("attempt_binding_mismatch"));
    }
    Ok(Verification {
        schema: "openagents.terminal-attempt.verification.v1".into(),
        artifact_id: artifact.id.0.clone(),
        content_hash: format!("blake3:{}", artifact.content_hash),
        signer_id: artifact.agent.0.clone(),
        scope: binding.scope,
        job_id: binding.receipt.job_id,
        attempt_id: binding.receipt.attempt_id,
        outcome: binding.receipt.outcome,
        failure_code: binding.receipt.failure_code,
        policy_sha256: sha256(context.policy_bytes),
        expectation_sha256: binding.expectation_sha256,
        receipt_sha256: binding.receipt_sha256,
        event_sha256: binding.event_sha256,
        evidence_content_hash: binding.evidence_content_hash,
        expected_posyn_root: binding.expected_posyn_root,
        verified_at_unix: context.now,
        trust_state: "local_snapshot".into(),
        verification_scope: "signed_statement_and_exact_retained_bytes_only".into(),
        execution: "not_verified".into(),
        authorization_reference_verified: false,
        execution_authorized: false,
        scientific_review: "not_established".into(),
        human_awareness: "unknown".into(),
        publication_authorized: false,
    })
}
