//! Raft consensus implementation for cluster coordination.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::config::ClusterConfig;
use crate::error::{ClusterError, ClusterResult};
use crate::network::NetworkTransport;
use crate::node::NodeId;

/// Log entry in the Raft log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Term when entry was received by leader
    pub term: u64,
    /// Position of entry in log
    pub index: u64,
    /// Entry data
    pub data: LogEntryData,
}

/// Types of log entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEntryData {
    /// No-op entry (used for leader establishment)
    Noop,
    /// Configuration change (membership change)
    ConfigChange(ConfigChange),
    /// Data operation
    Command(Vec<u8>),
}

/// Configuration change for cluster membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    /// Type of configuration change
    pub change_type: ConfigChangeType,
    /// Node ID affected by the change
    pub node_id: NodeId,
    /// Node address (for AddNode)
    pub address: Option<String>,
}

/// Types of configuration changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigChangeType {
    /// Add a new node to the cluster
    AddNode,
    /// Remove a node from the cluster
    RemoveNode,
    /// Promote a learner to voter
    PromoteLearner,
}

/// Raft state for the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftState {
    /// Initial state
    Init,
    /// Follower state
    Follower,
    /// Candidate state (running for election)
    Candidate,
    /// Leader state
    Leader,
}

/// Internal consensus state.
#[allow(dead_code)]
struct ConsensusState {
    /// Current term
    current_term: u64,
    /// Voted for in current term (used in full Raft implementation)
    voted_for: Option<NodeId>,
    /// Current state
    state: RaftState,
    /// Commit index (highest log entry known to be committed)
    commit_index: u64,
    /// Last applied (highest log entry applied to state machine)
    last_applied: u64,
    /// Current leader (if known)
    current_leader: Option<NodeId>,
    /// Log entries
    log: Vec<LogEntry>,
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            state: RaftState::Init,
            commit_index: 0,
            last_applied: 0,
            current_leader: None,
            log: Vec::new(),
        }
    }
}

/// Raft consensus module for the cluster.
#[allow(dead_code)]
pub struct RaftConsensus {
    /// Node ID
    node_id: NodeId,
    /// Network transport for communication (used in full implementation)
    transport: Arc<NetworkTransport>,
    /// Configuration (used in full implementation)
    config: ClusterConfig,
    /// Internal state
    state: RwLock<ConsensusState>,
    /// Running flag
    running: RwLock<bool>,
}

impl RaftConsensus {
    /// Create a new Raft consensus module.
    pub async fn new(
        node_id: NodeId,
        transport: Arc<NetworkTransport>,
        config: ClusterConfig,
    ) -> ClusterResult<Self> {
        info!(node_id, "Creating Raft consensus module");

        Ok(Self {
            node_id,
            transport,
            config,
            state: RwLock::new(ConsensusState::default()),
            running: RwLock::new(false),
        })
    }

    /// Start the consensus module.
    pub async fn start(&self) -> ClusterResult<()> {
        info!(node_id = self.node_id, "Starting Raft consensus");

        {
            let mut running = self.running.write().await;
            *running = true;
        }

        {
            let mut state = self.state.write().await;
            state.state = RaftState::Follower;
        }

        // Start background tasks
        self.start_election_timer().await;
        self.start_heartbeat_timer().await;

        info!(node_id = self.node_id, "Raft consensus started");
        Ok(())
    }

    /// Stop the consensus module.
    pub async fn stop(&self) -> ClusterResult<()> {
        info!(node_id = self.node_id, "Stopping Raft consensus");

        {
            let mut running = self.running.write().await;
            *running = false;
        }

        Ok(())
    }

    /// Transfer leadership to another node.
    pub async fn transfer_leadership(&self) -> ClusterResult<()> {
        let state = self.state.read().await;
        if state.state != RaftState::Leader {
            return Err(ClusterError::NotLeader(self.node_id, state.current_leader));
        }

        info!(node_id = self.node_id, "Initiating leadership transfer");

        // In a full implementation, we would:
        // 1. Stop accepting new proposals
        // 2. Send TimeoutNow to the best follower
        // 3. Wait for new leader election

        Ok(())
    }

    /// Get the current term.
    pub async fn current_term(&self) -> u64 {
        self.state.read().await.current_term
    }

    /// Get the current leader.
    pub async fn current_leader(&self) -> Option<NodeId> {
        self.state.read().await.current_leader
    }

    /// Check if this node is the leader.
    pub async fn is_leader(&self) -> bool {
        self.state.read().await.state == RaftState::Leader
    }

    /// Propose a new command to be replicated.
    pub async fn propose(&self, data: Vec<u8>) -> ClusterResult<u64> {
        let mut state = self.state.write().await;

        if state.state != RaftState::Leader {
            return Err(ClusterError::NotLeader(self.node_id, state.current_leader));
        }

        let index = state.log.len() as u64 + 1;
        let entry = LogEntry {
            term: state.current_term,
            index,
            data: LogEntryData::Command(data),
        };

        state.log.push(entry);
        debug!(node_id = self.node_id, index, "Proposed new log entry");

        Ok(index)
    }

    /// Get the commit index.
    pub async fn commit_index(&self) -> u64 {
        self.state.read().await.commit_index
    }

    /// Get the last applied index.
    pub async fn last_applied(&self) -> u64 {
        self.state.read().await.last_applied
    }

    /// Start the election timer (background task).
    async fn start_election_timer(&self) {
        debug!(node_id = self.node_id, "Starting election timer");
        // In a full implementation, this would spawn a tokio task
        // that triggers elections when no heartbeat is received
    }

    /// Start the heartbeat timer (background task, for leaders).
    async fn start_heartbeat_timer(&self) {
        debug!(node_id = self.node_id, "Starting heartbeat timer");
        // In a full implementation, this would spawn a tokio task
        // that sends heartbeats to followers when this node is leader
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_state() {
        assert_eq!(RaftState::Init, RaftState::Init);
        assert_ne!(RaftState::Leader, RaftState::Follower);
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            term: 1,
            index: 1,
            data: LogEntryData::Noop,
        };

        let serialized = serde_json::to_string(&entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.term, 1);
        assert_eq!(deserialized.index, 1);
    }
}
