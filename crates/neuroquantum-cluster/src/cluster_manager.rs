//! Cluster Manager - High-level coordination for multi-node deployments.
//!
//! This module provides the main entry point for cluster management,
//! coordinating between consensus, replication, sharding, and discovery.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::ClusterConfig;
use crate::consensus::RaftConsensus;
use crate::discovery::DiscoveryService;
use crate::error::{ClusterError, ClusterResult};
use crate::metrics::ClusterMetrics;
use crate::node::{ClusterNode, NodeId, NodeRole, NodeState, PeerInfo};
use crate::replication::ReplicationManager;
use crate::sharding::ShardManager;

/// Cluster status for health checks and monitoring.
#[derive(Debug, Clone)]
pub struct ClusterStatus {
    /// Whether the cluster is healthy
    pub healthy: bool,
    /// Current cluster leader (if known)
    pub leader_id: Option<NodeId>,
    /// Number of nodes in the cluster
    pub node_count: usize,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Whether quorum is available
    pub has_quorum: bool,
    /// Local node state
    pub local_state: NodeState,
    /// Local node role
    pub local_role: NodeRole,
}

/// Internal state for the cluster manager.
struct ClusterManagerState {
    /// Whether the cluster manager is running
    running: bool,
    /// Known peers in the cluster
    peers: Vec<PeerInfo>,
    /// Last successful health check time
    last_health_check_ms: u64,
}

/// High-level cluster manager that coordinates all components.
pub struct ClusterManager {
    /// Local cluster node
    node: Arc<ClusterNode>,
    /// Configuration
    config: ClusterConfig,
    /// Internal state
    state: Arc<RwLock<ClusterManagerState>>,
    /// Replication manager
    replication_manager: Arc<ReplicationManager>,
    /// Cluster metrics
    metrics: Arc<ClusterMetrics>,
}

impl ClusterManager {
    /// Create a new cluster manager.
    ///
    /// # Errors
    ///
    /// Returns an error if the cluster node cannot be initialized.
    pub async fn new(config: ClusterConfig) -> ClusterResult<Self> {
        info!(node_id = config.node_id, "Initializing cluster manager");

        // Create the cluster node
        let node = Arc::new(ClusterNode::new(config.clone()).await?);

        // Create the replication manager
        let replication_manager = Arc::new(ReplicationManager::new(
            config.node_id,
            config.sharding.replication_factor,
            Duration::from_secs(30),
        ));

        // Create metrics
        let metrics = Arc::new(ClusterMetrics::new(config.node_id));

        let state = Arc::new(RwLock::new(ClusterManagerState {
            running: false,
            peers: Vec::new(),
            last_health_check_ms: 0,
        }));

        Ok(Self {
            node,
            config,
            state,
            replication_manager,
            metrics,
        })
    }

    /// Start the cluster manager.
    ///
    /// This will:
    /// 1. Start the underlying cluster node
    /// 2. Begin background health monitoring
    /// 3. Start metrics collection
    pub async fn start(&self) -> ClusterResult<()> {
        info!(node_id = self.config.node_id, "Starting cluster manager");

        {
            let mut state = self.state.write().await;
            state.running = true;
        }

        // Start the cluster node
        self.node.start().await?;

        // Start background tasks
        self.start_health_monitor().await;
        self.start_peer_discovery().await;
        self.start_replication_cleanup().await;

        // Update metrics
        self.metrics.record_start();

        info!(
            node_id = self.config.node_id,
            "Cluster manager started successfully"
        );
        Ok(())
    }

    /// Stop the cluster manager gracefully.
    pub async fn stop(&self) -> ClusterResult<()> {
        info!(node_id = self.config.node_id, "Stopping cluster manager");

        {
            let mut state = self.state.write().await;
            state.running = false;
        }

        // Stop the cluster node
        self.node.stop().await?;

        info!(node_id = self.config.node_id, "Cluster manager stopped");
        Ok(())
    }

    /// Get the current cluster status.
    pub async fn status(&self) -> ClusterStatus {
        let state = self.state.read().await;
        let node_state = self.node.state().await;
        let node_role = self.node.role().await;
        let leader_id = self.node.leader_id().await;
        let peers = &state.peers;

        let healthy_nodes = peers.iter().filter(|p| p.healthy).count();
        let node_count = peers.len() + 1; // Include self
        let quorum_size = (node_count / 2) + 1;
        let has_quorum = healthy_nodes + 1 >= quorum_size; // +1 for self

        ClusterStatus {
            healthy: node_state == NodeState::Running && has_quorum,
            leader_id,
            node_count,
            healthy_nodes: healthy_nodes + 1, // Include self as healthy
            has_quorum,
            local_state: node_state,
            local_role: node_role,
        }
    }

    /// Get the underlying cluster node.
    #[must_use]
    pub fn node(&self) -> Arc<ClusterNode> {
        Arc::clone(&self.node)
    }

    /// Get the shard manager.
    #[must_use]
    pub fn shard_manager(&self) -> Arc<ShardManager> {
        self.node.shard_manager()
    }

    /// Get the replication manager.
    #[must_use]
    pub fn replication_manager(&self) -> Arc<ReplicationManager> {
        Arc::clone(&self.replication_manager)
    }

    /// Get the cluster metrics.
    #[must_use]
    pub fn metrics(&self) -> Arc<ClusterMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Get the consensus module.
    #[must_use]
    pub fn consensus(&self) -> Arc<RaftConsensus> {
        self.node.consensus()
    }

