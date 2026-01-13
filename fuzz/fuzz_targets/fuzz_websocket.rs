//! Fuzz target for WebSocket Messages
//!
//! This fuzz target tests WebSocket message parsing and handling to find
//! parsing bugs, protocol violations, and edge cases in message processing.

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use serde_json::{json, Value};

/// WebSocket message types (matching the API structure)
#[derive(Debug, Clone)]
enum WsMessageType {
    Subscribe,
    Unsubscribe,
    Publish,
    StreamQuery,
    CancelQuery,
    Ping,
    Pong,
    QueryStatus,
    Message,
}

impl<'a> Arbitrary<'a> for WsMessageType {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let choice: u8 = u.int_in_range(0..=8)?;
        Ok(match choice {
            0 => WsMessageType::Subscribe,
            1 => WsMessageType::Unsubscribe,
            2 => WsMessageType::Publish,
            3 => WsMessageType::StreamQuery,
            4 => WsMessageType::CancelQuery,
            5 => WsMessageType::Ping,
            6 => WsMessageType::Pong,
            7 => WsMessageType::QueryStatus,
            _ => WsMessageType::Message,
        })
    }
}

/// Generate a random string for fuzzing
fn generate_string(u: &mut Unstructured, max_len: usize) -> arbitrary::Result<String> {
    let len: usize = u.int_in_range(0..=max_len)?;
    let s: String = (0..len)
        .map(|_| u.int_in_range(32u8..=126u8).unwrap_or(b'x') as char)
        .collect();
    Ok(s)
}

