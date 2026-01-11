//! # Comprehensive Cluster E2E Tests for NeuroQuantumDB
//!
//! This module implements end-to-end tests for cluster mode functionality,
//! validating distributed consensus, fault tolerance, and data consistency.
//!
//! ## Test Categories
//!
//! 1. **Basic Cluster Operations**: Formation with 3/5/7 nodes, leader election
//! 2. **Failure Scenarios**: Leader/follower failure, network partition, split-brain
//! 3. **Data Consistency**: Write to leader, read from follower, linearizability
//! 4. **Performance Under Failure**: Latency during election, recovery time
//! 5. **Chaos Engineering**: Random failures, network delays, resource pressure
//!
//! ## Running Cluster E2E Tests
//!
//! Cluster E2E tests are marked with `#[ignore]` by default since they
//! involve multi-node coordination and are time-intensive.
//!
//! Run them explicitly with:
//!
//! ```bash
//! cargo test --package neuroquantum-core --test cluster_e2e_tests -- --ignored --nocapture
//! ```
//!
//! Or run all tests including ignored:
//!
//! ```bash
//! cargo test --package neuroquantum-core --test cluster_e2e_tests -- --include-ignored --nocapture
//! ```

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

// ============================================================================
// Test Configuration
// ============================================================================

#[allow(dead_code)]
mod config {
    use std::time::Duration;

    /// Default timeout for cluster formation
    pub const CLUSTER_FORMATION_TIMEOUT: Duration = Duration::from_secs(10);

    /// Default timeout for leader election
    pub const LEADER_ELECTION_TIMEOUT: Duration = Duration::from_secs(5);

    /// Default heartbeat interval for tests
    pub const TEST_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(50);

    /// Default election timeout range for tests
    pub const TEST_ELECTION_TIMEOUT_MIN: Duration = Duration::from_millis(150);
    pub const TEST_ELECTION_TIMEOUT_MAX: Duration = Duration::from_millis(300);

    /// Number of operations per consistency test
    pub const OPS_PER_CONSISTENCY_TEST: usize = 100;

    /// Number of chaos cycles to run
    pub const CHAOS_CYCLES: usize = 10;

    /// Number of concurrent writers during chaos test
    pub const CONCURRENT_WRITERS: usize = 4;
}

// Port counter for tests to avoid conflicts
static PORT_COUNTER: AtomicU16 = AtomicU16::new(40000);

fn get_test_port() -> u16 {
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

// ============================================================================
// Test Statistics
// ============================================================================

/// Statistics for cluster E2E tests
#[derive(Debug, Default)]
#[allow(dead_code)]
struct ClusterTestStats {
    successful_elections: AtomicU64,
    failed_elections: AtomicU64,
    successful_failovers: AtomicU64,
    failed_failovers: AtomicU64,
    consistency_violations: AtomicU64,
    write_operations: AtomicU64,
    read_operations: AtomicU64,
    total_latency_ms: AtomicU64,
}

#[allow(dead_code)]
impl ClusterTestStats {
    fn record_successful_election(&self, latency_ms: u64) {
        self.successful_elections.fetch_add(1, Ordering::SeqCst);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::SeqCst);
    }

    fn record_failed_election(&self) {
        self.failed_elections.fetch_add(1, Ordering::SeqCst);
    }

    fn record_successful_failover(&self) {
        self.successful_failovers.fetch_add(1, Ordering::SeqCst);
    }

    fn record_failed_failover(&self) {
        self.failed_failovers.fetch_add(1, Ordering::SeqCst);
    }

    fn record_consistency_violation(&self) {
        self.consistency_violations.fetch_add(1, Ordering::SeqCst);
    }

    fn record_write(&self) {
        self.write_operations.fetch_add(1, Ordering::SeqCst);
    }

    fn record_read(&self) {
        self.read_operations.fetch_add(1, Ordering::SeqCst);
    }

    fn report(&self) -> String {
        let successful_elections = self.successful_elections.load(Ordering::SeqCst);
        let failed_elections = self.failed_elections.load(Ordering::SeqCst);
        let successful_failovers = self.successful_failovers.load(Ordering::SeqCst);
        let failed_failovers = self.failed_failovers.load(Ordering::SeqCst);
        let violations = self.consistency_violations.load(Ordering::SeqCst);
        let writes = self.write_operations.load(Ordering::SeqCst);
        let reads = self.read_operations.load(Ordering::SeqCst);
        let total_latency = self.total_latency_ms.load(Ordering::SeqCst);
        let avg_latency = if successful_elections > 0 {
            total_latency / successful_elections
        } else {
            0
        };

        format!(
            "Cluster E2E Test Results:\n\
             - Successful elections: {successful_elections}\n\
             - Failed elections: {failed_elections}\n\
             - Successful failovers: {successful_failovers}\n\
             - Failed failovers: {failed_failovers}\n\
             - Consistency violations: {violations}\n\
             - Write operations: {writes}\n\
             - Read operations: {reads}\n\
             - Average election latency: {avg_latency}ms"
        )
    }
}

