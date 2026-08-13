use crate::model::{
    DiscoveryDossier, DiscoveryError, DISCOVERY_SKILL, DOSSIER_SCHEMA, MAX_DOSSIER_BYTES,
};
use crate::strict_json::parse_strict_json_with_limit;
use crate::validation::{
    canonical_dossier_bytes, decode_dossier, is_lower_hex, validate_artifact_id,
};
use chimiaclaw_artifact::{
    blake3_hex, Artifact, ArtifactDraft, ArtifactError, ArtifactId, ArtifactSigner,
    PayloadEncoding, PayloadLocation, PayloadRef,
};
use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
use serde_json::Value;
use std::collections::BTreeSet;
/// Construct a payload-bound draft. Sealing remains an explicit caller action.
///
/// # Errors
///
/// Returns an error if the dossier is invalid or cannot be serialized.
pub fn dossier_artifact_draft(dossier: &DiscoveryDossier) -> Result<ArtifactDraft, DiscoveryError> {
    let bytes = canonical_dossier_bytes(dossier)?;
    let payload = PayloadRef {
        hash: blake3_hex(&bytes),
        encoding: PayloadEncoding::Json,
        location: PayloadLocation::Inline {
            bytes_hex: hex::encode(&bytes),
        },
    };
    Ok(ArtifactDraft {
        skill: SkillId(DISCOVERY_SKILL.to_owned()),
        agent: AgentId(dossier.provenance.agent_ids[0].clone()),
        topic: dossier.title.clone(),
        input_fingerprint: dossier.provenance.prompt_sha256.clone(),
        output_cid: None,
        parent_artifact_ids: dossier
            .provenance
            .parent_artifact_ids
            .iter()
            .cloned()
            .map(ArtifactId)
            .collect(),
        schema_tags: BTreeSet::from([SchemaTag(DOSSIER_SCHEMA.to_owned())]),
        payload: Some(payload),
    })
}

/// Validate and seal one dossier with the existing artifact signer.
///
/// # Errors
///
/// Returns an error if validation, canonicalization, or sealing fails.
pub fn seal_dossier(
    dossier: &DiscoveryDossier,
    signer: &ArtifactSigner,
) -> Result<Artifact, DiscoveryError> {
    let created_at = u64::try_from(dossier.provenance.generated_at_unix)
        .map_err(|_| DiscoveryError::new("invalid-integer", "$.provenance.generated_at_unix"))?;
    dossier_artifact_draft(dossier)?
        .seal(signer, created_at)
        .map_err(|_| DiscoveryError::new("artifact-seal-failed", "$"))
}

/// Verify a signed, inline discovery artifact and return the typed dossier.
///
/// # Errors
///
/// Returns a stable error if the artifact, payload, dossier, or cross-binding
/// fails verification.
pub fn verify_export(artifact: &Artifact) -> Result<DiscoveryDossier, DiscoveryError> {
    artifact
        .verify()
        .map_err(|_| DiscoveryError::new("artifact-integrity-failed", "$"))?;
    if artifact.skill.0 != DISCOVERY_SKILL
        || artifact.schema_tags != BTreeSet::from([SchemaTag(DOSSIER_SCHEMA.to_owned())])
        || artifact.output_cid.is_some()
    {
        return Err(DiscoveryError::new("artifact-contract-mismatch", "$"));
    }
    let payload = artifact
        .payload
        .as_ref()
        .ok_or_else(|| DiscoveryError::new("artifact-payload-missing", "$"))?;
    if payload.encoding != PayloadEncoding::Json {
        return Err(DiscoveryError::new("artifact-payload-not-json", "$"));
    }
    let bytes = payload
        .inline_bytes()
        .map_err(|_| DiscoveryError::new("artifact-payload-invalid", "$"))?
        .ok_or_else(|| DiscoveryError::new("artifact-payload-not-inline", "$"))?;
    artifact
        .verify_payload_bytes(&bytes)
        .map_err(|_| DiscoveryError::new("artifact-payload-hash-mismatch", "$"))?;
    let dossier = decode_dossier(&bytes)?;
    let expected_parents = dossier
        .provenance
        .parent_artifact_ids
        .iter()
        .map(|id| ArtifactId(id.clone()))
        .collect::<Vec<_>>();
    if artifact.parent_artifact_ids != expected_parents
        || dossier.provenance.agent_ids.first() != Some(&artifact.agent.0)
        || artifact.topic != dossier.title
        || artifact.input_fingerprint != dossier.provenance.prompt_sha256
        || artifact.created_at_unix
            != u64::try_from(dossier.provenance.generated_at_unix).unwrap_or(u64::MAX)
    {
        return Err(DiscoveryError::new(
            "artifact-dossier-binding-mismatch",
            "$",
        ));
    }
    Ok(dossier)
}

