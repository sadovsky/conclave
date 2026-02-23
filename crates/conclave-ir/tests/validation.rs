use conclave_ir::*;
use std::collections::BTreeMap;

fn minimal_ir() -> PlanIr {
    PlanIr {
        conclave_ir_version: "0.1".into(),
        module: Module {
            name: "test".into(),
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
            entry_goal: "gid:test".into(),
        },
    }
}

#[test]
fn validate_ok_minimal() {
    assert!(validate_plan_ir(&minimal_ir()).is_ok());
}

#[test]
fn validate_rejects_wrong_version() {
    let mut ir = minimal_ir();
    ir.conclave_ir_version = "0.2".into();
    assert!(matches!(
        validate_plan_ir(&ir),
        Err(IrError::UnsupportedVersion(_))
    ));
}

#[test]
fn validate_rejects_duplicate_node_id() {
    let mut ir = minimal_ir();
    let node = Node {
        node_id: "nid:dup".into(),
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
        import_subgraph_id: None,
        meta: None,
    };
    ir.nodes.push(node.clone());
    ir.nodes.push(node);
    assert!(matches!(
        validate_plan_ir(&ir),
        Err(IrError::DuplicateNodeId(_))
    ));
}

#[test]
fn validate_rejects_edge_with_unknown_node() {
    let mut ir = minimal_ir();
    ir.edges.push(Edge {
        edge_id: "eid:test".into(),
        from: EdgeEndpoint {
            node_id: "nid:nonexistent".into(),
            port: "out".into(),
        },
        to: EdgeEndpoint {
            node_id: "nid:also_nonexistent".into(),
            port: "in".into(),
        },
    });
    assert!(matches!(
        validate_plan_ir(&ir),
        Err(IrError::EdgeReferencesUnknownNode { .. })
    ));
}
