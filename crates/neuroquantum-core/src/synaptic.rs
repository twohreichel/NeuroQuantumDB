//! # Synaptic Index Networks (SINs)
//!
//! Core synaptic data structures implementing neuromorphic computing principles
//! for self-optimizing data organization and intelligent indexing.

use crate::error::{CoreError, CoreResult};
use crate::neon_optimization::NeonOptimizer;
use std::collections::HashMap;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};

/// Types of synaptic connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    Excitatory,
    Inhibitory,
    Modulatory,
}

/// Individual synaptic node representing a data point or index entry
#[derive(Debug, Clone)]
pub struct SynapticNode {
    pub id: u64,
    pub strength: f32,
    pub connections: Vec<SynapticConnection>,
    pub last_access: Instant,
    pub access_count: u64,
    pub data_payload: Vec<u8>,
    pub activation_level: f32,
    pub learning_rate: f32,
    pub decay_factor: f32,
}

/// Synaptic connection between nodes
#[derive(Debug, Clone)]
pub struct SynapticConnection {
    pub target_id: u64,
    pub weight: f32,
    pub connection_type: ConnectionType,
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
            access_count: 0,
            data_payload: Vec::new(),
            activation_level: 0.0,
            learning_rate: 0.01,
            decay_factor: 0.99,
        }
    }

    /// Create a node with data payload
    pub fn with_data(id: u64, data: Vec<u8>) -> Self {
        Self {
            id,
            strength: 0.0,
            connections: Vec::new(),
            last_access: Instant::now(),
            access_count: 0,
            data_payload: data,
            activation_level: 0.0,
            learning_rate: 0.01,
            decay_factor: 0.99,
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
    pub fn apply_decay(&mut self) {
        self.strength *= self.decay_factor;
        self.activation_level *= self.decay_factor;
    }

    /// Add a connection to another node
    pub fn add_connection(&mut self, target_id: u64, weight: f32, connection_type: ConnectionType) -> CoreResult<()> {
        // Check if connection already exists
        if self.connections.iter().any(|c| c.target_id == target_id) {
            return Err(CoreError::InvalidOperation(
                format!("Connection to node {} already exists", target_id)
            ));
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

    /// Strengthen a specific connection using Hebbian learning
    pub fn strengthen_connection(&mut self, target_id: u64, amount: f32) -> CoreResult<()> {
        let connection = self.connections.iter_mut()
            .find(|c| c.target_id == target_id)
            .ok_or_else(|| CoreError::NotFound(format!("Connection to node {} not found", target_id)))?;

        connection.weight += amount * connection.plasticity_factor;
        connection.weight = connection.weight.min(1.0).max(-1.0); // Keep in range [-1, 1]
        connection.last_strengthened = Instant::now();
        connection.usage_count += 1;

        Ok(())
    }

    /// Calculate activation based on input signals
    pub fn calculate_activation(&mut self, input_signals: &[f32]) -> f32 {
        let weighted_sum: f32 = self.connections.iter()
            .zip(input_signals.iter())
            .map(|(conn, signal)| conn.weight * signal)
            .sum();

        // Apply sigmoid activation function
        self.activation_level = 1.0 / (1.0 + (-weighted_sum).exp());
        self.activation_level
    }

    /// Get memory usage of this node
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() +
        self.connections.len() * std::mem::size_of::<SynapticConnection>() +
        self.data_payload.len()
    }
}

/// Synaptic network managing collections of nodes and their relationships
#[derive(Debug)]
pub struct SynapticNetwork {
    pub nodes: HashMap<u64, SynapticNode>, // Made public for learning algorithm access
    max_nodes: usize,
    activation_threshold: f32,
    total_connections: usize,
    memory_usage: usize,
    neon_optimizer: Option<NeonOptimizer>,
}

impl SynapticNetwork {
    /// Create a new synaptic network
    pub fn new(max_nodes: usize, activation_threshold: f32) -> CoreResult<Self> {
        let neon_optimizer = if cfg!(target_arch = "aarch64") {
            Some(NeonOptimizer::new()?)
        } else {
            None
        };

        Ok(Self {
            nodes: HashMap::with_capacity(max_nodes.min(1000)), // Initial capacity
            max_nodes,
            activation_threshold,
            total_connections: 0,
            memory_usage: 0,
            neon_optimizer,
        })
    }

    /// Add a node to the network
    #[instrument(level = "debug", skip(self, node))]
    pub fn add_node(&mut self, node: SynapticNode) -> CoreResult<()> {
        if self.nodes.len() >= self.max_nodes {
            return Err(CoreError::ResourceExhausted(
                format!("Maximum nodes ({}) exceeded", self.max_nodes)
            ));
        }

        if self.nodes.contains_key(&node.id) {
            return Err(CoreError::InvalidOperation(
                format!("Node with ID {} already exists", node.id)
            ));
        }

        self.memory_usage += node.memory_usage();
        self.nodes.insert(node.id, node);

        debug!("Added node to network, total nodes: {}", self.nodes.len());
        Ok(())
    }

    /// Connect two nodes in the network
    #[instrument(level = "debug", skip(self))]
    pub fn connect_nodes(&mut self, source_id: u64, target_id: u64, weight: f32, connection_type: ConnectionType) -> CoreResult<()> {
        // Verify both nodes exist
        if !self.nodes.contains_key(&source_id) {
            return Err(CoreError::NotFound(format!("Source node {} not found", source_id)));
        }
        if !self.nodes.contains_key(&target_id) {
            return Err(CoreError::NotFound(format!("Target node {} not found", target_id)));
        }

        // Get mutable reference to source node
        let source_node = self.nodes.get_mut(&source_id)
            .ok_or_else(|| CoreError::NotFound(format!("Source node {} not found", source_id)))?;

        source_node.add_connection(target_id, weight, connection_type)?;
        self.total_connections += 1;

        debug!("Connected nodes {} -> {}, total connections: {}", source_id, target_id, self.total_connections);
        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, id: u64) -> Option<&SynapticNode> {
        self.nodes.get(&id)
    }

    /// Get a mutable reference to a node
    pub fn get_node_mut(&mut self, id: u64) -> Option<&mut SynapticNode> {
        self.nodes.get_mut(&id)
    }

    /// Activate a node and propagate signals
    #[instrument(level = "debug", skip(self))]
    pub fn activate_node(&mut self, id: u64, input_strength: f32) -> CoreResult<Vec<(u64, f32)>> {
        let mut propagated_signals = Vec::new();

        {
            let node = self.nodes.get_mut(&id)
                .ok_or_else(|| CoreError::NotFound(format!("Node {} not found", id)))?;

            node.strengthen(input_strength);

            // Calculate activation
            let activation = if node.activation_level > self.activation_threshold {
                node.activation_level
            } else {
                0.0
            };

            // Prepare signals to propagate
            for connection in &node.connections {
                let signal_strength = activation * connection.weight;
                propagated_signals.push((connection.target_id, signal_strength));
            }
        }

        // Propagate signals to connected nodes
        for (target_id, signal_strength) in &propagated_signals {
            if let Some(target_node) = self.nodes.get_mut(target_id) {
                target_node.activation_level += signal_strength;
            }
        }

        Ok(propagated_signals)
    }

    /// Apply decay to all nodes (simulating natural forgetting)
    pub fn apply_global_decay(&mut self) {
        for node in self.nodes.values_mut() {
            node.apply_decay();
        }
    }

    /// Find most connected nodes (hubs)
    pub fn find_hub_nodes(&self, top_n: usize) -> Vec<(u64, usize)> {
        let mut nodes_by_connections: Vec<_> = self.nodes.iter()
            .map(|(id, node)| (*id, node.connections.len()))
            .collect();

        nodes_by_connections.sort_by(|a, b| b.1.cmp(&a.1));
        nodes_by_connections.truncate(top_n);
        nodes_by_connections
    }

    /// Get nodes with highest strength (most frequently accessed)
    pub fn get_strongest_nodes(&self, top_n: usize) -> Vec<(u64, f32)> {
        let mut nodes_by_strength: Vec<_> = self.nodes.iter()
            .map(|(id, node)| (*id, node.strength))
            .collect();

        nodes_by_strength.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        nodes_by_strength.truncate(top_n);
        nodes_by_strength
    }

    /// Get serialized data for quantum algorithms
    pub async fn get_serialized_data(&self) -> CoreResult<Vec<u8>> {
        let mut serialized_data = Vec::new();

        // Serialize node data and connections for quantum processing
        for (node_id, node) in &self.nodes {
            // Add node ID (8 bytes)
            serialized_data.extend_from_slice(&node_id.to_le_bytes());

            // Add node strength (4 bytes)
            serialized_data.extend_from_slice(&node.strength.to_le_bytes());

            // Add data payload length and data
            let payload_len = node.data_payload.len() as u32;
            serialized_data.extend_from_slice(&payload_len.to_le_bytes());
            serialized_data.extend_from_slice(&node.data_payload);

            // Add connection count and connections
            let conn_count = node.connections.len() as u32;
            serialized_data.extend_from_slice(&conn_count.to_le_bytes());

            for connection in &node.connections {
                serialized_data.extend_from_slice(&connection.target_id.to_le_bytes());
                serialized_data.extend_from_slice(&connection.weight.to_le_bytes());
            }
        }

        Ok(serialized_data)
    }

    /// Process query using synaptic network
    pub async fn process_query(&self, query: &crate::query::Query) -> CoreResult<crate::query::QueryResult> {
        use crate::query::QueryResult;

        let start_time = std::time::Instant::now();
        let mut activated_nodes = Vec::new();
        let mut total_activation = 0.0;

        // Find nodes that match the query pattern
        for (node_id, node) in &self.nodes {
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

        let base_score = if total_comparisons > 0 {
            matches as f32 / total_comparisons as f32
        } else {
            0.0
        };

        // Boost by node strength and recent activity
        base_score * node.strength * (1.0 + node.access_count as f32 / 1000.0)
    }

    /// Get average connection strength for metrics
    pub fn average_connection_strength(&self) -> f32 {
        let mut total_weight = 0.0;
        let mut total_connections = 0;

        for node in self.nodes.values() {
            for connection in &node.connections {
                total_weight += connection.weight.abs();
                total_connections += 1;
            }
        }

        if total_connections > 0 {
            total_weight / total_connections as f32
        } else {
            0.0
        }
    }

    /// Get network statistics
    pub fn get_statistics(&self) -> NetworkStatistics {
        let total_strength: f32 = self.nodes.values().map(|n| n.strength).sum();
        let avg_connections = if !self.nodes.is_empty() {
            self.total_connections as f32 / self.nodes.len() as f32
        } else {
            0.0
        };

        NetworkStatistics {
            total_nodes: self.nodes.len(),
            total_connections: self.total_connections,
            average_connections_per_node: avg_connections,
            total_strength,
            average_strength: total_strength / self.nodes.len().max(1) as f32,
            memory_usage_bytes: self.memory_usage,
        }
    }

    /// Get current memory usage
    pub fn memory_usage(&self) -> usize {
        self.memory_usage
    }

    /// Optimize network using NEON SIMD instructions (ARM64 only)
    pub fn optimize_with_neon(&mut self) -> CoreResult<()> {
        if let Some(ref optimizer) = self.neon_optimizer {
            optimizer.optimize_connections(&mut self.nodes)?;
        }
        Ok(())
    }

    /// Apply NEON optimizations if available
    #[cfg(feature = "neon-optimizations")]
    pub fn apply_neon_optimizations(&mut self) -> CoreResult<()> {
        if let Some(ref optimizer) = self.neon_optimizer {
            // Apply NEON-SIMD optimizations to all connections at once
            optimizer.optimize_connections(&mut self.nodes)?;
        }
        Ok(())
    }
}

/// Network performance and health statistics
#[derive(Debug, Clone, Serialize)]
pub struct NetworkStatistics {
    pub total_nodes: usize,
    pub total_connections: usize,
    pub average_connections_per_node: f32,
    pub total_strength: f32,
    pub average_strength: f32,
    pub memory_usage_bytes: usize,
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
    fn test_node_strengthening() {
        let mut node = SynapticNode::new(1);
        node.strengthen(0.5);
        assert!(node.strength > 0.0);
        assert_eq!(node.access_count, 1);
    }

    #[test]
    fn test_node_connections() {
        let mut node = SynapticNode::new(1);
        node.add_connection(2, 0.5, ConnectionType::Excitatory).unwrap();
        assert_eq!(node.connections.len(), 1);
        assert_eq!(node.connections[0].target_id, 2);
    }

    #[test]
    fn test_network_creation() {
        let network = SynapticNetwork::new(1000, 0.5).unwrap();
        assert_eq!(network.max_nodes, 1000);
        assert_eq!(network.activation_threshold, 0.5);
    }

    #[test]
    fn test_network_node_management() {
        let mut network = SynapticNetwork::new(1000, 0.5).unwrap();
        let node = SynapticNode::new(1);
        network.add_node(node).unwrap();

        assert!(network.get_node(1).is_some());
        assert_eq!(network.nodes.len(), 1);
    }

    #[test]
    fn test_node_activation() {
        let mut network = SynapticNetwork::new(1000, 0.3).unwrap();
        let node1 = SynapticNode::new(1);
        let node2 = SynapticNode::new(2);

        network.add_node(node1).unwrap();
        network.add_node(node2).unwrap();
        network.connect_nodes(1, 2, 0.8, ConnectionType::Excitatory).unwrap();

        let signals = network.activate_node(1, 0.7).unwrap();
        assert!(!signals.is_empty());
    }

    #[test]
    fn test_network_statistics() {
        let mut network = SynapticNetwork::new(1000, 0.5).unwrap();
        let node = SynapticNode::new(1);
        network.add_node(node).unwrap();

        let stats = network.get_statistics();
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.total_connections, 0);
    }
}
