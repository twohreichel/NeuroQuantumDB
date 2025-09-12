//! Synaptic network implementation for NeuroQuantumDB
//!
//! This module implements the core synaptic data structures that form the
//! foundation of the neuromorphic computing layer.

use std::time::Instant;
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use parking_lot::RwLock;
use crate::error::{CoreError, CoreResult};

/// Unique identifier for synaptic nodes
pub type NodeId = u64;

/// Type of synaptic connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    /// Excitatory connection (strengthens signal)
    Excitatory,
    /// Inhibitory connection (weakens signal)
    Inhibitory,
}

/// Usage statistics for optimization
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    pub access_count: u64,
    #[serde(skip)] // Skip serialization for Instant
    pub last_access: Option<Instant>,
    pub avg_response_time_ns: u64,
}

/// Synaptic connection between nodes
#[derive(Debug, Clone)]
pub struct SynapticConnection {
    /// Target node identifier
    pub target_id: NodeId,
    /// Connection weight (-1.0 to 1.0)
    pub weight: f32,
    /// Connection type
    pub connection_type: ConnectionType,
    /// Usage statistics for this connection
    pub usage_stats: UsageStats,
    /// Creation timestamp
    #[serde(skip)] // Skip serialization for Instant
    pub created_at: Instant,
}

impl SynapticConnection {
    pub fn new(target_id: NodeId, weight: f32, connection_type: ConnectionType) -> Self {
        Self {
            target_id,
            weight: weight.clamp(-1.0, 1.0),
            connection_type,
            usage_stats: UsageStats::default(),
            created_at: Instant::now(),
        }
    }

    /// Update connection usage statistics
    pub fn record_usage(&mut self, response_time_ns: u64) {
        self.usage_stats.access_count += 1;
        self.usage_stats.last_access = Some(Instant::now());

        // Update rolling average response time
        let alpha = 0.1; // Smoothing factor
        self.usage_stats.avg_response_time_ns =
            ((1.0 - alpha) * self.usage_stats.avg_response_time_ns as f32 +
             alpha * response_time_ns as f32) as u64;
    }
}

/// Individual synaptic node in the network
#[derive(Debug, Clone)]
pub struct SynapticNode {
    /// Unique node identifier
    pub id: NodeId,
    /// Current synaptic strength (0.0 - 1.0)
    pub strength: f32,
    /// Outgoing connections to other nodes
    pub connections: Vec<SynapticConnection>,
    /// Node activation level
    pub activation: f32,
    /// Data payload (optional)
    pub data: Option<Vec<u8>>,
    /// Node metadata
    pub metadata: NodeMetadata,
    /// Usage statistics
    pub usage_stats: UsageStats,
}

#[derive(Debug, Clone)]
pub struct NodeMetadata {
    /// Data type hint
    pub data_type: String,
    /// Size in bytes
    pub size: usize,
    /// Creation timestamp
    #[serde(skip)] // Skip serialization for Instant
    pub created_at: Instant,
    /// Last modification timestamp
    #[serde(skip)] // Skip serialization for Instant
    pub modified_at: Instant,
}

impl Default for NodeMetadata {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            data_type: "unknown".to_string(),
            size: 0,
            created_at: now,
            modified_at: now,
        }
    }
}

