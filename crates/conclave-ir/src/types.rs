use std::collections::BTreeMap;

/// A named type definition in the Plan IR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeDef {
    pub kind: String, // "primitive" | "struct" | "list" | "map" | "union" | "alias"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicates: Option<Vec<Predicate>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Predicate {
    pub lang: String,
    pub expr: String,
}
