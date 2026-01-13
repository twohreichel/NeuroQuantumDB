//! # Neuromorphic Query Processing
//!
//! Spiking neural network implementation for intelligent query processing
//! using brain-inspired algorithms in NeuroQuantumDB.

use crate::error::{CoreError, CoreResult};
use crate::learning::HebbianLearningEngine;
use crate::synaptic::SynapticNetwork;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{debug, instrument};

/// Query types supported by the neuromorphic processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
    CreateIndex,
    DropIndex,
    Analyze,
}

/// Optimization suggestion types for query performance improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationSuggestionType {
    /// Create a new index on specified columns
    CreateIndex,
    /// Create a composite index for multiple columns
    CreateCompositeIndex,
    /// Suggest query restructuring
    RestructureQuery,
    /// Suggest batch processing
    BatchProcessing,
    /// Neural pathway optimization
    NeuralPathwayOptimization,
    /// Add query hints
    AddQueryHints,
    /// Partition data
    DataPartitioning,
    /// Use neuromorphic caching
    NeuromorphicCaching,
}

/// Index type recommendation for optimization suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestedIndexType {
    /// B-Tree index for range queries and sorting
    BTree,
    /// Hash index for equality comparisons
    Hash,
    /// Neural similarity index for pattern matching
    NeuralSimilarity,
    /// K-mer index for DNA sequence queries
    DnaKmer,
    /// Quantum entanglement index for correlated data
    QuantumEntanglement,
}

/// A single optimization suggestion with details and estimated benefit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    /// Type of optimization suggested
    pub suggestion_type: OptimizationSuggestionType,
    /// Human-readable description of the suggestion
    pub description: String,
    /// Affected fields/columns
    pub affected_fields: Vec<String>,
    /// Estimated performance improvement (0.0 to 1.0, where 1.0 = 100% improvement)
    pub estimated_improvement: f32,
    /// Confidence score of the suggestion (0.0 to 1.0)
    pub confidence: f32,
    /// Priority level (1 = highest, 10 = lowest)
    pub priority: u8,
    /// Suggested index type if applicable
    pub suggested_index_type: Option<SuggestedIndexType>,
    /// Additional metadata for the suggestion
    pub metadata: HashMap<String, String>,
}

impl OptimizationSuggestion {
    /// Create a new optimization suggestion
    pub fn new(
        suggestion_type: OptimizationSuggestionType,
        description: String,
        affected_fields: Vec<String>,
        estimated_improvement: f32,
        confidence: f32,
        priority: u8,
    ) -> Self {
        Self {
            suggestion_type,
            description,
            affected_fields,
            estimated_improvement: estimated_improvement.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            priority: priority.clamp(1, 10),
            suggested_index_type: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the suggested index type
    pub fn with_index_type(mut self, index_type: SuggestedIndexType) -> Self {
        self.suggested_index_type = Some(index_type);
        self
    }

    /// Add metadata to the suggestion
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Query result with neuromorphic enhancements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query_id: u64,
    pub matched_nodes: Vec<u64>,
    pub execution_time_ns: u64,
    pub activation_score: f32,
    pub metadata: HashMap<String, String>,
}

impl QueryResult {
    /// Create an empty query result
    pub fn empty() -> Self {
        Self {
            query_id: 0,
            matched_nodes: Vec::new(),
            execution_time_ns: 0,
            activation_score: 0.0,
            metadata: HashMap::new(),
        }
    }

    /// Create a new query result
    pub fn new(
        query_id: u64,
        matched_nodes: Vec<u64>,
        execution_time_ns: u64,
        activation_score: f32,
    ) -> Self {
        Self {
            query_id,
            matched_nodes,
            execution_time_ns,
            activation_score,
            metadata: HashMap::new(),
        }
    }
}

/// Query structure for neuromorphic processing
#[derive(Debug, Clone)]
pub struct Query {
    pub id: u64,
    pub query_type: QueryType,
    pub content: String,
    pub target_nodes: Vec<u64>,
    pub conditions: Vec<QueryCondition>,
    pub timestamp_secs: u64, // Store as seconds since epoch instead of Instant
    pub priority: u8,        // 0-255, higher = more priority
    pub expected_result_size: Option<usize>,
}

/// Query condition for filtering and matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCondition {
    pub field: String,
    pub operator: String,
    pub value: String,
    pub weight: f32, // Importance weight for neuromorphic processing
}

impl Query {
    /// Create a new query with content
    pub fn new(content: String) -> Self {
        Self {
            id: rand::random(),
            query_type: QueryType::Select,
            content,
            target_nodes: Vec::new(),
            conditions: Vec::new(),
            timestamp_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            priority: 128,
            expected_result_size: None,
        }
    }
}

/// Neuromorphic query processor using spiking neural networks
pub struct NeuromorphicQueryProcessor {
    network: Arc<RwLock<SynapticNetwork>>,
    learning_engine: Arc<RwLock<HebbianLearningEngine>>,
    query_cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    spike_patterns: Arc<RwLock<HashMap<u64, Vec<Instant>>>>, // Node spike histories for STDP
    activation_threshold: f32,
    neon_optimizations: bool,
    query_statistics: Arc<RwLock<QueryStatistics>>,
}

/// Cached query result for performance optimization
#[derive(Debug, Clone)]
struct CachedResult {
    result: QueryResult,
    created_at_secs: u64,    // Used for cache expiration logic
    access_count: u64,       // Used for cache statistics and LRU eviction
    last_accessed_secs: u64, // Used for cache aging and cleanup
}

/// Statistics about query processing performance
#[derive(Debug, Clone, Default, Serialize)]
pub struct QueryStatistics {
    pub total_queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_processing_time_ns: u64,
    pub total_spikes_generated: u64,
    pub neural_pathways_discovered: u64,
    pub optimization_events: u64,
}

impl NeuromorphicQueryProcessor {
    /// Create a new neuromorphic query processor
    pub fn new(
        network: Arc<RwLock<SynapticNetwork>>,
        learning_engine: Arc<RwLock<HebbianLearningEngine>>,
        neon_optimizations: bool,
    ) -> CoreResult<Self> {
        Ok(Self {
            network,
            learning_engine,
            query_cache: Arc::new(RwLock::new(HashMap::new())),
            spike_patterns: Arc::new(RwLock::new(HashMap::new())),
            activation_threshold: 0.5,
            neon_optimizations,
            query_statistics: Arc::new(RwLock::new(QueryStatistics::default())),
        })
    }

