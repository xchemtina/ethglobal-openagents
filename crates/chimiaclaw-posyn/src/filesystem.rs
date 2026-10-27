use crate::{
    manifest, require, verify_portable, BundleObject, Error, VerifiedBundle, MAX_MANIFEST_BYTES,
    MAX_OBJECT_BYTES, PORTABLE_SCHEMA,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::Path;

fn directory(path: &Path) -> Result<(), Error> {
    require(
        fs::symlink_metadata(path)
            .map_err(|_| Error("directory_unavailable"))?
            .is_dir(),
        "directory_required",
    )
}

fn names(path: &Path, limit: usize) -> Result<BTreeSet<OsString>, Error> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(path).map_err(|_| Error("directory_unavailable"))? {
        names.insert(
            entry
                .map_err(|_| Error("directory_unavailable"))?
                .file_name(),
        );
        require(names.len() <= limit, "directory_inventory_limit")?;
    }
    Ok(names)
}

fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Error> {
    require(
        fs::symlink_metadata(path)
            .map_err(|_| Error("file_unavailable"))?
            .is_file(),
        "regular_file_required",
    )?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|_| Error("file_unavailable"))?;
    let metadata = file.metadata().map_err(|_| Error("file_unavailable"))?;
    require(metadata.is_file(), "regular_file_required")?;
    require(metadata.len() <= limit as u64, "file_size_limit")?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error("file_unavailable"))?;
    require(bytes.len() <= limit, "file_size_limit")?;
    Ok(bytes)
}

pub(crate) fn import(path: &Path, expected_root: &str) -> Result<VerifiedBundle, Error> {
    require(crate::valid_digest(expected_root), "invalid_expected_root")?;
    directory(path)?;
    directory(&path.join("objects"))?;
    let expected_root_names = [OsString::from("manifest.json"), OsString::from("objects")]
        .into_iter()
        .collect();
    require(
        names(path, 2)? == expected_root_names,
        "unexpected_root_inventory",
    )?;
    let manifest_bytes = read(&path.join("manifest.json"), MAX_MANIFEST_BYTES)?;
    let manifest = manifest::parse(&manifest_bytes)?;
    require(
        manifest.attestation_hash == expected_root,
        "expected_root_mismatch",
    )?;
    let mut digests = BTreeMap::new();
    for artifact in &manifest.payload.artifacts {
        if let Some(size) = digests.insert(artifact.digest.clone(), artifact.size_bytes) {
            require(size == artifact.size_bytes, "inconsistent_object_size")?;
        }
    }
    let expected_names = digests.keys().map(|s| OsString::from(&s[7..])).collect();
    require(
        names(&path.join("objects"), crate::MAX_ARTIFACTS)? == expected_names,
        "unexpected_object_inventory",
    )?;
    let mut objects = Vec::new();
    for (digest, size) in digests {
        let size = usize::try_from(size).map_err(|_| Error("object_size_limit"))?;
        require(size <= MAX_OBJECT_BYTES, "object_size_limit")?;
        let bytes = read(&path.join("objects").join(&digest[7..]), size)?;
        objects.push(BundleObject {
            digest,
            bytes_hex: hex::encode(bytes),
        });
    }
    // Recheck inventory after reads. This is not a sandbox against replacement
    // of an ancestor directory; ownership remains an explicit local boundary.
    require(
        names(path, 2)? == expected_root_names,
        "unexpected_root_inventory",
    )?;
    require(
        names(&path.join("objects"), crate::MAX_ARTIFACTS)? == expected_names,
        "unexpected_object_inventory",
    )?;
    let bundle = VerifiedBundle {
        schema: PORTABLE_SCHEMA.into(),
        manifest_hex: hex::encode(manifest_bytes),
        objects,
    };
    verify_portable(&bundle, expected_root)?;
    Ok(bundle)
}
