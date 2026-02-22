use std::path::PathBuf;

#[derive(clap::Args)]
pub struct SealArgs {
    /// Canonical plan_ir.json (produced by `conclave plan`).
    #[arg(long, value_name = "PLAN_IR_JSON")]
    pub plan: PathBuf,
    /// Manifest template JSON (partially filled; plan_ir_hash may be empty).
    #[arg(long, value_name = "MANIFEST_TEMPLATE_JSON")]
    pub manifest: PathBuf,
    /// Output canonical manifest.json (default: stdout).
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

pub fn run(args: SealArgs) -> anyhow::Result<()> {
    let plan_raw = std::fs::read_to_string(&args.plan)
        .map_err(|e| anyhow::anyhow!("failed to read plan: {e}"))?;
    let plan_ir: conclave_ir::PlanIr = serde_json::from_str(&plan_raw)
        .map_err(|e| anyhow::anyhow!("failed to parse Plan IR: {e}"))?;

    let manifest_raw = std::fs::read_to_string(&args.manifest)
        .map_err(|e| anyhow::anyhow!("failed to read manifest template: {e}"))?;
    let manifest: conclave_manifest::Manifest = serde_json::from_str(&manifest_raw)
        .map_err(|e| anyhow::anyhow!("failed to parse manifest template: {e}"))?;

    let output = conclave_seal::seal(conclave_seal::SealInput { plan_ir, manifest })
        .map_err(|e| anyhow::anyhow!("seal failed: {e}"))?;

    eprintln!("plan_ir_hash:            {}", output.plan_ir_hash);
    eprintln!("canonical_manifest_hash: {}", output.canonical_manifest_hash);

    let canonical_manifest = conclave_hash::to_canonical_json(
        &serde_json::to_value(&output.manifest)
            .map_err(|e| anyhow::anyhow!("failed to serialize manifest: {e}"))?,
    );

    match args.output {
        Some(path) => std::fs::write(&path, &canonical_manifest)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))?,
        None => print!("{canonical_manifest}"),
    }
    Ok(())
}