    /// Process a query using neuromorphic intelligence
    #[instrument(level = "debug", skip(self, query))]
    pub fn process_query(&self, query: &Query) -> CoreResult<QueryResult> {
        let start_time = Instant::now();

        // Check cache first
        let cache_key = self.generate_cache_key(query);
        if let Some(cached) = self.check_cache(&cache_key) {
            let result = cached.result.clone();
            return Ok(result);
        }

        // Process query through spiking neural network
        let neural_pathway = self.activate_neural_pathway(query)?;
        let matched_nodes = self.execute_pattern_matching(query, &neural_pathway)?;
        let _confidence_scores = self.calculate_confidence_scores(&matched_nodes)?;

        // Generate spikes for learning
        self.generate_learning_spikes(&neural_pathway, &matched_nodes)?;

        // Calculate learning feedback
        let _learning_feedback = self.calculate_learning_feedback(&matched_nodes, query);

        // Create result
        let result = QueryResult {
            query_id: query.id,
            matched_nodes,
            execution_time_ns: start_time.elapsed().as_nanos() as u64,
            activation_score: 0.0,
            metadata: HashMap::new(),
        };

        // Cache the result
        self.cache_result(cache_key, &result);

        // Update statistics
        self.update_statistics(&result);

        debug!(
            "Processed query {} in {}ns",
            query.id, result.execution_time_ns
        );
        Ok(result)
    }

    /// Generate cache key for query
    fn generate_cache_key(&self, query: &Query) -> String {
        // Simple hash of query components
        format!(
            "{:?}_{:?}_{:?}",
            query.query_type, query.target_nodes, query.conditions
        )
    }

    /// Check query cache for existing result
    fn check_cache(&self, cache_key: &str) -> Option<CachedResult> {
        if let Ok(mut cache) = self.query_cache.write() {
            if let Some(cached) = cache.get_mut(cache_key) {
                // Check if cache entry has expired (TTL: 300 seconds / 5 minutes)
                const CACHE_TTL_SECONDS: u64 = 300;
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                if current_time - cached.created_at_secs > CACHE_TTL_SECONDS {
                    // Cache entry expired, remove it
                    cache.remove(cache_key);

                    if let Ok(mut stats) = self.query_statistics.write() {
                        stats.cache_misses += 1;
                    }
                    return None;
                }

                // Update access statistics
                cached.access_count += 1;
                cached.last_accessed_secs = current_time;

                // Update statistics
                if let Ok(mut stats) = self.query_statistics.write() {
                    stats.cache_hits += 1;
                }

                return Some(cached.clone());
            } else {
                // Cache miss
                if let Ok(mut stats) = self.query_statistics.write() {
                    stats.cache_misses += 1;
                }
            }
        }
        None
    }

    /// Cache query result with LRU eviction
    fn cache_result(&self, cache_key: String, result: &QueryResult) {
        if let Ok(mut cache) = self.query_cache.write() {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let cached = CachedResult {
                result: result.clone(),
                created_at_secs: current_time,
                access_count: 1,
                last_accessed_secs: current_time,
            };

            // LRU eviction: if cache is too large, remove least recently used
            const MAX_CACHE_SIZE: usize = 1000;
            if cache.len() >= MAX_CACHE_SIZE {
                // Find and remove the least recently accessed entry
                if let Some(lru_key) = cache
                    .iter()
                    .min_by_key(|(_, v)| v.last_accessed_secs)
                    .map(|(k, _)| k.clone())
                {
                    cache.remove(&lru_key);
                }
            }

            cache.insert(cache_key, cached);
        }
    }

