use std::collections::BTreeMap;

/// A structured, deterministic runtime error.
///
/// Serializes to canonical JSON — no nondeterministic stack traces.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, thiserror::Error)]
#[error("{code}")]
pub struct RuntimeError {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default)]
    pub details: BTreeMap<String, serde_json::Value>,
}

impl RuntimeError {
    pub fn new(code: &str) -> Self {
        RuntimeError {
            code: code.to_string(),
            node_id: None,
            capability: None,
            details: BTreeMap::new(),
        }
    }

    pub fn with_node(mut self, node_id: &str) -> Self {
        self.node_id = Some(node_id.to_string());
        self
    }

    pub fn with_capability(mut self, capability: &str) -> Self {
        self.capability = Some(capability.to_string());
        self
    }

    pub fn with_detail(mut self, key: &str, value: serde_json::Value) -> Self {
        self.details.insert(key.to_string(), value);
        self
    }
}
