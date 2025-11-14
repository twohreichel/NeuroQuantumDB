//! # Synaptic Index Networks (SINs)
//!
//! Core synaptic data structures implementing neuromorphic computing principles
//! for self-optimizing data organization and intelligent indexing.

use crate::error::{CoreError, CoreResult};
use crate::neon_optimization::NeonOptimizer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;
use tracing::{debug, info, instrument, warn};

/// Helper function for serde default with Instant
fn instant_now() -> Instant {
    Instant::now()
}

/// Types of synaptic connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    Excitatory,
    Inhibitory,
    Modulatory,
}

/// Neuron activation functions for neuromorphic processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationFunction {
    Sigmoid,
    ReLU,
    Tanh,
    Linear,
    LeakyReLU,
}

impl ActivationFunction {
    /// Apply the activation function to an input value
    pub fn activate(&self, x: f32) -> f32 {
        match self {
            ActivationFunction::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            ActivationFunction::ReLU => x.max(0.0),
            ActivationFunction::Tanh => x.tanh(),
            ActivationFunction::Linear => x,
            ActivationFunction::LeakyReLU => {
                if x > 0.0 {
                    x
                } else {
                    0.01 * x
                }
            }
        }
    }

    /// Derivative of the activation function for backpropagation
    pub fn derivative(&self, x: f32) -> f32 {
        match self {
            ActivationFunction::Sigmoid => {
                let s = self.activate(x);
                s * (1.0 - s)
            }
            ActivationFunction::ReLU => {
                if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            ActivationFunction::Tanh => 1.0 - x.tanh().powi(2),
            ActivationFunction::Linear => 1.0,
            ActivationFunction::LeakyReLU => {
                if x > 0.0 {
                    1.0
                } else {
                    0.01
                }
            }
        }
    }
}

/// Advanced neuron structure with full activation capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neuron {
    pub id: u64,
    pub activation: f32,
    pub bias: f32,
    pub activation_function: ActivationFunction,
    pub input_sum: f32,
    pub output: f32,
    #[serde(skip, default)]
    pub last_spike_time: Option<Instant>,
    pub refractory_period_ms: u64,
    pub threshold: f32,
    pub learning_rate: f32,
}

impl Neuron {
    /// Create a new neuron with specified activation function
    pub fn new(id: u64, activation_function: ActivationFunction) -> Self {
        Self {
            id,
            activation: 0.0,
            bias: 0.0,
            activation_function,
            input_sum: 0.0,
            output: 0.0,
            last_spike_time: None,
            refractory_period_ms: 5,
            threshold: 0.5,
            learning_rate: 0.01,
        }
    }

    /// Calculate neuron output based on inputs
    pub fn activate(&mut self, weighted_inputs: f32) -> f32 {
        self.input_sum = weighted_inputs + self.bias;
        self.activation = self.input_sum;
        self.output = self.activation_function.activate(self.input_sum);
        self.output
    }

    /// Check if neuron can fire (not in refractory period)
    pub fn can_fire(&self) -> bool {
        if let Some(last_spike) = self.last_spike_time {
            let elapsed = Instant::now().duration_since(last_spike);
            elapsed.as_millis() > self.refractory_period_ms as u128
        } else {
            true
        }
    }

    /// Fire the neuron if threshold is exceeded
    pub fn fire(&mut self) -> bool {
        if self.output > self.threshold && self.can_fire() {
            self.last_spike_time = Some(Instant::now());
            true
        } else {
            false
        }
    }
}

/// Synapse structure with full plasticity support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Synapse {
    pub pre_neuron: u64,
    pub post_neuron: u64,
    pub weight: f32,
    pub plasticity_factor: f32,
    pub eligibility_trace: f32,
    #[serde(skip, default = "instant_now")]
    pub last_update: Instant,
    pub connection_type: ConnectionType,
}

impl Synapse {
    /// Create a new synapse
    pub fn new(pre_neuron: u64, post_neuron: u64, initial_weight: f32) -> Self {
        Self {
            pre_neuron,
            post_neuron,
            weight: initial_weight,
            plasticity_factor: 1.0,
            eligibility_trace: 0.0,
            last_update: Instant::now(),
            connection_type: ConnectionType::Excitatory,
        }
    }

    /// Update synaptic weight using Hebbian rule
    pub fn hebbian_update(&mut self, pre_activity: f32, post_activity: f32, learning_rate: f32) {
        // "Neurons that fire together, wire together"
        let weight_change = learning_rate * pre_activity * post_activity * self.plasticity_factor;
        self.weight += weight_change;

        // Bound weights to prevent explosion
        self.weight = self.weight.clamp(-2.0, 2.0);

        // Update plasticity factor based on usage
        if weight_change.abs() > 0.01 {
            self.plasticity_factor = (self.plasticity_factor * 1.05).min(2.0);
        } else {
            self.plasticity_factor = (self.plasticity_factor * 0.99).max(0.1);
        }

        self.last_update = Instant::now();
    }

