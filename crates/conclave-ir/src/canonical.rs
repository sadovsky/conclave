use crate::{Constraint, Edge, Goal, Node, PlanIr, Subgraph};
use conclave_hash::{
    compute_stable_id, remove_field_recursive, sha256_str, to_canonical_json, Hash,
};
use serde_json::Value;

/// Produce a canonical `serde_json::Value` from a Plan IR, suitable for hashing.
///
/// - Removes all `meta` fields recursively.
/// - Sorts nodes by node_id, edges by edge_id, goals by goal_id, subgraphs by subgraph_id.
/// - Constraints BTreeMap is already sorted by key.
/// - Passes through `to_canonical_json` for final key-sorted, number-normalized encoding.
pub fn canonicalize_plan_ir(ir: &PlanIr) -> Value {
    // Serialize to Value, then strip meta, then re-canonicalize.
    let mut v: Value = serde_json::to_value(ir).expect("PlanIr is always serializable");
    remove_field_recursive(&mut v, "meta");

    // Sort nodes/edges/goals/subgraphs by their id fields.
    sort_array_by_field(&mut v, "nodes", "node_id");
    sort_array_by_field(&mut v, "edges", "edge_id");
    sort_array_by_field(&mut v, "goals", "goal_id");
    sort_array_by_field(&mut v, "subgraphs", "subgraph_id");

    // Apply canonical JSON (sorted object keys, numeric normalization).
    let canonical_str = to_canonical_json(&v);
    serde_json::from_str(&canonical_str).expect("re-parse of canonical JSON always succeeds")
}

fn sort_array_by_field(root: &mut Value, array_field: &str, id_field: &str) {
    if let Some(arr) = root
        .as_object_mut()
        .and_then(|o| o.get_mut(array_field))
        .and_then(|v| v.as_array_mut())
    {
        arr.sort_by(|a, b| {
            let a_id = a.get(id_field).and_then(|v| v.as_str()).unwrap_or("");
            let b_id = b.get(id_field).and_then(|v| v.as_str()).unwrap_or("");
            a_id.cmp(b_id)
        });
    }
}

/// Compute `plan_ir_hash = sha256(canonical_plan_ir_json)`.
pub fn compute_plan_ir_hash(ir: &PlanIr) -> Hash {
    let canonical = canonicalize_plan_ir(ir);
    let canonical_str = to_canonical_json(&canonical);
    sha256_str(&canonical_str)
}

// ---------------------------------------------------------------------------
// Stable ID computation
// ---------------------------------------------------------------------------
// IDs are computed from canonical bodies that EXCLUDE the id field itself.
// Bootstrapping order: node_ids first (no edge refs needed), then edge_ids
// (which use node_ids), then goal_ids and constraint_ids.

/// Compute a stable `node_id`.
/// Body: {kind, op, inputs (port+type only, no edge_id), outputs, attrs, constraints}
pub fn compute_node_id(node: &Node) -> Hash {
    // Build a minimal canonical body excluding the id and meta.
    let body = serde_json::json!({
        "attrs": serde_json::to_value(&node.attrs).unwrap(),
        "constraints": serde_json::to_value(&node.constraints).unwrap(),
        "inputs": node.inputs.iter().map(|p| serde_json::json!({
            "port": p.port,
            "type": p.type_name,
        })).collect::<Vec<_>>(),
        "kind": serde_json::to_value(&node.kind).unwrap(),
        "op": serde_json::to_value(&node.op).unwrap(),
        "outputs": serde_json::to_value(&node.outputs).unwrap(),
    });
    let canonical = to_canonical_json(&body);
    compute_stable_id("node", &canonical)
}

/// Compute a stable `edge_id`.
/// Body: {from, to} using already-resolved node_ids.
pub fn compute_edge_id(edge: &Edge) -> Hash {
    let body = serde_json::json!({
        "from": serde_json::to_value(&edge.from).unwrap(),
        "to": serde_json::to_value(&edge.to).unwrap(),
    });
    let canonical = to_canonical_json(&body);
    compute_stable_id("edge", &canonical)
}

/// Compute a stable `goal_id`.
/// Body: {name, params, returns, entry_nodes, exit_nodes, constraints, accept}
pub fn compute_goal_id(goal: &Goal) -> Hash {
    let body = serde_json::json!({
        "accept": serde_json::to_value(&goal.accept).unwrap(),
        "constraints": serde_json::to_value(&goal.constraints).unwrap(),
        "entry_nodes": serde_json::to_value(&goal.entry_nodes).unwrap(),
        "exit_nodes": serde_json::to_value(&goal.exit_nodes).unwrap(),
        "name": goal.name,
        "params": serde_json::to_value(&goal.params).unwrap(),
        "returns": serde_json::to_value(&goal.returns).unwrap(),
    });
    let canonical = to_canonical_json(&body);
    compute_stable_id("goal", &canonical)
}

/// Compute a stable `constraint_id`.
/// Body: {scope, expr}
pub fn compute_constraint_id(constraint: &Constraint) -> Hash {
    let body = serde_json::json!({
        "expr": serde_json::to_value(&constraint.expr).unwrap(),
        "scope": serde_json::to_value(&constraint.scope).unwrap(),
    });
    let canonical = to_canonical_json(&body);
    compute_stable_id("constraint", &canonical)
}

/// Compute a stable `subgraph_id`.
pub fn compute_subgraph_id(subgraph: &Subgraph) -> Hash {
    let body = serde_json::json!({
        "constraints": serde_json::to_value(&subgraph.constraints).unwrap(),
        "kind": subgraph.kind,
        "nodes": serde_json::to_value(&subgraph.nodes).unwrap(),
    });
    let canonical = to_canonical_json(&body);
    compute_stable_id("subgraph", &canonical)
}
