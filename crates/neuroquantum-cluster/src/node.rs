//! Cluster node management and lifecycle.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::ClusterConfig;
use crate::consensus::RaftConsensus;
use crate::discovery::DiscoveryService;
use crate::error::{ClusterError, ClusterResult};
use crate::network::NetworkTransport;
use crate::sharding::ShardManager;

/// Unique identifier for a node in the cluster.
pub type NodeId = u64;

/// Role of a node in the Raft cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Node is the leader and handles all write requests
    Leader,
    /// Node is a follower and replicates from the leader
    Follower,
    /// Node is a candidate running for election
    Candidate,
    /// Node is a learner (non-voting member)
    Learner,
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::Leader => write!(f, "Leader"),
            | Self::Follower => write!(f, "Follower"),
            | Self::Candidate => write!(f, "Candidate"),
            | Self::Learner => write!(f, "Learner"),
        }
    }
}

/// Current state of a cluster node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is initializing
    Initializing,
    /// Node is joining the cluster
    Joining,
    /// Node is running normally
    Running,
    /// Node is in read-only mode (network partition detected)
    ReadOnly,
    /// Node is draining connections before upgrade
    Draining,
    /// Node is leaving the cluster gracefully
    Leaving,
    /// Node has stopped
    Stopped,
    /// Node is in an error state
    Error,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::Initializing => write!(f, "Initializing"),
            | Self::Joining => write!(f, "Joining"),
            | Self::Running => write!(f, "Running"),
            | Self::ReadOnly => write!(f, "ReadOnly"),
            | Self::Draining => write!(f, "Draining"),
            | Self::Leaving => write!(f, "Leaving"),
            | Self::Stopped => write!(f, "Stopped"),
            | Self::Error => write!(f, "Error"),
        }
    }
}

/// Information about a peer node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Node identifier
    pub node_id: NodeId,
    /// Node address for cluster communication
    pub addr: SocketAddr,
    /// Current role of the peer
    pub role: NodeRole,
    /// Last time we received a heartbeat from this peer (Unix timestamp in ms)
    #[serde(skip)]
    pub last_heartbeat: Option<Instant>,
    /// Whether the peer is considered healthy
    pub healthy: bool,
    /// Protocol version of the peer
    pub protocol_version: u32,
}

/// Internal state of the cluster node.
pub(crate) struct NodeInner {
    /// Node configuration (used in full implementation)
    pub(crate) config: ClusterConfig,
    /// Current node state
    state: NodeState,
    /// Current node role
    role: NodeRole,
    /// Known peers in the cluster
    peers: Vec<PeerInfo>,
    /// Current leader ID (if known)
    leader_id: Option<NodeId>,
    /// Node start time
    start_time: Instant,
}

/// A node in the NeuroQuantumDB cluster.
///
/// The `ClusterNode` manages the node's lifecycle, handles Raft consensus,
/// and coordinates with other nodes in the cluster.
pub struct ClusterNode {
    /// Node identifier
    node_id: NodeId,
    /// Internal state protected by RwLock
    pub(crate) inner: Arc<RwLock<NodeInner>>,
    /// Raft consensus module
    consensus: Arc<RaftConsensus>,
    /// Network transport layer
    transport: Arc<NetworkTransport>,
    /// Service discovery
    discovery: Arc<DiscoveryService>,
    /// Shard manager for data distribution
    shard_manager: Arc<ShardManager>,
}

impl ClusterNode {
    /// Create a new cluster node with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration is invalid
    /// - The network transport cannot be initialized
    /// - The consensus module cannot be started
    pub async fn new(config: ClusterConfig) -> ClusterResult<Self> {
        config.validate()?;

        let node_id = config.node_id;
        info!(node_id, "Initializing cluster node");

        // Initialize network transport
        let transport = Arc::new(NetworkTransport::new(&config).await?);

        // Initialize service discovery
        let discovery = Arc::new(DiscoveryService::new(&config)?);

        // Initialize shard manager
        let shard_manager = Arc::new(ShardManager::new(&config)?);

        // Initialize Raft consensus
        let consensus =
            Arc::new(RaftConsensus::new(node_id, transport.clone(), config.clone()).await?);

        let inner = Arc::new(RwLock::new(NodeInner {
            config,
            state: NodeState::Initializing,
            role: NodeRole::Follower,
            peers: Vec::new(),
            leader_id: None,
            start_time: Instant::now(),
        }));

        Ok(Self {
            node_id,
            inner,
            consensus,
            transport,
            discovery,
            shard_manager,
        })
    }

