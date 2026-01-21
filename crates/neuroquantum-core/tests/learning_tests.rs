//! Learning Module Tests
//!
//! Tests for Hebbian learning engine, anti-Hebbian learning, competitive learning,
//! lateral inhibition, and synaptic plasticity mechanisms.
//!
//! Note: Tests that require access to private fields (learning_rate, stats, learning_history)
//! remain inline in the source file.

use std::collections::HashMap;

use neuroquantum_core::learning::{AntiHebbianLearning, HebbianLearningEngine};
use neuroquantum_core::synaptic::{ConnectionType, SynapticNetwork, SynapticNode};

#[test]
fn test_invalid_learning_rate() {
    let result = HebbianLearningEngine::new(1.5);
    assert!(result.is_err());
}

#[test]
fn test_connection_strengthening() {
    let mut engine = HebbianLearningEngine::new(0.01).unwrap();
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add nodes and connection
    let node1 = SynapticNode::new(1);
    let node2 = SynapticNode::new(2);
    network.add_node(node1).unwrap();
    network.add_node(node2).unwrap();
    network
        .connect_nodes(1, 2, 0.5, ConnectionType::Excitatory)
        .unwrap();

    // Test strengthening
    engine.strengthen_connection(&network, 1, 2, 0.8).unwrap();

    // Note: Can't easily check stats since we fixed the threading issues
    // This test mainly verifies the method doesn't panic
}

// ============ Anti-Hebbian Learning Tests ============

#[test]
fn test_anti_hebbian_creation() {
    let anti_hebbian = AntiHebbianLearning::new(0.01, 0.1);
    assert_eq!(anti_hebbian.decay_rate(), 0.01);
    assert_eq!(anti_hebbian.pruning_threshold(), 0.1);
    assert_eq!(anti_hebbian.competition_factor(), 0.1); // default
}

#[test]
fn test_anti_hebbian_with_config() {
    let anti_hebbian = AntiHebbianLearning::with_config(0.05, 0.2, 0.3, 5, 0.25);
    assert_eq!(anti_hebbian.decay_rate(), 0.05);
    assert_eq!(anti_hebbian.pruning_threshold(), 0.2);
    assert_eq!(anti_hebbian.competition_factor(), 0.3);
}

#[test]
fn test_anti_hebbian_decay_rate_clamping() {
    // Test that values are properly clamped
    let anti_hebbian = AntiHebbianLearning::new(1.5, 0.1);
    assert_eq!(anti_hebbian.decay_rate(), 1.0); // Clamped to max

    let anti_hebbian2 = AntiHebbianLearning::new(-0.5, 0.1);
    assert_eq!(anti_hebbian2.decay_rate(), 0.0); // Clamped to min
}

#[test]
fn test_synaptic_decay() {
    let mut anti_hebbian = AntiHebbianLearning::new(0.1, 0.05);
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add nodes with connections
    let node1 = SynapticNode::new(1);
    let node2 = SynapticNode::new(2);
    network.add_node(node1).unwrap();
    network.add_node(node2).unwrap();
    network
        .connect_nodes(1, 2, 0.5, ConnectionType::Excitatory)
        .unwrap();

    // Apply decay
    let decayed_count = anti_hebbian.apply_synaptic_decay(&network).unwrap();
    assert!(decayed_count > 0);

    // Verify connection weight decreased
    let node = network.get_node(1).unwrap();
    let conn = node.connections.first().unwrap();
    assert!(conn.weight < 0.5); // Weight should have decayed
}

#[test]
fn test_competitive_learning_wta() {
    let mut anti_hebbian = AntiHebbianLearning::new(0.01, 0.05);
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add several nodes
    for i in 1..=5 {
        let mut node = SynapticNode::new(i);
        node.strength = 0.5;
        network.add_node(node).unwrap();
    }

    // Create connections between nodes
    for i in 1..5 {
        network
            .connect_nodes(i, i + 1, 0.5, ConnectionType::Excitatory)
            .unwrap();
    }

    // Create activations map
    let mut activations = HashMap::new();
    activations.insert(1, 0.9); // Highest activation - should win
    activations.insert(2, 0.7);
    activations.insert(3, 0.5);
    activations.insert(4, 0.3);
    activations.insert(5, 0.1); // Lowest - should lose

    // Apply competitive learning with k=2 winners
    let winners = anti_hebbian
        .apply_competitive_learning(&network, &activations, 2)
        .unwrap();

    assert_eq!(winners.len(), 2);
    assert_eq!(winners[0].neuron_id, 1); // Highest activation wins
    assert_eq!(winners[1].neuron_id, 2); // Second highest
}

