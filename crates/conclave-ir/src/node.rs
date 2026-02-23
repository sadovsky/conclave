/// A node in the Plan IR graph.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub node_id: String,
    pub kind: NodeKind,
    pub op: Op,
    pub inputs: Vec<InputPort>,
    pub outputs: Vec<OutputPort>,
    pub attrs: NodeAttrs,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<ConstraintRef>,
    /// Present for parsing; excluded from hashing via canonicalize_plan_ir.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
    /// Attribution tag: the plan_ir_hash of the imported module this node came from.
    /// Set by the lowerer when expanding an `import` declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_subgraph_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    CapabilityCall,
    Intrinsic,
    Control,
    Aggregate,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Op {
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputPort {
    pub port: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<EdgeRef>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutputPort {
    pub port: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeRef {
    pub edge_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeAttrs {
    pub determinism_profile: DeterminismProfile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_hints: Option<CostHints>,
    /// url_index for scheduler ordering within map constructs (u32::MAX sentinel = no index).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_index: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismProfile {
    Replayable,
    Fixed,
    Nondet,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CostHints {
    pub latency: String,
    pub cpu: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstraintRef {
    #[serde(rename = "$ref")]
    pub ref_path: String,
}
