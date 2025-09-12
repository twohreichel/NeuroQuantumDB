//! Hebbian learning algorithms for synaptic adaptation
//!
//! "Neurons that fire together, wire together" - Donald Hebb

use std::time::Instant;
use parking_lot::RwLock;
use crate::synaptic::{NodeId, SynapticNetwork, ConnectionType};
use crate::error::{CoreError, CoreResult};

/// Hebbian learning engine for synaptic adaptation
pub struct HebbianLearningEngine {
    /// Learning rate (0.0 - 1.0)
    learning_rate: f32,
    /// Decay rate for unused connections
    decay_rate: f32,
    /// Threshold for connection formation
    formation_threshold: f32,
    /// Threshold for connection pruning
    pruning_threshold: f32,
    /// Learning statistics
    stats: RwLock<LearningStats>,
}

#[derive(Debug, Default)]
pub struct LearningStats {
    pub learning_events: u64,
    pub connections_strengthened: u64,
    pub connections_weakened: u64,
    pub connections_formed: u64,
    pub connections_pruned: u64,
    pub avg_learning_time_ns: u64,
}

/// Learning parameters for fine-tuning
#[derive(Debug, Clone)]
pub struct LearningParams {
    pub learning_rate: f32,
    pub decay_rate: f32,
    pub formation_threshold: f32,
    pub pruning_threshold: f32,
    pub max_connections_per_node: usize,
}

impl Default for LearningParams {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            decay_rate: 0.001,
            formation_threshold: 0.3,
            pruning_threshold: 0.1,
            max_connections_per_node: 100,
        }
    }
}