    /// Update eligibility trace for temporal credit assignment
    pub fn update_eligibility_trace(&mut self, pre_activity: f32, post_activity: f32, decay: f32) {
        self.eligibility_trace = self.eligibility_trace * decay + pre_activity * post_activity;
    }
}

/// Individual synaptic node representing a data point or index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynapticNode {
    pub id: u64,
    pub strength: f32,
    pub connections: Vec<SynapticConnection>,
    #[serde(skip, default = "instant_now")]
    pub last_access: Instant,
    #[serde(skip, default = "instant_now")]
    pub last_decay: Instant, // Track when decay was last applied
    pub access_count: u64,
    pub data_payload: Vec<u8>,
    pub activation_level: f32,
    pub learning_rate: f32,
    pub decay_factor: f32,
    pub decay_tau_ms: f32, // Time constant for exponential decay (in milliseconds)
}

/// Synaptic connection between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynapticConnection {
    pub target_id: u64,
    pub weight: f32,
    pub connection_type: ConnectionType,
    #[serde(skip, default = "instant_now")]
    pub last_strengthened: Instant,
    pub usage_count: u64,
    pub plasticity_factor: f32,
}

impl SynapticNode {
    /// Create a new synaptic node
    pub fn new(id: u64) -> Self {
        Self {
            id,
            strength: 0.0,
            connections: Vec::new(),
            last_access: Instant::now(),
            last_decay: Instant::now(),
            access_count: 0,
            data_payload: Vec::new(),
            activation_level: 0.0,
            learning_rate: 0.01,
            decay_factor: 0.99,
            decay_tau_ms: 60_000.0, // 1 minute time constant (biologically realistic for STM)
        }
    }

    /// Create a node with data payload
    pub fn with_data(id: u64, data: Vec<u8>) -> Self {
        Self {
            id,
            strength: 0.0,
            connections: Vec::new(),
            last_access: Instant::now(),
            last_decay: Instant::now(),
            access_count: 0,
            data_payload: data,
            activation_level: 0.0,
            learning_rate: 0.01,
            decay_factor: 0.99,
            decay_tau_ms: 60_000.0, // 1 minute time constant
        }
    }

    /// Strengthen the node based on access
    #[instrument(level = "debug", skip(self))]
    pub fn strengthen(&mut self, amount: f32) {
        self.strength += amount * self.learning_rate;
        self.strength = self.strength.min(1.0); // Cap at 1.0
        self.last_access = Instant::now();
        self.access_count += 1;

        debug!("Node {} strengthened to {}", self.id, self.strength);
    }

    /// Apply natural decay to simulate forgetting
    /// Uses exponential decay: weight(t) = weight(0) * exp(-dt/τ)
    /// where τ is the time constant (decay_tau_ms)
    pub fn apply_decay(&mut self) {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_decay).as_millis() as f32;

        if elapsed_ms > 0.0 {
            // Exponential decay formula: exp(-t/τ)
            let decay_multiplier = (-elapsed_ms / self.decay_tau_ms).exp();

            self.strength *= decay_multiplier;
            self.activation_level *= decay_multiplier;

            // Update last decay time
            self.last_decay = now;
        }
    }

    /// Apply decay with custom time constant (for LTP vs LTD)
    /// LTP (Long-Term Potentiation): τ ≈ hours to days
    /// LTD (Long-Term Depression): τ ≈ minutes
    pub fn apply_decay_with_tau(&mut self, tau_ms: f32) {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_decay).as_millis() as f32;

        if elapsed_ms > 0.0 {
            let decay_multiplier = (-elapsed_ms / tau_ms).exp();
            self.strength *= decay_multiplier;
            self.activation_level *= decay_multiplier;
            self.last_decay = now;
        }
    }

    /// Add a connection to another node
    pub fn add_connection(
        &mut self,
        target_id: u64,
        weight: f32,
        connection_type: ConnectionType,
    ) -> CoreResult<()> {
        // Check if connection already exists
        if self.connections.iter().any(|c| c.target_id == target_id) {
            return Err(CoreError::InvalidOperation(format!(
                "Connection to node {} already exists",
                target_id
            )));
        }

        let connection = SynapticConnection {
            target_id,
            weight,
            connection_type,
            last_strengthened: Instant::now(),
            usage_count: 0,
            plasticity_factor: 1.0,
        };

        self.connections.push(connection);
        Ok(())
    }

    /// Get memory usage of this node
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.connections.len() * std::mem::size_of::<SynapticConnection>()
            + self.data_payload.len()
    }
}

