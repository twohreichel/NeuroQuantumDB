//! Tests for metrics recording and rendering
//!
//! These tests validate Prometheus-style metric recording for various
//! API operations.

use neuroquantum_api::metrics::{
    get_uptime_seconds, record_api_request, record_auth_request, record_db_operation,
    record_dna_compression, record_neural_training, record_quantum_search, record_query,
    record_websocket_connection, record_websocket_message, render_metrics, update_system_metrics,
};

#[test]
fn test_record_query() {
    record_query("neuromorphic", 0.05);
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_queries_total"));
}

#[test]
fn test_record_auth_request() {
    record_auth_request("success");
    record_auth_request("failed");
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_auth_requests_total"));
}

#[test]
fn test_record_api_request() {
    record_api_request("/api/v1/query", "POST", "200", 0.1);
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_api_requests_total"));
    assert!(metrics.contains("neuroquantum_api_request_duration_seconds"));
}

#[test]
fn test_websocket_metrics() {
    record_websocket_connection("connected");
    record_websocket_message("sent", "query");
    record_websocket_connection("disconnected");
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_websocket_connections_total"));
    assert!(metrics.contains("neuroquantum_websocket_messages_total"));
}

#[test]
fn test_db_operations() {
    record_db_operation("select", "success", 0.001);
    record_db_operation("insert", "success", 0.005);
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_db_operations_total"));
}

#[test]
fn test_dna_compression() {
    record_dna_compression("success", 3.5);
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_dna_compression_total"));
    assert!(metrics.contains("neuroquantum_dna_compression_ratio"));
}

#[test]
fn test_quantum_search() {
    record_quantum_search("success");
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_quantum_search_total"));
}

#[test]
fn test_neural_training() {
    record_neural_training("success", 60.0);
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_neural_training_total"));
}

#[test]
fn test_system_metrics() {
    update_system_metrics();
    let metrics = render_metrics().unwrap();
    assert!(metrics.contains("neuroquantum_memory_usage_bytes"));
}

#[test]
fn test_uptime() {
    std::thread::sleep(std::time::Duration::from_millis(100));
    let uptime = get_uptime_seconds();
    assert!(uptime >= 0.0);
}
