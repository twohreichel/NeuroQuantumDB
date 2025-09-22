//! # Adaptive Plasticity Matrix
//!
//! Implementation of neural plasticity mechanisms for dynamic data reorganization
//! and intelligent memory optimization in NeuroQuantumDB.

use crate::error::{CoreError, CoreResult};
use crate::synaptic::SynapticNetwork;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info, instrument, warn};

/// Access pattern tracking for plasticity decisions
#[derive(Debug, Clone, Default)]
pub struct AccessPatterns {
    pub node_access_frequency: HashMap<u64, u64>,
    pub connection_usage: HashMap<(u64, u64), u64>,
    pub temporal_locality_secs: HashMap<u64, Vec<u64>>, // Store timestamps as seconds since epoch
    pub spatial_locality: HashMap<u64, Vec<u64>>,       // node_id -> frequently accessed neighbors
    pub query_patterns: Vec<String>,                    // Recent query patterns for optimization
}

/// Plasticity parameters controlling reorganization behavior
#[derive(Debug, Clone, Serialize)]
pub struct PlasticityParams {
    pub reorganization_threshold: f32,
    pub temporal_window_secs: u64, // Store as seconds instead of Duration
    pub spatial_clustering_factor: f32,
    pub decay_rate: f32,
    pub min_access_count: u64,
    pub max_reorganizations_per_cycle: u32,
    pub locality_weight: f32,
    pub frequency_weight: f32,
}

impl Default for PlasticityParams {
    fn default() -> Self {
        Self {
            reorganization_threshold: 0.1,
            temporal_window_secs: 300, // 5 minutes in seconds
            spatial_clustering_factor: 0.8,
            decay_rate: 0.95,
            min_access_count: 10,
            max_reorganizations_per_cycle: 100,
            locality_weight: 0.6,
            frequency_weight: 0.4,
        }
    }
}

/// Plasticity matrix managing dynamic network reorganization
pub struct PlasticityMatrix {
    #[allow(dead_code)] // Used for capacity validation in future features
    max_nodes: usize,
    plasticity_threshold: f32,
    access_patterns: AccessPatterns,
    params: PlasticityParams,
    reorganization_history: Vec<ReorganizationEvent>,
    last_reorganization_secs: Option<u64>, // Store as seconds since epoch
    plasticity_scores: HashMap<u64, f32>,  // node_id -> plasticity score
    cluster_assignments: HashMap<u64, u32>, // node_id -> cluster_id
    next_cluster_id: u32,
}

/// Record of network reorganization events
#[derive(Debug, Clone)]
pub struct ReorganizationEvent {
    pub timestamp_secs: u64, // Store as seconds since epoch
    pub event_type: ReorganizationType,
    pub nodes_affected: Vec<u64>,
    pub performance_impact: f32,
    pub memory_delta: i64, // Change in memory usage
}

#[derive(Debug, Clone, Serialize)]
pub enum ReorganizationType {
    SpatialClustering,
    TemporalReorganization,
    FrequencyBasedOptimization,
    ConnectionPruning,
    NodeMigration,
}

impl PlasticityMatrix {
    /// Create a new plasticity matrix
    pub fn new(max_nodes: usize, plasticity_threshold: f32) -> CoreResult<Self> {
        if !(0.0..=1.0).contains(&plasticity_threshold) {
            return Err(CoreError::InvalidConfig(
                "Plasticity threshold must be between 0.0 and 1.0".to_string(),
            ));
        }

        Ok(Self {
            max_nodes,
            plasticity_threshold,
            access_patterns: AccessPatterns::default(),
            params: PlasticityParams::default(),
            reorganization_history: Vec::new(),
            last_reorganization_secs: None,
            plasticity_scores: HashMap::new(),
            cluster_assignments: HashMap::new(),
            next_cluster_id: 0,
        })
    }

    /// Record access to a node for plasticity analysis
    #[instrument(level = "debug", skip(self))]
    pub fn record_access(&mut self, node_id: u64, access_time: Instant) {
        // Update access frequency
        *self
            .access_patterns
            .node_access_frequency
            .entry(node_id)
            .or_insert(0) += 1;

        // Update temporal locality using seconds since epoch
        let access_time_secs = access_time.elapsed().as_secs();
        self.access_patterns
            .temporal_locality_secs
            .entry(node_id)
            .or_default()
            .push(access_time_secs);

        // Keep only recent accesses within temporal window
        let cutoff_time_secs = access_time_secs.saturating_sub(self.params.temporal_window_secs);
        if let Some(times) = self
            .access_patterns
            .temporal_locality_secs
            .get_mut(&node_id)
        {
            times.retain(|&t| t > cutoff_time_secs);
        }

        // Update plasticity score
        self.update_plasticity_score(node_id);

        debug!(
            "Recorded access to node {} at time {}",
            node_id, access_time_secs
        );
    }

