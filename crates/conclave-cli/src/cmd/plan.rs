use std::path::PathBuf;

#[derive(clap::Args)]
pub struct PlanArgs {
    /// Input Plan IR JSON file or `.conclave` source file.
    /// `.conclave` files are lowered automatically; use `conclave lower` for explicit control.
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,
    /// Output canonical plan_ir.json (default: stdout).
    #[arg(long, short)]
    pub output: Option<PathBuf>,
    /// URL count for `.conclave` source lowering (ignored for JSON inputs).
    #[arg(long, value_name = "N", default_value = "1")]
    pub url_count: usize,
}

pub fn run(args: PlanArgs) -> anyhow::Result<()> {
    let ir = load_plan_ir(&args.input, args.url_count)?;

    conclave_ir::validate_plan_ir(&ir)
        .map_err(|e| anyhow::anyhow!("Plan IR validation failed: {e}"))?;

    let plan_ir_hash = conclave_ir::compute_plan_ir_hash(&ir);
    eprintln!("plan_ir_hash: {plan_ir_hash}");

    let canonical = conclave_ir::canonicalize_plan_ir(&ir);
    let canonical_str = conclave_hash::to_canonical_json(&canonical);

    match args.output {
        Some(path) => std::fs::write(&path, &canonical_str)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))?,
        None => print!("{canonical_str}"),
    }
    Ok(())
}

/// Load a Plan IR from either a `.conclave` source file or a JSON Plan IR file.
/// Exported so the `seal` command can also accept `.conclave` files.
pub fn load_plan_ir(path: &PathBuf, url_count: usize) -> anyhow::Result<conclave_ir::PlanIr> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "conclave" {
        let source = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
        let out = conclave_lang::lower(&source, url_count)
            .map_err(|e| anyhow::anyhow!("lowering failed: {e}"))?;
        eprintln!("source_hash:  {}", out.source_hash);
        eprintln!("ast_hash:     {}", out.ast_hash);
        Ok(out.plan_ir)
    } else {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
        serde_json::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("failed to parse Plan IR JSON: {e}"))
    }
}
