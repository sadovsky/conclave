use std::path::PathBuf;

#[derive(clap::Args)]
pub struct PackArgs {
    /// Path to the conclave-runtime binary.
    #[arg(long, value_name = "RUNTIME_BINARY")]
    pub runtime: PathBuf,
    /// Sealed manifest.json (produced by `conclave seal`).
    #[arg(long, value_name = "MANIFEST_JSON")]
    pub manifest: PathBuf,
    /// Canonical plan_ir.json (produced by `conclave plan`).
    #[arg(long, value_name = "PLAN_IR_JSON")]
    pub plan: PathBuf,
    /// Output artifact path.
    #[arg(long, short, value_name = "OUTPUT")]
    pub output: PathBuf,
}

pub fn run(args: PackArgs) -> anyhow::Result<()> {
    let runtime_bytes = std::fs::read(&args.runtime)
        .map_err(|e| anyhow::anyhow!("failed to read runtime: {e}"))?;

    let manifest_raw = std::fs::read_to_string(&args.manifest)
        .map_err(|e| anyhow::anyhow!("failed to read manifest: {e}"))?;
    let manifest: conclave_manifest::Manifest = serde_json::from_str(&manifest_raw)
        .map_err(|e| anyhow::anyhow!("failed to parse manifest: {e}"))?;

    let plan_raw = std::fs::read_to_string(&args.plan)
        .map_err(|e| anyhow::anyhow!("failed to read plan: {e}"))?;
    let plan_ir: conclave_ir::PlanIr = serde_json::from_str(&plan_raw)
        .map_err(|e| anyhow::anyhow!("failed to parse Plan IR: {e}"))?;

    let bundle = conclave_pack::Bundle {
        bundle_version: "0.1".into(),
        manifest,
        plan_ir,
        embedded_artifacts: None,
        bundle_hashes: conclave_pack::BundleHashes {
            canonical_manifest_hash: String::new(),
            plan_ir_hash: String::new(),
            bundle_hash: String::new(),
        },
    };

    let result = conclave_pack::pack(conclave_pack::PackInput { runtime_bytes, bundle })
        .map_err(|e| anyhow::anyhow!("pack failed: {e}"))?;

    std::fs::write(&args.output, &result.artifact_bytes)
        .map_err(|e| anyhow::anyhow!("failed to write artifact: {e}"))?;

    eprintln!("artifact_hash: {}", result.artifact_hash);
    eprintln!("bundle_hash:   {}", result.bundle_hash);
    eprintln!("artifact written to: {}", args.output.display());
    Ok(())
}
