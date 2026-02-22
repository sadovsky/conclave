use conclave_ir::*;
use std::collections::BTreeMap;

fn minimal_ir() -> PlanIr {
    PlanIr {
        conclave_ir_version: "0.1".into(),
        module: Module {
            name: "test_module".into(),
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
    }
}

#[test]
fn plan_ir_hash_is_deterministic() {
    let ir = minimal_ir();
    let h1 = compute_plan_ir_hash(&ir);
    let h2 = compute_plan_ir_hash(&ir);
    assert_eq!(h1, h2);
}

#[test]
fn plan_ir_hash_has_sha256_prefix() {
    let ir = minimal_ir();
    let h = compute_plan_ir_hash(&ir);
    assert!(h.to_string().starts_with("sha256:"));
}

#[test]
fn plan_ir_hash_differs_with_different_module_name() {
    let mut ir1 = minimal_ir();
    let mut ir2 = minimal_ir();
    ir1.module.name = "a".into();
    ir2.module.name = "b".into();
    assert_ne!(compute_plan_ir_hash(&ir1), compute_plan_ir_hash(&ir2));
}

#[test]
fn plan_ir_hash_ignores_meta_field() {
    let mut ir_with_meta = minimal_ir();
    ir_with_meta.nodes.push(Node {
        node_id: "nid:n1".into(),
        kind: NodeKind::Intrinsic,
        op: Op {
            name: "noop".into(),
            signature: "noop()->()".into(),
        },
        inputs: vec![],
        outputs: vec![],
        attrs: NodeAttrs {
            determinism_profile: DeterminismProfile::Fixed,
            cost_hints: None,
            url_index: None,
        },
        constraints: vec![],
        meta: Some(serde_json::json!({"span": {"start": 0, "end": 100}, "origin": "test"})),
    });

    let mut ir_without_meta = minimal_ir();
    ir_without_meta.nodes.push(Node {
        node_id: "nid:n1".into(),
        kind: NodeKind::Intrinsic,
        op: Op {
            name: "noop".into(),
            signature: "noop()->()".into(),
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

    assert_eq!(
        compute_plan_ir_hash(&ir_with_meta),
        compute_plan_ir_hash(&ir_without_meta),
        "meta fields must not affect plan_ir_hash"
    );
}

#[test]
fn node_id_is_deterministic() {
    let node = Node {
        node_id: String::new(),
        kind: NodeKind::CapabilityCall,
        op: Op {
            name: "fetch".into(),
            signature: "fetch(Url)->Html".into(),
        },
        inputs: vec![InputPort {
            port: "in.url".into(),
            type_name: "Url".into(),
            source: None,
        }],
        outputs: vec![OutputPort {
            port: "out.html".into(),
            type_name: "Html".into(),
        }],
        attrs: NodeAttrs {
            determinism_profile: DeterminismProfile::Replayable,
            cost_hints: None,
            url_index: Some(0),
        },
        constraints: vec![],
        meta: None,
    };
    let id1 = compute_node_id(&node);
    let id2 = compute_node_id(&node);
    assert_eq!(id1, id2);
    assert!(id1.to_string().starts_with("sha256:"));
}

#[test]
fn edge_id_is_deterministic() {
    let edge = Edge {
        edge_id: String::new(),
        from: EdgeEndpoint {
            node_id: "nid:a".into(),
            port: "out".into(),
        },
        to: EdgeEndpoint {
            node_id: "nid:b".into(),
            port: "in".into(),
        },
    };
    let id1 = compute_edge_id(&edge);
    let id2 = compute_edge_id(&edge);
    assert_eq!(id1, id2);
}