    /// Check if this node is the leader.
    pub async fn is_leader(&self) -> bool {
        self.node.is_leader().await
    }

    /// Get the node ID.
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        self.node.node_id()
    }

    /// Propose a command to be replicated across the cluster.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - This node is not the leader
    /// - The command cannot be proposed
    pub async fn propose(&self, command: Vec<u8>) -> ClusterResult<u64> {
        if !self.is_leader().await {
            return Err(ClusterError::NotLeader(
                self.node_id(),
                self.node.leader_id().await,
            ));
        }

        // Propose through Raft consensus
        let index = self.consensus().propose(command).await?;

        // Update metrics
        self.metrics.record_proposal();

        Ok(index)
    }

    /// Start background health monitoring.
    async fn start_health_monitor(&self) {
        debug!(
            node_id = self.config.node_id,
            "Starting health monitor task"
        );

        let state = Arc::clone(&self.state);
        let node = Arc::clone(&self.node);
        let metrics = Arc::clone(&self.metrics);
        let node_id = self.config.node_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Check if still running
                {
                    let s = state.read().await;
                    if !s.running {
                        debug!(node_id, "Health monitor stopped");
                        break;
                    }
                }

                // Get health status
                let health = node.health().await;

                // Update metrics
                metrics.record_health_check(&health);

                // Update last health check time
                {
                    let mut s = state.write().await;
                    s.last_health_check_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                }

                debug!(
                    node_id,
                    state = %health.state,
                    role = %health.role,
                    healthy_peers = health.healthy_peers,
                    "Health check completed"
                );
            }
        });
    }

    /// Start background peer discovery.
    async fn start_peer_discovery(&self) {
        debug!(
            node_id = self.config.node_id,
            "Starting peer discovery task"
        );

        let state = Arc::clone(&self.state);
        let config = self.config.clone();
        let node_id = self.config.node_id;
        let shard_manager = self.shard_manager();

        tokio::spawn(async move {
            let discovery = match DiscoveryService::new(&config) {
                Ok(d) => d,
                Err(e) => {
                    error!(node_id, error = %e, "Failed to create discovery service");
                    return;
                }
            };

            let mut interval = tokio::time::interval(config.discovery.refresh_interval);

            loop {
                interval.tick().await;

                // Check if still running
                {
                    let s = state.read().await;
                    if !s.running {
                        debug!(node_id, "Peer discovery stopped");
                        break;
                    }
                }

                // Discover peers
                match discovery.discover().await {
                    Ok(peers) => {
                        let peer_count = peers.len();

                        // Update state with discovered peers
                        {
                            let mut s = state.write().await;
                            s.peers = peers.clone();
                        }

                        // Update shard manager with discovered nodes
                        for peer in &peers {
                            if let Err(e) = shard_manager.add_node(peer.node_id).await {
                                // Ignore if node already exists
                                if !matches!(e, ClusterError::NodeAlreadyExists(_)) {
                                    warn!(
                                        node_id,
                                        peer_id = peer.node_id,
                                        error = %e,
                                        "Failed to add peer to shard manager"
                                    );
                                }
                            }
                        }

                        debug!(node_id, peer_count, "Peer discovery completed");
                    }
                    Err(e) => {
                        warn!(node_id, error = %e, "Peer discovery failed");
                    }
                }
            }
        });
    }

    /// Start background replication cleanup.
    async fn start_replication_cleanup(&self) {
        debug!(
            node_id = self.config.node_id,
            "Starting replication cleanup task"
        );

        let state = Arc::clone(&self.state);
        let replication_manager = Arc::clone(&self.replication_manager);
        let node_id = self.config.node_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                // Check if still running
                {
                    let s = state.read().await;
                    if !s.running {
                        debug!(node_id, "Replication cleanup stopped");
                        break;
                    }
                }

                // Cleanup old replication requests
                let removed = replication_manager.cleanup().await;
                if removed > 0 {
                    debug!(
                        node_id,
                        removed_count = removed,
                        "Cleaned up old replication requests"
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_config() -> ClusterConfig {
        use std::sync::atomic::{AtomicU16, Ordering};
        static PORT_COUNTER: AtomicU16 = AtomicU16::new(20000);

        let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        ClusterConfig {
            node_id: 1,
            bind_addr: format!("127.0.0.1:{}", port).parse().unwrap(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_cluster_manager_creation() {
        let config = get_test_config();
        let manager = ClusterManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_manager_start_stop() {
        let config = get_test_config();
        let manager = ClusterManager::new(config).await.unwrap();

        // Start
        let start_result = manager.start().await;
        assert!(start_result.is_ok());

        // Check status
        let status = manager.status().await;
        assert_eq!(status.local_state, NodeState::Running);

        // Stop
        let stop_result = manager.stop().await;
        assert!(stop_result.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_manager_status() {
        let config = get_test_config();
        let manager = ClusterManager::new(config).await.unwrap();

        manager.start().await.unwrap();

        let status = manager.status().await;
        assert_eq!(status.node_count, 1); // Just self
        assert_eq!(status.healthy_nodes, 1); // Self is healthy
        assert!(status.has_quorum); // Single node has quorum

        manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_propose_not_leader() {
        let config = get_test_config();
        let manager = ClusterManager::new(config).await.unwrap();

        manager.start().await.unwrap();

        // Initially not a leader
        let result = manager.propose(b"test command".to_vec()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClusterError::NotLeader(_, _)));

        manager.stop().await.unwrap();
    }
}
