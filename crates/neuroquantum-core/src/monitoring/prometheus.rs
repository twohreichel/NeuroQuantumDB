//! Prometheus Metrics Exporter for `NeuroQuantumDB`
//!
//! Provides Prometheus-compatible metrics endpoint for monitoring including
//! comprehensive neuromorphic metrics for synaptic learning, plasticity,
//! and neural network operations.

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

    // ==========================================================
    // Neuromorphic Metrics - Synaptic Learning & Plasticity
    // ==========================================================

    // Synaptic Network Metrics
    /// Total number of neurons in the network
    pub synaptic_neurons_total: IntGauge,
    /// Total number of synaptic connections
    pub synaptic_connections_total: IntGauge,
    /// Average synaptic weight across all connections
    pub synaptic_weight_average: GenericGauge<AtomicU64>,
    /// Distribution of synaptic weights
    pub synaptic_weight_distribution: HistogramVec,
    /// Neuron activations by layer
    pub neuron_activations_total: IntCounterVec,
    /// Neuron firing events
    pub neuron_firing_events_total: IntCounterVec,
    /// Neurons in refractory period
    pub neurons_refractory: IntGauge,

    // Hebbian Learning Metrics
    /// Total Hebbian learning events
    pub hebbian_learning_events_total: IntCounter,
    /// Connections strengthened by Hebbian learning
    pub hebbian_strengthened_connections: IntCounter,
    /// Connections weakened by Hebbian learning
    pub hebbian_weakened_connections: IntCounter,
    /// New connections formed through learning
    pub hebbian_new_connections: IntCounter,
    /// Learning rate adaptation over time
    pub hebbian_learning_rate: GenericGauge<AtomicU64>,
    /// Learning efficiency ratio
    pub hebbian_learning_efficiency: GenericGauge<AtomicU64>,

    // Anti-Hebbian Learning Metrics
    /// Synaptic decay operations applied
    pub anti_hebbian_decay_operations: IntCounter,
    /// Connections pruned by anti-Hebbian learning
    pub anti_hebbian_pruned_connections: IntCounter,
    /// Competition loser neurons
    pub anti_hebbian_competition_losers: IntCounter,
    /// Lateral inhibition events
    pub anti_hebbian_lateral_inhibition: IntCounter,
    /// Average decay applied per operation
    pub anti_hebbian_average_decay: GenericGauge<AtomicU64>,

    // STDP (Spike-Timing-Dependent Plasticity) Metrics
    /// STDP potentiation events (pre before post)
    pub stdp_potentiation_events: IntCounter,
    /// STDP depression events (post before pre)
    pub stdp_depression_events: IntCounter,
    /// STDP timing window distribution
    pub stdp_timing_distribution: Histogram,
    /// Total weight change from STDP
    pub stdp_weight_change_total: GenericGauge<AtomicU64>,

    // Plasticity Matrix Metrics
    /// Network reorganization events
    pub plasticity_reorganizations_total: IntCounterVec,
    /// Nodes affected by plasticity operations
    pub plasticity_nodes_affected: IntCounter,
    /// Current plasticity efficiency score
    pub plasticity_efficiency: GenericGauge<AtomicU64>,
    /// Cluster count in the network
    pub plasticity_cluster_count: IntGauge,
    /// Memory delta from plasticity operations (bytes)
    pub plasticity_memory_delta: IntGauge,
    /// Consolidation operations performed
    pub plasticity_consolidations_total: IntCounter,
    /// Capacity utilization of the synaptic network
    pub plasticity_capacity_utilization: GenericGauge<AtomicU64>,

    // Access Pattern Metrics
    /// Node access frequency distribution
    pub access_pattern_frequency: HistogramVec,
    /// Temporal locality score
    pub access_pattern_temporal_locality: GenericGauge<AtomicU64>,
    /// Spatial locality score
    pub access_pattern_spatial_locality: GenericGauge<AtomicU64>,

    // Neural Network Performance Metrics
    /// Forward propagation duration
    pub neural_forward_propagation_duration: Histogram,
    /// Backpropagation duration
    pub neural_backpropagation_duration: Histogram,
    /// Network convergence progress
    pub neural_convergence_progress: GenericGauge<AtomicU64>,
    /// Training epochs completed
    pub neural_training_epochs: IntCounter,
    /// Inference latency
    pub neural_inference_latency: Histogram,
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

        // ==========================================================
        // Neuromorphic Metrics - Synaptic Learning & Plasticity
        // ==========================================================

        // Synaptic Network Metrics
        let synaptic_neurons_total = IntGauge::new(
            "neuroquantum_synaptic_neurons_total",
            "Total number of neurons in the synaptic network",
        )?;
        registry.register(Box::new(synaptic_neurons_total.clone()))?;

        let synaptic_connections_total = IntGauge::new(
            "neuroquantum_synaptic_connections_total",
            "Total number of synaptic connections",
        )?;
        registry.register(Box::new(synaptic_connections_total.clone()))?;

        let synaptic_weight_average = GenericGauge::new(
            "neuroquantum_synaptic_weight_average",
            "Average synaptic weight across all connections (scaled by 1000)",
        )?;
        registry.register(Box::new(synaptic_weight_average.clone()))?;

        let synaptic_weight_distribution = HistogramVec::new(
            HistogramOpts::new(
                "neuroquantum_synaptic_weight_distribution",
                "Distribution of synaptic connection weights",
            )
            .buckets(vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0]),
            &["connection_type"],
        )?;
        registry.register(Box::new(synaptic_weight_distribution.clone()))?;

        let neuron_activations_total = IntCounterVec::new(
            Opts::new(
                "neuroquantum_neuron_activations_total",
                "Total neuron activation events by layer",
            ),
            &["layer", "activation_function"],
        )?;
        registry.register(Box::new(neuron_activations_total.clone()))?;

        let neuron_firing_events_total = IntCounterVec::new(
            Opts::new(
                "neuroquantum_neuron_firing_events_total",
                "Total neuron firing (spike) events",
            ),
            &["layer"],
        )?;
        registry.register(Box::new(neuron_firing_events_total.clone()))?;

        let neurons_refractory = IntGauge::new(
            "neuroquantum_neurons_refractory",
            "Number of neurons currently in refractory period",
        )?;
        registry.register(Box::new(neurons_refractory.clone()))?;

        // Hebbian Learning Metrics
        let hebbian_learning_events_total = IntCounter::new(
            "neuroquantum_hebbian_learning_events_total",
            "Total Hebbian learning events (LTP/LTD)",
        )?;
        registry.register(Box::new(hebbian_learning_events_total.clone()))?;

        let hebbian_strengthened_connections = IntCounter::new(
            "neuroquantum_hebbian_strengthened_connections_total",
            "Connections strengthened by Hebbian learning (LTP)",
        )?;
        registry.register(Box::new(hebbian_strengthened_connections.clone()))?;

        let hebbian_weakened_connections = IntCounter::new(
            "neuroquantum_hebbian_weakened_connections_total",
            "Connections weakened by Hebbian learning (LTD)",
        )?;
        registry.register(Box::new(hebbian_weakened_connections.clone()))?;

        let hebbian_new_connections = IntCounter::new(
            "neuroquantum_hebbian_new_connections_total",
            "New synaptic connections formed through learning",
        )?;
        registry.register(Box::new(hebbian_new_connections.clone()))?;

        let hebbian_learning_rate = GenericGauge::new(
            "neuroquantum_hebbian_learning_rate",
            "Current adaptive learning rate (scaled by 10000)",
        )?;
        registry.register(Box::new(hebbian_learning_rate.clone()))?;

        let hebbian_learning_efficiency = GenericGauge::new(
            "neuroquantum_hebbian_learning_efficiency",
            "Learning efficiency ratio (0-100)",
        )?;
        registry.register(Box::new(hebbian_learning_efficiency.clone()))?;

        // Anti-Hebbian Learning Metrics
        let anti_hebbian_decay_operations = IntCounter::new(
            "neuroquantum_anti_hebbian_decay_operations_total",
            "Total synaptic decay operations applied",
        )?;
        registry.register(Box::new(anti_hebbian_decay_operations.clone()))?;

        let anti_hebbian_pruned_connections = IntCounter::new(
            "neuroquantum_anti_hebbian_pruned_connections_total",
            "Total connections pruned by anti-Hebbian learning",
        )?;
        registry.register(Box::new(anti_hebbian_pruned_connections.clone()))?;

        let anti_hebbian_competition_losers = IntCounter::new(
            "neuroquantum_anti_hebbian_competition_losers_total",
            "Neurons that lost in competitive learning",
        )?;
        registry.register(Box::new(anti_hebbian_competition_losers.clone()))?;

        let anti_hebbian_lateral_inhibition = IntCounter::new(
            "neuroquantum_anti_hebbian_lateral_inhibition_total",
            "Lateral inhibition events (surround suppression)",
        )?;
        registry.register(Box::new(anti_hebbian_lateral_inhibition.clone()))?;

        let anti_hebbian_average_decay = GenericGauge::new(
            "neuroquantum_anti_hebbian_average_decay",
            "Average decay applied per operation (scaled by 10000)",
        )?;
        registry.register(Box::new(anti_hebbian_average_decay.clone()))?;

        // STDP (Spike-Timing-Dependent Plasticity) Metrics
        let stdp_potentiation_events = IntCounter::new(
            "neuroquantum_stdp_potentiation_events_total",
            "STDP potentiation events (pre-synaptic before post-synaptic)",
        )?;
        registry.register(Box::new(stdp_potentiation_events.clone()))?;

        let stdp_depression_events = IntCounter::new(
            "neuroquantum_stdp_depression_events_total",
            "STDP depression events (post-synaptic before pre-synaptic)",
        )?;
        registry.register(Box::new(stdp_depression_events.clone()))?;

        let stdp_timing_distribution = Histogram::with_opts(
            HistogramOpts::new(
                "neuroquantum_stdp_timing_distribution_ms",
                "STDP timing window distribution in milliseconds",
            )
            .buckets(vec![
                -50.0, -40.0, -30.0, -20.0, -10.0, 0.0, 10.0, 20.0, 30.0, 40.0, 50.0,
            ]),
        )?;
        registry.register(Box::new(stdp_timing_distribution.clone()))?;

        let stdp_weight_change_total = GenericGauge::new(
            "neuroquantum_stdp_weight_change_total",
            "Cumulative weight change from STDP (scaled by 10000)",
        )?;
        registry.register(Box::new(stdp_weight_change_total.clone()))?;

        // Plasticity Matrix Metrics
        let plasticity_reorganizations_total = IntCounterVec::new(
            Opts::new(
                "neuroquantum_plasticity_reorganizations_total",
                "Network reorganization events by type",
            ),
            &["reorganization_type"],
        )?;
        registry.register(Box::new(plasticity_reorganizations_total.clone()))?;

        let plasticity_nodes_affected = IntCounter::new(
            "neuroquantum_plasticity_nodes_affected_total",
            "Total nodes affected by plasticity operations",
        )?;
        registry.register(Box::new(plasticity_nodes_affected.clone()))?;

        let plasticity_efficiency = GenericGauge::new(
            "neuroquantum_plasticity_efficiency",
            "Current plasticity efficiency score (0-100)",
        )?;
        registry.register(Box::new(plasticity_efficiency.clone()))?;

        let plasticity_cluster_count = IntGauge::new(
            "neuroquantum_plasticity_cluster_count",
            "Number of clusters in the synaptic network",
        )?;
        registry.register(Box::new(plasticity_cluster_count.clone()))?;

        let plasticity_memory_delta = IntGauge::new(
            "neuroquantum_plasticity_memory_delta_bytes",
            "Memory change from plasticity operations in bytes",
        )?;
        registry.register(Box::new(plasticity_memory_delta.clone()))?;

        let plasticity_consolidations_total = IntCounter::new(
            "neuroquantum_plasticity_consolidations_total",
            "Total consolidation operations performed",
        )?;
        registry.register(Box::new(plasticity_consolidations_total.clone()))?;

        let plasticity_capacity_utilization = GenericGauge::new(
            "neuroquantum_plasticity_capacity_utilization",
            "Synaptic network capacity utilization (0-100)",
        )?;
        registry.register(Box::new(plasticity_capacity_utilization.clone()))?;

        // Access Pattern Metrics
        let access_pattern_frequency = HistogramVec::new(
            HistogramOpts::new(
                "neuroquantum_access_pattern_frequency",
                "Node access frequency distribution",
            )
            .buckets(vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0]),
            &["node_type"],
        )?;
        registry.register(Box::new(access_pattern_frequency.clone()))?;

        let access_pattern_temporal_locality = GenericGauge::new(
            "neuroquantum_access_pattern_temporal_locality",
            "Temporal locality score (0-100)",
        )?;
        registry.register(Box::new(access_pattern_temporal_locality.clone()))?;

        let access_pattern_spatial_locality = GenericGauge::new(
            "neuroquantum_access_pattern_spatial_locality",
            "Spatial locality score (0-100)",
        )?;
        registry.register(Box::new(access_pattern_spatial_locality.clone()))?;

        // Neural Network Performance Metrics
        let neural_forward_propagation_duration = Histogram::with_opts(
            HistogramOpts::new(
                "neuroquantum_neural_forward_propagation_seconds",
                "Forward propagation duration in seconds",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5]),
        )?;
        registry.register(Box::new(neural_forward_propagation_duration.clone()))?;

        let neural_backpropagation_duration = Histogram::with_opts(
            HistogramOpts::new(
                "neuroquantum_neural_backpropagation_seconds",
                "Backpropagation duration in seconds",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5]),
        )?;
        registry.register(Box::new(neural_backpropagation_duration.clone()))?;

        let neural_convergence_progress = GenericGauge::new(
            "neuroquantum_neural_convergence_progress",
            "Network convergence progress (0-100)",
        )?;
        registry.register(Box::new(neural_convergence_progress.clone()))?;

        let neural_training_epochs = IntCounter::new(
            "neuroquantum_neural_training_epochs_total",
            "Total training epochs completed",
        )?;
        registry.register(Box::new(neural_training_epochs.clone()))?;

        let neural_inference_latency = Histogram::with_opts(
            HistogramOpts::new(
                "neuroquantum_neural_inference_latency_seconds",
                "Neural network inference latency in seconds",
            )
            .buckets(vec![0.00001, 0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05]),
        )?;
        registry.register(Box::new(neural_inference_latency.clone()))?;

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
            // Neuromorphic Metrics
            synaptic_neurons_total,
            synaptic_connections_total,
            synaptic_weight_average,
            synaptic_weight_distribution,
            neuron_activations_total,
            neuron_firing_events_total,
            neurons_refractory,
            hebbian_learning_events_total,
            hebbian_strengthened_connections,
            hebbian_weakened_connections,
            hebbian_new_connections,
            hebbian_learning_rate,
            hebbian_learning_efficiency,
            anti_hebbian_decay_operations,
            anti_hebbian_pruned_connections,
            anti_hebbian_competition_losers,
            anti_hebbian_lateral_inhibition,
            anti_hebbian_average_decay,
            stdp_potentiation_events,
            stdp_depression_events,
            stdp_timing_distribution,
            stdp_weight_change_total,
            plasticity_reorganizations_total,
            plasticity_nodes_affected,
            plasticity_efficiency,
            plasticity_cluster_count,
            plasticity_memory_delta,
            plasticity_consolidations_total,
            plasticity_capacity_utilization,
            access_pattern_frequency,
            access_pattern_temporal_locality,
            access_pattern_spatial_locality,
            neural_forward_propagation_duration,
            neural_backpropagation_duration,
            neural_convergence_progress,
            neural_training_epochs,
            neural_inference_latency,
        })
    }

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer)
            .map_err(|e| prometheus::Error::Msg(format!("Invalid UTF-8 in metrics: {e}")))
    }

    /// Get the registry
    #[must_use] 
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl Default for MetricsExporter {
    /// Creates a default metrics exporter.
    ///
    /// # Panics
    ///
    /// Panics if metrics registry initialization fails. This is acceptable for
    /// Default implementations used during application startup.
    #[allow(clippy::expect_used)] // Acceptable for Default impl at startup
    fn default() -> Self {
        Self::new().expect("Failed to create metrics exporter")
    }
}