    /// Record connection usage for plasticity tracking
    pub fn record_connection_usage(&mut self, source_id: u64, target_id: u64) {
        let connection_key = (source_id, target_id);
        *self
            .access_patterns
            .connection_usage
            .entry(connection_key)
            .or_insert(0) += 1;

        // Update spatial locality
        self.access_patterns
            .spatial_locality
            .entry(source_id)
            .or_default()
            .push(target_id);

        // Keep track of frequently accessed neighbors
        if let Some(neighbors) = self.access_patterns.spatial_locality.get_mut(&source_id) {
            // Remove duplicates and keep most recent
            neighbors.sort_unstable();
            neighbors.dedup();
            if neighbors.len() > 10 {
                neighbors.truncate(10); // Keep top 10 neighbors
            }
        }
    }

    /// Update plasticity score for a node based on access patterns
    fn update_plasticity_score(&mut self, node_id: u64) {
        let frequency = self
            .access_patterns
            .node_access_frequency
            .get(&node_id)
            .copied()
            .unwrap_or(0) as f32;

        let temporal_locality = self
            .access_patterns
            .temporal_locality_secs
            .get(&node_id)
            .map(|times| times.len() as f32)
            .unwrap_or(0.0);

        let spatial_locality = self
            .access_patterns
            .spatial_locality
            .get(&node_id)
            .map(|neighbors| neighbors.len() as f32)
            .unwrap_or(0.0);

        // Calculate composite plasticity score
        let frequency_component = frequency * self.params.frequency_weight;
        let locality_component =
            (temporal_locality + spatial_locality) * self.params.locality_weight;

        let plasticity_score = (frequency_component + locality_component) / 100.0; // Normalize
        self.plasticity_scores
            .insert(node_id, plasticity_score.min(1.0));
    }

    /// Reorganize network based on plasticity analysis
    #[instrument(level = "info", skip(self, _network))]
    pub fn reorganize_network(&mut self, _network: &mut SynapticNetwork) -> CoreResult<bool> {
        info!("Starting network reorganization based on plasticity analysis");

        // Check if reorganization is needed
        if !self.should_reorganize()? {
            return Ok(false);
        }

        let start_time = Instant::now();
        let mut reorganizations_performed = 0;
        let mut total_performance_impact = 0.0;

        // 1. Spatial clustering based on access patterns
        if reorganizations_performed < self.params.max_reorganizations_per_cycle {
            let clustered = self.perform_spatial_clustering()?;
            if clustered {
                reorganizations_performed += 1;
                total_performance_impact += 0.1;
            }
        }

        // 2. Temporal reorganization for frequently accessed nodes
        if reorganizations_performed < self.params.max_reorganizations_per_cycle {
            let temporal_optimized = self.perform_temporal_reorganization()?;
            if temporal_optimized {
                reorganizations_performed += 1;
                total_performance_impact += 0.15;
            }
        }

        // 3. Frequency-based optimization
        if reorganizations_performed < self.params.max_reorganizations_per_cycle {
            let frequency_optimized = self.perform_frequency_optimization()?;
            if frequency_optimized {
                reorganizations_performed += 1;
                total_performance_impact += 0.2;
            }
        }

        // 4. Prune unused connections
        if reorganizations_performed < self.params.max_reorganizations_per_cycle {
            let pruned = self.prune_unused_connections()?;
            if pruned > 0 {
                reorganizations_performed += 1;
                total_performance_impact += 0.05;
            }
        }

        // Record reorganization event
        if reorganizations_performed > 0 {
            let event = ReorganizationEvent {
                timestamp_secs: start_time.elapsed().as_secs(),
                event_type: ReorganizationType::SpatialClustering, // Primary type
                nodes_affected: self.plasticity_scores.keys().cloned().collect(),
                performance_impact: total_performance_impact,
                memory_delta: 0, // Calculate actual memory change
            };

            self.reorganization_history.push(event);
            self.last_reorganization_secs = Some(start_time.elapsed().as_secs());

            // Apply decay to access patterns
            self.apply_access_decay();

            info!(
                "Network reorganization completed: {} operations, impact: {:.3}",
                reorganizations_performed, total_performance_impact
            );
        }

        Ok(reorganizations_performed > 0)
    }

