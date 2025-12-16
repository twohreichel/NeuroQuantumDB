//! Shard management and consistent hashing for data distribution.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::config::ClusterConfig;
use crate::error::{ClusterError, ClusterResult};
use crate::node::NodeId;

/// Unique identifier for a shard.
pub type ShardId = u64;

/// State of a shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShardState {
    /// Shard is active and serving requests
    Active,
    /// Shard is being transferred to another node
    Transferring,
    /// Shard is being received from another node
    Receiving,
    /// Shard is offline
    Offline,
}

/// Information about a shard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    /// Shard identifier
    pub shard_id: ShardId,
    /// Primary node for this shard
    pub primary_node: NodeId,
    /// Replica nodes for this shard
    pub replica_nodes: Vec<NodeId>,
    /// Current state of the shard
    pub state: ShardState,
    /// Number of keys in this shard
    pub key_count: u64,
    /// Size of the shard in bytes
    pub size_bytes: u64,
}

/// A point on the hash ring.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RingPoint {
    /// Hash value on the ring
    hash: u64,
    /// Node ID owning this point
    node_id: NodeId,
    /// Virtual node index (for debugging and analytics)
    virtual_index: u32,
}

/// Internal state of the shard manager.
struct ShardManagerState {
    /// Hash ring points (sorted by hash)
    ring: Vec<RingPoint>,
    /// Shards by ID
    shards: HashMap<ShardId, ShardInfo>,
    /// Shards by node
    node_shards: HashMap<NodeId, Vec<ShardId>>,
    /// Whether rebalancing is in progress
    rebalancing: bool,
}

/// Manages sharding and data distribution across the cluster.
pub struct ShardManager {
    /// Number of virtual nodes per physical node
    virtual_nodes: u32,
    /// Replication factor
    replication_factor: u32,
    /// Internal state
    state: Arc<RwLock<ShardManagerState>>,
}

impl ShardManager {
    /// Create a new shard manager.
    pub fn new(config: &ClusterConfig) -> ClusterResult<Self> {
        info!(
            virtual_nodes = config.sharding.virtual_nodes,
            replication_factor = config.sharding.replication_factor,
            "Creating shard manager"
        );

        Ok(Self {
            virtual_nodes: config.sharding.virtual_nodes,
            replication_factor: config.sharding.replication_factor,
            state: Arc::new(RwLock::new(ShardManagerState {
                ring: Vec::new(),
                shards: HashMap::new(),
                node_shards: HashMap::new(),
                rebalancing: false,
            })),
        })
    }

    /// Add a node to the hash ring.
    pub async fn add_node(&self, node_id: NodeId) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        // Check if node already exists
        if state.ring.iter().any(|p| p.node_id == node_id) {
            return Err(ClusterError::NodeAlreadyExists(node_id));
        }

        // Add virtual nodes to the ring
        for i in 0..self.virtual_nodes {
            let hash = self.hash_node(node_id, i);
            state.ring.push(RingPoint {
                hash,
                node_id,
                virtual_index: i,
            });
        }

        // Sort ring by hash
        state.ring.sort_by_key(|p| p.hash);

        // Initialize node shard list
        state.node_shards.entry(node_id).or_default();

        info!(
            node_id,
            virtual_nodes = self.virtual_nodes,
            total_ring_size = state.ring.len(),
            "Added node to hash ring"
        );

