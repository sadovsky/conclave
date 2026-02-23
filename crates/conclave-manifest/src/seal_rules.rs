use crate::{Manifest, SealError};
use conclave_hash::Hash;
use conclave_ir::PlanIr;

/// Validate that the manifest satisfies all Seal rules before producing an artifact.
///
/// Rules enforced (per CLAUDE.md §7 and manifest_spec.md §2):
/// 1. `program.plan_ir_hash` is present and valid "sha256:..." format.
/// 2. Every `capability_call` node in Plan IR has a binding in `capability_bindings`.
/// 3. Every capability binding has a pinned `artifact_hash` (no floating references).
/// 4. `toolchain.lowerer_hash` and `toolchain.runtime_hash` are pinned.
/// 5. `determinism.clock == "virtual"`.
/// 6. In `sealed_replay` mode, any capability with network trust is configured as replay-only.
pub fn validate_seal(manifest: &Manifest, plan_ir: &PlanIr) -> Result<(), SealError> {
    // Rule 1: plan_ir_hash present and valid.
    if manifest.program.plan_ir_hash.is_empty() {
        return Err(SealError::MissingPlanIrHash);
    }
    Hash::parse(&manifest.program.plan_ir_hash).map_err(|_| SealError::MissingPlanIrHash)?;

    // Rule 2: all capability_call nodes must have a binding.
    for node in &plan_ir.nodes {
        if matches!(node.kind, conclave_ir::NodeKind::CapabilityCall)
            && !manifest
                .capability_bindings
                .contains_key(&node.op.signature)
        {
            return Err(SealError::MissingCapabilityBinding(
                node.op.signature.clone(),
            ));
        }
    }

    // Rule 3: all bindings must have a pinned artifact_hash.
    for (sig, binding) in &manifest.capability_bindings {
        if binding.artifact_hash.is_empty() {
            return Err(SealError::FloatingCapabilityReference(sig.clone()));
        }
        Hash::parse(&binding.artifact_hash)
            .map_err(|_| SealError::FloatingCapabilityReference(sig.clone()))?;
    }

    // Rule 4: toolchain hashes must be pinned.
    let tc = &manifest.toolchain;
    if tc.lowerer_hash.is_empty() || tc.runtime_hash.is_empty() {
        return Err(SealError::UnpinnedToolchain);
    }
    Hash::parse(&tc.lowerer_hash).map_err(|_| SealError::UnpinnedToolchain)?;
    Hash::parse(&tc.runtime_hash).map_err(|_| SealError::UnpinnedToolchain)?;

    // Rule 5: determinism.clock must be "virtual".
    if manifest.determinism.clock != "virtual" {
        return Err(SealError::ClockNotVirtual);
    }

    // Rule 6: in sealed_replay mode, network capabilities must be replay-only.
    if manifest.determinism.mode == "sealed_replay" {
        for (sig, binding) in &manifest.capability_bindings {
            let is_network = binding.trust.contains("network");
            if is_network {
                let fetch_mode = binding
                    .config
                    .as_ref()
                    .and_then(|c| c.get("fetch_mode"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if fetch_mode != "replay" {
                    return Err(SealError::NetworkCapabilityNotReplay(sig.clone()));
                }
            }
        }
    }

    // Rule 7: if signatures.required == true, accepted_keys must be non-empty.
    for (sig, binding) in &manifest.capability_bindings {
        if let Some(sigs) = &binding.signatures {
            if sigs.required && sigs.accepted_keys.is_empty() {
                return Err(SealError::SignatureRequiredButNoKeys(sig.clone()));
            }
        }
    }

    // Rule 8: every import in Plan IR must have a corresponding module_bindings entry
    // with the matching hash. This makes transitive dependencies explicit and auditable.
    for (import_name, plan_ir_hash) in &plan_ir.imports {
        match manifest.module_bindings.get(import_name) {
            None => return Err(SealError::MissingModuleBinding(import_name.clone())),
            Some(manifest_hash) if manifest_hash != plan_ir_hash => {
                return Err(SealError::ModuleBindingHashMismatch {
                    import_name: import_name.clone(),
                    plan_ir_hash: plan_ir_hash.clone(),
                    manifest_hash: manifest_hash.clone(),
                });
            }
            _ => {}
        }
    }

    Ok(())
}