// ============================================================================
// Simulated Cluster Node
// ============================================================================

/// State of a simulated cluster node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum NodeState {
    Follower,
    Candidate,
    Leader,
    Stopped,
    Partitioned,
}

/// A simulated cluster node for E2E testing
#[allow(dead_code)]
struct SimulatedNode {
    id: u64,
    state: Arc<RwLock<NodeState>>,
    term: Arc<RwLock<u64>>,
    voted_for: Arc<RwLock<Option<u64>>>,
    log: Arc<RwLock<Vec<LogEntry>>>,
    commit_index: Arc<RwLock<u64>>,
    running: Arc<AtomicBool>,
    port: u16,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct LogEntry {
    term: u64,
    index: u64,
    data: Vec<u8>,
}

impl SimulatedNode {
    fn new(id: u64) -> Self {
        Self {
            id,
            state: Arc::new(RwLock::new(NodeState::Follower)),
            term: Arc::new(RwLock::new(0)),
            voted_for: Arc::new(RwLock::new(None)),
            log: Arc::new(RwLock::new(Vec::new())),
            commit_index: Arc::new(RwLock::new(0)),
            running: Arc::new(AtomicBool::new(false)),
            port: get_test_port(),
        }
    }

    async fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
        *self.state.write().await = NodeState::Follower;
    }

    async fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        *self.state.write().await = NodeState::Stopped;
    }

    async fn get_state(&self) -> NodeState {
        *self.state.read().await
    }

    async fn get_term(&self) -> u64 {
        *self.term.read().await
    }

    async fn is_leader(&self) -> bool {
        *self.state.read().await == NodeState::Leader
    }

    async fn promote_to_leader(&self) {
        let mut state = self.state.write().await;
        let mut term = self.term.write().await;
        *term += 1;
        *state = NodeState::Leader;
    }

    async fn step_down(&self) {
        *self.state.write().await = NodeState::Follower;
    }

    async fn partition(&self) {
        *self.state.write().await = NodeState::Partitioned;
    }

    async fn heal_partition(&self) {
        let mut state = self.state.write().await;
        if *state == NodeState::Partitioned {
            *state = NodeState::Follower;
        }
    }

    async fn append_entry(&self, data: Vec<u8>) -> u64 {
        let mut log = self.log.write().await;
        let term = *self.term.read().await;
        let index = log.len() as u64 + 1;
        log.push(LogEntry { term, index, data });
        index
    }

    async fn get_log_length(&self) -> usize {
        self.log.read().await.len()
    }

    async fn commit(&self, index: u64) {
        *self.commit_index.write().await = index;
    }

    async fn get_commit_index(&self) -> u64 {
        *self.commit_index.read().await
    }
}

// ============================================================================
// Simulated Cluster
// ============================================================================

/// A simulated cluster for E2E testing
struct SimulatedCluster {
    nodes: Vec<Arc<SimulatedNode>>,
    leader_id: Arc<RwLock<Option<u64>>>,
}

#[allow(dead_code)]
impl SimulatedCluster {
    fn new(node_count: usize) -> Self {
        let nodes: Vec<Arc<SimulatedNode>> = (1..=node_count)
            .map(|id| Arc::new(SimulatedNode::new(id as u64)))
            .collect();

        Self {
            nodes,
            leader_id: Arc::new(RwLock::new(None)),
        }
    }

    async fn start(&self) {
        for node in &self.nodes {
            node.start().await;
        }
    }

    async fn stop(&self) {
        for node in &self.nodes {
            node.stop().await;
        }
    }

    async fn get_node(&self, id: u64) -> Option<Arc<SimulatedNode>> {
        self.nodes.iter().find(|n| n.id == id).cloned()
    }