impl SynapticNode {
    /// Create a new synaptic node
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            strength: 0.0,
            connections: Vec::new(),
            activation: 0.0,
            data: None,
            metadata: NodeMetadata::default(),
            usage_stats: UsageStats::default(),
        }
    }

    /// Strengthen the node by a given amount
    pub fn strengthen(&mut self, amount: f32) {
        self.strength = (self.strength + amount).clamp(0.0, 1.0);
        self.metadata.modified_at = Instant::now();
    }

    /// Add a connection to another node
    pub fn add_connection(&mut self, target_id: NodeId, weight: f32, connection_type: ConnectionType) -> CoreResult<()> {
        // Check if connection already exists
        if self.connections.iter().any(|conn| conn.target_id == target_id) {
            return Err(CoreError::ConnectionAlreadyExists { source: self.id, target: target_id });
        }

        let connection = SynapticConnection::new(target_id, weight, connection_type);
        self.connections.push(connection);
        self.metadata.modified_at = Instant::now();

        Ok(())
    }

    /// Remove a connection to another node
    pub fn remove_connection(&mut self, target_id: NodeId) -> CoreResult<()> {
        let initial_len = self.connections.len();
        self.connections.retain(|conn| conn.target_id != target_id);

        if self.connections.len() == initial_len {
            return Err(CoreError::ConnectionNotFound { source: self.id, target: target_id });
        }

        self.metadata.modified_at = Instant::now();
        Ok(())
    }

    /// Calculate total outgoing signal strength
    pub fn calculate_output_signal(&self) -> f32 {
        self.strength * self.activation
    }

    /// Update node activation based on input signals
    pub fn update_activation(&mut self, input_signals: &[f32]) {
        // Sigmoid activation function
        let total_input: f32 = input_signals.iter().sum();
        self.activation = 1.0 / (1.0 + (-total_input).exp());
        self.metadata.modified_at = Instant::now();
    }

    /// Set data payload for the node
    pub fn set_data(&mut self, data: Vec<u8>, data_type: String) {
        self.metadata.size = data.len();
        self.metadata.data_type = data_type;
        self.metadata.modified_at = Instant::now();
        self.data = Some(data);
    }

    /// Record usage of this node
    pub fn record_usage(&mut self, response_time_ns: u64) {
        self.usage_stats.access_count += 1;
        self.usage_stats.last_access = Some(Instant::now());

        // Update rolling average response time
        let alpha = 0.1;
        self.usage_stats.avg_response_time_ns =
            ((1.0 - alpha) * self.usage_stats.avg_response_time_ns as f32 +
             alpha * response_time_ns as f32) as u64;
    }
}

/// High-performance synaptic network implementation
pub struct SynapticNetwork {
    /// Maximum number of nodes
    max_nodes: usize,
    /// Thread-safe node storage
    nodes: DashMap<NodeId, RwLock<SynapticNode>>,
    /// Node ID counter
    next_id: parking_lot::Mutex<NodeId>,
    /// Network statistics
    stats: RwLock<NetworkStats>,
}

#[derive(Debug, Default)]
pub struct NetworkStats {
    pub total_nodes: usize,
    pub total_connections: usize,
    pub avg_node_degree: f32,
    pub memory_usage_bytes: usize,
}

impl SynapticNetwork {
    /// Create a new synaptic network
    pub fn new(max_nodes: usize) -> CoreResult<Self> {
        Ok(Self {
            max_nodes,
            nodes: DashMap::new(),
            next_id: parking_lot::Mutex::new(1),
            stats: RwLock::new(NetworkStats::default()),
        })
    }

    /// Create a new node in the network
    pub fn create_node(&self) -> CoreResult<NodeId> {
        if self.nodes.len() >= self.max_nodes {
            return Err(CoreError::NetworkCapacityExceeded);
        }

        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;
        drop(next_id);

        let node = SynapticNode::new(id);
        self.nodes.insert(id, RwLock::new(node));

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_nodes = self.nodes.len();

        Ok(id)
    }

    /// Connect two nodes with a synaptic connection
    pub fn connect_nodes(
        &self,
        source_id: NodeId,
        target_id: NodeId,
        weight: f32,
        connection_type: ConnectionType,
    ) -> CoreResult<()> {
        // Verify both nodes exist
        let source_entry = self.nodes.get(&source_id)
            .ok_or(CoreError::NodeNotFound(source_id))?;
        let _target_entry = self.nodes.get(&target_id)
            .ok_or(CoreError::NodeNotFound(target_id))?;

        // Add connection to source node
        let mut source_node = source_entry.write();
        source_node.add_connection(target_id, weight, connection_type)?;

        // Update network statistics
        let mut stats = self.stats.write();
        stats.total_connections += 1;
        stats.avg_node_degree = stats.total_connections as f32 / stats.total_nodes as f32;

        Ok(())
    }

    /// Get a node by ID (read-only access)
    pub fn get_node(&self, id: NodeId) -> CoreResult<dashmap::mapref::one::Ref<NodeId, RwLock<SynapticNode>>> {
        self.nodes.get(&id).ok_or(CoreError::NodeNotFound(id))
    }

