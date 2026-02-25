use crate::node::ConstraintRef;

/// A named subgraph (e.g. a map/reduce construct).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Subgraph {
    pub subgraph_id: String,
    pub kind: String, // "map" | "reduce" | "pipeline" | "branch" | "conditional_true" | "conditional_false" | "import"
    pub nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<ConstraintRef>,
    /// For conditional_true/conditional_false subgraphs: the node_id of the
    /// `conditional_branch` Control node that gates this subgraph.
    /// Excluded from subgraph_id computation; used only by the runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_node_id: Option<String>,
}