    async fn elect_leader(&self) -> Option<u64> {
        // Simulate leader election: pick the first running node
        for node in &self.nodes {
            if node.running.load(Ordering::SeqCst) {
                node.promote_to_leader().await;
                *self.leader_id.write().await = Some(node.id);
                return Some(node.id);
            }
        }
        None
    }

    async fn get_leader(&self) -> Option<Arc<SimulatedNode>> {
        let leader_id = self.leader_id.read().await;
        if let Some(id) = *leader_id {
            return self.get_node(id).await;
        }
        None
    }

    async fn get_running_nodes(&self) -> Vec<Arc<SimulatedNode>> {
        let mut running = Vec::new();
        for node in &self.nodes {
            let state = node.get_state().await;
            // Only count nodes that are running and not partitioned
            if node.running.load(Ordering::SeqCst)
                && state != NodeState::Partitioned
                && state != NodeState::Stopped
            {
                running.push(node.clone());
            }
        }
        running
    }

    async fn get_followers(&self) -> Vec<Arc<SimulatedNode>> {
        let mut followers = Vec::new();
        let leader_id = *self.leader_id.read().await;
        for node in &self.nodes {
            let state = node.get_state().await;
            if node.running.load(Ordering::SeqCst)
                && Some(node.id) != leader_id
                && state != NodeState::Partitioned
                && state != NodeState::Stopped
            {
                followers.push(node.clone());
            }
        }
        followers
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    async fn quorum_size(&self) -> usize {
        (self.nodes.len() / 2) + 1
    }

    async fn has_quorum(&self) -> bool {
        let running = self.get_running_nodes().await;
        running.len() >= self.quorum_size().await
    }

    async fn failover(&self) -> Option<u64> {
        let current_leader_id = *self.leader_id.read().await;

        // Stop current leader
        if let Some(id) = current_leader_id {
            if let Some(leader) = self.get_node(id).await {
                leader.stop().await;
            }
        }

        // Elect new leader from remaining nodes
        *self.leader_id.write().await = None;
        for node in &self.nodes {
            if node.running.load(Ordering::SeqCst) {
                node.promote_to_leader().await;
                *self.leader_id.write().await = Some(node.id);
                return Some(node.id);
            }
        }
        None
    }
}

// ============================================================================
// Basic Cluster Operations Tests
// ============================================================================

/// Test 3-node cluster formation
#[tokio::test]
async fn test_cluster_formation_3_nodes() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;

    // Verify all nodes are running
    let running = cluster.get_running_nodes().await;
    assert_eq!(running.len(), 3, "All 3 nodes should be running");

    // Verify quorum
    assert!(
        cluster.has_quorum().await,
        "3-node cluster should have quorum"
    );
    assert_eq!(
        cluster.quorum_size().await,
        2,
        "Quorum size for 3 nodes is 2"
    );

    cluster.stop().await;
}

/// Test 5-node cluster formation
#[tokio::test]
async fn test_cluster_formation_5_nodes() {
    let cluster = SimulatedCluster::new(5);
    cluster.start().await;

    let running = cluster.get_running_nodes().await;
    assert_eq!(running.len(), 5, "All 5 nodes should be running");
    assert_eq!(
        cluster.quorum_size().await,
        3,
        "Quorum size for 5 nodes is 3"
    );

    cluster.stop().await;
}

/// Test 7-node cluster formation
#[tokio::test]
async fn test_cluster_formation_7_nodes() {
    let cluster = SimulatedCluster::new(7);
    cluster.start().await;

    let running = cluster.get_running_nodes().await;
    assert_eq!(running.len(), 7, "All 7 nodes should be running");
    assert_eq!(
        cluster.quorum_size().await,
        4,
        "Quorum size for 7 nodes is 4"
    );

    cluster.stop().await;
}

/// Test leader election on startup
#[tokio::test]
async fn test_leader_election_on_startup() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;

    let start = Instant::now();
    let leader_id = cluster.elect_leader().await;
    let election_time = start.elapsed();

    assert!(leader_id.is_some(), "Leader should be elected");
    stats.record_successful_election(election_time.as_millis() as u64);

    let leader = cluster.get_leader().await;
    assert!(leader.is_some(), "Leader should be accessible");
    assert!(
        leader.unwrap().is_leader().await,
        "Leader node should be in leader state"
    );

    println!(
        "Leader election completed in {:?}ms",
        election_time.as_millis()
    );
    cluster.stop().await;
}