/// Synaptic network managing collections of nodes and their relationships
#[derive(Debug)]
pub struct SynapticNetwork {
    nodes: RwLock<HashMap<u64, SynapticNode>>,
    neurons: RwLock<HashMap<u64, Neuron>>,
    synapses: RwLock<Vec<Synapse>>,
    max_nodes: usize,
    activation_threshold: f32,
    learning_rate: f32,
    plasticity_threshold: f32,
    total_connections: RwLock<usize>,
    memory_usage: RwLock<usize>,
    query_patterns: RwLock<HashMap<String, QueryPattern>>,
    #[allow(dead_code)]
    neon_optimizer: Option<NeonOptimizer>,
}

/// Query pattern structure for learning and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPattern {
    pub pattern_hash: String,
    pub access_count: u64,
    pub avg_execution_time_ms: f32,
    pub optimal_neurons: Vec<u64>,
    pub synaptic_pathway: Vec<u64>,
    pub performance_score: f32,
    #[serde(skip, default = "instant_now")]
    pub last_accessed: Instant,
}

impl SynapticNetwork {
    /// Create a new synaptic network
    pub fn new(max_nodes: usize, activation_threshold: f32) -> CoreResult<Self> {
        // Validate parameters
        if max_nodes == 0 {
            return Err(CoreError::InvalidConfig(
                "max_nodes must be greater than 0".to_string(),
            ));
        }

        if !(0.0..=1.0).contains(&activation_threshold) {
            return Err(CoreError::InvalidConfig(
                "activation_threshold must be between 0.0 and 1.0".to_string(),
            ));
        }

        let neon_optimizer = if cfg!(target_arch = "aarch64") {
            Some(NeonOptimizer::new()?)
        } else {
            None
        };

        Ok(Self {
            nodes: RwLock::new(HashMap::with_capacity(max_nodes.min(1000))),
            neurons: RwLock::new(HashMap::new()),
            synapses: RwLock::new(Vec::new()),
            max_nodes,
            activation_threshold,
            learning_rate: 0.01,
            plasticity_threshold: 0.1,
            total_connections: RwLock::new(0),
            memory_usage: RwLock::new(0),
            query_patterns: RwLock::new(HashMap::new()),
            neon_optimizer,
        })
    }

    /// Hebbian learning update - "Neurons that fire together, wire together"
    pub fn hebbian_update(&self, _input_pattern: &[f32], _target_output: &[f32]) -> CoreResult<()> {
        let mut synapses = self.synapses.write().unwrap();
        let neurons = self.neurons.read().unwrap();

        for synapse in synapses.iter_mut() {
            if let (Some(pre_neuron), Some(post_neuron)) = (
                neurons.get(&synapse.pre_neuron),
                neurons.get(&synapse.post_neuron),
            ) {
                let pre_activity = pre_neuron.activation;
                let post_activity = post_neuron.activation;

                synapse.hebbian_update(pre_activity, post_activity, self.learning_rate);

                // Apply synaptic plasticity strengthening
                let weight_change = self.learning_rate * pre_activity * post_activity;
                if weight_change.abs() > self.plasticity_threshold {
                    synapse.plasticity_factor *= 1.1;
                    synapse.plasticity_factor = synapse.plasticity_factor.min(2.0);
                }
            }
        }

        debug!("Hebbian update applied to {} synapses", synapses.len());
        Ok(())
    }

    /// Adapt query pattern recognition based on performance feedback
    pub fn adapt_query_pattern(
        &self,
        query_embedding: &[f32],
        performance_metric: f32,
    ) -> CoreResult<()> {
        let query_hash = self.hash_query_pattern(query_embedding);
        let mut patterns = self.query_patterns.write().unwrap();

        let pattern = patterns
            .entry(query_hash.clone())
            .or_insert_with(|| QueryPattern {
                pattern_hash: query_hash.clone(),
                access_count: 0,
                avg_execution_time_ms: 0.0,
                optimal_neurons: Vec::new(),
                synaptic_pathway: Vec::new(),
                performance_score: 0.0,
                last_accessed: Instant::now(),
            });

        // Update pattern statistics
        pattern.access_count += 1;
        pattern.performance_score = (pattern.performance_score * 0.9) + (performance_metric * 0.1);
        pattern.last_accessed = Instant::now();

        // Find optimal neurons for this pattern
        pattern.optimal_neurons = self.find_optimal_neurons_for_pattern(query_embedding)?;

        // Strengthen synaptic pathways that perform well
        if performance_metric > 0.7 {
            self.strengthen_pathway(&pattern.optimal_neurons)?;
        }

        debug!(
            "Adapted query pattern {} with performance score {}",
            query_hash, pattern.performance_score
        );

        Ok(())
    }

