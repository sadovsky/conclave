//! Conformance test for the deterministic scheduler golden trace.
//!
//! Scenario: SummarizeUrls([u1, u2, u3])
//! Per tests/conformance/v0.1.md:
//! - rate_limit(fetch) = 2 req/s (2 tokens per 1000ms window)
//! - max_inflight = 2
//! - ready_queue_order: (url_index, node_kind, node_id)
//! - node_kind_order: FETCH < EXTRACT < SUMMARIZE < ASSEMBLE
//! - replay durations: F1=40ms, F2=70ms, F3=40ms
//! - local durations: E*=15ms, S*=85ms, A=4ms (but for conformance, event ORDER is primary)

use conclave_ir::*;
use conclave_manifest::*;
use conclave_runtime::*;
use std::collections::BTreeMap;

/// Build the SummarizeUrls Plan IR with 3 URLs.
///
/// Nodes:
/// - F1, F2, F3: fetch (capability_call, url_index 0/1/2)
/// - E1, E2, E3: extract_text (intrinsic, depends on F1/F2/F3)
/// - S1, S2, S3: summarize (intrinsic, depends on E1/E2/E3)
/// - A: assemble_json (aggregate, depends on S1+S2+S3)
fn build_summarize_plan_ir() -> PlanIr {
    let fetch_sig = "fetch(Url)->Html";
    let extract_sig = "extract_text(Html)->String";
    let summarize_sig = "summarize(String)->Summary";
    let assemble_sig = "assemble_json";

    fn make_node(id: &str, kind: NodeKind, op_name: &str, sig: &str, url_idx: Option<u32>, inputs: Vec<InputPort>) -> Node {
        Node {
            node_id: id.into(),
            kind,
            op: Op { name: op_name.into(), signature: sig.into() },
            inputs,
            outputs: vec![OutputPort { port: "out".into(), type_name: "Any".into() }],
            attrs: NodeAttrs {
                determinism_profile: DeterminismProfile::Fixed,
                cost_hints: None,
                url_index: url_idx,
            },
            constraints: vec![],
            meta: None,
        }
    }

    fn edge(id: &str, from_node: &str, to_node: &str) -> Edge {
        Edge {
            edge_id: id.into(),
            from: EdgeEndpoint { node_id: from_node.into(), port: "out".into() },
            to: EdgeEndpoint { node_id: to_node.into(), port: "in".into() },
        }
    }

    fn dep_input(edge_id: &str) -> InputPort {
        InputPort {
            port: "in".into(),
            type_name: "Any".into(),
            source: Some(EdgeRef { edge_id: edge_id.into() }),
        }
    }

    let nodes = vec![
        // Fetch nodes
        make_node("F1", NodeKind::CapabilityCall, "fetch", fetch_sig, Some(0), vec![]),
        make_node("F2", NodeKind::CapabilityCall, "fetch", fetch_sig, Some(1), vec![]),
        make_node("F3", NodeKind::CapabilityCall, "fetch", fetch_sig, Some(2), vec![]),
        // Extract nodes
        make_node("E1", NodeKind::Intrinsic, "extract_text", extract_sig, Some(0), vec![dep_input("e_f1_e1")]),
        make_node("E2", NodeKind::Intrinsic, "extract_text", extract_sig, Some(1), vec![dep_input("e_f2_e2")]),
        make_node("E3", NodeKind::Intrinsic, "extract_text", extract_sig, Some(2), vec![dep_input("e_f3_e3")]),
        // Summarize nodes
        make_node("S1", NodeKind::Intrinsic, "summarize", summarize_sig, Some(0), vec![dep_input("e_e1_s1")]),
        make_node("S2", NodeKind::Intrinsic, "summarize", summarize_sig, Some(1), vec![dep_input("e_e2_s2")]),
        make_node("S3", NodeKind::Intrinsic, "summarize", summarize_sig, Some(2), vec![dep_input("e_e3_s3")]),
        // Assemble node
        make_node("A", NodeKind::Aggregate, "assemble_json", assemble_sig, None, vec![
            dep_input("e_s1_a"),
            dep_input("e_s2_a"),
            dep_input("e_s3_a"),
        ]),
    ];

    let edges = vec![
        edge("e_f1_e1", "F1", "E1"),
        edge("e_f2_e2", "F2", "E2"),
        edge("e_f3_e3", "F3", "E3"),
        edge("e_e1_s1", "E1", "S1"),
        edge("e_e2_s2", "E2", "S2"),
        edge("e_e3_s3", "E3", "S3"),
        edge("e_s1_a", "S1", "A"),
        edge("e_s2_a", "S2", "A"),
        edge("e_s3_a", "S3", "A"),
    ];

    PlanIr {
        conclave_ir_version: "0.1".into(),
        module: Module {
            name: "summarize_urls".into(),
            source_fingerprint:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        },
        types: BTreeMap::new(),
        goals: vec![],
        nodes,
        edges,
        constraints: BTreeMap::new(),
        subgraphs: vec![],
        exports: Exports { entry_goal: "gid:entry".into() },
    }
}

fn build_scheduler_policy() -> SchedulerPolicy {
    SchedulerPolicy {
        strategy: "bounded_parallel_map".into(),
        max_inflight: 2,
        ready_queue_order: vec!["url_index".into(), "node_kind".into(), "node_id".into()],
        node_kind_order: vec!["FETCH".into(), "EXTRACT".into(), "SUMMARIZE".into(), "ASSEMBLE".into()],
        tie_breaker: TieBreaker { kind: "stable".into(), seed: 0 },
    }
}

