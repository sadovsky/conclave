pub mod verify;
pub use verify::{verify_capability, VerifyError};

use conclave_hash::Hash;
use conclave_ir::PlanIr;
use conclave_manifest::{compute_canonical_manifest_hash, validate_seal, Manifest, SealError};

/// Input to the seal phase.
pub struct SealInput {
    pub plan_ir: PlanIr,
    /// Manifest template — may have `program.plan_ir_hash` empty; seal will fill it in.
    pub manifest: Manifest,
}

/// Output of the seal phase.
pub struct SealOutput {
    /// Fully sealed manifest (plan_ir_hash guaranteed to be set and consistent).
    pub manifest: Manifest,
    pub canonical_manifest_hash: Hash,
    pub plan_ir_hash: Hash,
}

/// Execute the seal phase.
///
/// Deterministic: same `SealInput` → same `SealOutput` bytes.
/// No filesystem, environment, or wall-clock access.
pub fn seal(mut input: SealInput) -> Result<SealOutput, SealError> {
    // 1. Compute plan_ir_hash from the canonical Plan IR.
    let plan_ir_hash = conclave_ir::compute_plan_ir_hash(&input.plan_ir);

    // 2. Set or verify program.plan_ir_hash.
    if input.manifest.program.plan_ir_hash.is_empty() {
        input.manifest.program.plan_ir_hash = plan_ir_hash.to_string();
    } else if input.manifest.program.plan_ir_hash != plan_ir_hash.to_string() {
        return Err(SealError::PlanIrHashMismatch {
            manifest: input.manifest.program.plan_ir_hash.clone(),
            computed: plan_ir_hash.to_string(),
        });
    }

    // 3. Validate all seal rules.
    validate_seal(&input.manifest, &input.plan_ir)?;

    // 4. Compute canonical_manifest_hash.
    let canonical_manifest_hash = compute_canonical_manifest_hash(&input.manifest);

    Ok(SealOutput {
        manifest: input.manifest,
        canonical_manifest_hash,
        plan_ir_hash,
    })
}
