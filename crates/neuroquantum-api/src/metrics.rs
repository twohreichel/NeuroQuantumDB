// filepath: /Users/andreasreichel/workspace/NeuroQuantumDB/crates/neuroquantum-api/src/metrics.rs
//! Prometheus metrics collection for NeuroQuantumDB API
//!
//! This module provides comprehensive metrics for monitoring API performance,
//! database operations, and system health.

use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_gauge, register_gauge_vec, register_histogram_vec, CounterVec,
    Encoder, Gauge, GaugeVec, HistogramVec, TextEncoder,
};
use std::time::SystemTime;

// ===== Counters =====

/// Total number of queries processed by type
pub static QUERIES_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_queries_total",
        "Total number of queries processed",
        &["type"]
    )
    .expect("Failed to register queries_total metric")
});

/// Total authentication requests by status
pub static AUTH_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_auth_requests_total",
        "Total authentication requests",
        &["status"]
    )
    .expect("Failed to register auth_requests_total metric")
});

/// Total API requests by endpoint and method
pub static API_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_api_requests_total",
        "Total API requests",
        &["endpoint", "method", "status"]
    )
    .expect("Failed to register api_requests_total metric")
});

/// Total WebSocket connections
pub static WEBSOCKET_CONNECTIONS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_websocket_connections_total",
        "Total WebSocket connections",
        &["status"]
    )
    .expect("Failed to register websocket_connections_total metric")
});

/// Total messages sent/received via WebSocket
pub static WEBSOCKET_MESSAGES_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_websocket_messages_total",
        "Total WebSocket messages",
        &["direction", "type"]
    )
    .expect("Failed to register websocket_messages_total metric")
});

/// Total database operations
pub static DB_OPERATIONS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_db_operations_total",
        "Total database operations",
        &["operation", "status"]
    )
    .expect("Failed to register db_operations_total metric")
});

/// Total DNA compression operations
pub static DNA_COMPRESSION_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_dna_compression_total",
        "Total DNA compression operations",
        &["status"]
    )
    .expect("Failed to register dna_compression_total metric")
});

/// Total quantum search operations
pub static QUANTUM_SEARCH_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_quantum_search_total",
        "Total quantum search operations",
        &["status"]
    )
    .expect("Failed to register quantum_search_total metric")
});

/// Total neural network training operations
pub static NEURAL_TRAINING_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "neuroquantum_neural_training_total",
        "Total neural network training operations",
        &["status"]
    )
    .expect("Failed to register neural_training_total metric")
});

// ===== Gauges =====

/// Current active WebSocket connections
pub static ACTIVE_CONNECTIONS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "neuroquantum_active_connections",
        "Current active WebSocket connections"
    )
    .expect("Failed to register active_connections metric")
});

/// Current active database transactions
pub static ACTIVE_TRANSACTIONS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "neuroquantum_active_transactions",
        "Current active database transactions"
    )
    .expect("Failed to register active_transactions metric")
});

/// System memory usage in bytes
pub static MEMORY_USAGE_BYTES: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "neuroquantum_memory_usage_bytes",
        "System memory usage in bytes"
    )
    .expect("Failed to register memory_usage_bytes metric")
});

/// Database size in bytes
pub static DATABASE_SIZE_BYTES: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!("neuroquantum_database_size_bytes", "Database size in bytes")
        .expect("Failed to register database_size_bytes metric")
});

/// Buffer pool hit rate (0.0 - 1.0)
pub static BUFFER_POOL_HIT_RATE: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "neuroquantum_buffer_pool_hit_rate",
        "Buffer pool hit rate (0.0 - 1.0)"
    )
    .expect("Failed to register buffer_pool_hit_rate metric")
});

/// Current neural networks in training
pub static NEURAL_NETWORKS_TRAINING: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "neuroquantum_neural_networks_training",
        "Current neural networks in training"
    )
    .expect("Failed to register neural_networks_training metric")
});

/// System temperature in Celsius (for Raspberry Pi monitoring)
pub static SYSTEM_TEMPERATURE_CELSIUS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "neuroquantum_system_temperature_celsius",
        "System temperature in Celsius"
    )
    .expect("Failed to register system_temperature_celsius metric")
});

/// API Key usage by key name
pub static API_KEY_USAGE: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "neuroquantum_api_key_usage",
        "API key usage statistics",
        &["key_name", "permission"]
    )
    .expect("Failed to register api_key_usage metric")
});

// ===== Histograms =====

/// Response time for queries in seconds
pub static QUERY_RESPONSE_TIME_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "neuroquantum_query_response_time_seconds",
        "Query response time in seconds",
        &["type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Failed to register query_response_time_seconds metric")
});

/// API request duration in seconds
pub static API_REQUEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "neuroquantum_api_request_duration_seconds",
        "API request duration in seconds",
        &["endpoint", "method"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("Failed to register api_request_duration_seconds metric")
});

/// Database operation duration in seconds
pub static DB_OPERATION_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "neuroquantum_db_operation_duration_seconds",
        "Database operation duration in seconds",
        &["operation"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    )
    .expect("Failed to register db_operation_duration_seconds metric")
});

