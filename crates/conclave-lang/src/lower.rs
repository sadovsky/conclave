#![allow(dead_code, unused_variables, unused_mut)]
use std::collections::BTreeMap;

use conclave_hash::{compute_stable_id, sha256_str};
use conclave_ir::{
    compute_constraint_id, compute_edge_id, compute_goal_id, Constraint,
    ConstraintExpr as IrConstraintExpr, ConstraintRef, ConstraintScope, DeterminismProfile, Edge,
    EdgeEndpoint, EdgeRef, Exports, Goal, GoalParam, InputPort, Module as IrModule, Node,
    NodeAttrs, NodeKind, Op, OutputPort, PlanIr, Subgraph,
};

use crate::ast::*;
use crate::error::LangError;
use crate::module_cache::ModuleCache;
use crate::normalize::{ast_hash, normalize};
use crate::parser::parse;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// The output of successfully lowering a Conclave source file.
#[allow(dead_code)]
pub struct LowerOutput {
    /// The Plan IR (ready to validate and seal).
    pub plan_ir: PlanIr,
    /// `sha256(<source_bytes>)`.
    pub source_hash: String,
    /// `sha256(<canonical_ast_json>)`.
    pub ast_hash: String,
    /// `sha256(<canonical_plan_ir_json>)`.
    pub plan_ir_hash: String,
}

/// Lower a Conclave v0.1 source string to Plan IR.
///
/// Lowers the **first** goal in the file.
/// `url_count` is the compile-time list length used to expand `map`/`reduce` constructs.
pub fn lower(source: &str, url_count: usize) -> Result<LowerOutput, LangError> {
    lower_with_cache(source, url_count, None)
}

/// Lower with an optional module cache for `import` declaration expansion.
///
/// If the source contains `import` declarations and `cache` is `None`,
/// returns `LangError::ImportResolutionRequired`.
pub fn lower_with_cache(
    source: &str,
    url_count: usize,
    cache: Option<&ModuleCache>,
) -> Result<LowerOutput, LangError> {
    let normalized_source = source.replace("\r\n", "\n").replace('\r', "\n");
    let module = parse(&normalized_source)?;
    let module = normalize(module)?;
    let goal_decl = module.goals.first().ok_or(LangError::NoGoals)?;
    lower_goal_from_module(&module, goal_decl, &normalized_source, url_count, cache)
}

/// Lower a named goal from the source string.
pub fn lower_named(source: &str, goal_name: &str, url_count: usize) -> Result<LowerOutput, LangError> {
    lower_named_with_cache(source, goal_name, url_count, None)
}

/// Lower a named goal with an optional module cache.
pub fn lower_named_with_cache(
    source: &str,
    goal_name: &str,
    url_count: usize,
    cache: Option<&ModuleCache>,
) -> Result<LowerOutput, LangError> {
    let normalized_source = source.replace("\r\n", "\n").replace('\r', "\n");
    let module = parse(&normalized_source)?;
    let module = normalize(module)?;
    let goal_decl = module
        .goals
        .iter()
        .find(|g| g.name == goal_name)
        .ok_or_else(|| LangError::GoalNotFound(goal_name.to_string()))?;
    lower_goal_from_module(&module, goal_decl, &normalized_source, url_count, cache)
}

/// Lower all goals in the source string, returning one `LowerOutput` per goal.
pub fn lower_all(source: &str, url_count: usize) -> Result<Vec<LowerOutput>, LangError> {
    lower_all_with_cache(source, url_count, None)
}

/// Lower all goals with an optional module cache.
pub fn lower_all_with_cache(
    source: &str,
    url_count: usize,
    cache: Option<&ModuleCache>,
) -> Result<Vec<LowerOutput>, LangError> {
    let normalized_source = source.replace("\r\n", "\n").replace('\r', "\n");
    let module = parse(&normalized_source)?;
    let module = normalize(module)?;
    if module.goals.is_empty() {
        return Err(LangError::NoGoals);
    }
    module
        .goals
        .iter()
        .map(|g| lower_goal_from_module(&module, g, &normalized_source, url_count, cache))
        .collect()
}

fn lower_goal_from_module(
    module: &Module,
    goal_decl: &GoalDecl,
    normalized_source: &str,
    url_count: usize,
    cache: Option<&ModuleCache>,
) -> Result<LowerOutput, LangError> {
    let source_hash = sha256_str(normalized_source).to_string();
    let ast_h = ast_hash(module).to_string();

    // Collect the set of function names actually called in this goal's want block.
    let mut called_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    collect_call_names(&goal_decl.want.stmts, &mut called_names);

    // Pre-resolve only imports that are actually called — unreferenced imports
    // are recorded in plan_ir.imports but do not require a cache entry.
    let mut resolved_imports: BTreeMap<String, conclave_ir::PlanIr> = BTreeMap::new();
    for imp in &module.imports {
        if !called_names.contains(&imp.name) {
            continue; // declared but not used — no resolution needed
        }
        match cache {
            Some(c) => {
                let sub_ir = c
                    .require(&imp.hash)
                    .map_err(|_| LangError::ImportNotFound(imp.hash.clone()))?;
                resolved_imports.insert(imp.name.clone(), sub_ir);
            }
            None => {
                return Err(LangError::ImportResolutionRequired(imp.name.clone()));
            }
        }
    }

    // Build lookup maps for cap/intrinsic declarations.
    let cap_map: BTreeMap<&str, &CapDecl> = module
        .capabilities
        .iter()
        .map(|c| (c.alias.as_str(), c))
        .collect();
    let intr_map: BTreeMap<&str, &IntrinsicDecl> = module
        .intrinsics
        .iter()
        .map(|i| (i.alias.as_str(), i))
        .collect();

    let mut state = LowerState::new(goal_decl.name.clone(), url_count, cap_map, intr_map, resolved_imports);
    state.lower_goal(goal_decl)?;

    let plan_ir = state.build_plan_ir(module, goal_decl, normalized_source);
    let plan_ir_hash = conclave_ir::compute_plan_ir_hash(&plan_ir).to_string();

    Ok(LowerOutput {
        plan_ir,
        source_hash,
        ast_hash: ast_h,
        plan_ir_hash,
    })
}

