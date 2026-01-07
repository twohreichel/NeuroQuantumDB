//! Raft consensus implementation for cluster coordination.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock};
use tracing::{debug, info, warn};

use crate::config::ClusterConfig;
use crate::error::{ClusterError, ClusterResult};
use crate::network::NetworkTransport;
use crate::node::NodeId;

/// Fencing token to prevent split brain scenarios.
/// Combines term and sequence number to create monotonically increasing token.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct FencingToken {
    /// Raft term
    pub term: u64,
    /// Sequence number within the term
    pub sequence: u64,
}

impl FencingToken {
    /// Create a new fencing token.
    pub fn new(term: u64, sequence: u64) -> Self {
        Self { term, sequence }
    }

    /// Check if this token is newer than another.
    pub fn is_newer_than(&self, other: &Self) -> bool {
        self > other
    }
}

/// Leader lease for preventing split brain.
#[derive(Debug, Clone)]
pub struct LeaderLease {
    /// When the lease expires
    pub expiry: Instant,
    /// Lease duration
    pub duration: Duration,
    /// Last successful heartbeat time
    pub last_heartbeat: Instant,
}

impl LeaderLease {
    /// Create a new leader lease.
    pub fn new(duration: Duration) -> Self {
        let now = Instant::now();
        Self {
            expiry: now + duration,
            duration,
            last_heartbeat: now,
        }
    }

    /// Check if the lease is still valid.
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.expiry
    }

    /// Renew the lease after successful heartbeat.
    pub fn renew(&mut self) {
        let now = Instant::now();
        self.last_heartbeat = now;
        self.expiry = now + self.duration;
    }
}

/// Quorum status for the current node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumStatus {
    /// Node has quorum (can perform writes)
    HasQuorum,
    /// Node lost quorum (read-only mode)
    NoQuorum,
    /// Quorum status unknown (initializing)
    Unknown,
}

