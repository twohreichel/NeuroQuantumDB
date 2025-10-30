//! Prometheus Metrics Exporter for NeuroQuantumDB
//!
//! Provides Prometheus-compatible metrics endpoint for monitoring

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{
    core::{AtomicU64, GenericGauge},
    Encoder, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::sync::Arc;

/// Prometheus metrics registry and collectors
#[derive(Clone)]
pub struct MetricsExporter {
    registry: Arc<Registry>,

    // Query metrics
    pub queries_total: IntCounterVec,
    pub query_duration: HistogramVec,
    pub slow_queries_total: IntCounter,
    pub query_errors_total: IntCounterVec,
    pub active_queries: IntGauge,
    pub query_queue_length: IntGauge,

    // Connection metrics
    pub active_connections: IntGauge,
    pub max_connections: IntGauge,
    pub connections_total: IntCounter,
    pub connection_errors_total: IntCounter,

    // WebSocket metrics
    pub websocket_connections_active: IntGauge,
    pub websocket_connections_total: IntCounter,
    pub websocket_messages_sent_total: IntCounter,
    pub websocket_messages_received_total: IntCounter,
    pub websocket_messages_dropped_total: IntCounter,
    pub websocket_channels_total: IntGauge,
    pub websocket_active_streams: IntGauge,
    pub websocket_message_latency: Histogram,

    // Buffer pool metrics
    pub buffer_pool_hit_ratio: GenericGauge<AtomicU64>,
    pub buffer_pool_pages: IntGaugeVec,
    pub buffer_pool_evictions_total: IntCounter,

    // Disk I/O metrics
    pub disk_read_bytes_total: IntCounter,
    pub disk_write_bytes_total: IntCounter,
    pub disk_read_ops_total: IntCounter,
    pub disk_write_ops_total: IntCounter,

    // WAL metrics
    pub wal_bytes_written: IntCounter,
    pub wal_segments_total: IntGauge,
    pub wal_fsync_duration: Histogram,

    // Lock metrics
    pub lock_wait_seconds_total: HistogramVec,
    pub lock_contention_by_resource: IntCounterVec,

    // Index metrics
    pub index_scans_total: IntCounterVec,
    pub index_rows_read_total: IntCounterVec,
    pub index_hit_ratio: GenericGauge<AtomicU64>,

    // Memory metrics
    pub memory_usage_bytes: IntGauge,
    pub memory_limit_bytes: IntGauge,

    // System metrics
    pub up: IntGauge,
    pub errors_total: IntCounterVec,
}

impl MetricsExporter {
    /// Create a new metrics exporter
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // Query metrics
        let queries_total = IntCounterVec::new(
            Opts::new(
                "neuroquantum_queries_total",
                "Total number of queries executed",
            ),
            &["query_type", "status"],
        )?;
        registry.register(Box::new(queries_total.clone()))?;

        let query_duration = HistogramVec::new(
            HistogramOpts::new(
                "neuroquantum_query_duration_seconds",
                "Query execution duration",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
            &["query_type"],
        )?;
        registry.register(Box::new(query_duration.clone()))?;

        let slow_queries_total = IntCounter::new(
            "neuroquantum_slow_queries_total",
            "Total number of slow queries",
        )?;
        registry.register(Box::new(slow_queries_total.clone()))?;

        let query_errors_total = IntCounterVec::new(
            Opts::new("neuroquantum_query_errors_total", "Total query errors"),
            &["error_type"],
        )?;
        registry.register(Box::new(query_errors_total.clone()))?;

        let active_queries = IntGauge::new(
            "neuroquantum_active_queries",
            "Number of currently executing queries",
        )?;
        registry.register(Box::new(active_queries.clone()))?;

        let query_queue_length = IntGauge::new(
            "neuroquantum_query_queue_length",
            "Number of queries waiting in queue",
        )?;
        registry.register(Box::new(query_queue_length.clone()))?;

        // Connection metrics
        let active_connections = IntGauge::new(
            "neuroquantum_active_connections",
            "Number of active connections",
        )?;
        registry.register(Box::new(active_connections.clone()))?;

        let max_connections = IntGauge::new(
            "neuroquantum_max_connections",
            "Maximum allowed connections",
        )?;
        registry.register(Box::new(max_connections.clone()))?;

        let connections_total = IntCounter::new(
            "neuroquantum_connections_total",
            "Total connections established",
        )?;
        registry.register(Box::new(connections_total.clone()))?;

        let connection_errors_total = IntCounter::new(
            "neuroquantum_connection_errors_total",
            "Total connection errors",
        )?;
        registry.register(Box::new(connection_errors_total.clone()))?;

        // WebSocket metrics
        let websocket_connections_active = IntGauge::new(
            "neuroquantum_websocket_connections_active",
            "Active WebSocket connections",
        )?;
        registry.register(Box::new(websocket_connections_active.clone()))?;

        let websocket_connections_total = IntCounter::new(
            "neuroquantum_websocket_connections_total",
            "Total WebSocket connections",
        )?;
        registry.register(Box::new(websocket_connections_total.clone()))?;

        let websocket_messages_sent_total = IntCounter::new(
            "neuroquantum_websocket_messages_sent_total",
            "Total WebSocket messages sent",
        )?;
        registry.register(Box::new(websocket_messages_sent_total.clone()))?;

        let websocket_messages_received_total = IntCounter::new(
            "neuroquantum_websocket_messages_received_total",
            "Total WebSocket messages received",
        )?;
        registry.register(Box::new(websocket_messages_received_total.clone()))?;

        let websocket_messages_dropped_total = IntCounter::new(
            "neuroquantum_websocket_messages_dropped_total",
            "Total WebSocket messages dropped (backpressure)",
        )?;
        registry.register(Box::new(websocket_messages_dropped_total.clone()))?;

        let websocket_channels_total = IntGauge::new(
            "neuroquantum_websocket_channels_total",
            "Number of active pub/sub channels",
        )?;
        registry.register(Box::new(websocket_channels_total.clone()))?;

        let websocket_active_streams = IntGauge::new(
            "neuroquantum_websocket_active_streams",
            "Number of active query streams",
        )?;
        registry.register(Box::new(websocket_active_streams.clone()))?;

        let websocket_message_latency = Histogram::with_opts(
            HistogramOpts::new(
                "neuroquantum_websocket_message_latency_seconds",
                "WebSocket message latency",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]),
        )?;
        registry.register(Box::new(websocket_message_latency.clone()))?;

        // Buffer pool metrics
        let buffer_pool_hit_ratio = GenericGauge::new(
            "neuroquantum_buffer_pool_hit_ratio",
            "Buffer pool cache hit ratio",
        )?;
        registry.register(Box::new(buffer_pool_hit_ratio.clone()))?;

        let buffer_pool_pages = IntGaugeVec::new(
            Opts::new(
                "neuroquantum_buffer_pool_pages",
                "Buffer pool pages by state",
            ),
            &["state"],
        )?;
        registry.register(Box::new(buffer_pool_pages.clone()))?;

        let buffer_pool_evictions_total = IntCounter::new(
            "neuroquantum_buffer_pool_evictions_total",
            "Total buffer pool page evictions",
        )?;
        registry.register(Box::new(buffer_pool_evictions_total.clone()))?;

        // Disk I/O metrics
        let disk_read_bytes_total = IntCounter::new(
            "neuroquantum_disk_read_bytes_total",
            "Total bytes read from disk",
        )?;
        registry.register(Box::new(disk_read_bytes_total.clone()))?;

        let disk_write_bytes_total = IntCounter::new(
            "neuroquantum_disk_write_bytes_total",
            "Total bytes written to disk",
        )?;
        registry.register(Box::new(disk_write_bytes_total.clone()))?;

        let disk_read_ops_total = IntCounter::new(
            "neuroquantum_disk_read_ops_total",
            "Total disk read operations",
        )?;
        registry.register(Box::new(disk_read_ops_total.clone()))?;

        let disk_write_ops_total = IntCounter::new(
            "neuroquantum_disk_write_ops_total",
            "Total disk write operations",
        )?;
        registry.register(Box::new(disk_write_ops_total.clone()))?;

        // WAL metrics
        let wal_bytes_written = IntCounter::new(
            "neuroquantum_wal_bytes_written",
            "Total bytes written to WAL",
        )?;
        registry.register(Box::new(wal_bytes_written.clone()))?;

        let wal_segments_total =
            IntGauge::new("neuroquantum_wal_segments_total", "Number of WAL segments")?;
        registry.register(Box::new(wal_segments_total.clone()))?;

        let wal_fsync_duration = Histogram::with_opts(
            HistogramOpts::new(
                "neuroquantum_wal_fsync_duration_seconds",
                "WAL fsync duration",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]),
        )?;
        registry.register(Box::new(wal_fsync_duration.clone()))?;

        // Lock metrics
        let lock_wait_seconds_total = HistogramVec::new(
            HistogramOpts::new("neuroquantum_lock_wait_seconds_total", "Lock wait time")
                .buckets(vec![0.0001, 0.001, 0.01, 0.1, 1.0, 10.0]),
            &["lock_type"],
        )?;
        registry.register(Box::new(lock_wait_seconds_total.clone()))?;

        let lock_contention_by_resource = IntCounterVec::new(
            Opts::new(
                "neuroquantum_lock_contention_by_resource",
                "Lock contention by resource",
            ),
            &["resource", "lock_type"],
        )?;
        registry.register(Box::new(lock_contention_by_resource.clone()))?;

        // Index metrics
        let index_scans_total = IntCounterVec::new(
            Opts::new("neuroquantum_index_scans_total", "Total index scans"),
            &["table_name", "index_name"],
        )?;
        registry.register(Box::new(index_scans_total.clone()))?;

        let index_rows_read_total = IntCounterVec::new(
            Opts::new(
                "neuroquantum_index_rows_read_total",
                "Total rows read via index",
            ),
            &["table_name", "index_name"],
        )?;
        registry.register(Box::new(index_rows_read_total.clone()))?;

        let index_hit_ratio =
            GenericGauge::new("neuroquantum_index_hit_ratio", "Index cache hit ratio")?;
        registry.register(Box::new(index_hit_ratio.clone()))?;

        // Memory metrics
        let memory_usage_bytes = IntGauge::new(
            "neuroquantum_memory_usage_bytes",
            "Current memory usage in bytes",
        )?;
        registry.register(Box::new(memory_usage_bytes.clone()))?;

        let memory_limit_bytes =
            IntGauge::new("neuroquantum_memory_limit_bytes", "Memory limit in bytes")?;
        registry.register(Box::new(memory_limit_bytes.clone()))?;

        // System metrics
        let up = IntGauge::new("up", "Service is up")?;
        registry.register(Box::new(up.clone()))?;
        up.set(1);

        let errors_total = IntCounterVec::new(
            Opts::new("neuroquantum_errors_total", "Total errors"),
            &["error_type"],
        )?;
        registry.register(Box::new(errors_total.clone()))?;

        Ok(Self {
            registry: Arc::new(registry),
            queries_total,
            query_duration,
            slow_queries_total,
            query_errors_total,
            active_queries,
            query_queue_length,
            active_connections,
            max_connections,
            connections_total,
            connection_errors_total,
            websocket_connections_active,
            websocket_connections_total,
            websocket_messages_sent_total,
            websocket_messages_received_total,
            websocket_messages_dropped_total,
            websocket_channels_total,
            websocket_active_streams,
            websocket_message_latency,
            buffer_pool_hit_ratio,
            buffer_pool_pages,
            buffer_pool_evictions_total,
            disk_read_bytes_total,
            disk_write_bytes_total,
            disk_read_ops_total,
            disk_write_ops_total,
            wal_bytes_written,
            wal_segments_total,
            wal_fsync_duration,
            lock_wait_seconds_total,
            lock_contention_by_resource,
            index_scans_total,
            index_rows_read_total,
            index_hit_ratio,
            memory_usage_bytes,
            memory_limit_bytes,
            up,
            errors_total,
        })
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }

    /// Get the registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl Default for MetricsExporter {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics exporter")
    }
}

/// HTTP handler for /metrics endpoint
pub async fn metrics_handler(
    State(exporter): State<Arc<MetricsExporter>>,
) -> Result<impl IntoResponse, StatusCode> {
    match exporter.export() {
        Ok(metrics) => Ok((StatusCode::OK, metrics)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Create Axum router for metrics endpoint
pub fn metrics_router(exporter: Arc<MetricsExporter>) -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(exporter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_exporter_creation() {
        let exporter = MetricsExporter::new();
        assert!(exporter.is_ok());
    }

    #[test]
    fn test_metrics_export() {
        let exporter = MetricsExporter::new().unwrap();
        exporter
            .queries_total
            .with_label_values(&["SELECT", "success"])
            .inc();

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_queries_total"));
        assert!(output.contains("up 1"));
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let exporter = Arc::new(MetricsExporter::new().unwrap());
        let app = metrics_router(exporter);

        // This would require axum testing utilities
        // Just verify the router can be created
        assert!(!format!("{:?}", app).is_empty());
    }
}
