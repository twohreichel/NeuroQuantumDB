//! Data replication across cluster nodes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::error::{ClusterError, ClusterResult};
use crate::node::NodeId;
use crate::sharding::ShardId;

/// Replication consistency level for read/write operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConsistencyLevel {
    /// Write to one node (fastest, least durable)
    One,
    /// Write to a quorum of nodes
    #[default]
    Quorum,
    /// Write to all replica nodes (slowest, most durable)
    All,
    /// Write to local datacenter quorum
    LocalQuorum,
}

/// Status of a replication operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationStatus {
    /// Replication pending
    Pending,
    /// Replication in progress
    InProgress,
    /// Replication succeeded
    Succeeded,
    /// Replication failed
    Failed,
}

/// A replication request for data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationRequest {
    /// Unique request ID
    pub request_id: u64,
    /// Source node
    pub source_node: NodeId,
    /// Target nodes
    pub target_nodes: Vec<NodeId>,
    /// Shard ID
    pub shard_id: ShardId,
    /// Key being replicated
    pub key: Vec<u8>,
    /// Value being replicated
    pub value: Vec<u8>,
    /// Required consistency level
    pub consistency_level: ConsistencyLevel,
    /// Request timestamp
    pub timestamp: u64,
}

/// Acknowledgment of a replication request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationAck {
    /// Request ID being acknowledged
    pub request_id: u64,
    /// Node sending the ack
    pub node_id: NodeId,
    /// Whether replication succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Tracks pending replication operations.
struct PendingReplication {
    /// The replication request
    request: ReplicationRequest,
    /// Acks received so far
    acks: Vec<ReplicationAck>,
    /// When the request was created
    created_at: Instant,
    /// Current status
    status: ReplicationStatus,
}

/// Manages data replication across the cluster.
pub struct ReplicationManager {
    /// Local node ID
    node_id: NodeId,
    /// Replication factor
    replication_factor: u32,
    /// Pending replications
    pending: Arc<RwLock<HashMap<u64, PendingReplication>>>,
    /// Replication timeout
    timeout: Duration,
    /// Next request ID
    next_request_id: Arc<RwLock<u64>>,
}

