use std::path::PathBuf;

#[derive(clap::Args)]
pub struct LowerArgs {
    /// Input `.conclave` source file.
    #[arg(value_name = "SOURCE")]
    pub input: PathBuf,

    /// Number of URLs to expand map constructs over (required for map lowering).
    #[arg(long, value_name = "N", default_value = "1")]
    pub url_count: usize,

    /// Output canonical plan_ir.json (default: stdout).
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

pub fn run(args: LowerArgs) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(&args.input)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", args.input.display(), e))?;

    let out = conclave_lang::lower(&source, args.url_count)
        .map_err(|e| anyhow::anyhow!("lowering failed: {e}"))?;

    conclave_ir::validate_plan_ir(&out.plan_ir)
        .map_err(|e| anyhow::anyhow!("Plan IR validation failed: {e}"))?;

    eprintln!("source_hash:  {}", out.source_hash);
    eprintln!("ast_hash:     {}", out.ast_hash);
    eprintln!("plan_ir_hash: {}", out.plan_ir_hash);

    let canonical = conclave_ir::canonicalize_plan_ir(&out.plan_ir);
    let json_str = conclave_hash::to_canonical_json(&canonical);

    match args.output {
        Some(path) => std::fs::write(&path, &json_str)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))?,
        None => print!("{json_str}"),
    }
    Ok(())
}
