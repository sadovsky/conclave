use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("artifact not found: {0}")]
    NotFound(String),
    #[error("artifact hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("base64 decode error: {0}")]
    Base64Decode(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
