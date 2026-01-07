//! Raft consensus implementation for cluster coordination.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock};
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
    state: Arc<RwLock<ConsensusState>>,
    /// Running flag
    running: Arc<RwLock<bool>>,
    /// Notifier for heartbeat received events
    heartbeat_received: Arc<Notify>,
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
            state: Arc::new(RwLock::new(ConsensusState::default())),
            running: Arc::new(RwLock::new(false)),
            heartbeat_received: Arc::new(Notify::new()),
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

    /// Generate a random election timeout within the configured range.
    fn random_election_timeout(&self) -> Duration {
        Self::generate_random_timeout(
            &self.config.raft.election_timeout_min,
            &self.config.raft.election_timeout_max,
        )
    }

    /// Generate a random timeout between min and max durations.
    fn generate_random_timeout(min: &Duration, max: &Duration) -> Duration {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let min_ms = min.as_millis() as u64;
        let max_ms = max.as_millis() as u64;
        let timeout_ms = rng.gen_range(min_ms..=max_ms);
        Duration::from_millis(timeout_ms)
    }

    /// Start the election timer (background task).
    async fn start_election_timer(&self) {
        debug!(node_id = self.node_id, "Starting election timer");
        
        let node_id = self.node_id;
        let state = self.state.clone();
        let running = self.running.clone();
        let heartbeat_received = self.heartbeat_received.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            loop {
                // Check if still running
                {
                    let is_running = running.read().await;
                    if !*is_running {
                        debug!(node_id, "Election timer stopped");
                        break;
                    }
                }
                
                // Generate random election timeout
                let timeout = RaftConsensus::generate_random_timeout(
                    &config.raft.election_timeout_min,
                    &config.raft.election_timeout_max,
                );
                
                // Wait for timeout or heartbeat
                tokio::select! {
                    () = tokio::time::sleep(timeout) => {
                        // Election timeout reached
                        let current_state = {
                            let state_guard = state.read().await;
                            state_guard.state
                        };
                        
                        // Only start election if we're a follower or candidate
                        if current_state == RaftState::Follower || current_state == RaftState::Candidate {
                            info!(node_id, "Election timeout reached, starting election");
                            
                            // Transition to candidate state
                            {
                                let mut state_guard = state.write().await;
                                state_guard.state = RaftState::Candidate;
                                state_guard.current_term += 1;
                                state_guard.voted_for = Some(node_id);
                            }
                            
                            // In a full implementation, we would:
                            // 1. Request votes from other nodes
                            // 2. Wait for responses
                            // 3. Become leader if we get majority
                        }
                    }
                    () = heartbeat_received.notified() => {
                        // Heartbeat received, reset timer
                        debug!(node_id, "Heartbeat received, resetting election timer");
                    }
                }
            }
        });
    }

    /// Start the heartbeat timer (background task, for leaders).
    async fn start_heartbeat_timer(&self) {
        debug!(node_id = self.node_id, "Starting heartbeat timer");
        
        let node_id = self.node_id;
        let state = self.state.clone();
        let running = self.running.clone();
        let heartbeat_interval = self.config.raft.heartbeat_interval;
        
        tokio::spawn(async move {
            loop {
                // Check if still running
                {
                    let is_running = running.read().await;
                    if !*is_running {
                        debug!(node_id, "Heartbeat timer stopped");
                        break;
                    }
                }
                
                // Wait for heartbeat interval
                tokio::time::sleep(heartbeat_interval).await;
                
                // Send heartbeat if we're the leader
                let is_leader = {
                    let state_guard = state.read().await;
                    state_guard.state == RaftState::Leader
                };
                
                if is_leader {
                    debug!(node_id, "Sending heartbeat to followers");
                    
                    // In a full implementation, we would:
                    // 1. Send AppendEntries RPC with no entries to all followers
                    // 2. Update match_index and next_index for each follower
                    // 3. Commit entries that have been replicated to majority
                }
            }
        });
    }

    /// Notify that a heartbeat was received (to reset election timer).
    pub async fn notify_heartbeat(&self) {
        self.heartbeat_received.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::sync::atomic::{AtomicU16, Ordering};

    // Start port counter at 10000 to avoid conflicts
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(10000);
    
    fn get_test_config() -> ClusterConfig {
        let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut config = ClusterConfig::default();
        config.bind_addr = format!("127.0.0.1:{}", port).parse().unwrap();
        config
    }

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

    #[tokio::test]
    async fn test_consensus_creation() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await;
        
        assert!(consensus.is_ok());
    }

    #[tokio::test]
    async fn test_consensus_start_stop() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        // Start consensus
        let result = consensus.start().await;
        assert!(result.is_ok());
        
        // Verify state changed to Follower
        let state = consensus.state.read().await;
        assert_eq!(state.state, RaftState::Follower);
        drop(state);
        
        // Stop consensus
        let result = consensus.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_heartbeat_notification() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        // Start consensus
        consensus.start().await.unwrap();
        
        // Notify heartbeat (should not panic)
        consensus.notify_heartbeat().await;
        
        // Stop consensus
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_election_timeout_triggers_candidate_state() {
        let mut config = get_test_config();
        // Set very short timeouts for testing
        config.raft.election_timeout_min = Duration::from_millis(50);
        config.raft.election_timeout_max = Duration::from_millis(100);
        
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        // Start consensus
        consensus.start().await.unwrap();
        
        // Wait for election timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Check if state transitioned to Candidate
        let state = consensus.state.read().await;
        assert_eq!(state.state, RaftState::Candidate);
        assert_eq!(state.current_term, 1); // Term should have incremented
        assert_eq!(state.voted_for, Some(1)); // Should have voted for self
        drop(state);
        
        // Stop consensus
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_heartbeat_resets_election_timer() {
        let mut config = get_test_config();
        // Set short timeouts for testing
        config.raft.election_timeout_min = Duration::from_millis(100);
        config.raft.election_timeout_max = Duration::from_millis(150);
        
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        // Start consensus
        consensus.start().await.unwrap();
        
        // Send heartbeats periodically to prevent election
        for _ in 0..5 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            consensus.notify_heartbeat().await;
        }
        
        // After receiving heartbeats, should still be follower
        let state = consensus.state.read().await;
        assert_eq!(state.state, RaftState::Follower);
        drop(state);
        
        // Stop consensus
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_leader_sends_heartbeats() {
        let mut config = get_test_config();
        config.raft.heartbeat_interval = Duration::from_millis(50);
        
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        // Start consensus
        consensus.start().await.unwrap();
        
        // Manually set state to Leader
        {
            let mut state = consensus.state.write().await;
            state.state = RaftState::Leader;
        }
        
        // Wait for a few heartbeat intervals
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Verify still in leader state
        let state = consensus.state.read().await;
        assert_eq!(state.state, RaftState::Leader);
        drop(state);
        
        // Stop consensus
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_random_election_timeout_range() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config.clone()).await.unwrap();
        
        // Generate multiple timeouts and verify they're in range
        for _ in 0..10 {
            let timeout = consensus.random_election_timeout();
            assert!(timeout >= config.raft.election_timeout_min);
            assert!(timeout <= config.raft.election_timeout_max);
        }
    }
}
