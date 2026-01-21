//! Shard management and consistent hashing for data distribution.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::ClusterConfig;
use crate::error::{ClusterError, ClusterResult};
use crate::node::NodeId;

/// Unique identifier for a shard.
pub type ShardId = u64;

/// Unique identifier for a transfer operation.
pub type TransferId = u64;

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

/// Status of a shard transfer operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    /// Transfer is pending
    Pending,
    /// Transfer is in progress
    InProgress,
    /// Transfer completed successfully
    Completed,
    /// Transfer failed
    Failed,
    /// Transfer was cancelled
    Cancelled,
}

/// A shard transfer request during rebalancing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardTransfer {
    /// Unique transfer ID
    pub transfer_id: TransferId,
    /// Shard being transferred
    pub shard_id: ShardId,
    /// Source node
    pub source_node: NodeId,
    /// Target node
    pub target_node: NodeId,
    /// Transfer status
    pub status: TransferStatus,
    /// Bytes transferred so far
    pub bytes_transferred: u64,
    /// Total bytes to transfer
    pub total_bytes: u64,
    /// Keys transferred so far
    pub keys_transferred: u64,
    /// Total keys to transfer
    pub total_keys: u64,
    /// When the transfer started (Unix timestamp ms)
    pub started_at_ms: u64,
    /// When the transfer completed (Unix timestamp ms, 0 if not complete)
    pub completed_at_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
}

impl ShardTransfer {
    /// Calculate transfer progress as a percentage (0-100).
    #[must_use]
    pub fn progress_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return if self.status == TransferStatus::Completed {
                100.0
            } else {
                0.0
            };
        }
        (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0
    }
}

/// Progress of the rebalancing operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RebalanceProgress {
    /// Whether rebalancing is active
    pub active: bool,
    /// Total transfers planned
    pub total_transfers: usize,
    /// Transfers completed
    pub completed_transfers: usize,
    /// Transfers in progress
    pub in_progress_transfers: usize,
    /// Transfers failed
    pub failed_transfers: usize,
    /// Total bytes to transfer
    pub total_bytes: u64,
    /// Bytes transferred so far
    pub bytes_transferred: u64,
    /// Estimated time to completion in seconds (0 if unknown)
    pub eta_seconds: u64,
    /// When rebalancing started (Unix timestamp ms)
    pub started_at_ms: u64,
    /// Bytes per second throughput (0 if not yet calculated)
    pub throughput_bytes_per_sec: u64,
}

/// Configuration for bandwidth throttling during rebalancing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceConfig {
    /// Maximum bytes per second for all transfers combined (0 = unlimited)
    pub max_bandwidth_bytes_per_sec: u64,
    /// Maximum concurrent transfers
    pub max_concurrent_transfers: u32,
    /// Delay before starting rebalance after membership change
    pub rebalance_delay: Duration,
    /// Whether automatic rebalancing is enabled
    pub auto_rebalance: bool,
}

impl Default for RebalanceConfig {
    fn default() -> Self {
        Self {
            max_bandwidth_bytes_per_sec: 0,
            max_concurrent_transfers: 2,
            rebalance_delay: Duration::from_secs(30),
            auto_rebalance: true,
        }
    }
}

/// A point on the hash ring.
#[derive(Debug, Clone)]
struct RingPoint {
    /// Hash value on the ring
    hash: u64,
    /// Node ID owning this point
    node_id: NodeId,
    /// Virtual node index (used for ring distribution analytics)
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
    /// Active and historical transfers
    transfers: HashMap<TransferId, ShardTransfer>,
    /// When rebalancing started
    rebalance_started_at: Option<Instant>,
    /// Total bytes planned for current rebalance
    rebalance_total_bytes: u64,
}

/// Manages sharding and data distribution across the cluster.
pub struct ShardManager {
    /// Number of virtual nodes per physical node
    virtual_nodes: u32,
    /// Replication factor
    replication_factor: u32,
    /// Internal state
    state: Arc<RwLock<ShardManagerState>>,
    /// Rebalance configuration
    rebalance_config: RebalanceConfig,
    /// Next transfer ID counter
    next_transfer_id: AtomicU64,
}

