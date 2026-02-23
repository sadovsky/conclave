use conclave_lang::normalize::{ast_hash, normalize, normalize_signature};
use conclave_lang::parser::parse;

fn source() -> &'static str {
    include_str!("fixtures/summarize_urls/source.conclave")
}

#[test]
fn normalize_version_ok() {
    let m = parse(source()).unwrap();
    let n = normalize(m).unwrap();
    assert_eq!(n.version, "0.1");
}

#[test]
fn normalize_version_wrong() {
    // parse will produce version "0.2", but we haven't declared assemble_json, so let's use a
    // simpler module without goals.
    // parse will succeed but normalize should reject wrong version
    let m = conclave_lang::ast::Module {
        version: "0.2".into(),
        imports: vec![],
        types: vec![],
        capabilities: vec![],
        intrinsics: vec![],
        goals: vec![],
    };
    let result = normalize(m);
    assert!(result.is_err());
    match result.unwrap_err() {
        conclave_lang::LangError::VersionMismatch { expected, got } => {
            assert_eq!(expected, "0.1");
            assert_eq!(got, "0.2");
        }
        e => panic!("unexpected error: {e}"),
    }
}

#[test]
fn normalize_sorts_capabilities() {
    let src = r#"version 0.1;
capability zzz: zzz(String) -> String;
capability aaa: aaa(String) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(x: String) -> Json { want { return assemble_json(collected); } }
"#;
    let m = parse(src).unwrap();
    let n = normalize(m).unwrap();
    assert_eq!(n.capabilities[0].alias, "aaa");
    assert_eq!(n.capabilities[1].alias, "zzz");
}

#[test]
fn normalize_sorts_types() {
    let src = r#"version 0.1;
type Zzz = String;
type Aaa = String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(x: String) -> Json { want { return assemble_json(collected); } }
"#;
    let m = parse(src).unwrap();
    let n = normalize(m).unwrap();
    assert_eq!(n.types[0].name, "Aaa");
    assert_eq!(n.types[1].name, "Zzz");
}

#[test]
fn normalize_signature_strips_whitespace() {
    assert_eq!(
        normalize_signature("fetch( Url ) -> Html"),
        "fetch(Url)->Html"
    );
    assert_eq!(normalize_signature("fetch(Url)->Html"), "fetch(Url)->Html");
    assert_eq!(
        normalize_signature("assemble_json(List<String>) -> Json"),
        "assemble_json(List<String>)->Json"
    );
}

#[test]
fn ast_hash_is_stable() {
    let m1 = parse(source()).unwrap();
    let n1 = normalize(m1).unwrap();

    let m2 = parse(source()).unwrap();
    let n2 = normalize(m2).unwrap();

    assert_eq!(ast_hash(&n1), ast_hash(&n2));
}

#[test]
fn ast_hash_whitespace_variants_equal() {
    let src_a = source();
    // Add extra whitespace/blank lines.
    let src_b = source().replace("\n", "\n\n");
    let src_b = src_b.replace("  ", "    ");

    let ma = parse(src_a).unwrap();
    let na = normalize(ma).unwrap();

    let mb = parse(&src_b).unwrap();
    let nb = normalize(mb).unwrap();

    // The normalized ASTs should be identical (and hence have the same hash).
    assert_eq!(ast_hash(&na), ast_hash(&nb));
}

#[test]
fn parse_and_normalize_import_decl() {
    let src = r#"version 0.1;
import FetchExtract: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(x: String) -> Json { want { return assemble_json(collected); } }
"#;
    let m = parse(src).unwrap();
    let n = normalize(m).unwrap();
    assert_eq!(n.imports.len(), 1);
    assert_eq!(n.imports[0].name, "FetchExtract");
    assert_eq!(
        n.imports[0].hash,
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
}

#[test]
fn normalize_sorts_imports() {
    let src = r#"version 0.1;
import ZZZ: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
import AAA: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(x: String) -> Json { want { return assemble_json(collected); } }
"#;
    let m = parse(src).unwrap();
    let n = normalize(m).unwrap();
    assert_eq!(n.imports[0].name, "AAA");
    assert_eq!(n.imports[1].name, "ZZZ");
}

#[test]
fn normalize_rejects_invalid_import_hash() {
    let src = r#"version 0.1;
import Bad: "not-a-valid-hash";
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(x: String) -> Json { want { return assemble_json(collected); } }
"#;
    let m = parse(src).unwrap();
    let result = normalize(m);
    assert!(result.is_err());
    match result.unwrap_err() {
        conclave_lang::LangError::InvalidImportHash { name, .. } => {
            assert_eq!(name, "Bad");
        }
        e => panic!("unexpected error: {e}"),
    }
}

#[test]
fn normalize_rejects_duplicate_import() {
    let src = r#"version 0.1;
import Dup: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
import Dup: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(x: String) -> Json { want { return assemble_json(collected); } }
"#;
    let m = parse(src).unwrap();
    let result = normalize(m);
    assert!(result.is_err());
    match result.unwrap_err() {
        conclave_lang::LangError::DuplicateDeclaration(s) => {
            assert!(s.contains("Dup"), "got: {s}");
        }
        e => panic!("unexpected error: {e}"),
    }
}

#[test]
fn import_in_plan_ir_imports_field() {
    let src = r#"version 0.1;
import MyModule: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(x: String) -> Json { want { return assemble_json(collected); } }
"#;
    let out = conclave_lang::lower(src, 1).unwrap();
    assert_eq!(
        out.plan_ir.imports.get("MyModule").map(String::as_str),
        Some("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
    );
}

#[test]
fn duplicate_capability_error() {
    let src = r#"version 0.1;
capability fetch: fetch(String) -> String;
capability fetch: fetch(String) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(x: String) -> Json { want { return assemble_json(collected); } }
"#;
    let m = parse(src).unwrap();
    let result = normalize(m);
    assert!(result.is_err());
    match result.unwrap_err() {
        conclave_lang::LangError::DuplicateDeclaration(s) => {
            assert!(s.contains("fetch"), "got: {s}");
        }
        e => panic!("unexpected error: {e}"),
    }
}