impl ReplicationManager {
    /// Create a new replication manager.
    #[must_use]
    pub fn new(node_id: NodeId, replication_factor: u32, timeout: Duration) -> Self {
        info!(
            node_id,
            replication_factor,
            timeout_ms = timeout.as_millis(),
            "Creating replication manager"
        );

        Self {
            node_id,
            replication_factor,
            pending: Arc::new(RwLock::new(HashMap::new())),
            timeout,
            next_request_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Start a replication request.
    pub async fn replicate(
        &self,
        shard_id: ShardId,
        key: Vec<u8>,
        value: Vec<u8>,
        target_nodes: Vec<NodeId>,
        consistency_level: ConsistencyLevel,
    ) -> ClusterResult<u64> {
        let request_id = {
            let mut id = self.next_request_id.write().await;
            let current = *id;
            *id += 1;
            current
        };

        let request = ReplicationRequest {
            request_id,
            source_node: self.node_id,
            target_nodes: target_nodes.clone(),
            shard_id,
            key,
            value,
            consistency_level,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        {
            let mut pending = self.pending.write().await;
            pending.insert(
                request_id,
                PendingReplication {
                    request: request.clone(),
                    acks: Vec::new(),
                    created_at: Instant::now(),
                    status: ReplicationStatus::InProgress,
                },
            );
        }

        debug!(
            request_id,
            target_count = target_nodes.len(),
            "Started replication request"
        );

        // In a full implementation, send request to target nodes via network transport

        Ok(request_id)
    }

    /// Process a replication acknowledgment.
    pub async fn process_ack(&self, ack: ReplicationAck) -> ClusterResult<bool> {
        let mut pending = self.pending.write().await;

        let repl = pending
            .get_mut(&ack.request_id)
            .ok_or_else(|| ClusterError::Internal(format!("Unknown request {}", ack.request_id)))?;

        repl.acks.push(ack.clone());

        let required_acks = self.required_acks(repl.request.consistency_level);
        let successful_acks = repl.acks.iter().filter(|a| a.success).count();

        debug!(
            request_id = ack.request_id,
            successful = successful_acks,
            required = required_acks,
            "Processed replication ack"
        );

        if successful_acks >= required_acks {
            repl.status = ReplicationStatus::Succeeded;
            return Ok(true);
        }

        // Check if we can still reach required acks
        let remaining = repl.request.target_nodes.len() - repl.acks.len();
        if successful_acks + remaining < required_acks {
            repl.status = ReplicationStatus::Failed;
            return Err(ClusterError::QuorumNotReached {
                needed: required_acks,
                have: successful_acks,
            });
        }

        Ok(false)
    }

    /// Get the status of a replication request.
    pub async fn get_status(&self, request_id: u64) -> ClusterResult<ReplicationStatus> {
        let pending = self.pending.read().await;

        pending
            .get(&request_id)
            .map(|r| r.status)
            .ok_or_else(|| ClusterError::Internal(format!("Unknown request {}", request_id)))
    }

    /// Clean up completed or expired replication requests.
    pub async fn cleanup(&self) -> usize {
        let mut pending = self.pending.write().await;
        let before = pending.len();

        pending.retain(|_, repl| {
            // Keep if still in progress and not timed out
            if repl.status == ReplicationStatus::InProgress
                && repl.created_at.elapsed() < self.timeout
            {
                return true;
            }

            // Keep if succeeded/failed but recent (for status queries)
            if repl.status != ReplicationStatus::InProgress
                && repl.created_at.elapsed() < Duration::from_secs(60)
            {
                return true;
            }

            false
        });

        let removed = before - pending.len();
        if removed > 0 {
            debug!(removed, "Cleaned up old replication requests");
        }

        removed
    }

    /// Calculate required acks for a consistency level.
    fn required_acks(&self, level: ConsistencyLevel) -> usize {
        match level {
            ConsistencyLevel::One => 1,
            ConsistencyLevel::Quorum => (self.replication_factor as usize / 2) + 1,
            ConsistencyLevel::All => self.replication_factor as usize,
            ConsistencyLevel::LocalQuorum => (self.replication_factor as usize / 2) + 1,
        }
    }
}

/// Anti-entropy repair for maintaining replica consistency.
#[allow(dead_code)]
pub struct AntiEntropyRepair {
    /// Local node ID
    node_id: NodeId,
    /// Repair interval (used in scheduled repair cycles)
    repair_interval: Duration,
}

impl AntiEntropyRepair {
    /// Create a new anti-entropy repair service.
    #[must_use]
    pub fn new(node_id: NodeId, repair_interval: Duration) -> Self {
        info!(
            node_id,
            interval_secs = repair_interval.as_secs(),
            "Creating anti-entropy repair service"
        );

        Self {
            node_id,
            repair_interval,
        }
    }

    /// Run a repair cycle for a shard.
    pub async fn repair_shard(
        &self,
        shard_id: ShardId,
        replica_nodes: &[NodeId],
    ) -> ClusterResult<RepairResult> {
        debug!(
            node_id = self.node_id,
            shard_id,
            replicas = replica_nodes.len(),
            "Starting shard repair"
        );

        // In a full implementation:
        // 1. Exchange Merkle tree roots with replicas
        // 2. Identify divergent ranges
        // 3. Stream differing data
        // 4. Apply repairs

        Ok(RepairResult {
            shard_id,
            keys_repaired: 0,
            bytes_transferred: 0,
            duration_ms: 0,
        })
    }
}

/// Result of an anti-entropy repair operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairResult {
    /// Shard that was repaired
    pub shard_id: ShardId,
    /// Number of keys that were repaired
    pub keys_repaired: u64,
    /// Bytes transferred during repair
    pub bytes_transferred: u64,
    /// Duration of repair in milliseconds
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistency_level_default() {
        assert_eq!(ConsistencyLevel::default(), ConsistencyLevel::Quorum);
    }

    #[tokio::test]
    async fn test_replication_request() {
        let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

        let request_id = manager
            .replicate(
                1,
                b"key".to_vec(),
                b"value".to_vec(),
                vec![2, 3],
                ConsistencyLevel::One,
            )
            .await
            .unwrap();

        assert_eq!(request_id, 1);

        let status = manager.get_status(request_id).await.unwrap();
        assert_eq!(status, ReplicationStatus::InProgress);
    }

    #[tokio::test]
    async fn test_replication_ack() {
        let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

        let request_id = manager
            .replicate(
                1,
                b"key".to_vec(),
                b"value".to_vec(),
                vec![2, 3],
                ConsistencyLevel::One,
            )
            .await
            .unwrap();

        let completed = manager
            .process_ack(ReplicationAck {
                request_id,
                node_id: 2,
                success: true,
                error: None,
            })
            .await
            .unwrap();

        assert!(completed);

        let status = manager.get_status(request_id).await.unwrap();
        assert_eq!(status, ReplicationStatus::Succeeded);
    }

    #[test]
    fn test_required_acks() {
        let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

        assert_eq!(manager.required_acks(ConsistencyLevel::One), 1);
        assert_eq!(manager.required_acks(ConsistencyLevel::Quorum), 2);
        assert_eq!(manager.required_acks(ConsistencyLevel::All), 3);
    }
}
