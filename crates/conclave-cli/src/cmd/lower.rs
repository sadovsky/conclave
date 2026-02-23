use std::path::PathBuf;

#[derive(clap::Args)]
pub struct LowerArgs {
    /// Input `.conclave` source file.
    #[arg(value_name = "SOURCE")]
    pub input: PathBuf,

    /// Number of URLs to expand map/reduce constructs over (required for map/reduce lowering).
    #[arg(long, value_name = "N", default_value = "1")]
    pub url_count: usize,

    /// Lower only the named goal (default: lower the first goal).
    /// Use `--goal all` to lower every goal and emit a JSON array.
    #[arg(long, value_name = "NAME")]
    pub goal: Option<String>,

    /// Output canonical plan_ir.json (default: stdout).
    #[arg(long, short)]
    pub output: Option<PathBuf>,
}

pub fn run(args: LowerArgs) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(&args.input)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", args.input.display(), e))?;

    match args.goal.as_deref() {
        Some("all") => run_all(&source, args.url_count, args.output),
        Some(name) => run_named(&source, name, args.url_count, args.output),
        None => run_first(&source, args.url_count, args.output),
    }
}

fn run_first(source: &str, url_count: usize, output: Option<PathBuf>) -> anyhow::Result<()> {
    let out = conclave_lang::lower(source, url_count)
        .map_err(|e| anyhow::anyhow!("lowering failed: {e}"))?;

    conclave_ir::validate_plan_ir(&out.plan_ir)
        .map_err(|e| anyhow::anyhow!("Plan IR validation failed: {e}"))?;

    eprintln!("source_hash:  {}", out.source_hash);
    eprintln!("ast_hash:     {}", out.ast_hash);
    eprintln!("plan_ir_hash: {}", out.plan_ir_hash);

    emit_plan_ir(&out.plan_ir, output)
}

fn run_named(source: &str, goal_name: &str, url_count: usize, output: Option<PathBuf>) -> anyhow::Result<()> {
    let out = conclave_lang::lower_named(source, goal_name, url_count)
        .map_err(|e| anyhow::anyhow!("lowering failed: {e}"))?;

    conclave_ir::validate_plan_ir(&out.plan_ir)
        .map_err(|e| anyhow::anyhow!("Plan IR validation failed: {e}"))?;

    eprintln!("goal:         {}", goal_name);
    eprintln!("source_hash:  {}", out.source_hash);
    eprintln!("ast_hash:     {}", out.ast_hash);
    eprintln!("plan_ir_hash: {}", out.plan_ir_hash);

    emit_plan_ir(&out.plan_ir, output)
}

fn run_all(source: &str, url_count: usize, output: Option<PathBuf>) -> anyhow::Result<()> {
    let outputs = conclave_lang::lower_all(source, url_count)
        .map_err(|e| anyhow::anyhow!("lowering failed: {e}"))?;

    let mut canonical_list: Vec<serde_json::Value> = Vec::new();

    for out in &outputs {
        conclave_ir::validate_plan_ir(&out.plan_ir)
            .map_err(|e| anyhow::anyhow!("Plan IR validation failed: {e}"))?;

        eprintln!("goal:         {}", out.plan_ir.module.name);
        eprintln!("plan_ir_hash: {}", out.plan_ir_hash);

        let canonical = conclave_ir::canonicalize_plan_ir(&out.plan_ir);
        canonical_list.push(serde_json::from_str(&conclave_hash::to_canonical_json(&canonical))
            .expect("canonical JSON is valid"));
    }

    let json_str = serde_json::to_string_pretty(&canonical_list)
        .map_err(|e| anyhow::anyhow!("JSON serialization failed: {e}"))?;

    match output {
        Some(path) => std::fs::write(&path, &json_str)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))?,
        None => print!("{json_str}"),
    }
    Ok(())
}

fn emit_plan_ir(plan_ir: &conclave_ir::PlanIr, output: Option<PathBuf>) -> anyhow::Result<()> {
    let canonical = conclave_ir::canonicalize_plan_ir(plan_ir);
    let json_str = conclave_hash::to_canonical_json(&canonical);

    match output {
        Some(path) => std::fs::write(&path, &json_str)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", path.display()))?,
        None => print!("{json_str}"),
    }
    Ok(())
}
