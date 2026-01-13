//! Fuzz target for REST API JSON payloads
//!
//! This fuzz target tests the API JSON parsing and validation with arbitrary
//! JSON payloads to find parsing bugs, validation bypasses, and edge cases.

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

/// Arbitrary JSON value generator for structured fuzzing
#[derive(Debug, Clone)]
struct ArbitraryJsonValue {
    value: Value,
}

impl<'a> Arbitrary<'a> for ArbitraryJsonValue {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let value = generate_json_value(u, 3)?;
        Ok(ArbitraryJsonValue { value })
    }
}

fn generate_json_value(u: &mut Unstructured, depth: u8) -> arbitrary::Result<Value> {
    if depth == 0 {
        // At max depth, only generate primitives
        return generate_primitive(u);
    }

    let choice: u8 = u.int_in_range(0..=6)?;
    match choice {
        0 => Ok(Value::Null),
        1 => Ok(Value::Bool(u.arbitrary()?)),
        2 => Ok(json!(u.arbitrary::<i64>()?)),
        3 => Ok(json!(u.arbitrary::<f64>()?)),
        4 => {
            let len: usize = u.int_in_range(0..=20)?;
            let s: String = (0..len)
                .map(|_| u.int_in_range(32u8..=126u8).unwrap_or(b'x') as char)
                .collect();
            Ok(Value::String(s))
        }
        5 => {
            // Array
            let len: usize = u.int_in_range(0..=5)?;
            let arr: Vec<Value> = (0..len)
                .filter_map(|_| generate_json_value(u, depth - 1).ok())
                .collect();
            Ok(Value::Array(arr))
        }
        6 => {
            // Object
            let len: usize = u.int_in_range(0..=5)?;
            let mut map = serde_json::Map::new();
            for _ in 0..len {
                let key_len: usize = u.int_in_range(1..=15)?;
                let key: String = (0..key_len)
                    .map(|_| u.int_in_range(b'a'..=b'z').unwrap_or(b'x') as char)
                    .collect();
                if let Ok(val) = generate_json_value(u, depth - 1) {
                    map.insert(key, val);
                }
            }
            Ok(Value::Object(map))
        }
        _ => Ok(Value::Null),
    }
}

fn generate_primitive(u: &mut Unstructured) -> arbitrary::Result<Value> {
    let choice: u8 = u.int_in_range(0..=4)?;
    match choice {
        0 => Ok(Value::Null),
        1 => Ok(Value::Bool(u.arbitrary()?)),
        2 => Ok(json!(u.arbitrary::<i64>()?)),
        3 => Ok(json!(u.arbitrary::<f64>()?)),
        4 => {
            let len: usize = u.int_in_range(0..=50)?;
            let s: String = (0..len)
                .map(|_| u.int_in_range(32u8..=126u8).unwrap_or(b'x') as char)
                .collect();
            Ok(Value::String(s))
        }
        _ => Ok(Value::Null),
    }
}

/// Simulates API request validation for SQL query endpoint
fn validate_sql_query_request(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;

    // Check required fields
    let query = obj.get("query").ok_or("Missing 'query' field")?;
    if !query.is_string() {
        return Err("'query' must be a string".to_string());
    }

    // Check optional fields with correct types
    if let Some(params) = obj.get("parameters") {
        if !params.is_object() && !params.is_array() {
            return Err("'parameters' must be object or array".to_string());
        }
    }

    if let Some(timeout) = obj.get("timeout_ms") {
        if !timeout.is_number() {
            return Err("'timeout_ms' must be a number".to_string());
        }
    }

    Ok(())
}

/// Simulates API request validation for table creation
fn validate_create_table_request(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;

    let schema = obj.get("schema").ok_or("Missing 'schema' field")?;
    let schema_obj = schema.as_object().ok_or("'schema' must be object")?;

    let name = schema_obj.get("name").ok_or("Missing 'schema.name'")?;
    if !name.is_string() {
        return Err("'schema.name' must be string".to_string());
    }

    let columns = schema_obj
        .get("columns")
        .ok_or("Missing 'schema.columns'")?;
    if !columns.is_array() {
        return Err("'schema.columns' must be array".to_string());
    }

    // Validate each column
    for (idx, col) in columns.as_array().unwrap().iter().enumerate() {
        let col_obj = col
            .as_object()
            .ok_or_else(|| format!("Column {} must be object", idx))?;
        col_obj
            .get("name")
            .ok_or_else(|| format!("Column {} missing 'name'", idx))?;
        col_obj
            .get("data_type")
            .ok_or_else(|| format!("Column {} missing 'data_type'", idx))?;
    }

    Ok(())
}

/// Simulates API request validation for insert data
fn validate_insert_data_request(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;

    let records = obj.get("records").ok_or("Missing 'records' field")?;
    if !records.is_array() {
        return Err("'records' must be array".to_string());
    }

    if let Some(batch_size) = obj.get("batch_size") {
        if !batch_size.is_number() {
            return Err("'batch_size' must be number".to_string());
        }
    }

    Ok(())
}

/// Simulates API request validation for quantum search
fn validate_quantum_search_request(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;

    let table = obj.get("table_name").ok_or("Missing 'table_name'")?;
    if !table.is_string() {
        return Err("'table_name' must be string".to_string());
    }

    let query = obj.get("search_query").ok_or("Missing 'search_query'")?;
    if !query.is_string() {
        return Err("'search_query' must be string".to_string());
    }

    if let Some(iterations) = obj.get("max_iterations") {
        if !iterations.is_number() {
            return Err("'max_iterations' must be number".to_string());
        }
    }

    Ok(())
}

