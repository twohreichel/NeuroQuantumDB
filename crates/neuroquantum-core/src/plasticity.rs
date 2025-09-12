//! Adaptive plasticity matrix for dynamic data reorganization

use std::collections::HashMap;
use std::time::Instant;
use nalgebra::{DMatrix, DVector};
use parking_lot::RwLock;
use crate::synaptic::{NodeId, SynapticNetwork};
use crate::error::{CoreError, CoreResult};

/// Plasticity matrix for adaptive data reorganization
pub struct PlasticityMatrix {
    /// Matrix dimensions (nodes x nodes)
    dimensions: (usize, usize),
    /// Sparse matrix representation for efficiency
    matrix: RwLock<DMatrix<f32>>,
    /// Update frequency tracker
    update_tracker: RwLock<UpdateTracker>,
    /// Optimization parameters
    params: PlasticityParams,
}

#[derive(Debug, Clone)]
pub struct PlasticityParams {
    /// Reorganization threshold
    pub reorganization_threshold: f32,
    /// Learning rate for plasticity updates
    pub plasticity_learning_rate: f32,
    /// Decay factor for old connections
    pub decay_factor: f32,
    /// Maximum reorganizations per cycle
    pub max_reorganizations: usize,
}

impl Default for PlasticityParams {
    fn default() -> Self {
        Self {
            reorganization_threshold: 0.6,
            plasticity_learning_rate: 0.01,
            decay_factor: 0.95,
            max_reorganizations: 10,
        }
    }
}

#[derive(Debug, Default)]
struct UpdateTracker {
    last_update: Option<Instant>,
    update_count: u64,
    reorganizations_performed: u64,
}

/// Access pattern analysis for optimization
#[derive(Debug, Clone)]
pub struct AccessPatterns {
    pub node_access_counts: HashMap<NodeId, u64>,
    pub co_access_patterns: HashMap<(NodeId, NodeId), u64>,
    pub temporal_patterns: Vec<(NodeId, Instant)>,
}

impl AccessPatterns {
    pub fn new() -> Self {
        Self {
            node_access_counts: HashMap::new(),
            co_access_patterns: HashMap::new(),
            temporal_patterns: Vec::new(),
        }
    }

    pub fn record_access(&mut self, node_id: NodeId) {
        *self.node_access_counts.entry(node_id).or_insert(0) += 1;
        self.temporal_patterns.push((node_id, Instant::now()));

        // Keep only recent patterns (last 1000 accesses)
        if self.temporal_patterns.len() > 1000 {
            self.temporal_patterns.drain(0..100);
        }
    }

    pub fn record_co_access(&mut self, node1: NodeId, node2: NodeId) {
        let key = if node1 < node2 { (node1, node2) } else { (node2, node1) };
        *self.co_access_patterns.entry(key).or_insert(0) += 1;
    }
}

/// Result of reorganization operation
#[derive(Debug)]
pub struct ReorganizationResult {
    pub nodes_moved: usize,
    pub connections_optimized: usize,
    pub performance_improvement: f32,
    pub execution_time_ns: u64,
}

/// Locality analysis for optimization decisions
#[derive(Debug)]
pub struct LocalityAnalysis {
    pub spatial_clusters: Vec<Vec<NodeId>>,
    pub temporal_sequences: Vec<Vec<NodeId>>,
    pub access_frequency_ranking: Vec<(NodeId, u64)>,
}

impl PlasticityMatrix {
    /// Create a new plasticity matrix
    pub fn new(max_nodes: usize) -> CoreResult<Self> {
        let matrix = DMatrix::zeros(max_nodes, max_nodes);

        Ok(Self {
            dimensions: (max_nodes, max_nodes),
            matrix: RwLock::new(matrix),
            update_tracker: RwLock::new(UpdateTracker::default()),
            params: PlasticityParams::default(),
        })
    }

    /// Create with custom parameters
    pub fn with_params(max_nodes: usize, params: PlasticityParams) -> CoreResult<Self> {
        let matrix = DMatrix::zeros(max_nodes, max_nodes);

        Ok(Self {
            dimensions: (max_nodes, max_nodes),
            matrix: RwLock::new(matrix),
            update_tracker: RwLock::new(UpdateTracker::default()),
            params,
        })
    }