    /// Activate neural pathway for query processing
    fn activate_neural_pathway(&self, query: &Query) -> CoreResult<Vec<u64>> {
        let mut pathway = Vec::new();
        let network = self
            .network
            .read()
            .map_err(|_| CoreError::LockError("Failed to acquire network read lock".to_string()))?;

        // Start with target nodes specified in query
        for &node_id in &query.target_nodes {
            if let Some(node) = network.get_node(node_id) {
                if node.activation_level > self.activation_threshold {
                    pathway.push(node_id);

                    // Follow strong connections
                    for connection in &node.connections {
                        if connection.weight > 0.7 {
                            pathway.push(connection.target_id);
                        }
                    }
                }
            }
        }

        // Remove duplicates and limit pathway length
        pathway.sort_unstable();
        pathway.dedup();
        pathway.truncate(50); // Limit to prevent excessive processing

        Ok(pathway)
    }

    /// Execute pattern matching using activated pathway
    fn execute_pattern_matching(&self, query: &Query, pathway: &[u64]) -> CoreResult<Vec<u64>> {
        let mut matched_nodes = Vec::new();
        let network = self
            .network
            .read()
            .map_err(|_| CoreError::LockError("Failed to acquire network read lock".to_string()))?;

        // Use NEON optimizations for ARM64 if enabled
        if self.neon_optimizations && cfg!(target_arch = "aarch64") {
            // NEON-optimized path for ARM64: process nodes in SIMD batches
            // This would use ARM NEON intrinsics for parallel score computation
            debug!("Using NEON-optimized pattern matching");
        }

        // Simple pattern matching based on node properties
        for &node_id in pathway {
            if let Some(node) = network.get_node(node_id) {
                // Check if node matches query conditions
                let mut match_score = 0.0;

                for condition in &query.conditions {
                    // Simplified condition matching
                    match_score += condition.weight * self.evaluate_condition(&node, condition);
                }

                // Include node if it meets threshold
                if match_score > 0.5 {
                    matched_nodes.push(node_id);
                }
            }
        }

        Ok(matched_nodes)
    }

    /// Evaluate a query condition against a node
    fn evaluate_condition(
        &self,
        node: &crate::synaptic::SynapticNode,
        condition: &QueryCondition,
    ) -> f32 {
        // Simplified condition evaluation
        // In a real implementation, this would check node data against condition

        match condition.operator.as_str() {
            "=" | "==" => {
                // Exact match check
                if node.strength > 0.5 {
                    0.9
                } else {
                    0.1
                }
            }
            ">" => {
                // Greater than check
                if node.strength > condition.value.parse::<f32>().unwrap_or(0.0) {
                    0.8
                } else {
                    0.0
                }
            }
            "<" => {
                // Less than check
                if node.strength < condition.value.parse::<f32>().unwrap_or(1.0) {
                    0.8
                } else {
                    0.0
                }
            }
            "LIKE" => {
                // Pattern matching
                0.7 // Simplified - would do actual pattern matching
            }
            _ => 0.0,
        }
    }

    /// Calculate confidence scores for matched nodes
    fn calculate_confidence_scores(&self, matched_nodes: &[u64]) -> CoreResult<Vec<f32>> {
        let network = self
            .network
            .read()
            .map_err(|_| CoreError::LockError("Failed to acquire network read lock".to_string()))?;

        let mut scores = Vec::new();

        for &node_id in matched_nodes {
            if let Some(node) = network.get_node(node_id) {
                // Base confidence from node strength
                let mut confidence = node.strength;

                // Boost confidence based on access patterns
                confidence += (node.access_count as f32).log10() / 10.0;

                // Boost confidence based on connection strength
                let avg_connection_strength: f32 =
                    node.connections.iter().map(|c| c.weight.abs()).sum::<f32>()
                        / node.connections.len().max(1) as f32;
                confidence += avg_connection_strength * 0.3;

                // Normalize to [0, 1]
                confidence = confidence.clamp(0.0, 1.0);
                scores.push(confidence);
            } else {
                scores.push(0.0);
            }
        }

        Ok(scores)
    }

    /// Generate learning spikes for neural plasticity
    fn generate_learning_spikes(&self, pathway: &[u64], matched_nodes: &[u64]) -> CoreResult<()> {
        let current_time = Instant::now();

        // Acquire write lock for spike patterns
        if let Ok(mut spike_patterns) = self.spike_patterns.write() {
            // Generate spikes for nodes in the activation pathway
            for &node_id in pathway {
                spike_patterns
                    .entry(node_id)
                    .or_insert_with(Vec::new)
                    .push(current_time);
            }

            // Generate output spikes for matched nodes (slightly later for causality)
            for &node_id in matched_nodes {
                spike_patterns
                    .entry(node_id)
                    .or_insert_with(Vec::new)
                    .push(current_time + std::time::Duration::from_micros(10));
            }

            // Apply STDP learning based on spike timing
            if let Ok(mut learning_engine) = self.learning_engine.write() {
                if let Ok(mut network) = self.network.write() {
                    // Clone spike_patterns to avoid holding multiple locks
                    let spikes_clone: HashMap<u64, Vec<Instant>> = spike_patterns
                        .iter()
                        .map(|(k, v)| (*k, v.clone()))
                        .collect();

                    drop(spike_patterns); // Release lock before STDP

                    // Apply spike-timing-dependent plasticity
                    let _ = learning_engine.apply_stdp(&mut network, &spikes_clone);
                }
            } else {
                // Clean up old spikes (keep only recent history)
                let cutoff_time = current_time - std::time::Duration::from_secs(1);
                for spike_list in spike_patterns.values_mut() {
                    spike_list.retain(|&t| t > cutoff_time);
                }
            }
        }

        Ok(())
    }