/// Simulates API request validation for DNA compression
fn validate_dna_compress_request(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;

    let sequences = obj.get("sequences").ok_or("Missing 'sequences'")?;
    if !sequences.is_array() {
        return Err("'sequences' must be array".to_string());
    }

    for (idx, seq) in sequences.as_array().unwrap().iter().enumerate() {
        if !seq.is_string() && !seq.is_object() {
            return Err(format!("Sequence {} must be string or object", idx));
        }
    }

    Ok(())
}

/// Simulates API request validation for neural network training
fn validate_train_nn_request(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;

    obj.get("network_name")
        .ok_or("Missing 'network_name'")?
        .as_str()
        .ok_or("'network_name' must be string")?;

    if let Some(config) = obj.get("config") {
        config
            .as_object()
            .ok_or("'config' must be object".to_string())?;
    }

    if let Some(data) = obj.get("training_data") {
        data.as_array()
            .ok_or("'training_data' must be array".to_string())?;
    }

    Ok(())
}

/// Simulates biometric auth request validation
fn validate_eeg_auth_request(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;

    obj.get("user_id")
        .ok_or("Missing 'user_id'")?
        .as_str()
        .ok_or("'user_id' must be string")?;

    let eeg_data = obj.get("eeg_data").ok_or("Missing 'eeg_data'")?;
    eeg_data
        .as_array()
        .ok_or("'eeg_data' must be array".to_string())?;

    Ok(())
}

fuzz_target!(|data: &[u8]| {
    // Skip empty inputs
    if data.is_empty() {
        return;
    }

    // Try to parse as JSON
    if let Ok(json) = serde_json::from_slice::<Value>(data) {
        // Test all API endpoint validators - they should never panic
        let _ = validate_sql_query_request(&json);
        let _ = validate_create_table_request(&json);
        let _ = validate_insert_data_request(&json);
        let _ = validate_quantum_search_request(&json);
        let _ = validate_dna_compress_request(&json);
        let _ = validate_train_nn_request(&json);
        let _ = validate_eeg_auth_request(&json);
    }

    // Test structured fuzzing with arbitrary JSON
    if data.len() >= 4 {
        let mut u = Unstructured::new(data);
        if let Ok(arbitrary_json) = ArbitraryJsonValue::arbitrary(&mut u) {
            let _ = validate_sql_query_request(&arbitrary_json.value);
            let _ = validate_create_table_request(&arbitrary_json.value);
            let _ = validate_insert_data_request(&arbitrary_json.value);
            let _ = validate_quantum_search_request(&arbitrary_json.value);
            let _ = validate_dna_compress_request(&arbitrary_json.value);
            let _ = validate_train_nn_request(&arbitrary_json.value);
            let _ = validate_eeg_auth_request(&arbitrary_json.value);
        }
    }

    // Test with realistic API payloads generated from fuzz data
    if data.len() >= 8 {
        let table_name = format!("table_{}", data[0] % 26);
        let field_name = format!("field_{}", data[1] % 10);

        // SQL Query payloads
        let sql_payloads = vec![
            json!({
                "query": String::from_utf8_lossy(&data[2..]).to_string(),
                "parameters": {}
            }),
            json!({
                "query": format!("SELECT * FROM {}", table_name),
                "timeout_ms": (data[2] as u32) * 100
            }),
            json!({
                "query": format!("SELECT {} FROM {} WHERE id = $1", field_name, table_name),
                "parameters": [data[3] as i32]
            }),
        ];

        for payload in sql_payloads {
            let _ = validate_sql_query_request(&payload);
        }

        // Create table payloads
        let create_payloads = vec![
            json!({
                "schema": {
                    "name": table_name,
                    "columns": [
                        {"name": "id", "data_type": "Integer"},
                        {"name": field_name, "data_type": "Text"}
                    ]
                }
            }),
            json!({
                "schema": {
                    "name": String::from_utf8_lossy(&data[2..10.min(data.len())]).to_string(),
                    "columns": []
                }
            }),
        ];

        for payload in create_payloads {
            let _ = validate_create_table_request(&payload);
        }

        // Quantum search payloads
        let quantum_payloads = vec![
            json!({
                "table_name": table_name,
                "search_query": String::from_utf8_lossy(&data[4..]).to_string(),
                "max_iterations": data[3] as u32,
                "amplitude_amplification": data[4] % 2 == 0
            }),
            json!({
                "table_name": table_name,
                "search_query": format!("{} LIKE '%pattern%'", field_name),
                "grover_enabled": true
            }),
        ];

        for payload in quantum_payloads {
            let _ = validate_quantum_search_request(&payload);
        }

        // DNA compression payloads
        let dna_payloads = vec![
            json!({
                "sequences": [
                    String::from_utf8_lossy(data).to_string()
                ],
                "compression_level": data[2] % 10
            }),
            json!({
                "sequences": [
                    {"data": data.iter().map(|b| *b as i32).collect::<Vec<_>>()}
                ],
                "enable_error_correction": true
            }),
        ];

        for payload in dna_payloads {
            let _ = validate_dna_compress_request(&payload);
        }
    }

    // Test malformed JSON edge cases
    let malformed_cases = [
        b"{}".as_slice(),
        b"[]",
        b"null",
        b"true",
        b"false",
        b"0",
        b"\"\"",
        b"{\"query\": null}",
        b"{\"query\": []}",
        b"{\"query\": {}}",
        b"{\"schema\": {\"name\": \"\", \"columns\": null}}",
        b"{\"records\": null}",
        b"{\"sequences\": \"not_an_array\"}",
    ];

    for case in malformed_cases {
        if let Ok(json) = serde_json::from_slice::<Value>(case) {
            let _ = validate_sql_query_request(&json);
            let _ = validate_create_table_request(&json);
            let _ = validate_insert_data_request(&json);
        }
    }
});
