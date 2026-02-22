use conclave_store::*;

#[test]
fn filesystem_store_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let store = FilesystemStore::new(dir.path().to_path_buf());

    let bytes = b"#!/bin/sh\necho hello";
    let hash = store.install(bytes).unwrap();

    assert!(hash.starts_with("sha256:"));
    let retrieved = store.get(&hash).unwrap();
    assert_eq!(retrieved, bytes);
}

#[test]
fn filesystem_store_miss_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let store = FilesystemStore::new(dir.path().to_path_buf());
    assert!(store.get("sha256:deadbeef").is_none());
}

#[test]
fn filesystem_store_hash_verification() {
    let bytes = b"capability binary";
    let hash = conclave_hash::sha256_bytes(bytes).to_string();
    verify_hash(&hash, bytes).unwrap();

    let result = verify_hash(&hash, b"wrong bytes");
    assert!(result.is_err());
}

#[test]
fn embedded_store_empty_bundle() {
    use conclave_ir::*;
    use conclave_manifest::*;
    use conclave_pack::*;
    use std::collections::BTreeMap;

    let plan_ir = PlanIr {
        conclave_ir_version: "0.1".into(),
        module: Module {
            name: "test".into(),
            source_fingerprint: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        },
        types: BTreeMap::new(),
        goals: vec![],
        nodes: vec![],
        edges: vec![],
        constraints: BTreeMap::new(),
        subgraphs: vec![],
        exports: Exports { entry_goal: "".into() },
    };

    let manifest = Manifest {
        conclave_manifest_version: "0.1".into(),
        program: Program {
            name: "test".into(),
            plan_ir_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        },
        target: Target { triple: "x86_64-unknown-linux-gnu".into(), os: "linux".into(), arch: "x86_64".into() },
        toolchain: Toolchain {
            lowerer_hash: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            runtime_hash: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            stdlib_hash: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
        },
        capability_bindings: BTreeMap::new(),
        scheduler_policy: SchedulerPolicy {
            strategy: "bounded_parallel_map".into(),
            max_inflight: 2,
            ready_queue_order: vec![],
            node_kind_order: vec![],
            tie_breaker: TieBreaker { kind: "stable".into(), seed: 0 },
        },
        determinism: Determinism {
            mode: "sealed_replay".into(),
            clock: "virtual".into(),
            randomness: RandomnessPolicy { allowed: false, seed: 0, source: "none".into() },
            float: "strict".into(),
            io_policy: IoPolicy {
                network: NetworkPolicy::ReplayOnly,
                filesystem: FilesystemPolicy::Sandboxed,
                env: EnvPolicy::Frozen,
            },
        },
        observability: Observability {
            trace_level: "deterministic".into(),
            emit_scheduler_trace: false,
            emit_capability_metrics: false,
        },
        supply_chain: SupplyChain {
            artifact_store: "content_addressed".into(),
            require_artifact_signatures: false,
            manifest_signature: None,
        },
    };

    let bundle = Bundle {
        bundle_version: "0.1".into(),
        manifest,
        plan_ir,
        embedded_artifacts: None,
        bundle_hashes: BundleHashes {
            canonical_manifest_hash: String::new(),
            plan_ir_hash: String::new(),
            bundle_hash: String::new(),
        },
    };

    let store = EmbeddedStore::from_bundle(&bundle).unwrap();
    assert!(store.is_empty());
    assert!(store.get("sha256:anything").is_none());
}

#[test]
fn chained_store_prefers_primary() {
    let dir1 = tempfile::tempdir().unwrap();
    let dir2 = tempfile::tempdir().unwrap();
    let s1 = FilesystemStore::new(dir1.path().to_path_buf());
    let s2 = FilesystemStore::new(dir2.path().to_path_buf());

    let bytes_a = b"capability-a";
    let bytes_b = b"capability-b";
    let hash_a = s1.install(bytes_a).unwrap();
    let hash_b = s2.install(bytes_b).unwrap();

    let chain = ChainedStore::new(s1, s2);

    // hash_a is in primary → returns from primary
    assert_eq!(chain.get(&hash_a).unwrap(), bytes_a);
    // hash_b is only in fallback → still found via chain
    assert_eq!(chain.get(&hash_b).unwrap(), bytes_b);
    // unknown hash → None
    assert!(chain.get("sha256:00000000000000000000000000000000000000000000000000000000000000ff").is_none());
}

#[test]
fn empty_cap_store_always_misses() {
    let store = EmptyCapStore;
    assert!(store.get("sha256:anything").is_none());
}