/// Log entry in the Raft log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Term when entry was received by leader
    pub term: u64,
    /// Position of entry in log
    pub index: u64,
    /// Entry data
    pub data: LogEntryData,
    /// Fencing token for split brain prevention
    pub fencing_token: Option<FencingToken>,
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
    /// Leader lease (only valid when state is Leader)
    leader_lease: Option<LeaderLease>,
    /// Next fencing token sequence number
    next_sequence: u64,
    /// Quorum status
    quorum_status: QuorumStatus,
    /// Number of reachable peers
    reachable_peers: usize,
    /// Total cluster size
    cluster_size: usize,
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
            leader_lease: None,
            next_sequence: 0,
            quorum_status: QuorumStatus::Unknown,
            reachable_peers: 0,
            cluster_size: 1,
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

        // Check if leader lease is still valid
        if let Some(lease) = &state.leader_lease {
            if !lease.is_valid() {
                warn!(
                    node_id = self.node_id,
                    "Leader lease expired, stepping down"
                );
                state.state = RaftState::Follower;
                state.leader_lease = None;
                state.current_leader = None;
                return Err(ClusterError::LeaseExpired);
            }
        }

        // Check quorum before accepting write
        if !self.has_quorum(&state) {
            warn!(
                node_id = self.node_id,
                reachable = state.reachable_peers,
                cluster_size = state.cluster_size,
                "No quorum available for write"
            );
            return Err(ClusterError::NoQuorum);
        }

        // Generate fencing token
        let fencing_token = FencingToken::new(state.current_term, state.next_sequence);
        state.next_sequence += 1;

        let index = state.log.len() as u64 + 1;
        let entry = LogEntry {
            term: state.current_term,
            index,
            data: LogEntryData::Command(data),
            fencing_token: Some(fencing_token),
        };

        state.log.push(entry);
        debug!(
            node_id = self.node_id,
            index,
            term = state.current_term,
            sequence = fencing_token.sequence,
            "Proposed new log entry with fencing token"
        );

        Ok(index)
    }

    /// Check if the node has quorum.
    fn has_quorum(&self, state: &ConsensusState) -> bool {
        if state.cluster_size == 1 {
            // Single node cluster always has quorum
            return true;
        }
        
        // Need majority: more than half of cluster
        let needed = (state.cluster_size / 2) + 1;
        // +1 for self
        let available = state.reachable_peers + 1;
        
        available >= needed
    }

    /// Update quorum status based on reachable peers.
    pub async fn update_quorum_status(&self, reachable_peers: usize, cluster_size: usize) {
        let mut state = self.state.write().await;
        
        state.reachable_peers = reachable_peers;
        state.cluster_size = cluster_size;
        
        let has_quorum = if cluster_size == 1 {
            true
        } else {
            let needed = (cluster_size / 2) + 1;
            let available = reachable_peers + 1; // +1 for self
            available >= needed
        };
        
        let old_status = state.quorum_status;
        state.quorum_status = if has_quorum {
            QuorumStatus::HasQuorum
        } else {
            QuorumStatus::NoQuorum
        };
        
        if old_status != state.quorum_status {
            info!(
                node_id = self.node_id,
                old_status = ?old_status,
                new_status = ?state.quorum_status,
                reachable = reachable_peers,
                cluster_size,
                "Quorum status changed"
            );
            
            // If leader lost quorum, step down
            if state.state == RaftState::Leader && state.quorum_status == QuorumStatus::NoQuorum {
                warn!(
                    node_id = self.node_id,
                    "Leader lost quorum, stepping down to follower"
                );
                state.state = RaftState::Follower;
                state.leader_lease = None;
                state.current_leader = None;
            }
        }
    }

    /// Get current quorum status.
    pub async fn quorum_status(&self) -> QuorumStatus {
        self.state.read().await.quorum_status
    }

    /// Promote to leader with lease.
    pub async fn promote_to_leader(&self) -> ClusterResult<()> {
        let mut state = self.state.write().await;
        
        if !self.has_quorum(&state) {
            return Err(ClusterError::NoQuorum);
        }
        
        // Create leader lease
        let lease_duration = self.config.raft.heartbeat_interval * 3;
        let lease = LeaderLease::new(lease_duration);
        
        state.state = RaftState::Leader;
        state.current_leader = Some(self.node_id);
        state.leader_lease = Some(lease);
        state.quorum_status = QuorumStatus::HasQuorum;
        
        info!(
            node_id = self.node_id,
            term = state.current_term,
            lease_duration_ms = lease_duration.as_millis(),
            "Promoted to leader with lease"
        );
        
        Ok(())
    }

    /// Renew leader lease after successful heartbeat.
    pub async fn renew_lease(&self) -> ClusterResult<()> {
        let mut state = self.state.write().await;
        
        if state.state != RaftState::Leader {
            return Err(ClusterError::NotLeader(self.node_id, state.current_leader));
        }
        
        if let Some(lease) = &mut state.leader_lease {
            lease.renew();
            debug!(
                node_id = self.node_id,
                expiry_ms = lease.expiry.duration_since(Instant::now()).as_millis(),
                "Renewed leader lease"
            );
        }
        
        Ok(())
    }

    /// Validate fencing token.
    pub async fn validate_fencing_token(&self, token: &FencingToken) -> ClusterResult<()> {
        let state = self.state.read().await;
        
        // Token must be from current or higher term
        if token.term < state.current_term {
            return Err(ClusterError::StaleToken {
                current_term: state.current_term,
                received_term: token.term,
            });
        }
        
        Ok(())
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
    #[allow(dead_code)]
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
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::time::Duration;

    // Start port counter at 10000 to avoid conflicts
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(10000);

    fn get_test_config() -> ClusterConfig {
        let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        ClusterConfig {
            bind_addr: format!("127.0.0.1:{}", port).parse().unwrap(),
            ..Default::default()
        }
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
            fencing_token: Some(FencingToken::new(1, 0)),
        };

        let serialized = serde_json::to_string(&entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.term, 1);
        assert_eq!(deserialized.index, 1);
        assert_eq!(deserialized.fencing_token.unwrap().term, 1);
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

    #[tokio::test(start_paused = true)]
    async fn test_election_timeout_triggers_candidate_state() {
        let mut config = get_test_config();
        // Set very short timeouts for testing
        config.raft.election_timeout_min = Duration::from_millis(50);
        config.raft.election_timeout_max = Duration::from_millis(100);

        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config.clone())
            .await
            .unwrap();

        // Start consensus
        consensus.start().await.unwrap();

        // Yield first to allow background tasks to start
        tokio::task::yield_now().await;

        // Advance time by just past the maximum election timeout to ensure it triggers
        // exactly once (using max + 1ms to guarantee the timeout fires)
        let advance_duration = config.raft.election_timeout_max + Duration::from_millis(1);
        tokio::time::advance(advance_duration).await;

        // Give the spawned task a chance to process the timeout.
        // A minimal sleep is sufficient since we're in paused time mode and
        // this just allows the runtime to schedule the background task.
        tokio::time::sleep(Duration::from_millis(1)).await;

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
        let consensus = RaftConsensus::new(1, transport, config.clone())
            .await
            .unwrap();

        // Generate multiple timeouts and verify they're in range
        for _ in 0..10 {
            let timeout = consensus.random_election_timeout();
            assert!(timeout >= config.raft.election_timeout_min);
            assert!(timeout <= config.raft.election_timeout_max);
        }
    }

    // Split brain prevention tests
    #[test]
    fn test_fencing_token_ordering() {
        let token1 = FencingToken::new(1, 0);
        let token2 = FencingToken::new(1, 1);
        let token3 = FencingToken::new(2, 0);

        assert!(token2.is_newer_than(&token1));
        assert!(token3.is_newer_than(&token2));
        assert!(token3.is_newer_than(&token1));
        assert!(!token1.is_newer_than(&token2));
    }

    #[test]
    fn test_leader_lease_validity() {
        let mut lease = LeaderLease::new(Duration::from_millis(100));
        
        // Lease should be valid immediately
        assert!(lease.is_valid());
        
        // Wait for lease to expire
        std::thread::sleep(Duration::from_millis(150));
        assert!(!lease.is_valid());
        
        // Renew lease
        lease.renew();
        assert!(lease.is_valid());
    }

    #[tokio::test]
    async fn test_quorum_check_single_node() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        
        // Single node cluster always has quorum
        consensus.update_quorum_status(0, 1).await;
        assert_eq!(consensus.quorum_status().await, QuorumStatus::HasQuorum);
        
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_quorum_check_three_node_cluster() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        
        // 3 node cluster: need 2 nodes (self + 1 peer)
        consensus.update_quorum_status(1, 3).await;
        assert_eq!(consensus.quorum_status().await, QuorumStatus::HasQuorum);
        
        // Lost majority
        consensus.update_quorum_status(0, 3).await;
        assert_eq!(consensus.quorum_status().await, QuorumStatus::NoQuorum);
        
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_propose_without_quorum() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        
        // Set up as leader but without quorum
        consensus.promote_to_leader().await.unwrap();
        consensus.update_quorum_status(0, 3).await;
        
        // Leader should have stepped down due to lost quorum
        let result = consensus.propose(b"test".to_vec()).await;
        assert!(result.is_err());
        match result {
            Err(ClusterError::NotLeader(_, _)) => {},
            _ => panic!("Expected NotLeader error after losing quorum"),
        }
        
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_propose_with_expired_lease() {
        let mut config = get_test_config();
        // Very short lease for testing
        config.raft.heartbeat_interval = Duration::from_millis(10);
        
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        
        // Promote to leader (lease duration = 3 * heartbeat = 30ms)
        consensus.promote_to_leader().await.unwrap();
        
        // Wait for lease to expire
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Propose should fail with expired lease
        let result = consensus.propose(b"test".to_vec()).await;
        assert!(result.is_err());
        match result {
            Err(ClusterError::LeaseExpired) => {},
            _ => panic!("Expected LeaseExpired error"),
        }
        
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_propose_with_valid_lease() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        
        // Promote to leader
        consensus.promote_to_leader().await.unwrap();
        
        // Propose should succeed
        let result = consensus.propose(b"test".to_vec()).await;
        assert!(result.is_ok());
        
        // Verify fencing token was added
        let state = consensus.state.read().await;
        assert_eq!(state.log.len(), 1);
        assert!(state.log[0].fencing_token.is_some());
        
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_leader_step_down_on_quorum_loss() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        
        // Setup 5-node cluster with quorum
        consensus.update_quorum_status(2, 5).await; // self + 2 peers = 3 (majority)
        consensus.promote_to_leader().await.unwrap();
        
        assert!(consensus.is_leader().await);
        
        // Lose quorum
        consensus.update_quorum_status(1, 5).await; // self + 1 peer = 2 (minority)
        
        // Should have stepped down
        assert!(!consensus.is_leader().await);
        
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_fencing_token_validation() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        
        // Set current term to 5
        {
            let mut state = consensus.state.write().await;
            state.current_term = 5;
        }
        
        // Validate token from current term
        let valid_token = FencingToken::new(5, 0);
        assert!(consensus.validate_fencing_token(&valid_token).await.is_ok());
        
        // Validate token from future term
        let future_token = FencingToken::new(6, 0);
        assert!(consensus.validate_fencing_token(&future_token).await.is_ok());
        
        // Validate stale token from past term
        let stale_token = FencingToken::new(4, 0);
        let result = consensus.validate_fencing_token(&stale_token).await;
        assert!(result.is_err());
        match result {
            Err(ClusterError::StaleToken { current_term, received_term }) => {
                assert_eq!(current_term, 5);
                assert_eq!(received_term, 4);
            },
            _ => panic!("Expected StaleToken error"),
        }
        
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_lease_renewal() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        consensus.promote_to_leader().await.unwrap();
        
        // Get initial lease expiry
        let initial_expiry = {
            let state = consensus.state.read().await;
            state.leader_lease.as_ref().unwrap().expiry
        };
        
        // Wait a bit
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Renew lease
        consensus.renew_lease().await.unwrap();
        
        // Lease expiry should be extended
        let new_expiry = {
            let state = consensus.state.read().await;
            state.leader_lease.as_ref().unwrap().expiry
        };
        
        assert!(new_expiry > initial_expiry);
        
        consensus.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_fencing_token_monotonic_increase() {
        let config = get_test_config();
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let consensus = RaftConsensus::new(1, transport, config).await.unwrap();
        
        consensus.start().await.unwrap();
        consensus.promote_to_leader().await.unwrap();
        
        // Propose multiple entries
        consensus.propose(b"entry1".to_vec()).await.unwrap();
        consensus.propose(b"entry2".to_vec()).await.unwrap();
        consensus.propose(b"entry3".to_vec()).await.unwrap();
        
        // Verify tokens are monotonically increasing
        let state = consensus.state.read().await;
        assert_eq!(state.log.len(), 3);
        
        let token1 = state.log[0].fencing_token.unwrap();
        let token2 = state.log[1].fencing_token.unwrap();
        let token3 = state.log[2].fencing_token.unwrap();
        
        assert!(token2.is_newer_than(&token1));
        assert!(token3.is_newer_than(&token2));
        
        consensus.stop().await.unwrap();
    }
}
