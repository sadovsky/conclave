use crate::node::ConstraintRef;

/// A named subgraph (e.g. a map/reduce construct).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Subgraph {
    pub subgraph_id: String,
    pub kind: String, // "map" | "reduce" | "pipeline" | "branch"
    pub nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<ConstraintRef>,
}
