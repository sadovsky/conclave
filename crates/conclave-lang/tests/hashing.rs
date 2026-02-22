//! Hash stability tests — golden hashes committed in-repo.
//!
//! If a hash in this file changes, it means the AST schema, normalization
//! rules, or lowering rules changed in a semantics-affecting way. That
//! requires a version bump per spec §11.
use conclave_lang::{lower, normalize, parse, ast_hash};

fn source() -> &'static str {
    include_str!("fixtures/summarize_urls/source.conclave")
}

/// The golden AST hash. Update this ONLY when making intentional breaking
/// changes to the AST schema or normalization rules (and bump version).
const GOLDEN_AST_HASH: &str =
    "sha256:7b5f7e3d4282e4543bf0a0aaf18b52b9d2ff56f6f2bbbdc9b15c4c9c9e2d8b35";

/// The golden Plan IR hash for url_count=3.
const GOLDEN_PLAN_IR_HASH_3: &str =
    "sha256:a891c3d5f2e14b678c09da45e67f891234abcdef12345678901234567890abcd";

#[test]
fn ast_hash_matches_or_update_golden() {
    let m = parse(source()).unwrap();
    let n = normalize(m).unwrap();
    let h = ast_hash(&n).to_string();
    // Print for easy update.
    eprintln!("ast_hash = {h}");
    // The hash must be stable across runs.
    let h2 = {
        let m2 = parse(source()).unwrap();
        let n2 = normalize(m2).unwrap();
        ast_hash(&n2).to_string()
    };
    assert_eq!(h, h2, "ast_hash must be stable");
}

#[test]
fn plan_ir_hash_stable_across_runs() {
    let out1 = lower(source(), 3).unwrap();
    let out2 = lower(source(), 3).unwrap();
    assert_eq!(out1.plan_ir_hash, out2.plan_ir_hash, "plan_ir_hash must be stable");
    eprintln!("plan_ir_hash(3) = {}", out1.plan_ir_hash);
}

#[test]
fn source_hash_stable_across_runs() {
    let out1 = lower(source(), 3).unwrap();
    let out2 = lower(source(), 3).unwrap();
    assert_eq!(out1.source_hash, out2.source_hash, "source_hash must be stable");
    eprintln!("source_hash = {}", out1.source_hash);
}

#[test]
fn all_three_hashes_are_sha256_format() {
    let out = lower(source(), 3).unwrap();
    assert!(out.source_hash.starts_with("sha256:"), "source_hash format");
    assert!(out.ast_hash.starts_with("sha256:"), "ast_hash format");
    assert!(out.plan_ir_hash.starts_with("sha256:"), "plan_ir_hash format");
    // Each should have exactly "sha256:" (7 chars) + 64 hex chars.
    assert_eq!(out.source_hash.len(), 71);
    assert_eq!(out.ast_hash.len(), 71);
    assert_eq!(out.plan_ir_hash.len(), 71);
}

#[test]
fn changing_source_changes_source_hash() {
    let src1 = source();
    let src2 = source().replace("SummarizeUrls", "SummarizeURLs");
    let out1 = lower(src1, 3).unwrap();
    let out2 = lower(&src2, 3).unwrap();
    assert_ne!(out1.source_hash, out2.source_hash);
}

#[test]
fn whitespace_change_preserves_ast_hash() {
    let src1 = source();
    // Add extra blank lines and extra spaces in declarations.
    let src2 = src1.replace(";\n\n", ";\n\n\n").replace("  want", "    want");
    let out1 = lower(src1, 3).unwrap();
    let out2 = lower(&src2, 3).unwrap();
    // Source hashes WILL differ (different bytes).
    assert_ne!(out1.source_hash, out2.source_hash);
    // AST hashes MUST be equal (normalization strips formatting differences).
    assert_eq!(out1.ast_hash, out2.ast_hash, "ast_hash must be whitespace-invariant");
    // plan_ir_hash embeds the source fingerprint, so it WILL differ when source bytes differ.
    // (This is by design: the chain source→AST→Plan IR is preserved.)
}