/// Generate a structured WebSocket message for fuzzing
fn generate_ws_message(u: &mut Unstructured) -> arbitrary::Result<Value> {
    let msg_type = WsMessageType::arbitrary(u)?;

    match msg_type {
        WsMessageType::Subscribe => Ok(json!({
            "type": "subscribe",
            "channel": generate_string(u, 50)?
        })),
        WsMessageType::Unsubscribe => Ok(json!({
            "type": "unsubscribe",
            "channel": generate_string(u, 50)?
        })),
        WsMessageType::Publish => Ok(json!({
            "type": "publish",
            "channel": generate_string(u, 50)?,
            "data": generate_arbitrary_json(u, 2)?
        })),
        WsMessageType::StreamQuery => Ok(json!({
            "type": "stream_query",
            "query": generate_string(u, 500)?,
            "batch_size": u.int_in_range(1u32..=10000)?
        })),
        WsMessageType::CancelQuery => Ok(json!({
            "type": "cancel_query",
            "stream_id": generate_string(u, 36)?
        })),
        WsMessageType::Ping => {
            let has_timestamp: bool = u.arbitrary()?;
            if has_timestamp {
                Ok(json!({
                    "type": "ping",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            } else {
                Ok(json!({ "type": "ping" }))
            }
        }
        WsMessageType::Pong => Ok(json!({
            "type": "pong",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
        WsMessageType::QueryStatus => Ok(json!({
            "type": "query_status",
            "query_id": generate_string(u, 36)?
        })),
        WsMessageType::Message => Ok(json!({
            "type": "message",
            "data": generate_arbitrary_json(u, 2)?
        })),
    }
}

/// Generate arbitrary JSON for fuzzing
fn generate_arbitrary_json(u: &mut Unstructured, depth: u8) -> arbitrary::Result<Value> {
    if depth == 0 {
        let choice: u8 = u.int_in_range(0..=4)?;
        return match choice {
            0 => Ok(Value::Null),
            1 => Ok(Value::Bool(u.arbitrary()?)),
            2 => Ok(json!(u.arbitrary::<i32>()?)),
            3 => Ok(json!(u.arbitrary::<f64>()?)),
            _ => Ok(Value::String(generate_string(u, 100)?)),
        };
    }

    let choice: u8 = u.int_in_range(0..=6)?;
    match choice {
        0..=4 => generate_arbitrary_json(u, 0), // Primitives
        5 => {
            // Array
            let len: usize = u.int_in_range(0..=5)?;
            let arr: Vec<Value> = (0..len)
                .filter_map(|_| generate_arbitrary_json(u, depth - 1).ok())
                .collect();
            Ok(Value::Array(arr))
        }
        _ => {
            // Object
            let len: usize = u.int_in_range(0..=5)?;
            let mut map = serde_json::Map::new();
            for _ in 0..len {
                let key = generate_string(u, 20)?;
                if let Ok(val) = generate_arbitrary_json(u, depth - 1) {
                    map.insert(key, val);
                }
            }
            Ok(Value::Object(map))
        }
    }
}

/// Validate WebSocket message structure
fn validate_ws_message(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;
    let msg_type = obj
        .get("type")
        .ok_or("Missing 'type' field")?
        .as_str()
        .ok_or("'type' must be string")?;

    match msg_type {
        "subscribe" | "unsubscribe" => {
            obj.get("channel")
                .ok_or("Missing 'channel'")?
                .as_str()
                .ok_or("'channel' must be string")?;
        }
        "publish" => {
            obj.get("channel")
                .ok_or("Missing 'channel'")?
                .as_str()
                .ok_or("'channel' must be string")?;
            obj.get("data").ok_or("Missing 'data'")?;
        }
        "stream_query" => {
            obj.get("query")
                .ok_or("Missing 'query'")?
                .as_str()
                .ok_or("'query' must be string")?;
        }
        "cancel_query" => {
            obj.get("stream_id")
                .ok_or("Missing 'stream_id'")?
                .as_str()
                .ok_or("'stream_id' must be string")?;
        }
        "ping" => {
            // timestamp is optional
        }
        "pong" => {
            obj.get("timestamp")
                .ok_or("Missing 'timestamp'")?
                .as_str()
                .ok_or("'timestamp' must be string")?;
        }
        "query_status" => {
            obj.get("query_id")
                .ok_or("Missing 'query_id'")?
                .as_str()
                .ok_or("'query_id' must be string")?;
        }
        "message" => {
            obj.get("data").ok_or("Missing 'data'")?;
        }
        _ => {
            return Err(format!("Unknown message type: {}", msg_type));
        }
    }

    Ok(())
}

/// Validate WebSocket response structure
fn validate_ws_response(json: &Value) -> Result<(), String> {
    let obj = json.as_object().ok_or("Expected object")?;
    let msg_type = obj
        .get("type")
        .ok_or("Missing 'type' field")?
        .as_str()
        .ok_or("'type' must be string")?;

    match msg_type {
        "subscription_confirmed" | "unsubscription_confirmed" => {
            obj.get("channel")
                .ok_or("Missing 'channel'")?
                .as_str()
                .ok_or("'channel' must be string")?;
            obj.get("timestamp")
                .ok_or("Missing 'timestamp'")?
                .as_str()
                .ok_or("'timestamp' must be string")?;
        }
        "channel_message" => {
            obj.get("channel")
                .ok_or("Missing 'channel'")?
                .as_str()
                .ok_or("'channel' must be string")?;
            obj.get("data").ok_or("Missing 'data'")?;
            obj.get("timestamp")
                .ok_or("Missing 'timestamp'")?
                .as_str()
                .ok_or("'timestamp' must be string")?;
        }
        "query_started" => {
            obj.get("stream_id")
                .ok_or("Missing 'stream_id'")?
                .as_str()
                .ok_or("'stream_id' must be string")?;
            obj.get("query")
                .ok_or("Missing 'query'")?
                .as_str()
                .ok_or("'query' must be string")?;
        }
        "query_progress" => {
            obj.get("stream_id")
                .ok_or("Missing 'stream_id'")?
                .as_str()
                .ok_or("'stream_id' must be string")?;
            obj.get("rows_processed").ok_or("Missing 'rows_processed'")?;
        }
        "query_batch" => {
            obj.get("stream_id")
                .ok_or("Missing 'stream_id'")?
                .as_str()
                .ok_or("'stream_id' must be string")?;
            obj.get("batch_number").ok_or("Missing 'batch_number'")?;
            obj.get("rows")
                .ok_or("Missing 'rows'")?
                .as_array()
                .ok_or("'rows' must be array")?;
        }
        "query_completed" => {
            obj.get("stream_id")
                .ok_or("Missing 'stream_id'")?
                .as_str()
                .ok_or("'stream_id' must be string")?;
            obj.get("total_rows").ok_or("Missing 'total_rows'")?;
        }
        "query_cancelled" => {
            obj.get("stream_id")
                .ok_or("Missing 'stream_id'")?
                .as_str()
                .ok_or("'stream_id' must be string")?;
            obj.get("reason")
                .ok_or("Missing 'reason'")?
                .as_str()
                .ok_or("'reason' must be string")?;
        }
        "pong" => {
            obj.get("timestamp")
                .ok_or("Missing 'timestamp'")?
                .as_str()
                .ok_or("'timestamp' must be string")?;
        }
        "error" => {
            obj.get("code")
                .ok_or("Missing 'code'")?
                .as_str()
                .ok_or("'code' must be string")?;
            obj.get("message")
                .ok_or("Missing 'message'")?
                .as_str()
                .ok_or("'message' must be string")?;
        }
        "query_status" => {
            obj.get("query_id")
                .ok_or("Missing 'query_id'")?
                .as_str()
                .ok_or("'query_id' must be string")?;
            obj.get("status")
                .ok_or("Missing 'status'")?
                .as_str()
                .ok_or("'status' must be string")?;
        }
        _ => {
            return Err(format!("Unknown response type: {}", msg_type));
        }
    }

    Ok(())
}

fuzz_target!(|data: &[u8]| {
    // Skip empty inputs
    if data.is_empty() {
        return;
    }

    // Test raw bytes as JSON
    if let Ok(json) = serde_json::from_slice::<Value>(data) {
        // Validate as WebSocket message - should never panic
        let _ = validate_ws_message(&json);
        let _ = validate_ws_response(&json);
    }

    // Test structured WebSocket message generation
    if data.len() >= 4 {
        let mut u = Unstructured::new(data);
        if let Ok(msg) = generate_ws_message(&mut u) {
            // Generated messages should be valid
            let _ = validate_ws_message(&msg);

            // Test serialization roundtrip
            if let Ok(serialized) = serde_json::to_string(&msg) {
                if let Ok(deserialized) = serde_json::from_str::<Value>(&serialized) {
                    let _ = validate_ws_message(&deserialized);
                }
            }
        }
    }

    // Test specific WebSocket message patterns
    let test_messages = vec![
        // Valid messages
        json!({"type": "subscribe", "channel": "test"}),
        json!({"type": "unsubscribe", "channel": "updates"}),
        json!({"type": "publish", "channel": "data", "data": {"key": "value"}}),
        json!({"type": "stream_query", "query": "SELECT * FROM users"}),
        json!({"type": "stream_query", "query": "NEUROMATCH test PATTERN 'pattern'", "batch_size": 100}),
        json!({"type": "cancel_query", "stream_id": "abc-123"}),
        json!({"type": "ping"}),
        json!({"type": "ping", "timestamp": "2025-01-13T12:00:00Z"}),
        json!({"type": "pong", "timestamp": "2025-01-13T12:00:00Z"}),
        json!({"type": "query_status", "query_id": "q-123"}),
        json!({"type": "message", "data": [1, 2, 3]}),
        // Invalid messages (should fail validation but not panic)
        json!({"type": "subscribe"}), // Missing channel
        json!({"type": "publish", "channel": "test"}), // Missing data
        json!({"type": "stream_query"}), // Missing query
        json!({"type": "cancel_query"}), // Missing stream_id
        json!({"type": "pong"}), // Missing timestamp
        json!({"type": "unknown_type"}), // Unknown type
        json!({}), // Empty object
        json!({"type": 123}), // Wrong type for type field
        json!({"type": "subscribe", "channel": 123}), // Wrong type for channel
    ];

    for msg in test_messages {
        let _ = validate_ws_message(&msg);
    }

    // Test with fuzz data injected into messages
    if let Ok(input) = std::str::from_utf8(data) {
        let fuzz_messages = vec![
            json!({"type": "subscribe", "channel": input}),
            json!({"type": "stream_query", "query": input}),
            json!({"type": "publish", "channel": "test", "data": input}),
            json!({"type": "message", "data": {"content": input}}),
        ];

        for msg in fuzz_messages {
            let _ = validate_ws_message(&msg);
        }
    }

    // Test WebSocket response patterns
    let test_responses = vec![
        json!({"type": "subscription_confirmed", "channel": "test", "timestamp": "2025-01-13T12:00:00Z"}),
        json!({"type": "channel_message", "channel": "data", "data": {}, "timestamp": "2025-01-13T12:00:00Z"}),
        json!({"type": "query_started", "stream_id": "s-123", "query": "SELECT *"}),
        json!({"type": "query_progress", "stream_id": "s-123", "rows_processed": 1000}),
        json!({"type": "query_batch", "stream_id": "s-123", "batch_number": 1, "rows": [], "has_more": true}),
        json!({"type": "query_completed", "stream_id": "s-123", "total_rows": 5000, "execution_time_ms": 150}),
        json!({"type": "query_cancelled", "stream_id": "s-123", "reason": "User requested"}),
        json!({"type": "pong", "timestamp": "2025-01-13T12:00:00Z"}),
        json!({"type": "error", "code": "E001", "message": "Something went wrong"}),
        json!({"type": "query_status", "query_id": "q-123", "status": "running", "progress": 50}),
    ];

    for resp in test_responses {
        let _ = validate_ws_response(&resp);
    }
});
