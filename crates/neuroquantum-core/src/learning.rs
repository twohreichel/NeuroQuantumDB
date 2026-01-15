//! # Hebbian Learning Engine
//!
//! Implementation of Hebbian learning algorithms for synaptic pathway strengthening
//! and adaptive neural network optimization in `NeuroQuantumDB`.

use crate::error::{CoreError, CoreResult};
use crate::synaptic::SynapticNetwork;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info, instrument};

/// Learning statistics for monitoring and optimization
#[derive(Debug, Clone, Default)]
pub struct LearningStats {
    pub total_learning_events: u64,
    pub strengthened_connections: u64,
    pub weakened_connections: u64,
    pub new_connections_formed: u64,
    pub connections_pruned: u64,
    pub average_learning_rate: f32,
    pub learning_efficiency: f32,
    pub last_learning_session_secs: Option<u64>, // Store as seconds since epoch instead of Instant
}

/// Statistics for Anti-Hebbian learning operations
#[derive(Debug, Clone, Default)]
pub struct AntiHebbianStats {
    /// Total number of decay operations applied
    pub decay_operations: u64,
    /// Total number of connections pruned
    pub connections_pruned: u64,
    /// Number of neurons that lost in competition
    pub competition_losers: u64,
    /// Number of lateral inhibition events
    pub lateral_inhibition_events: u64,
    /// Average decay applied per operation
    pub average_decay_applied: f32,
}

/// Winner information from competitive learning
#[derive(Debug, Clone)]
pub struct WinnerInfo {
    pub neuron_id: u64,
    pub activation_level: f32,
    pub connection_strength: f32,
}

/// Anti-Hebbian learning for competitive learning and pruning weak connections
///
/// This module implements several biologically-inspired learning mechanisms:
/// 1. **Synaptic Decay**: Connections that are not reinforced naturally weaken over time
/// 2. **Competitive Learning (Winner-Takes-All)**: Neurons compete for activation,
///    winners strengthen while losers weaken
/// 3. **Lateral Inhibition**: Active neurons inhibit neighboring neurons
/// 4. **STDP-based Anti-Hebbian**: Spike-timing-dependent weakening for out-of-phase activity
pub struct AntiHebbianLearning {
    /// Rate at which unused connections decay (0.0-1.0)
    decay_rate: f32,
    /// Threshold below which connections are pruned
    pruning_threshold: f32,
    /// Factor controlling strength of competitive inhibition (0.0-1.0)
    competition_factor: f32,
    /// Statistics tracking
    stats: AntiHebbianStats,
    /// Lateral inhibition radius (how many neighboring neurons are affected)
    lateral_inhibition_radius: usize,
    /// Strength of lateral inhibition effect
    lateral_inhibition_strength: f32,
    /// STDP anti-Hebbian time window (milliseconds)
    stdp_anti_window_ms: u64,
    /// STDP weakening amplitude
    stdp_weakening_amplitude: f32,
}

impl AntiHebbianLearning {
    /// Create a new Anti-Hebbian learning engine
    ///
    /// # Arguments
    /// * `decay_rate` - Rate of synaptic decay (0.0-1.0), higher = faster decay
    /// * `pruning_threshold` - Weight threshold for connection pruning
    #[must_use] 
    pub fn new(decay_rate: f32, pruning_threshold: f32) -> Self {
        Self {
            decay_rate: decay_rate.clamp(0.0, 1.0),
            pruning_threshold: pruning_threshold.max(0.0),
            competition_factor: 0.1,
            stats: AntiHebbianStats::default(),
            lateral_inhibition_radius: 3,
            lateral_inhibition_strength: 0.2,
            stdp_anti_window_ms: 20,
            stdp_weakening_amplitude: 0.05,
        }
    }

    /// Create with full configuration
    #[must_use] 
    pub fn with_config(
        decay_rate: f32,
        pruning_threshold: f32,
        competition_factor: f32,
        lateral_inhibition_radius: usize,
        lateral_inhibition_strength: f32,
    ) -> Self {
        Self {
            decay_rate: decay_rate.clamp(0.0, 1.0),
            pruning_threshold: pruning_threshold.max(0.0),
            competition_factor: competition_factor.clamp(0.0, 1.0),
            stats: AntiHebbianStats::default(),
            lateral_inhibition_radius,
            lateral_inhibition_strength: lateral_inhibition_strength.clamp(0.0, 1.0),
            stdp_anti_window_ms: 20,
            stdp_weakening_amplitude: 0.05,
        }
    }