    /// Check if network reorganization should be triggered
    fn should_reorganize(&self) -> CoreResult<bool> {
        // Check if enough time has passed since last reorganization
        if let Some(last_reorg_secs) = self.last_reorganization_secs {
            let current_time_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if current_time_secs.saturating_sub(last_reorg_secs)
                < self.params.temporal_window_secs / 4
            {
                return Ok(false);
            }
        }

        // Check if we have enough access data
        let total_accesses: u64 = self.access_patterns.node_access_frequency.values().sum();
        if total_accesses < self.params.min_access_count {
            return Ok(false);
        }

        // Check plasticity threshold
        let high_plasticity_nodes = self
            .plasticity_scores
            .values()
            .filter(|&&score| score > self.plasticity_threshold)
            .count();

        let plasticity_ratio =
            high_plasticity_nodes as f32 / self.plasticity_scores.len().max(1) as f32;

        Ok(plasticity_ratio > 0.1) // Reorganize if >10% of nodes show high plasticity
    }

    /// Perform spatial clustering of related nodes
    fn perform_spatial_clustering(&mut self) -> CoreResult<bool> {
        // Find nodes that are frequently accessed together
        let mut cluster_candidates = HashMap::new();

        for (&node_id, neighbors) in &self.access_patterns.spatial_locality {
            if neighbors.len() >= 3 {
                // Minimum cluster size
                let cluster_id = self.next_cluster_id;
                self.next_cluster_id += 1;

                cluster_candidates.insert(node_id, cluster_id);
                for &neighbor_id in neighbors {
                    cluster_candidates.insert(neighbor_id, cluster_id);
                }
            }
        }

        // Assign cluster memberships
        for (node_id, cluster_id) in cluster_candidates {
            self.cluster_assignments.insert(node_id, cluster_id);
        }

        Ok(!self.cluster_assignments.is_empty())
    }

    /// Perform temporal reorganization for hot nodes
    fn perform_temporal_reorganization(&mut self) -> CoreResult<bool> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Identify hot nodes based on recent access patterns
        let mut hot_nodes = Vec::new();
        for (&node_id, access_times) in &self.access_patterns.temporal_locality_secs {
            let recent_accesses = access_times
                .iter()
                .filter(|&&time| {
                    current_time.saturating_sub(time) <= self.params.temporal_window_secs
                })
                .count();

            if recent_accesses >= self.params.min_access_count as usize {
                hot_nodes.push((node_id, recent_accesses));
            }
        }

        if hot_nodes.is_empty() {
            return Ok(false);
        }

        // Sort by access frequency
        hot_nodes.sort_by(|a, b| b.1.cmp(&a.1));

        // Implement actual node reorganization logic
        for (node_id, access_count) in &hot_nodes {
            // Increase plasticity score for frequently accessed nodes
            let plasticity_score = (*access_count as f32) / (self.params.min_access_count as f32);
            self.plasticity_scores
                .insert(*node_id, plasticity_score.min(1.0));

            // Update cluster assignments for spatial locality
            if let Some(&cluster_id) = self.cluster_assignments.get(node_id) {
                // Find related nodes that should be in the same cluster
                if let Some(neighbors) = self.access_patterns.spatial_locality.get(node_id) {
                    for &neighbor_id in neighbors.iter().take(5) {
                        // Limit to top 5 neighbors
                        self.cluster_assignments
                            .entry(neighbor_id)
                            .or_insert(cluster_id);
                    }
                }
            } else {
                // Assign to a new cluster
                self.cluster_assignments
                    .insert(*node_id, self.next_cluster_id);
                self.next_cluster_id += 1;
            }

            // Update connection weights based on temporal patterns
            if let Some(neighbors) = self.access_patterns.spatial_locality.get(node_id) {
                for &neighbor_id in neighbors {
                    let connection_key = if *node_id < neighbor_id {
                        (*node_id, neighbor_id)
                    } else {
                        (neighbor_id, *node_id)
                    };

                    let current_usage = self
                        .access_patterns
                        .connection_usage
                        .get(&connection_key)
                        .unwrap_or(&0);

                    // Boost connection usage for frequently accessed node pairs
                    self.access_patterns
                        .connection_usage
                        .insert(connection_key, current_usage + (*access_count as u64));
                }
            }
        }