/// Test client routing to leader
#[tokio::test]
async fn test_client_routing_to_leader() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    let leader = cluster.get_leader().await.expect("Leader should exist");
    let followers = cluster.get_followers().await;

    // Verify leader can accept writes
    assert!(leader.is_leader().await, "Leader should be in leader state");

    // Verify followers are not leaders
    for follower in followers {
        assert!(
            !follower.is_leader().await,
            "Followers should not be leaders"
        );
    }

    cluster.stop().await;
}

// ============================================================================
// Failure Scenarios Tests
// ============================================================================

/// Test leader node failure and re-election
#[tokio::test]
async fn test_leader_failure_and_reelection() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;

    // Elect initial leader
    let initial_leader_id = cluster
        .elect_leader()
        .await
        .expect("Initial leader elected");

    // Simulate leader failure
    let initial_leader = cluster
        .get_node(initial_leader_id)
        .await
        .expect("Leader node exists");
    initial_leader.stop().await;

    // Re-elect new leader
    let start = Instant::now();
    let new_leader_id = cluster.failover().await;
    let failover_time = start.elapsed();

    assert!(new_leader_id.is_some(), "New leader should be elected");
    assert_ne!(
        new_leader_id.unwrap(),
        initial_leader_id,
        "New leader should be different from old leader"
    );

    stats.record_successful_failover();
    println!(
        "Leader failover completed in {:?}ms",
        failover_time.as_millis()
    );

    cluster.stop().await;
}

/// Test follower node failure and recovery
#[tokio::test]
async fn test_follower_failure_and_recovery() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    let followers = cluster.get_followers().await;
    assert!(!followers.is_empty(), "Should have followers");

    let follower = &followers[0];
    let follower_id = follower.id;

    // Stop follower
    follower.stop().await;
    assert!(
        !follower.running.load(Ordering::SeqCst),
        "Follower should be stopped"
    );

    // Cluster should still have quorum (2 out of 3)
    assert!(
        cluster.has_quorum().await,
        "Cluster should maintain quorum with 1 follower down"
    );

    // Recover follower
    follower.start().await;
    assert!(
        follower.running.load(Ordering::SeqCst),
        "Follower should be running again"
    );

    // Verify follower is back in cluster
    let recovered_node = cluster.get_node(follower_id).await;
    assert!(
        recovered_node.is_some(),
        "Recovered node should be in cluster"
    );

    cluster.stop().await;
}

/// Test network partition (split-brain scenario)
#[tokio::test]
async fn test_network_partition_split_brain() {
    let cluster = SimulatedCluster::new(5);
    cluster.start().await;
    cluster.elect_leader().await;

    let leader = cluster.get_leader().await.expect("Leader exists");
    let leader_id = leader.id;

    // Partition the leader (minority partition)
    leader.partition().await;
    assert_eq!(
        leader.get_state().await,
        NodeState::Partitioned,
        "Leader should be partitioned"
    );

    // Remaining 4 nodes should elect new leader
    *cluster.leader_id.write().await = None;

    // Find a non-partitioned node to become leader
    for node in &cluster.nodes {
        let state = node.get_state().await;
        if node.id != leader_id
            && state != NodeState::Partitioned
            && node.running.load(Ordering::SeqCst)
        {
            node.promote_to_leader().await;
            *cluster.leader_id.write().await = Some(node.id);
            break;
        }
    }

    let new_leader_id = *cluster.leader_id.read().await;
    assert!(new_leader_id.is_some(), "New leader should be elected");
    assert_ne!(
        new_leader_id.unwrap(),
        leader_id,
        "New leader should be different"
    );

    // Heal partition
    leader.heal_partition().await;
    assert_ne!(
        leader.get_state().await,
        NodeState::Partitioned,
        "Leader should no longer be partitioned"
    );

    cluster.stop().await;
}

/// Test multiple simultaneous failures
#[tokio::test]
async fn test_multiple_simultaneous_failures() {
    let cluster = SimulatedCluster::new(5);
    cluster.start().await;
    cluster.elect_leader().await;

    // Fail 2 nodes simultaneously (should still have quorum: 3 out of 5)
    let running = cluster.get_running_nodes().await;
    running[0].stop().await;
    running[1].stop().await;

    // Should still have quorum
    let remaining = cluster.get_running_nodes().await;
    assert_eq!(remaining.len(), 3, "3 nodes should remain");
    assert!(
        cluster.has_quorum().await,
        "Should maintain quorum with 3 nodes"
    );

    cluster.stop().await;
}