impl ShardManager {
    /// Create a new shard manager.
    pub fn new(config: &ClusterConfig) -> ClusterResult<Self> {
        info!(
            virtual_nodes = config.sharding.virtual_nodes,
            replication_factor = config.sharding.replication_factor,
            "Creating shard manager"
        );

        let rebalance_config = RebalanceConfig {
            max_bandwidth_bytes_per_sec: 0,
            max_concurrent_transfers: config.sharding.max_concurrent_transfers,
            rebalance_delay: config.sharding.rebalance_delay,
            auto_rebalance: config.sharding.auto_rebalance,
        };

        Ok(Self {
            virtual_nodes: config.sharding.virtual_nodes,
            replication_factor: config.sharding.replication_factor,
            state: Arc::new(RwLock::new(ShardManagerState {
                ring: Vec::new(),
                shards: HashMap::new(),
                node_shards: HashMap::new(),
                rebalancing: false,
                transfers: HashMap::new(),
                rebalance_started_at: None,
                rebalance_total_bytes: 0,
            })),
            rebalance_config,
            next_transfer_id: AtomicU64::new(1),
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

    /// Get the current rebalance configuration.
    #[must_use]
    pub const fn rebalance_config(&self) -> &RebalanceConfig {
        &self.rebalance_config
    }

    /// Start rebalancing shards across nodes.
    ///
    /// This calculates the optimal shard distribution based on the current
    /// hash ring and creates transfer plans for any shards that need to move.
    pub async fn start_rebalance(&self) -> ClusterResult<Vec<ShardTransfer>> {
        let mut state = self.state.write().await;

        if state.rebalancing {
            return Err(ClusterError::RebalancingInProgress);
        }

        state.rebalancing = true;
        state.rebalance_started_at = Some(Instant::now());
        state.transfers.clear();

        info!("Starting shard rebalancing");

        // Calculate transfers needed based on current ring state
        let transfers = self.calculate_transfers(&state);

        // Store transfers
        let mut total_bytes = 0u64;
        for transfer in &transfers {
            total_bytes = total_bytes.saturating_add(transfer.total_bytes);
            state
                .transfers
                .insert(transfer.transfer_id, transfer.clone());
        }
        state.rebalance_total_bytes = total_bytes;

        info!(
            transfer_count = transfers.len(),
            total_bytes, "Rebalancing plan created"
        );

        Ok(transfers)
    }

    /// Calculate which shards need to be transferred based on the hash ring.
    fn calculate_transfers(&self, state: &ShardManagerState) -> Vec<ShardTransfer> {
        let mut transfers = Vec::new();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // For each shard, determine if it needs to move to a different primary node
        for (shard_id, shard_info) in &state.shards {
            // Hash the shard ID to find its correct primary node
            let shard_hash = {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                shard_id.hash(&mut hasher);
                hasher.finish()
            };

            if state.ring.is_empty() {
                continue;
            }

            let target_node = self.find_node_for_hash(&state.ring, shard_hash);

            // If the shard's current primary is different from target, create a transfer
            if shard_info.primary_node != target_node {
                let transfer_id = self.next_transfer_id.fetch_add(1, Ordering::SeqCst);

                transfers.push(ShardTransfer {
                    transfer_id,
                    shard_id: *shard_id,
                    source_node: shard_info.primary_node,
                    target_node,
                    status: TransferStatus::Pending,
                    bytes_transferred: 0,
                    total_bytes: shard_info.size_bytes,
                    keys_transferred: 0,
                    total_keys: shard_info.key_count,
                    started_at_ms: now_ms,
                    completed_at_ms: 0,
                    error: None,
                });

                debug!(
                    shard_id,
                    source_node = shard_info.primary_node,
                    target_node,
                    "Shard requires transfer"
                );
            }
        }

        transfers
    }

    /// Calculate transfers needed when a new node joins the cluster.
    ///
    /// This determines which shards should be moved to the new node to
    /// achieve a balanced distribution.
    pub async fn calculate_node_join_transfers(
        &self,
        new_node_id: NodeId,
    ) -> ClusterResult<Vec<ShardTransfer>> {
        let state = self.state.read().await;

        if state.ring.is_empty() {
            return Ok(Vec::new());
        }

        // Identify shards that should now belong to the new node
        let transfers = self.calculate_transfers(&state);

        // Filter to only transfers TO the new node
        let join_transfers: Vec<ShardTransfer> = transfers
            .into_iter()
            .filter(|t| t.target_node == new_node_id)
            .collect();

        info!(
            new_node_id,
            transfer_count = join_transfers.len(),
            "Calculated node join transfers"
        );

        Ok(join_transfers)
    }

    /// Calculate transfers needed when a node leaves the cluster.
    ///
    /// This identifies orphaned shards and determines which remaining
    /// nodes should take ownership.
    pub async fn calculate_node_leave_transfers(
        &self,
        leaving_node_id: NodeId,
    ) -> ClusterResult<Vec<ShardTransfer>> {
        let state = self.state.read().await;

        // Find all shards that have the leaving node as primary
        let orphaned_shards: Vec<ShardInfo> = state
            .shards
            .values()
            .filter(|s| s.primary_node == leaving_node_id)
            .cloned()
            .collect();

        if orphaned_shards.is_empty() {
            return Ok(Vec::new());
        }

        // For each orphaned shard, find the next best node
        let mut transfers = Vec::new();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Create a temporary ring without the leaving node
        let remaining_ring: Vec<RingPoint> = state
            .ring
            .iter()
            .filter(|p| p.node_id != leaving_node_id)
            .cloned()
            .collect();

        if remaining_ring.is_empty() {
            warn!("No remaining nodes to receive orphaned shards");
            return Ok(Vec::new());
        }

        for shard_info in orphaned_shards {
            let shard_hash = {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                shard_info.shard_id.hash(&mut hasher);
                hasher.finish()
            };

            let target_node = self.find_node_for_hash(&remaining_ring, shard_hash);
            let transfer_id = self.next_transfer_id.fetch_add(1, Ordering::SeqCst);

            transfers.push(ShardTransfer {
                transfer_id,
                shard_id: shard_info.shard_id,
                source_node: leaving_node_id,
                target_node,
                status: TransferStatus::Pending,
                bytes_transferred: 0,
                total_bytes: shard_info.size_bytes,
                keys_transferred: 0,
                total_keys: shard_info.key_count,
                started_at_ms: now_ms,
                completed_at_ms: 0,
                error: None,
            });

            debug!(
                shard_id = shard_info.shard_id,
                target_node, "Orphaned shard reassigned"
            );
        }

        info!(
            leaving_node_id,
            orphaned_count = transfers.len(),
            "Calculated node leave transfers"
        );

        Ok(transfers)
    }

    /// Start a shard transfer.
    pub async fn start_transfer(&self, transfer_id: TransferId) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        // First, check if transfer exists and is pending
        {
            let transfer = state.transfers.get(&transfer_id).ok_or_else(|| {
                ClusterError::Internal(format!("Transfer {transfer_id} not found"))
            })?;

            if transfer.status != TransferStatus::Pending {
                return Err(ClusterError::Internal(format!(
                    "Transfer {transfer_id} is not in pending state"
                )));
            }
        }

        // Check concurrent transfer limit
        let in_progress_count = state
            .transfers
            .values()
            .filter(|t| t.status == TransferStatus::InProgress)
            .count();

        if in_progress_count >= self.rebalance_config.max_concurrent_transfers as usize {
            return Err(ClusterError::Internal(
                "Maximum concurrent transfers reached".into(),
            ));
        }

        // Get the shard_id before mutating transfer
        let shard_id = state.transfers.get(&transfer_id).map(|t| t.shard_id);

        // Update shard state
        if let Some(sid) = shard_id {
            if let Some(shard) = state.shards.get_mut(&sid) {
                shard.state = ShardState::Transferring;
            }
        }

        // Now update the transfer
        if let Some(transfer) = state.transfers.get_mut(&transfer_id) {
            transfer.status = TransferStatus::InProgress;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            transfer.started_at_ms = now_ms;
        }

        debug!(transfer_id, "Transfer started");

        Ok(())
    }

    /// Update transfer progress.
    pub async fn update_transfer_progress(
        &self,
        transfer_id: TransferId,
        bytes_transferred: u64,
        keys_transferred: u64,
    ) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        let transfer = state
            .transfers
            .get_mut(&transfer_id)
            .ok_or_else(|| ClusterError::Internal(format!("Transfer {transfer_id} not found")))?;

        transfer.bytes_transferred = bytes_transferred;
        transfer.keys_transferred = keys_transferred;

        Ok(())
    }

    /// Complete a shard transfer successfully.
    pub async fn complete_transfer(&self, transfer_id: TransferId) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        // First, update the transfer and extract needed values
        let (shard_id, target_node) = {
            let transfer = state.transfers.get_mut(&transfer_id).ok_or_else(|| {
                ClusterError::Internal(format!("Transfer {transfer_id} not found"))
            })?;

            transfer.status = TransferStatus::Completed;
            transfer.bytes_transferred = transfer.total_bytes;
            transfer.keys_transferred = transfer.total_keys;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            transfer.completed_at_ms = now_ms;

            (transfer.shard_id, transfer.target_node)
        };

        // Get the old primary node
        let old_primary = state.shards.get(&shard_id).map(|s| s.primary_node);

        // Update shard primary node
        if let Some(shard) = state.shards.get_mut(&shard_id) {
            shard.primary_node = target_node;
            shard.state = ShardState::Active;
        }

        // Remove shard from old node's list
        if let Some(old_node) = old_primary {
            if let Some(old_node_shards) = state.node_shards.get_mut(&old_node) {
                old_node_shards.retain(|&id| id != shard_id);
            }
        }

        // Add shard to new node's list
        state
            .node_shards
            .entry(target_node)
            .or_default()
            .push(shard_id);

        info!(transfer_id, shard_id, target_node, "Transfer completed");

        Ok(())
    }

