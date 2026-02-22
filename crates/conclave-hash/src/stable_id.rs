use crate::digest::{sha256_bytes, Hash};

/// Compute a stable entity ID from its canonical body.
///
/// Formula: sha256("conclave:v0.1" || entity_kind || canonical_body)
///
/// The prefix "conclave:v0.1" is a domain separator preventing cross-version and
/// cross-entity-type collisions. All three parts are concatenated at the byte level
/// with no additional delimiters.
pub fn compute_stable_id(entity_kind: &str, canonical_body: &str) -> Hash {
    let mut data = Vec::with_capacity(
        "conclave:v0.1".len() + entity_kind.len() + canonical_body.len(),
    );
    data.extend_from_slice(b"conclave:v0.1");
    data.extend_from_slice(entity_kind.as_bytes());
    data.extend_from_slice(canonical_body.as_bytes());
    sha256_bytes(&data)
}
