use conclave_hash::{remove_field_at_path, sha256_str, to_canonical_json, Hash};
use conclave_ir::PlanIr;
use conclave_manifest::Manifest;
use std::collections::BTreeMap;

/// The sealed program payload embedded in an artifact.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bundle {
    pub bundle_version: String, // "0.1"
    pub manifest: Manifest,
    pub plan_ir: PlanIr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_artifacts: Option<BTreeMap<String, EmbeddedArtifact>>,
    pub bundle_hashes: BundleHashes,
}

/// An optional embedded capability artifact (base64-encoded bytes for JSON transport).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddedArtifact {
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub bytes_encoding: String, // "raw"
    /// Base64-encoded raw bytes (for JSON transport only; hashing uses raw bytes).
    pub bytes: String,
}

/// Hashes pinned inside the bundle for integrity verification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BundleHashes {
    pub canonical_manifest_hash: String,
    pub plan_ir_hash: String,
    pub bundle_hash: String,
}

/// Serialize a bundle to canonical JSON bytes (the `bundle_bytes` in the artifact).
pub fn serialize_bundle(bundle: &Bundle) -> Vec<u8> {
    let v = serde_json::to_value(bundle).expect("Bundle is always serializable");
    to_canonical_json(&v).into_bytes()
}

/// Compute `bundle_hash`.
///
/// Algorithm:
/// 1. Serialize bundle to canonical JSON Value.
/// 2. Remove `bundle_hashes.bundle_hash` (that field can't sign itself).
/// 3. Re-canonicalize.
/// 4. sha256 the UTF-8 bytes.
pub fn compute_bundle_hash(bundle: &Bundle) -> Hash {
    let mut v = serde_json::to_value(bundle).expect("Bundle is always serializable");
    remove_field_at_path(&mut v, &["bundle_hashes", "bundle_hash"]);
    let canonical = to_canonical_json(&v);
    sha256_str(&canonical)
}
