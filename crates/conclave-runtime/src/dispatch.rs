use crate::error::RuntimeError;
use std::collections::BTreeMap;

/// An immutable value blob tagged with its type.
#[derive(Debug, Clone)]
pub struct Value {
    pub type_name: String,
    pub data: Vec<u8>,
}

/// A deterministic response from a replay store.
pub struct ReplayEntry {
    pub output: Value,
    pub duration_ms: u64,
}

/// A read-only replay store keyed by (capability_signature, normalized_request_key).
pub trait ReplayStore {
    fn get(&self, capability: &str, normalized_key: &str) -> Option<ReplayEntry>;
}

/// A no-op replay store that always returns a miss.
pub struct EmptyReplayStore;

impl ReplayStore for EmptyReplayStore {
    fn get(&self, _capability: &str, _normalized_key: &str) -> Option<ReplayEntry> {
        None
    }
}

/// A simple in-memory replay store backed by a BTreeMap.
///
/// Keyed by "capability::normalized_key".
pub struct MapReplayStore {
    entries: BTreeMap<String, (Vec<u8>, String, u64)>, // key -> (data, type_name, duration_ms)
}

impl MapReplayStore {
    pub fn new() -> Self {
        MapReplayStore {
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, capability: &str, key: &str, data: Vec<u8>, type_name: &str, duration_ms: u64) {
        let map_key = format!("{}::{}", capability, key);
        self.entries.insert(map_key, (data, type_name.to_string(), duration_ms));
    }
}

impl Default for MapReplayStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayStore for MapReplayStore {
    fn get(&self, capability: &str, normalized_key: &str) -> Option<ReplayEntry> {
        let map_key = format!("{}::{}", capability, normalized_key);
        self.entries.get(&map_key).map(|(data, type_name, duration_ms)| ReplayEntry {
            output: Value {
                type_name: type_name.clone(),
                data: data.clone(),
            },
            duration_ms: *duration_ms,
        })
    }
}

/// Dispatch result: either a completed output with duration, or an error.
pub enum DispatchResult {
    Ok { output: Value, duration_ms: u64 },
    Err(RuntimeError),
}

/// Dispatch a capability call using the replay store.
pub fn dispatch_capability(
    node_id: &str,
    capability_signature: &str,
    normalized_key: &str,
    replay_store: &dyn ReplayStore,
) -> DispatchResult {
    match replay_store.get(capability_signature, normalized_key) {
        Some(entry) => DispatchResult::Ok {
            output: entry.output,
            duration_ms: entry.duration_ms,
        },
        None => DispatchResult::Err(
            RuntimeError::new("ERR_REPLAY_MISS")
                .with_node(node_id)
                .with_capability(capability_signature)
                .with_detail("normalized_key", serde_json::Value::String(normalized_key.to_string())),
        ),
    }
}