    /// Hash query pattern for tracking
    fn hash_query_pattern(&self, embedding: &[f32]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for &val in embedding {
            ((val * 1000.0) as i32).hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }

    /// Find optimal neurons for a query pattern
    fn find_optimal_neurons_for_pattern(&self, embedding: &[f32]) -> CoreResult<Vec<u64>> {
        let neurons = self.neurons.read().unwrap();
        let mut scored_neurons: Vec<(u64, f32)> = Vec::new();

        for (id, neuron) in neurons.iter() {
            // Calculate activation score based on embedding
            let score = self.calculate_neuron_activation_score(neuron, embedding);
            if score > self.activation_threshold {
                scored_neurons.push((*id, score));
            }
        }

        // Sort by score and take top neurons
        scored_neurons.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(scored_neurons.iter().take(10).map(|(id, _)| *id).collect())
    }

    /// Calculate neuron activation score for pattern matching
    fn calculate_neuron_activation_score(&self, neuron: &Neuron, embedding: &[f32]) -> f32 {
        // Simple dot product with neuron's current state
        let base_score = neuron.activation * embedding.iter().sum::<f32>() / embedding.len() as f32;
        base_score.abs()
    }

    /// Strengthen synaptic pathway for frequently used routes
    fn strengthen_pathway(&self, neuron_ids: &[u64]) -> CoreResult<()> {
        let mut synapses = self.synapses.write().unwrap();

        for window in neuron_ids.windows(2) {
            let (pre_id, post_id) = (window[0], window[1]);

            // Find and strengthen synapses in this pathway
            for synapse in synapses.iter_mut() {
                if synapse.pre_neuron == pre_id && synapse.post_neuron == post_id {
                    synapse.weight *= 1.1;
                    synapse.weight = synapse.weight.min(2.0);
                    synapse.plasticity_factor = (synapse.plasticity_factor * 1.05).min(2.0);
                }
            }
        }

        Ok(())
    }

    /// Add a neuron to the network
    pub fn add_neuron(&self, neuron: Neuron) -> CoreResult<()> {
        let mut neurons = self.neurons.write().unwrap();

        if neurons.contains_key(&neuron.id) {
            return Err(CoreError::InvalidOperation(format!(
                "Neuron with ID {} already exists",
                neuron.id
            )));
        }

        neurons.insert(neuron.id, neuron);
        Ok(())
    }

    /// Add a synapse connecting two neurons
    pub fn add_synapse(&self, synapse: Synapse) -> CoreResult<()> {
        let mut synapses = self.synapses.write().unwrap();
        let mut total_connections = self.total_connections.write().unwrap();

        synapses.push(synapse);
        *total_connections += 1;

        Ok(())
    }

    /// Forward propagation through the network
    pub fn forward_propagate(&self, inputs: &[f32]) -> CoreResult<Vec<f32>> {
        let mut neurons = self.neurons.write().unwrap();
        let synapses = self.synapses.read().unwrap();

        // Reset neuron inputs
        for neuron in neurons.values_mut() {
            neuron.input_sum = 0.0;
        }

        // Set input layer activations
        for (i, &input_val) in inputs.iter().enumerate() {
            if let Some(neuron) = neurons.get_mut(&(i as u64)) {
                neuron.activation = input_val;
                neuron.output = input_val;
            }
        }

        // Propagate through synapses
        for synapse in synapses.iter() {
            if let Some(pre_neuron) = neurons.get(&synapse.pre_neuron) {
                let weighted_input = pre_neuron.output * synapse.weight;

                if let Some(post_neuron) = neurons.get_mut(&synapse.post_neuron) {
                    post_neuron.input_sum += weighted_input;
                }
            }
        }

        // Apply activation functions
        let mut outputs = Vec::new();
        for neuron in neurons.values_mut() {
            neuron.output = neuron.activation_function.activate(neuron.input_sum);
            neuron.activation = neuron.output;
            outputs.push(neuron.output);
        }

        Ok(outputs)
    }

    /// Select optimal index based on learned patterns
    pub fn select_adaptive_index(&self, query_embedding: &[f32]) -> CoreResult<Option<String>> {
        let patterns = self.query_patterns.read().unwrap();
        let query_hash = self.hash_query_pattern(query_embedding);

        // Check if we have learned an optimal pattern for this query
        if let Some(pattern) = patterns.get(&query_hash) {
            if pattern.performance_score > 0.7 {
                // Return suggested index based on optimal neurons
                let index_name = format!(
                    "neuro_index_{}",
                    pattern.optimal_neurons.first().unwrap_or(&0)
                );
                debug!("Selected adaptive index: {}", index_name);
                return Ok(Some(index_name));
            }
        }

        Ok(None)
    }

    /// Implement long-term potentiation for memory consolidation
    pub fn apply_long_term_potentiation(
        &self,
        activation_pairs: &[(u64, u64, f32)],
    ) -> CoreResult<()> {
        let mut synapses = self.synapses.write().unwrap();

        for &(pre_id, post_id, correlation) in activation_pairs {
            for synapse in synapses.iter_mut() {
                if synapse.pre_neuron == pre_id && synapse.post_neuron == post_id {
                    // LTP: Strengthen frequently co-activated connections
                    let ltp_factor = 1.5;
                    synapse.weight += correlation * ltp_factor * self.learning_rate;
                    synapse.weight = synapse.weight.clamp(-2.0, 2.0);
                    synapse.plasticity_factor = (synapse.plasticity_factor * 1.2).min(2.0);

                    debug!(
                        "Applied LTP to synapse {}->{}: weight = {:.3}",
                        pre_id, post_id, synapse.weight
                    );
                }
            }
        }

        info!(
            "Applied long-term potentiation to {} synaptic pairs",
            activation_pairs.len()
        );
        Ok(())
    }

    /// Consolidate memory by strengthening important pathways
    pub fn consolidate_memory(&self, importance_threshold: f32) -> CoreResult<()> {
        let patterns = self.query_patterns.read().unwrap();
        let mut ltp_pairs = Vec::new();

        // Identify important patterns for consolidation
        for pattern in patterns.values() {
            if pattern.performance_score > importance_threshold {
                // Create LTP pairs from optimal pathway
                for window in pattern.optimal_neurons.windows(2) {
                    ltp_pairs.push((window[0], window[1], pattern.performance_score));
                }
            }
        }

        // Apply LTP to consolidate these patterns
        self.apply_long_term_potentiation(&ltp_pairs)?;

        info!("Consolidated {} important memory patterns", patterns.len());

        Ok(())
    }

    /// Add a node to the network
    #[instrument(level = "debug", skip(self, node))]
    pub fn add_node(&self, node: SynapticNode) -> CoreResult<()> {
        let mut nodes = self.nodes.write().unwrap();

        if nodes.len() >= self.max_nodes {
            return Err(CoreError::ResourceExhausted(format!(
                "Maximum nodes ({}) exceeded",
                self.max_nodes
            )));
        }

        if nodes.contains_key(&node.id) {
            return Err(CoreError::InvalidOperation(format!(
                "Node with ID {} already exists",
                node.id
            )));
        }

        let mut memory_usage = self.memory_usage.write().unwrap();
        *memory_usage += node.memory_usage();
        nodes.insert(node.id, node);

        debug!("Added node to network, total nodes: {}", nodes.len());
        Ok(())
    }

    /// Store data in the network and return an ID
    pub async fn store_data(&self, data: crate::dna::EncodedData) -> CoreResult<String> {
        // Generate a new node ID
        let node_id = self.nodes.read().unwrap().len() as u64 + 1;

        // Create a new node with the encoded data
        let mut node = SynapticNode::new(node_id);
        // Convert DNASequence to Vec<u8> by serializing it
        node.data_payload = serde_json::to_vec(&data.sequence)
            .map_err(crate::error::NeuroQuantumError::JsonError)?;

        // Add the node to the network
        self.add_node(node)?;

        Ok(node_id.to_string())
    }

    /// Optimize the network structure with timeout and performance improvements
    pub async fn optimize_network(&self) -> CoreResult<()> {
        let start_time = Instant::now();
        let timeout = std::time::Duration::from_millis(5000); // 5 second timeout

        debug!("Starting network optimization with timeout {:?}", timeout);

        // Apply decay to all nodes (fast operation)
        self.apply_global_decay();

        // Collect weak connections in batches to avoid holding locks too long
        let weak_connections = self.collect_weak_connections(0.01)?;

        if start_time.elapsed() > timeout {
            warn!("Network optimization timed out during weak connection collection");
            return Ok(());
        }

        // Remove weak connections in optimized batches
        self.remove_connections_batch(&weak_connections)?;

        // Update memory usage efficiently
        self.update_memory_usage_cache();

        let elapsed = start_time.elapsed();
        tracing::info!("Network optimization completed in {:?}", elapsed);
        Ok(())
    }

    /// Collect weak connections efficiently without holding long locks
    fn collect_weak_connections(&self, threshold: f32) -> CoreResult<Vec<(u64, usize)>> {
        let mut weak_connections = Vec::new();

        // Use read lock and process in chunks to avoid long lock times
        let nodes = self.nodes.read().unwrap();

        for (node_id, node) in nodes.iter() {
            for (i, connection) in node.connections.iter().enumerate() {
                if connection.weight.abs() < threshold {
                    weak_connections.push((*node_id, i));
                }
            }

            // Limit the number of connections we check per optimization cycle
            if weak_connections.len() > 1000 {
                break;
            }
        }

        Ok(weak_connections)
    }

    /// Remove connections in optimized batches
    fn remove_connections_batch(&self, connections_to_remove: &[(u64, usize)]) -> CoreResult<()> {
        if connections_to_remove.is_empty() {
            return Ok(());
        }

        // Group removals by node to minimize lock overhead
        let mut removals_by_node: HashMap<u64, Vec<usize>> = HashMap::new();
        for &(node_id, connection_idx) in connections_to_remove {
            removals_by_node
                .entry(node_id)
                .or_default()
                .push(connection_idx);
        }

        let mut nodes = self.nodes.write().unwrap();
        let mut total_connections = self.total_connections.write().unwrap();
        let mut removed_count = 0;

        for (node_id, mut indices) in removals_by_node {
            if let Some(node) = nodes.get_mut(&node_id) {
                // Sort indices in descending order to avoid index shifts during removal
                indices.sort_by(|a, b| b.cmp(a));

                for &idx in &indices {
                    if idx < node.connections.len() {
                        node.connections.remove(idx);
                        removed_count += 1;
                    }
                }
            }
        }

        *total_connections = total_connections.saturating_sub(removed_count);
        debug!("Removed {} weak connections", removed_count);

        Ok(())
    }

    /// Update memory usage cache efficiently
    fn update_memory_usage_cache(&self) {
        let nodes = self.nodes.read().unwrap();
        let total_memory: usize = nodes.values().map(|node| node.memory_usage()).sum();

        if let Ok(mut memory_usage) = self.memory_usage.write() {
            *memory_usage = total_memory;
        }
    }

    /// Apply decay to all nodes (simulating natural forgetting)
    pub fn apply_global_decay(&self) {
        let mut nodes = self.nodes.write().unwrap();
        for node in nodes.values_mut() {
            node.apply_decay();
        }
    }

    /// Process query using synaptic network
    pub async fn process_query(
        &self,
        query: &crate::query::Query,
    ) -> CoreResult<crate::query::QueryResult> {
        use crate::query::QueryResult;

        let start_time = std::time::Instant::now();
        let mut activated_nodes = Vec::new();
        let mut total_activation = 0.0;

        // Find nodes that match the query pattern
        for (node_id, node) in self.nodes.read().unwrap().iter() {
            let match_score = self.calculate_match_score(node, &query.content);

            if match_score > self.activation_threshold {
                activated_nodes.push(*node_id);
                total_activation += match_score;
            }
        }

        let execution_time = start_time.elapsed();

        Ok(QueryResult {
            query_id: query.id,
            matched_nodes: activated_nodes,
            execution_time_ns: execution_time.as_nanos() as u64,
            activation_score: total_activation,
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Calculate match score between node and query
    fn calculate_match_score(&self, node: &SynapticNode, query_content: &str) -> f32 {
        if node.data_payload.is_empty() {
            return 0.0;
        }

        let node_content = String::from_utf8_lossy(&node.data_payload);
        let query_bytes = query_content.as_bytes();
        let node_bytes = node_content.as_bytes();

        // Simple pattern matching with boost from node strength
        let mut matches = 0;
        let mut total_comparisons = 0;

        for window in node_bytes.windows(query_bytes.len()) {
            total_comparisons += 1;
            if window == query_bytes {
                matches += 1;
            }
        }

        if total_comparisons == 0 {
            return 0.0;
        }

        let base_score = matches as f32 / total_comparisons as f32;
        base_score * (1.0 + node.strength) // Boost by node strength
    }

    /// Get the number of nodes in the network
    pub fn node_count(&self) -> usize {
        self.nodes.read().unwrap().len()
    }

    /// Get a reference to a node
    pub fn get_node(&self, node_id: u64) -> Option<SynapticNode> {
        self.nodes.read().unwrap().get(&node_id).cloned()
    }

    /// Get a mutable reference to a node
    pub fn get_node_mut(&self, node_id: u64) -> Option<()> {
        // For thread safety, we can't return a mutable reference directly
        // Instead, we provide a way to check if the node exists
        self.nodes
            .read()
            .unwrap()
            .contains_key(&node_id)
            .then_some(())
    }

    /// Optimize query using neuromorphic principles
    pub async fn optimize_query(&self, query: &str) -> CoreResult<String> {
        // Simple query optimization - in production this would be more sophisticated
        let optimized = query.to_lowercase().trim().to_string();

        // Record query patterns for learning
        // In a real implementation, this would update synaptic weights

        Ok(optimized)
    }

    /// Strengthen neural pathways for a given query
    pub async fn strengthen_pathways_for_query(&self, query: &str) -> CoreResult<()> {
        let mut nodes = self.nodes.write().unwrap();

        // Find nodes that match the query and strengthen them
        for node in nodes.values_mut() {
            let match_score = self.calculate_match_score_internal(node, query);
            if match_score > 0.1 {
                node.strengthen(match_score);
            }
        }

        Ok(())
    }

    /// Save the current learning state to persistent storage
    pub async fn save_learning_state(&self) -> CoreResult<()> {
        let state = self.serialize_network_state()?;

        // Save to file
        let state_path = std::path::Path::new("./neuroquantum_data/synaptic_state.bin");

        // Create directory if it doesn't exist
        if let Some(parent) = state_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                CoreError::StorageError(format!("Failed to create synaptic state directory: {}", e))
            })?;
        }

        tokio::fs::write(state_path, &state).await.map_err(|e| {
            CoreError::StorageError(format!("Failed to write synaptic state: {}", e))
        })?;

        tracing::info!("✅ Synaptic learning state saved ({} bytes)", state.len());
        Ok(())
    }

    /// Load learning state from persistent storage
    pub async fn load_learning_state(&mut self) -> CoreResult<()> {
        let state_path = std::path::Path::new("./neuroquantum_data/synaptic_state.bin");

        if !state_path.exists() {
            tracing::warn!("No saved synaptic state found, starting fresh");
            return Ok(());
        }

        let state = tokio::fs::read(state_path).await.map_err(|e| {
            CoreError::StorageError(format!("Failed to read synaptic state: {}", e))
        })?;

        self.deserialize_network_state(&state)?;

        tracing::info!("✅ Synaptic learning state loaded ({} bytes)", state.len());
        Ok(())
    }

    /// Serialize the entire network state
    fn serialize_network_state(&self) -> CoreResult<Vec<u8>> {
        #[derive(Serialize)]
        struct NetworkState {
            nodes: Vec<(u64, SynapticNode)>,
            neurons: Vec<(u64, Neuron)>,
            synapses: Vec<Synapse>,
            query_patterns: Vec<(String, QueryPattern)>,
            total_connections: usize,
            memory_usage: usize,
        }

        let nodes: Vec<_> = self
            .nodes
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let neurons: Vec<_> = self
            .neurons
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let synapses = self.synapses.read().unwrap().clone();
        let query_patterns: Vec<_> = self
            .query_patterns
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let total_connections = *self.total_connections.read().unwrap();
        let memory_usage = *self.memory_usage.read().unwrap();

        let state = NetworkState {
            nodes,
            neurons,
            synapses,
            query_patterns,
            total_connections,
            memory_usage,
        };

        bincode::serialize(&state).map_err(|e| {
            CoreError::SerializationError(format!("Failed to serialize network state: {}", e))
        })
    }

    /// Deserialize and restore network state
    fn deserialize_network_state(&mut self, data: &[u8]) -> CoreResult<()> {
        #[derive(Deserialize)]
        struct NetworkState {
            nodes: Vec<(u64, SynapticNode)>,
            neurons: Vec<(u64, Neuron)>,
            synapses: Vec<Synapse>,
            query_patterns: Vec<(String, QueryPattern)>,
            total_connections: usize,
            memory_usage: usize,
        }

        let state: NetworkState = bincode::deserialize(data).map_err(|e| {
            CoreError::StorageError(format!("Failed to deserialize network state: {}", e))
        })?;

        // Restore nodes
        let mut nodes = self.nodes.write().unwrap();
        nodes.clear();
        for (id, node) in state.nodes {
            nodes.insert(id, node);
        }
        drop(nodes);

        // Restore neurons
        let mut neurons = self.neurons.write().unwrap();
        neurons.clear();
        for (id, neuron) in state.neurons {
            neurons.insert(id, neuron);
        }
        drop(neurons);

        // Restore synapses
        *self.synapses.write().unwrap() = state.synapses;

        // Restore query patterns
        let mut query_patterns = self.query_patterns.write().unwrap();
        query_patterns.clear();
        for (key, pattern) in state.query_patterns {
            query_patterns.insert(key, pattern);
        }
        drop(query_patterns);

        // Restore statistics
        *self.total_connections.write().unwrap() = state.total_connections;
        *self.memory_usage.write().unwrap() = state.memory_usage;

        Ok(())
    }

    /// Get serialized network data
    pub async fn get_serialized_data(&self) -> CoreResult<Vec<u8>> {
        // For now, return a simple serialized representation
        let nodes = self.nodes.read().unwrap();
        let node_count = nodes.len() as u32;

        let mut data = Vec::new();
        data.extend_from_slice(&node_count.to_le_bytes());

        for (id, node) in nodes.iter() {
            data.extend_from_slice(&id.to_le_bytes());
            data.extend_from_slice(&node.strength.to_le_bytes());
            data.extend_from_slice(&(node.data_payload.len() as u32).to_le_bytes());
            data.extend_from_slice(&node.data_payload);
        }

        Ok(data)
    }

    /// Internal helper for match scoring
    fn calculate_match_score_internal(&self, node: &SynapticNode, query_content: &str) -> f32 {
        if node.data_payload.is_empty() {
            return 0.0;
        }

        let node_content = String::from_utf8_lossy(&node.data_payload);
        let query_bytes = query_content.as_bytes();
        let node_bytes = node_content.as_bytes();

        // Simple pattern matching with boost from node strength
        let mut matches = 0;
        let mut total_comparisons = 0;

        for window in node_bytes.windows(query_bytes.len()) {
            total_comparisons += 1;
            if window == query_bytes {
                matches += 1;
            }
        }

        if total_comparisons == 0 {
            return 0.0;
        }

        let base_score = matches as f32 / total_comparisons as f32;
        base_score * (1.0 + node.strength) // Boost by node strength
    }

    /// Get network statistics
    pub fn stats(&self) -> NetworkStats {
        let nodes = self.nodes.read().unwrap();
        let total_connections = self.total_connections.read().unwrap();
        let memory_usage = self.memory_usage.read().unwrap();

        NetworkStats {
            node_count: nodes.len(),
            connection_count: *total_connections,
            memory_usage_bytes: *memory_usage,
            activation_threshold: self.activation_threshold,
        }
    }

    /// Modify a node with a closure (thread-safe mutation)
    pub fn modify_node<F, R>(&self, node_id: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut SynapticNode) -> R,
    {
        self.nodes.write().unwrap().get_mut(&node_id).map(f)
    }

    /// Get all node IDs
    pub fn get_node_ids(&self) -> Vec<u64> {
        self.nodes.read().unwrap().keys().cloned().collect()
    }

    /// Remove weak connections below threshold
    pub fn prune_weak_connections(&self, threshold: f32) -> usize {
        let mut pruned_count = 0;
        let mut connections_to_prune = Vec::new();

        // Collect weak connections
        {
            let nodes = self.nodes.read().unwrap();
            for (&node_id, node) in nodes.iter() {
                for (conn_idx, connection) in node.connections.iter().enumerate() {
                    if connection.weight.abs() < threshold {
                        connections_to_prune.push((node_id, conn_idx));
                    }
                }
            }
        }

        // Remove weak connections (in reverse order to maintain indices)
        {
            let mut nodes = self.nodes.write().unwrap();
            for (node_id, conn_idx) in connections_to_prune.into_iter().rev() {
                if let Some(node) = nodes.get_mut(&node_id) {
                    if conn_idx < node.connections.len() {
                        node.connections.remove(conn_idx);
                        pruned_count += 1;
                    }
                }
            }
        }

        pruned_count
    }

    /// Connect two nodes with a weighted connection
    pub fn connect_nodes(
        &self,
        source_id: u64,
        target_id: u64,
        weight: f32,
        connection_type: ConnectionType,
    ) -> CoreResult<()> {
        self.modify_node(source_id, |source_node| {
            source_node.add_connection(target_id, weight, connection_type)
        })
        .ok_or_else(|| CoreError::NotFound(format!("Source node {} not found", source_id)))?
    }
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub node_count: usize,
    pub connection_count: usize,
    pub memory_usage_bytes: usize,
    pub activation_threshold: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synaptic_node_creation() {
        let node = SynapticNode::new(1);
        assert_eq!(node.id, 1);
        assert_eq!(node.strength, 0.0);
        assert!(node.connections.is_empty());
    }

    #[test]
    fn test_network_creation() {
        let network = SynapticNetwork::new(1000, 0.5).unwrap();
        assert_eq!(network.max_nodes, 1000);
        assert_eq!(network.activation_threshold, 0.5);
    }

    #[test]
    fn test_network_node_management() {
        let network = SynapticNetwork::new(1000, 0.5).unwrap();
        let node = SynapticNode::new(1);
        network.add_node(node).unwrap();

        assert_eq!(network.nodes.read().unwrap().len(), 1);
    }
}
