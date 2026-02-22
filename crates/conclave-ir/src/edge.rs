/// A value-dependency edge between two node ports.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    pub edge_id: String,
    pub from: EdgeEndpoint,
    pub to: EdgeEndpoint,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeEndpoint {
    pub node_id: String,
    pub port: String,
}
