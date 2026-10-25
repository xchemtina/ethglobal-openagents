//! File-backed implementation of [`ArtifactStore`].
//!
//! Artifacts are verified on write and read, and published atomically without
//! replacing existing files. The directory itself must be owned by the operator;
//! this is not a sandbox against an actor who can replace the store directory.

use crate::{Artifact, ArtifactError, ArtifactId, ArtifactStore, ArtifactStoreError};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Directory-backed artifact store. Each artifact is one JSON file.
pub struct FileArtifactStore {
    root: PathBuf,
}

fn backend(error: std::io::Error) -> ArtifactStoreError {
    ArtifactStoreError::Backend(error.to_string())
}

fn validate_id(id: &ArtifactId) -> Result<(), ArtifactStoreError> {
    let suffix = id.0.strip_prefix("art_").unwrap_or("");
    if suffix.len() == 16
        && suffix
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(ArtifactStoreError::Backend(
            "invalid artifact id".to_string(),
        ))
    }
}

impl FileArtifactStore {
    /// Open or create an operator-owned store directory.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ArtifactStoreError> {
        fs::create_dir_all(path.as_ref()).map_err(backend)?;
        let root = fs::canonicalize(path.as_ref()).map_err(backend)?;
        Ok(Self { root })
    }

    /// Filesystem path for a canonical `art_` + 16 lowercase hex digit id.
    /// Invalid ids are rejected before any filesystem access.
    pub fn path_for(&self, id: &ArtifactId) -> Result<PathBuf, ArtifactStoreError> {
        validate_id(id)?;
        Ok(self.root.join(format!("{}.json", id.0)))
    }

    /// Root directory of this store.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn read_one(&self, id: &ArtifactId) -> Result<Option<Artifact>, ArtifactStoreError> {
        let path = self.path_for(id)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(backend(error)),
        };
        if !metadata.file_type().is_file() {
            return Err(ArtifactStoreError::Backend(
                "artifact path is not a regular file".to_string(),
            ));
        }
        let mut options = OpenOptions::new();
        options.read(true);
        // Refuse symlink substitution between metadata inspection and open.
        // NONBLOCK prevents a substituted FIFO from hanging the reader.
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let mut file = options.open(&path).map_err(backend)?;
        if !file.metadata().map_err(backend)?.is_file() {
            return Err(ArtifactStoreError::Backend(
                "artifact path is not a regular file".to_string(),
            ));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(backend)?;
        let artifact: Artifact = serde_json::from_slice(&bytes)
            .map_err(|error| ArtifactStoreError::Backend(error.to_string()))?;
        artifact.verify().map_err(ArtifactStoreError::Invalid)?;
        if &artifact.id != id {
            return Err(ArtifactStoreError::Invalid(
                ArtifactError::ArtifactIdMismatch {
                    expected: id.clone(),
                    actual: artifact.id,
                },
            ));
        }
        Ok(Some(artifact))
    }
}

