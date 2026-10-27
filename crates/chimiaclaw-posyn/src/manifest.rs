use crate::{
    require, valid_digest, Error, MAX_ARTIFACTS, MAX_MANIFEST_BYTES, MAX_OBJECT_BYTES,
    MAX_TOTAL_BYTES,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Input,
    Specification,
    Result,
    Provenance,
    Evidence,
}

impl Role {
    fn label(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Specification => "specification",
            Self::Result => "result",
            Self::Provenance => "provenance",
            Self::Evidence => "evidence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub role: Role,
    pub name: String,
    pub digest: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub execution: String,
    pub reproducibility: String,
    pub physical_synthesis: String,
}

impl Default for Claims {
    fn default() -> Self {
        Self {
            execution: "not_verified".into(),
            reproducibility: "not_checked".into(),
            physical_synthesis: "not_attested".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    pub schema: String,
    pub profile: String,
    pub claims: Claims,
    pub artifacts: Vec<Artifact>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub payload: Payload,
    pub attestation_hash: String,
}

// Explicit recursive ordering makes this independent of serde_json's optional
// preserve_order feature in other workspace crates. The validated PoSyn payload
// uses only portable ASCII strings and nonnegative integers; this is not JCS.
fn canonical(value: &serde_json::Value, out: &mut Vec<u8>) -> Result<(), Error> {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            out.push(b'{');
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_unstable();
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key).map_err(|_| Error("canonical_encoding"))?;
                out.push(b':');
                canonical(&map[key], out)?;
            }
            out.push(b'}');
        }
        Value::Array(values) => {
            out.push(b'[');
            for (i, value) in values.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                canonical(value, out)?;
            }
            out.push(b']');
        }
        _ => serde_json::to_writer(out, value).map_err(|_| Error("canonical_encoding"))?,
    }
    Ok(())
}

pub(crate) fn root(payload: &Payload) -> Result<String, Error> {
    let mut encoded = Vec::new();
    canonical(
        &serde_json::to_value(payload).map_err(|_| Error("canonical_encoding"))?,
        &mut encoded,
    )?;
    let mut hash = blake3::Hasher::new();
    hash.update(b"ChimiaDAO-PoSyn\0posyn.bundle.v1\0");
    hash.update(&encoded);
    Ok(format!("blake3:{}", hash.finalize().to_hex()))
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Manifest, Error> {
    require(bytes.len() <= MAX_MANIFEST_BYTES, "manifest_size_limit")?;
    let manifest: Manifest =
        serde_json::from_slice(bytes).map_err(|_| Error("invalid_manifest_json"))?;
    let payload = &manifest.payload;
    require(
        payload.schema == "posyn.bundle.v1" && payload.profile == "artifact-integrity.v1",
        "unsupported_posyn_profile",
    )?;
    require(
        payload.claims == Claims::default(),
        "unsupported_scientific_claims",
    )?;
    require(
        (4..=MAX_ARTIFACTS).contains(&payload.artifacts.len()),
        "artifact_count_limit",
    )?;
    for role in [
        Role::Input,
        Role::Specification,
        Role::Result,
        Role::Provenance,
    ] {
        require(
            payload.artifacts.iter().filter(|a| a.role == role).count() == 1,
            "required_role_count",
        )?;
    }
    let mut total = 0u64;
    for artifact in &payload.artifacts {
        let name = artifact.name.as_bytes();
        require(
            !name.is_empty()
                && name.len() <= 128
                && name[0].is_ascii_alphanumeric()
                && name
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(b)),
            "unsafe_artifact_name",
        )?;
        require(valid_digest(&artifact.digest), "invalid_artifact_digest")?;
        require(
            artifact.size_bytes <= MAX_OBJECT_BYTES as u64,
            "object_size_limit",
        )?;
        require(
            artifact.role == Role::Evidence || artifact.size_bytes > 0,
            "empty_required_artifact",
        )?;
        total = total
            .checked_add(artifact.size_bytes)
            .ok_or(Error("size_limit"))?;
    }
    require(total <= MAX_TOTAL_BYTES as u64, "total_size_limit")?;
    for pair in payload.artifacts.windows(2) {
        require(
            (pair[0].role.label(), pair[0].name.as_str())
                < (pair[1].role.label(), pair[1].name.as_str()),
            "artifact_order_or_duplicate",
        )?;
    }
    require(
        valid_digest(&manifest.attestation_hash),
        "invalid_attestation_hash",
    )?;
    require(
        root(payload)? == manifest.attestation_hash,
        "attestation_hash_mismatch",
    )?;
    Ok(manifest)
}