// ---------------------------------------------------------------------------
// Symbol table entry
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Symbol {
    /// Output port of an already-lowered node.
    NodePort {
        node_id: String,
        port: String,
        type_name: String,
    },
    /// The URL binder from a `map`/`reduce` at a given url_index.
    UrlParam { url_index: u32 },
}

// ---------------------------------------------------------------------------
// Lowering state
// ---------------------------------------------------------------------------

struct LowerState<'a> {
    goal_name: String,
    url_count: usize,
    cap_map: BTreeMap<&'a str, &'a CapDecl>,
    intr_map: BTreeMap<&'a str, &'a IntrinsicDecl>,
    /// Fully resolved imported Plan IRs, keyed by the import alias.
    resolved_imports: BTreeMap<String, conclave_ir::PlanIr>,

    nodes: Vec<Node>,
    edges: Vec<Edge>,
    subgraphs: Vec<Subgraph>,
    /// Constraint BTreeMap keyed by a human-readable key.
    constraints: BTreeMap<String, Constraint>,

    /// All `emit` outputs: (url_index, node_id, port, type_name).
    collected: Vec<(u32, String, String, String)>,

    /// Goal-level entry and exit node IDs.
    entry_nodes: Vec<String>,
    exit_node: Option<String>,
}

impl<'a> LowerState<'a> {
    fn new(
        goal_name: String,
        url_count: usize,
        cap_map: BTreeMap<&'a str, &'a CapDecl>,
        intr_map: BTreeMap<&'a str, &'a IntrinsicDecl>,
        resolved_imports: BTreeMap<String, conclave_ir::PlanIr>,
    ) -> Self {
        LowerState {
            goal_name,
            url_count,
            cap_map,
            intr_map,
            resolved_imports,
            nodes: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            constraints: BTreeMap::new(),
            collected: Vec::new(),
            entry_nodes: Vec::new(),
            exit_node: None,
        }
    }