    /// Strengthen a connection between two nodes
    pub fn strengthen_connection(&self, source_id: NodeId, target_id: NodeId, amount: f32) -> CoreResult<()> {
        let source_entry = self.nodes.get(&source_id)
            .ok_or(CoreError::NodeNotFound(source_id))?;

        let mut source_node = source_entry.write();

        // Find and update the connection
        for connection in &mut source_node.connections {
            if connection.target_id == target_id {
                connection.weight = (connection.weight + amount).clamp(-1.0, 1.0);
                return Ok(());
            }
        }

        Err(CoreError::ConnectionNotFound { source: source_id, target: target_id })
    }

    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats {
        let stats = self.stats.read();
        NetworkStats {
            total_nodes: stats.total_nodes,
            total_connections: stats.total_connections,
            avg_node_degree: stats.avg_node_degree,
            memory_usage_bytes: self.estimate_memory_usage(),
        }
    }

    /// Estimate memory usage of the network
    fn estimate_memory_usage(&self) -> usize {
        let base_size = std::mem::size_of::<SynapticNetwork>();
        let node_size = std::mem::size_of::<SynapticNode>();
        let connection_size = std::mem::size_of::<SynapticConnection>();

        let stats = self.stats.read();
        base_size +
        (stats.total_nodes * node_size) +
        (stats.total_connections * connection_size)
    }

    /// Validate network integrity
    pub fn validate(&self) -> CoreResult<()> {
        let mut total_connections = 0;

        for entry in self.nodes.iter() {
            let node = entry.value().read();

            // Validate node constraints
            if node.strength < 0.0 || node.strength > 1.0 {
                return Err(CoreError::InvalidNodeState(node.id));
            }

            // Validate connections
            for connection in &node.connections {
                if connection.weight < -1.0 || connection.weight > 1.0 {
                    return Err(CoreError::InvalidConnectionWeight {
                        source: node.id,
                        target: connection.target_id,
                        weight: connection.weight,
                    });
                }

                // Verify target node exists
                if !self.nodes.contains_key(&connection.target_id) {
                    return Err(CoreError::DanglingConnection {
                        source: node.id,
                        target: connection.target_id,
                    });
                }

                total_connections += 1;
            }
        }

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_nodes = self.nodes.len();
        stats.total_connections = total_connections;
        stats.avg_node_degree = if stats.total_nodes > 0 {
            total_connections as f32 / stats.total_nodes as f32
        } else {
            0.0
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synaptic_node_creation() {
        let node = SynapticNode::new(1);
        assert_eq!(node.id, 1);
        assert_eq!(node.strength, 0.0);
        assert_eq!(node.activation, 0.0);
        assert!(node.connections.is_empty());
    }

    #[test]
    fn test_node_strengthening() {
        let mut node = SynapticNode::new(1);
        node.strengthen(0.5);
        assert_eq!(node.strength, 0.5);

        // Test clamping
        node.strengthen(0.8);
        assert_eq!(node.strength, 1.0);
    }

    #[test]
    fn test_connection_management() {
        let mut node = SynapticNode::new(1);

        // Add connection
        node.add_connection(2, 0.7, ConnectionType::Excitatory).unwrap();
        assert_eq!(node.connections.len(), 1);
        assert_eq!(node.connections[0].target_id, 2);
        assert_eq!(node.connections[0].weight, 0.7);

        // Remove connection
        node.remove_connection(2).unwrap();
        assert!(node.connections.is_empty());
    }

    #[test]
    fn test_network_creation() {
        let network = SynapticNetwork::new(1000).unwrap();
        let stats = network.get_stats();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_connections, 0);
    }

    #[test]
    fn test_node_creation_and_connection() {
        let network = SynapticNetwork::new(1000).unwrap();

        let node1 = network.create_node().unwrap();
        let node2 = network.create_node().unwrap();

        network.connect_nodes(node1, node2, 0.8, ConnectionType::Excitatory).unwrap();

        let stats = network.get_stats();
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.total_connections, 1);
    }

    #[test]
    fn test_network_validation() {
        let network = SynapticNetwork::new(1000).unwrap();

        let node1 = network.create_node().unwrap();
        let node2 = network.create_node().unwrap();

        network.connect_nodes(node1, node2, 0.5, ConnectionType::Excitatory).unwrap();

        // Network should be valid
        network.validate().unwrap();
    }
}
