//! Owner-only files, bounded reads, no symlink leaves and no overwrite.
use crate::{strict, Error, IdentityKind, LocalIdentity, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SecretFile {
    schema: String,
    id: String,
    kind: IdentityKind,
    seed_hex: String,
}

/// Read one bounded regular file, rejecting a symlink at the leaf.
/// # Errors
/// Rejects unavailable, oversized, nonregular or insecure secret files.
pub fn read(path: &Path, maximum: usize, secret: bool) -> Result<Vec<u8>> {
    let before = fs::symlink_metadata(path).map_err(|_| Error("input_unavailable"))?;
    if !before.is_file() || before.len() > maximum as u64 {
        return Err(Error("invalid_input_file"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|_| Error("input_unavailable"))?;
    let metadata = file.metadata().map_err(|_| Error("input_unavailable"))?;
    if !metadata.is_file() || metadata.len() > maximum as u64 {
        return Err(Error("invalid_input_file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if secret && metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error("insecure_key_permissions"));
        }
    }
    #[cfg(not(unix))]
    if secret {
        return Err(Error("key_permissions_unsupported_platform"));
    }
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error("input_unavailable"))?;
    if bytes.len() > maximum {
        return Err(Error("input_too_large"));
    }
    Ok(bytes)
}

/// Write a new owner-only file without replacing existing data.
/// # Errors
/// Rejects an existing path, unsupported permissions, or filesystem failure.
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    #[cfg(not(unix))]
    {
        let _ = (path, bytes);
        return Err(Error("private_file_permissions_unsupported_platform"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| Error("output_exists_or_unavailable"))?;
        if file
            .write_all(bytes)
            .and_then(|()| file.sync_all())
            .is_err()
        {
            return Err(Error("output_write_failed"));
        }
        Ok(())
    }
}

/// Generate a fresh key using operating-system randomness and store it privately.
/// # Errors
/// Rejects invalid identity IDs, random-source failure or unsafe output paths.
pub fn initialize(id: String, kind: IdentityKind, path: &Path) -> Result<LocalIdentity> {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).map_err(|_| Error("secure_random_unavailable"))?;
    let local = LocalIdentity::from_seed(id.clone(), kind.clone(), seed)?;
    let secret = SecretFile {
        schema: "openagents.federation.key.v1".into(),
        id,
        kind,
        seed_hex: hex::encode(seed),
    };
    write_new(
        path,
        &serde_json::to_vec(&secret).map_err(|_| Error("key_encoding_failed"))?,
    )?;
    Ok(local)
}

/// Restore a local key without exposing its seed in diagnostics.
/// # Errors
/// Rejects insecure files or invalid key records.
pub fn load_key(path: &Path) -> Result<LocalIdentity> {
    let bytes = read(path, 4096, true)?;
    let key: SecretFile = strict::decode(&bytes, 4096)?;
    if key.schema != "openagents.federation.key.v1" {
        return Err(Error("invalid_key_schema"));
    }
    let raw = hex::decode(&key.seed_hex).map_err(|_| Error("invalid_key"))?;
    let seed: [u8; 32] = raw.try_into().map_err(|_| Error("invalid_key"))?;
    LocalIdentity::from_seed(key.id, key.kind, seed)
}