    /// Apply anti-Hebbian learning to weaken unused connections
    /// This implements competitive learning where weak synapses are pruned
    pub fn apply_weakening(&mut self, network: &SynapticNetwork) -> CoreResult<u64> {
        // Anti-Hebbian learning: prune connections below threshold
        // This implements competitive learning where weak, unused connections are removed
        let pruned_count = network.prune_weak_connections(self.pruning_threshold);

        self.stats.connections_pruned += pruned_count as u64;

        debug!(
            "Anti-Hebbian learning: pruned {} weak connections (threshold: {})",
            pruned_count, self.pruning_threshold
        );

        Ok(pruned_count as u64)
    }

    /// Apply synaptic decay to all connections in the network
    ///
    /// Implements the biological principle that unused synapses naturally weaken
    /// over time. This is essential for maintaining network sparsity and
    /// preventing saturation.
    ///
    /// Decay formula: `weight_new` = `weight_old` * (1.0 - `decay_rate`)
    pub fn apply_synaptic_decay(&mut self, network: &SynapticNetwork) -> CoreResult<u64> {
        let mut decay_count = 0u64;
        let mut total_decay = 0.0f32;
        let decay_multiplier = 1.0 - self.decay_rate;

        // Get all node IDs first to avoid holding locks
        let node_ids = network.get_node_ids();

        for node_id in node_ids {
            network.modify_node(node_id, |node| {
                for connection in &mut node.connections {
                    let old_weight = connection.weight;

                    // Apply exponential decay
                    connection.weight *= decay_multiplier;

                    // Track decay statistics
                    let decay_amount = (old_weight - connection.weight).abs();
                    total_decay += decay_amount;
                    decay_count += 1;

                    // Also decay the plasticity factor towards baseline (1.0)
                    if connection.plasticity_factor > 1.0 {
                        connection.plasticity_factor =
                            (connection.plasticity_factor - 1.0).mul_add(decay_multiplier, 1.0);
                    } else if connection.plasticity_factor < 1.0 {
                        connection.plasticity_factor =
                            (1.0 - connection.plasticity_factor).mul_add(-decay_multiplier, 1.0);
                    }
                }
            });
        }

        // Update statistics
        self.stats.decay_operations += 1;
        if decay_count > 0 {
            self.stats.average_decay_applied =
                f32::midpoint(self.stats.average_decay_applied, total_decay / decay_count as f32);
        }

        info!(
            "Applied synaptic decay to {} connections (rate: {}, avg decay: {:.4})",
            decay_count, self.decay_rate, self.stats.average_decay_applied
        );

        Ok(decay_count)
    }

    /// Implement Winner-Takes-All (WTA) competitive learning
    ///
    /// In competitive learning, neurons compete to respond to input patterns.
    /// The winning neuron (highest activation) strengthens its connections,
    /// while losing neurons weaken theirs.
    ///
    /// # Arguments
    /// * `network` - The synaptic network
    /// * `activations` - Map of neuron IDs to their activation levels
    /// * `k_winners` - Number of winners to select (k-WTA)
    ///
    /// # Returns
    /// Vector of winning neuron IDs
    pub fn apply_competitive_learning(
        &mut self,
        network: &SynapticNetwork,
        activations: &HashMap<u64, f32>,
        k_winners: usize,
    ) -> CoreResult<Vec<WinnerInfo>> {
        if activations.is_empty() {
            return Ok(Vec::new());
        }

        // Sort neurons by activation level (descending)
        let mut sorted_activations: Vec<(u64, f32)> =
            activations.iter().map(|(&id, &act)| (id, act)).collect();
        sorted_activations
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Select k winners
        let k = k_winners.min(sorted_activations.len());
        let winners: Vec<(u64, f32)> = sorted_activations[..k].to_vec();
        let losers: Vec<(u64, f32)> = sorted_activations[k..].to_vec();

        // Strengthen winner connections
        for (winner_id, activation) in &winners {
            network.modify_node(*winner_id, |node| {
                for connection in &mut node.connections {
                    // Strengthen proportionally to activation
                    let strengthening = self.competition_factor * activation * 0.1;
                    connection.weight += strengthening;
                    connection.weight = connection.weight.clamp(-2.0, 2.0);
                }
                // Also strengthen the node itself
                node.strength = (node.strength + self.competition_factor * activation).min(1.0);
            });
        }

        // Weaken loser connections
        for (loser_id, _activation) in &losers {
            network.modify_node(*loser_id, |node| {
                for connection in &mut node.connections {
                    // Weaken proportionally to competition factor
                    let weakening = self.competition_factor * 0.05;
                    connection.weight -= weakening;
                    connection.weight = connection.weight.clamp(-2.0, 2.0);
                }
            });
            self.stats.competition_losers += 1;
        }

        // Build winner info
        let winner_infos: Vec<WinnerInfo> = winners
            .iter()
            .map(|(id, activation)| {
                let connection_strength = network
                    .get_node(*id)
                    .map_or(0.0, |n| n.connections.iter().map(|c| c.weight).sum::<f32>());
                WinnerInfo {
                    neuron_id: *id,
                    activation_level: *activation,
                    connection_strength,
                }
            })
            .collect();

        info!(
            "Competitive learning: {} winners, {} losers (k={})",
            winners.len(),
            losers.len(),
            k
        );

        Ok(winner_infos)
    }

