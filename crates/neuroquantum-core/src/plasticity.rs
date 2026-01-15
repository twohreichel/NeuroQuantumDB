//! # Adaptive Plasticity Matrix
//!
//! Implementation of neural plasticity mechanisms for dynamic data reorganization
//! and intelligent memory optimization in `NeuroQuantumDB`.

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

/// Capacity management configuration
#[derive(Debug, Clone, Serialize)]
pub struct CapacityConfig {
    /// Threshold percentage (0.0-1.0) at which consolidation is triggered
    pub consolidation_threshold: f32,
    /// Threshold percentage (0.0-1.0) at which warnings are emitted
    pub warning_threshold: f32,
    /// Maximum nodes that can be consolidated in one cycle
    pub max_consolidation_batch: usize,
    /// Minimum plasticity score for a node to be considered for consolidation
    pub min_consolidation_plasticity: f32,
}

impl Default for CapacityConfig {
    fn default() -> Self {
        Self {
            consolidation_threshold: 0.90,     // Trigger at 90% capacity
            warning_threshold: 0.80,           // Warn at 80% capacity
            max_consolidation_batch: 100,      // Max 100 nodes per consolidation cycle
            min_consolidation_plasticity: 0.1, // Only consolidate low-plasticity nodes
        }
    }
}

/// Result of a capacity check operation
#[derive(Debug, Clone, Serialize)]
pub struct CapacityCheckResult {
    /// Current node count in the network
    pub current_nodes: usize,
    /// Maximum allowed nodes
    pub max_nodes: usize,
    /// Current capacity utilization (0.0-1.0)
    pub utilization: f32,
    /// Whether consolidation is needed
    pub needs_consolidation: bool,
    /// Whether we're at warning level
    pub at_warning_level: bool,
    /// Number of nodes that would need to be consolidated
    pub consolidation_candidates: usize,
}

/// Result of a consolidation operation
#[derive(Debug, Clone, Serialize)]
pub struct ConsolidationResult {
    /// Number of nodes consolidated (merged or removed)
    pub nodes_consolidated: usize,
    /// Number of connections pruned during consolidation
    pub connections_pruned: usize,
    /// Memory freed in bytes (estimated)
    pub memory_freed_bytes: i64,
    /// New capacity utilization after consolidation
    pub new_utilization: f32,
    /// Nodes that were merged into others
    pub merged_nodes: Vec<(u64, u64)>, // (source_node, target_node)
    /// Nodes that were removed entirely
    pub removed_nodes: Vec<u64>,
}

