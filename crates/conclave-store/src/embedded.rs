use crate::error::StoreError;
use crate::CapabilityStore;
use base64::Engine;
use conclave_pack::Bundle;
use std::collections::BTreeMap;

/// In-memory store backed by a bundle's `embedded_artifacts`.
///
/// Decodes base64-encoded bytes at construction time.
pub struct EmbeddedStore {
    artifacts: BTreeMap<String, Vec<u8>>,
}

impl EmbeddedStore {
    /// Build from a packed bundle. Returns an empty store if the bundle has no
    /// embedded artifacts.
    pub fn from_bundle(bundle: &Bundle) -> Result<Self, StoreError> {
        let mut artifacts = BTreeMap::new();
        if let Some(embedded) = &bundle.embedded_artifacts {
            for (hash, artifact) in embedded {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&artifact.bytes)
                    .map_err(|e| StoreError::Base64Decode(e.to_string()))?;
                artifacts.insert(hash.clone(), bytes);
            }
        }
        Ok(EmbeddedStore { artifacts })
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

impl CapabilityStore for EmbeddedStore {
    fn get(&self, artifact_hash: &str) -> Option<Vec<u8>> {
        self.artifacts.get(artifact_hash).cloned()
    }
}