    fn lower_goal(&mut self, goal: &GoalDecl) -> Result<(), LangError> {
        // Lower the want block.
        let mut scope = Scope::new();
        self.lower_stmts(&goal.want.stmts, &mut scope, None)?;

        // Lower constraints.
        for (idx, cexpr) in goal.constraints.iter().enumerate() {
            self.lower_constraint(cexpr, idx)?;
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Statement lowering
    // -----------------------------------------------------------------------

    fn lower_stmts(
        &mut self,
        stmts: &[Stmt],
        scope: &mut Scope,
        url_index: Option<u32>,
    ) -> Result<(), LangError> {
        for stmt in stmts {
            self.lower_stmt(stmt, scope, url_index)?;
        }
        Ok(())
    }

    fn lower_stmt(
        &mut self,
        stmt: &Stmt,
        scope: &mut Scope,
        url_index: Option<u32>,
    ) -> Result<(), LangError> {
        match stmt {
            Stmt::Let { name, expr } => self.lower_let(name, expr, scope, url_index)?,
            Stmt::Map { list, binder, body } => {
                self.lower_map(list, binder, body, scope, url_index)?
            }
            Stmt::If {
                condition,
                true_body,
                false_body,
            } => self.lower_if(condition, true_body, false_body, scope, url_index)?,
            Stmt::Reduce {
                list,
                binder,
                accum,
                body,
            } => self.lower_reduce(list, binder, accum, body, scope, url_index)?,
            Stmt::Assign { name, expr } => self.lower_assign(name, expr, scope, url_index)?,
            Stmt::Emit { expr } => self.lower_emit(expr, scope, url_index)?,
            Stmt::Return { expr } => self.lower_return(expr, scope, url_index)?,
        }
        Ok(())
    }

    // `let name = call(...);`
    fn lower_let(
        &mut self,
        name: &str,
        expr: &Expr,
        scope: &mut Scope,
        url_index: Option<u32>,
    ) -> Result<(), LangError> {
        if scope.get(name).is_some() {
            return Err(LangError::ShadowedBinding(name.to_string()));
        }
        let (node_id, out_port, out_type) = self.lower_call_node(expr, scope, url_index, name)?;
        scope.set(
            name,
            Symbol::NodePort {
                node_id,
                port: out_port,
                type_name: out_type,
            },
        );
        Ok(())
    }

    // `map LIST as BINDER { BODY }`
    fn lower_map(
        &mut self,
        list: &str,
        binder: &str,
        body: &[Stmt],
        _parent_scope: &mut Scope,
        parent_url_index: Option<u32>,
    ) -> Result<(), LangError> {
        if self.url_count == 0 {
            return Err(LangError::MapRequiresUrlCount);
        }

        let mut map_node_ids: Vec<String> = Vec::new();

        for i in 0..self.url_count {
            let ui = i as u32;
            let mut child_scope = Scope::new();
            // Bind the loop variable to the URL parameter for this index.
            child_scope.set(binder, Symbol::UrlParam { url_index: ui });

            let nodes_before = self.nodes.len();
            self.lower_stmts(body, &mut child_scope, Some(ui))?;
            // Collect node IDs added during this iteration.
            for node in &self.nodes[nodes_before..] {
                map_node_ids.push(node.node_id.clone());
            }
        }

        // Register a subgraph for this map construct.
        let subgraph_id =
            compute_stable_id("subgraph", &format!("{}.map.{}", self.goal_name, list)).to_string();
        let sg = Subgraph {
            subgraph_id,
            kind: "map".into(),
            nodes: map_node_ids,
            constraints: Vec::new(),
        };
        self.subgraphs.push(sg);

        Ok(())
    }

    // `if COND { true_body } else { false_body }`
    //
    // Lowers to:
    // - A condition node (CapabilityCall or Intrinsic)
    // - A Control node `conditional_branch` with input=condition, outputs: branch_true/branch_false
    // - Two Subgraphs (branch_true, branch_false) containing the body nodes
    // Emits from both branches are collected; the runtime decides which fires.
    fn lower_if(
        &mut self,
        condition: &Expr,
        true_body: &[Stmt],
        false_body: &[Stmt],
        scope: &mut Scope,
        url_index: Option<u32>,
    ) -> Result<(), LangError> {
        let branch_counter = self.nodes.len();
        let ui_label = url_index
            .map(|u| u.to_string())
            .unwrap_or_else(|| "none".into());

        // Lower the condition expression to a node.
        let cond_binder = format!("_if_cond_{}", branch_counter);
        let (cond_node_id, cond_out_port, cond_out_type) =
            self.lower_call_node(condition, scope, url_index, &cond_binder)?;

        // Build a Control "conditional_branch" node that reads the condition.
        let gate_key = format!("{}.if_gate.{}.{}", self.goal_name, branch_counter, ui_label);
        let gate_id = stable_node_id(&gate_key);

        let gate_edge_placeholder = Edge {
            edge_id: "placeholder".into(),
            from: EdgeEndpoint {
                node_id: cond_node_id.clone(),
                port: cond_out_port.clone(),
            },
            to: EdgeEndpoint {
                node_id: gate_id.clone(),
                port: "condition".into(),
            },
        };
        let gate_edge_id = compute_edge_id(&gate_edge_placeholder).to_string();
        let gate_edge = Edge {
            edge_id: gate_edge_id.clone(),
            ..gate_edge_placeholder
        };

        let gate_node = Node {
            node_id: gate_id.clone(),
            kind: NodeKind::Control,
            op: Op {
                name: "conditional_branch".into(),
                signature: format!("conditional_branch({})->Bool", cond_out_type),
            },
            inputs: vec![InputPort {
                port: "condition".into(),
                type_name: cond_out_type,
                source: Some(EdgeRef {
                    edge_id: gate_edge_id,
                }),
            }],
            outputs: vec![
                OutputPort {
                    port: "branch_true".into(),
                    type_name: "Bool".into(),
                },
                OutputPort {
                    port: "branch_false".into(),
                    type_name: "Bool".into(),
                },
            ],
            attrs: NodeAttrs {
                determinism_profile: DeterminismProfile::Fixed,
                cost_hints: None,
                url_index,
            },
            constraints: Vec::new(),
            meta: None,
            import_subgraph_id: None,
        };
        self.edges.push(gate_edge);
        self.nodes.push(gate_node);

        // Lower true branch.
        let nodes_before_true = self.nodes.len();
        let mut true_scope = scope.child_with_gate(&gate_id, "branch_true");
        self.lower_stmts(true_body, &mut true_scope, url_index)?;
        let true_node_ids: Vec<String> = self.nodes[nodes_before_true..]
            .iter()
            .map(|n| n.node_id.clone())
            .collect();

        // Lower false branch.
        let nodes_before_false = self.nodes.len();
        let mut false_scope = scope.child_with_gate(&gate_id, "branch_false");
        self.lower_stmts(false_body, &mut false_scope, url_index)?;
        let false_node_ids: Vec<String> = self.nodes[nodes_before_false..]
            .iter()
            .map(|n| n.node_id.clone())
            .collect();

        // Register subgraphs.
        let true_sg_id =
            compute_stable_id("subgraph", &format!("{}.if_true.{}", self.goal_name, branch_counter))
                .to_string();
        self.subgraphs.push(Subgraph {
            subgraph_id: true_sg_id,
            kind: "conditional_true".into(),
            nodes: true_node_ids,
            constraints: Vec::new(),
        });

        let false_sg_id =
            compute_stable_id("subgraph", &format!("{}.if_false.{}", self.goal_name, branch_counter))
                .to_string();
        self.subgraphs.push(Subgraph {
            subgraph_id: false_sg_id,
            kind: "conditional_false".into(),
            nodes: false_node_ids,
            constraints: Vec::new(),
        });

        Ok(())
    }

    // `reduce LIST as BINDER into ACCUM { body }`
    //
    // Unrolls into a sequential chain for each list item (url_index 0..url_count).
    // Each iteration's body must contain `Stmt::Assign { name: accum, .. }` which
    // produces the accumulator value fed into the next iteration.
    fn lower_reduce(
        &mut self,
        list: &str,
        binder: &str,
        accum: &str,
        body: &[Stmt],
        parent_scope: &mut Scope,
        _parent_url_index: Option<u32>,
    ) -> Result<(), LangError> {
        if self.url_count == 0 {
            return Err(LangError::MapRequiresUrlCount);
        }

        // The accumulator symbol: starts as a special "init" node that the
        // runtime provides the zero value for.
        let init_key = format!("{}.reduce_init.{}", self.goal_name, list);
        let init_id = stable_node_id(&init_key);
        let init_node = Node {
            node_id: init_id.clone(),
            kind: NodeKind::Control,
            op: Op {
                name: "reduce_init".into(),
                signature: "reduce_init()->Any".into(),
            },
            inputs: vec![],
            outputs: vec![OutputPort {
                port: "output".into(),
                type_name: "Any".into(),
            }],
            attrs: NodeAttrs {
                determinism_profile: DeterminismProfile::Fixed,
                cost_hints: None,
                url_index: None,
            },
            constraints: Vec::new(),
            meta: None,
            import_subgraph_id: None,
        };
        self.entry_nodes.push(init_id.clone());
        self.nodes.push(init_node);

        // Current accumulator symbol — starts pointing to init_node.
        let mut acc_sym = Symbol::NodePort {
            node_id: init_id,
            port: "output".into(),
            type_name: "Any".into(),
        };

        let mut reduce_node_ids: Vec<String> = Vec::new();
        let nodes_before = self.nodes.len();

        for i in 0..self.url_count {
            let ui = i as u32;
            let mut child_scope = Scope::new();
            child_scope.set(binder, Symbol::UrlParam { url_index: ui });
            child_scope.set(accum, acc_sym.clone());

            let nodes_before_iter = self.nodes.len();
            self.lower_stmts_reduce(body, &mut child_scope, Some(ui), accum)?;

            // The last node added should be the assignment result.
            let new_nodes = &self.nodes[nodes_before_iter..];
            if let Some(last) = new_nodes.last() {
                // Update acc_sym to point to the assignment node's output.
                acc_sym = Symbol::NodePort {
                    node_id: last.node_id.clone(),
                    port: "output".into(),
                    type_name: last.outputs.first().map(|o| o.type_name.clone()).unwrap_or_else(|| "Any".into()),
                };
                for n in new_nodes {
                    reduce_node_ids.push(n.node_id.clone());
                }
            }
        }

        // Expose the final accumulator as a named binding in the parent scope.
        parent_scope.set(accum, acc_sym);

        // Register a subgraph for the reduce.
        let subgraph_id =
            compute_stable_id("subgraph", &format!("{}.reduce.{}", self.goal_name, list))
                .to_string();
        self.subgraphs.push(Subgraph {
            subgraph_id,
            kind: "reduce".into(),
            nodes: reduce_node_ids,
            constraints: Vec::new(),
        });

        Ok(())
    }

    /// Lower a reduce body — same as `lower_stmts` but allows `Stmt::Assign`
    /// and validates the body ends with an assignment to `accum_name`.
    fn lower_stmts_reduce(
        &mut self,
        stmts: &[Stmt],
        scope: &mut Scope,
        url_index: Option<u32>,
        accum_name: &str,
    ) -> Result<(), LangError> {
        let has_assign = stmts.iter().any(|s| matches!(s, Stmt::Assign { name, .. } if name == accum_name));
        if !has_assign {
            return Err(LangError::ReduceBodyMissingAssign(accum_name.to_string()));
        }
        for stmt in stmts {
            self.lower_stmt(stmt, scope, url_index)?;
        }
        Ok(())
    }

    // `ACCUM = EXPR;` — update accumulator in scope.
    fn lower_assign(
        &mut self,
        name: &str,
        expr: &Expr,
        scope: &mut Scope,
        url_index: Option<u32>,
    ) -> Result<(), LangError> {
        let binder_name = format!("_assign_{}_{}", name, self.nodes.len());
        let (node_id, out_port, out_type) =
            self.lower_call_node(expr, scope, url_index, &binder_name)?;
        scope.set(
            name,
            Symbol::NodePort {
                node_id,
                port: out_port,
                type_name: out_type,
            },
        );
        Ok(())
    }

    // `emit EXPR;`
    // EXPR can be a call (`emit fn(arg);`) or an identifier (`emit bound_name;`).
    fn lower_emit(
        &mut self,
        expr: &Expr,
        scope: &mut Scope,
        url_index: Option<u32>,
    ) -> Result<(), LangError> {
        let ui = url_index.unwrap_or(0);
        match expr {
            Expr::Ident { name } => {
                // `emit ident;` — look up the bound symbol and collect its output port.
                let sym = scope
                    .get(name)
                    .cloned()
                    .ok_or_else(|| LangError::UndefinedBinding(name.clone()))?;
                match sym {
                    Symbol::NodePort {
                        node_id,
                        port,
                        type_name,
                    } => {
                        self.collected.push((ui, node_id, port, type_name));
                    }
                    Symbol::UrlParam { .. } => {
                        return Err(LangError::UnexpectedToken {
                            expected: "emit expression".into(),
                            got: format!("URL parameter '{name}' cannot be emitted directly"),
                            line: 0,
                        });
                    }
                }
            }
            Expr::Call { .. } => {
                let binder_name = format!("_emit_{}", self.collected.len());
                let (node_id, out_port, out_type) =
                    self.lower_call_node(expr, scope, url_index, &binder_name)?;
                self.collected.push((ui, node_id, out_port, out_type));
            }
            _ => {
                return Err(LangError::UnexpectedToken {
                    expected: "function call or identifier".into(),
                    got: "unsupported emit expression".into(),
                    line: 0,
                });
            }
        }
        Ok(())
    }

    // `return EXPR;`
    // EXPR is either:
    //   - a Call (the common case: `return assemble_json(collected)`)
    //   - an Ident bound to a node port (for `return acc;` in reduce)
    fn lower_return(
        &mut self,
        expr: &Expr,
        scope: &mut Scope,
        _url_index: Option<u32>,
    ) -> Result<(), LangError> {
        // Handle `return ident;` where the ident is a bound NodePort (e.g. reduce accumulator).
        if let Expr::Ident { name } = expr {
            if name != "collected" {
                let sym = scope
                    .get(name)
                    .cloned()
                    .ok_or_else(|| LangError::UndefinedBinding(name.clone()))?;
                match sym {
                    Symbol::NodePort { node_id, .. } => {
                        self.exit_node = Some(node_id);
                        return Ok(());
                    }
                    Symbol::UrlParam { .. } => {
                        return Err(LangError::UnexpectedToken {
                            expected: "bound value in return".into(),
                            got: "URL parameter cannot be returned directly".into(),
                            line: 0,
                        });
                    }
                }
            }
        }

        let (fn_name, _args) = match expr {
            Expr::Call { name, args } => (name.as_str(), args),
            Expr::Ident { name } if name == "collected" => {
                // bare `return collected;` — not valid, must be wrapped in call
                return Err(LangError::UnexpectedToken {
                    expected: "function call in return statement".into(),
                    got: "bare identifier 'collected'".into(),
                    line: 0,
                });
            }
            _ => {
                return Err(LangError::UnexpectedToken {
                    expected: "function call in return statement".into(),
                    got: "non-call expression".into(),
                    line: 0,
                });
            }
        };

        // Determine output type from intrinsic or cap declaration.
        let (kind, signature, out_type) = self.resolve_fn(fn_name)?;

        // Use Aggregate kind for the terminal node.
        let node_id_key = format!("{}.terminal.{}", self.goal_name, fn_name);
        let node_id = stable_node_id(&node_id_key);

        // Build input ports — one per collected item (ordered by url_index, then
        // insertion order within same url_index — our Vec is already correct).
        let mut input_ports: Vec<InputPort> = Vec::new();
        let mut input_edges: Vec<Edge> = Vec::new();

        let collected = self.collected.clone();
        for (idx, (ui, src_node_id, src_port, src_type)) in collected.iter().enumerate() {
            let port_name = format!("in_{}", idx);
            let edge = Edge {
                edge_id: "placeholder".into(),
                from: EdgeEndpoint {
                    node_id: src_node_id.clone(),
                    port: src_port.clone(),
                },
                to: EdgeEndpoint {
                    node_id: node_id.clone(),
                    port: port_name.clone(),
                },
            };
            let edge_id = compute_edge_id(&edge).to_string();
            let edge = Edge {
                edge_id: edge_id.clone(),
                ..edge
            };

            input_ports.push(InputPort {
                port: port_name,
                type_name: src_type.clone(),
                source: Some(EdgeRef { edge_id }),
            });
            input_edges.push(edge);
        }

        let out_port = "output".to_string();
        let output_ports = vec![OutputPort {
            port: out_port.clone(),
            type_name: out_type.clone(),
        }];

        let node = Node {
            node_id: node_id.clone(),
            kind: NodeKind::Aggregate,
            op: Op {
                name: fn_name.to_string(),
                signature,
            },
            inputs: input_ports,
            outputs: output_ports,
            attrs: NodeAttrs {
                determinism_profile: DeterminismProfile::Fixed,
                cost_hints: None,
                url_index: None,
            },
            constraints: Vec::new(),
            meta: None,
            import_subgraph_id: None,
        };

        self.edges.extend(input_edges);
        self.nodes.push(node);
        self.exit_node = Some(node_id);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Call-node creation
    // -----------------------------------------------------------------------

    /// Lower a `Call { name, args }` or `Pure { body }` expression into a node.
    /// Returns (node_id, output_port_name, output_type_name).
    fn lower_call_node(
        &mut self,
        expr: &Expr,
        scope: &Scope,
        url_index: Option<u32>,
        binder: &str,
    ) -> Result<(String, String, String), LangError> {
        // Unwrap `pure { CALL }` — validate the inner call is an intrinsic.
        let (fn_name, args, pure_block) = match expr {
            Expr::Call { name, args } => (name.as_str(), args.as_slice(), false),
            Expr::Pure { body } => {
                match body.as_ref() {
                    Expr::Call { name, args } => (name.as_str(), args.as_slice(), true),
                    _ => {
                        return Err(LangError::UnexpectedToken {
                            expected: "function call inside pure block".into(),
                            got: "non-call expression".into(),
                            line: 0,
                        })
                    }
                }
            }
            _ => {
                return Err(LangError::UnexpectedToken {
                    expected: "function call".into(),
                    got: "non-call expression".into(),
                    line: 0,
                })
            }
        };

        // Check for import call BEFORE cap/intrinsic lookup.
        if !pure_block && self.resolved_imports.contains_key(fn_name) {
            return self.lower_import_call(fn_name, args, scope, url_index, binder);
        }

        let (kind, signature, out_type) = self.resolve_fn(fn_name)?;

        // `pure { ... }` blocks may only contain intrinsics, not capabilities.
        if pure_block && kind == NodeKind::CapabilityCall {
            return Err(LangError::PureBlockContainsCapability(fn_name.to_string()));
        }

        let ui_label = url_index
            .map(|u| u.to_string())
            .unwrap_or_else(|| "none".into());
        let node_id_key = format!("{}.{}.{}.{}", self.goal_name, binder, fn_name, ui_label);
        let node_id = stable_node_id(&node_id_key);

        // Determine determinism profile from kind.
        let det_profile = match kind {
            NodeKind::CapabilityCall => DeterminismProfile::Replayable,
            _ => DeterminismProfile::Fixed,
        };

        // Build input ports from args.
        let mut input_ports: Vec<InputPort> = Vec::new();
        let mut new_edges: Vec<Edge> = Vec::new();

        let sig_args = parse_signature_args(&signature);

        for (i, arg) in args.iter().enumerate() {
            let port_name = format!("in_{}", i);
            let arg_type = sig_args.get(i).cloned().unwrap_or_else(|| "Unknown".into());

            match self.resolve_expr(arg, scope)? {
                Symbol::NodePort {
                    node_id: src_id,
                    port: src_port,
                    ..
                } => {
                    let edge = Edge {
                        edge_id: "placeholder".into(),
                        from: EdgeEndpoint {
                            node_id: src_id,
                            port: src_port,
                        },
                        to: EdgeEndpoint {
                            node_id: node_id.clone(),
                            port: port_name.clone(),
                        },
                    };
                    let edge_id = compute_edge_id(&edge).to_string();
                    let edge = Edge {
                        edge_id: edge_id.clone(),
                        ..edge
                    };
                    input_ports.push(InputPort {
                        port: port_name,
                        type_name: arg_type,
                        source: Some(EdgeRef { edge_id }),
                    });
                    new_edges.push(edge);
                }
                Symbol::UrlParam { url_index: ui } => {
                    // URL comes from runtime url_inputs[ui]; no edge needed.
                    input_ports.push(InputPort {
                        port: port_name,
                        type_name: arg_type,
                        source: None,
                    });
                }
            }
        }

        let out_port = "output".to_string();
        let output_ports = vec![OutputPort {
            port: out_port.clone(),
            type_name: out_type.clone(),
        }];

        let node = Node {
            node_id: node_id.clone(),
            kind,
            op: Op {
                name: fn_name.to_string(),
                signature,
            },
            inputs: input_ports,
            outputs: output_ports,
            attrs: NodeAttrs {
                determinism_profile: det_profile,
                cost_hints: None,
                url_index,
            },
            constraints: Vec::new(),
            meta: None,
            import_subgraph_id: None,
        };

        // If this node has no incoming edges from within the graph, it's an
        // entry node (its inputs come from the goal parameters or url_inputs).
        if new_edges.is_empty() {
            self.entry_nodes.push(node_id.clone());
        }

        self.edges.extend(new_edges);
        self.nodes.push(node);

        Ok((node_id, out_port, out_type))
    }

    // -----------------------------------------------------------------------
    // Import expansion
    // -----------------------------------------------------------------------

    /// Inline an imported sub-goal's Plan IR at the call site.
    ///
    /// Steps:
    /// 1. Remap every node/edge ID from the sub-IR to avoid collisions.
    /// 2. Copy nodes (updating `url_index` and tagging `import_subgraph_id`).
    /// 3. Copy and remap internal edges, updating input-port sources.
    /// 4. Wire the call-site argument to the sub-goal's entry node input(s).
    /// 5. Return (exit_node_new_id, "output", out_type) like a normal call node.
    fn lower_import_call(
        &mut self,
        import_name: &str,
        args: &[Expr],
        scope: &Scope,
        url_index: Option<u32>,
        binder: &str,
    ) -> Result<(String, String, String), LangError> {
        // Clone the sub-IR so we can inspect it without holding a borrow on self.
        let sub_ir = self.resolved_imports[import_name].clone();
        let import_hash = sub_ir
            .module
            .source_fingerprint
            .clone(); // used as attribution tag below
        // The canonical import_subgraph_id is the plan_ir_hash of the sub-IR.
        let import_subgraph_id = conclave_ir::compute_plan_ir_hash(&sub_ir).to_string();

        let sub_goal = sub_ir.goals.first().ok_or(LangError::NoGoals)?.clone();

        // --- Step 1: Build node_id_remap ---
        // Include url_index in the key so repeated inlining at different loop
        // iterations produces distinct node IDs.
        let ui_label = url_index
            .map(|u| u.to_string())
            .unwrap_or_else(|| "none".into());
        let mut node_id_remap: BTreeMap<String, String> = BTreeMap::new();
        for node in &sub_ir.nodes {
            let new_key = format!(
                "{}.import.{}.{}.{}.{}",
                self.goal_name, binder, import_name, ui_label, node.node_id
            );
            node_id_remap.insert(node.node_id.clone(), stable_node_id(&new_key));
        }

        // --- Step 2: Remap edges and build (new_to_node_id, port) → new_edge_id map ---
        let mut port_to_edge: BTreeMap<(String, String), String> = BTreeMap::new();
        let mut remapped_edges: Vec<Edge> = Vec::new();
        for edge in &sub_ir.edges {
            let new_from = node_id_remap
                .get(&edge.from.node_id)
                .cloned()
                .unwrap_or_else(|| edge.from.node_id.clone());
            let new_to = node_id_remap
                .get(&edge.to.node_id)
                .cloned()
                .unwrap_or_else(|| edge.to.node_id.clone());
            let placeholder = Edge {
                edge_id: "placeholder".into(),
                from: EdgeEndpoint { node_id: new_from, port: edge.from.port.clone() },
                to: EdgeEndpoint { node_id: new_to.clone(), port: edge.to.port.clone() },
            };
            let new_edge_id = compute_edge_id(&placeholder).to_string();
            port_to_edge.insert((new_to, edge.to.port.clone()), new_edge_id.clone());
            remapped_edges.push(Edge { edge_id: new_edge_id, ..placeholder });
        }

        // --- Step 3: Copy nodes with remapped IDs, updated url_index, and attribution tag ---
        let entry_set: std::collections::BTreeSet<&str> =
            sub_goal.entry_nodes.iter().map(|s| s.as_str()).collect();
        let mut inlined_node_ids: Vec<String> = Vec::new();

        for node in &sub_ir.nodes {
            let new_id = node_id_remap[&node.node_id].clone();
            inlined_node_ids.push(new_id.clone());
            let is_entry = entry_set.contains(node.node_id.as_str());

            let new_inputs: Vec<InputPort> = node
                .inputs
                .iter()
                .map(|inp| {
                    // Entry-node inputs will be wired in Step 4; leave source None for now.
                    let source = if is_entry {
                        None
                    } else {
                        port_to_edge
                            .get(&(new_id.clone(), inp.port.clone()))
                            .map(|eid| EdgeRef { edge_id: eid.clone() })
                    };
                    InputPort {
                        port: inp.port.clone(),
                        type_name: inp.type_name.clone(),
                        source,
                    }
                })
                .collect();

            self.nodes.push(Node {
                node_id: new_id,
                kind: node.kind.clone(),
                op: node.op.clone(),
                inputs: new_inputs,
                outputs: node.outputs.clone(),
                attrs: NodeAttrs {
                    determinism_profile: node.attrs.determinism_profile.clone(),
                    cost_hints: node.attrs.cost_hints.clone(),
                    url_index, // caller's url_index
                },
                constraints: Vec::new(),
                meta: None,
                import_subgraph_id: Some(import_subgraph_id.clone()),
            });
        }
        self.edges.extend(remapped_edges);

        // --- Step 4: Wire call-site argument to entry node input(s) ---
        if let Some(first_arg) = args.first() {
            let arg_sym = self.resolve_expr(first_arg, scope)?;

            for entry_orig_id in &sub_goal.entry_nodes {
                let entry_new_id = match node_id_remap.get(entry_orig_id) {
                    Some(id) => id.clone(),
                    None => continue,
                };

                match &arg_sym {
                    Symbol::NodePort { node_id: src_id, port: src_port, .. } => {
                        let placeholder = Edge {
                            edge_id: "placeholder".into(),
                            from: EdgeEndpoint {
                                node_id: src_id.clone(),
                                port: src_port.clone(),
                            },
                            to: EdgeEndpoint {
                                node_id: entry_new_id.clone(),
                                port: "in_0".into(),
                            },
                        };
                        let wire_edge_id = compute_edge_id(&placeholder).to_string();
                        let wire_edge = Edge { edge_id: wire_edge_id.clone(), ..placeholder };

                        if let Some(en) =
                            self.nodes.iter_mut().find(|n| n.node_id == entry_new_id)
                        {
                            if let Some(inp) =
                                en.inputs.iter_mut().find(|p| p.port == "in_0")
                            {
                                inp.source = Some(EdgeRef { edge_id: wire_edge_id });
                            }
                        }
                        self.edges.push(wire_edge);
                    }
                    Symbol::UrlParam { .. } => {
                        // URL comes from the runtime's url_inputs; no edge needed.
                        // url_index on the node's attrs already points to the right slot.
                    }
                }
            }
        }

        // --- Step 5: Determine exit node and output type ---
        let exit_orig_id = sub_goal.exit_nodes.first().ok_or_else(|| {
            LangError::UnexpectedToken {
                expected: "exit node in imported sub-goal".into(),
                got: "imported Plan IR has no exit nodes".into(),
                line: 0,
            }
        })?;
        let exit_new_id = node_id_remap
            .get(exit_orig_id)
            .cloned()
            .unwrap_or_else(|| exit_orig_id.clone());

        let out_type = self
            .nodes
            .iter()
            .find(|n| n.node_id == exit_new_id)
            .and_then(|n| n.outputs.first())
            .map(|o| o.type_name.clone())
            .unwrap_or_else(|| "Any".into());

        // --- Step 6: Register an "import" subgraph ---
        let sg_key = format!(
            "{}.import.{}.{}.{}",
            self.goal_name, binder, import_name, ui_label
        );
        self.subgraphs.push(Subgraph {
            subgraph_id: compute_stable_id("subgraph", &sg_key).to_string(),
            kind: "import".into(),
            nodes: inlined_node_ids,
            constraints: Vec::new(),
        });

        Ok((exit_new_id, "output".into(), out_type))
    }

    // -----------------------------------------------------------------------
    // Symbol resolution
    // -----------------------------------------------------------------------

    fn resolve_expr(&self, expr: &Expr, scope: &Scope) -> Result<Symbol, LangError> {
        match expr {
            Expr::Ident { name } => scope
                .get(name)
                .cloned()
                .ok_or_else(|| LangError::UndefinedBinding(name.clone())),
            Expr::StringLit { value } => {
                Err(LangError::UnexpectedToken {
                    expected: "identifier or call expression".into(),
                    got: format!("string literal '{value}'"),
                    line: 0,
                })
            }
            Expr::Call { .. } | Expr::Pure { .. } => {
                // Nested call/pure — not supported as argument in v0.1.
                Err(LangError::UnexpectedToken {
                    expected: "identifier".into(),
                    got: "nested call or pure expression".into(),
                    line: 0,
                })
            }
        }
    }

    /// Resolve a function name to (NodeKind, canonical_signature, output_type).
    fn resolve_fn(&self, fn_name: &str) -> Result<(NodeKind, String, String), LangError> {
        if let Some(cap) = self.cap_map.get(fn_name) {
            let out_type = parse_signature_return(&cap.signature);
            return Ok((NodeKind::CapabilityCall, cap.signature.clone(), out_type));
        }
        if let Some(intr) = self.intr_map.get(fn_name) {
            let out_type = parse_signature_return(&intr.signature);
            return Ok((NodeKind::Intrinsic, intr.signature.clone(), out_type));
        }
        Err(LangError::UnknownCapability(fn_name.to_string()))
    }

    // -----------------------------------------------------------------------
    // Constraint lowering
    // -----------------------------------------------------------------------

    fn lower_constraint(&mut self, cexpr: &ConstraintExpr, _idx: usize) -> Result<(), LangError> {
        let (key, constraint_key, ast) = build_constraint_ast(cexpr);

        // Assign an ID via compute_constraint_id (content-addressed).
        let mut c = Constraint {
            constraint_id: "placeholder".into(),
            scope: ConstraintScope::Goal,
            expr: IrConstraintExpr {
                lang: "conclave_v0.1".into(),
                ast: ast.clone(),
            },
        };
        let cid = compute_constraint_id(&c).to_string();
        c.constraint_id = cid;

        self.constraints.insert(constraint_key, c);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Plan IR assembly
    // -----------------------------------------------------------------------

    fn build_plan_ir(mut self, module: &Module, goal_decl: &GoalDecl, source: &str) -> PlanIr {
        // Compute goal-level constraint refs.
        let goal_constraint_refs: Vec<ConstraintRef> = self
            .constraints
            .values()
            .map(|c| ConstraintRef {
                ref_path: format!("#/constraints/{}", c.constraint_id),
            })
            .collect();

        // Collect exit nodes.
        let exit_nodes = self.exit_node.iter().cloned().collect::<Vec<_>>();

        // Deduplicate entry nodes (stable order is maintained by Vec insertion).
        let mut seen_entry = std::collections::BTreeSet::new();
        let entry_nodes: Vec<String> = self
            .entry_nodes
            .iter()
            .filter(|id| seen_entry.insert((*id).clone()))
            .cloned()
            .collect();

        // Build the Goal IR object.
        let goal_goal = Goal {
            goal_id: "placeholder".into(),
            name: goal_decl.name.clone(),
            params: goal_decl
                .params
                .iter()
                .map(|p| GoalParam {
                    name: p.name.clone(),
                    type_name: p.type_name.clone(),
                })
                .collect(),
            returns: vec![GoalParam {
                name: "result".into(),
                type_name: goal_decl.returns.clone(),
            }],
            constraints: goal_constraint_refs,
            accept: Vec::new(),
            entry_nodes,
            exit_nodes,
        };
        let goal_id = compute_goal_id(&goal_goal).to_string();
        let goal_goal = Goal {
            goal_id,
            ..goal_goal
        };

        // Re-compute stable node IDs (they were already computed during
        // lowering; nodes may have had their edge refs filled in, so recompute).
        // Actually node_ids are pre-assigned; no need to recompute.

        // Source fingerprint.
        let source_fp = sha256_str(source).to_string();

        // Types from the DSL module.
        let mut types: std::collections::BTreeMap<String, conclave_ir::TypeDef> =
            std::collections::BTreeMap::new();
        for td in &module.types {
            let predicates = td.constraint.as_ref().map(|tc| {
                vec![conclave_ir::Predicate {
                    lang: tc.validator.clone(),
                    expr: tc.pattern.clone(),
                }]
            });
            types.insert(
                td.name.clone(),
                conclave_ir::TypeDef {
                    kind: "alias".into(),
                    of: Some(td.base.clone()),
                    fields: None,
                    variants: None,
                    predicates,
                },
            );
        }

        let imports: BTreeMap<String, String> = module
            .imports
            .iter()
            .map(|imp| (imp.name.clone(), imp.hash.clone()))
            .collect();

        PlanIr {
            conclave_ir_version: "0.1".into(),
            module: IrModule {
                name: goal_decl.name.clone(),
                source_fingerprint: source_fp,
            },
            imports,
            types,
            goals: vec![goal_goal],
            nodes: self.nodes,
            edges: self.edges,
            constraints: self.constraints,
            subgraphs: self.subgraphs,
            exports: Exports {
                entry_goal: goal_decl.name.clone(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Scope (symbol table for one nesting level)
// ---------------------------------------------------------------------------

struct Scope {
    bindings: BTreeMap<String, Symbol>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            bindings: BTreeMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&Symbol> {
        self.bindings.get(name)
    }

    fn set(&mut self, name: &str, sym: Symbol) {
        self.bindings.insert(name.to_string(), sym);
    }

    /// Create a child scope pre-populated with the current scope's bindings,
    /// plus a special `__branch_gate` binding for `if` branching context.
    fn child_with_gate(&self, gate_id: &str, branch: &str) -> Scope {
        let mut child = Scope {
            bindings: self.bindings.clone(),
        };
        // The gate output port is available as a condition signal in the branch.
        child.set(
            &format!("__branch_{}", branch),
            Symbol::NodePort {
                node_id: gate_id.to_string(),
                port: branch.to_string(),
                type_name: "Bool".into(),
            },
        );
        child
    }
}

// ---------------------------------------------------------------------------
// AST call-name scanner (for selective import pre-resolution)
// ---------------------------------------------------------------------------

/// Recursively collect every function-call name reachable from `stmts`.
fn collect_call_names(stmts: &[Stmt], out: &mut std::collections::BTreeSet<String>) {
    for stmt in stmts {
        collect_call_names_in_stmt(stmt, out);
    }
}

fn collect_call_names_in_stmt(stmt: &Stmt, out: &mut std::collections::BTreeSet<String>) {
    match stmt {
        Stmt::Let { expr, .. }
        | Stmt::Emit { expr }
        | Stmt::Return { expr }
        | Stmt::Assign { expr, .. } => collect_call_names_in_expr(expr, out),
        Stmt::Map { body, .. } | Stmt::Reduce { body, .. } => collect_call_names(body, out),
        Stmt::If {
            condition,
            true_body,
            false_body,
        } => {
            collect_call_names_in_expr(condition, out);
            collect_call_names(true_body, out);
            collect_call_names(false_body, out);
        }
    }
}

fn collect_call_names_in_expr(expr: &Expr, out: &mut std::collections::BTreeSet<String>) {
    match expr {
        Expr::Call { name, args } => {
            out.insert(name.clone());
            for arg in args {
                collect_call_names_in_expr(arg, out);
            }
        }
        Expr::Pure { body } => collect_call_names_in_expr(body, out),
        Expr::Ident { .. } | Expr::StringLit { .. } => {}
    }
}

// ---------------------------------------------------------------------------
// Signature helpers
// ---------------------------------------------------------------------------

/// Extract the return type from a normalized signature: `"fetch(Url)->Html"` → `"Html"`.
fn parse_signature_return(sig: &str) -> String {
    if let Some(pos) = sig.rfind("->") {
        sig[pos + 2..].to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Extract argument types from a normalized signature: `"fetch(Url)->Html"` → `["Url"]`.
fn parse_signature_args(sig: &str) -> Vec<String> {
    // Find the first '(' and last ')' before '->'
    let arrow_pos = sig.rfind("->").unwrap_or(sig.len());
    let paren_start = sig.find('(').unwrap_or(0);
    let paren_end = sig[..arrow_pos].rfind(')').unwrap_or(arrow_pos);
    let args_str = &sig[paren_start + 1..paren_end];
    if args_str.is_empty() {
        return Vec::new();
    }
    // Split on commas at depth 0 (respects List<X> nesting).
    split_args(args_str)
}

fn split_args(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = s[start..i].trim().to_string();
                if !arg.is_empty() {
                    result.push(arg);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = s[start..].trim().to_string();
    if !last.is_empty() {
        result.push(last);
    }
    result
}

// ---------------------------------------------------------------------------
// Stable node ID helper
// ---------------------------------------------------------------------------

fn stable_node_id(key: &str) -> String {
    compute_stable_id("node", key).to_string()
}

// ---------------------------------------------------------------------------
// Constraint AST builder
// ---------------------------------------------------------------------------

fn build_constraint_ast(cexpr: &ConstraintExpr) -> (String, String, serde_json::Value) {
    let op_str = match cexpr.op {
        CmpOp::Eq => "==",
        CmpOp::LtEq => "<=",
    };

    let (key, constraint_key, left_ast) = match &cexpr.left {
        ConstraintLeft::Path { segments } => {
            let path = segments.join(".");
            let ck = segments.join(":");
            let ast = serde_json::json!({ "path": segments });
            (path, ck, ast)
        }
        ConstraintLeft::FnCall { name, args } => {
            let path = format!("{}({})", name, args.join(", "));
            let ck = format!("{}:{}", name, args.join(":"));
            let arg_vals: Vec<serde_json::Value> = args
                .iter()
                .map(|a| serde_json::json!({ "ident": a }))
                .collect();
            let ast = serde_json::json!({ "fn": name, "args": arg_vals });
            (path, ck, ast)
        }
    };

    let right_ast = match &cexpr.right {
        ConstraintValue::Number { value } => serde_json::json!({ "number": value }),
        ConstraintValue::Rate { value, unit } => {
            serde_json::json!({ "rate": value, "unit": unit })
        }
        ConstraintValue::StringLit { value } => serde_json::json!({ "string": value }),
    };

    let ast = serde_json::json!({
        "op":    op_str,
        "left":  left_ast,
        "right": right_ast,
    });

    (key, constraint_key, ast)
}
