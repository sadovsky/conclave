/// A declarative constraint attached to a scope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Constraint {
    pub constraint_id: String,
    pub scope: ConstraintScope,
    pub expr: ConstraintExpr,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintScope {
    Goal,
    Node,
    Subgraph,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConstraintExpr {
    pub lang: String,
    /// AST-structured constraint expression; never a raw unparsed string.
    pub ast: serde_json::Value,
}
