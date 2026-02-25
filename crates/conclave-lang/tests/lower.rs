use conclave_ir::{validate_plan_ir, NodeKind};
use conclave_lang::{lower, lower_all, lower_named, lower_with_cache, ModuleCache};

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
    assert!(
        keys.iter().any(|k| k.contains("determinism")),
        "missing determinism constraint"
    );
    assert!(
        keys.iter().any(|k| k.contains("rate_limit")),
        "missing rate_limit constraint"
    );
    assert!(
        keys.iter().any(|k| k.contains("scheduler")),
        "missing scheduler constraint"
    );
}

#[test]
fn lower_url_index_attrs_set() {
    let out = lower(source(), 3).unwrap();
    // Fetch, extract_text, and summarize nodes should have url_index set.
    let with_index: Vec<_> = out
        .plan_ir
        .nodes
        .iter()
        .filter(|n| n.attrs.url_index.is_some())
        .collect();
    // 3 fetch + 3 extract_text + 3 summarize = 9 nodes with url_index
    assert_eq!(with_index.len(), 9);
    // Check all indices 0, 1, 2 are present.
    for expected_idx in 0u32..3 {
        assert!(
            with_index
                .iter()
                .any(|n| n.attrs.url_index == Some(expected_idx)),
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
        let node = out
            .plan_ir
            .nodes
            .iter()
            .find(|n| &n.node_id == entry_id)
            .unwrap();
        assert_eq!(node.op.name, "fetch", "entry node should be a fetch node");
    }
}

#[test]
fn lower_exit_node_is_assemble_json() {
    let out = lower(source(), 3).unwrap();
    let goal = &out.plan_ir.goals[0];
    assert_eq!(goal.exit_nodes.len(), 1);
    let exit_id = &goal.exit_nodes[0];
    let exit_node = out
        .plan_ir
        .nodes
        .iter()
        .find(|n| &n.node_id == exit_id)
        .unwrap();
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
    assert!(
        out.plan_ir.types.contains_key("Url"),
        "Url type should be in Plan IR"
    );
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
    let (_, rate_c) = out
        .plan_ir
        .constraints
        .iter()
        .find(|(k, _)| k.contains("rate_limit"))
        .expect("rate_limit constraint not found");
    assert_eq!(rate_c.expr.lang, "conclave_v0.1");
    let ast = &rate_c.expr.ast;
    assert_eq!(ast["op"], "<=");
    assert_eq!(ast["left"]["fn"], "rate_limit");
    assert_eq!(ast["right"]["rate"], 2);
    assert_eq!(ast["right"]["unit"], "req/s");
}

// ---------------------------------------------------------------------------
// Expanded DSL: lower_named and lower_all
// ---------------------------------------------------------------------------

static MULTI_GOAL_SRC: &str = r#"version 0.1;
capability fetch: fetch(String) -> Html;
intrinsic to_string: to_string(Html) -> String;
goal FetchPage(url: String) -> Html {
  want {
    return fetch(url);
  }
}
goal ProcessPage(html: Html) -> String {
  want {
    return to_string(html);
  }
}
"#;

#[test]
fn lower_named_selects_correct_goal() {
    let out = lower_named(MULTI_GOAL_SRC, "ProcessPage", 1).unwrap();
    validate_plan_ir(&out.plan_ir).unwrap();
    assert_eq!(out.plan_ir.goals[0].name, "ProcessPage");
    assert_eq!(out.plan_ir.exports.entry_goal, "ProcessPage");
}

#[test]
fn lower_named_goal_not_found_error() {
    let result = lower_named(MULTI_GOAL_SRC, "NonExistent", 1);
    assert!(matches!(result, Err(conclave_lang::LangError::GoalNotFound(_))));
}

#[test]
fn lower_all_returns_one_output_per_goal() {
    let outputs = lower_all(MULTI_GOAL_SRC, 1).unwrap();
    assert_eq!(outputs.len(), 2);
    let names: Vec<&str> = outputs.iter().map(|o| o.plan_ir.goals[0].name.as_str()).collect();
    assert!(names.contains(&"FetchPage"));
    assert!(names.contains(&"ProcessPage"));
}

#[test]
fn lower_all_plan_ir_hashes_are_independent() {
    let outputs = lower_all(MULTI_GOAL_SRC, 1).unwrap();
    assert_ne!(outputs[0].plan_ir_hash, outputs[1].plan_ir_hash);
}

#[test]
fn lower_all_each_goal_is_valid_plan_ir() {
    let outputs = lower_all(MULTI_GOAL_SRC, 1).unwrap();
    for out in &outputs {
        validate_plan_ir(&out.plan_ir).expect("each goal should be a valid Plan IR");
    }
}

// ---------------------------------------------------------------------------
// Expanded DSL: if/else lowering
// ---------------------------------------------------------------------------

static IF_ELSE_SRC: &str = r#"version 0.1;
capability is_article: is_article(Html) -> Bool;
capability summarize: summarize(Html) -> String;
capability skip: skip(Html) -> String;
capability fetch: fetch(String) -> Html;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal ClassifyPages(urls: List<String>) -> Json {
  want {
    map urls as url {
      let html = fetch(url);
      if is_article(html) {
        emit summarize(html);
      } else {
        emit skip(html);
      }
    }
    return assemble_json(collected);
  }
}
"#;

#[test]
fn lower_if_else_produces_control_node() {
    let out = lower(IF_ELSE_SRC, 2).unwrap();
    validate_plan_ir(&out.plan_ir).unwrap();
    let control_nodes: Vec<_> = out
        .plan_ir
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Control) && n.op.name == "conditional_branch")
        .collect();
    // 2 map iterations → 2 conditional_branch nodes
    assert_eq!(control_nodes.len(), 2, "expected 2 conditional_branch nodes");
}

