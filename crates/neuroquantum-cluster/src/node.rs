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
use crate::error::ClusterResult;
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
            Self::Leader => write!(f, "Leader"),
            Self::Follower => write!(f, "Follower"),
            Self::Candidate => write!(f, "Candidate"),
            Self::Learner => write!(f, "Learner"),
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
            Self::Initializing => write!(f, "Initializing"),
            Self::Joining => write!(f, "Joining"),
            Self::Running => write!(f, "Running"),
            Self::Leaving => write!(f, "Leaving"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Error => write!(f, "Error"),
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
}

/// Internal state of the cluster node.
#[allow(dead_code)]
struct NodeInner {
    /// Node configuration (used in full implementation)
    config: ClusterConfig,
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
    inner: Arc<RwLock<NodeInner>>,
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

    /// Update the node's role.
    #[allow(dead_code)]
    pub(crate) async fn set_role(&self, role: NodeRole) {
        let mut inner = self.inner.write().await;
        if inner.role != role {
            info!(
                node_id = self.node_id,
                old_role = %inner.role,
                new_role = %role,
                "Node role changed"
            );
            inner.role = role;
        }
    }

    /// Update the known leader ID.
    #[allow(dead_code)]
    pub(crate) async fn set_leader(&self, leader_id: Option<NodeId>) {
        let mut inner = self.inner.write().await;
        if inner.leader_id != leader_id {
            info!(
                node_id = self.node_id,
                old_leader = ?inner.leader_id,
                new_leader = ?leader_id,
                "Leader changed"
            );
            inner.leader_id = leader_id;
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