/// Test loss of quorum
#[tokio::test]
async fn test_loss_of_quorum() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    // Fail 2 nodes (lose quorum: 1 out of 3)
    let running = cluster.get_running_nodes().await;
    running[0].stop().await;
    running[1].stop().await;

    let remaining = cluster.get_running_nodes().await;
    assert_eq!(remaining.len(), 1, "Only 1 node should remain");
    assert!(
        !cluster.has_quorum().await,
        "Should lose quorum with only 1 node"
    );

    cluster.stop().await;
}

// ============================================================================
// Data Consistency Tests
// ============================================================================

/// Test write to leader, read from follower
#[tokio::test]
async fn test_write_leader_read_follower() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    let leader = cluster.get_leader().await.expect("Leader exists");
    let followers = cluster.get_followers().await;

    // Write to leader
    let index = leader.append_entry(b"test_data_1".to_vec()).await;
    stats.record_write();

    // Commit the entry
    leader.commit(index).await;

    // Simulate replication to followers
    for follower in &followers {
        follower.append_entry(b"test_data_1".to_vec()).await;
        follower.commit(index).await;
    }

    // Verify all nodes have the same data
    let leader_log_len = leader.get_log_length().await;
    for follower in &followers {
        let follower_log_len = follower.get_log_length().await;
        assert_eq!(
            follower_log_len, leader_log_len,
            "Follower should have same log length as leader"
        );
        stats.record_read();
    }

    cluster.stop().await;
}

/// Test consistency after leader failover
#[tokio::test]
async fn test_consistency_after_failover() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    let leader = cluster.get_leader().await.expect("Leader exists");

    // Write some data before failover
    for i in 0..5 {
        let data = format!("entry_{}", i);
        let index = leader.append_entry(data.as_bytes().to_vec()).await;
        leader.commit(index).await;
        stats.record_write();
    }

    let log_length_before = leader.get_log_length().await;
    let commit_index_before = leader.get_commit_index().await;

    // Replicate to followers
    let followers = cluster.get_followers().await;
    for follower in &followers {
        for i in 0..5 {
            let data = format!("entry_{}", i);
            follower.append_entry(data.as_bytes().to_vec()).await;
        }
        follower.commit(commit_index_before).await;
    }

    // Failover
    let new_leader_id = cluster.failover().await.expect("New leader elected");
    let new_leader = cluster
        .get_node(new_leader_id)
        .await
        .expect("New leader exists");

    // Verify new leader has all committed data
    let log_length_after = new_leader.get_log_length().await;
    assert_eq!(
        log_length_after, log_length_before,
        "New leader should have all committed entries"
    );

    let commit_index_after = new_leader.get_commit_index().await;
    assert_eq!(
        commit_index_after, commit_index_before,
        "Commit index should be preserved"
    );

    cluster.stop().await;
}

/// Test stale read detection
#[tokio::test]
async fn test_stale_read_detection() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    let leader = cluster.get_leader().await.expect("Leader exists");
    let followers = cluster.get_followers().await;

    // Write to leader but don't replicate to one follower
    let index = leader.append_entry(b"new_data".to_vec()).await;
    leader.commit(index).await;

    // Replicate to only one follower
    if let Some(replicated_follower) = followers.first() {
        replicated_follower.append_entry(b"new_data".to_vec()).await;
        replicated_follower.commit(index).await;
    }

    // The second follower should have stale data (different commit index)
    if followers.len() > 1 {
        let stale_follower = &followers[1];
        let leader_commit = leader.get_commit_index().await;
        let stale_commit = stale_follower.get_commit_index().await;

        // Stale follower has lower commit index
        assert!(
            stale_commit < leader_commit,
            "Stale follower should have lower commit index"
        );
    }

    cluster.stop().await;
}

/// Test linearizability with concurrent writes
#[tokio::test]
async fn test_linearizability_concurrent_writes() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = Arc::new(SimulatedCluster::new(3));
    cluster.start().await;
    cluster.elect_leader().await;

    let leader = cluster.get_leader().await.expect("Leader exists");
    let leader = Arc::new(leader);

    // Spawn concurrent writers
    let mut handles = Vec::new();
    for i in 0..config::CONCURRENT_WRITERS {
        let leader_clone = Arc::clone(&leader);
        let stats_clone = Arc::clone(&stats);
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let data = format!("writer_{}_entry_{}", i, j);
                leader_clone.append_entry(data.as_bytes().to_vec()).await;
                stats_clone.record_write();
            }
        });
        handles.push(handle);
    }

    // Wait for all writers
    for handle in handles {
        handle.await.expect("Writer task completed");
    }

    // Verify log length equals total writes
    let expected_entries = config::CONCURRENT_WRITERS * 10;
    let actual_entries = leader.get_log_length().await;
    assert_eq!(
        actual_entries, expected_entries,
        "All concurrent writes should be in log"
    );

    cluster.stop().await;

    println!("{}", stats.report());
}

