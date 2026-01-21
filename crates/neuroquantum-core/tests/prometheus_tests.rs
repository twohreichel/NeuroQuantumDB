//! Prometheus Metrics Exporter Tests
//!
//! Tests for the Prometheus metrics exporter including neuromorphic metrics,
//! synaptic network metrics, Hebbian learning metrics, and STDP metrics.

use std::sync::Arc;

use neuroquantum_core::monitoring::prometheus::{metrics_router, MetricsExporter};

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
    assert!(!format!("{app:?}").is_empty());
}
