use crate::bundle::{compute_bundle_hash, serialize_bundle, Bundle, BundleHashes};
use crate::error::PackError;
use conclave_hash::sha256_bytes;
use conclave_ir::compute_plan_ir_hash;
use conclave_manifest::compute_canonical_manifest_hash;

/// Trailer magic: "CNCLV01\0" (7 ASCII chars + NUL byte).
const MAGIC: &[u8; 8] = b"CNCLV01\0";
const TRAILER_LEN: usize = 16;

/// Input for the pack operation.
pub struct PackInput {
    pub runtime_bytes: Vec<u8>,
    pub bundle: Bundle,
}

/// Output from the pack operation.
pub struct PackOutput {
    pub artifact_bytes: Vec<u8>,
    pub artifact_hash: conclave_hash::Hash,
    pub bundle_hash: conclave_hash::Hash,
}

/// Pack: `runtime_bytes || bundle_bytes || trailer`.
///
/// Deterministic: no timestamps, no randomness, no host-specific data.
pub fn pack(mut input: PackInput) -> Result<PackOutput, PackError> {
    // Compute hashes that go inside the bundle.
    let plan_ir_hash = compute_plan_ir_hash(&input.bundle.plan_ir);
    let canonical_manifest_hash = compute_canonical_manifest_hash(&input.bundle.manifest);

    // Set bundle_hashes (bundle_hash placeholder; computed after).
    input.bundle.bundle_hashes = BundleHashes {
        canonical_manifest_hash: canonical_manifest_hash.to_string(),
        plan_ir_hash: plan_ir_hash.to_string(),
        bundle_hash: String::new(), // placeholder; filled below
    };

    // Compute and insert bundle_hash.
    let bundle_hash = compute_bundle_hash(&input.bundle);
    input.bundle.bundle_hashes.bundle_hash = bundle_hash.to_string();

    // Serialize bundle to canonical JSON bytes.
    let bundle_bytes = serialize_bundle(&input.bundle);
    let bundle_len = bundle_bytes.len() as u64;

    // Build trailer: bundle_len_u64_le (8 bytes) || MAGIC (8 bytes).
    let mut trailer = [0u8; TRAILER_LEN];
    trailer[..8].copy_from_slice(&bundle_len.to_le_bytes());
    trailer[8..].copy_from_slice(MAGIC);

    // Assemble artifact.
    let mut artifact = Vec::with_capacity(
        input.runtime_bytes.len() + bundle_bytes.len() + TRAILER_LEN,
    );
    artifact.extend_from_slice(&input.runtime_bytes);
    artifact.extend_from_slice(&bundle_bytes);
    artifact.extend_from_slice(&trailer);

    let artifact_hash = sha256_bytes(&artifact);

    Ok(PackOutput {
        artifact_bytes: artifact,
        artifact_hash,
        bundle_hash,
    })
}

/// Unpack: validate trailer, extract and verify bundle.
pub fn unpack(artifact_bytes: &[u8]) -> Result<Bundle, PackError> {
    let len = artifact_bytes.len();

    // Minimum size: at least the trailer.
    if len < TRAILER_LEN {
        return Err(PackError::ArtifactTruncated);
    }

    // Validate magic (last 8 bytes).
    let magic_start = len - 8;
    if &artifact_bytes[magic_start..] != MAGIC {
        return Err(PackError::ArtifactBadMagic);
    }

    // Read bundle_len (bytes len-16..len-8).
    let bundle_len_bytes: [u8; 8] = artifact_bytes[len - 16..len - 8]
        .try_into()
        .map_err(|_| PackError::ArtifactTruncated)?;
    let bundle_len = u64::from_le_bytes(bundle_len_bytes) as usize;

    // Locate bundle start.
    let bundle_end = len - TRAILER_LEN;
    let bundle_start = bundle_end.checked_sub(bundle_len).ok_or(PackError::ArtifactTruncated)?;

    // Parse bundle JSON.
    let bundle_bytes = &artifact_bytes[bundle_start..bundle_end];
    let bundle: Bundle = serde_json::from_slice(bundle_bytes)
        .map_err(|e| PackError::BundleParseFailed(e.to_string()))?;

    // Verify plan_ir_hash consistency.
    let stored_plan_hash = bundle.bundle_hashes.plan_ir_hash.clone();
    let computed_plan_hash = compute_plan_ir_hash(&bundle.plan_ir).to_string();
    if stored_plan_hash != computed_plan_hash {
        return Err(PackError::PlanHashMismatch {
            expected: stored_plan_hash,
            got: computed_plan_hash,
        });
    }
    if bundle.manifest.program.plan_ir_hash != stored_plan_hash {
        return Err(PackError::PlanHashMismatch {
            expected: bundle.manifest.program.plan_ir_hash.clone(),
            got: stored_plan_hash,
        });
    }

    // Verify canonical_manifest_hash.
    let stored_manifest_hash = bundle.bundle_hashes.canonical_manifest_hash.clone();
    let computed_manifest_hash = compute_canonical_manifest_hash(&bundle.manifest).to_string();
    if stored_manifest_hash != computed_manifest_hash {
        return Err(PackError::ManifestHashMismatch {
            expected: stored_manifest_hash,
            got: computed_manifest_hash,
        });
    }

    // Verify bundle_hash (must match after removing bundle_hash field from the computation).
    let stored_bundle_hash = bundle.bundle_hashes.bundle_hash.clone();
    let computed_bundle_hash = compute_bundle_hash(&bundle).to_string();
    if stored_bundle_hash != computed_bundle_hash {
        return Err(PackError::BundleHashMismatch {
            expected: stored_bundle_hash,
            got: computed_bundle_hash,
        });
    }

    Ok(bundle)
}
