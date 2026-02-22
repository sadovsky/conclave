pub mod canonical;
pub mod constraint;
pub mod edge;
pub mod error;
pub mod goal;
pub mod node;
pub mod subgraph;
pub mod types;

pub use canonical::{
    canonicalize_plan_ir, compute_constraint_id, compute_edge_id, compute_goal_id,
    compute_node_id, compute_plan_ir_hash, compute_subgraph_id,
};
pub use constraint::{Constraint, ConstraintExpr, ConstraintScope};
pub use edge::{Edge, EdgeEndpoint};
pub use error::IrError;
pub use goal::{Goal, GoalParam};
pub use node::{
    ConstraintRef, CostHints, DeterminismProfile, EdgeRef, InputPort, Node, NodeAttrs, NodeKind,
    Op, OutputPort,
};
pub use subgraph::Subgraph;
pub use types::{Predicate, TypeDef};

use std::collections::BTreeMap;

/// The complete Plan IR for one compilation unit.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanIr {
    pub conclave_ir_version: String,
    pub module: Module,
    #[serde(default)]
    pub types: BTreeMap<String, TypeDef>,
    #[serde(default)]
    pub goals: Vec<Goal>,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub constraints: BTreeMap<String, Constraint>,
    #[serde(default)]
    pub subgraphs: Vec<Subgraph>,
    pub exports: Exports,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Module {
    pub name: String,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Exports {
    pub entry_goal: String,
}

/// Validate structural invariants of a Plan IR.
pub fn validate_plan_ir(ir: &PlanIr) -> Result<(), IrError> {
    // Version check.
    if ir.conclave_ir_version != "0.1" {
        return Err(IrError::UnsupportedVersion(ir.conclave_ir_version.clone()));
    }

    // Collect all node_ids.
    let mut node_ids: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for node in &ir.nodes {
        if !node_ids.insert(node.node_id.as_str()) {
            return Err(IrError::DuplicateNodeId(node.node_id.clone()));
        }
    }

    // Collect all edge_ids and verify edge endpoints reference known nodes.
    let mut edge_ids: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for edge in &ir.edges {
        if !edge_ids.insert(edge.edge_id.as_str()) {
            return Err(IrError::DuplicateEdgeId(edge.edge_id.clone()));
        }
        if !node_ids.contains(edge.from.node_id.as_str()) {
            return Err(IrError::EdgeReferencesUnknownNode {
                edge_id: edge.edge_id.clone(),
                node_id: edge.from.node_id.clone(),
            });
        }
        if !node_ids.contains(edge.to.node_id.as_str()) {
            return Err(IrError::EdgeReferencesUnknownNode {
                edge_id: edge.edge_id.clone(),
                node_id: edge.to.node_id.clone(),
            });
        }
    }

    Ok(())
}
