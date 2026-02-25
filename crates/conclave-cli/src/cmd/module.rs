use clap::Subcommand;
use conclave_lang::ModuleCache;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Top-level: `conclave module <subcommand>`
// ---------------------------------------------------------------------------

#[derive(clap::Args)]
pub struct ModuleArgs {
    #[command(subcommand)]
    pub command: ModuleCommands,
}

#[derive(Subcommand)]
pub enum ModuleCommands {
    /// Lower a goal and publish it to the local module cache. Prints the plan_ir_hash.
    Publish(PublishArgs),
    /// List all modules in the local module cache.
    List(ListArgs),
    /// Print the Plan IR for a cached module.
    Inspect(ModuleInspectArgs),
}

pub fn run(args: ModuleArgs) -> anyhow::Result<()> {
    match args.command {
        ModuleCommands::Publish(a) => publish(a),
        ModuleCommands::List(a) => list(a),
        ModuleCommands::Inspect(a) => inspect(a),
    }
}

// ---------------------------------------------------------------------------
// conclave module publish
// ---------------------------------------------------------------------------

#[derive(clap::Args)]
pub struct PublishArgs {
    /// Input `.conclave` source file.
    #[arg(value_name = "SOURCE")]
    pub input: PathBuf,

    /// Number of URLs to expand map constructs over (required for map lowering).
    #[arg(long, value_name = "N", default_value = "1")]
    pub url_count: usize,

    /// Module cache directory (default: platform cache dir / conclave / modules).
    #[arg(long, value_name = "DIR")]
    pub module_cache: Option<PathBuf>,
}

fn publish(args: PublishArgs) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(&args.input)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", args.input.display()))?;

    let out = conclave_lang::lower(&source, args.url_count)
        .map_err(|e| anyhow::anyhow!("lowering failed: {e}"))?;

    conclave_ir::validate_plan_ir(&out.plan_ir)
        .map_err(|e| anyhow::anyhow!("Plan IR validation failed: {e}"))?;

    let cache = ModuleCache::new(args.module_cache.unwrap_or_else(default_module_cache_dir));
    let hash = cache
        .put(&out.plan_ir)
        .map_err(|e| anyhow::anyhow!("failed to write to module cache: {e}"))?;

    eprintln!("published: {hash}");
    eprintln!("cache:     {}", cache.root.display());
    println!("{hash}");
    Ok(())
}

// ---------------------------------------------------------------------------
// conclave module list
// ---------------------------------------------------------------------------

#[derive(clap::Args)]
pub struct ListArgs {
    /// Module cache directory (default: platform cache dir / conclave / modules).
    #[arg(long, value_name = "DIR")]
    pub module_cache: Option<PathBuf>,
}

fn list(args: ListArgs) -> anyhow::Result<()> {
    let cache = ModuleCache::new(args.module_cache.unwrap_or_else(default_module_cache_dir));
    let entries = cache.list();
    if entries.is_empty() {
        eprintln!("(no modules cached at {})", cache.root.display());
    } else {
        for (hash, preview) in &entries {
            println!("{hash}  {preview}...");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// conclave module inspect
// ---------------------------------------------------------------------------

#[derive(clap::Args)]
pub struct ModuleInspectArgs {
    /// The plan_ir_hash of the module to inspect (sha256:<64 hex chars>).
    #[arg(value_name = "HASH")]
    pub hash: String,

    /// Module cache directory (default: platform cache dir / conclave / modules).
    #[arg(long, value_name = "DIR")]
    pub module_cache: Option<PathBuf>,
}

fn inspect(args: ModuleInspectArgs) -> anyhow::Result<()> {
    let cache = ModuleCache::new(args.module_cache.unwrap_or_else(default_module_cache_dir));
    let plan_ir = cache
        .require(&args.hash)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Print Plan IR as canonical JSON.
    let canonical = conclave_ir::canonicalize_plan_ir(&plan_ir);
    let json = conclave_hash::to_canonical_json(&canonical);
    println!("{json}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn default_module_cache_dir() -> PathBuf {
    dirs_next::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("conclave")
        .join("modules")
}