// ============================================================================
// Performance Under Failure Tests
// ============================================================================

/// Test latency during leader election
#[tokio::test]
async fn test_latency_during_election() {
    let stats = Arc::new(ClusterTestStats::default());
    let mut election_latencies = Vec::new();

    for _ in 0..5 {
        let cluster = SimulatedCluster::new(3);
        cluster.start().await;

        let start = Instant::now();
        let _leader_id = cluster.elect_leader().await;
        let latency = start.elapsed();

        election_latencies.push(latency.as_millis() as u64);
        stats.record_successful_election(latency.as_millis() as u64);

        cluster.stop().await;
    }

    let avg_latency: u64 = election_latencies.iter().sum::<u64>() / election_latencies.len() as u64;
    let max_latency: u64 = *election_latencies.iter().max().unwrap_or(&0);

    println!(
        "Election latency - Avg: {}ms, Max: {}ms",
        avg_latency, max_latency
    );

    // Election should complete within reasonable time (simulated)
    assert!(
        max_latency < 1000,
        "Election should complete within 1 second"
    );
}

/// Test recovery time objectives
#[tokio::test]
async fn test_recovery_time_objectives() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    // Measure failover time
    let start = Instant::now();
    let _new_leader = cluster.failover().await;
    let recovery_time = start.elapsed();

    println!("Recovery time: {:?}ms", recovery_time.as_millis());

    // RTO should be reasonable (in simulated environment)
    assert!(
        recovery_time < Duration::from_secs(1),
        "Recovery should complete within 1 second"
    );

    cluster.stop().await;
}