/// Strictly decode an artifact envelope before signature and payload checks.
///
/// # Errors
///
/// Returns a stable error for an oversized or malformed artifact envelope.
pub fn decode_artifact(bytes: &[u8]) -> Result<Artifact, DiscoveryError> {
    if bytes.len() > MAX_DOSSIER_BYTES.saturating_mul(3) {
        return Err(DiscoveryError::new("artifact-input-too-large", "$"));
    }
    let value = parse_strict_json_with_limit(bytes, MAX_DOSSIER_BYTES.saturating_mul(3))?;
    preflight_artifact_value(&value)?;
    serde_json::from_value(value).map_err(|_| DiscoveryError::new("invalid-artifact-shape", "$"))
}

fn object_with_exact_keys<'a>(
    value: &'a Value,
    expected: &[&str],
    at: &str,
) -> Result<&'a serde_json::Map<String, Value>, DiscoveryError> {
    object_with_exact_keys_code(value, expected, at, "invalid-artifact-shape")
}
fn object_with_exact_keys_code<'a>(
    value: &'a Value,
    expected: &[&str],
    at: &str,
    code: &'static str,
) -> Result<&'a serde_json::Map<String, Value>, DiscoveryError> {
    let object = value
        .as_object()
        .ok_or_else(|| DiscoveryError::new(code, at))?;
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(DiscoveryError::new(code, at));
    }
    Ok(object)
}

fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    at: &str,
) -> Result<&'a str, DiscoveryError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| DiscoveryError::new("invalid-artifact-shape", at))
}

fn preflight_artifact_value(value: &Value) -> Result<(), DiscoveryError> {
    let artifact = object_with_exact_keys(
        value,
        &[
            "id",
            "content_hash",
            "skill",
            "agent",
            "topic",
            "input_fingerprint",
            "output_cid",
            "parent_artifact_ids",
            "schema_tags",
            "payload",
            "created_at_unix",
            "signing_public_key",
            "signature",
        ],
        "$",
    )?;
    validate_artifact_id(required_string(artifact, "id", "$.id")?, "$.id")?;
    for (key, length) in [
        ("content_hash", 64),
        ("signing_public_key", 64),
        ("signature", 128),
    ] {
        if !is_lower_hex(required_string(artifact, key, &format!("$.{key}"))?, length) {
            return Err(DiscoveryError::new(
                "invalid-artifact-shape",
                format!("$.{key}"),
            ));
        }
    }
    let schema_tags = artifact
        .get("schema_tags")
        .and_then(Value::as_array)
        .ok_or_else(|| DiscoveryError::new("invalid-artifact-shape", "$.schema_tags"))?;
    if schema_tags.as_slice() != [Value::String(DOSSIER_SCHEMA.to_owned())] {
        return Err(DiscoveryError::new(
            "artifact-contract-mismatch",
            "$.schema_tags",
        ));
    }
    let payload = object_with_exact_keys(
        artifact
            .get("payload")
            .ok_or_else(|| DiscoveryError::new("artifact-payload-missing", "$.payload"))?,
        &["hash", "encoding", "location"],
        "$.payload",
    )?;
    if !is_lower_hex(required_string(payload, "hash", "$.payload.hash")?, 64)
        || required_string(payload, "encoding", "$.payload.encoding")? != "Json"
    {
        return Err(DiscoveryError::new("invalid-artifact-shape", "$.payload"));
    }
    let location = object_with_exact_keys(
        payload
            .get("location")
            .ok_or_else(|| DiscoveryError::new("invalid-artifact-shape", "$.payload.location"))?,
        &["Inline"],
        "$.payload.location",
    )?;
    let inline = object_with_exact_keys(
        location
            .get("Inline")
            .ok_or_else(|| DiscoveryError::new("invalid-artifact-shape", "$.payload.location"))?,
        &["bytes_hex"],
        "$.payload.location.Inline",
    )?;
    let bytes_hex = required_string(inline, "bytes_hex", "$.payload.location.Inline.bytes_hex")?;
    if bytes_hex.len() > MAX_DOSSIER_BYTES * 2
        || bytes_hex.len() % 2 != 0
        || !bytes_hex
            .bytes()
            .all(|item| item.is_ascii_digit() || (b'a'..=b'f').contains(&item))
    {
        return Err(DiscoveryError::new(
            "invalid-artifact-shape",
            "$.payload.location.Inline.bytes_hex",
        ));
    }
    Ok(())
}
/// Verify one serialized artifact and return canonical dossier bytes.
///
/// # Errors
///
/// Returns a stable error if decoding, signed integrity, binding, or dossier
/// validation fails.
pub fn verify_export_bytes(bytes: &[u8]) -> Result<Vec<u8>, DiscoveryError> {
    let artifact = decode_artifact(bytes)?;
    let dossier = verify_export(&artifact)?;
    canonical_dossier_bytes(&dossier)
}

impl From<ArtifactError> for DiscoveryError {
    fn from(_error: ArtifactError) -> Self {
        Self::new("artifact-error", "$")
    }
}