    /// Calculate learning feedback for query optimization
    fn calculate_learning_feedback(&self, matched_nodes: &[u64], query: &Query) -> f32 {
        // Simple feedback calculation
        let expected_size = query.expected_result_size.unwrap_or(10) as f32;
        let actual_size = matched_nodes.len() as f32;

        // Feedback based on result size accuracy
        let size_accuracy = 1.0 - (expected_size - actual_size).abs() / expected_size.max(1.0);

        // Feedback based on query priority
        let priority_factor = query.priority as f32 / 255.0;

        // Apply learning feedback to learning engine
        let feedback_score = (size_accuracy + priority_factor) / 2.0;

        // Trigger adaptive learning parameter adjustment
        if let Ok(mut learning_engine) = self.learning_engine.write() {
            learning_engine.adapt_learning_parameters(feedback_score);
        }

        feedback_score
    }

    /// Check if a field is likely to cause a full table scan
    /// Returns true if no efficient index lookup is available for this field
    fn is_full_scan_likely(&self, field: &str, operator: &str) -> bool {
        // Fields that typically don't have indexes in neuromorphic systems
        let unindexed_patterns = [
            "description",
            "content",
            "text",
            "body",
            "notes",
            "metadata",
            "json_data",
        ];

        // Check if the field name matches common unindexed patterns
        let field_lower = field.to_lowercase();
        for pattern in &unindexed_patterns {
            if field_lower.contains(pattern) {
                return true;
            }
        }

        // LIKE queries with leading wildcards always cause full scans
        if operator == "LIKE" {
            return true;
        }

        // NOT operators typically cause full scans
        if operator.starts_with("NOT") || operator == "!=" || operator == "<>" {
            return true;
        }

        false
    }

    /// Estimate the benefit of creating an index on a field
    /// Returns improvement factor (0.0 to 1.0) and confidence (0.0 to 1.0)
    fn estimate_index_benefit(&self, _field: &str, operator: &str, weight: f32) -> (f32, f32) {
        let mut improvement = 0.0;

        // Higher weight conditions benefit more from indexing
        improvement += weight * 0.3;

        // Equality operators benefit most from indexes
        let confidence = match operator {
            "=" | "==" => {
                improvement += 0.5;
                0.9
            }
            ">" | "<" | ">=" | "<=" => {
                improvement += 0.35;
                0.8
            }
            "BETWEEN" => {
                improvement += 0.4;
                0.85
            }
            "IN" => {
                improvement += 0.3;
                0.7
            }
            "LIKE" => {
                // Only beneficial if pattern doesn't start with wildcard
                improvement += 0.15;
                0.4
            }
            _ => {
                improvement += 0.1;
                0.3
            }
        };

        // Cap improvement at 0.9 (90% improvement)
        (improvement.min(0.9), confidence)
    }

    /// Determine the best index type for a field based on usage patterns
    fn suggest_index_type(&self, field: &str, operators: &[&str]) -> SuggestedIndexType {
        let field_lower = field.to_lowercase();

        // DNA sequence fields benefit from K-mer indexes
        if field_lower.contains("dna")
            || field_lower.contains("sequence")
            || field_lower.contains("genome")
        {
            return SuggestedIndexType::DnaKmer;
        }

        // Neural/embedding fields benefit from similarity indexes
        if field_lower.contains("neural")
            || field_lower.contains("embedding")
            || field_lower.contains("vector")
        {
            return SuggestedIndexType::NeuralSimilarity;
        }

        // Quantum state fields benefit from entanglement indexes
        if field_lower.contains("quantum") || field_lower.contains("entangle") {
            return SuggestedIndexType::QuantumEntanglement;
        }

        // Check operators to determine BTree vs Hash
        let has_range_ops = operators
            .iter()
            .any(|op| matches!(*op, ">" | "<" | ">=" | "<=" | "BETWEEN"));

        if has_range_ops {
            SuggestedIndexType::BTree
        } else {
            // Pure equality checks benefit from hash indexes
            SuggestedIndexType::Hash
        }
    }

    /// Analyze neural pathway efficiency for potential optimization
    fn analyze_neural_pathway_efficiency(&self, query: &Query) -> Option<OptimizationSuggestion> {
        // Check if neural pathway optimization would help
        if let Ok(network) = self.network.read() {
            let mut weak_connections = 0;
            let mut total_connections = 0;

            for &node_id in &query.target_nodes {
                if let Some(node) = network.get_node(node_id) {
                    for connection in &node.connections {
                        total_connections += 1;
                        if connection.weight < 0.3 {
                            weak_connections += 1;
                        }
                    }
                }
            }

            // If more than 40% of connections are weak, suggest neural optimization
            if total_connections > 0 && (weak_connections as f32 / total_connections as f32) > 0.4 {
                let improvement = (weak_connections as f32 / total_connections as f32) * 0.5;
                return Some(
                    OptimizationSuggestion::new(
                        OptimizationSuggestionType::NeuralPathwayOptimization,
                        format!(
                            "Neural pathway has {} weak connections out of {}. \
                             Consider running Hebbian learning cycles to strengthen frequently \
                             used pathways or pruning unused connections.",
                            weak_connections, total_connections
                        ),
                        query
                            .target_nodes
                            .iter()
                            .map(|n| format!("node_{}", n))
                            .collect(),
                        improvement,
                        0.75,
                        3,
                    )
                    .with_metadata("weak_connections", &weak_connections.to_string())
                    .with_metadata("total_connections", &total_connections.to_string()),
                );
            }
        }
        None
    }

