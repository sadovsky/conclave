use crate::ast::*;
use crate::error::LangError;
use std::collections::BTreeSet;

/// Apply all v0.1 normalization rules to a parsed `Module`.
///
/// Rules (from spec §5):
/// 1. Line endings already normalized before lexing.
/// 2. Validate `version == "0.1"`.
/// 3. Sort type declarations by name.
/// 4. Sort capability declarations by alias.
/// 5. Sort intrinsic declarations by alias.
/// 6. Normalize signature formatting: strip ALL whitespace.
/// 7. Validate no duplicate names within each declaration kind.
pub fn normalize(mut module: Module) -> Result<Module, LangError> {
    // Rule 2: version must be "0.1"
    if module.version != "0.1" {
        return Err(LangError::VersionMismatch {
            expected: "0.1".into(),
            got: module.version.clone(),
        });
    }

    // Validate import hash format and sort imports.
    for imp in &module.imports {
        validate_import_hash(&imp.name, &imp.hash)?;
    }
    module.imports.sort_by(|a, b| a.name.cmp(&b.name));
    check_no_duplicates(module.imports.iter().map(|i| i.name.as_str()), "import")?;

    // Rule 6: normalize signatures (before dedup checks so duplicates after
    // normalization are caught).
    for cap in &mut module.capabilities {
        cap.signature = normalize_signature(&cap.signature);
    }
    for intr in &mut module.intrinsics {
        intr.signature = normalize_signature(&intr.signature);
    }

    // Rules 3–5: sort declarations.
    module.types.sort_by(|a, b| a.name.cmp(&b.name));
    module.capabilities.sort_by(|a, b| a.alias.cmp(&b.alias));
    module.intrinsics.sort_by(|a, b| a.alias.cmp(&b.alias));

    // Rule 7: duplicate checks.
    check_no_duplicates(module.types.iter().map(|t| t.name.as_str()), "type")?;
    check_no_duplicates(
        module.capabilities.iter().map(|c| c.alias.as_str()),
        "capability",
    )?;
    check_no_duplicates(
        module.intrinsics.iter().map(|i| i.alias.as_str()),
        "intrinsic",
    )?;
    check_no_duplicates(module.goals.iter().map(|g| g.name.as_str()), "goal")?;

    Ok(module)
}

/// Strip all ASCII whitespace from a signature string.
///
/// `"fetch( Url ) -> Html"` → `"fetch(Url)->Html"`
/// `"assemble_json(List<String>) -> Json"` → `"assemble_json(List<String>)->Json"`
pub fn normalize_signature(sig: &str) -> String {
    sig.chars().filter(|c| !c.is_ascii_whitespace()).collect()
}

fn check_no_duplicates<'a, I>(names: I, kind: &str) -> Result<(), LangError>
where
    I: Iterator<Item = &'a str>,
{
    let mut seen = BTreeSet::new();
    for name in names {
        if !seen.insert(name) {
            return Err(LangError::DuplicateDeclaration(format!("{kind}::{name}")));
        }
    }
    Ok(())
}

/// Validate that a hash is `sha256:` followed by exactly 64 lowercase hex chars.
fn validate_import_hash(name: &str, hash: &str) -> Result<(), LangError> {
    let hex = hash.strip_prefix("sha256:").ok_or_else(|| LangError::InvalidImportHash {
        name: name.to_string(),
        hash: hash.to_string(),
    })?;
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(LangError::InvalidImportHash {
            name: name.to_string(),
            hash: hash.to_string(),
        });
    }
    Ok(())
}

/// Compute a canonical JSON hash of a normalized AST module.
///
/// This hash is stable: the same source (modulo whitespace/formatting) always
/// produces the same hash.
pub fn ast_hash(module: &Module) -> conclave_hash::Hash {
    let v = serde_json::to_value(module).expect("Module always serializable");
    let canonical = conclave_hash::to_canonical_json(&v);
    conclave_hash::sha256_str(&canonical)
}