/// HTTP handler for /metrics endpoint
pub async fn metrics_handler(
    State(exporter): State<Arc<MetricsExporter>>,
) -> Result<impl IntoResponse, StatusCode> {
    match exporter.export() {
        | Ok(metrics) => Ok((StatusCode::OK, metrics)),
        | Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
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

    #[test]
    fn test_neuromorphic_metrics_synaptic() {
        let exporter = MetricsExporter::new().unwrap();

        // Test synaptic network metrics
        exporter.synaptic_neurons_total.set(1000);
        exporter.synaptic_connections_total.set(5000);
        exporter.neurons_refractory.set(50);

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_synaptic_neurons_total 1000"));
        assert!(output.contains("neuroquantum_synaptic_connections_total 5000"));
        assert!(output.contains("neuroquantum_neurons_refractory 50"));
    }

    #[test]
    fn test_neuromorphic_metrics_hebbian() {
        let exporter = MetricsExporter::new().unwrap();

        // Test Hebbian learning metrics
        exporter.hebbian_learning_events_total.inc_by(100);
        exporter.hebbian_strengthened_connections.inc_by(75);
        exporter.hebbian_weakened_connections.inc_by(25);
        exporter.hebbian_new_connections.inc_by(10);

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_hebbian_learning_events_total 100"));
        assert!(output.contains("neuroquantum_hebbian_strengthened_connections_total 75"));
        assert!(output.contains("neuroquantum_hebbian_weakened_connections_total 25"));
        assert!(output.contains("neuroquantum_hebbian_new_connections_total 10"));
    }

    #[test]
    fn test_neuromorphic_metrics_anti_hebbian() {
        let exporter = MetricsExporter::new().unwrap();

        // Test Anti-Hebbian learning metrics
        exporter.anti_hebbian_decay_operations.inc_by(500);
        exporter.anti_hebbian_pruned_connections.inc_by(150);
        exporter.anti_hebbian_competition_losers.inc_by(30);
        exporter.anti_hebbian_lateral_inhibition.inc_by(200);

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_anti_hebbian_decay_operations_total 500"));
        assert!(output.contains("neuroquantum_anti_hebbian_pruned_connections_total 150"));
        assert!(output.contains("neuroquantum_anti_hebbian_competition_losers_total 30"));
        assert!(output.contains("neuroquantum_anti_hebbian_lateral_inhibition_total 200"));
    }

    #[test]
    fn test_neuromorphic_metrics_stdp() {
        let exporter = MetricsExporter::new().unwrap();

        // Test STDP metrics
        exporter.stdp_potentiation_events.inc_by(1000);
        exporter.stdp_depression_events.inc_by(800);

        // Test timing distribution (positive = potentiation, negative = depression)
        exporter.stdp_timing_distribution.observe(10.0); // pre before post
        exporter.stdp_timing_distribution.observe(-15.0); // post before pre

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_stdp_potentiation_events_total 1000"));
        assert!(output.contains("neuroquantum_stdp_depression_events_total 800"));
        assert!(output.contains("neuroquantum_stdp_timing_distribution_ms"));
    }

    #[test]
    fn test_neuromorphic_metrics_plasticity() {
        let exporter = MetricsExporter::new().unwrap();

        // Test plasticity metrics
        exporter
            .plasticity_reorganizations_total
            .with_label_values(&["SpatialClustering"])
            .inc_by(5);
        exporter
            .plasticity_reorganizations_total
            .with_label_values(&["TemporalReorganization"])
            .inc_by(3);
        exporter.plasticity_nodes_affected.inc_by(250);
        exporter.plasticity_cluster_count.set(15);
        exporter.plasticity_memory_delta.set(-1024);
        exporter.plasticity_consolidations_total.inc_by(10);

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_plasticity_reorganizations_total"));
        assert!(output.contains("neuroquantum_plasticity_nodes_affected_total 250"));
        assert!(output.contains("neuroquantum_plasticity_cluster_count 15"));
        assert!(output.contains("neuroquantum_plasticity_consolidations_total 10"));
    }

    #[test]
    fn test_neuromorphic_metrics_neural_network() {
        let exporter = MetricsExporter::new().unwrap();

        // Test neural network performance metrics
        exporter.neural_forward_propagation_duration.observe(0.005);
        exporter.neural_backpropagation_duration.observe(0.008);
        exporter.neural_training_epochs.inc_by(100);
        exporter.neural_inference_latency.observe(0.0001);

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_neural_forward_propagation_seconds"));
        assert!(output.contains("neuroquantum_neural_backpropagation_seconds"));
        assert!(output.contains("neuroquantum_neural_training_epochs_total 100"));
        assert!(output.contains("neuroquantum_neural_inference_latency_seconds"));
    }

    #[test]
    fn test_neuromorphic_metrics_neuron_activations() {
        let exporter = MetricsExporter::new().unwrap();

        // Test neuron activations by layer and function
        exporter
            .neuron_activations_total
            .with_label_values(&["input", "Sigmoid"])
            .inc_by(10000);
        exporter
            .neuron_activations_total
            .with_label_values(&["hidden", "ReLU"])
            .inc_by(50000);
        exporter
            .neuron_activations_total
            .with_label_values(&["output", "Tanh"])
            .inc_by(1000);
        exporter
            .neuron_firing_events_total
            .with_label_values(&["hidden"])
            .inc_by(25000);

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_neuron_activations_total"));
        assert!(output.contains("neuroquantum_neuron_firing_events_total"));
    }

    #[test]
    fn test_neuromorphic_metrics_synaptic_weight_distribution() {
        let exporter = MetricsExporter::new().unwrap();

        // Test weight distribution by connection type
        exporter
            .synaptic_weight_distribution
            .with_label_values(&["Excitatory"])
            .observe(0.7);
        exporter
            .synaptic_weight_distribution
            .with_label_values(&["Excitatory"])
            .observe(0.8);
        exporter
            .synaptic_weight_distribution
            .with_label_values(&["Inhibitory"])
            .observe(0.3);
        exporter
            .synaptic_weight_distribution
            .with_label_values(&["Modulatory"])
            .observe(0.5);

        let output = exporter.export().unwrap();
        assert!(output.contains("neuroquantum_synaptic_weight_distribution"));
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
