use crate::node::ConstraintRef;

/// A top-level goal (entry point) in the Plan IR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Goal {
    pub goal_id: String,
    pub name: String,
    pub params: Vec<GoalParam>,
    pub returns: Vec<GoalParam>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<ConstraintRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accept: Vec<ConstraintRef>,
    pub entry_nodes: Vec<String>,
    pub exit_nodes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalParam {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}