/// Plasticity matrix managing dynamic network reorganization
pub struct PlasticityMatrix {
    max_nodes: usize,
    plasticity_threshold: f32,
    access_patterns: AccessPatterns,
    params: PlasticityParams,
    reorganization_history: Vec<ReorganizationEvent>,
    last_reorganization_secs: Option<u64>, // Store as seconds since epoch
    plasticity_scores: HashMap<u64, f32>,  // node_id -> plasticity score
    cluster_assignments: HashMap<u64, u32>, // node_id -> cluster_id
    next_cluster_id: u32,
    capacity_config: CapacityConfig,
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
            capacity_config: CapacityConfig::default(),
        })
    }

    /// Create a new plasticity matrix with custom capacity configuration
    pub fn with_capacity_config(
        max_nodes: usize,
        plasticity_threshold: f32,
        capacity_config: CapacityConfig,
    ) -> CoreResult<Self> {
        if !(0.0..=1.0).contains(&plasticity_threshold) {
            return Err(CoreError::InvalidConfig(
                "Plasticity threshold must be between 0.0 and 1.0".to_string(),
            ));
        }

        if capacity_config.consolidation_threshold < capacity_config.warning_threshold {
            return Err(CoreError::InvalidConfig(
                "Consolidation threshold must be >= warning threshold".to_string(),
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
            capacity_config,
        })
    }

    /// Get the maximum nodes capacity
    #[must_use]
    pub const fn max_nodes(&self) -> usize {
        self.max_nodes
    }

    /// Set the capacity configuration
    pub const fn set_capacity_config(&mut self, config: CapacityConfig) {
        self.capacity_config = config;
    }

    /// Get the current capacity configuration
    #[must_use]
    pub const fn capacity_config(&self) -> &CapacityConfig {
        &self.capacity_config
    }

    /// Check capacity and trigger consolidation if needed
    ///
    /// This method monitors the network's node count against the configured
    /// maximum capacity and triggers consolidation when the utilization
    /// exceeds the configured threshold.
    ///
    /// # Arguments
    /// * `network` - Reference to the synaptic network to check
    ///
    /// # Returns
    /// * `Ok(true)` - Consolidation was performed
    /// * `Ok(false)` - No consolidation needed
    /// * `Err(_)` - An error occurred during capacity check or consolidation
    #[instrument(level = "info", skip(self, network))]
    pub fn check_and_reorganize(&mut self, network: &SynapticNetwork) -> CoreResult<bool> {
        let capacity_result = self.check_capacity(network)?;

        if capacity_result.at_warning_level {
            warn!(
                "Network capacity at warning level: {:.1}% ({}/{} nodes)",
                capacity_result.utilization * 100.0,
                capacity_result.current_nodes,
                capacity_result.max_nodes
            );
        }

        if capacity_result.needs_consolidation {
            info!(
                "Triggering consolidation: {:.1}% capacity, {} candidates",
                capacity_result.utilization * 100.0,
                capacity_result.consolidation_candidates
            );

            let consolidation_result = self.trigger_consolidation(network)?;

            info!(
                "Consolidation complete: {} nodes consolidated, {} connections pruned, new utilization: {:.1}%",
                consolidation_result.nodes_consolidated,
                consolidation_result.connections_pruned,
                consolidation_result.new_utilization * 100.0
            );

            return Ok(true);
        }

        Ok(false)
    }

    /// Check the current capacity utilization of the network
    ///
    /// # Arguments
    /// * `network` - Reference to the synaptic network to check
    ///
    /// # Returns
    /// Detailed capacity check result including utilization metrics
    pub fn check_capacity(&self, network: &SynapticNetwork) -> CoreResult<CapacityCheckResult> {
        let current_nodes = network.node_count();
        let utilization = current_nodes as f32 / self.max_nodes as f32;

        let at_warning_level = utilization >= self.capacity_config.warning_threshold;
        let needs_consolidation = utilization >= self.capacity_config.consolidation_threshold;

        // Count consolidation candidates (low-plasticity nodes that can be merged/removed)
        let consolidation_candidates = self
            .plasticity_scores
            .values()
            .filter(|&&score| score < self.capacity_config.min_consolidation_plasticity)
            .count();

        Ok(CapacityCheckResult {
            current_nodes,
            max_nodes: self.max_nodes,
            utilization,
            needs_consolidation,
            at_warning_level,
            consolidation_candidates,
        })
    }

    /// Trigger network consolidation to free up capacity
    ///
    /// This method implements neuroplasticity-inspired consolidation where
    /// low-activity nodes are merged or removed to make room for new data.
    /// This mimics the brain's synaptic pruning process where unused
    /// connections are eliminated to optimize neural efficiency.
    ///
    /// # Arguments
    /// * `network` - Reference to the synaptic network to consolidate
    ///
    /// # Returns
    /// Detailed result of the consolidation operation
    #[instrument(level = "info", skip(self, network))]
    pub fn trigger_consolidation(
        &mut self,
        network: &SynapticNetwork,
    ) -> CoreResult<ConsolidationResult> {
        let start_nodes = network.node_count();
        let mut nodes_consolidated = 0;
        let mut connections_pruned = 0;
        let mut merged_nodes = Vec::new();
        let mut removed_nodes = Vec::new();

        // Phase 1: Identify low-plasticity nodes for consolidation
        let mut candidates: Vec<_> = self
            .plasticity_scores
            .iter()
            .filter(|(_, &score)| score < self.capacity_config.min_consolidation_plasticity)
            .map(|(&node_id, &score)| (node_id, score))
            .collect();

        // Sort by plasticity score (lowest first - most eligible for consolidation)
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to max batch size
        candidates.truncate(self.capacity_config.max_consolidation_batch);

        // Phase 2: Process each candidate
        for (node_id, _score) in candidates {
            // Check if this node has a cluster assignment
            if let Some(&cluster_id) = self.cluster_assignments.get(&node_id) {
                // Find the best target node in the same cluster to merge into
                let merge_target = self.find_merge_target(node_id, cluster_id);

                if let Some(target_id) = merge_target {
                    // Merge node into target
                    self.merge_node_data(node_id, target_id);
                    merged_nodes.push((node_id, target_id));
                    nodes_consolidated += 1;

                    // Prune connections to the merged node
                    let pruned = self.prune_node_connections(node_id);
                    connections_pruned += pruned;
                } else {
                    // No suitable merge target - mark for removal if very low plasticity
                    if *self.plasticity_scores.get(&node_id).unwrap_or(&1.0)
                        < self.capacity_config.min_consolidation_plasticity / 2.0
                    {
                        self.remove_node_data(node_id);
                        removed_nodes.push(node_id);
                        nodes_consolidated += 1;

                        let pruned = self.prune_node_connections(node_id);
                        connections_pruned += pruned;
                    }
                }
            } else {
                // Node not in any cluster - candidate for removal
                if *self.plasticity_scores.get(&node_id).unwrap_or(&1.0)
                    < self.capacity_config.min_consolidation_plasticity / 2.0
                {
                    self.remove_node_data(node_id);
                    removed_nodes.push(node_id);
                    nodes_consolidated += 1;

                    let pruned = self.prune_node_connections(node_id);
                    connections_pruned += pruned;
                }
            }

            // Check if we've reached batch limit
            if nodes_consolidated >= self.capacity_config.max_consolidation_batch {
                break;
            }
        }

        // Phase 3: Prune low-usage connections network-wide
        let additional_pruned = self.prune_unused_connections()?;
        connections_pruned += additional_pruned as usize;

        // Calculate new utilization
        let estimated_new_nodes = start_nodes.saturating_sub(nodes_consolidated);
        let new_utilization = estimated_new_nodes as f32 / self.max_nodes as f32;

        // Estimate memory freed (rough estimate: ~1KB per node, ~100 bytes per connection)
        let memory_freed_bytes = (nodes_consolidated * 1024 + connections_pruned * 100) as i64;

        // Record the consolidation event
        let event = ReorganizationEvent {
            timestamp_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event_type: ReorganizationType::ConnectionPruning,
            nodes_affected: removed_nodes
                .iter()
                .chain(merged_nodes.iter().map(|(src, _)| src))
                .copied()
                .collect(),
            performance_impact: 0.05 * nodes_consolidated as f32,
            memory_delta: -memory_freed_bytes,
        };
        self.reorganization_history.push(event);

        Ok(ConsolidationResult {
            nodes_consolidated,
            connections_pruned,
            memory_freed_bytes,
            new_utilization,
            merged_nodes,
            removed_nodes,
        })
    }

    /// Find the best merge target for a node within its cluster
    fn find_merge_target(&self, node_id: u64, cluster_id: u32) -> Option<u64> {
        // Find nodes in the same cluster with higher plasticity scores
        let candidates: Vec<_> = self
            .cluster_assignments
            .iter()
            .filter(|(&id, &cid)| id != node_id && cid == cluster_id)
            .filter_map(|(&id, _)| self.plasticity_scores.get(&id).map(|&score| (id, score)))
            .filter(|(_id, score)| *score > self.capacity_config.min_consolidation_plasticity)
            .collect();

        // Return the node with highest plasticity score
        candidates
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id)
    }

    /// Merge data from source node into target node
    fn merge_node_data(&mut self, source_id: u64, target_id: u64) {
        // Transfer access frequency to target
        if let Some(source_freq) = self
            .access_patterns
            .node_access_frequency
            .remove(&source_id)
        {
            *self
                .access_patterns
                .node_access_frequency
                .entry(target_id)
                .or_insert(0) += source_freq;
        }

        // Transfer temporal locality data
        if let Some(source_times) = self
            .access_patterns
            .temporal_locality_secs
            .remove(&source_id)
        {
            self.access_patterns
                .temporal_locality_secs
                .entry(target_id)
                .or_default()
                .extend(source_times);
        }

        // Transfer spatial locality - redirect neighbors to target
        if let Some(source_neighbors) = self.access_patterns.spatial_locality.remove(&source_id) {
            let target_neighbors = self
                .access_patterns
                .spatial_locality
                .entry(target_id)
                .or_default();
            for neighbor in source_neighbors {
                if neighbor != target_id && !target_neighbors.contains(&neighbor) {
                    target_neighbors.push(neighbor);
                }
            }
        }

        // Update plasticity score for target
        let source_score = self.plasticity_scores.remove(&source_id).unwrap_or(0.0);
        if let Some(target_score) = self.plasticity_scores.get_mut(&target_id) {
            // Combine scores with weighted average
            *target_score = (*target_score).mul_add(0.7, source_score * 0.3).min(1.0);
        }

        // Remove cluster assignment for source
        self.cluster_assignments.remove(&source_id);

        debug!("Merged node {} into node {}", source_id, target_id);
    }

    /// Remove all data for a node
    fn remove_node_data(&mut self, node_id: u64) {
        self.access_patterns.node_access_frequency.remove(&node_id);
        self.access_patterns.temporal_locality_secs.remove(&node_id);
        self.access_patterns.spatial_locality.remove(&node_id);
        self.plasticity_scores.remove(&node_id);
        self.cluster_assignments.remove(&node_id);

        debug!("Removed node data for node {}", node_id);
    }

    /// Prune all connections involving a specific node
    fn prune_node_connections(&mut self, node_id: u64) -> usize {
        let connections_before = self.access_patterns.connection_usage.len();

        // Remove connections where node_id is source or target
        self.access_patterns
            .connection_usage
            .retain(|(src, tgt), _| *src != node_id && *tgt != node_id);

        // Also update spatial locality for other nodes that reference this node
        for neighbors in self.access_patterns.spatial_locality.values_mut() {
            neighbors.retain(|&n| n != node_id);
        }

        connections_before - self.access_patterns.connection_usage.len()
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
            .map_or(0.0, |times| times.len() as f32);

        let spatial_locality = self
            .access_patterns
            .spatial_locality
            .get(&node_id)
            .map_or(0.0, |neighbors| neighbors.len() as f32);

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
                nodes_affected: self.plasticity_scores.keys().copied().collect(),
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
            let frequency_percentile =
                (*frequency as f32) / (nodes_by_frequency.first().map_or(1, |(_, f)| *f) as f32);

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
        let node_ids: Vec<_> = self.plasticity_scores.keys().copied().collect();
        for node_id in node_ids {
            self.update_plasticity_score(node_id);
        }
    }

    /// Get plasticity statistics
    #[must_use]
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
    pub const fn set_params(&mut self, params: PlasticityParams) {
        self.params = params;
    }

    /// Get current access patterns
    #[must_use]
    pub const fn get_access_patterns(&self) -> &AccessPatterns {
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

    #[test]
    fn test_capacity_config_default() {
        let config = CapacityConfig::default();
        assert_eq!(config.consolidation_threshold, 0.90);
        assert_eq!(config.warning_threshold, 0.80);
        assert_eq!(config.max_consolidation_batch, 100);
        assert_eq!(config.min_consolidation_plasticity, 0.1);
    }

    #[test]
    fn test_plasticity_matrix_with_capacity_config() {
        let config = CapacityConfig {
            consolidation_threshold: 0.95,
            warning_threshold: 0.85,
            max_consolidation_batch: 50,
            min_consolidation_plasticity: 0.05,
        };

        let matrix = PlasticityMatrix::with_capacity_config(1000, 0.5, config).unwrap();
        assert_eq!(matrix.max_nodes(), 1000);
        assert_eq!(matrix.capacity_config().consolidation_threshold, 0.95);
        assert_eq!(matrix.capacity_config().warning_threshold, 0.85);
    }

    #[test]
    fn test_invalid_capacity_config() {
        // Warning threshold > consolidation threshold should fail
        let config = CapacityConfig {
            consolidation_threshold: 0.80,
            warning_threshold: 0.90, // Invalid: warning > consolidation
            max_consolidation_batch: 100,
            min_consolidation_plasticity: 0.1,
        };

        let result = PlasticityMatrix::with_capacity_config(1000, 0.5, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_capacity_below_threshold() {
        let matrix = PlasticityMatrix::new(100, 0.5).unwrap();

        // Create a network with max 1000 nodes, activation threshold 0.5
        let network = SynapticNetwork::new(1000, 0.5).unwrap();
        // Network starts empty, so utilization should be 0%

        let result = matrix.check_capacity(&network).unwrap();
        assert_eq!(result.current_nodes, 0);
        assert_eq!(result.max_nodes, 100);
        assert_eq!(result.utilization, 0.0);
        assert!(!result.needs_consolidation);
        assert!(!result.at_warning_level);
    }

    #[test]
    fn test_check_capacity_high_utilization() {
        let mut matrix = PlasticityMatrix::new(100, 0.5).unwrap();

        // Simulate high plasticity scores for consolidation candidate counting
        for i in 0..10 {
            matrix.plasticity_scores.insert(i, 0.05); // Low plasticity = consolidation candidate
        }

        let result = matrix
            .check_capacity(&SynapticNetwork::new(1000, 0.5).unwrap())
            .unwrap();

        // Should have consolidation candidates due to low plasticity scores
        assert_eq!(result.consolidation_candidates, 10);
    }

    #[test]
    fn test_find_merge_target() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();

        // Set up nodes in same cluster
        matrix.cluster_assignments.insert(1, 0);
        matrix.cluster_assignments.insert(2, 0);
        matrix.cluster_assignments.insert(3, 0);

        // Set plasticity scores
        matrix.plasticity_scores.insert(1, 0.1); // Low - merge candidate
        matrix.plasticity_scores.insert(2, 0.5); // High - merge target
        matrix.plasticity_scores.insert(3, 0.3); // Medium

        // Node 1 should merge into node 2 (highest plasticity in cluster)
        let target = matrix.find_merge_target(1, 0);
        assert_eq!(target, Some(2));
    }

    #[test]
    fn test_find_merge_target_no_suitable_target() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();

        // Set up nodes in same cluster with all low plasticity
        matrix.cluster_assignments.insert(1, 0);
        matrix.cluster_assignments.insert(2, 0);

        // All nodes have low plasticity
        matrix.plasticity_scores.insert(1, 0.05);
        matrix.plasticity_scores.insert(2, 0.05);

        // No suitable merge target (all below threshold)
        let target = matrix.find_merge_target(1, 0);
        assert_eq!(target, None);
    }

    #[test]
    fn test_merge_node_data() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();

        // Set up source node data
        matrix.access_patterns.node_access_frequency.insert(1, 100);
        matrix.access_patterns.node_access_frequency.insert(2, 50);
        matrix
            .access_patterns
            .temporal_locality_secs
            .insert(1, vec![1000, 2000]);
        matrix
            .access_patterns
            .spatial_locality
            .insert(1, vec![3, 4]);
        matrix.plasticity_scores.insert(1, 0.3);
        matrix.plasticity_scores.insert(2, 0.5);
        matrix.cluster_assignments.insert(1, 0);
        matrix.cluster_assignments.insert(2, 0);

        // Merge node 1 into node 2
        matrix.merge_node_data(1, 2);

        // Source node data should be removed
        assert!(!matrix
            .access_patterns
            .node_access_frequency
            .contains_key(&1));
        assert!(!matrix
            .access_patterns
            .temporal_locality_secs
            .contains_key(&1));
        assert!(!matrix.plasticity_scores.contains_key(&1));
        assert!(!matrix.cluster_assignments.contains_key(&1));

        // Target node should have combined data
        assert_eq!(
            matrix.access_patterns.node_access_frequency.get(&2),
            Some(&150)
        ); // 100 + 50
        assert!(matrix
            .access_patterns
            .temporal_locality_secs
            .contains_key(&2));
    }

    #[test]
    fn test_remove_node_data() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();

        // Set up node data
        matrix.access_patterns.node_access_frequency.insert(1, 100);
        matrix
            .access_patterns
            .temporal_locality_secs
            .insert(1, vec![1000]);
        matrix
            .access_patterns
            .spatial_locality
            .insert(1, vec![2, 3]);
        matrix.plasticity_scores.insert(1, 0.5);
        matrix.cluster_assignments.insert(1, 0);

        // Remove node
        matrix.remove_node_data(1);

        // All data should be removed
        assert!(!matrix
            .access_patterns
            .node_access_frequency
            .contains_key(&1));
        assert!(!matrix
            .access_patterns
            .temporal_locality_secs
            .contains_key(&1));
        assert!(!matrix.access_patterns.spatial_locality.contains_key(&1));
        assert!(!matrix.plasticity_scores.contains_key(&1));
        assert!(!matrix.cluster_assignments.contains_key(&1));
    }

    #[test]
    fn test_prune_node_connections() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();

        // Set up connections
        matrix.access_patterns.connection_usage.insert((1, 2), 10);
        matrix.access_patterns.connection_usage.insert((1, 3), 5);
        matrix.access_patterns.connection_usage.insert((2, 3), 8);
        matrix
            .access_patterns
            .spatial_locality
            .insert(2, vec![1, 3]);
        matrix
            .access_patterns
            .spatial_locality
            .insert(3, vec![1, 2]);

        // Prune connections for node 1
        let pruned = matrix.prune_node_connections(1);

        assert_eq!(pruned, 2); // (1,2) and (1,3)
        assert!(!matrix
            .access_patterns
            .connection_usage
            .contains_key(&(1, 2)));
        assert!(!matrix
            .access_patterns
            .connection_usage
            .contains_key(&(1, 3)));
        assert!(matrix
            .access_patterns
            .connection_usage
            .contains_key(&(2, 3)));

        // Node 1 should be removed from spatial locality of other nodes
        assert!(!matrix
            .access_patterns
            .spatial_locality
            .get(&2)
            .unwrap()
            .contains(&1));
        assert!(!matrix
            .access_patterns
            .spatial_locality
            .get(&3)
            .unwrap()
            .contains(&1));
    }

    #[test]
    fn test_trigger_consolidation() {
        let mut matrix = PlasticityMatrix::new(100, 0.5).unwrap();
        let network = SynapticNetwork::new(1000, 0.5).unwrap();

        // Set up nodes with low plasticity scores
        for i in 0..5 {
            matrix.access_patterns.node_access_frequency.insert(i, 10);
            matrix.plasticity_scores.insert(i, 0.01); // Very low - consolidation candidate
            matrix.cluster_assignments.insert(i, 0);
        }

        // Set up a high plasticity node as merge target
        matrix
            .access_patterns
            .node_access_frequency
            .insert(100, 1000);
        matrix.plasticity_scores.insert(100, 0.8);
        matrix.cluster_assignments.insert(100, 0);

        // Trigger consolidation
        let result = matrix.trigger_consolidation(&network).unwrap();

        // Should have consolidated some nodes
        assert!(result.nodes_consolidated > 0 || result.connections_pruned > 0);
        assert!(result.memory_freed_bytes >= 0);
    }

    #[test]
    fn test_check_and_reorganize_no_action_needed() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();
        let network = SynapticNetwork::new(1000, 0.5).unwrap();

        // Empty network - no consolidation needed
        let result = matrix.check_and_reorganize(&network).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_capacity_check_result_structure() {
        let result = CapacityCheckResult {
            current_nodes: 80,
            max_nodes: 100,
            utilization: 0.8,
            needs_consolidation: false,
            at_warning_level: true,
            consolidation_candidates: 5,
        };

        assert_eq!(result.current_nodes, 80);
        assert_eq!(result.max_nodes, 100);
        assert!((result.utilization - 0.8).abs() < 0.001);
        assert!(!result.needs_consolidation);
        assert!(result.at_warning_level);
        assert_eq!(result.consolidation_candidates, 5);
    }

    #[test]
    fn test_consolidation_result_structure() {
        let result = ConsolidationResult {
            nodes_consolidated: 10,
            connections_pruned: 25,
            memory_freed_bytes: 15000,
            new_utilization: 0.75,
            merged_nodes: vec![(1, 2), (3, 4)],
            removed_nodes: vec![5, 6, 7],
        };

        assert_eq!(result.nodes_consolidated, 10);
        assert_eq!(result.connections_pruned, 25);
        assert_eq!(result.memory_freed_bytes, 15000);
        assert!((result.new_utilization - 0.75).abs() < 0.001);
        assert_eq!(result.merged_nodes.len(), 2);
        assert_eq!(result.removed_nodes.len(), 3);
    }

    #[test]
    fn test_set_capacity_config() {
        let mut matrix = PlasticityMatrix::new(1000, 0.5).unwrap();

        let new_config = CapacityConfig {
            consolidation_threshold: 0.95,
            warning_threshold: 0.85,
            max_consolidation_batch: 200,
            min_consolidation_plasticity: 0.2,
        };

        matrix.set_capacity_config(new_config);

        assert_eq!(matrix.capacity_config().consolidation_threshold, 0.95);
        assert_eq!(matrix.capacity_config().warning_threshold, 0.85);
        assert_eq!(matrix.capacity_config().max_consolidation_batch, 200);
        assert_eq!(matrix.capacity_config().min_consolidation_plasticity, 0.2);
    }
}
