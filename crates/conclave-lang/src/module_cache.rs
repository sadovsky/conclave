/// Content-addressed module cache.
///
/// Stores canonical Plan IR JSON files indexed by their `plan_ir_hash`.
///
/// Layout: `<root>/<sha256_hex>.json`
/// where `sha256_hex` is the 64-char hex string (no `sha256:` prefix) of the
/// canonical Plan IR.
///
/// The caller (usually the CLI) provides the root directory.
/// Platform-appropriate defaults:
/// - macOS: `~/Library/Caches/conclave/modules/`
/// - Linux: `~/.cache/conclave/modules/`
use conclave_ir::{canonicalize_plan_ir, compute_plan_ir_hash, PlanIr};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ModuleCacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("module not found in cache: {0}")]
    NotFound(String),
}

pub struct ModuleCache {
    pub root: PathBuf,
}

impl ModuleCache {
    pub fn new(root: PathBuf) -> Self {
        ModuleCache { root }
    }

    /// Look up a Plan IR by hash. Returns `None` if not in cache.
    ///
    /// Accepts both `sha256:<hex>` and bare `<hex>` formats.
    pub fn get(&self, hash: &str) -> Option<PlanIr> {
        let hex = hash.strip_prefix("sha256:").unwrap_or(hash);
        let path = self.root.join(format!("{hex}.json"));
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Look up a Plan IR by hash, returning an error if not found.
    pub fn require(&self, hash: &str) -> Result<PlanIr, ModuleCacheError> {
        self.get(hash)
            .ok_or_else(|| ModuleCacheError::NotFound(hash.to_string()))
    }

    /// Store a Plan IR in the cache.
    ///
    /// Serializes to canonical JSON, writes to `<root>/<sha256_hex>.json`, and
    /// returns the `plan_ir_hash` (`sha256:<hex>`).
    pub fn put(&self, plan_ir: &PlanIr) -> Result<String, ModuleCacheError> {
        let hash = compute_plan_ir_hash(plan_ir).to_string();
        let hex = hash.strip_prefix("sha256:").unwrap_or(&hash);
        std::fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{hex}.json"));
        let canonical = canonicalize_plan_ir(plan_ir);
        let json = conclave_hash::to_canonical_json(&canonical);
        std::fs::write(&path, json.as_bytes())?;
        Ok(hash)
    }

    /// List all cached modules sorted by hash.
    ///
    /// Returns `(plan_ir_hash, preview)` pairs where `preview` is the first
    /// 60 chars of the canonical JSON (useful for `conclave module list`).
    pub fn list(&self) -> Vec<(String, String)> {
        let mut entries = Vec::new();
        let dir = match std::fs::read_dir(&self.root) {
            Ok(d) => d,
            Err(_) => return entries,
        };
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let hash = format!("sha256:{stem}");
                    let preview = std::fs::read_to_string(&path)
                        .unwrap_or_default()
                        .chars()
                        .take(60)
                        .collect::<String>();
                    entries.push((hash, preview));
                }
            }
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }
}