    /// Start the cluster node.
    ///
    /// This will:
    /// 1. Start the network transport
    /// 2. Discover peers
    /// 3. Join the Raft cluster
    /// 4. Begin participating in consensus
    pub async fn start(&self) -> ClusterResult<()> {
        info!(node_id = self.node_id, "Starting cluster node");

        {
            let mut inner = self.inner.write().await;
            inner.state = NodeState::Joining;
        }

        // Start network transport
        Arc::clone(&self.transport).start().await?;

        // Discover peers
        let peers = self.discovery.discover().await?;
        debug!(
            node_id = self.node_id,
            peer_count = peers.len(),
            "Discovered peers"
        );

        {
            let mut inner = self.inner.write().await;
            inner.peers = peers;
        }

        // Start Raft consensus
        self.consensus.start().await?;

        {
            let mut inner = self.inner.write().await;
            inner.state = NodeState::Running;
        }

        info!(node_id = self.node_id, "Cluster node started successfully");
        Ok(())
    }

    /// Stop the cluster node gracefully.
    ///
    /// This will:
    /// 1. Notify other nodes of departure
    /// 2. Transfer leadership if this node is the leader
    /// 3. Stop the Raft consensus module
    /// 4. Close network connections
    pub async fn stop(&self) -> ClusterResult<()> {
        info!(node_id = self.node_id, "Stopping cluster node");

        {
            let mut inner = self.inner.write().await;
            inner.state = NodeState::Leaving;
        }

        // If we're the leader, trigger leadership transfer
        if self.is_leader().await {
            warn!(
                node_id = self.node_id,
                "Leader stepping down, transferring leadership"
            );
            self.consensus.transfer_leadership().await?;
        }

        // Stop consensus
        self.consensus.stop().await?;

        // Stop transport
        self.transport.stop().await?;

        {
            let mut inner = self.inner.write().await;
            inner.state = NodeState::Stopped;
        }

        info!(node_id = self.node_id, "Cluster node stopped");
        Ok(())
    }

    /// Get the node ID.
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Get the current node state.
    pub async fn state(&self) -> NodeState {
        self.inner.read().await.state
    }

    /// Get the current node role.
    pub async fn role(&self) -> NodeRole {
        self.inner.read().await.role
    }

    /// Check if this node is the leader.
    pub async fn is_leader(&self) -> bool {
        self.inner.read().await.role == NodeRole::Leader
    }

    /// Get the current leader ID.
    pub async fn leader_id(&self) -> Option<NodeId> {
        self.inner.read().await.leader_id
    }

    /// Get the list of known peers.
    pub async fn peers(&self) -> Vec<PeerInfo> {
        self.inner.read().await.peers.clone()
    }

    /// Get cluster health status.
    pub async fn health(&self) -> ClusterHealth {
        let inner = self.inner.read().await;
        let healthy_peers = inner.peers.iter().filter(|p| p.healthy).count();
        let total_peers = inner.peers.len();

        ClusterHealth {
            node_id: self.node_id,
            state: inner.state,
            role: inner.role,
            leader_id: inner.leader_id,
            healthy_peers,
            total_peers,
            uptime_secs: inner.start_time.elapsed().as_secs(),
        }
    }

    /// Get the shard manager for data distribution.
    #[must_use]
    pub fn shard_manager(&self) -> Arc<ShardManager> {
        self.shard_manager.clone()
    }

    /// Get the consensus module for Raft operations.
    #[must_use]
    pub fn consensus(&self) -> Arc<RaftConsensus> {
        self.consensus.clone()
    }