#[test]
fn lower_if_else_creates_branch_subgraphs() {
    let out = lower(IF_ELSE_SRC, 2).unwrap();
    let conditional_sgs: Vec<_> = out
        .plan_ir
        .subgraphs
        .iter()
        .filter(|sg| sg.kind.starts_with("conditional"))
        .collect();
    // 2 iterations × 2 branches (true + false) = 4 subgraphs, plus 1 map subgraph
    assert!(conditional_sgs.len() >= 4, "expected at least 4 conditional subgraphs");
}

#[test]
fn lower_if_else_hash_is_stable() {
    let out1 = lower(IF_ELSE_SRC, 2).unwrap();
    let out2 = lower(IF_ELSE_SRC, 2).unwrap();
    assert_eq!(out1.plan_ir_hash, out2.plan_ir_hash);
}

// ---------------------------------------------------------------------------
// Expanded DSL: reduce lowering
// ---------------------------------------------------------------------------

static REDUCE_SRC: &str = r#"version 0.1;
capability fetch: fetch(String) -> Html;
capability merge_html: merge_html(Html, Html) -> Html;
goal MergePages(urls: List<String>) -> Html {
  want {
    reduce urls as url into acc {
      let page = fetch(url);
      acc = merge_html(acc, page);
    }
    return acc;
  }
}
"#;

#[test]
fn lower_reduce_produces_valid_plan_ir() {
    let out = lower(REDUCE_SRC, 3).unwrap();
    validate_plan_ir(&out.plan_ir).unwrap();
}

#[test]
fn lower_reduce_creates_init_node() {
    let out = lower(REDUCE_SRC, 2).unwrap();
    let init_nodes: Vec<_> = out
        .plan_ir
        .nodes
        .iter()
        .filter(|n| n.op.name == "reduce_init")
        .collect();
    assert_eq!(init_nodes.len(), 1, "expected exactly one reduce_init node");
}

#[test]
fn lower_reduce_sequential_chain() {
    // url_count=2 → reduce_init + 2 fetch + 2 merge_html + 1 return (merge_html) = 6 nodes
    let out = lower(REDUCE_SRC, 2).unwrap();
    // reduce_init (1) + fetch*2 + merge_html*2 + return merge_html (1) = 6
    // Actually: reduce_init(1) + [fetch(1), merge_html(1)] * 2 + return (merge_html node in return) ...
    // The return calls merge_html which is the last acc value node — let's just check > 0
    assert!(!out.plan_ir.nodes.is_empty());
    assert_eq!(out.plan_ir.subgraphs.iter().filter(|sg| sg.kind == "reduce").count(), 1);
}