impl ArtifactStore for FileArtifactStore {
    fn put(&mut self, artifact: Artifact) -> Result<(), ArtifactStoreError> {
        artifact.verify().map_err(ArtifactStoreError::Invalid)?;
        let path = self.path_for(&artifact.id)?;
        let bytes = serde_json::to_vec_pretty(&artifact)
            .map_err(|error| ArtifactStoreError::Backend(error.to_string()))?;
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let (temporary, mut file) = loop {
            let nonce = COUNTER.fetch_add(1, Ordering::Relaxed);
            let temporary = self
                .root
                .join(format!(".artifact-{}-{nonce}.tmp", std::process::id()));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => break (temporary, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(backend(error)),
            }
        };
        // Publish only complete bytes. hard_link creates the destination
        // atomically and fails if *any* entry (including a symlink) exists.
        let result = (|| {
            file.write_all(&bytes).map_err(backend)?;
            file.sync_all().map_err(backend)?;
            fs::hard_link(&temporary, &path).map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    ArtifactStoreError::Conflict(artifact.id.clone())
                } else {
                    backend(error)
                }
            })
        })();
        drop(file);
        let cleanup = fs::remove_file(&temporary);
        result?;
        cleanup.map_err(backend)?;
        Ok(())
    }

    fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, ArtifactStoreError> {
        self.read_one(id)
    }

    fn children_of(&self, id: &ArtifactId) -> Result<Vec<Artifact>, ArtifactStoreError> {
        validate_id(id)?;
        Ok(self
            .all()?
            .into_iter()
            .filter(|artifact| artifact.has_parent(id))
            .collect())
    }

    fn all(&self) -> Result<Vec<Artifact>, ArtifactStoreError> {
        let mut artifacts = Vec::new();
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(artifacts),
            Err(error) => return Err(backend(error)),
        };
        for entry in entries {
            let path = entry.map_err(backend)?.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    ArtifactStoreError::Backend("invalid artifact filename".to_string())
                })?;
            if let Some(artifact) = self.read_one(&ArtifactId(name.to_string()))? {
                artifacts.push(artifact);
            }
        }
        artifacts.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArtifactDraft, ArtifactSigner, PayloadRef};
    use chimiaclaw_schema::{AgentId, SchemaTag, SkillId};
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeSet;

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct DemoPayload {
        molecule: String,
    }

    fn temp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "chimiaclaw-store-{nanos}-{}-{count}",
            std::process::id()
        ));
        path
    }

    fn payload_artifact(parent: Vec<ArtifactId>, payload: PayloadRef) -> Artifact {
        ArtifactDraft {
            skill: SkillId("chem.demo.v1".to_string()),
            agent: AgentId("worker.demo.eth".to_string()),
            topic: "demo".to_string(),
            input_fingerprint: "input:demo".to_string(),
            output_cid: None,
            parent_artifact_ids: parent,
            schema_tags: BTreeSet::from([SchemaTag("chem.demo".to_string())]),
            payload: Some(payload),
        }
        .seal(&ArtifactSigner::from_seed([13; 32]), 1)
        .expect("seal demo")
    }

    #[test]
    fn round_trips_artifact_through_disk() {
        let dir = temp_dir();
        {
            let mut store = FileArtifactStore::open(&dir).expect("open");
            let payload = PayloadRef::inline_json(&DemoPayload {
                molecule: "H2O".to_string(),
            })
            .expect("payload");
            let artifact = payload_artifact(Vec::new(), payload);
            store.put(artifact.clone()).expect("put");
            let fetched = store.get(&artifact.id).expect("get").expect("present");
            assert_eq!(fetched, artifact);
        }
        // Reopen the store in a new process-like scope to prove restart.
        let store = FileArtifactStore::open(&dir).expect("reopen");
        let all = store.all().expect("all");
        assert_eq!(all.len(), 1);
        all[0].verify().expect("survives reload");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_conflicting_put() {
        let dir = temp_dir();
        let mut store = FileArtifactStore::open(&dir).expect("open");
        let payload = PayloadRef::inline_json(&DemoPayload {
            molecule: "CO2".to_string(),
        })
        .expect("payload");
        let artifact = payload_artifact(Vec::new(), payload);
        store.put(artifact.clone()).expect("put");
        let err = store.put(artifact.clone()).expect_err("conflict");
        assert!(matches!(err, ArtifactStoreError::Conflict(_)));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_tampered_file_on_read() {
        let dir = temp_dir();
        let mut store = FileArtifactStore::open(&dir).expect("open");
        let payload = PayloadRef::inline_json(&DemoPayload {
            molecule: "NH3".to_string(),
        })
        .expect("payload");
        let artifact = payload_artifact(Vec::new(), payload);
        store.put(artifact.clone()).expect("put");

        // Mutate the persisted JSON in a way that breaks verification.
        let path = store.path_for(&artifact.id).expect("valid path");
        let raw = fs::read_to_string(&path).expect("read disk");
        let tampered = raw.replace("\"demo\"", "\"forged\"");
        fs::write(&path, tampered).expect("write tampered");

        let err = store.get(&artifact.id).expect_err("rejected");
        assert!(matches!(err, ArtifactStoreError::Invalid(_)));
        fs::remove_dir_all(&dir).ok();
    }
    fn fixture() -> Artifact {
        payload_artifact(
            Vec::new(),
            PayloadRef::inline_json(&DemoPayload {
                molecule: "H2O".to_string(),
            })
            .expect("payload"),
        )
    }

    #[test]
    fn rejects_noncanonical_ids_before_accessing_disk() {
        let dir = temp_dir();
        let store = FileArtifactStore::open(&dir).expect("open");
        for value in [
            "../outside",
            "/tmp/outside",
            "art_123",
            "art_ABCDEF0123456789",
            "art_0123456789abcdef/../outside",
            "art_0123456789abcdef\\outside",
        ] {
            let id = ArtifactId(value.to_string());
            assert!(store.path_for(&id).is_err(), "{value}");
            assert!(store.get(&id).is_err(), "{value}");
        }
        assert!(store
            .get(&ArtifactId("art_0123456789abcdef".to_string()))
            .expect("get")
            .is_none());
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn rejects_valid_artifact_under_another_id() {
        let dir = temp_dir();
        let store = FileArtifactStore::open(&dir).expect("open");
        let wrong_id = ArtifactId("art_0123456789abcdef".to_string());
        let artifact = fixture();
        assert_ne!(artifact.id, wrong_id);
        fs::write(
            store.path_for(&wrong_id).expect("path"),
            serde_json::to_vec(&artifact).expect("serialize"),
        )
        .expect("write");
        assert!(matches!(
            store.get(&wrong_id),
            Err(ArtifactStoreError::Invalid(
                ArtifactError::ArtifactIdMismatch { .. }
            ))
        ));
        assert!(store.all().is_err());
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn rejects_unrecognized_json_filename_and_nonregular_entry() {
        let dir = temp_dir();
        let store = FileArtifactStore::open(&dir).expect("open");
        fs::write(
            dir.join("outside.json"),
            serde_json::to_vec(&fixture()).expect("serialize"),
        )
        .expect("write");
        assert!(store.all().is_err());
        fs::remove_file(dir.join("outside.json")).expect("remove");
        fs::create_dir(store.path_for(&fixture().id).expect("path")).expect("directory");
        assert!(store.get(&fixture().id).is_err());
        assert!(store.all().is_err());
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn rejects_weak_key_forgery_on_write_and_read() {
        let dir = temp_dir();
        let mut store = FileArtifactStore::open(&dir).expect("open");
        let mut artifact = fixture();
        artifact.signing_public_key = format!("01{}", "00".repeat(31));
        artifact.signature = format!("01{}", "00".repeat(63));
        assert!(matches!(
            store.put(artifact.clone()),
            Err(ArtifactStoreError::Invalid(ArtifactError::Signature(_)))
        ));
        fs::write(
            store.path_for(&artifact.id).expect("path"),
            serde_json::to_vec(&artifact).expect("serialize"),
        )
        .expect("write");
        assert!(matches!(
            store.get(&artifact.id),
            Err(ArtifactStoreError::Invalid(ArtifactError::Signature(_)))
        ));
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn concurrent_publication_has_exactly_one_winner() {
        use std::sync::{Arc, Barrier};
        let dir = temp_dir();
        let first = FileArtifactStore::open(&dir).expect("open first");
        let second = FileArtifactStore::open(&dir).expect("open second");
        let barrier = Arc::new(Barrier::new(2));
        let artifact = fixture();
        let threads: Vec<_> = [first, second]
            .into_iter()
            .map(|mut store| {
                let barrier = barrier.clone();
                let artifact = artifact.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    store.put(artifact)
                })
            })
            .collect();
        let outcomes: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().expect("join"))
            .collect();
        assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|r| matches!(r, Err(ArtifactStoreError::Conflict(_))))
                .count(),
            1
        );
        let store = FileArtifactStore::open(&dir).expect("reopen");
        assert_eq!(store.get(&artifact.id).expect("get"), Some(artifact));
        assert_eq!(
            fs::read_dir(&dir).expect("entries").count(),
            1,
            "no temporary file left"
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_reads_and_writes_never_reach_external_target() {
        use std::os::unix::fs::symlink;
        let dir = temp_dir();
        let mut store = FileArtifactStore::open(&dir).expect("open");
        let artifact = fixture();
        let external = dir.with_extension("external");
        let bytes = serde_json::to_vec(&artifact).expect("serialize");
        fs::write(&external, &bytes).expect("external fixture");
        let path = store.path_for(&artifact.id).expect("path");
        symlink(&external, &path).expect("symlink");
        assert!(store.get(&artifact.id).is_err());
        assert!(store.all().is_err());
        assert!(matches!(
            store.put(artifact.clone()),
            Err(ArtifactStoreError::Conflict(_))
        ));
        assert_eq!(fs::read(&external).expect("read external"), bytes);
        fs::remove_file(&external).expect("make dangling");
        assert!(store.get(&artifact.id).is_err());
        assert!(matches!(
            store.put(artifact),
            Err(ArtifactStoreError::Conflict(_))
        ));
        assert!(
            !external.exists(),
            "dangling link must not create external target"
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
