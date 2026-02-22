pub mod canonical;
pub mod digest;
pub mod stable_id;

pub use canonical::{
    remove_field, remove_field_at_path, remove_field_recursive, to_canonical_json,
};
pub use digest::{sha256_bytes, sha256_str, Hash};
pub use stable_id::compute_stable_id;

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("invalid hash format: {0}")]
    InvalidFormat(String),
}
