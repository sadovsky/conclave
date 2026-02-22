use conclave_hash::{
    remove_field, remove_field_at_path, remove_field_recursive, to_canonical_json,
};
use serde_json::json;

#[test]
fn sorted_keys() {
    let input = json!({"z": 1, "a": 2, "m": 3});
    assert_eq!(to_canonical_json(&input), r#"{"a":2,"m":3,"z":1}"#);
}

#[test]
fn sorted_keys_nested() {
    let input = json!({"b": {"z": 1, "a": 2}, "a": 3});
    assert_eq!(to_canonical_json(&input), r#"{"a":3,"b":{"a":2,"z":1}}"#);
}

#[test]
fn numeric_normalization_float_to_int() {
    let input = json!({"n": 1.0});
    assert_eq!(to_canonical_json(&input), r#"{"n":1}"#);
}

#[test]
fn numeric_normalization_integer_unchanged() {
    let input = json!({"n": 42});
    assert_eq!(to_canonical_json(&input), r#"{"n":42}"#);
}

#[test]
fn numeric_normalization_negative() {
    let input = json!({"n": -5.0});
    assert_eq!(to_canonical_json(&input), r#"{"n":-5}"#);
}

#[test]
fn arrays_preserve_order() {
    let input = json!([3, 1, 2]);
    assert_eq!(to_canonical_json(&input), "[3,1,2]");
}

#[test]
fn arrays_with_objects_sorted_internally() {
    let input = json!([{"b": 1, "a": 2}]);
    assert_eq!(to_canonical_json(&input), r#"[{"a":2,"b":1}]"#);
}

#[test]
fn null_string_bool_unchanged() {
    let input = json!({"a": null, "b": true, "c": false, "d": "hello"});
    assert_eq!(
        to_canonical_json(&input),
        r#"{"a":null,"b":true,"c":false,"d":"hello"}"#
    );
}

#[test]
fn empty_object() {
    let input = json!({});
    assert_eq!(to_canonical_json(&input), "{}");
}

#[test]
fn empty_array() {
    let input = json!([]);
    assert_eq!(to_canonical_json(&input), "[]");
}

#[test]
fn remove_field_top_level() {
    let mut value = json!({"a": 1, "b": 2});
    remove_field(&mut value, "a");
    assert_eq!(to_canonical_json(&value), r#"{"b":2}"#);
}

#[test]
fn remove_field_missing_is_noop() {
    let mut value = json!({"a": 1});
    remove_field(&mut value, "x");
    assert_eq!(to_canonical_json(&value), r#"{"a":1}"#);
}

#[test]
fn remove_field_recursive_deep() {
    let mut value = json!({"a": 1, "meta": "x", "nested": {"meta": "y", "b": 2}});
    remove_field_recursive(&mut value, "meta");
    assert_eq!(to_canonical_json(&value), r#"{"a":1,"nested":{"b":2}}"#);
}

#[test]
fn remove_field_recursive_in_array() {
    let mut value = json!([{"meta": 1, "a": 2}, {"b": 3}]);
    remove_field_recursive(&mut value, "meta");
    assert_eq!(to_canonical_json(&value), r#"[{"a":2},{"b":3}]"#);
}

#[test]
fn remove_field_at_path_nested() {
    let mut value =
        json!({"supply_chain": {"manifest_signature": {"signature": "abc", "algo": "ed25519"}}});
    remove_field_at_path(
        &mut value,
        &["supply_chain", "manifest_signature", "signature"],
    );
    assert_eq!(
        to_canonical_json(&value),
        r#"{"supply_chain":{"manifest_signature":{"algo":"ed25519"}}}"#
    );
}

#[test]
fn no_insignificant_whitespace() {
    let input = json!({"a": 1, "b": [2, 3]});
    let out = to_canonical_json(&input);
    assert!(!out.contains(' '), "found whitespace in: {out}");
    assert!(!out.contains('\n'), "found newline in: {out}");
}