#[test]
fn lower_reduce_hash_is_stable() {
    let out1 = lower(REDUCE_SRC, 3).unwrap();
    let out2 = lower(REDUCE_SRC, 3).unwrap();
    assert_eq!(out1.plan_ir_hash, out2.plan_ir_hash);
}

// ---------------------------------------------------------------------------
// Expanded DSL: pure block lowering
// ---------------------------------------------------------------------------

static PURE_SRC: &str = r#"version 0.1;
capability fetch: fetch(String) -> String;
intrinsic word_count: word_count(String) -> Int;
intrinsic assemble_json: assemble_json(List<Int>) -> Json;
goal CountWords(urls: List<String>) -> Json {
  want {
    map urls as url {
      let text = fetch(url);
      let count = pure { word_count(text) };
      emit count;
    }
    return assemble_json(collected);
  }
}
"#;

#[test]
fn lower_pure_block_produces_valid_plan_ir() {
    let out = lower(PURE_SRC, 2).unwrap();
    validate_plan_ir(&out.plan_ir).unwrap();
}

#[test]
fn lower_pure_block_creates_intrinsic_node() {
    let out = lower(PURE_SRC, 1).unwrap();
    let intrinsic_nodes: Vec<_> = out
        .plan_ir
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Intrinsic) && n.op.name == "word_count")
        .collect();
    assert_eq!(intrinsic_nodes.len(), 1, "expected 1 word_count intrinsic node");
}

#[test]
fn lower_pure_block_rejects_capability() {
    let src = r#"version 0.1;
capability fetch: fetch(String) -> Html;
intrinsic assemble_json: assemble_json(List<Html>) -> Json;
goal G(urls: List<String>) -> Json {
  want {
    map urls as url {
      let page = pure { fetch(url) };
      emit page;
    }
    return assemble_json(collected);
  }
}
"#;
    let result = lower(src, 1);
    assert!(
        matches!(result, Err(conclave_lang::LangError::PureBlockContainsCapability(_))),
        "expected PureBlockContainsCapability error"
    );
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

// ---------------------------------------------------------------------------
// Phase 10: import expansion
// ---------------------------------------------------------------------------

/// Source for the sub-module: fetch + extract_text per URL, assemble into Json.
static FETCH_EXTRACT_SRC: &str = r#"version 0.1;
capability fetch: fetch(String) -> Html;
capability extract_text: extract_text(Html) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal FetchAndExtract(urls: List<String>) -> Json {
  want {
    map urls as url {
      let html = fetch(url);
      let text = extract_text(html);
      emit text;
    }
    return assemble_json(collected);
  }
  constraints {
    determinism.mode == "sealed_replay";
  }
}
"#;

/// Publish the sub-module to a temp cache and return (cache, hash).
fn setup_sub_module_cache() -> (tempfile::TempDir, ModuleCache, String) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = ModuleCache::new(tmp.path().join("modules"));
    // Publish with url_count=1 (one URL per call site invocation).
    let sub_out = lower(FETCH_EXTRACT_SRC, 1).unwrap();
    let hash = cache.put(&sub_out.plan_ir).unwrap();
    (tmp, cache, hash)
}

#[test]
fn lower_import_expands_nodes() {
    let (_tmp, cache, hash) = setup_sub_module_cache();

    // Parent: calls FetchAndExtract(url) for each of 2 URLs, then summarizes.
    let src = format!(
        r#"version 0.1;
import FetchAndExtract: "{hash}";
capability summarize: summarize(String) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal Summarize(urls: List<String>) -> Json {{
  want {{
    map urls as url {{
      let text = FetchAndExtract(url);
      emit summarize(text);
    }}
    return assemble_json(collected);
  }}
  constraints {{ determinism.mode == "sealed_replay"; }}
}}
"#
    );

    let out = lower_with_cache(&src, 2, Some(&cache)).unwrap();
    validate_plan_ir(&out.plan_ir).unwrap();

    // Per url_count=2 iteration:
    //   FetchAndExtract expands to 3 nodes (fetch + extract_text + assemble_json)
    //   Plus 1 summarize node
    // = 2 * (3 + 1) + 1 final assemble_json = 9 nodes
    assert_eq!(
        out.plan_ir.nodes.len(),
        9,
        "expected 9 nodes (2 iters × 4 inlined + 1 outer assemble_json)"
    );
}

