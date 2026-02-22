#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("ERR_ARTIFACT_TRUNCATED: artifact is too short to contain a valid trailer")]
    ArtifactTruncated,
    #[error("ERR_ARTIFACT_BAD_MAGIC: trailer magic bytes do not match CNCLV01\\0")]
    ArtifactBadMagic,
    #[error("ERR_BUNDLE_PARSE_FAILED: {0}")]
    BundleParseFailed(String),
    #[error("ERR_BUNDLE_HASH_MISMATCH: expected {expected}, got {got}")]
    BundleHashMismatch { expected: String, got: String },
    #[error("ERR_MANIFEST_HASH_MISMATCH: expected {expected}, got {got}")]
    ManifestHashMismatch { expected: String, got: String },
    #[error("ERR_PLAN_HASH_MISMATCH: expected {expected}, got {got}")]
    PlanHashMismatch { expected: String, got: String },
}
