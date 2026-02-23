use conclave_ir::*;
use conclave_manifest::*;
use conclave_seal::{seal, SealInput};
use std::collections::BTreeMap;

fn fixture_ir() -> PlanIr {
    PlanIr {
        conclave_ir_version: "0.1".into(),
        module: Module {
            name: "seal_test".into(),
            source_fingerprint:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        },
        imports: BTreeMap::new(),
        types: BTreeMap::new(),
        goals: vec![],
        nodes: vec![],
        edges: vec![],
        constraints: BTreeMap::new(),
        subgraphs: vec![],
        exports: Exports {
            entry_goal: "gid:entry".into(),
        },
    }
}

fn fixture_manifest_template(plan_ir_hash: &str) -> Manifest {
    Manifest {
        conclave_manifest_version: "0.1".into(),
        program: Program {
            name: "seal_test".into(),
            plan_ir_hash: plan_ir_hash.into(),
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
        module_bindings: BTreeMap::new(),
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
    }
}

#[test]
fn seal_fills_in_empty_plan_ir_hash() {
    let ir = fixture_ir();
    let manifest = fixture_manifest_template("");
    let output = seal(SealInput {
        plan_ir: ir,
        manifest,
    })
    .unwrap();
    assert!(output.manifest.program.plan_ir_hash.starts_with("sha256:"));
    assert_eq!(
        output.manifest.program.plan_ir_hash,
        output.plan_ir_hash.to_string()
    );
}

#[test]
fn seal_is_deterministic() {
    let ir = fixture_ir();

    let out1 = seal(SealInput {
        plan_ir: ir.clone(),
        manifest: fixture_manifest_template(""),
    })
    .unwrap();

    let out2 = seal(SealInput {
        plan_ir: ir,
        manifest: fixture_manifest_template(""),
    })
    .unwrap();

    // Same plan IR hash.
    assert_eq!(out1.plan_ir_hash, out2.plan_ir_hash);
    // Same canonical manifest hash.
    assert_eq!(out1.canonical_manifest_hash, out2.canonical_manifest_hash);
    // Same plan_ir_hash embedded in manifest.
    assert_eq!(
        out1.manifest.program.plan_ir_hash,
        out2.manifest.program.plan_ir_hash
    );
}

#[test]
fn seal_rejects_mismatched_plan_ir_hash() {
    let ir = fixture_ir();
    let manifest = fixture_manifest_template(
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let result = seal(SealInput {
        plan_ir: ir,
        manifest,
    });
    assert!(matches!(
        result,
        Err(conclave_manifest::SealError::PlanIrHashMismatch { .. })
    ));
}

#[test]
fn seal_accepts_correct_pre_set_plan_ir_hash() {
    let ir = fixture_ir();
    // Pre-compute the correct hash.
    let correct_hash = conclave_ir::compute_plan_ir_hash(&ir).to_string();
    let manifest = fixture_manifest_template(&correct_hash);
    let result = seal(SealInput {
        plan_ir: ir,
        manifest,
    });
    assert!(result.is_ok());
}