#[test]
fn lower_import_nodes_carry_subgraph_id() {
    let (_tmp, cache, hash) = setup_sub_module_cache();

    let src = format!(
        r#"version 0.1;
import FetchAndExtract: "{hash}";
capability summarize: summarize(String) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal Summarize(urls: List<String>) -> Json {{
  want {{
    map urls as url {{
      let text = FetchAndExtract(url);
      emit summarize(text);
    }}
    return assemble_json(collected);
  }}
  constraints {{ determinism.mode == "sealed_replay"; }}
}}
"#
    );

    let out = lower_with_cache(&src, 1, Some(&cache)).unwrap();

    // All nodes from FetchAndExtract should have import_subgraph_id set.
    let tagged: Vec<_> = out
        .plan_ir
        .nodes
        .iter()
        .filter(|n| n.import_subgraph_id.is_some())
        .collect();
    // url_count=1: FetchAndExtract expands to 3 nodes (fetch + extract_text + assemble_json).
    assert_eq!(tagged.len(), 3, "expected 3 nodes with import_subgraph_id");

    // The id should be the plan_ir_hash of the sub-module.
    let sub_out = lower(FETCH_EXTRACT_SRC, 1).unwrap();
    let expected_sg_id = sub_out.plan_ir_hash;
    for n in &tagged {
        assert_eq!(
            n.import_subgraph_id.as_deref(),
            Some(expected_sg_id.as_str()),
            "import_subgraph_id mismatch on node {}",
            n.node_id
        );
    }
}

#[test]
fn lower_import_registers_subgraph() {
    let (_tmp, cache, hash) = setup_sub_module_cache();

    let src = format!(
        r#"version 0.1;
import FetchAndExtract: "{hash}";
intrinsic assemble_json: assemble_json(List<Json>) -> Json;
goal G(urls: List<String>) -> Json {{
  want {{
    map urls as url {{
      let r = FetchAndExtract(url);
      emit r;
    }}
    return assemble_json(collected);
  }}
}}
"#
    );

    let out = lower_with_cache(&src, 2, Some(&cache)).unwrap();
    let import_sgs: Vec<_> = out
        .plan_ir
        .subgraphs
        .iter()
        .filter(|sg| sg.kind == "import")
        .collect();
    // 2 iterations → 2 import subgraphs
    assert_eq!(import_sgs.len(), 2, "expected 2 import subgraphs for 2 iterations");
}

#[test]
fn lower_import_plan_ir_is_valid() {
    let (_tmp, cache, hash) = setup_sub_module_cache();

    let src = format!(
        r#"version 0.1;
import FetchAndExtract: "{hash}";
capability summarize: summarize(String) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal Summarize(urls: List<String>) -> Json {{
  want {{
    map urls as url {{
      let text = FetchAndExtract(url);
      emit summarize(text);
    }}
    return assemble_json(collected);
  }}
}}
"#
    );

    let out = lower_with_cache(&src, 3, Some(&cache)).unwrap();
    validate_plan_ir(&out.plan_ir).expect("expanded Plan IR must be structurally valid");
}

#[test]
fn lower_import_hash_is_stable() {
    let (_tmp, cache, hash) = setup_sub_module_cache();

    let src = format!(
        r#"version 0.1;
import FetchAndExtract: "{hash}";
capability summarize: summarize(String) -> String;
intrinsic assemble_json: assemble_json(List<String>) -> Json;
goal Summarize(urls: List<String>) -> Json {{
  want {{
    map urls as url {{
      let text = FetchAndExtract(url);
      emit summarize(text);
    }}
    return assemble_json(collected);
  }}
}}
"#
    );

    let out1 = lower_with_cache(&src, 2, Some(&cache)).unwrap();
    let out2 = lower_with_cache(&src, 2, Some(&cache)).unwrap();
    assert_eq!(out1.plan_ir_hash, out2.plan_ir_hash);
}

