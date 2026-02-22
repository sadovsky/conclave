use std::path::PathBuf;

#[derive(clap::Args)]
pub struct PlanArgs {
    /// Input Plan IR JSON file (already parsed/normalized; v0.1 has no source compiler).
    #[arg(value_name = "PLAN_IR_JSON")]
    pub input: PathBuf,
    /// Output canonical plan_ir.json (default: stdout).
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

pub fn run(args: PlanArgs) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(&args.input)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", args.input.display(), e))?;
    let ir: conclave_ir::PlanIr = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("failed to parse Plan IR: {e}"))?;

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