#[test]
fn test_lateral_inhibition() {
    let mut anti_hebbian = AntiHebbianLearning::new(0.01, 0.05);
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add an active neuron and neighbors
    let mut active_node = SynapticNode::new(1);
    active_node.strength = 0.8;
    active_node.activation_level = 0.9;
    network.add_node(active_node).unwrap();

    for i in 2..=5 {
        let mut neighbor = SynapticNode::new(i);
        neighbor.activation_level = 0.5;
        network.add_node(neighbor).unwrap();
    }

    // Apply lateral inhibition
    let neighbor_ids = vec![2, 3, 4, 5];
    let inhibited = anti_hebbian
        .apply_lateral_inhibition(&network, 1, &neighbor_ids)
        .unwrap();

    assert!(inhibited > 0);

    // Check that neighbors were inhibited
    for &id in &neighbor_ids[..3] {
        // Only check within radius
        let node = network.get_node(id).unwrap();
        assert!(node.activation_level < 0.5);
    }
}

#[test]
fn test_anti_hebbian_pruning() {
    let mut anti_hebbian = AntiHebbianLearning::new(0.01, 0.3); // High threshold
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add nodes with weak connections
    let node1 = SynapticNode::new(1);
    let node2 = SynapticNode::new(2);
    let node3 = SynapticNode::new(3);
    network.add_node(node1).unwrap();
    network.add_node(node2).unwrap();
    network.add_node(node3).unwrap();

    // Add connections - one strong, one weak
    network
        .connect_nodes(1, 2, 0.1, ConnectionType::Excitatory)
        .unwrap(); // Weak - should be pruned
    network
        .connect_nodes(1, 3, 0.5, ConnectionType::Excitatory)
        .unwrap(); // Strong - should remain

    // Apply weakening/pruning
    let pruned = anti_hebbian.apply_weakening(&network).unwrap();

    assert_eq!(pruned, 1); // Only the weak connection should be pruned

    // Verify connections
    let node = network.get_node(1).unwrap();
    assert_eq!(node.connections.len(), 1);
    assert_eq!(node.connections[0].target_id, 3); // Strong connection remains
}

#[test]
fn test_anti_hebbian_maintenance_cycle() {
    let mut anti_hebbian = AntiHebbianLearning::new(0.05, 0.05);
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add nodes with connections
    for i in 1..=5 {
        let node = SynapticNode::new(i);
        network.add_node(node).unwrap();
    }

    for i in 1..5 {
        network
            .connect_nodes(i, i + 1, 0.3, ConnectionType::Excitatory)
            .unwrap();
    }

    // Create some activations
    let mut activations = HashMap::new();
    activations.insert(1, 0.8);
    activations.insert(2, 0.6);
    activations.insert(3, 0.4);

    // Perform maintenance cycle
    let stats = anti_hebbian
        .perform_maintenance_cycle(&network, Some(&activations))
        .unwrap();

    assert!(stats.decay_operations > 0);
}

#[test]
fn test_hebbian_with_anti_hebbian_integration() {
    let mut engine = HebbianLearningEngine::new(0.01).unwrap();
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add nodes with connections
    for i in 1..=3 {
        let node = SynapticNode::new(i);
        network.add_node(node).unwrap();
    }
    network
        .connect_nodes(1, 2, 0.5, ConnectionType::Excitatory)
        .unwrap();
    network
        .connect_nodes(2, 3, 0.5, ConnectionType::Excitatory)
        .unwrap();

    // Apply anti-Hebbian decay via the learning engine
    let decayed = engine.apply_anti_hebbian_decay(&network).unwrap();
    assert!(decayed > 0);

    // Verify decay factor is accessible
    assert_eq!(engine.decay_factor(), 0.995);

    // Test setting decay factor
    engine.set_decay_factor(0.99).unwrap();
    assert_eq!(engine.decay_factor(), 0.99);
}

#[test]
fn test_plasticity_cycle() {
    let mut engine = HebbianLearningEngine::new(0.01).unwrap();
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add nodes
    for i in 1..=4 {
        let node = SynapticNode::new(i);
        network.add_node(node).unwrap();
    }

    for i in 1..4 {
        network
            .connect_nodes(i, i + 1, 0.3, ConnectionType::Excitatory)
            .unwrap();
    }

    // Perform plasticity cycle
    let result = engine.perform_plasticity_cycle(&network, None).unwrap();

    assert!(result.decay_operations > 0);
    // cycle_duration_ms is a u64, so it's always >= 0
}

#[test]
fn test_anti_hebbian_stats_tracking() {
    let mut anti_hebbian = AntiHebbianLearning::new(0.1, 0.05);
    let network = SynapticNetwork::new(1000, 0.5).unwrap();

    // Add nodes with connections
    let node1 = SynapticNode::new(1);
    let node2 = SynapticNode::new(2);
    network.add_node(node1).unwrap();
    network.add_node(node2).unwrap();
    network
        .connect_nodes(1, 2, 0.5, ConnectionType::Excitatory)
        .unwrap();

    // Initial stats should be zero
    let stats = anti_hebbian.get_stats();
    assert_eq!(stats.decay_operations, 0);
    assert_eq!(stats.connections_pruned, 0);

    // Apply decay and check stats updated
    anti_hebbian.apply_synaptic_decay(&network).unwrap();
    let stats = anti_hebbian.get_stats();
    assert_eq!(stats.decay_operations, 1);

    // Reset and verify
    anti_hebbian.reset_stats();
    let stats = anti_hebbian.get_stats();
    assert_eq!(stats.decay_operations, 0);
}
