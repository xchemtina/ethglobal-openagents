//! Portable `PoSyn` evidence, verified against a separately held commitment.
//!
//! This implements the `posyn.bundle.v1` / `artifact-integrity.v1` wire
//! profile with smaller pilot limits. Source bytes, refusals, and unverified
//! scientific claims are retained. No chemistry, process, or network is run.

mod filesystem;
mod manifest;

pub use manifest::{Artifact, Claims, Manifest, Payload, Role};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::path::Path;

pub const PORTABLE_SCHEMA: &str = "openagents.posyn-bundle.v1";
pub const MAX_ARTIFACTS: usize = 128;
pub const MAX_OBJECT_BYTES: usize = 1 << 20;
pub const MAX_TOTAL_BYTES: usize = 8 << 20;
pub const MAX_MANIFEST_BYTES: usize = 256 << 10;
pub const MAX_PORTABLE_BYTES: usize = 18 << 20;

/// An error code that never embeds private filenames or source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error(pub &'static str);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}

/// A transport container. The type name is not a trust assertion after decoding:
/// always call [`verify_portable`] with the recipient's expected root.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedBundle {
    pub schema: String,
    pub manifest_hex: String,
    pub objects: Vec<BundleObject>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleObject {
    pub digest: String,
    pub bytes_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub schema: String,
    pub status: String,
    pub attestation_hash: String,
    pub artifact_count: usize,
    pub total_bytes: u64,
    pub expected_hash_matched: bool,
    pub claims: Claims,
}

pub(crate) fn require(condition: bool, code: &'static str) -> Result<(), Error> {
    if condition {
        Ok(())
    } else {
        Err(Error(code))
    }
}

pub(crate) fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == 71 && value.starts_with("blake3:") && lower_hex(&value[7..])
}

fn decode_hex(value: &str, limit: usize) -> Result<Vec<u8>, Error> {
    require(value.len() <= limit * 2, "size_limit")?;
    require(
        value.len().is_multiple_of(2) && lower_hex(value),
        "invalid_hex",
    )?;
    hex::decode(value).map_err(|_| Error("invalid_hex"))
}

/// Parse and validate the original manifest, including its domain-separated root.
/// This does not check object bytes or establish an independently expected root.
///
/// # Errors
/// Returns a path-redacted error for invalid hex, an unsupported or malformed
/// manifest, size limits, unsupported claims, or a mismatched manifest root.
pub fn parse_manifest(bundle: &VerifiedBundle) -> Result<Manifest, Error> {
    require(
        bundle.schema == PORTABLE_SCHEMA,
        "unsupported_portable_schema",
    )?;
    manifest::parse(&decode_hex(&bundle.manifest_hex, MAX_MANIFEST_BYTES)?)
}

/// Recheck every byte and inventory against a separately held expected root.
/// A sender-controlled adjacent digest is not a trusted expected root.
///
/// # Errors
/// Rejects malformed or oversized data, noncanonical object inventories,
/// mismatched object bytes/sizes, and missing or mismatched expected roots.
pub fn verify_portable(
    bundle: &VerifiedBundle,
    expected_root: &str,
) -> Result<Verification, Error> {
    require(valid_digest(expected_root), "invalid_expected_root")?;
    let manifest = parse_manifest(bundle)?;
    require(
        manifest.attestation_hash == expected_root,
        "expected_root_mismatch",
    )?;
    require(bundle.objects.len() <= MAX_ARTIFACTS, "object_count_limit")?;
    let mut seen = BTreeMap::new();
    let mut previous = None;
    let mut unique_bytes = 0usize;
    for object in &bundle.objects {
        require(valid_digest(&object.digest), "invalid_object_digest")?;
        require(
            previous.is_none_or(|p: &str| p < object.digest.as_str()),
            "object_order_or_duplicate",
        )?;
        previous = Some(object.digest.as_str());
        let bytes = decode_hex(&object.bytes_hex, MAX_OBJECT_BYTES)?;
        unique_bytes = unique_bytes
            .checked_add(bytes.len())
            .ok_or(Error("size_limit"))?;
        require(unique_bytes <= MAX_TOTAL_BYTES, "size_limit")?;
        require(
            format!("blake3:{}", blake3::hash(&bytes).to_hex()) == object.digest,
            "object_digest_mismatch",
        )?;
        seen.insert(object.digest.as_str(), bytes.len() as u64);
    }
    let mut required = BTreeSet::new();
    let mut total_bytes = 0u64;
    for artifact in &manifest.payload.artifacts {
        required.insert(artifact.digest.as_str());
        require(
            seen.get(artifact.digest.as_str()) == Some(&artifact.size_bytes),
            "object_missing_or_size_mismatch",
        )?;
        total_bytes += artifact.size_bytes;
    }
    require(required.len() == seen.len(), "unexpected_object")?;
    Ok(Verification {
        schema: "posyn.verification.v1".into(),
        status: "integrity_verified_against_expected_hash".into(),
        attestation_hash: manifest.attestation_hash,
        artifact_count: manifest.payload.artifacts.len(),
        total_bytes,
        expected_hash_matched: true,
        claims: manifest.payload.claims,
    })
}

/// Decode bounded, typed JSON (duplicate and unknown members are rejected) and
/// verify it before returning. All unsigned source bytes remain opaque.
///
/// # Errors
/// Rejects invalid, oversized, duplicate-member or unknown-member JSON, or any
/// package that fails [`verify_portable`].
pub fn decode_portable(bytes: &[u8], expected_root: &str) -> Result<VerifiedBundle, Error> {
    require(bytes.len() <= MAX_PORTABLE_BYTES, "portable_size_limit")?;
    let bundle = serde_json::from_slice(bytes).map_err(|_| Error("invalid_portable_json"))?;
    verify_portable(&bundle, expected_root)?;
    Ok(bundle)
}

/// Import an operator-owned directory without following final symlinks. The
/// directory's parent must not be concurrently controlled by an adversary.
///
/// # Errors
/// Rejects unavailable, nonregular or oversized files, symlinked bundle paths,
/// unexpected inventories, and all failures described by [`verify_portable`].
pub fn import_bundle(path: &Path, expected_root: &str) -> Result<VerifiedBundle, Error> {
    filesystem::import(path, expected_root)
}