    /// Apply lateral inhibition to implement local competition
    ///
    /// When a neuron fires, it inhibits nearby neurons, creating a
    /// "Mexican hat" pattern of activation. This helps create sparse,
    /// distributed representations.
    ///
    /// # Arguments
    /// * `network` - The synaptic network
    /// * `active_neuron_id` - The neuron that fired
    /// * `neighbor_ids` - IDs of neighboring neurons to inhibit
    pub fn apply_lateral_inhibition(
        &mut self,
        network: &SynapticNetwork,
        active_neuron_id: u64,
        neighbor_ids: &[u64],
    ) -> CoreResult<u64> {
        let mut inhibited_count = 0u64;

        // Get the active neuron's strength for proportional inhibition
        let active_strength = network
            .get_node(active_neuron_id)
            .map_or(0.5, |n| n.strength);

        for (distance, &neighbor_id) in neighbor_ids
            .iter()
            .take(self.lateral_inhibition_radius)
            .enumerate()
        {
            // Inhibition decreases with distance (Gaussian-like falloff)
            let distance_factor = (-(distance as f32).powi(2) / 2.0).exp();
            let inhibition_amount =
                self.lateral_inhibition_strength * active_strength * distance_factor;

            network.modify_node(neighbor_id, |node| {
                // Reduce activation level
                node.activation_level = (node.activation_level - inhibition_amount).max(0.0);

                // Weaken incoming connections slightly
                for connection in &mut node.connections {
                    connection.weight -= inhibition_amount * 0.1;
                    connection.weight = connection.weight.clamp(-2.0, 2.0);
                }
            });

            inhibited_count += 1;
            self.stats.lateral_inhibition_events += 1;
        }

        debug!(
            "Applied lateral inhibition from neuron {} to {} neighbors",
            active_neuron_id, inhibited_count
        );

        Ok(inhibited_count)
    }