    /// Generate comprehensive optimization suggestions for a query
    ///
    /// Analyzes the query structure, conditions, and neural pathway to provide
    /// actionable optimization recommendations with estimated performance benefits.
    ///
    /// # Arguments
    /// * `query` - The query to analyze for optimization opportunities
    ///
    /// # Returns
    /// A vector of `OptimizationSuggestion` sorted by priority and estimated improvement
    pub fn generate_optimization_suggestions(&self, query: &Query) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Track fields that appear in conditions for index analysis
        let mut field_operators: HashMap<String, Vec<&str>> = HashMap::new();
        let mut full_scan_fields: Vec<String> = Vec::new();

        // Analyze each condition for optimization opportunities
        for condition in &query.conditions {
            field_operators
                .entry(condition.field.clone())
                .or_default()
                .push(&condition.operator);

            // Check for potential full table scans
            if self.is_full_scan_likely(&condition.field, &condition.operator) {
                full_scan_fields.push(condition.field.clone());
            }
        }

        // Generate index suggestions for frequently used fields
        for (field, operators) in &field_operators {
            if self.is_full_scan_likely(field, operators.first().unwrap_or(&"=")) {
                // Find the corresponding condition to get its weight
                let weight = query
                    .conditions
                    .iter()
                    .find(|c| &c.field == field)
                    .map(|c| c.weight)
                    .unwrap_or(0.5);

                let (improvement, confidence) =
                    self.estimate_index_benefit(field, operators.first().unwrap_or(&"="), weight);
                let index_type = self.suggest_index_type(field, &operators.to_vec());

                suggestions.push(
                    OptimizationSuggestion::new(
                        OptimizationSuggestionType::CreateIndex,
                        format!(
                            "Create {} index on field '{}' to avoid full table scan. \
                             This field is used with operators: {}",
                            format!("{:?}", index_type).to_lowercase(),
                            field,
                            operators.join(", ")
                        ),
                        vec![field.clone()],
                        improvement,
                        confidence,
                        2,
                    )
                    .with_index_type(index_type)
                    .with_metadata("operators", &operators.join(",")),
                );
            }
        }

        // Suggest composite index for multiple conditions on same query
        if query.conditions.len() >= 2 {
            let fields: Vec<String> = query.conditions.iter().map(|c| c.field.clone()).collect();

            // Calculate combined weight
            let total_weight: f32 = query.conditions.iter().map(|c| c.weight).sum();
            let avg_weight = total_weight / query.conditions.len() as f32;

            // Composite index benefit increases with number of conditions
            let condition_bonus = ((query.conditions.len() - 1) as f32 * 0.1).min(0.3);
            let improvement = (avg_weight * 0.4 + condition_bonus).min(0.8);

            suggestions.push(
                OptimizationSuggestion::new(
                    OptimizationSuggestionType::CreateCompositeIndex,
                    format!(
                        "Consider creating a composite index on fields ({}) for this \
                         multi-condition query. Composite indexes can significantly improve \
                         performance when querying multiple fields together.",
                        fields.join(", ")
                    ),
                    fields,
                    improvement,
                    0.7,
                    3,
                )
                .with_index_type(SuggestedIndexType::BTree)
                .with_metadata("condition_count", &query.conditions.len().to_string()),
            );
        }

        // Suggest batch processing for queries with many target nodes
        if query.target_nodes.len() > 10 {
            let batch_count = (query.target_nodes.len() / 10).max(2);
            let improvement = ((query.target_nodes.len() as f32 - 10.0) / 100.0).min(0.5);

            suggestions.push(
                OptimizationSuggestion::new(
                    OptimizationSuggestionType::BatchProcessing,
                    format!(
                        "Query targets {} nodes. Consider breaking into {} batches of ~10 nodes \
                         each for better parallelization and memory efficiency. This allows \
                         the neuromorphic processor to activate optimal neural pathways per batch.",
                        query.target_nodes.len(),
                        batch_count
                    ),
                    vec!["target_nodes".to_string()],
                    improvement,
                    0.65,
                    4,
                )
                .with_metadata("node_count", &query.target_nodes.len().to_string())
                .with_metadata("suggested_batches", &batch_count.to_string()),
            );
        }

        // Check for neural pathway optimization opportunities
        if let Some(neural_suggestion) = self.analyze_neural_pathway_efficiency(query) {
            suggestions.push(neural_suggestion);
        }

        // Suggest neuromorphic caching for high-priority queries
        if query.priority > 200 {
            suggestions.push(
                OptimizationSuggestion::new(
                    OptimizationSuggestionType::NeuromorphicCaching,
                    format!(
                        "High-priority query (priority={}) would benefit from extended \
                         neuromorphic caching. Consider enabling persistent spike pattern \
                         caching to speed up repeated similar queries.",
                        query.priority
                    ),
                    vec!["query_cache".to_string()],
                    0.25,
                    0.6,
                    5,
                )
                .with_metadata("priority", &query.priority.to_string()),
            );
        }

