use crate::error::StoreError;
use crate::CapabilityStore;
use conclave_hash::sha256_bytes;
use std::path::PathBuf;

/// Content-addressed filesystem store.
///
/// Layout: `<root>/<artifact_hash>/capability`
/// where `artifact_hash` is the full "sha256:<hex>" string used as a directory name.
pub struct FilesystemStore {
    pub root: PathBuf,
}

impl FilesystemStore {
    pub fn new(root: PathBuf) -> Self {
        FilesystemStore { root }
    }

    /// Install raw artifact bytes into the store. Returns the artifact_hash.
    pub fn install(&self, bytes: &[u8]) -> Result<String, StoreError> {
        let hash = sha256_bytes(bytes).to_string();
        let dir = self.root.join(&hash);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("capability"), bytes)?;
        Ok(hash)
    }
}

impl CapabilityStore for FilesystemStore {
    fn get(&self, artifact_hash: &str) -> Option<Vec<u8>> {
        let path = self.root.join(artifact_hash).join("capability");
        std::fs::read(path).ok()
    }
}

/// Verify the bytes returned from the store match the expected hash.
pub fn verify_hash(artifact_hash: &str, bytes: &[u8]) -> Result<(), StoreError> {
    let actual = sha256_bytes(bytes).to_string();
    if actual != artifact_hash {
        return Err(StoreError::HashMismatch {
            expected: artifact_hash.to_string(),
            actual,
        });
    }
    Ok(())
}
