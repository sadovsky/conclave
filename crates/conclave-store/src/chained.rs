use crate::CapabilityStore;

/// A store that tries `primary` first, then falls back to `fallback`.
pub struct ChainedStore<A: CapabilityStore, B: CapabilityStore> {
    primary: A,
    fallback: B,
}

impl<A: CapabilityStore, B: CapabilityStore> ChainedStore<A, B> {
    pub fn new(primary: A, fallback: B) -> Self {
        ChainedStore { primary, fallback }
    }
}

impl<A: CapabilityStore, B: CapabilityStore> CapabilityStore for ChainedStore<A, B> {
    fn get(&self, artifact_hash: &str) -> Option<Vec<u8>> {
        self.primary.get(artifact_hash).or_else(|| self.fallback.get(artifact_hash))
    }
}