    /// Check network partition and update node state.
    pub async fn check_network_partition(&self) -> ClusterResult<()> {
        let inner = self.inner.read().await;
        let total_peers = inner.peers.len();
        let healthy_peers = inner.peers.iter().filter(|p| p.healthy).count();
        drop(inner);

        // Update consensus quorum status
        // cluster_size = total_peers + 1 (self)
        self.consensus
            .update_quorum_status(healthy_peers, total_peers + 1)
            .await;

        let quorum_status = self.consensus.quorum_status().await;

        // Update node state based on quorum
        let mut inner = self.inner.write().await;
        match quorum_status {
            | crate::consensus::QuorumStatus::NoQuorum => {
                if inner.state == NodeState::Running {
                    warn!(
                        node_id = self.node_id,
                        healthy_peers,
                        total_peers,
                        "Network partition detected, entering read-only mode"
                    );
                    inner.state = NodeState::ReadOnly;
                }
            },
            | crate::consensus::QuorumStatus::HasQuorum => {
                if inner.state == NodeState::ReadOnly {
                    info!(
                        node_id = self.node_id,
                        "Network partition healed, resuming normal operation"
                    );
                    inner.state = NodeState::Running;
                }
            },
            | crate::consensus::QuorumStatus::Unknown => {
                // Do nothing during initialization
            },
        }

        Ok(())
    }

    /// Check if the node can accept writes.
    pub async fn can_accept_writes(&self) -> bool {
        let inner = self.inner.read().await;
        if inner.state != NodeState::Running {
            return false;
        }
        drop(inner);

        // Must be leader with valid quorum and valid lease
        if !self.is_leader().await {
            return false;
        }

        if self.consensus.quorum_status().await != crate::consensus::QuorumStatus::HasQuorum {
            return false;
        }

        // Check if leader lease is still valid
        self.consensus.is_leader_lease_valid().await
    }

    /// Update peer health status.
    pub async fn update_peer_health(&self, node_id: NodeId, healthy: bool) {
        let mut inner = self.inner.write().await;
        if let Some(peer) = inner.peers.iter_mut().find(|p| p.node_id == node_id) {
            if peer.healthy != healthy {
                debug!(
                    local_node = self.node_id,
                    peer_node = node_id,
                    healthy,
                    "Peer health status changed"
                );
                peer.healthy = healthy;
                if healthy {
                    peer.last_heartbeat = Some(Instant::now());
                }
            }
        }
    }