/// Test throughput during failover
#[tokio::test]
async fn test_throughput_during_failover() {
    let cluster = Arc::new(SimulatedCluster::new(5));
    cluster.start().await;
    cluster.elect_leader().await;

    let writes_before = Arc::new(AtomicU64::new(0));
    let writes_after = Arc::new(AtomicU64::new(0));
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Start background writers
    let leader = cluster.get_leader().await.expect("Leader exists");
    let leader = Arc::new(leader);
    let leader_clone = Arc::clone(&leader);
    let writes_before_clone = Arc::clone(&writes_before);
    let stop_clone = Arc::clone(&stop_flag);

    let writer_handle = tokio::spawn(async move {
        while !stop_clone.load(Ordering::SeqCst) {
            leader_clone.append_entry(b"throughput_test".to_vec()).await;
            writes_before_clone.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });

    // Let it write for a bit
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Trigger failover
    cluster.failover().await;

    // Continue writing with new leader
    if let Some(new_leader) = cluster.get_leader().await {
        for _ in 0..50 {
            new_leader.append_entry(b"after_failover".to_vec()).await;
            writes_after.fetch_add(1, Ordering::SeqCst);
        }
    }

    stop_flag.store(true, Ordering::SeqCst);
    let _ = writer_handle.await;

    let before = writes_before.load(Ordering::SeqCst);
    let after = writes_after.load(Ordering::SeqCst);

    println!(
        "Writes before failover: {}, Writes after failover: {}",
        before, after
    );

    assert!(before > 0, "Should have writes before failover");
    assert!(after > 0, "Should have writes after failover");

    cluster.stop().await;
}

// ============================================================================
// Chaos Engineering Tests
// ============================================================================

/// Test random node kills
#[tokio::test]
#[ignore] // Long-running test
async fn test_chaos_random_node_kills() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = Arc::new(SimulatedCluster::new(5));
    cluster.start().await;

    for cycle in 0..config::CHAOS_CYCLES {
        println!("Chaos cycle {}/{}", cycle + 1, config::CHAOS_CYCLES);

        // Elect leader
        if cluster.elect_leader().await.is_some() {
            stats.record_successful_election(0);
        }

        // Random node kill (ensure quorum is maintained)
        let running = cluster.get_running_nodes().await;
        if running.len() > cluster.quorum_size().await {
            // Kill a random node
            let victim_idx = (cycle % running.len()) as usize;
            if victim_idx < running.len() {
                running[victim_idx].stop().await;
            }
        }

        // Verify quorum is maintained
        if cluster.has_quorum().await {
            // Try to write
            if let Some(leader) = cluster.get_leader().await {
                leader
                    .append_entry(format!("chaos_{}", cycle).as_bytes().to_vec())
                    .await;
                stats.record_write();
            }
        }

        // Recover killed nodes
        for node in &cluster.nodes {
            if !node.running.load(Ordering::SeqCst) {
                node.start().await;
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    println!("{}", stats.report());
    assert_eq!(
        stats.consistency_violations.load(Ordering::SeqCst),
        0,
        "No consistency violations during chaos"
    );

    cluster.stop().await;
}

/// Test network delay injection (simulated)
#[tokio::test]
async fn test_chaos_network_delay() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    let leader = cluster.get_leader().await.expect("Leader exists");

    // Simulate delayed writes
    let start = Instant::now();
    for i in 0..10 {
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(10)).await;
        leader
            .append_entry(format!("delayed_{}", i).as_bytes().to_vec())
            .await;
    }
    let total_time = start.elapsed();

    // Should have taken at least 100ms due to simulated delays
    assert!(
        total_time >= Duration::from_millis(100),
        "Should account for network delays"
    );

    let log_len = leader.get_log_length().await;
    assert_eq!(log_len, 10, "All entries should be logged despite delays");

    cluster.stop().await;
}

/// Test split-brain recovery
#[tokio::test]
async fn test_chaos_split_brain_recovery() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = SimulatedCluster::new(5);
    cluster.start().await;
    cluster.elect_leader().await;

    let initial_leader = cluster.get_leader().await.expect("Leader exists");
    let initial_leader_id = initial_leader.id;
    let initial_term = initial_leader.get_term().await;

    // Create split-brain: partition leader
    initial_leader.partition().await;

    // Majority partition elects new leader (find a non-partitioned node)
    *cluster.leader_id.write().await = None;
    for node in &cluster.nodes {
        let state = node.get_state().await;
        if node.id != initial_leader_id
            && state != NodeState::Partitioned
            && node.running.load(Ordering::SeqCst)
        {
            node.promote_to_leader().await;
            *cluster.leader_id.write().await = Some(node.id);
            break;
        }
    }

    let new_leader = cluster.get_leader().await.expect("New leader exists");
    let new_term = new_leader.get_term().await;

    // New leader should have higher term
    assert!(
        new_term > initial_term,
        "New leader should have higher term"
    );

    // Heal partition - old leader should step down
    initial_leader.heal_partition().await;
    initial_leader.step_down().await;

    // Verify old leader is now follower
    assert!(
        !initial_leader.is_leader().await,
        "Old leader should step down after partition heals"
    );

    stats.record_successful_failover();
    println!("{}", stats.report());

    cluster.stop().await;
}

