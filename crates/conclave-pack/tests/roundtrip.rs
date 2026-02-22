use conclave_ir::*;
use conclave_manifest::*;
use conclave_pack::*;
use std::collections::BTreeMap;

fn fixture_bundle() -> Bundle {
    let ir = PlanIr {
        conclave_ir_version: "0.1".into(),
        module: Module {
            name: "pack_test".into(),
            source_fingerprint:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        },
        types: BTreeMap::new(),
        goals: vec![],
        nodes: vec![],
        edges: vec![],
        constraints: BTreeMap::new(),
        subgraphs: vec![],
        exports: Exports {
            entry_goal: "gid:entry".into(),
        },
    };

    let manifest = Manifest {
        conclave_manifest_version: "0.1".into(),
        program: Program {
            name: "pack_test".into(),
            plan_ir_hash: conclave_ir::compute_plan_ir_hash(&ir).to_string(),
        },
        target: Target {
            triple: "aarch64-apple-darwin".into(),
            os: "macos".into(),
            arch: "aarch64".into(),
        },
        toolchain: Toolchain {
            lowerer_hash: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                .into(),
            runtime_hash: "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                .into(),
            stdlib_hash: "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                .into(),
        },
        capability_bindings: BTreeMap::new(),
        scheduler_policy: SchedulerPolicy {
            strategy: "bounded_parallel_map".into(),
            max_inflight: 1,
            ready_queue_order: vec![],
            node_kind_order: vec![],
            tie_breaker: TieBreaker {
                kind: "stable".into(),
                seed: 0,
            },
        },
        determinism: Determinism {
            mode: "sealed_replay".into(),
            clock: "virtual".into(),
            randomness: RandomnessPolicy {
                allowed: false,
                seed: 0,
                source: "none".into(),
            },
            float: "strict".into(),
            io_policy: IoPolicy {
                network: NetworkPolicy::Deny,
                filesystem: FilesystemPolicy::Deny,
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

    Bundle {
        bundle_version: "0.1".into(),
        manifest,
        plan_ir: ir,
        embedded_artifacts: None,
        bundle_hashes: BundleHashes {
            canonical_manifest_hash: String::new(),
            plan_ir_hash: String::new(),
            bundle_hash: String::new(),
        },
    }
}

#[test]
fn pack_unpack_roundtrip() {
    let runtime_bytes = b"fake_runtime_binary".to_vec();
    let bundle = fixture_bundle();

    let output = pack(PackInput {
        runtime_bytes,
        bundle,
    })
    .unwrap();
    let recovered = unpack(&output.artifact_bytes).unwrap();

    assert_eq!(recovered.manifest.program.name, "pack_test");
    assert_eq!(recovered.bundle_version, "0.1");
    assert_eq!(
        recovered.bundle_hashes.bundle_hash,
        output.bundle_hash.to_string()
    );
}

#[test]
fn pack_is_deterministic() {
    let runtime_bytes = b"fake_runtime_binary".to_vec();

    let out1 = pack(PackInput {
        runtime_bytes: runtime_bytes.clone(),
        bundle: fixture_bundle(),
    })
    .unwrap();

    let out2 = pack(PackInput {
        runtime_bytes,
        bundle: fixture_bundle(),
    })
    .unwrap();

    assert_eq!(
        out1.artifact_bytes, out2.artifact_bytes,
        "pack must produce identical bytes given identical inputs"
    );
    assert_eq!(out1.artifact_hash, out2.artifact_hash);
    assert_eq!(out1.bundle_hash, out2.bundle_hash);
}

#[test]
fn unpack_rejects_bad_magic() {
    let mut artifact = b"some_runtime".to_vec();
    artifact.extend_from_slice(b"bundle_data");
    // Wrong magic.
    let bundle_len = 11u64.to_le_bytes();
    artifact.extend_from_slice(&bundle_len);
    artifact.extend_from_slice(b"BADMAGIC");
    assert!(matches!(
        unpack(&artifact),
        Err(PackError::ArtifactBadMagic)
    ));
}

#[test]
fn unpack_rejects_too_short() {
    let artifact = b"short".to_vec();
    assert!(matches!(
        unpack(&artifact),
        Err(PackError::ArtifactTruncated)
    ));
}
