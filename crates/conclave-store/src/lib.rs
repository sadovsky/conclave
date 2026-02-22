pub mod chained;
pub mod embedded;
pub mod error;
pub mod fs_store;

pub use chained::ChainedStore;
pub use embedded::EmbeddedStore;
pub use error::StoreError;
pub use fs_store::{verify_hash, FilesystemStore};

/// Core trait: look up a capability artifact by its content-addressed hash.
///
/// Returns raw binary bytes of the capability executable, or `None` if not found.
pub trait CapabilityStore {
    fn get(&self, artifact_hash: &str) -> Option<Vec<u8>>;
}

/// A no-op store that always returns `None`. Used as the fallback in sealed-replay
/// mode when no external store is provided.
pub struct EmptyCapStore;

impl CapabilityStore for EmptyCapStore {
    fn get(&self, _artifact_hash: &str) -> Option<Vec<u8>> {
        None
    }
}
