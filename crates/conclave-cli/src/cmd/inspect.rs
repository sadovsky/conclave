use std::path::PathBuf;

#[derive(clap::Args)]
pub struct InspectArgs {
    /// Conclave artifact to inspect.
    #[arg(value_name = "ARTIFACT")]
    pub artifact: PathBuf,
}

pub fn run(args: InspectArgs) -> anyhow::Result<()> {
    let artifact_bytes = std::fs::read(&args.artifact)
        .map_err(|e| anyhow::anyhow!("failed to read artifact: {e}"))?;

    let bundle = conclave_pack::unpack(&artifact_bytes)
        .map_err(|e| anyhow::anyhow!("artifact verification failed: {e}"))?;

    println!("bundle_version:          {}", bundle.bundle_version);
    println!("program:                 {}", bundle.manifest.program.name);
    println!("plan_ir_hash:            {}", bundle.bundle_hashes.plan_ir_hash);
    println!("canonical_manifest_hash: {}", bundle.bundle_hashes.canonical_manifest_hash);
    println!("bundle_hash:             {}", bundle.bundle_hashes.bundle_hash);
    println!("determinism.mode:        {}", bundle.manifest.determinism.mode);
    println!("determinism.clock:       {}", bundle.manifest.determinism.clock);
    println!("capability_bindings:");
    for (sig, binding) in &bundle.manifest.capability_bindings {
        println!("  {sig}");
        println!("    artifact_hash: {}", binding.artifact_hash);
        println!("    profile:       {}", binding.determinism_profile);
    }
    Ok(())
}
