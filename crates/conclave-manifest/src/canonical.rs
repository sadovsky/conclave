use crate::Manifest;
use conclave_hash::{remove_field_at_path, sha256_str, to_canonical_json, Hash};

/// Compute `canonical_manifest_hash`.
///
/// Excludes `supply_chain.manifest_signature.signature` from the hash input —
/// a signature cannot sign itself. All other fields (including the signature
/// wrapper object and `public_key_id`) are included.
pub fn compute_canonical_manifest_hash(manifest: &Manifest) -> Hash {
    let mut v = serde_json::to_value(manifest).expect("Manifest is always serializable");

    // Targeted removal of just the signature field; parent objects remain.
    remove_field_at_path(&mut v, &["supply_chain", "manifest_signature", "signature"]);

    let canonical = to_canonical_json(&v);
    sha256_str(&canonical)
}
