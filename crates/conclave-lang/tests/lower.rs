use conclave_lang::lower;
use conclave_ir::validate_plan_ir;

fn source() -> &'static str {
    include_str!("fixtures/summarize_urls/source.conclave")
}

#[test]
fn lower_produces_valid_plan_ir() {
    let out = lower(source(), 3).unwrap();
    validate_plan_ir(&out.plan_ir).expect("Plan IR should be structurally valid");
}

#[test]
fn lower_node_count_for_3_urls() {
    // 3 fetch + 3 extract_text + 3 summarize + 1 assemble_json = 10 nodes
    let out = lower(source(), 3).unwrap();
    assert_eq!(out.plan_ir.nodes.len(), 10, "expected 10 nodes for 3 URLs");
}

#[test]
fn lower_edge_count_for_3_urls() {
    // Per url: extract_text←fetch (1) + summarize←extract_text (1) = 2 edges
    // Plus 3 summarize→assemble_json edges = 9 total edges
    let out = lower(source(), 3).unwrap();
    assert_eq!(out.plan_ir.edges.len(), 9, "expected 9 edges for 3 URLs");
}

#[test]
fn lower_goal_name() {
    let out = lower(source(), 3).unwrap();
    assert_eq!(out.plan_ir.goals[0].name, "SummarizeUrls");
    assert_eq!(out.plan_ir.exports.entry_goal, "SummarizeUrls");
}

#[test]
fn lower_goal_params() {
    let out = lower(source(), 3).unwrap();
    let g = &out.plan_ir.goals[0];
    assert_eq!(g.params.len(), 1);
    assert_eq!(g.params[0].name, "urls");
    assert_eq!(g.params[0].type_name, "List<Url>");
}

#[test]
fn lower_constraints_present() {
    let out = lower(source(), 3).unwrap();
    // Should have 3 constraints
    assert_eq!(out.plan_ir.constraints.len(), 3);
    // Check keys
    let keys: Vec<&str> = out.plan_ir.constraints.keys().map(|s| s.as_str()).collect();
    assert!(keys.iter().any(|k| k.contains("determinism")), "missing determinism constraint");
    assert!(keys.iter().any(|k| k.contains("rate_limit")), "missing rate_limit constraint");
    assert!(keys.iter().any(|k| k.contains("scheduler")), "missing scheduler constraint");
}

#[test]
fn lower_url_index_attrs_set() {
    let out = lower(source(), 3).unwrap();
    // Fetch, extract_text, and summarize nodes should have url_index set.
    let with_index: Vec<_> = out.plan_ir.nodes.iter()
        .filter(|n| n.attrs.url_index.is_some())
        .collect();
    // 3 fetch + 3 extract_text + 3 summarize = 9 nodes with url_index
    assert_eq!(with_index.len(), 9);
    // Check all indices 0, 1, 2 are present.
    for expected_idx in 0u32..3 {
        assert!(
            with_index.iter().any(|n| n.attrs.url_index == Some(expected_idx)),
            "missing url_index {expected_idx}"
        );
    }
}

#[test]
fn lower_entry_nodes_are_fetch_nodes() {
    let out = lower(source(), 3).unwrap();
    let goal = &out.plan_ir.goals[0];
    // Entry nodes = fetch nodes (3)
    assert_eq!(goal.entry_nodes.len(), 3, "expected 3 entry nodes");
    // All entry nodes should have kind CapabilityCall and op.name fetch
    for entry_id in &goal.entry_nodes {
        let node = out.plan_ir.nodes.iter().find(|n| &n.node_id == entry_id).unwrap();
        assert_eq!(node.op.name, "fetch", "entry node should be a fetch node");
    }
}

#[test]
fn lower_exit_node_is_assemble_json() {
    let out = lower(source(), 3).unwrap();
    let goal = &out.plan_ir.goals[0];
    assert_eq!(goal.exit_nodes.len(), 1);
    let exit_id = &goal.exit_nodes[0];
    let exit_node = out.plan_ir.nodes.iter().find(|n| &n.node_id == exit_id).unwrap();
    assert_eq!(exit_node.op.name, "assemble_json");
}

