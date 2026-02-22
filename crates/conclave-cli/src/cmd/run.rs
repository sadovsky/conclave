use std::path::PathBuf;

#[derive(clap::Args)]
pub struct RunArgs {
    /// Conclave artifact (produced by `conclave pack`).
    #[arg(value_name = "ARTIFACT")]
    pub artifact: PathBuf,
    /// Optional replay store JSON (maps "capability::key" -> {output, duration_ms}).
    #[arg(long, value_name = "REPLAY_STORE_JSON")]
    pub replay: Option<PathBuf>,
    /// Emit scheduler trace to this file (JSON array of events).
    #[arg(long, value_name = "TRACE_OUT")]
    pub trace_out: Option<PathBuf>,
}

pub fn run(args: RunArgs) -> anyhow::Result<()> {
    let artifact_bytes = std::fs::read(&args.artifact)
        .map_err(|e| anyhow::anyhow!("failed to read artifact: {e}"))?;

    let bundle = conclave_pack::unpack(&artifact_bytes)
        .map_err(|e| anyhow::anyhow!("artifact verification failed: {e}"))?;

    // Build replay store.
    let mut store = conclave_runtime::MapReplayStore::new();
    if let Some(replay_path) = &args.replay {
        let raw = std::fs::read_to_string(replay_path)
            .map_err(|e| anyhow::anyhow!("failed to read replay store: {e}"))?;
        let map: std::collections::BTreeMap<String, serde_json::Value> =
            serde_json::from_str(&raw)
                .map_err(|e| anyhow::anyhow!("failed to parse replay store: {e}"))?;
        for (key, entry) in &map {
            let parts: Vec<&str> = key.splitn(2, "::").collect();
            if parts.len() != 2 {
                anyhow::bail!("invalid replay store key: {key}");
            }
            let capability = parts[0];
            let node_key = parts[1];
            let data = entry["output"].as_str().unwrap_or("").as_bytes().to_vec();
            let duration = entry["duration_ms"].as_u64().unwrap_or(0);
            store.insert(capability, node_key, data, "Any", duration);
        }
    }

    let policy = bundle.manifest.scheduler_policy.clone();
    let mut scheduler = conclave_runtime::Scheduler::new(policy);
    let mut trace = conclave_runtime::TraceEmitter::new();

    let outputs = scheduler
        .run(&bundle.plan_ir, &store, &mut trace)
        .map_err(|e| anyhow::anyhow!("runtime error: {e}"))?;

    eprintln!("trace_hash: {}", trace.trace_hash());

    if let Some(trace_path) = &args.trace_out {
        std::fs::write(trace_path, trace.to_canonical_json())
            .map_err(|e| anyhow::anyhow!("failed to write trace: {e}"))?;
    }

    eprintln!("completed nodes: {}", outputs.len());
    Ok(())
}
