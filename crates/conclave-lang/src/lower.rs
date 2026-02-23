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
/// `url_count` is the compile-time list length used to expand `map` constructs.
/// v0.1 decision: require list length as lowering input.
pub fn lower(source: &str, url_count: usize) -> Result<LowerOutput, LangError> {
    let normalized_source = source.replace("\r\n", "\n").replace('\r', "\n");
    let module = parse(&normalized_source)?;
    let module = normalize(module)?;

    let source_hash = sha256_str(&normalized_source).to_string();
    let ast_h = ast_hash(&module).to_string();

    let goal_decl = module.goals.first().ok_or(LangError::NoGoals)?;

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

    let mut state = LowerState::new(goal_decl.name.clone(), url_count, cap_map, intr_map);

    state.lower_goal(goal_decl)?;

    let plan_ir = state.build_plan_ir(&module, &normalized_source);
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
    /// The URL binder from a `map urls as url { ... }` at a given url_index.
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
    ) -> Self {
        LowerState {
            goal_name,
            url_count,
            cap_map,
            intr_map,
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

    // `return CALL(collected);`
    fn lower_return(
        &mut self,
        expr: &Expr,
        scope: &mut Scope,
        _url_index: Option<u32>,
    ) -> Result<(), LangError> {
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

    /// Lower a `Call { name, args }` expression into a node.
    /// Returns (node_id, output_port_name, output_type_name).
    fn lower_call_node(
        &mut self,
        expr: &Expr,
        scope: &Scope,
        url_index: Option<u32>,
        binder: &str,
    ) -> Result<(String, String, String), LangError> {
        let (fn_name, args) = match expr {
            Expr::Call { name, args } => (name.as_str(), args.as_slice()),
            _ => {
                return Err(LangError::UnexpectedToken {
                    expected: "function call".into(),
                    got: "non-call expression".into(),
                    line: 0,
                })
            }
        };

        let (kind, signature, out_type) = self.resolve_fn(fn_name)?;

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
    // Symbol resolution
    // -----------------------------------------------------------------------

    fn resolve_expr(&self, expr: &Expr, scope: &Scope) -> Result<Symbol, LangError> {
        match expr {
            Expr::Ident { name } => scope
                .get(name)
                .cloned()
                .ok_or_else(|| LangError::UndefinedBinding(name.clone())),
            Expr::StringLit { value } => {
                // String literal — produce a synthetic constant symbol.
                // In v0.1 only identifiers and calls are used as args; string
                // lits in positions other than constraint RHS are unusual.
                // Return a special NodePort pointing to a nonexistent "const" node.
                // (The lowerer does not create constant nodes in v0.1.)
                Err(LangError::UnexpectedToken {
                    expected: "identifier or call expression".into(),
                    got: format!("string literal '{value}'"),
                    line: 0,
                })
            }
            Expr::Call { .. } => {
                // Nested call — not supported in v0.1 args.
                Err(LangError::UnexpectedToken {
                    expected: "identifier".into(),
                    got: "nested call expression".into(),
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

    fn build_plan_ir(mut self, module: &Module, source: &str) -> PlanIr {
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
            name: module.goals[0].name.clone(),
            params: module.goals[0]
                .params
                .iter()
                .map(|p| GoalParam {
                    name: p.name.clone(),
                    type_name: p.type_name.clone(),
                })
                .collect(),
            returns: vec![GoalParam {
                name: "result".into(),
                type_name: module.goals[0].returns.clone(),
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
                name: module.goals[0].name.clone(),
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
                entry_goal: module.goals[0].name.clone(),
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