#[test]
fn lower_subgraph_registered() {
    let out = lower(source(), 3).unwrap();
    assert_eq!(out.plan_ir.subgraphs.len(), 1);
    assert_eq!(out.plan_ir.subgraphs[0].kind, "map");
    assert_eq!(out.plan_ir.subgraphs[0].nodes.len(), 9); // 3 * 3
}

#[test]
fn lower_plan_ir_hash_is_stable() {
    let out1 = lower(source(), 3).unwrap();
    let out2 = lower(source(), 3).unwrap();
    assert_eq!(out1.plan_ir_hash, out2.plan_ir_hash);
}

#[test]
fn lower_source_hash_is_stable() {
    let out1 = lower(source(), 3).unwrap();
    let out2 = lower(source(), 3).unwrap();
    assert_eq!(out1.source_hash, out2.source_hash);
}

#[test]
fn lower_ast_hash_is_stable() {
    let out1 = lower(source(), 3).unwrap();
    let out2 = lower(source(), 3).unwrap();
    assert_eq!(out1.ast_hash, out2.ast_hash);
}

#[test]
fn lower_different_url_counts_differ() {
    let out2 = lower(source(), 2).unwrap();
    let out3 = lower(source(), 3).unwrap();
    assert_ne!(out2.plan_ir_hash, out3.plan_ir_hash);
    assert_eq!(out2.plan_ir.nodes.len(), 7); // 2*3 + 1
    assert_eq!(out3.plan_ir.nodes.len(), 10); // 3*3 + 1
}

#[test]
fn lower_unknown_capability_error() {
    let src = r#"version 0.1;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal G(urls: List<String>) -> Json {
  want {
    map urls as url {
      let x = nonexistent_fn(url);
      emit x;
    }
    return assemble_json(collected);
  }
}
"#;
    let result = lower(src, 1);
    match result {
        Err(conclave_lang::LangError::UnknownCapability(name)) => {
            assert_eq!(name, "nonexistent_fn");
        }
        Err(e) => panic!("unexpected error: {e}"),
        Ok(_) => panic!("expected UnknownCapability error, got Ok"),
    }
}

#[test]
fn lower_type_registered_in_plan_ir() {
    let out = lower(source(), 1).unwrap();
    assert!(out.plan_ir.types.contains_key("Url"), "Url type should be in Plan IR");
    let url_type = &out.plan_ir.types["Url"];
    assert_eq!(url_type.kind, "alias");
    assert_eq!(url_type.of.as_deref(), Some("String"));
    let predicates = url_type.predicates.as_ref().unwrap();
    assert_eq!(predicates[0].lang, "re2");
}

#[test]
fn lower_constraint_ast_structure() {
    let out = lower(source(), 1).unwrap();
    // Find the rate_limit constraint.
    let (_, rate_c) = out.plan_ir.constraints.iter()
        .find(|(k, _)| k.contains("rate_limit"))
        .expect("rate_limit constraint not found");
    assert_eq!(rate_c.expr.lang, "conclave_v0.1");
    let ast = &rate_c.expr.ast;
    assert_eq!(ast["op"], "<=");
    assert_eq!(ast["left"]["fn"], "rate_limit");
    assert_eq!(ast["right"]["rate"], 2);
    assert_eq!(ast["right"]["unit"], "req/s");
}

#[test]
fn lower_emit_ident_works() {
    // `emit ident;` should work the same as `emit call_that_produces_ident();`
    let src = r#"version 0.1;
capability fetch: fetch(String) -> Html;
intrinsic assemble_json: assemble_json(List<Html>) -> Json;
goal G(urls: List<String>) -> Json {
  want {
    map urls as url {
      let page = fetch(url);
      emit page;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "sealed_replay";
  }
}
"#;
    let out = lower(src, 2).unwrap();
    validate_plan_ir(&out.plan_ir).unwrap();
    // 2 fetch nodes + 1 assemble = 3 nodes
    assert_eq!(out.plan_ir.nodes.len(), 3, "expected 3 nodes");
    // 2 edges: fetch[0] → assemble.in_0, fetch[1] → assemble.in_1
    assert_eq!(out.plan_ir.edges.len(), 2, "expected 2 edges");
}