    /// Fail a shard transfer.
    pub async fn fail_transfer(&self, transfer_id: TransferId, error: String) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        // Update transfer and get shard_id
        let shard_id = {
            let transfer = state.transfers.get_mut(&transfer_id).ok_or_else(|| {
                ClusterError::Internal(format!("Transfer {transfer_id} not found"))
            })?;

            transfer.status = TransferStatus::Failed;
            transfer.error = Some(error.clone());
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            transfer.completed_at_ms = now_ms;

            transfer.shard_id
        };

        // Reset shard state
        if let Some(shard) = state.shards.get_mut(&shard_id) {
            shard.state = ShardState::Active;
        }

        warn!(transfer_id, error, "Transfer failed");

        Ok(())
    }

    /// Get current rebalance progress.
    pub async fn get_rebalance_progress(&self) -> RebalanceProgress {
        let state = self.state.read().await;

        if !state.rebalancing {
            return RebalanceProgress::default();
        }

        let total_transfers = state.transfers.len();
        let completed_transfers = state
            .transfers
            .values()
            .filter(|t| t.status == TransferStatus::Completed)
            .count();
        let in_progress_transfers = state
            .transfers
            .values()
            .filter(|t| t.status == TransferStatus::InProgress)
            .count();
        let failed_transfers = state
            .transfers
            .values()
            .filter(|t| t.status == TransferStatus::Failed)
            .count();

        let bytes_transferred: u64 = state.transfers.values().map(|t| t.bytes_transferred).sum();

        let started_at_ms = state.rebalance_started_at.map_or(0, |start| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
                - start.elapsed().as_millis() as u64
        });

        // Calculate throughput and ETA
        let (throughput_bytes_per_sec, eta_seconds) =
            if let Some(start) = state.rebalance_started_at {
                let elapsed_secs = start.elapsed().as_secs_f64();
                if elapsed_secs > 0.0 && bytes_transferred > 0 {
                    let throughput = (bytes_transferred as f64 / elapsed_secs) as u64;
                    let remaining_bytes = state
                        .rebalance_total_bytes
                        .saturating_sub(bytes_transferred);
                    let eta = if throughput > 0 {
                        remaining_bytes / throughput
                    } else {
                        0
                    };
                    (throughput, eta)
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            };

        RebalanceProgress {
            active: true,
            total_transfers,
            completed_transfers,
            in_progress_transfers,
            failed_transfers,
            total_bytes: state.rebalance_total_bytes,
            bytes_transferred,
            eta_seconds,
            started_at_ms,
            throughput_bytes_per_sec,
        }
    }

    /// Get a specific transfer by ID.
    pub async fn get_transfer(&self, transfer_id: TransferId) -> ClusterResult<ShardTransfer> {
        let state = self.state.read().await;

        state
            .transfers
            .get(&transfer_id)
            .cloned()
            .ok_or_else(|| ClusterError::Internal(format!("Transfer {transfer_id} not found")))
    }

    /// Get all pending transfers.
    pub async fn get_pending_transfers(&self) -> Vec<ShardTransfer> {
        let state = self.state.read().await;

        state
            .transfers
            .values()
            .filter(|t| t.status == TransferStatus::Pending)
            .cloned()
            .collect()
    }

    /// Complete rebalancing.
    pub async fn complete_rebalance(&self) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        // Check if all transfers are complete
        let incomplete = state
            .transfers
            .values()
            .any(|t| t.status == TransferStatus::InProgress || t.status == TransferStatus::Pending);

        if incomplete {
            warn!("Completing rebalance with incomplete transfers");
        }

        state.rebalancing = false;
        state.rebalance_started_at = None;
        info!("Shard rebalancing completed");
        Ok(())
    }

    /// Cancel rebalancing.
    pub async fn cancel_rebalance(&self) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        // Cancel all pending transfers
        for transfer in state.transfers.values_mut() {
            if transfer.status == TransferStatus::Pending
                || transfer.status == TransferStatus::InProgress
            {
                transfer.status = TransferStatus::Cancelled;
            }
        }

        // Reset any transferring shards to active
        for shard in state.shards.values_mut() {
            if shard.state == ShardState::Transferring || shard.state == ShardState::Receiving {
                shard.state = ShardState::Active;
            }
        }

        state.rebalancing = false;
        state.rebalance_started_at = None;
        info!("Shard rebalancing cancelled");
        Ok(())
    }

    /// Register a shard with the manager.
    pub async fn register_shard(&self, shard_info: ShardInfo) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        let shard_id = shard_info.shard_id;
        let primary_node = shard_info.primary_node;

        state.shards.insert(shard_id, shard_info);
        state
            .node_shards
            .entry(primary_node)
            .or_default()
            .push(shard_id);

        debug!(shard_id, primary_node, "Shard registered");

        Ok(())
    }

    /// Get ring distribution information showing how virtual nodes are distributed.
    ///
    /// Returns a map of node IDs to the number of virtual nodes they have on the ring.
    pub async fn get_ring_distribution(&self) -> HashMap<NodeId, u32> {
        let state = self.state.read().await;
        let mut distribution: HashMap<NodeId, u32> = HashMap::new();

        for point in &state.ring {
            *distribution.entry(point.node_id).or_insert(0) += 1;
            // Use virtual_index to verify distribution is correct
            debug!(
                node_id = point.node_id,
                virtual_index = point.virtual_index,
                "Ring point"
            );
        }

        distribution
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
            | Ok(i) => i,
            | Err(i) => {
                if i >= ring.len() {
                    0 // Wrap around to first node
                } else {
                    i
                }
            },
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
            | Ok(i) => i,
            | Err(i) => i % ring.len(),
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
