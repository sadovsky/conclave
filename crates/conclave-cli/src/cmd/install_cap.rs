use std::path::PathBuf;

#[derive(clap::Args)]
pub struct InstallCapArgs {
    /// Path to the capability binary to install.
    #[arg(value_name = "CAPABILITY_BINARY")]
    pub path: PathBuf,
    /// Capability store directory (default: ~/.cache/conclave/caps).
    #[arg(long, value_name = "STORE_DIR")]
    pub store: Option<PathBuf>,
}

pub fn run(args: InstallCapArgs) -> anyhow::Result<()> {
    let bytes = std::fs::read(&args.path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", args.path.display()))?;

    let store_dir = args.store.unwrap_or_else(default_store_dir);
    let store = conclave_store::FilesystemStore::new(store_dir.clone());

    let artifact_hash = store
        .install(&bytes)
        .map_err(|e| anyhow::anyhow!("failed to install capability: {e}"))?;

    eprintln!("installed: {artifact_hash}");
    eprintln!("store:     {}", store_dir.display());
    println!("{artifact_hash}");
    Ok(())
}

pub fn default_store_dir() -> PathBuf {
    dirs_next::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("conclave")
        .join("caps")
}