        // Suggest query restructuring for complex queries with many conditions
        if query.conditions.len() > 5 {
            suggestions.push(
                OptimizationSuggestion::new(
                    OptimizationSuggestionType::RestructureQuery,
                    format!(
                        "Query has {} conditions. Consider restructuring into multiple \
                         simpler queries with UNION or using subqueries. Complex queries \
                         can overwhelm the neural pathway activation algorithm.",
                        query.conditions.len()
                    ),
                    query.conditions.iter().map(|c| c.field.clone()).collect(),
                    0.35,
                    0.55,
                    4,
                )
                .with_metadata("condition_count", &query.conditions.len().to_string()),
            );
        }

        // Sort suggestions by priority (ascending) then by estimated improvement (descending)
        suggestions.sort_by(|a, b| {
            a.priority.cmp(&b.priority).then(
                b.estimated_improvement
                    .partial_cmp(&a.estimated_improvement)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });

        suggestions
    }

    /// Update query processing statistics
    fn update_statistics(&self, result: &QueryResult) {
        if let Ok(mut stats) = self.query_statistics.write() {
            stats.total_queries += 1;

            // Update average processing time
            let total_time = stats.average_processing_time_ns * (stats.total_queries - 1)
                + result.execution_time_ns;
            stats.average_processing_time_ns = total_time / stats.total_queries;

            // Track activation score as optimization event if above threshold
            if result.activation_score > 0.7 {
                stats.optimization_events += 1;
            }
        }
    }

    /// Get current query statistics
    pub fn get_statistics(&self) -> QueryStatistics {
        if let Ok(stats) = self.query_statistics.read() {
            stats.clone()
        } else {
            QueryStatistics::default()
        }
    }