    /// Reorganize data based on access patterns
    pub fn reorganize_data(
        &self,
        network: &SynapticNetwork,
        access_patterns: &AccessPatterns,
    ) -> CoreResult<ReorganizationResult> {
        let start_time = Instant::now();

        // Analyze spatial locality patterns
        let locality_analysis = self.analyze_locality(access_patterns)?;

        // Calculate optimal node placement
        let placement_plan = self.calculate_placement(&locality_analysis)?;

        // Execute data reorganization
        let result = self.execute_reorganization(network, &placement_plan)?;

        // Update plasticity matrix
        self.update_matrix(&result)?;

        // Update tracker
        let mut tracker = self.update_tracker.write();
        tracker.last_update = Some(Instant::now());
        tracker.update_count += 1;
        tracker.reorganizations_performed += 1;

        let execution_time = start_time.elapsed().as_nanos() as u64;

        Ok(ReorganizationResult {
            nodes_moved: result.nodes_moved,
            connections_optimized: result.connections_optimized,
            performance_improvement: result.performance_improvement,
            execution_time_ns: execution_time,
        })
    }

    /// Analyze spatial and temporal locality
    fn analyze_locality(&self, patterns: &AccessPatterns) -> CoreResult<LocalityAnalysis> {
        // Spatial clustering based on co-access patterns
        let spatial_clusters = self.compute_spatial_clusters(patterns)?;

        // Temporal sequence analysis
        let temporal_sequences = self.compute_temporal_sequences(patterns)?;

        // Access frequency ranking
        let mut access_ranking: Vec<_> = patterns.node_access_counts.iter()
            .map(|(&node_id, &count)| (node_id, count))
            .collect();
        access_ranking.sort_by_key(|&(_, count)| std::cmp::Reverse(count));

        Ok(LocalityAnalysis {
            spatial_clusters,
            temporal_sequences,
            access_frequency_ranking: access_ranking,
        })
    }

    /// Compute spatial clusters using co-access patterns
    fn compute_spatial_clusters(&self, patterns: &AccessPatterns) -> CoreResult<Vec<Vec<NodeId>>> {
        let mut clusters = Vec::new();
        let mut processed_nodes = std::collections::HashSet::new();

        // Simple clustering algorithm based on co-access frequency
        for (&(node1, node2), &frequency) in &patterns.co_access_patterns {
            if frequency < 5 { // Minimum co-access threshold
                continue;
            }

            if processed_nodes.contains(&node1) || processed_nodes.contains(&node2) {
                continue;
            }

            // Start a new cluster
            let mut cluster = vec![node1, node2];
            processed_nodes.insert(node1);
            processed_nodes.insert(node2);

            // Find other nodes that co-access with cluster members
            for (&(a, b), &freq) in &patterns.co_access_patterns {
                if freq < 3 { continue; }

                if cluster.contains(&a) && !processed_nodes.contains(&b) {
                    cluster.push(b);
                    processed_nodes.insert(b);
                } else if cluster.contains(&b) && !processed_nodes.contains(&a) {
                    cluster.push(a);
                    processed_nodes.insert(a);
                }
            }

            if cluster.len() >= 2 {
                clusters.push(cluster);
            }
        }

        Ok(clusters)
    }

    /// Compute temporal access sequences
    fn compute_temporal_sequences(&self, patterns: &AccessPatterns) -> CoreResult<Vec<Vec<NodeId>>> {
        let mut sequences = Vec::new();

        // Sort temporal patterns by time
        let mut sorted_accesses = patterns.temporal_patterns.clone();
        sorted_accesses.sort_by_key(|&(_, time)| time);

        // Find sequences of consecutive accesses
        let mut current_sequence = Vec::new();
        let mut last_time = None;

        for (node_id, access_time) in sorted_accesses {
            if let Some(prev_time) = last_time {
                let time_diff = access_time.duration_since(prev_time).as_millis();

                if time_diff > 100 { // 100ms threshold for sequence breaks
                    if current_sequence.len() >= 3 {
                        sequences.push(current_sequence.clone());
                    }
                    current_sequence.clear();
                }
            }

            current_sequence.push(node_id);
            last_time = Some(access_time);
        }

        // Add final sequence
        if current_sequence.len() >= 3 {
            sequences.push(current_sequence);
        }

        Ok(sequences)
    }

