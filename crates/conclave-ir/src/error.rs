#[derive(Debug, thiserror::Error)]
pub enum IrError {
    #[error("unsupported conclave_ir_version: expected \"0.1\", got \"{0}\"")]
    UnsupportedVersion(String),
    #[error("duplicate node_id: {0}")]
    DuplicateNodeId(String),
    #[error("duplicate edge_id: {0}")]
    DuplicateEdgeId(String),
    #[error("edge {edge_id} references unknown node_id: {node_id}")]
    EdgeReferencesUnknownNode { edge_id: String, node_id: String },
    #[error("constraint ref {ref_path} does not resolve")]
    UnresolvedConstraintRef { ref_path: String },
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
