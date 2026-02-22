use conclave_ir::*;
use conclave_manifest::*;
use std::collections::BTreeMap;

fn fixture_manifest() -> Manifest {
    let mut bindings = BTreeMap::new();
    bindings.insert(
        "fetch(Url)->Html".into(),
        CapabilityBinding {
            capability_name: "fetch".into(),
            artifact_hash:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            determinism_profile: "replayable".into(),
            trust: "sandboxed_network_only".into(),
            config: Some({
                let mut c = BTreeMap::new();
                c.insert("fetch_mode".into(), serde_json::json!("replay"));
                c.insert(
                    "replay_store_hash".into(),
                    serde_json::json!(
                        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    ),
                );
                c
            }),
            signatures: None,
        },
    );

    Manifest {
        conclave_manifest_version: "0.1".into(),
        program: Program {
            name: "test_program".into(),
            plan_ir_hash: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                .into(),
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
        capability_bindings: bindings,
        scheduler_policy: SchedulerPolicy {
            strategy: "bounded_parallel_map".into(),
            max_inflight: 2,
            ready_queue_order: vec!["url_index".into(), "node_kind".into(), "node_id".into()],
            node_kind_order: vec![
                "FETCH".into(),
                "EXTRACT".into(),
                "SUMMARIZE".into(),
                "ASSEMBLE".into(),
            ],
            tie_breaker: TieBreaker {
                kind: "stable".into(),
                seed: 0,
            },
        },
        determinism: Determinism {
            mode: "sealed_replay".into(),
            clock: "virtual".into(),
            randomness: RandomnessPolicy {
                allowed: true,
                seed: 1337,
                source: "ctr_drbg".into(),
            },
            float: "strict".into(),
            io_policy: IoPolicy {
                network: NetworkPolicy::ReplayOnly,
                filesystem: FilesystemPolicy::Sandboxed,
                env: EnvPolicy::Frozen,
            },
        },
        observability: Observability {
            trace_level: "deterministic".into(),
            emit_scheduler_trace: true,
            emit_capability_metrics: true,
        },
        supply_chain: SupplyChain {
            artifact_store: "content_addressed".into(),
            require_artifact_signatures: false,
            manifest_signature: None,
        },
    }
}

fn fixture_plan_ir() -> PlanIr {
    // A minimal Plan IR with one capability_call node matching the manifest's binding.
    PlanIr {
        conclave_ir_version: "0.1".into(),
        module: Module {
            name: "test".into(),
            source_fingerprint:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        },
        types: BTreeMap::new(),
        goals: vec![],
        nodes: vec![Node {
            node_id: "nid:f1".into(),
            kind: NodeKind::CapabilityCall,
            op: Op {
                name: "fetch".into(),
                signature: "fetch(Url)->Html".into(),
            },
            inputs: vec![],
            outputs: vec![],
            attrs: NodeAttrs {
                determinism_profile: DeterminismProfile::Replayable,
                cost_hints: None,
                url_index: Some(0),
            },
            constraints: vec![],
            meta: None,
        }],
        edges: vec![],
        constraints: BTreeMap::new(),
        subgraphs: vec![],
        exports: Exports {
            entry_goal: "gid:entry".into(),
        },
    }
}

#[test]
fn seal_ok_with_valid_fixture() {
    assert!(validate_seal(&fixture_manifest(), &fixture_plan_ir()).is_ok());
}

#[test]
fn seal_rejects_empty_plan_ir_hash() {
    let mut m = fixture_manifest();
    m.program.plan_ir_hash = String::new();
    assert!(matches!(
        validate_seal(&m, &fixture_plan_ir()),
        Err(SealError::MissingPlanIrHash)
    ));
}

#[test]
fn seal_rejects_non_hash_plan_ir_hash() {
    let mut m = fixture_manifest();
    m.program.plan_ir_hash = "latest".into();
    assert!(matches!(
        validate_seal(&m, &fixture_plan_ir()),
        Err(SealError::MissingPlanIrHash)
    ));
}

#[test]
fn seal_rejects_missing_capability_binding() {
    let m = fixture_manifest();
    let mut ir = fixture_plan_ir();
    // Add a cap_call node whose signature has no binding.
    ir.nodes.push(Node {
        node_id: "nid:x".into(),
        kind: NodeKind::CapabilityCall,
        op: Op {
            name: "unknown".into(),
            signature: "unknown(A)->B".into(),
        },
        inputs: vec![],
        outputs: vec![],
        attrs: NodeAttrs {
            determinism_profile: DeterminismProfile::Fixed,
            cost_hints: None,
            url_index: None,
        },
        constraints: vec![],
        meta: None,
    });
    assert!(matches!(
        validate_seal(&m, &ir),
        Err(SealError::MissingCapabilityBinding(_))
    ));
}

#[test]
fn seal_rejects_floating_artifact_hash() {
    let mut m = fixture_manifest();
    m.capability_bindings
        .get_mut("fetch(Url)->Html")
        .unwrap()
        .artifact_hash = "latest".into();
    assert!(matches!(
        validate_seal(&m, &fixture_plan_ir()),
        Err(SealError::FloatingCapabilityReference(_))
    ));
}

#[test]
fn seal_rejects_unpinned_toolchain() {
    let mut m = fixture_manifest();
    m.toolchain.lowerer_hash = String::new();
    assert!(matches!(
        validate_seal(&m, &fixture_plan_ir()),
        Err(SealError::UnpinnedToolchain)
    ));
}

#[test]
fn seal_rejects_non_virtual_clock() {
    let mut m = fixture_manifest();
    m.determinism.clock = "wall".into();
    assert!(matches!(
        validate_seal(&m, &fixture_plan_ir()),
        Err(SealError::ClockNotVirtual)
    ));
}

#[test]
fn seal_rejects_network_cap_not_replay_in_sealed_replay_mode() {
    let mut m = fixture_manifest();
    // Remove fetch_mode from config so it's no longer replay.
    m.capability_bindings
        .get_mut("fetch(Url)->Html")
        .unwrap()
        .config
        .as_mut()
        .unwrap()
        .remove("fetch_mode");
    assert!(matches!(
        validate_seal(&m, &fixture_plan_ir()),
        Err(SealError::NetworkCapabilityNotReplay(_))
    ));
}

#[test]
fn seal_rejects_signatures_required_with_no_accepted_keys() {
    let mut m = fixture_manifest();
    let sig = "fetch(Url)->Html";
    m.capability_bindings.insert(
        sig.into(),
        CapabilityBinding {
            capability_name: "fetch".into(),
            artifact_hash:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            determinism_profile: "replayable".into(),
            trust: "deterministic".into(),
            config: None,
            signatures: Some(CapabilitySignatures {
                required: true,
                accepted_keys: vec![], // empty — should be rejected
            }),
        },
    );
    assert!(matches!(
        validate_seal(&m, &fixture_plan_ir()),
        Err(SealError::SignatureRequiredButNoKeys(_))
    ));
}