impl HebbianLearningEngine {
    /// Create a new Hebbian learning engine
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate: learning_rate.clamp(0.0, 1.0),
            decay_rate: 0.001,
            formation_threshold: 0.3,
            pruning_threshold: 0.1,
            stats: RwLock::new(LearningStats::default()),
        }
    }

    /// Create with custom parameters
    pub fn with_params(params: LearningParams) -> Self {
        Self {
            learning_rate: params.learning_rate.clamp(0.0, 1.0),
            decay_rate: params.decay_rate.clamp(0.0, 1.0),
            formation_threshold: params.formation_threshold.clamp(0.0, 1.0),
            pruning_threshold: params.pruning_threshold.clamp(0.0, 1.0),
            stats: RwLock::new(LearningStats::default()),
        }
    }

    /// Apply Hebbian learning rule to strengthen synaptic connections
    pub fn strengthen_pathway(
        &self,
        network: &SynapticNetwork,
        source_id: NodeId,
        target_id: NodeId,
        activation_strength: f32,
    ) -> CoreResult<()> {
        let start_time = Instant::now();

        // Get source and target nodes
        let source_ref = network.get_node(source_id)?;
        let target_ref = network.get_node(target_id)?;

        let source_activation = {
            let source_node = source_ref.read();
            source_node.activation
        };

        let target_activation = {
            let target_node = target_ref.read();
            target_node.activation
        };

        // Hebbian rule: Δw = η * x_i * x_j * activation_strength
        // where η is learning rate, x_i and x_j are activations
        let weight_delta = self.learning_rate *
            source_activation *
            target_activation *
            activation_strength;

        // Apply weight update
        if weight_delta.abs() > f32::EPSILON {
            network.strengthen_connection(source_id, target_id, weight_delta)?;

            // Update statistics
            let mut stats = self.stats.write();
            stats.learning_events += 1;
            if weight_delta > 0.0 {
                stats.connections_strengthened += 1;
            } else {
                stats.connections_weakened += 1;
            }

            let elapsed = start_time.elapsed().as_nanos() as u64;
            stats.avg_learning_time_ns =
                (stats.avg_learning_time_ns + elapsed) / 2;
        }

        Ok(())
    }

    /// Form new synaptic connections based on co-activation
    pub fn form_connections(
        &self,
        network: &SynapticNetwork,
        activations: &[(NodeId, f32)],
    ) -> CoreResult<usize> {
        let mut connections_formed = 0;

        // Find pairs of highly activated nodes
        for (i, &(source_id, source_activation)) in activations.iter().enumerate() {
            if source_activation < self.formation_threshold {
                continue;
            }

            for &(target_id, target_activation) in activations.iter().skip(i + 1) {
                if target_activation < self.formation_threshold {
                    continue;
                }

                // Calculate connection strength based on co-activation
                let connection_strength = (source_activation * target_activation).sqrt();

                // Attempt to form bidirectional connections
                match network.connect_nodes(
                    source_id,
                    target_id,
                    connection_strength,
                    ConnectionType::Excitatory
                ) {
                    Ok(()) => {
                        connections_formed += 1;

                        // Try reverse connection
                        if network.connect_nodes(
                            target_id,
                            source_id,
                            connection_strength,
                            ConnectionType::Excitatory
                        ).is_ok() {
                            connections_formed += 1;
                        }
                    }
                    Err(CoreError::ConnectionAlreadyExists { .. }) => {
                        // Connection already exists, strengthen it instead
                        let _ = self.strengthen_pathway(
                            network,
                            source_id,
                            target_id,
                            connection_strength
                        );
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Update formation statistics
        let mut stats = self.stats.write();
        stats.connections_formed += connections_formed as u64;

        Ok(connections_formed)
    }

    /// Prune weak synaptic connections
    pub fn prune_connections(&self, network: &SynapticNetwork) -> CoreResult<usize> {
        let mut connections_pruned = 0;

        // This is a simplified implementation - in practice, we'd need
        // to iterate through all nodes and their connections
        // For now, we'll return the count of pruned connections

        let mut stats = self.stats.write();
        stats.connections_pruned += connections_pruned as u64;

        Ok(connections_pruned)
    }

    /// Apply temporal decay to all connections
    pub fn apply_decay(&self, network: &SynapticNetwork) -> CoreResult<()> {
        // This would iterate through all connections and apply decay
        // Implementation would be similar to pruning but with gradual weakening
        Ok(())
    }

    /// Get learning statistics
    pub fn get_stats(&self) -> LearningStats {
        let stats = self.stats.read();
        LearningStats {
            learning_events: stats.learning_events,
            connections_strengthened: stats.connections_strengthened,
            connections_weakened: stats.connections_weakened,
            connections_formed: stats.connections_formed,
            connections_pruned: stats.connections_pruned,
            avg_learning_time_ns: stats.avg_learning_time_ns,
        }
    }

    /// Reset learning statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = LearningStats::default();
    }

    /// Update learning parameters
    pub fn update_params(&mut self, params: LearningParams) {
        self.learning_rate = params.learning_rate.clamp(0.0, 1.0);
        self.decay_rate = params.decay_rate.clamp(0.0, 1.0);
        self.formation_threshold = params.formation_threshold.clamp(0.0, 1.0);
        self.pruning_threshold = params.pruning_threshold.clamp(0.0, 1.0);
    }
}

/// Anti-Hebbian learning for competitive inhibition
pub struct AntiHebbianLearning {
    inhibition_rate: f32,
    competition_threshold: f32,
}

impl AntiHebbianLearning {
    pub fn new(inhibition_rate: f32) -> Self {
        Self {
            inhibition_rate: inhibition_rate.clamp(0.0, 1.0),
            competition_threshold: 0.7,
        }
    }

    /// Apply competitive inhibition between strongly activated nodes
    pub fn apply_competition(
        &self,
        network: &SynapticNetwork,
        activations: &[(NodeId, f32)],
    ) -> CoreResult<()> {
        // Find highly activated nodes
        let mut competitors: Vec<_> = activations.iter()
            .filter(|(_, activation)| *activation > self.competition_threshold)
            .collect();

        // Sort by activation level (highest first)
        competitors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Apply inhibition between competitors
        for (i, &(source_id, _)) in competitors.iter().enumerate() {
            for &(target_id, _) in competitors.iter().skip(i + 1) {
                // Create or strengthen inhibitory connections
                match network.connect_nodes(
                    *source_id,
                    *target_id,
                    -self.inhibition_rate, // Negative weight for inhibition
                    ConnectionType::Inhibitory
                ) {
                    Ok(()) => {},
                    Err(CoreError::ConnectionAlreadyExists { .. }) => {
                        // Strengthen existing inhibitory connection
                        network.strengthen_connection(*source_id, *target_id, -self.inhibition_rate / 2.0)?;
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synaptic::SynapticNetwork;

    #[test]
    fn test_learning_engine_creation() {
        let engine = HebbianLearningEngine::new(0.05);
        assert_eq!(engine.learning_rate, 0.05);

        let stats = engine.get_stats();
        assert_eq!(stats.learning_events, 0);
    }

    #[test]
    fn test_learning_params() {
        let params = LearningParams {
            learning_rate: 0.02,
            decay_rate: 0.005,
            formation_threshold: 0.4,
            pruning_threshold: 0.15,
            max_connections_per_node: 50,
        };

        let engine = HebbianLearningEngine::with_params(params);
        assert_eq!(engine.learning_rate, 0.02);
        assert_eq!(engine.decay_rate, 0.005);
    }

    #[tokio::test]
    async fn test_pathway_strengthening() {
        let network = SynapticNetwork::new(100).unwrap();
        let engine = HebbianLearningEngine::new(0.1);

        let node1 = network.create_node().unwrap();
        let node2 = network.create_node().unwrap();

        // Create initial connection
        network.connect_nodes(node1, node2, 0.5, ConnectionType::Excitatory).unwrap();

        // Set activations for both nodes
        {
            let node1_ref = network.get_node(node1).unwrap();
            let mut node1_guard = node1_ref.write();
            node1_guard.activation = 0.8;
        }

        {
            let node2_ref = network.get_node(node2).unwrap();
            let mut node2_guard = node2_ref.write();
            node2_guard.activation = 0.7;
        }

        // Apply learning
        engine.strengthen_pathway(&network, node1, node2, 1.0).unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.learning_events, 1);
        assert_eq!(stats.connections_strengthened, 1);
    }

    #[test]
    fn test_anti_hebbian_learning() {
        let anti_hebbian = AntiHebbianLearning::new(0.2);
        assert_eq!(anti_hebbian.inhibition_rate, 0.2);
        assert_eq!(anti_hebbian.competition_threshold, 0.7);
    }

    #[test]
    fn test_learning_rate_clamping() {
        let engine = HebbianLearningEngine::new(1.5); // > 1.0
        assert_eq!(engine.learning_rate, 1.0);

        let engine2 = HebbianLearningEngine::new(-0.5); // < 0.0
        assert_eq!(engine2.learning_rate, 0.0);
    }
}
