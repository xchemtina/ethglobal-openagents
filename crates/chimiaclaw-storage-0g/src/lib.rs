//! 0G Storage adapter placeholder.

use chimiaclaw_artifact::{
    Artifact, ArtifactId, ArtifactStore, ArtifactStoreError, InMemoryArtifactStore,
};

pub struct ZeroGArtifactStore {
    inner: InMemoryArtifactStore,
    pub endpoint: String,
}

impl ZeroGArtifactStore {
    #[must_use]
    pub fn mocked(endpoint: impl Into<String>) -> Self {
        Self {
            inner: InMemoryArtifactStore::new(),
            endpoint: endpoint.into(),
        }
    }
}

impl ArtifactStore for ZeroGArtifactStore {
    fn put(&mut self, artifact: Artifact) -> Result<(), ArtifactStoreError> {
        self.inner.put(artifact)
    }
    fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, ArtifactStoreError> {
        self.inner.get(id)
    }
    fn children_of(&self, id: &ArtifactId) -> Result<Vec<Artifact>, ArtifactStoreError> {
        self.inner.children_of(id)
    }
    fn all(&self) -> Result<Vec<Artifact>, ArtifactStoreError> {
        self.inner.all()
    }
}