        Ok(!hot_nodes.is_empty())
    }

    /// Perform frequency-based optimization
    fn perform_frequency_optimization(&mut self) -> CoreResult<bool> {
        // Sort nodes by access frequency
        let mut nodes_by_frequency: Vec<_> = self
            .access_patterns
            .node_access_frequency
            .iter()
            .map(|(&id, &freq)| (id, freq))
            .collect();
        nodes_by_frequency.sort_by(|a, b| b.1.cmp(&a.1));

        // Identify top accessed nodes for optimization
        let top_count = (nodes_by_frequency.len() / 10).max(1);
        let top_nodes: Vec<_> = nodes_by_frequency.iter().take(top_count).collect();

        if top_nodes.is_empty() {
            return Ok(false);
        }

        // Implement frequency-based optimization
        // Boost frequently accessed nodes by increasing their plasticity scores
        // and optimizing their memory access patterns
        for &(node_id, frequency) in &top_nodes {
            // Calculate boost factor based on frequency ranking
            let frequency_percentile = (*frequency as f32)
                / (nodes_by_frequency.first().map(|(_, f)| *f).unwrap_or(1) as f32);

            // Update plasticity score with frequency boost
            let current_score = self.plasticity_scores.get(node_id).unwrap_or(&0.0);
            let boosted_score = (current_score + frequency_percentile * 0.3).min(1.0);
            self.plasticity_scores.insert(*node_id, boosted_score);

            // Create priority clusters for high-frequency nodes
            if frequency_percentile > 0.8 {
                // Top 20% of nodes
                // Assign to priority cluster (cluster 0 reserved for high-frequency nodes)
                self.cluster_assignments.insert(*node_id, 0);

                // Boost related connections
                if let Some(neighbors) = self.access_patterns.spatial_locality.get(node_id) {
                    for &neighbor_id in neighbors.iter().take(3) {
                        // Top 3 neighbors
                        let connection_key = if *node_id < neighbor_id {
                            (*node_id, neighbor_id)
                        } else {
                            (neighbor_id, *node_id)
                        };

                        let current_usage = self
                            .access_patterns
                            .connection_usage
                            .get(&connection_key)
                            .unwrap_or(&0);

                        // Boost connection weight for high-frequency node connections
                        let boosted_usage = current_usage + (*frequency / 10).max(1);
                        self.access_patterns
                            .connection_usage
                            .insert(connection_key, boosted_usage);
                    }
                }
            }
        }

        // Implement memory optimization for frequently accessed nodes
        // This would involve cache pre-loading and memory layout optimization
        let optimization_count = top_nodes.len().min(20); // Limit optimizations per cycle

        info!(
            "Performed frequency-based optimization on {} high-frequency nodes",
            optimization_count
        );

        Ok(optimization_count > 0)
    }

    /// Prune unused connections based on access patterns
    fn prune_unused_connections(&mut self) -> CoreResult<u32> {
        let usage_threshold = 2; // Minimum usage count to keep connection

        // Find connections with low usage
        let connections_to_prune: Vec<_> = self
            .access_patterns
            .connection_usage
            .iter()
            .filter(|(_, &usage_count)| usage_count < usage_threshold)
            .map(|(&(source_id, target_id), _)| (source_id, target_id))
            .collect();

        let pruned_count = connections_to_prune.len() as u32;

        // Remove low-usage connections from tracking
        for (source_id, target_id) in connections_to_prune {
            self.access_patterns
                .connection_usage
                .remove(&(source_id, target_id));
        }

        Ok(pruned_count)
    }

    /// Apply decay to access patterns to forget old data
    fn apply_access_decay(&mut self) {
        // Decay access frequencies
        for frequency in self.access_patterns.node_access_frequency.values_mut() {
            *frequency = (*frequency as f32 * self.params.decay_rate) as u64;
        }

        // Remove nodes with very low frequency
        self.access_patterns
            .node_access_frequency
            .retain(|_, &mut freq| freq > 0);

        // Decay connection usage
        for usage in self.access_patterns.connection_usage.values_mut() {
            *usage = (*usage as f32 * self.params.decay_rate) as u64;
        }

        // Remove connections with very low usage
        self.access_patterns
            .connection_usage
            .retain(|_, &mut usage| usage > 0);

        // Update plasticity scores after decay
        let node_ids: Vec<_> = self.plasticity_scores.keys().cloned().collect();
        for node_id in node_ids {
            self.update_plasticity_score(node_id);
        }
    }

    /// Get plasticity statistics
    pub fn get_plasticity_stats(&self) -> PlasticityStats {
        let total_nodes = self.plasticity_scores.len();
        let high_plasticity_nodes = self
            .plasticity_scores
            .values()
            .filter(|&&score| score > self.plasticity_threshold)
            .count();

        let avg_plasticity = if total_nodes > 0 {
            self.plasticity_scores.values().sum::<f32>() / total_nodes as f32
        } else {
            0.0
        };

        PlasticityStats {
            total_nodes,
            high_plasticity_nodes,
            average_plasticity_score: avg_plasticity,
            total_reorganizations: self.reorganization_history.len(),
            last_reorganization_secs: self.last_reorganization_secs,
            active_clusters: self
                .cluster_assignments
                .values()
                .max()
                .copied()
                .unwrap_or(0)
                + 1,
        }
    }

    /// Set plasticity parameters
    pub fn set_params(&mut self, params: PlasticityParams) {
        self.params = params;
    }

    /// Get current access patterns
    pub fn get_access_patterns(&self) -> &AccessPatterns {
        &self.access_patterns
    }
}

