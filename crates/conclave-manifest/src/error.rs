#[derive(Debug, thiserror::Error)]
pub enum SealError {
    #[error("program.plan_ir_hash is missing or not a valid sha256 hash")]
    MissingPlanIrHash,
    #[error("capability '{0}' referenced in Plan IR has no binding in manifest")]
    MissingCapabilityBinding(String),
    #[error("capability '{0}' has no pinned artifact_hash (floating reference)")]
    FloatingCapabilityReference(String),
    #[error("toolchain.lowerer_hash and/or runtime_hash are not pinned")]
    UnpinnedToolchain,
    #[error("determinism.clock must be \"virtual\"")]
    ClockNotVirtual,
    #[error("network capability '{0}' must be configured as replay-only in sealed_replay mode")]
    NetworkCapabilityNotReplay(String),
    #[error("plan_ir_hash mismatch: manifest has {manifest}, computed {computed}")]
    PlanIrHashMismatch { manifest: String, computed: String },
}