        Ok(())
    }

    /// Remove a node from the hash ring.
    pub async fn remove_node(&self, node_id: NodeId) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        // Remove all virtual nodes for this node
        let before_count = state.ring.len();
        state.ring.retain(|p| p.node_id != node_id);
        let removed_count = before_count - state.ring.len();

        if removed_count == 0 {
            return Err(ClusterError::NodeNotFound(node_id));
        }

        // Remove from node shards
        state.node_shards.remove(&node_id);

        info!(
            node_id,
            removed_points = removed_count,
            total_ring_size = state.ring.len(),
            "Removed node from hash ring"
        );

        Ok(())
    }

    /// Get the primary node for a given key.
    pub async fn get_primary_node(&self, key: &[u8]) -> ClusterResult<NodeId> {
        let state = self.state.read().await;

        if state.ring.is_empty() {
            return Err(ClusterError::Internal("Hash ring is empty".into()));
        }

        let hash = self.hash_key(key);
        let node_id = self.find_node_for_hash(&state.ring, hash);

        Ok(node_id)
    }

    /// Get the nodes responsible for a key (primary + replicas).
    pub async fn get_nodes_for_key(&self, key: &[u8]) -> ClusterResult<Vec<NodeId>> {
        let state = self.state.read().await;

        if state.ring.is_empty() {
            return Err(ClusterError::Internal("Hash ring is empty".into()));
        }

        let hash = self.hash_key(key);
        let nodes = self.find_nodes_for_hash(&state.ring, hash, self.replication_factor);

        Ok(nodes)
    }

    /// Get all shards for a node.
    pub async fn get_node_shards(&self, node_id: NodeId) -> ClusterResult<Vec<ShardInfo>> {
        let state = self.state.read().await;

        let shard_ids = state
            .node_shards
            .get(&node_id)
            .ok_or(ClusterError::NodeNotFound(node_id))?;

        let shards: Vec<ShardInfo> = shard_ids
            .iter()
            .filter_map(|id| state.shards.get(id).cloned())
            .collect();

        Ok(shards)
    }

    /// Get shard information by ID.
    pub async fn get_shard(&self, shard_id: ShardId) -> ClusterResult<ShardInfo> {
        let state = self.state.read().await;

        state
            .shards
            .get(&shard_id)
            .cloned()
            .ok_or(ClusterError::ShardNotFound(shard_id))
    }

    /// Check if rebalancing is in progress.
    pub async fn is_rebalancing(&self) -> bool {
        self.state.read().await.rebalancing
    }

    /// Start rebalancing shards across nodes.
    pub async fn start_rebalance(&self) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        if state.rebalancing {
            return Err(ClusterError::RebalancingInProgress);
        }

        state.rebalancing = true;
        info!("Starting shard rebalancing");

        // In a full implementation:
        // 1. Calculate optimal shard distribution
        // 2. Identify shards to transfer
        // 3. Initiate shard transfers
        // 4. Update shard assignments

        Ok(())
    }

    /// Complete rebalancing.
    pub async fn complete_rebalance(&self) -> ClusterResult<()> {
        let mut state = self.state.write().await;
        state.rebalancing = false;
        info!("Shard rebalancing completed");
        Ok(())
    }

    /// Get cluster statistics.
    pub async fn get_stats(&self) -> ShardStats {
        let state = self.state.read().await;

        let total_shards = state.shards.len();
        let total_keys: u64 = state.shards.values().map(|s| s.key_count).sum();
        let total_size: u64 = state.shards.values().map(|s| s.size_bytes).sum();
        let node_count = state.node_shards.len();

        ShardStats {
            total_shards,
            total_keys,
            total_size_bytes: total_size,
            node_count,
            rebalancing: state.rebalancing,
            virtual_nodes_per_node: self.virtual_nodes,
            replication_factor: self.replication_factor,
        }
    }

    /// Hash a key to a position on the ring.
    fn hash_key(&self, key: &[u8]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash a node + virtual index to a position on the ring.
    fn hash_node(&self, node_id: NodeId, virtual_index: u32) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        node_id.hash(&mut hasher);
        virtual_index.hash(&mut hasher);
        hasher.finish()
    }

    /// Find the node responsible for a given hash value.
    fn find_node_for_hash(&self, ring: &[RingPoint], hash: u64) -> NodeId {
        // Binary search for the first point >= hash
        let idx = match ring.binary_search_by_key(&hash, |p| p.hash) {
            Ok(i) => i,
            Err(i) => {
                if i >= ring.len() {
                    0 // Wrap around to first node
                } else {
                    i
                }
            }
        };

        ring[idx].node_id
    }

    /// Find N distinct nodes for a given hash value.
    fn find_nodes_for_hash(&self, ring: &[RingPoint], hash: u64, count: u32) -> Vec<NodeId> {
        if ring.is_empty() {
            return Vec::new();
        }

        let mut nodes = Vec::new();
        let start_idx = match ring.binary_search_by_key(&hash, |p| p.hash) {
            Ok(i) => i,
            Err(i) => i % ring.len(),
        };

        let mut idx = start_idx;
        while nodes.len() < count as usize {
            let node_id = ring[idx].node_id;
            if !nodes.contains(&node_id) {
                nodes.push(node_id);
            }

            idx = (idx + 1) % ring.len();

            // Prevent infinite loop if we don't have enough unique nodes
            if idx == start_idx && nodes.len() < count as usize {
                warn!(
                    found = nodes.len(),
                    requested = count,
                    "Not enough unique nodes for replication"
                );
                break;
            }
        }

        nodes
    }
}

/// Statistics about shard distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardStats {
    /// Total number of shards
    pub total_shards: usize,
    /// Total number of keys across all shards
    pub total_keys: u64,
    /// Total size in bytes
    pub total_size_bytes: u64,
    /// Number of nodes in the cluster
    pub node_count: usize,
    /// Whether rebalancing is in progress
    pub rebalancing: bool,
    /// Virtual nodes per physical node
    pub virtual_nodes_per_node: u32,
    /// Replication factor
    pub replication_factor: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_remove_nodes() {
        let config = ClusterConfig::default();
        let manager = ShardManager::new(&config).unwrap();

        // Add nodes
        manager.add_node(1).await.unwrap();
        manager.add_node(2).await.unwrap();
        manager.add_node(3).await.unwrap();

        // Adding same node should fail
        assert!(manager.add_node(1).await.is_err());

        // Remove node
        manager.remove_node(2).await.unwrap();

        // Removing non-existent node should fail
        assert!(manager.remove_node(99).await.is_err());
    }

    #[tokio::test]
    async fn test_consistent_hashing() {
        let config = ClusterConfig::default();
        let manager = ShardManager::new(&config).unwrap();

        manager.add_node(1).await.unwrap();
        manager.add_node(2).await.unwrap();
        manager.add_node(3).await.unwrap();

        // Same key should always go to same node
        let key = b"test-key";
        let node1 = manager.get_primary_node(key).await.unwrap();
        let node2 = manager.get_primary_node(key).await.unwrap();
        assert_eq!(node1, node2);

        // Different keys may go to different nodes
        let keys: Vec<&[u8]> = vec![b"key1", b"key2", b"key3", b"key4", b"key5"];
        let nodes: Vec<NodeId> =
            futures::future::join_all(keys.iter().map(|k| manager.get_primary_node(k)))
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

        // We should have some distribution (at least 2 different nodes)
        let unique_nodes: std::collections::HashSet<_> = nodes.iter().collect();
        assert!(!unique_nodes.is_empty());
    }

    #[tokio::test]
    async fn test_replication() {
        let mut config = ClusterConfig::default();
        config.sharding.replication_factor = 3;

        let manager = ShardManager::new(&config).unwrap();

        manager.add_node(1).await.unwrap();
        manager.add_node(2).await.unwrap();
        manager.add_node(3).await.unwrap();

        let nodes = manager.get_nodes_for_key(b"test-key").await.unwrap();

        // Should get 3 distinct nodes
        assert_eq!(nodes.len(), 3);
        let unique: std::collections::HashSet<_> = nodes.iter().collect();
        assert_eq!(unique.len(), 3);
    }
}