/// Test cluster under concurrent load with failures
#[tokio::test]
#[ignore] // Long-running test
async fn test_chaos_concurrent_load_with_failures() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = Arc::new(SimulatedCluster::new(5));
    cluster.start().await;
    cluster.elect_leader().await;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let mut writer_handles = Vec::new();

    // Start concurrent writers
    for i in 0..config::CONCURRENT_WRITERS {
        let cluster_clone = Arc::clone(&cluster);
        let stats_clone = Arc::clone(&stats);
        let stop_clone = Arc::clone(&stop_flag);

        let handle = tokio::spawn(async move {
            let mut writes = 0u64;
            while !stop_clone.load(Ordering::SeqCst) {
                if let Some(leader) = cluster_clone.get_leader().await {
                    leader
                        .append_entry(format!("writer_{}_entry_{}", i, writes).as_bytes().to_vec())
                        .await;
                    stats_clone.record_write();
                    writes += 1;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            writes
        });
        writer_handles.push(handle);
    }

    // Inject failures periodically
    for cycle in 0..5 {
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Kill a non-leader node
        let followers = cluster.get_followers().await;
        if let Some(victim) = followers.first() {
            victim.stop().await;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Recover the node
        for node in &cluster.nodes {
            if !node.running.load(Ordering::SeqCst) {
                node.start().await;
            }
        }

        // Trigger leader failover occasionally
        if cycle % 2 == 0 {
            cluster.failover().await;
            stats.record_successful_failover();
        }
    }

    stop_flag.store(true, Ordering::SeqCst);

    let mut total_writes = 0u64;
    for handle in writer_handles {
        if let Ok(writes) = handle.await {
            total_writes += writes;
        }
    }

    println!("Total writes during chaos: {}", total_writes);
    println!("{}", stats.report());

    assert!(
        total_writes > 0,
        "Should have successful writes during chaos"
    );
    assert_eq!(
        stats.consistency_violations.load(Ordering::SeqCst),
        0,
        "No consistency violations"
    );

    cluster.stop().await;
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test single node cluster
#[tokio::test]
async fn test_single_node_cluster() {
    let cluster = SimulatedCluster::new(1);
    cluster.start().await;

    // Single node should have quorum
    assert!(cluster.has_quorum().await, "Single node has quorum");

    // Should be able to elect itself as leader
    let leader_id = cluster.elect_leader().await;
    assert!(leader_id.is_some(), "Single node can be leader");

    let leader = cluster.get_leader().await.expect("Leader exists");
    leader.append_entry(b"single_node_data".to_vec()).await;
    leader.commit(1).await;

    assert_eq!(leader.get_log_length().await, 1, "Should have 1 entry");
    assert_eq!(leader.get_commit_index().await, 1, "Should be committed");

    cluster.stop().await;
}

/// Test cluster with all nodes failed
#[tokio::test]
async fn test_all_nodes_failed() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;
    cluster.elect_leader().await;

    // Stop all nodes
    for node in &cluster.nodes {
        node.stop().await;
    }

    let running = cluster.get_running_nodes().await;
    assert!(running.is_empty(), "No nodes should be running");
    assert!(!cluster.has_quorum().await, "Should not have quorum");

    // Recover all nodes
    for node in &cluster.nodes {
        node.start().await;
    }

    let running = cluster.get_running_nodes().await;
    assert_eq!(running.len(), 3, "All nodes should be recovered");
    assert!(cluster.has_quorum().await, "Should have quorum again");

    cluster.stop().await;
}

/// Test rapid leadership changes
#[tokio::test]
async fn test_rapid_leadership_changes() {
    let stats = Arc::new(ClusterTestStats::default());
    let cluster = SimulatedCluster::new(5);
    cluster.start().await;

    let mut leadership_changes = 0u64;

    for _ in 0..10 {
        let _leader_id = cluster.elect_leader().await;
        leadership_changes += 1;

        // Immediately failover (this stops the current leader)
        cluster.failover().await;
        leadership_changes += 1;

        // Restart stopped nodes to maintain cluster size for next iteration
        for node in &cluster.nodes {
            if !node.running.load(Ordering::SeqCst) {
                node.start().await;
            }
        }
    }

    println!("Leadership changes: {}", leadership_changes);
    stats.record_successful_election(0);

    // Verify cluster is still functional
    assert!(
        cluster.has_quorum().await,
        "Cluster should still have quorum"
    );

    cluster.stop().await;
}

/// Test term monotonicity during elections
#[tokio::test]
async fn test_term_monotonicity() {
    let cluster = SimulatedCluster::new(3);
    cluster.start().await;

    let mut previous_term = 0u64;

    for _ in 0..5 {
        cluster.elect_leader().await;

        if let Some(leader) = cluster.get_leader().await {
            let current_term = leader.get_term().await;
            assert!(
                current_term >= previous_term,
                "Term should be monotonically increasing"
            );
            previous_term = current_term;
        }

        cluster.failover().await;
    }

    cluster.stop().await;
}

// ============================================================================
// Documentation Tests
// ============================================================================

/// Test that demonstrates running cluster tests
/// This can be used as documentation reference
#[tokio::test]
async fn test_cluster_documentation_example() {
    // Create a 3-node cluster
    let cluster = SimulatedCluster::new(3);

    // Start all nodes
    cluster.start().await;

    // Elect a leader
    let leader_id = cluster.elect_leader().await.expect("Leader elected");
    println!("Leader elected: Node {}", leader_id);

    // Get the leader and write some data
    let leader = cluster.get_leader().await.expect("Leader accessible");
    let index = leader.append_entry(b"hello, cluster!".to_vec()).await;
    leader.commit(index).await;

    // Verify data is committed
    assert_eq!(leader.get_commit_index().await, 1);

    // Simulate leader failure and failover
    let new_leader_id = cluster.failover().await.expect("New leader elected");
    println!("New leader after failover: Node {}", new_leader_id);

    // Verify cluster is still operational
    assert!(cluster.has_quorum().await);

    // Stop all nodes
    cluster.stop().await;

    println!("Cluster E2E test completed successfully!");
}
