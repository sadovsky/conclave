use conclave_hash::compute_stable_id;

#[test]
fn stable_id_is_deterministic() {
    let id1 = compute_stable_id("node", r#"{"kind":"intrinsic","op":"assemble_json"}"#);
    let id2 = compute_stable_id("node", r#"{"kind":"intrinsic","op":"assemble_json"}"#);
    assert_eq!(id1, id2);
}

#[test]
fn stable_id_differs_by_entity_kind() {
    let body = r#"{"x":1}"#;
    let a = compute_stable_id("node", body);
    let b = compute_stable_id("edge", body);
    assert_ne!(a, b);
}

#[test]
fn stable_id_differs_by_body() {
    let a = compute_stable_id("node", r#"{"x":1}"#);
    let b = compute_stable_id("node", r#"{"x":2}"#);
    assert_ne!(a, b);
}

#[test]
fn stable_id_has_sha256_prefix() {
    let id = compute_stable_id("node", "{}");
    assert!(id.to_string().starts_with("sha256:"));
}

#[test]
fn stable_id_golden_node() {
    // Pre-computed: sha256("conclave:v0.1" + "node" + "{}")
    // $ python3 -c "import hashlib; print(hashlib.sha256(b'conclave:v0.1node{}').hexdigest())"
    let id = compute_stable_id("node", "{}");
    assert_eq!(
        id.to_string(),
        "sha256:ce12ab44806842a364bbd09b7e51b23a7108cc9e5d09c9f31e3ba2366ca9e6fd"
    );
}