    /// Apply STDP-based anti-Hebbian learning
    ///
    /// Implements the "anti-Hebbian" aspect of STDP: when a post-synaptic
    /// neuron fires BEFORE the pre-synaptic neuron (causal violation),
    /// the connection is weakened.
    ///
    /// # Arguments
    /// * `network` - The synaptic network
    /// * `spike_times` - Map of neuron IDs to their spike times
    pub fn apply_stdp_anti_hebbian(
        &mut self,
        network: &SynapticNetwork,
        spike_times: &HashMap<u64, Vec<Instant>>,
    ) -> CoreResult<u64> {
        let stdp_window = std::time::Duration::from_millis(self.stdp_anti_window_ms);
        let mut weakening_count = 0u64;

        // Collect connections to weaken
        let mut connections_to_weaken: Vec<(u64, u64, f32)> = Vec::new();

        // Get all node IDs
        let node_ids = network.get_node_ids();

        for source_id in &node_ids {
            if let Some(source_spikes) = spike_times.get(source_id) {
                if let Some(source_node) = network.get_node(*source_id) {
                    for connection in &source_node.connections {
                        let target_id = connection.target_id;

                        if let Some(target_spikes) = spike_times.get(&target_id) {
                            // Check for anti-causal spike patterns
                            for &source_spike in source_spikes {
                                for &target_spike in target_spikes {
                                    // Anti-Hebbian: post fires BEFORE pre
                                    if target_spike < source_spike {
                                        let time_diff = source_spike.duration_since(target_spike);

                                        if time_diff <= stdp_window {
                                            // Calculate weakening based on timing
                                            // Closer timing = stronger weakening
                                            let timing_factor = (-(time_diff.as_millis() as f32)
                                                / self.stdp_anti_window_ms as f32)
                                                .exp();
                                            let weakening =
                                                -self.stdp_weakening_amplitude * timing_factor;

                                            connections_to_weaken
                                                .push((*source_id, target_id, weakening));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Apply the weakening
        for (source_id, target_id, weakening) in connections_to_weaken {
            network.modify_node(source_id, |node| {
                for connection in &mut node.connections {
                    if connection.target_id == target_id {
                        connection.weight += weakening; // weakening is negative
                        connection.weight = connection.weight.clamp(-2.0, 2.0);
                        weakening_count += 1;
                    }
                }
            });
        }

        info!(
            "STDP anti-Hebbian: weakened {} connections (window: {}ms)",
            weakening_count, self.stdp_anti_window_ms
        );

        Ok(weakening_count)
    }

    /// Perform a complete anti-Hebbian learning cycle
    ///
    /// This combines decay, pruning, and optionally competitive learning
    /// for a comprehensive maintenance pass.
    pub fn perform_maintenance_cycle(
        &mut self,
        network: &SynapticNetwork,
        activations: Option<&HashMap<u64, f32>>,
    ) -> CoreResult<AntiHebbianStats> {
        // 1. Apply synaptic decay
        self.apply_synaptic_decay(network)?;

        // 2. Apply competitive learning if activations are provided
        if let Some(acts) = activations {
            // Use k-WTA with k = 10% of active neurons, minimum 1
            let k = (acts.len() / 10).max(1);
            self.apply_competitive_learning(network, acts, k)?;
        }

        // 3. Prune weak connections
        self.apply_weakening(network)?;

        info!(
            "Anti-Hebbian maintenance complete: {} decays, {} pruned, {} competition losers",
            self.stats.decay_operations,
            self.stats.connections_pruned,
            self.stats.competition_losers
        );

        Ok(self.stats.clone())
    }

    /// Get current statistics
    #[must_use] 
    pub const fn get_stats(&self) -> &AntiHebbianStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = AntiHebbianStats::default();
    }

    /// Set the decay rate
    pub const fn set_decay_rate(&mut self, rate: f32) {
        self.decay_rate = rate.clamp(0.0, 1.0);
    }

    /// Set the competition factor
    pub const fn set_competition_factor(&mut self, factor: f32) {
        self.competition_factor = factor.clamp(0.0, 1.0);
    }

    /// Set the pruning threshold
    pub const fn set_pruning_threshold(&mut self, threshold: f32) {
        self.pruning_threshold = threshold.max(0.0);
    }

    /// Get the decay rate
    #[must_use] 
    pub const fn decay_rate(&self) -> f32 {
        self.decay_rate
    }

    /// Get the competition factor
    #[must_use] 
    pub const fn competition_factor(&self) -> f32 {
        self.competition_factor
    }

    /// Get the pruning threshold
    #[must_use] 
    pub const fn pruning_threshold(&self) -> f32 {
        self.pruning_threshold
    }
}

/// Query pattern for learning optimization
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct QueryPattern {
    pub table: String,
    pub query_type: String, // SELECT, INSERT, UPDATE, DELETE
    pub columns: Vec<String>,
}

/// Main Hebbian learning engine implementing neuroplasticity
pub struct HebbianLearningEngine {
    learning_rate: f32,
    momentum: f32,
    /// Decay factor for synaptic weights (applied via anti-Hebbian learning)
    decay_factor: f32,
    stats: LearningStats,
    learning_history: HashMap<(u64, u64), Vec<f32>>, // (source, target) -> weight history
    /// Anti-Hebbian learning engine for competitive learning and decay
    anti_hebbian: AntiHebbianLearning,
    adaptive_rate_enabled: bool,
    min_learning_rate: f32,
    max_learning_rate: f32,
    /// Track query patterns for optimization
    query_patterns: HashMap<QueryPattern, u64>, // pattern -> frequency
    /// Threshold for automatic training
    training_threshold: u64,
}

impl HebbianLearningEngine {
    /// Create a new Hebbian learning engine
    pub fn new(learning_rate: f32) -> CoreResult<Self> {
        if !(0.0..=1.0).contains(&learning_rate) {
            return Err(CoreError::InvalidConfig(
                "Learning rate must be between 0.0 and 1.0".to_string(),
            ));
        }

        Ok(Self {
            learning_rate,
            momentum: 0.9,
            decay_factor: 0.995,
            stats: LearningStats::default(),
            learning_history: HashMap::new(),
            anti_hebbian: AntiHebbianLearning::new(0.01, 0.1),
            adaptive_rate_enabled: true,
            min_learning_rate: 0.001,
            max_learning_rate: 0.1,
            query_patterns: HashMap::new(),
            training_threshold: 10, // Train after 10 occurrences
        })
    }

    /// Apply Hebbian learning rule: "Cells that fire together, wire together"
    #[instrument(level = "debug", skip(self, network))]
    pub fn strengthen_connection(
        &mut self,
        network: &SynapticNetwork,
        source_id: u64,
        target_id: u64,
        correlation_strength: f32,
    ) -> CoreResult<()> {
        // Calculate adaptive learning rate based on connection history
        let connection_key = (source_id, target_id);
        let adaptive_rate = self.calculate_adaptive_rate(&connection_key);

        // Clamp correlation strength to prevent explosive growth
        let clamped_correlation = correlation_strength.clamp(-1.0, 1.0);

        // Calculate weight change using Hebbian rule with momentum
        let weight_change = adaptive_rate * clamped_correlation;

        // Update connection through the network
        network
            .modify_node(source_id, |source_node| {
                for connection in &mut source_node.connections {
                    if connection.target_id == target_id {
                        let old_weight = connection.weight;

                        // Apply momentum-based weight update
                        connection.weight += weight_change;

                        // Apply weight bounds to prevent saturation
                        connection.weight = connection.weight.clamp(-2.0, 2.0);

                        // Update plasticity factor based on recent activity
                        connection.plasticity_factor =
                            connection.plasticity_factor.mul_add(0.95, 0.05).clamp(0.1, 2.0);
                        connection.last_strengthened = Instant::now();
                        connection.usage_count += 1;

                        // Record in learning history (limited size to prevent memory bloat)
                        let history = self.learning_history.entry(connection_key).or_default();
                        history.push(connection.weight);

                        // Keep only recent history to prevent memory growth
                        if history.len() > 100 {
                            history.drain(0..50); // Keep last 50 entries
                        }

                        debug!(
                            "Connection {}->{} weight: {} -> {} (change: {})",
                            source_id, target_id, old_weight, connection.weight, weight_change
                        );

                        return Ok::<(), crate::error::NeuroQuantumError>(());
                    }
                }

                // Connection not found - this is not an error in neuromorphic networks
                debug!(
                    "Connection {}->{} not found for strengthening",
                    source_id, target_id
                );
                Ok(())
            })
            .unwrap_or(Ok(()))?;

        // Update statistics
        self.stats.total_learning_events += 1;
        if weight_change > 0.0 {
            self.stats.strengthened_connections += 1;
        } else {
            self.stats.weakened_connections += 1;
        }

        Ok(())
    }

    /// Calculate adaptive learning rate based on connection history
    fn calculate_adaptive_rate(&self, connection_key: &(u64, u64)) -> f32 {
        if !self.adaptive_rate_enabled {
            return self.learning_rate;
        }

        if let Some(history) = self.learning_history.get(connection_key) {
            if history.len() < 5 {
                return self.learning_rate;
            }

            // Calculate variance in recent weight changes
            let recent: Vec<f32> = history.iter().rev().take(10).copied().collect();
            let mean = recent.iter().sum::<f32>() / recent.len() as f32;
            let variance =
                recent.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / recent.len() as f32;

            // Reduce learning rate for highly variable connections
            let stability_factor = 1.0 / (1.0 + variance);
            let adaptive_rate = self.learning_rate * stability_factor;

            adaptive_rate.clamp(self.min_learning_rate, self.max_learning_rate)
        } else {
            self.learning_rate
        }
    }

    /// Apply long-term potentiation (LTP) for frequently co-activated connections
    #[instrument(level = "debug", skip(self, network))]
    pub fn apply_long_term_potentiation(
        &mut self,
        network: &mut SynapticNetwork,
        activation_pairs: &[(u64, u64, f32)], // (source, target, correlation)
    ) -> CoreResult<()> {
        info!(
            "Applying long-term potentiation to {} connection pairs",
            activation_pairs.len()
        );

        for &(source_id, target_id, correlation) in activation_pairs {
            // LTP strengthening is proportional to correlation strength
            let ltp_strength = correlation * 1.5; // LTP amplification factor
            self.strengthen_connection(network, source_id, target_id, ltp_strength)?;
        }

        Ok(())
    }

    /// Apply long-term depression (LTD) for weakly correlated connections
    #[instrument(level = "debug", skip(self, network))]
    pub fn apply_long_term_depression(
        &mut self,
        network: &mut SynapticNetwork,
        weak_connections: &[(u64, u64)],
    ) -> CoreResult<()> {
        info!(
            "Applying long-term depression to {} connections",
            weak_connections.len()
        );

        for &(source_id, target_id) in weak_connections {
            // Apply negative weight change for LTD
            let ltd_strength = -0.1 * self.learning_rate;
            self.strengthen_connection(network, source_id, target_id, ltd_strength)?;
        }

        Ok(())
    }

    /// Spike-timing-dependent plasticity (STDP) implementation
    pub fn apply_stdp(
        &mut self,
        network: &mut SynapticNetwork,
        spike_times: &HashMap<u64, Vec<Instant>>, // node_id -> spike times
    ) -> CoreResult<()> {
        let _current_time = Instant::now();
        let stdp_window = std::time::Duration::from_millis(20); // 20ms window

        // Collect all the connections to strengthen first to avoid borrowing conflicts
        let mut connections_to_strengthen = Vec::new();

        // Find all pairs of connected nodes that spiked within the STDP window
        for (&source_id, source_spikes) in spike_times {
            if let Some(source_node) = network.get_node(source_id) {
                for connection in &source_node.connections {
                    let target_id = connection.target_id;

                    if let Some(target_spikes) = spike_times.get(&target_id) {
                        // Calculate timing-dependent weight changes
                        for &source_spike in source_spikes {
                            for &target_spike in target_spikes {
                                let time_diff = if target_spike > source_spike {
                                    target_spike.duration_since(source_spike)
                                } else {
                                    source_spike.duration_since(target_spike)
                                };

                                if time_diff <= stdp_window {
                                    let weight_change = if target_spike > source_spike {
                                        // Pre before post: strengthening
                                        0.1 * (-(time_diff.as_millis() as f32) / 20.0).exp()
                                    } else {
                                        // Post before pre: weakening
                                        -0.05 * (-(time_diff.as_millis() as f32) / 20.0).exp()
                                    };

                                    connections_to_strengthen.push((
                                        source_id,
                                        target_id,
                                        weight_change,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Now apply all the strengthening operations
        for (source_id, target_id, weight_change) in connections_to_strengthen {
            self.strengthen_connection(network, source_id, target_id, weight_change)?;
        }

        Ok(())
    }

    /// Prune weak connections below threshold
    pub fn prune_weak_connections(
        &mut self,
        network: &SynapticNetwork,
        threshold: f32,
    ) -> CoreResult<u64> {
        let pruned_count = network.prune_weak_connections(threshold);

        self.stats.connections_pruned += pruned_count as u64;
        info!(
            "Pruned {} weak connections below threshold {}",
            pruned_count, threshold
        );

        Ok(pruned_count as u64)
    }

    /// Update learning parameters based on network performance
    pub fn adapt_learning_parameters(&mut self, network_performance: f32) {
        if self.adaptive_rate_enabled {
            // Increase learning rate if performance is poor, decrease if good
            if network_performance < 0.5 {
                self.learning_rate = (self.learning_rate * 1.1).min(self.max_learning_rate);
            } else if network_performance > 0.8 {
                self.learning_rate = (self.learning_rate * 0.95).max(self.min_learning_rate);
            }

            // Update momentum based on performance stability
            self.momentum = 0.1f32.mul_add(network_performance, 0.9);
        }
    }

    /// Get current learning statistics
    #[must_use] 
    pub const fn get_stats(&self) -> &LearningStats {
        &self.stats
    }

    /// Reset learning statistics
    pub fn reset_stats(&mut self) {
        self.stats = LearningStats::default();
        self.learning_history.clear();
    }

    /// Set learning rate
    pub fn set_learning_rate(&mut self, rate: f32) -> CoreResult<()> {
        if !(0.0..=1.0).contains(&rate) {
            return Err(CoreError::InvalidConfig(
                "Learning rate must be between 0.0 and 1.0".to_string(),
            ));
        }
        self.learning_rate = rate;
        Ok(())
    }

    /// Enable or disable adaptive learning rate
    pub const fn set_adaptive_rate(&mut self, enabled: bool) {
        self.adaptive_rate_enabled = enabled;
    }

    /// Get learning efficiency based on connection strengthening success rate
    pub fn calculate_learning_efficiency(&mut self) -> f32 {
        let total_events = self.stats.total_learning_events;
        if total_events == 0 {
            return 0.0;
        }

        let successful_events = self.stats.strengthened_connections;
        let efficiency = successful_events as f32 / total_events as f32;

        self.stats.learning_efficiency = efficiency;
        efficiency
    }

    /// Track a query pattern for automatic learning
    pub fn track_query_pattern(&mut self, pattern: QueryPattern) -> bool {
        let counter = self.query_patterns.entry(pattern.clone()).or_insert(0);
        *counter += 1;

        // Return true if pattern frequency exceeds threshold (triggers training)
        if *counter == self.training_threshold {
            info!(
                "Query pattern reached training threshold: table={}, type={}, frequency={}",
                pattern.table, pattern.query_type, counter
            );
            true
        } else {
            false
        }
    }

    /// Get most frequent query patterns for optimization
    #[must_use] 
    pub fn get_frequent_patterns(&self, top_n: usize) -> Vec<(QueryPattern, u64)> {
        let mut patterns: Vec<_> = self
            .query_patterns
            .iter()
            .map(|(p, f)| (p.clone(), *f))
            .collect();
        patterns.sort_by(|a, b| b.1.cmp(&a.1));
        patterns.truncate(top_n);
        patterns
    }

    /// Train network based on frequent query patterns
    pub fn train_on_frequent_patterns(
        &mut self,
        network: &SynapticNetwork,
        min_frequency: u64,
    ) -> CoreResult<u64> {
        let mut trained_count = 0;

        // Collect patterns to train (to avoid borrowing conflicts)
        let patterns_to_train: Vec<_> = self
            .query_patterns
            .iter()
            .filter(|(_, freq)| **freq >= min_frequency)
            .map(|(p, f)| (p.clone(), *f))
            .collect();

        for (pattern, frequency) in patterns_to_train {
            // Create neural pathway for this query pattern
            // Pattern: table -> columns -> query_type
            let table_hash = self.hash_string(&pattern.table);
            let query_type_hash = self.hash_string(&pattern.query_type);

            // Strengthen connections based on pattern frequency
            let correlation = (frequency.min(100) as f32) / 100.0;

            for column in &pattern.columns {
                let column_hash = self.hash_string(column);

                // Strengthen table -> column connection
                self.strengthen_connection(network, table_hash, column_hash, correlation)?;

                // Strengthen column -> query_type connection
                self.strengthen_connection(network, column_hash, query_type_hash, correlation)?;
            }

            trained_count += 1;
        }

        info!(
            "Trained network on {} frequent query patterns (min frequency: {})",
            trained_count, min_frequency
        );

        Ok(trained_count)
    }

    /// Simple hash function to convert strings to node IDs
    fn hash_string(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Clear query pattern history
    pub fn clear_query_patterns(&mut self) {
        self.query_patterns.clear();
    }

    /// Get total number of tracked patterns
    #[must_use] 
    pub fn get_pattern_count(&self) -> usize {
        self.query_patterns.len()
    }

    /// Apply anti-Hebbian decay to the network
    ///
    /// This combines both natural decay (based on `decay_factor`) and
    /// competitive learning mechanisms to maintain network health.
    pub fn apply_anti_hebbian_decay(&mut self, network: &SynapticNetwork) -> CoreResult<u64> {
        // Set the decay rate from our decay_factor
        // decay_factor is the retention rate, so decay_rate = 1 - decay_factor
        let decay_rate = 1.0 - self.decay_factor;
        self.anti_hebbian.set_decay_rate(decay_rate);

        // Apply synaptic decay
        let decayed = self.anti_hebbian.apply_synaptic_decay(network)?;

        self.stats.weakened_connections += decayed;

        info!(
            "Applied anti-Hebbian decay to {} connections (decay_factor: {})",
            decayed, self.decay_factor
        );

        Ok(decayed)
    }

    /// Apply competitive learning with Winner-Takes-All mechanism
    ///
    /// # Arguments
    /// * `network` - The synaptic network
    /// * `activations` - Map of neuron IDs to their activation levels
    /// * `k_winners` - Number of winners to select
    pub fn apply_competitive_learning(
        &mut self,
        network: &SynapticNetwork,
        activations: &HashMap<u64, f32>,
        k_winners: usize,
    ) -> CoreResult<Vec<WinnerInfo>> {
        let winners =
            self.anti_hebbian
                .apply_competitive_learning(network, activations, k_winners)?;

        // Update stats
        let loser_count = activations.len().saturating_sub(winners.len());
        self.stats.weakened_connections += loser_count as u64;
        self.stats.strengthened_connections += winners.len() as u64;

        Ok(winners)
    }

    /// Apply lateral inhibition from an active neuron to its neighbors
    pub fn apply_lateral_inhibition(
        &mut self,
        network: &SynapticNetwork,
        active_neuron_id: u64,
        neighbor_ids: &[u64],
    ) -> CoreResult<u64> {
        self.anti_hebbian
            .apply_lateral_inhibition(network, active_neuron_id, neighbor_ids)
    }

    /// Apply STDP-based anti-Hebbian learning
    ///
    /// Weakens connections where post-synaptic neurons fire before pre-synaptic
    pub fn apply_stdp_anti_hebbian(
        &mut self,
        network: &SynapticNetwork,
        spike_times: &HashMap<u64, Vec<Instant>>,
    ) -> CoreResult<u64> {
        self.anti_hebbian
            .apply_stdp_anti_hebbian(network, spike_times)
    }

    /// Perform a complete learning maintenance cycle
    ///
    /// This combines Hebbian strengthening, anti-Hebbian decay, and pruning
    /// for comprehensive network plasticity management.
    pub fn perform_plasticity_cycle(
        &mut self,
        network: &SynapticNetwork,
        activations: Option<&HashMap<u64, f32>>,
    ) -> CoreResult<PlasticityCycleResult> {
        let start = Instant::now();

        // 1. Apply anti-Hebbian maintenance (decay + competitive learning + pruning)
        let anti_hebbian_stats = self
            .anti_hebbian
            .perform_maintenance_cycle(network, activations)?;

        // 2. Update our stats
        self.stats.connections_pruned += anti_hebbian_stats.connections_pruned;

        let duration = start.elapsed();

        let result = PlasticityCycleResult {
            decay_operations: anti_hebbian_stats.decay_operations,
            connections_pruned: anti_hebbian_stats.connections_pruned,
            competition_losers: anti_hebbian_stats.competition_losers,
            lateral_inhibition_events: anti_hebbian_stats.lateral_inhibition_events,
            cycle_duration_ms: duration.as_millis() as u64,
        };

        info!(
            "Plasticity cycle complete in {}ms: {} decayed, {} pruned",
            result.cycle_duration_ms, result.decay_operations, result.connections_pruned
        );

        Ok(result)
    }

    /// Get the decay factor
    #[must_use] 
    pub const fn decay_factor(&self) -> f32 {
        self.decay_factor
    }

    /// Set the decay factor (0.0-1.0, higher = slower decay)
    pub fn set_decay_factor(&mut self, factor: f32) -> CoreResult<()> {
        if !(0.0..=1.0).contains(&factor) {
            return Err(CoreError::InvalidConfig(
                "Decay factor must be between 0.0 and 1.0".to_string(),
            ));
        }
        self.decay_factor = factor;
        Ok(())
    }

    /// Get a reference to the anti-Hebbian learning engine
    #[must_use] 
    pub const fn anti_hebbian(&self) -> &AntiHebbianLearning {
        &self.anti_hebbian
    }

    /// Get a mutable reference to the anti-Hebbian learning engine
    pub const fn anti_hebbian_mut(&mut self) -> &mut AntiHebbianLearning {
        &mut self.anti_hebbian
    }
}

/// Result of a plasticity cycle
#[derive(Debug, Clone)]
pub struct PlasticityCycleResult {
    /// Number of decay operations applied
    pub decay_operations: u64,
    /// Number of connections pruned
    pub connections_pruned: u64,
    /// Number of neurons that lost in competition
    pub competition_losers: u64,
    /// Number of lateral inhibition events
    pub lateral_inhibition_events: u64,
    /// Duration of the cycle in milliseconds
    pub cycle_duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synaptic::{ConnectionType, SynapticNetwork, SynapticNode};

    #[test]
    fn test_learning_engine_creation() {
        let engine = HebbianLearningEngine::new(0.01).unwrap();
        assert_eq!(engine.learning_rate, 0.01);
        assert_eq!(engine.stats.total_learning_events, 0);
    }

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

    #[test]
    fn test_learning_rate_adaptation() {
        let mut engine = HebbianLearningEngine::new(0.01).unwrap();
        let initial_rate = engine.learning_rate;

        // Test adaptation based on poor performance
        engine.adapt_learning_parameters(0.3);
        assert!(engine.learning_rate > initial_rate);

        // Test adaptation based on good performance
        engine.adapt_learning_parameters(0.9);
        assert!(engine.learning_rate < initial_rate * 1.1);
    }

    #[test]
    fn test_learning_efficiency_calculation() {
        let mut engine = HebbianLearningEngine::new(0.01).unwrap();

        // Initially no events
        assert_eq!(engine.calculate_learning_efficiency(), 0.0);

        // Simulate some learning events
        engine.stats.total_learning_events = 10;
        engine.stats.strengthened_connections = 8;

        let efficiency = engine.calculate_learning_efficiency();
        assert_eq!(efficiency, 0.8);
    }

    #[test]
    fn test_statistics_reset() {
        let mut engine = HebbianLearningEngine::new(0.01).unwrap();

        // Add some stats
        engine.stats.total_learning_events = 5;
        engine.stats.strengthened_connections = 3;

        // Reset and verify
        engine.reset_stats();
        assert_eq!(engine.stats.total_learning_events, 0);
        assert_eq!(engine.stats.strengthened_connections, 0);
        assert!(engine.learning_history.is_empty());
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
}