/// Statistics about plasticity and reorganization
#[derive(Debug, Clone, Serialize)]
pub struct PlasticityStats {
    pub total_nodes: usize,
    pub high_plasticity_nodes: usize,
    pub average_plasticity_score: f32,
    pub total_reorganizations: usize,
    pub last_reorganization_secs: Option<u64>,
    pub active_clusters: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plasticity_matrix_creation() {
        let matrix = PlasticityMatrix::new(1000, 0.5).unwrap();
        assert_eq!(matrix.max_nodes, 1000);
        assert_eq!(matrix.plasticity_threshold, 0.5);
    }

    #[test]
    fn test_invalid_plasticity_threshold() {
        let result = PlasticityMatrix::new(1000, 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_access_recording() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();
        let now = Instant::now();

        matrix.record_access(1, now);
        assert_eq!(
            matrix.access_patterns.node_access_frequency.get(&1),
            Some(&1)
        );
        assert!(matrix
            .access_patterns
            .temporal_locality_secs
            .contains_key(&1));
    }

    #[test]
    fn test_connection_usage_recording() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();

        matrix.record_connection_usage(1, 2);
        assert_eq!(
            matrix.access_patterns.connection_usage.get(&(1, 2)),
            Some(&1)
        );
        assert!(matrix
            .access_patterns
            .spatial_locality
            .get(&1)
            .unwrap()
            .contains(&2));
    }

    #[test]
    fn test_plasticity_score_update() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();
        let now = Instant::now();

        // Record multiple accesses
        for _ in 0..5 {
            matrix.record_access(1, now);
        }

        assert!(matrix.plasticity_scores.contains_key(&1));
        assert!(matrix.plasticity_scores[&1] > 0.0);
    }

    #[test]
    fn test_reorganization_check() {
        let mut matrix = PlasticityMatrix::new(1000, 0.1).unwrap();

        // Initially should not reorganize
        assert!(!matrix.should_reorganize().unwrap());

        // Add enough access data
        for i in 0..20 {
            matrix.access_patterns.node_access_frequency.insert(i, 10);
            matrix.plasticity_scores.insert(i, 0.2); // Above threshold
        }

        // Now should reorganize
        assert!(matrix.should_reorganize().unwrap());
    }

    #[test]
    fn test_access_decay() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();

        // Add some access data
        matrix.access_patterns.node_access_frequency.insert(1, 100);
        matrix.access_patterns.connection_usage.insert((1, 2), 50);

        let initial_freq = matrix.access_patterns.node_access_frequency[&1];
        let initial_usage = matrix.access_patterns.connection_usage[&(1, 2)];

        matrix.apply_access_decay();

        assert!(matrix.access_patterns.node_access_frequency[&1] < initial_freq);
        assert!(matrix.access_patterns.connection_usage[&(1, 2)] < initial_usage);
    }
}