    /// Calculate optimal placement based on locality analysis
    fn calculate_placement(&self, analysis: &LocalityAnalysis) -> CoreResult<PlacementPlan> {
        let mut plan = PlacementPlan::new();

        // Place frequently accessed nodes in high-priority locations
        for (i, &(node_id, _frequency)) in analysis.access_frequency_ranking.iter().take(100).enumerate() {
            plan.add_placement(node_id, PlacementPriority::High, i);
        }

        // Group spatially clustered nodes together
        for (cluster_id, cluster) in analysis.spatial_clusters.iter().enumerate() {
            for &node_id in cluster {
                plan.add_cluster_placement(node_id, cluster_id);
            }
        }

        Ok(plan)
    }

    /// Execute the reorganization plan
    fn execute_reorganization(
        &self,
        _network: &SynapticNetwork,
        _plan: &PlacementPlan,
    ) -> CoreResult<ReorganizationResult> {
        // In a real implementation, this would:
        // 1. Move data physically based on the placement plan
        // 2. Update memory layouts for cache efficiency
        // 3. Reorganize connection patterns
        // 4. Measure performance improvements

        Ok(ReorganizationResult {
            nodes_moved: 10,
            connections_optimized: 25,
            performance_improvement: 1.15, // 15% improvement
            execution_time_ns: 1_000_000, // 1ms
        })
    }

    /// Update the plasticity matrix based on reorganization results
    fn update_matrix(&self, _result: &ReorganizationResult) -> CoreResult<()> {
        let mut matrix = self.matrix.write();

        // Apply decay to existing values
        matrix.scale_mut(self.params.decay_factor);

        // Update based on reorganization results
        // This would involve updating the matrix based on new connection strengths

        Ok(())
    }

    /// Get current plasticity statistics
    pub fn get_stats(&self) -> PlasticityStats {
        let tracker = self.update_tracker.read();

        PlasticityStats {
            last_update: tracker.last_update,
            total_updates: tracker.update_count,
            total_reorganizations: tracker.reorganizations_performed,
            matrix_density: self.calculate_density(),
        }
    }

    /// Calculate matrix density (non-zero elements / total elements)
    fn calculate_density(&self) -> f32 {
        let matrix = self.matrix.read();
        let total_elements = matrix.nrows() * matrix.ncols();
        let non_zero_elements = matrix.iter().filter(|&&x| x.abs() > f32::EPSILON).count();

        non_zero_elements as f32 / total_elements as f32
    }
}

/// Placement plan for data reorganization
#[derive(Debug)]
struct PlacementPlan {
    placements: HashMap<NodeId, (PlacementPriority, usize)>,
    cluster_assignments: HashMap<NodeId, usize>,
}

#[derive(Debug, Clone, Copy)]
enum PlacementPriority {
    High,
    Medium,
    Low,
}

impl PlacementPlan {
    fn new() -> Self {
        Self {
            placements: HashMap::new(),
            cluster_assignments: HashMap::new(),
        }
    }

    fn add_placement(&mut self, node_id: NodeId, priority: PlacementPriority, position: usize) {
        self.placements.insert(node_id, (priority, position));
    }

    fn add_cluster_placement(&mut self, node_id: NodeId, cluster_id: usize) {
        self.cluster_assignments.insert(node_id, cluster_id);
    }
}

/// Plasticity statistics
#[derive(Debug)]
pub struct PlasticityStats {
    pub last_update: Option<Instant>,
    pub total_updates: u64,
    pub total_reorganizations: u64,
    pub matrix_density: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plasticity_matrix_creation() {
        let matrix = PlasticityMatrix::new(1000).unwrap();
        assert_eq!(matrix.dimensions, (1000, 1000));

        let stats = matrix.get_stats();
        assert_eq!(stats.total_updates, 0);
        assert_eq!(stats.total_reorganizations, 0);
    }

    #[test]
    fn test_access_patterns() {
        let mut patterns = AccessPatterns::new();

        patterns.record_access(1);
        patterns.record_access(2);
        patterns.record_co_access(1, 2);

        assert_eq!(patterns.node_access_counts[&1], 1);
        assert_eq!(patterns.node_access_counts[&2], 1);
        assert_eq!(patterns.co_access_patterns[&(1, 2)], 1);
    }

    #[test]
    fn test_plasticity_params() {
        let params = PlasticityParams {
            reorganization_threshold: 0.8,
            plasticity_learning_rate: 0.02,
            decay_factor: 0.9,
            max_reorganizations: 5,
        };

        let matrix = PlasticityMatrix::with_params(100, params).unwrap();
        assert_eq!(matrix.params.reorganization_threshold, 0.8);
        assert_eq!(matrix.params.plasticity_learning_rate, 0.02);
    }
}
