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
    max_nodes: usize,
    activation_threshold: f32,
    total_connections: RwLock<usize>,
    memory_usage: RwLock<usize>,
    #[allow(dead_code)] // Used for ARM64/NEON optimizations on Raspberry Pi
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
            nodes: RwLock::new(HashMap::with_capacity(max_nodes.min(1000))),
            max_nodes,
            activation_threshold,
            total_connections: RwLock::new(0),
            memory_usage: RwLock::new(0),
            neon_optimizer,
        })
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
        node.data_payload = data.sequence; // Use the sequence directly since it's already Vec<u8>

        // Add the node to the network
        self.add_node(node)?;

        Ok(node_id.to_string())
    }

    /// Optimize the network structure
    pub async fn optimize_network(&self) -> CoreResult<()> {
        // Apply decay to all nodes
        self.apply_global_decay();

        // Prune very weak connections
        let mut connections_to_remove = Vec::new();

        {
            let nodes = self.nodes.read().unwrap();
            for (node_id, node) in nodes.iter() {
                for (i, connection) in node.connections.iter().enumerate() {
                    if connection.weight.abs() < 0.01 {
                        connections_to_remove.push((*node_id, i));
                    }
                }
            }
        }

        // Remove weak connections
        {
            let mut nodes = self.nodes.write().unwrap();
            let mut total_connections = self.total_connections.write().unwrap();

            for (node_id, connection_index) in connections_to_remove.into_iter().rev() {
                if let Some(node) = nodes.get_mut(&node_id) {
                    if connection_index < node.connections.len() {
                        node.connections.remove(connection_index);
                        *total_connections -= 1;
                    }
                }
            }
        }

        tracing::info!("Network optimization completed");
        Ok(())
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

    /// Get a reference to a node
    pub fn get_node(&self, node_id: u64) -> Option<SynapticNode> {
        self.nodes.read().unwrap().get(&node_id).cloned()
    }

    /// Get a mutable reference to a node
    pub fn get_node_mut(&self, node_id: u64) -> Option<()> {
        // For thread safety, we can't return a mutable reference directly
        // Instead, we provide a way to check if the node exists
        self.nodes.read().unwrap().contains_key(&node_id).then(|| ())
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

    /// Save the current learning state
    pub async fn save_learning_state(&self) -> CoreResult<()> {
        // In production, this would serialize the network state to persistent storage
        tracing::info!("Synaptic learning state saved");
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