/// Build replay store with durations from conformance spec.
fn build_replay_store() -> MapReplayStore {
    let mut store = MapReplayStore::new();
    let fetch_sig = "fetch(Url)->Html";
    // Key is node_id (the stable normalized key used by our scheduler).
    store.insert(fetch_sig, "F1", b"<html>u1</html>".to_vec(), "Html", 40);
    store.insert(fetch_sig, "F2", b"<html>u2</html>".to_vec(), "Html", 70);
    store.insert(fetch_sig, "F3", b"<html>u3</html>".to_vec(), "Html", 40);
    store
}

/// Build replay store with only F1 and F3 (F2 missing — replay miss scenario).
fn build_replay_store_missing_f2() -> MapReplayStore {
    let mut store = MapReplayStore::new();
    let fetch_sig = "fetch(Url)->Html";
    store.insert(fetch_sig, "F1", b"<html>u1</html>".to_vec(), "Html", 40);
    store.insert(fetch_sig, "F3", b"<html>u3</html>".to_vec(), "Html", 40);
    store
}

/// Build a CapabilityDispatcher in sealed_replay mode wrapping a replay store.
fn make_dispatcher<'a>(
    store: &'a MapReplayStore,
    bindings: &'a BTreeMap<String, conclave_manifest::CapabilityBinding>,
) -> CapabilityDispatcher<'a> {
    CapabilityDispatcher {
        replay_store: store,
        cap_store: None,
        bindings,
        determinism_mode: "sealed_replay".into(),
        seed: 0,
        url_inputs: vec![],
    }
}

#[test]
fn conformance_dispatch_order() {
    let plan_ir = build_summarize_plan_ir();
    let policy = build_scheduler_policy();
    let store = build_replay_store();
    let bindings = BTreeMap::new();
    let dispatcher = make_dispatcher(&store, &bindings);
    let mut scheduler = Scheduler::new(policy);
    let mut trace = TraceEmitter::new();

    let _ = scheduler.run(&plan_ir, &dispatcher, &mut trace).unwrap();

    let dispatch_order: Vec<&str> = trace
        .events()
        .iter()
        .filter(|e| e.event == "DISPATCH")
        .map(|e| e.node.as_str())
        .collect();

    // Per conformance spec §1.2: F1, F2, E1, S1, E2, S2, F3, E3, S3, A
    let expected = vec!["F1", "F2", "E1", "S1", "E2", "S2", "F3", "E3", "S3", "A"];
    assert_eq!(
        dispatch_order, expected,
        "DISPATCH order must match golden trace"
    );
}

#[test]
fn conformance_f3_not_dispatched_before_window_1() {
    let plan_ir = build_summarize_plan_ir();
    let policy = build_scheduler_policy();
    let store = build_replay_store();
    let bindings = BTreeMap::new();
    let dispatcher = make_dispatcher(&store, &bindings);
    let mut scheduler = Scheduler::new(policy);
    let mut trace = TraceEmitter::new();

    let _ = scheduler.run(&plan_ir, &dispatcher, &mut trace).unwrap();

    // F3 must not dispatch before t=1000.
    let f3_dispatch = trace
        .events()
        .iter()
        .find(|e| e.event == "DISPATCH" && e.node == "F3")
        .expect("F3 must be dispatched");

    assert!(
        f3_dispatch.t >= 1000,
        "F3 must not dispatch before t=1000ms (rate window), got t={}",
        f3_dispatch.t
    );
}

#[test]
fn conformance_replay_miss_produces_deterministic_error() {
    let plan_ir = build_summarize_plan_ir();
    let policy = build_scheduler_policy();
    let store = build_replay_store_missing_f2();
    let bindings = BTreeMap::new();
    let dispatcher = make_dispatcher(&store, &bindings);
    let mut scheduler = Scheduler::new(policy);
    let mut trace = TraceEmitter::new();

    let result = scheduler.run(&plan_ir, &dispatcher, &mut trace);

    match result {
        Err(e) => {
            assert_eq!(e.code, "ERR_REPLAY_MISS", "error code must be ERR_REPLAY_MISS");
            assert_eq!(
                e.node_id.as_deref(),
                Some("F2"),
                "error node_id must be F2"
            );
        }
        Ok(_) => panic!("expected ERR_REPLAY_MISS but run succeeded"),
    }
}

#[test]
fn trace_is_deterministic() {
    let plan_ir = build_summarize_plan_ir();
    let policy = build_scheduler_policy();
    let store = build_replay_store();
    let bindings = BTreeMap::new();
    let d1 = make_dispatcher(&store, &bindings);
    let d2 = make_dispatcher(&store, &bindings);

    let mut s1 = Scheduler::new(policy.clone());
    let mut t1 = TraceEmitter::new();
    let _ = s1.run(&plan_ir, &d1, &mut t1).unwrap();

    let mut s2 = Scheduler::new(policy);
    let mut t2 = TraceEmitter::new();
    let _ = s2.run(&plan_ir, &d2, &mut t2).unwrap();

    assert_eq!(t1.events(), t2.events(), "trace must be identical across runs");
    assert_eq!(t1.trace_hash(), t2.trace_hash(), "trace_hash must be identical");
}

#[test]
fn rate_limiter_unit_window_reset() {
    let mut bucket = TokenBucket::new(1000, 2);
    assert!(bucket.try_consume(0).is_ok());
    assert!(bucket.try_consume(0).is_ok());
    assert!(bucket.try_consume(0).is_err()); // Window exhausted.
    assert!(bucket.try_consume(1000).is_ok()); // New window resets.
    assert!(bucket.try_consume(1000).is_ok());
    assert!(bucket.try_consume(1000).is_err());
}