    /// Prepare the node for upgrade by draining connections.
    ///
    /// This will:
    /// 1. Mark the node as draining
    /// 2. Stop accepting new client connections
    /// 3. Wait for existing connections to complete (up to drain timeout)
    /// 4. If leader, trigger leadership transfer
    ///
    /// # Errors
    ///
    /// Returns an error if the node is not in Running state or if draining fails.
    pub async fn prepare_for_upgrade(&self) -> ClusterResult<()> {
        info!(node_id = self.node_id, "Preparing node for upgrade");

        {
            let mut inner = self.inner.write().await;
            if inner.state != NodeState::Running {
                return Err(ClusterError::InvalidState {
                    expected: "Running".to_string(),
                    actual: format!("{:?}", inner.state),
                });
            }
            inner.state = NodeState::Draining;
        }

        // If we're the leader, transfer leadership before draining
        if self.is_leader().await {
            info!(
                node_id = self.node_id,
                "Node is leader, transferring leadership before upgrade"
            );
            self.consensus.transfer_leadership().await?;

            // Wait for leadership to be transferred
            let timeout = {
                let inner = self.inner.read().await;
                std::time::Duration::from_secs(
                    inner
                        .config
                        .manager
                        .upgrades
                        .leadership_transfer_timeout_secs,
                )
            };
            let start = std::time::Instant::now();
            while self.is_leader().await && start.elapsed() < timeout {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            if self.is_leader().await {
                warn!(
                    node_id = self.node_id,
                    "Leadership transfer timed out, proceeding anyway"
                );
            }
        }

        // Drain connections
        let drain_timeout = {
            let inner = self.inner.read().await;
            std::time::Duration::from_secs(inner.config.manager.upgrades.drain_timeout_secs)
        };

        info!(
            node_id = self.node_id,
            timeout_secs = drain_timeout.as_secs(),
            "Draining connections"
        );

        // In a real implementation, we would:
        // 1. Stop accepting new connections
        // 2. Wait for active connections to complete or timeout
        // For now, just wait for the timeout
        tokio::time::sleep(drain_timeout).await;

        info!(node_id = self.node_id, "Node prepared for upgrade");
        Ok(())
    }

    /// Perform health check after upgrade.
    ///
    /// Verifies that the node is healthy and ready to rejoin the cluster.
    ///
    /// # Errors
    ///
    /// Returns an error if health checks fail.
    pub async fn post_upgrade_health_check(&self) -> ClusterResult<()> {
        info!(
            node_id = self.node_id,
            "Performing post-upgrade health check"
        );

        // Check if consensus is running
        if !*self.consensus.running.read().await {
            return Err(ClusterError::HealthCheckFailed(
                "Consensus module not running".into(),
            ));
        }

        // Check if we can reach peers
        let healthy_peers = {
            let inner = self.inner.read().await;
            inner.peers.iter().filter(|p| p.healthy).count()
        };

        let min_healthy = {
            let inner = self.inner.read().await;
            inner.config.manager.upgrades.min_healthy_nodes
        };

        if healthy_peers < min_healthy {
            return Err(ClusterError::HealthCheckFailed(format!(
                "Not enough healthy peers: {} < {}",
                healthy_peers, min_healthy
            )));
        }

        info!(node_id = self.node_id, healthy_peers, "Health check passed");
        Ok(())
    }

    /// Check compatibility with peer protocol versions.
    ///
    /// # Errors
    ///
    /// Returns an error if any peer has an incompatible protocol version.
    pub async fn check_protocol_compatibility(&self) -> ClusterResult<()> {
        let inner = self.inner.read().await;
        let our_version = inner.config.manager.upgrades.protocol_version;
        let min_compatible = inner.config.manager.upgrades.min_compatible_version;

        for peer in &inner.peers {
            if peer.protocol_version < min_compatible {
                return Err(ClusterError::ProtocolVersionMismatch {
                    node_id: peer.node_id,
                    expected: min_compatible,
                    actual: peer.protocol_version,
                });
            }

            if peer.protocol_version < our_version && our_version - peer.protocol_version > 1 {
                warn!(
                    node_id = self.node_id,
                    peer_id = peer.node_id,
                    our_version,
                    peer_version = peer.protocol_version,
                    "Large protocol version difference detected"
                );
            }
        }

        info!(
            node_id = self.node_id,
            protocol_version = our_version,
            "Protocol compatibility check passed"
        );
        Ok(())
    }

    /// Get the protocol version of this node.
    pub async fn protocol_version(&self) -> u32 {
        let inner = self.inner.read().await;
        inner.config.manager.upgrades.protocol_version
    }

    /// Update the protocol version of a peer.
    pub async fn update_peer_protocol_version(&self, node_id: NodeId, version: u32) {
        let mut inner = self.inner.write().await;
        if let Some(peer) = inner.peers.iter_mut().find(|p| p.node_id == node_id) {
            if peer.protocol_version != version {
                info!(
                    local_node = self.node_id,
                    peer_node = node_id,
                    old_version = peer.protocol_version,
                    new_version = version,
                    "Peer protocol version changed"
                );
                peer.protocol_version = version;
            }
        }
    }
}

/// Health status of the cluster node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealth {
    /// Node identifier
    pub node_id: NodeId,
    /// Current node state
    pub state: NodeState,
    /// Current node role
    pub role: NodeRole,
    /// Current leader ID (if known)
    pub leader_id: Option<NodeId>,
    /// Number of healthy peers
    pub healthy_peers: usize,
    /// Total number of peers
    pub total_peers: usize,
    /// Uptime in seconds
    pub uptime_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_role_display() {
        assert_eq!(format!("{}", NodeRole::Leader), "Leader");
        assert_eq!(format!("{}", NodeRole::Follower), "Follower");
        assert_eq!(format!("{}", NodeRole::Candidate), "Candidate");
        assert_eq!(format!("{}", NodeRole::Learner), "Learner");
    }

    #[test]
    fn test_node_state_display() {
        assert_eq!(format!("{}", NodeState::Initializing), "Initializing");
        assert_eq!(format!("{}", NodeState::Running), "Running");
        assert_eq!(format!("{}", NodeState::Stopped), "Stopped");
    }
}