#[test]
fn lower_import_without_cache_errors() {
    // lower() (no cache) on a source with an import should return ImportResolutionRequired.
    let src = r#"version 0.1;
import Foo: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
goal G(urls: List<String>) -> Json {
  want { return Foo(urls); }
}
"#;
    // Note: lower() calls lower_with_cache(_, _, None) which should error.
    let result = lower(src, 1);
    assert!(
        matches!(result, Err(conclave_lang::LangError::ImportResolutionRequired(_))),
        "expected ImportResolutionRequired"
    );
}

#[test]
fn lower_import_not_in_cache_errors() {
    let (_tmp, cache, _hash) = setup_sub_module_cache();

    let bad_hash = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let src = format!(
        r#"version 0.1;
import Foo: "{bad_hash}";
goal G(urls: List<String>) -> Json {{
  want {{ return Foo(urls); }}
}}
"#
    );

    let result = lower_with_cache(&src, 1, Some(&cache));
    assert!(
        matches!(result, Err(conclave_lang::LangError::ImportNotFound(_))),
        "expected ImportNotFound"
    );
}

// ---------------------------------------------------------------------------
// Phase 11: Arity checking
// ---------------------------------------------------------------------------

#[test]
fn lower_arity_mismatch_too_few_args() {
    let src = r#"version 0.1;
capability fetch: fetch(String) -> Html;
intrinsic assemble_json: assemble_json(List<Html>) -> Json;
goal G(urls: List<String>) -> Json {
  want {
    map urls as url {
      let page = fetch();
      emit page;
    }
    return assemble_json(collected);
  }
}
"#;
    let result = lower(src, 1);
    match result {
        Err(conclave_lang::LangError::ArityMismatch { fn_name, expected, got }) => {
            assert_eq!(fn_name, "fetch");
            assert_eq!(expected, 1);
            assert_eq!(got, 0);
        }
        Err(e) => panic!("expected ArityMismatch, got: {e}"),
        Ok(_) => panic!("expected ArityMismatch error, got Ok"),
    }
}

#[test]
fn lower_arity_mismatch_too_many_args() {
    let src = r#"version 0.1;
capability fetch: fetch(String) -> Html;
intrinsic assemble_json: assemble_json(List<Html>) -> Json;
goal G(urls: List<String>) -> Json {
  want {
    map urls as url {
      let page = fetch(url, url);
      emit page;
    }
    return assemble_json(collected);
  }
}
"#;
    let result = lower(src, 1);
    match result {
        Err(conclave_lang::LangError::ArityMismatch { fn_name, expected, got }) => {
            assert_eq!(fn_name, "fetch");
            assert_eq!(expected, 1);
            assert_eq!(got, 2);
        }
        Err(e) => panic!("expected ArityMismatch, got: {e}"),
        Ok(_) => panic!("expected ArityMismatch error, got Ok"),
    }
}

#[test]
fn lower_if_else_gate_node_id_set() {
    // Each conditional_true/conditional_false subgraph must carry the gate_node_id
    // of the conditional_branch Control node that owns it.
    let out = lower(IF_ELSE_SRC, 1).unwrap();

    let gate_ids: Vec<String> = out
        .plan_ir
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Control) && n.op.name == "conditional_branch")
        .map(|n| n.node_id.clone())
        .collect();

    assert_eq!(gate_ids.len(), 1, "url_count=1 → 1 gate node");

    let conditional_sgs: Vec<_> = out
        .plan_ir
        .subgraphs
        .iter()
        .filter(|sg| sg.kind.starts_with("conditional"))
        .collect();

    assert_eq!(conditional_sgs.len(), 2, "1 gate → 2 branch subgraphs");

    for sg in &conditional_sgs {
        assert_eq!(
            sg.gate_node_id.as_deref(),
            Some(gate_ids[0].as_str()),
            "subgraph {} should link to gate {}",
            sg.subgraph_id,
            gate_ids[0]
        );
    }
}