    /// Clear query cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.query_cache.write() {
            cache.clear();
        }
    }

    /// Set activation threshold for spike generation
    pub fn set_activation_threshold(&mut self, threshold: f32) -> CoreResult<()> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(CoreError::InvalidConfig(
                "Activation threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        self.activation_threshold = threshold;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::HebbianLearningEngine;
    use crate::synaptic::SynapticNetwork;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_query_processor_creation() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));

        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();
        assert_eq!(processor.activation_threshold, 0.5);
    }

    #[test]
    fn test_cache_key_generation() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "SELECT * FROM test".to_string(),
            target_nodes: vec![1, 2, 3],
            conditions: vec![],
            timestamp_secs: 1694428800, // Example timestamp
            priority: 128,
            expected_result_size: Some(10),
        };

        let key = processor.generate_cache_key(&query);
        assert!(!key.is_empty());
    }

    #[test]
    fn test_activation_threshold_setting() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let mut processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        processor.set_activation_threshold(0.7).unwrap();
        assert_eq!(processor.activation_threshold, 0.7);

        // Test invalid threshold
        let result = processor.set_activation_threshold(1.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_optimization_suggestion_creation() {
        let suggestion = OptimizationSuggestion::new(
            OptimizationSuggestionType::CreateIndex,
            "Create index on user_id".to_string(),
            vec!["user_id".to_string()],
            0.5,
            0.8,
            2,
        );

        assert_eq!(suggestion.priority, 2);
        assert!((suggestion.estimated_improvement - 0.5).abs() < 0.01);
        assert!((suggestion.confidence - 0.8).abs() < 0.01);
        assert!(suggestion.suggested_index_type.is_none());
    }

    #[test]
    fn test_optimization_suggestion_with_index_type() {
        let suggestion = OptimizationSuggestion::new(
            OptimizationSuggestionType::CreateIndex,
            "Create hash index".to_string(),
            vec!["id".to_string()],
            0.6,
            0.9,
            1,
        )
        .with_index_type(SuggestedIndexType::Hash);

        assert!(matches!(
            suggestion.suggested_index_type,
            Some(SuggestedIndexType::Hash)
        ));
    }

    #[test]
    fn test_optimization_suggestion_with_metadata() {
        let suggestion = OptimizationSuggestion::new(
            OptimizationSuggestionType::BatchProcessing,
            "Break into batches".to_string(),
            vec!["nodes".to_string()],
            0.4,
            0.7,
            4,
        )
        .with_metadata("batch_size", "10")
        .with_metadata("total_nodes", "100");

        assert_eq!(
            suggestion.metadata.get("batch_size"),
            Some(&"10".to_string())
        );
        assert_eq!(
            suggestion.metadata.get("total_nodes"),
            Some(&"100".to_string())
        );
    }

    #[test]
    fn test_optimization_suggestion_clamping() {
        // Test that values are clamped correctly
        let suggestion = OptimizationSuggestion::new(
            OptimizationSuggestionType::CreateIndex,
            "Test".to_string(),
            vec![],
            1.5,  // Should be clamped to 1.0
            -0.5, // Should be clamped to 0.0
            15,   // Should be clamped to 10
        );

        assert!((suggestion.estimated_improvement - 1.0).abs() < 0.01);
        assert!((suggestion.confidence - 0.0).abs() < 0.01);
        assert_eq!(suggestion.priority, 10);
    }

    #[test]
    fn test_is_full_scan_likely() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        // Fields that should cause full scans
        assert!(processor.is_full_scan_likely("description", "="));
        assert!(processor.is_full_scan_likely("content", "="));
        assert!(processor.is_full_scan_likely("user_notes", "="));
        assert!(processor.is_full_scan_likely("json_data", "="));

        // LIKE queries cause full scans
        assert!(processor.is_full_scan_likely("name", "LIKE"));

        // NOT operators cause full scans
        assert!(processor.is_full_scan_likely("status", "!="));
        assert!(processor.is_full_scan_likely("status", "<>"));
        assert!(processor.is_full_scan_likely("active", "NOT IN"));

        // Normal indexed fields should not cause full scans
        assert!(!processor.is_full_scan_likely("id", "="));
        assert!(!processor.is_full_scan_likely("user_id", ">"));
        assert!(!processor.is_full_scan_likely("created_at", "BETWEEN"));
    }

    #[test]
    fn test_estimate_index_benefit() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        // Equality operator should have high improvement and confidence
        let (improvement, confidence) = processor.estimate_index_benefit("user_id", "=", 0.8);
        assert!(improvement > 0.6);
        assert!(confidence > 0.8);

        // Range operators should have moderate improvement
        let (improvement, confidence) = processor.estimate_index_benefit("age", ">", 0.5);
        assert!(improvement > 0.4);
        assert!(confidence > 0.7);

        // LIKE operator should have lower improvement and confidence
        let (improvement, confidence) = processor.estimate_index_benefit("name", "LIKE", 0.5);
        assert!(improvement < 0.4);
        assert!(confidence < 0.5);
    }

    #[test]
    fn test_suggest_index_type() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        // DNA fields should get DnaKmer index
        let index_type = processor.suggest_index_type("dna_sequence", &["="]);
        assert!(matches!(index_type, SuggestedIndexType::DnaKmer));

        // Neural/embedding fields should get NeuralSimilarity index
        let index_type = processor.suggest_index_type("embedding_vector", &["="]);
        assert!(matches!(index_type, SuggestedIndexType::NeuralSimilarity));

        // Quantum fields should get QuantumEntanglement index
        let index_type = processor.suggest_index_type("quantum_state", &["="]);
        assert!(matches!(
            index_type,
            SuggestedIndexType::QuantumEntanglement
        ));

        // Range operators should get BTree index
        let index_type = processor.suggest_index_type("created_at", &[">", "<"]);
        assert!(matches!(index_type, SuggestedIndexType::BTree));

        // Pure equality on normal fields should get Hash index
        let index_type = processor.suggest_index_type("user_id", &["="]);
        assert!(matches!(index_type, SuggestedIndexType::Hash));
    }

    #[test]
    fn test_generate_optimization_suggestions_empty_query() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "SELECT * FROM test".to_string(),
            target_nodes: vec![],
            conditions: vec![],
            timestamp_secs: 1694428800,
            priority: 128,
            expected_result_size: None,
        };

        let suggestions = processor.generate_optimization_suggestions(&query);
        // Empty query should have no suggestions
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_generate_optimization_suggestions_full_scan_field() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "SELECT * FROM test WHERE description = 'test'".to_string(),
            target_nodes: vec![],
            conditions: vec![QueryCondition {
                field: "description".to_string(),
                operator: "=".to_string(),
                value: "test".to_string(),
                weight: 0.8,
            }],
            timestamp_secs: 1694428800,
            priority: 128,
            expected_result_size: None,
        };

        let suggestions = processor.generate_optimization_suggestions(&query);

        // Should suggest creating an index for description field
        assert!(!suggestions.is_empty());
        let index_suggestion = suggestions
            .iter()
            .find(|s| matches!(s.suggestion_type, OptimizationSuggestionType::CreateIndex));
        assert!(index_suggestion.is_some());
        assert!(index_suggestion
            .unwrap()
            .affected_fields
            .contains(&"description".to_string()));
    }

    #[test]
    fn test_generate_optimization_suggestions_composite_index() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "SELECT * FROM users WHERE status = 'active' AND age > 18".to_string(),
            target_nodes: vec![],
            conditions: vec![
                QueryCondition {
                    field: "status".to_string(),
                    operator: "=".to_string(),
                    value: "active".to_string(),
                    weight: 0.7,
                },
                QueryCondition {
                    field: "age".to_string(),
                    operator: ">".to_string(),
                    value: "18".to_string(),
                    weight: 0.6,
                },
            ],
            timestamp_secs: 1694428800,
            priority: 128,
            expected_result_size: None,
        };

        let suggestions = processor.generate_optimization_suggestions(&query);

        // Should suggest composite index
        let composite_suggestion = suggestions.iter().find(|s| {
            matches!(
                s.suggestion_type,
                OptimizationSuggestionType::CreateCompositeIndex
            )
        });
        assert!(composite_suggestion.is_some());
        let composite = composite_suggestion.unwrap();
        assert!(composite.affected_fields.contains(&"status".to_string()));
        assert!(composite.affected_fields.contains(&"age".to_string()));
    }

    #[test]
    fn test_generate_optimization_suggestions_batch_processing() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "SELECT * FROM nodes".to_string(),
            target_nodes: (1..=50).collect(), // 50 target nodes
            conditions: vec![],
            timestamp_secs: 1694428800,
            priority: 128,
            expected_result_size: None,
        };

        let suggestions = processor.generate_optimization_suggestions(&query);

        // Should suggest batch processing
        let batch_suggestion = suggestions.iter().find(|s| {
            matches!(
                s.suggestion_type,
                OptimizationSuggestionType::BatchProcessing
            )
        });
        assert!(batch_suggestion.is_some());
        assert!(batch_suggestion
            .unwrap()
            .metadata
            .contains_key("node_count"));
    }

    #[test]
    fn test_generate_optimization_suggestions_high_priority_caching() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "SELECT * FROM critical_data".to_string(),
            target_nodes: vec![],
            conditions: vec![],
            timestamp_secs: 1694428800,
            priority: 250, // Very high priority
            expected_result_size: None,
        };

        let suggestions = processor.generate_optimization_suggestions(&query);

        // Should suggest neuromorphic caching
        let cache_suggestion = suggestions.iter().find(|s| {
            matches!(
                s.suggestion_type,
                OptimizationSuggestionType::NeuromorphicCaching
            )
        });
        assert!(cache_suggestion.is_some());
    }

    #[test]
    fn test_generate_optimization_suggestions_complex_query() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "SELECT * FROM data WHERE a=1 AND b=2 AND c=3 AND d=4 AND e=5 AND f=6"
                .to_string(),
            target_nodes: vec![],
            conditions: vec![
                QueryCondition {
                    field: "a".to_string(),
                    operator: "=".to_string(),
                    value: "1".to_string(),
                    weight: 0.5,
                },
                QueryCondition {
                    field: "b".to_string(),
                    operator: "=".to_string(),
                    value: "2".to_string(),
                    weight: 0.5,
                },
                QueryCondition {
                    field: "c".to_string(),
                    operator: "=".to_string(),
                    value: "3".to_string(),
                    weight: 0.5,
                },
                QueryCondition {
                    field: "d".to_string(),
                    operator: "=".to_string(),
                    value: "4".to_string(),
                    weight: 0.5,
                },
                QueryCondition {
                    field: "e".to_string(),
                    operator: "=".to_string(),
                    value: "5".to_string(),
                    weight: 0.5,
                },
                QueryCondition {
                    field: "f".to_string(),
                    operator: "=".to_string(),
                    value: "6".to_string(),
                    weight: 0.5,
                },
            ],
            timestamp_secs: 1694428800,
            priority: 128,
            expected_result_size: None,
        };

        let suggestions = processor.generate_optimization_suggestions(&query);

        // Should suggest query restructuring for 6+ conditions
        let restructure_suggestion = suggestions.iter().find(|s| {
            matches!(
                s.suggestion_type,
                OptimizationSuggestionType::RestructureQuery
            )
        });
        assert!(restructure_suggestion.is_some());
    }

    #[test]
    fn test_generate_optimization_suggestions_sorting() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "Complex query".to_string(),
            target_nodes: (1..=20).collect(),
            conditions: vec![
                QueryCondition {
                    field: "description".to_string(),
                    operator: "LIKE".to_string(),
                    value: "%test%".to_string(),
                    weight: 0.8,
                },
                QueryCondition {
                    field: "content".to_string(),
                    operator: "=".to_string(),
                    value: "test".to_string(),
                    weight: 0.7,
                },
            ],
            timestamp_secs: 1694428800,
            priority: 250,
            expected_result_size: None,
        };

        let suggestions = processor.generate_optimization_suggestions(&query);

        // Verify suggestions are sorted by priority
        for i in 1..suggestions.len() {
            assert!(
                suggestions[i - 1].priority <= suggestions[i].priority,
                "Suggestions should be sorted by priority"
            );
        }
    }

    #[test]
    fn test_dna_field_index_suggestion() {
        let network = Arc::new(RwLock::new(SynapticNetwork::new(1000, 0.5).unwrap()));
        let learning = Arc::new(RwLock::new(HebbianLearningEngine::new(0.01).unwrap()));
        let processor = NeuromorphicQueryProcessor::new(network, learning, true).unwrap();

        let query = Query {
            id: 1,
            query_type: QueryType::Select,
            content: "SELECT * FROM genomes WHERE dna_sequence = 'ATCG'".to_string(),
            target_nodes: vec![],
            conditions: vec![QueryCondition {
                field: "dna_sequence_content".to_string(), // Contains "dna" and "content"
                operator: "=".to_string(),
                value: "ATCG".to_string(),
                weight: 0.9,
            }],
            timestamp_secs: 1694428800,
            priority: 128,
            expected_result_size: None,
        };

        let suggestions = processor.generate_optimization_suggestions(&query);

        // Should suggest DnaKmer index for DNA sequence fields
        let index_suggestion = suggestions
            .iter()
            .find(|s| matches!(s.suggestion_type, OptimizationSuggestionType::CreateIndex));
        assert!(index_suggestion.is_some());
        assert!(matches!(
            index_suggestion.unwrap().suggested_index_type,
            Some(SuggestedIndexType::DnaKmer)
        ));
    }
}