/// DNA compression ratio
pub static DNA_COMPRESSION_RATIO: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "neuroquantum_dna_compression_ratio",
        "DNA compression ratio",
        &["status"],
        vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0]
    )
    .expect("Failed to register dna_compression_ratio metric")
});

/// Neural network training duration in seconds
pub static NEURAL_TRAINING_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "neuroquantum_neural_training_duration_seconds",
        "Neural network training duration in seconds",
        &["status"],
        vec![1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0]
    )
    .expect("Failed to register neural_training_duration_seconds metric")
});

// ===== Utility Functions =====

/// Record a query execution
pub fn record_query(query_type: &str, duration_secs: f64) {
    QUERIES_TOTAL.with_label_values(&[query_type]).inc();
    QUERY_RESPONSE_TIME_SECONDS
        .with_label_values(&[query_type])
        .observe(duration_secs);
}

/// Record an authentication request
pub fn record_auth_request(status: &str) {
    AUTH_REQUESTS_TOTAL.with_label_values(&[status]).inc();
}

/// Record an API request
pub fn record_api_request(endpoint: &str, method: &str, status: &str, duration_secs: f64) {
    API_REQUESTS_TOTAL
        .with_label_values(&[endpoint, method, status])
        .inc();
    API_REQUEST_DURATION_SECONDS
        .with_label_values(&[endpoint, method])
        .observe(duration_secs);
}

/// Record a WebSocket connection event
pub fn record_websocket_connection(status: &str) {
    WEBSOCKET_CONNECTIONS_TOTAL
        .with_label_values(&[status])
        .inc();

    if status == "connected" {
        ACTIVE_CONNECTIONS.inc();
    } else if status == "disconnected" {
        ACTIVE_CONNECTIONS.dec();
    }
}

/// Record a WebSocket message
pub fn record_websocket_message(direction: &str, message_type: &str) {
    WEBSOCKET_MESSAGES_TOTAL
        .with_label_values(&[direction, message_type])
        .inc();
}

/// Record a database operation
pub fn record_db_operation(operation: &str, status: &str, duration_secs: f64) {
    DB_OPERATIONS_TOTAL
        .with_label_values(&[operation, status])
        .inc();
    DB_OPERATION_DURATION_SECONDS
        .with_label_values(&[operation])
        .observe(duration_secs);
}

/// Record a DNA compression operation
pub fn record_dna_compression(status: &str, compression_ratio: f64) {
    DNA_COMPRESSION_TOTAL.with_label_values(&[status]).inc();
    DNA_COMPRESSION_RATIO
        .with_label_values(&[status])
        .observe(compression_ratio);
}

/// Record a quantum search operation
pub fn record_quantum_search(status: &str) {
    QUANTUM_SEARCH_TOTAL.with_label_values(&[status]).inc();
}

/// Record a neural network training operation
pub fn record_neural_training(status: &str, duration_secs: f64) {
    NEURAL_TRAINING_TOTAL.with_label_values(&[status]).inc();
    NEURAL_TRAINING_DURATION_SECONDS
        .with_label_values(&[status])
        .observe(duration_secs);
}

/// Update system metrics (memory, temperature, etc.)
pub fn update_system_metrics() {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();

    // Update memory usage
    let used_memory = sys.used_memory();
    MEMORY_USAGE_BYTES.set(used_memory as f64);

    // Update system temperature (if available)
    // Note: On Raspberry Pi, this reads from /sys/class/thermal/thermal_zone0/temp
    #[cfg(target_os = "linux")]
    {
        if let Ok(temp_str) = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
            if let Ok(temp_millidegrees) = temp_str.trim().parse::<f64>() {
                let temp_celsius = temp_millidegrees / 1000.0;
                SYSTEM_TEMPERATURE_CELSIUS.set(temp_celsius);
            }
        }
    }
}

/// Get the server uptime in seconds
static START_TIME: Lazy<SystemTime> = Lazy::new(SystemTime::now);

pub fn get_uptime_seconds() -> f64 {
    START_TIME.elapsed().map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Get system temperature in Celsius (Linux-specific, returns None on other platforms)
pub fn get_system_temperature() -> Option<f32> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|t| t / 1000.0)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Get average query response time in milliseconds from histogram metrics
pub fn get_average_query_time_ms() -> f32 {
    let sum = QUERY_RESPONSE_TIME_SECONDS
        .with_label_values(&["crud"])
        .get_sample_sum();
    let count = QUERY_RESPONSE_TIME_SECONDS
        .with_label_values(&["crud"])
        .get_sample_count();
    
    if count > 0 {
        (sum / count as f64 * 1000.0) as f32
    } else {
        1.0 // Default 1ms if no queries recorded
    }
}

/// Render all metrics in Prometheus text format
pub fn render_metrics() -> Result<String, Box<dyn std::error::Error>> {
    // Update system metrics before rendering
    update_system_metrics();

    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;

    Ok(String::from_utf8(buffer)?)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
