use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;

/// Serialize a JSON value to canonical form:
/// - Object keys sorted lexicographically at every level
/// - Numbers normalized: integer form preferred (eliminates 1.0 vs 1 ambiguity)
/// - Array order preserved (array order is semantic)
/// - No insignificant whitespace
pub fn to_canonical_json(value: &Value) -> String {
    let canonical = canonicalize_value(value.clone());
    serde_json::to_string(&canonical).expect("canonical value is always serializable")
}

/// Recursively canonicalize a JSON value.
fn canonicalize_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            // Collect into BTreeMap to sort keys, then rebuild as serde_json::Map
            // (which is IndexMap-backed and preserves insertion order).
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, canonicalize_value(v)))
                .collect();
            let mut out = Map::with_capacity(sorted.len());
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        Value::Array(arr) => {
            // Preserve array order — it is semantic, not sorted.
            Value::Array(arr.into_iter().map(canonicalize_value).collect())
        }
        Value::Number(n) => {
            // Prefer integer representation to eliminate 1 vs 1.0 ambiguity.
            // First try the internal integer fast-path (works when the JSON source used
            // an integer literal). Then fall back to checking whether a float value
            // is exactly a whole number (e.g. Rust's json!(1.0) produces a float Number).
            if let Some(i) = n.as_i64() {
                Value::Number(Number::from(i))
            } else if let Some(u) = n.as_u64() {
                Value::Number(Number::from(u))
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                    Value::Number(Number::from(f as i64))
                } else {
                    // True floating-point value; canonical structs should avoid these.
                    Value::Number(n)
                }
            } else {
                Value::Number(n)
            }
        }
        // String, Bool, Null: unchanged.
        other => other,
    }
}

/// Remove a named key from the top-level of a JSON object (non-recursive).
pub fn remove_field(value: &mut Value, field: &str) {
    if let Value::Object(map) = value {
        map.remove(field);
    }
}

/// Remove all instances of a named key throughout the entire JSON tree (recursive).
pub fn remove_field_recursive(value: &mut Value, field: &str) {
    match value {
        Value::Object(map) => {
            map.remove(field);
            for v in map.values_mut() {
                remove_field_recursive(v, field);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                remove_field_recursive(v, field);
            }
        }
        _ => {}
    }
}

/// Remove a value at a specific dot-separated JSON path (e.g. "a.b.c").
/// Only removes the leaf key; parent objects remain.
pub fn remove_field_at_path(value: &mut Value, path: &[&str]) {
    match path {
        [] => {}
        [field] => remove_field(value, field),
        [head, rest @ ..] => {
            if let Value::Object(map) = value {
                if let Some(child) = map.get_mut(*head) {
                    remove_field_at_path(child, rest);
                }
            }
        }
    }
}
