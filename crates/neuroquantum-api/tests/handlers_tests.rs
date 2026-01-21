//! Tests for JSON conversion in handlers
//!
//! These tests validate the json_to_storage_value function for converting
//! JSON values to the internal storage format.

use neuroquantum_api::json_to_storage_value;
use neuroquantum_core::storage::Value;

#[test]
fn test_json_to_storage_value_integer() {
    let json = serde_json::json!(42);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Integer(i) => assert_eq!(i, 42),
        | _ => panic!("Expected Integer value"),
    }
}

#[test]
fn test_json_to_storage_value_negative_integer() {
    let json = serde_json::json!(-100);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Integer(i) => assert_eq!(i, -100),
        | _ => panic!("Expected Integer value"),
    }
}

#[test]
fn test_json_to_storage_value_float() {
    let json = serde_json::json!(42.5);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Float(f) => assert!((f - 42.5).abs() < 0.001),
        | _ => panic!("Expected Float value"),
    }
}

#[test]
fn test_json_to_storage_value_string() {
    let json = serde_json::json!("hello world");
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Text(s) => assert_eq!(s.as_str(), "hello world"),
        | _ => panic!("Expected Text value"),
    }
}

#[test]
fn test_json_to_storage_value_boolean_true() {
    let json = serde_json::json!(true);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Boolean(b) => assert!(b),
        | _ => panic!("Expected Boolean value"),
    }
}

#[test]
fn test_json_to_storage_value_boolean_false() {
    let json = serde_json::json!(false);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Boolean(b) => assert!(!b),
        | _ => panic!("Expected Boolean value"),
    }
}

#[test]
fn test_json_to_storage_value_null() {
    let json = serde_json::Value::Null;
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Null => {},
        | _ => panic!("Expected Null value"),
    }
}

#[test]
fn test_json_to_storage_value_array_converts_to_text() {
    let json = serde_json::json!([1, 2, 3]);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Text(s) => assert!(s.as_str().contains("[1,2,3]")),
        | _ => panic!("Expected Text value for array"),
    }
}

#[test]
fn test_json_to_storage_value_object_converts_to_text() {
    let json = serde_json::json!({"key": "value"});
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Text(s) => assert!(s.as_str().contains("key") && s.as_str().contains("value")),
        | _ => panic!("Expected Text value for object"),
    }
}

#[test]
fn test_json_to_storage_value_large_integer() {
    // Test with i64::MAX which should be valid
    let json = serde_json::json!(i64::MAX);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Integer(i) => assert_eq!(i, i64::MAX),
        | _ => panic!("Expected Integer value"),
    }
}

#[test]
fn test_json_to_storage_value_zero() {
    let json = serde_json::json!(0);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Integer(i) => assert_eq!(i, 0),
        | _ => panic!("Expected Integer value"),
    }
}

#[test]
fn test_json_to_storage_value_zero_float() {
    let json = serde_json::json!(0.0);
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Float(f) => assert!((f - 0.0).abs() < f64::EPSILON),
        | _ => panic!("Expected Float value"),
    }
}

#[test]
fn test_json_to_storage_value_empty_string() {
    let json = serde_json::json!("");
    let result = json_to_storage_value(&json, "test_field");
    assert!(result.is_ok());
    match result.unwrap() {
        | Value::Text(s) => assert!(s.as_str().is_empty()),
        | _ => panic!("Expected Text value"),
    }
}
