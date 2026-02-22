use conclave_hash::{sha256_str, to_canonical_json, Hash};

/// A single scheduler event in the deterministic execution trace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TraceEvent {
    pub t: u64,
    pub event: String, // "DISPATCH" | "COMPLETE"
    pub node: String,  // symbolic node label (from node attrs or node_id)
}

/// Accumulates trace events in emission order.
pub struct TraceEmitter {
    events: Vec<TraceEvent>,
}

impl TraceEmitter {
    pub fn new() -> Self {
        TraceEmitter { events: Vec::new() }
    }

    /// Emit a DISPATCH event.
    pub fn dispatch(&mut self, t: u64, node: &str) {
        self.events.push(TraceEvent {
            t,
            event: "DISPATCH".into(),
            node: node.to_string(),
        });
    }

    /// Emit a COMPLETE event.
    pub fn complete(&mut self, t: u64, node: &str) {
        self.events.push(TraceEvent {
            t,
            event: "COMPLETE".into(),
            node: node.to_string(),
        });
    }

    pub fn events(&self) -> &[TraceEvent] {
        &self.events
    }

    /// Serialize all events to canonical JSON array (hashable trace representation).
    pub fn to_canonical_json(&self) -> String {
        let v = serde_json::to_value(&self.events).expect("TraceEvent is always serializable");
        to_canonical_json(&v)
    }

    /// Compute `trace_hash = sha256(canonical_trace_json)`.
    pub fn trace_hash(&self) -> Hash {
        sha256_str(&self.to_canonical_json())
    }
}

impl Default for TraceEmitter {
    fn default() -> Self {
        Self::new()
    }
}
