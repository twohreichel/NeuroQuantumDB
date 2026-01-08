//! Raft consensus implementation for cluster coordination.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock};
use tracing::{debug, info, warn};

use crate::config::ClusterConfig;
use crate::error::{ClusterError, ClusterResult};
use crate::network::{NetworkTransport, ClusterMessage, AppendEntriesRequest, AppendEntriesResponse, LogEntryCompact};
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
    /// Next index to send to each follower (leader only)
    next_index: std::collections::HashMap<NodeId, u64>,
    /// Highest log entry known to be replicated on each follower (leader only)
    match_index: std::collections::HashMap<NodeId, u64>,
    /// Votes received in current election (candidate only)
    votes_received: std::collections::HashSet<NodeId>,
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
            next_index: std::collections::HashMap::new(),
            match_index: std::collections::HashMap::new(),
            votes_received: std::collections::HashSet::new(),
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

impl Clone for RaftConsensus {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            transport: Arc::clone(&self.transport),
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            running: Arc::clone(&self.running),
            heartbeat_received: Arc::clone(&self.heartbeat_received),
        }
    }
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

        // Register vote handler with transport
        let consensus_clone = self.clone();
        self.transport
            .register_request_vote_handler(move |req| {
                let consensus = consensus_clone.clone();
                Box::pin(async move { consensus.handle_request_vote(req).await })
            })
            .await;

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
        
        // Increment term (as would happen in a real election)
        state.current_term += 1;
        
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
        let consensus = self.clone();

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
                            let peer_ids = {
                                let mut state_guard = state.write().await;
                                state_guard.state = RaftState::Candidate;
                                state_guard.current_term += 1;
                                state_guard.voted_for = Some(node_id);
                                // Vote for self
                                state_guard.votes_received.clear();
                                state_guard.votes_received.insert(node_id);
                                
                                // Get list of peers from next_index keys (if we have any)
                                // or we'll need to get them from cluster config
                                state_guard.next_index.keys().copied().collect::<Vec<_>>()
                            };

                            // Request votes from all peers
                            // Note: This is a simplified version. In a full implementation,
                            // we would get peer list from cluster membership.
                            if !peer_ids.is_empty() {
                                let consensus_clone = consensus.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = consensus_clone.request_votes(peer_ids).await {
                                        warn!(node_id, error = %e, "Failed to request votes");
                                    }
                                });
                            }
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
        let consensus = self.clone();

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
                let (is_leader, peer_ids) = {
                    let state_guard = state.read().await;
                    let is_leader = state_guard.state == RaftState::Leader;
                    let peer_ids: Vec<NodeId> = state_guard.next_index.keys().copied().collect();
                    (is_leader, peer_ids)
                };

                if is_leader && !peer_ids.is_empty() {
                    debug!(node_id, peers = peer_ids.len(), "Sending heartbeat to followers");

                    // Send AppendEntries (heartbeat) to all followers in parallel
                    let mut tasks = Vec::new();
                    for peer_id in peer_ids {
                        let consensus_clone = consensus.clone();
                        let task = tokio::spawn(async move {
                            if let Err(e) = consensus_clone.replicate_to_follower(peer_id).await {
                                warn!(
                                    node_id,
                                    peer = peer_id,
                                    error = %e,
                                    "Failed to send heartbeat to follower"
                                );
                            }
                        });
                        tasks.push(task);
                    }

                    // Wait for all heartbeat tasks to complete
                    for task in tasks {
                        let _ = task.await;
                    }
                }
            }
        });
    }

    /// Notify that a heartbeat was received (to reset election timer).
    pub async fn notify_heartbeat(&self) {
        self.heartbeat_received.notify_one();
    }

    /// Initialize replication state for all peers when becoming leader.
    pub async fn initialize_replication_state(&self, peer_ids: Vec<NodeId>) {
        let mut state = self.state.write().await;
        
        // Set next_index to last log index + 1 for all peers
        let next_idx = state.log.len() as u64 + 1;
        for peer_id in peer_ids {
            state.next_index.insert(peer_id, next_idx);
            state.match_index.insert(peer_id, 0);
        }
        
        info!(
            node_id = self.node_id,
            peers = state.next_index.len(),
            next_index = next_idx,
            "Initialized replication state for peers"
        );
    }

    /// Replicate log entries to a specific follower.
    pub async fn replicate_to_follower(&self, follower_id: NodeId) -> ClusterResult<()> {
        let state = self.state.read().await;
        
        if state.state != RaftState::Leader {
            return Err(ClusterError::NotLeader(self.node_id, state.current_leader));
        }
        
        let next_idx = state.next_index.get(&follower_id).copied().unwrap_or(1);
        let prev_log_index = if next_idx > 1 { next_idx - 1 } else { 0 };
        
        let prev_log_term = if prev_log_index > 0 && prev_log_index <= state.log.len() as u64 {
            state.log[prev_log_index as usize - 1].term
        } else {
            0
        };
        
        // Collect entries to send
        let entries: Vec<LogEntryCompact> = if next_idx <= state.log.len() as u64 {
            state.log[next_idx as usize - 1..]
                .iter()
                .map(|entry| LogEntryCompact {
                    term: entry.term,
                    data: bincode::serialize(&entry.data).unwrap_or_default(),
                })
                .collect()
        } else {
            Vec::new()
        };
        
        let request = AppendEntriesRequest {
            term: state.current_term,
            leader_id: self.node_id,
            prev_log_index,
            prev_log_term,
            entries: entries.clone(),
            leader_commit: state.commit_index,
        };
        
        debug!(
            node_id = self.node_id,
            follower = follower_id,
            prev_log_index,
            prev_log_term,
            entries_count = entries.len(),
            leader_commit = state.commit_index,
            "Sending AppendEntries to follower"
        );
        
        drop(state); // Release read lock before async call
        
        // Send AppendEntries RPC via network
        self.transport
            .send(follower_id, ClusterMessage::AppendEntries(request))
            .await?;
        
        Ok(())
    }

    /// Handle AppendEntries response from a follower (leader side).
    pub async fn handle_append_entries_response(
        &self,
        follower_id: NodeId,
        response: AppendEntriesResponse,
    ) -> ClusterResult<()> {
        let mut state = self.state.write().await;
        
        // If response term is higher, step down
        if response.term > state.current_term {
            warn!(
                node_id = self.node_id,
                current_term = state.current_term,
                response_term = response.term,
                "Received higher term in AppendEntries response, stepping down"
            );
            state.current_term = response.term;
            state.state = RaftState::Follower;
            state.leader_lease = None;
            state.current_leader = None;
            state.voted_for = None;
            return Ok(());
        }
        
        if state.state != RaftState::Leader {
            return Ok(()); // Ignore if no longer leader
        }
        
        if response.success {
            // Update next_index and match_index for follower
            let next_idx = state.next_index.get(&follower_id).copied().unwrap_or(1);
            let new_match_index = if response.match_index.is_some() {
                response.match_index.unwrap()
            } else {
                // Calculate based on what we sent
                let entries_sent = if next_idx <= state.log.len() as u64 {
                    state.log.len() as u64 - next_idx + 1
                } else {
                    0
                };
                next_idx + entries_sent - 1
            };
            
            state.match_index.insert(follower_id, new_match_index);
            state.next_index.insert(follower_id, new_match_index + 1);
            
            debug!(
                node_id = self.node_id,
                follower = follower_id,
                match_index = new_match_index,
                next_index = new_match_index + 1,
                "Updated follower replication state"
            );
            
            // Try to advance commit index
            self.try_advance_commit_index(&mut state).await;
        } else {
            // Replication failed, decrement next_index for this follower
            let next_idx = state.next_index.get(&follower_id).copied().unwrap_or(1);
            
            // Use conflict hints if provided for faster catchup
            let new_next_idx = if let (Some(conflict_index), Some(conflict_term)) = 
                (response.conflict_index, response.conflict_term) {
                // Find the last entry in leader's log with conflict_term
                let mut idx = conflict_index;
                while idx > 0 {
                    if idx as usize <= state.log.len() {
                        if state.log[idx as usize - 1].term == conflict_term {
                            break;
                        }
                    }
                    idx -= 1;
                }
                if idx > 0 {
                    idx + 1
                } else {
                    conflict_index
                }
            } else {
                // Simple backtracking
                if next_idx > 1 {
                    next_idx - 1
                } else {
                    1
                }
            };
            
            state.next_index.insert(follower_id, new_next_idx);
            
            warn!(
                node_id = self.node_id,
                follower = follower_id,
                old_next_index = next_idx,
                new_next_index = new_next_idx,
                "AppendEntries rejected, decrementing next_index"
            );
        }
        
        Ok(())
    }

    /// Try to advance commit index based on match_index from followers.
    async fn try_advance_commit_index(&self, state: &mut ConsensusState) {
        if state.state != RaftState::Leader {
            return;
        }
        
        // Find the highest index replicated to a majority
        let log_len = state.log.len() as u64;
        for n in (state.commit_index + 1..=log_len).rev() {
            // Count replicas (including self)
            let mut count = 1;
            for &match_idx in state.match_index.values() {
                if match_idx >= n {
                    count += 1;
                }
            }
            
            // Check if we have majority
            let needed = (state.cluster_size / 2) + 1;
            if count >= needed {
                // According to Raft §5.4.2, a leader can commit entries from previous terms
                // once an entry from its current term is committed. However, it should never
                // commit entries from previous terms directly.
                // For now, we allow committing if it's from the current term.
                // A more sophisticated implementation would track when a current-term entry is committed.
                if state.log[n as usize - 1].term == state.current_term {
                    let old_commit = state.commit_index;
                    state.commit_index = n;
                    info!(
                        node_id = self.node_id,
                        old_commit_index = old_commit,
                        new_commit_index = n,
                        replicas = count,
                        "Advanced commit index"
                    );
                    break;
                }
            }
        }
    }

    /// Handle incoming AppendEntries RPC (follower side).
    pub async fn handle_append_entries(
        &self,
        request: AppendEntriesRequest,
    ) -> ClusterResult<AppendEntriesResponse> {
        let mut state = self.state.write().await;
        
        // Reply false if term < currentTerm
        if request.term < state.current_term {
            return Ok(AppendEntriesResponse {
                term: state.current_term,
                success: false,
                match_index: None,
                conflict_index: None,
                conflict_term: None,
            });
        }
        
        // Update term if higher
        if request.term > state.current_term {
            state.current_term = request.term;
            state.state = RaftState::Follower;
            state.voted_for = None;
            state.leader_lease = None;
        }
        
        // Update current leader
        state.current_leader = Some(request.leader_id);
        
        // Reset election timer (heartbeat received)
        drop(state);
        self.notify_heartbeat().await;
        let mut state = self.state.write().await;
        
        // Check log consistency
        if request.prev_log_index > 0 {
            // If we don't have an entry at prev_log_index, reply false
            if request.prev_log_index > state.log.len() as u64 {
                return Ok(AppendEntriesResponse {
                    term: state.current_term,
                    success: false,
                    match_index: None,
                    conflict_index: Some(state.log.len() as u64 + 1),
                    conflict_term: None,
                });
            }
            
            // If entry at prev_log_index has different term, reply false
            let prev_entry = &state.log[request.prev_log_index as usize - 1];
            if prev_entry.term != request.prev_log_term {
                // Find conflict term and first index of that term
                let conflict_term = prev_entry.term;
                let mut conflict_index = request.prev_log_index;
                
                // Find first index of conflict term
                while conflict_index > 1 {
                    if state.log[conflict_index as usize - 2].term != conflict_term {
                        break;
                    }
                    conflict_index -= 1;
                }
                
                return Ok(AppendEntriesResponse {
                    term: state.current_term,
                    success: false,
                    match_index: None,
                    conflict_index: Some(conflict_index),
                    conflict_term: Some(conflict_term),
                });
            }
        }
        
        // Append new entries
        if !request.entries.is_empty() {
            let mut next_index = request.prev_log_index + 1;
            
            for entry_compact in &request.entries {
                let entry_data: LogEntryData = bincode::deserialize(&entry_compact.data)
                    .unwrap_or(LogEntryData::Noop);
                
                let entry = LogEntry {
                    term: entry_compact.term,
                    index: next_index,
                    data: entry_data,
                    fencing_token: None, // Follower doesn't generate tokens
                };
                
                // If an existing entry conflicts (same index, different term), delete it and all following
                if next_index as usize <= state.log.len() {
                    if state.log[next_index as usize - 1].term != entry.term {
                        state.log.truncate(next_index as usize - 1);
                        state.log.push(entry);
                    }
                    // Otherwise entry matches, skip
                } else {
                    state.log.push(entry);
                }
                
                next_index += 1;
            }
            
            debug!(
                node_id = self.node_id,
                leader = request.leader_id,
                entries_appended = request.entries.len(),
                log_length = state.log.len(),
                "Appended entries to log"
            );
        }
        
        // Update commit index
        if request.leader_commit > state.commit_index {
            let last_new_index = if request.entries.is_empty() {
                request.prev_log_index
            } else {
                request.prev_log_index + request.entries.len() as u64
            };
            
            let old_commit = state.commit_index;
            state.commit_index = std::cmp::min(request.leader_commit, last_new_index);
            
            if state.commit_index > old_commit {
                info!(
                    node_id = self.node_id,
                    old_commit = old_commit,
                    new_commit = state.commit_index,
                    "Updated commit index from leader"
                );
            }
        }
        
        let last_log_index = state.log.len() as u64;
        
        Ok(AppendEntriesResponse {
            term: state.current_term,
            success: true,
            match_index: Some(last_log_index),
            conflict_index: None,
            conflict_term: None,
        })
    }

    /// Apply committed entries to state machine.
    pub async fn apply_committed_entries(&self) -> ClusterResult<usize> {
        let mut state = self.state.write().await;
        
        let mut applied_count = 0;
        
        while state.last_applied < state.commit_index {
            state.last_applied += 1;
            
            if state.last_applied as usize <= state.log.len() {
                let entry = &state.log[state.last_applied as usize - 1];
                
                debug!(
                    node_id = self.node_id,
                    index = state.last_applied,
                    term = entry.term,
                    "Applying committed entry to state machine"
                );
                
                // In a full implementation, we would apply the entry to the state machine here
                // For now, we just track that it was applied
                applied_count += 1;
            }
        }
        
        if applied_count > 0 {
            info!(
                node_id = self.node_id,
                applied = applied_count,
                last_applied = state.last_applied,
                "Applied committed entries to state machine"
            );
        }
        
        Ok(applied_count)
    }

    /// Get the last log index.
    pub async fn last_log_index(&self) -> u64 {
        let state = self.state.read().await;
        state.log.len() as u64
    }

    /// Get the last log term.
    pub async fn last_log_term(&self) -> u64 {
        let state = self.state.read().await;
        if state.log.is_empty() {
            0
        } else {
            state.log[state.log.len() - 1].term
        }
    }

    /// Request votes from all peers (called when becoming a candidate).
    pub async fn request_votes(&self, peer_ids: Vec<NodeId>) -> ClusterResult<()> {
        let (term, last_log_index, last_log_term) = {
            let state = self.state.read().await;
            (
                state.current_term,
                if state.log.is_empty() {
                    0
                } else {
                    state.log.len() as u64
                },
                if state.log.is_empty() {
                    0
                } else {
                    state.log[state.log.len() - 1].term
                },
            )
        };

        debug!(
            node_id = self.node_id,
            term,
            last_log_index,
            last_log_term,
            peers = peer_ids.len(),
            "Requesting votes from peers"
        );

        let request = crate::network::RequestVoteRequest {
            term,
            candidate_id: self.node_id,
            last_log_index,
            last_log_term,
            is_pre_vote: false,
        };

        // Send vote requests to all peers in parallel and handle responses
        let mut tasks = Vec::new();
        for peer_id in peer_ids {
            let transport = Arc::clone(&self.transport);
            let consensus = self.clone();
            let request = request.clone();
            
            let task = tokio::spawn(async move {
                match transport.send_request_vote_rpc(peer_id, request).await {
                    Ok(response) => {
                        // Handle the vote response
                        if let Err(e) = consensus.handle_request_vote_response(peer_id, response).await {
                            warn!(
                                node_id = consensus.node_id,
                                peer = peer_id,
                                error = %e,
                                "Failed to handle vote response"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            node_id = consensus.node_id,
                            peer = peer_id,
                            error = %e,
                            "Failed to send vote request"
                        );
                    }
                }
            });
            tasks.push(task);
        }

        // Wait for all requests to complete
        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    /// Handle incoming RequestVote RPC (follower/candidate side).
    pub async fn handle_request_vote(
        &self,
        request: crate::network::RequestVoteRequest,
    ) -> ClusterResult<crate::network::RequestVoteResponse> {
        let mut state = self.state.write().await;

        debug!(
            node_id = self.node_id,
            candidate = request.candidate_id,
            term = request.term,
            current_term = state.current_term,
            "Received vote request"
        );

        // Reply false if term < currentTerm (§5.1)
        if request.term < state.current_term {
            return Ok(crate::network::RequestVoteResponse {
                term: state.current_term,
                vote_granted: false,
            });
        }

        // If RPC request or response contains term T > currentTerm:
        // set currentTerm = T, convert to follower (§5.1)
        if request.term > state.current_term {
            state.current_term = request.term;
            state.state = RaftState::Follower;
            state.voted_for = None;
            state.leader_lease = None;
            state.current_leader = None;
        }

        // Check if we can grant vote
        let can_grant_vote = if let Some(voted_for) = state.voted_for {
            // Already voted in this term - only grant if it's for the same candidate
            voted_for == request.candidate_id
        } else {
            // Haven't voted yet - check if candidate's log is at least as up-to-date
            // as receiver's log (§5.4, §5.2)
            let our_last_log_term = if state.log.is_empty() {
                0
            } else {
                state.log[state.log.len() - 1].term
            };
            let our_last_log_index = state.log.len() as u64;

            // Raft determines which of two logs is more up-to-date by comparing
            // the index and term of the last entries in the logs.
            // If the logs have last entries with different terms, then the log with
            // the later term is more up-to-date.
            // If the logs end with the same term, then whichever log is longer is more up-to-date.
            let candidate_log_up_to_date = request.last_log_term > our_last_log_term
                || (request.last_log_term == our_last_log_term
                    && request.last_log_index >= our_last_log_index);

            candidate_log_up_to_date
        };

        if can_grant_vote {
            state.voted_for = Some(request.candidate_id);
            info!(
                node_id = self.node_id,
                candidate = request.candidate_id,
                term = request.term,
                "Granted vote"
            );
            Ok(crate::network::RequestVoteResponse {
                term: state.current_term,
                vote_granted: true,
            })
        } else {
            debug!(
                node_id = self.node_id,
                candidate = request.candidate_id,
                term = request.term,
                voted_for = ?state.voted_for,
                "Denied vote"
            );
            Ok(crate::network::RequestVoteResponse {
                term: state.current_term,
                vote_granted: false,
            })
        }
    }

    /// Handle RequestVote response (candidate side).
    pub async fn handle_request_vote_response(
        &self,
        from_node: NodeId,
        response: crate::network::RequestVoteResponse,
    ) -> ClusterResult<()> {
        let mut state = self.state.write().await;

        debug!(
            node_id = self.node_id,
            from = from_node,
            term = response.term,
            vote_granted = response.vote_granted,
            current_state = ?state.state,
            "Received vote response"
        );

        // If response term is higher, step down
        if response.term > state.current_term {
            warn!(
                node_id = self.node_id,
                current_term = state.current_term,
                response_term = response.term,
                "Received higher term in vote response, stepping down"
            );
            state.current_term = response.term;
            state.state = RaftState::Follower;
            state.voted_for = None;
            state.leader_lease = None;
            state.current_leader = None;
            state.votes_received.clear();
            return Ok(());
        }

        // Ignore if we're no longer a candidate
        if state.state != RaftState::Candidate {
            return Ok(());
        }

        // Ignore stale responses
        if response.term < state.current_term {
            return Ok(());
        }

        // Record vote if granted
        if response.vote_granted {
            state.votes_received.insert(from_node);
            
            info!(
                node_id = self.node_id,
                from = from_node,
                votes = state.votes_received.len(),
                cluster_size = state.cluster_size,
                "Received vote"
            );

            // Check if we have majority
            // Need (cluster_size / 2) + 1 votes
            let votes_needed = (state.cluster_size / 2) + 1;
            let votes_count = state.votes_received.len() + 1; // +1 for self

            if votes_count >= votes_needed {
                info!(
                    node_id = self.node_id,
                    term = state.current_term,
                    votes = votes_count,
                    needed = votes_needed,
                    "Won election, becoming leader"
                );

                // Transition to leader
                state.state = RaftState::Leader;
                state.current_leader = Some(self.node_id);
                
                // Create leader lease
                let lease_duration = self.config.raft.heartbeat_interval * 3;
                state.leader_lease = Some(LeaderLease::new(lease_duration));
                
                // Clear votes
                state.votes_received.clear();

                // Initialize next_index and match_index for all peers
                // (Will be properly initialized when replication state is set up)
                info!(
                    node_id = self.node_id,
                    term = state.current_term,
                    "Successfully transitioned to leader"
                );
            }
        }

        Ok(())
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

// Include replication integration tests
#[cfg(test)]
#[path = "consensus_tests.rs"]
mod consensus_tests;

// Include chaos tests for failure scenarios
#[cfg(test)]
#[path = "consensus_chaos_tests.rs"]
mod consensus_chaos_tests;
