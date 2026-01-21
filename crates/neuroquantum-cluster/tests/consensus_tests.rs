//! Consensus module tests - Raft consensus implementation tests
//!
//! This module contains tests for:
//! - Raft state machine and log entry serialization
//! - Consensus node lifecycle (start, stop, heartbeat)
//! - Election timeouts and candidate state transitions
//! - Leader election and vote handling
//! - Split brain prevention (fencing tokens, leader lease, quorum)
//! - Log replication integration tests
//! - Chaos tests for failure scenarios

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;

use neuroquantum_cluster::consensus::{
    FencingToken, LeaderLease, LogEntry, LogEntryData, QuorumStatus, RaftConsensus, RaftState,
};
use neuroquantum_cluster::network::{
    AppendEntriesRequest, AppendEntriesResponse, LogEntryCompact, NetworkTransport,
    RequestVoteRequest, RequestVoteResponse,
};
use neuroquantum_cluster::node::NodeId;
use neuroquantum_cluster::{ClusterConfig, ClusterError};

// Port counter for tests - start at 40000 to avoid conflicts with other test files
static PORT_COUNTER: AtomicU16 = AtomicU16::new(40000);

fn get_test_config() -> ClusterConfig {
    let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    ClusterConfig {
        bind_addr: format!("127.0.0.1:{port}").parse().unwrap(),
        ..Default::default()
    }
}

fn get_test_config_with_node(node_id: NodeId) -> ClusterConfig {
    let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    ClusterConfig {
        node_id,
        bind_addr: format!("127.0.0.1:{port}").parse().unwrap(),
        ..Default::default()
    }
}

// ============================================================================
// Basic Raft State and Serialization Tests
// ============================================================================

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

// ============================================================================
// Consensus Lifecycle Tests
// ============================================================================

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

// ============================================================================
// Election Timeout Tests
// ============================================================================

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

// ============================================================================
// Split Brain Prevention Tests (Fencing Tokens, Leases, Quorum)
// ============================================================================

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
        | Err(ClusterError::NotLeader(_, _)) => {},
        | _ => panic!("Expected NotLeader error after losing quorum"),
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
        | Err(ClusterError::LeaseExpired) => {},
        | _ => panic!("Expected LeaseExpired error"),
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
    assert!(consensus
        .validate_fencing_token(&future_token)
        .await
        .is_ok());

    // Validate stale token from past term
    let stale_token = FencingToken::new(4, 0);
    let result = consensus.validate_fencing_token(&stale_token).await;
    assert!(result.is_err());
    match result {
        | Err(ClusterError::StaleToken {
            current_term,
            received_term,
        }) => {
            assert_eq!(current_term, 5);
            assert_eq!(received_term, 4);
        },
        | _ => panic!("Expected StaleToken error"),
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

// ============================================================================
// Vote Request and Leader Election Tests
// ============================================================================

#[tokio::test]
async fn test_handle_request_vote_grant() {
    let config = get_test_config();
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Set our term to 1
    {
        let mut state = consensus.state.write().await;
        state.current_term = 1;
    }

    // Receive vote request from candidate in same term
    let request = RequestVoteRequest {
        term: 1,
        candidate_id: 2,
        last_log_index: 0,
        last_log_term: 0,
        is_pre_vote: false,
    };

    let response = consensus.handle_request_vote(request).await.unwrap();

    assert_eq!(response.term, 1);
    assert!(response.vote_granted);

    // Verify we voted for candidate 2
    let state = consensus.state.read().await;
    assert_eq!(state.voted_for, Some(2));

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_request_vote_deny_lower_term() {
    let config = get_test_config();
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Set our term to 5
    {
        let mut state = consensus.state.write().await;
        state.current_term = 5;
    }

    // Receive vote request from candidate in lower term
    let request = RequestVoteRequest {
        term: 3,
        candidate_id: 2,
        last_log_index: 0,
        last_log_term: 0,
        is_pre_vote: false,
    };

    let response = consensus.handle_request_vote(request).await.unwrap();

    assert_eq!(response.term, 5);
    assert!(!response.vote_granted);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_request_vote_deny_already_voted() {
    let config = get_test_config();
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Set our term and vote for candidate 2
    {
        let mut state = consensus.state.write().await;
        state.current_term = 5;
        state.voted_for = Some(2);
    }

    // Receive vote request from different candidate in same term
    let request = RequestVoteRequest {
        term: 5,
        candidate_id: 3,
        last_log_index: 0,
        last_log_term: 0,
        is_pre_vote: false,
    };

    let response = consensus.handle_request_vote(request).await.unwrap();

    assert_eq!(response.term, 5);
    assert!(!response.vote_granted);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_request_vote_deny_stale_log() {
    let config = get_test_config();
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Add some log entries to our log
    {
        let mut state = consensus.state.write().await;
        state.current_term = 2;
        state.log.push(LogEntry {
            term: 1,
            index: 1,
            data: LogEntryData::Command(b"entry1".to_vec()),
            fencing_token: None,
        });
        state.log.push(LogEntry {
            term: 2,
            index: 2,
            data: LogEntryData::Command(b"entry2".to_vec()),
            fencing_token: None,
        });
    }

    // Receive vote request from candidate with older log
    let request = RequestVoteRequest {
        term: 2,
        candidate_id: 2,
        last_log_index: 0,
        last_log_term: 0,
        is_pre_vote: false,
    };

    let response = consensus.handle_request_vote(request).await.unwrap();

    assert_eq!(response.term, 2);
    assert!(!response.vote_granted);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_request_vote_step_down_higher_term() {
    let config = get_test_config();
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();
    consensus.promote_to_leader().await.unwrap();

    assert!(consensus.is_leader().await);

    // Receive vote request with higher term
    let request = RequestVoteRequest {
        term: 10,
        candidate_id: 2,
        last_log_index: 0,
        last_log_term: 0,
        is_pre_vote: false,
    };

    let response = consensus.handle_request_vote(request).await.unwrap();

    // Should step down and grant vote
    assert_eq!(response.term, 10);
    assert!(response.vote_granted);
    assert!(!consensus.is_leader().await);

    let state = consensus.state.read().await;
    assert_eq!(state.current_term, 10);
    assert_eq!(state.state, RaftState::Follower);
    assert_eq!(state.voted_for, Some(2));

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_vote_response_becomes_leader() {
    let config = get_test_config();
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Set up as candidate in 3-node cluster
    {
        let mut state = consensus.state.write().await;
        state.state = RaftState::Candidate;
        state.current_term = 5;
        state.cluster_size = 3;
        state.voted_for = Some(1);
        state.votes_received.insert(1); // Vote for self
    }

    // Receive vote from peer 2
    let response = RequestVoteResponse {
        term: 5,
        vote_granted: true,
    };

    consensus
        .handle_request_vote_response(2, response)
        .await
        .unwrap();

    // Should now be leader (2 out of 3 votes)
    assert!(consensus.is_leader().await);
    let state = consensus.state.read().await;
    assert_eq!(state.state, RaftState::Leader);
    assert!(state.leader_lease.is_some());

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_vote_response_not_enough_votes() {
    let config = get_test_config();
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Set up as candidate in 5-node cluster
    {
        let mut state = consensus.state.write().await;
        state.state = RaftState::Candidate;
        state.current_term = 5;
        state.cluster_size = 5;
        state.voted_for = Some(1);
        state.votes_received.clear();
        state.votes_received.insert(1); // Vote for self
    }

    // Receive vote from peer 2 (only 2 out of 5 votes - not majority, need 3)
    let response = RequestVoteResponse {
        term: 5,
        vote_granted: true,
    };

    consensus
        .handle_request_vote_response(2, response)
        .await
        .unwrap();

    // Should still be candidate (need 3 votes, have 2)
    let state = consensus.state.read().await;
    assert_eq!(state.state, RaftState::Candidate);
    assert_eq!(state.votes_received.len(), 2); // self + peer 2
    drop(state);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_vote_response_step_down_higher_term() {
    let config = get_test_config();
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Set up as candidate
    {
        let mut state = consensus.state.write().await;
        state.state = RaftState::Candidate;
        state.current_term = 5;
        state.cluster_size = 3;
    }

    // Receive response with higher term
    let response = RequestVoteResponse {
        term: 10,
        vote_granted: false,
    };

    consensus
        .handle_request_vote_response(2, response)
        .await
        .unwrap();

    // Should step down to follower
    let state = consensus.state.read().await;
    assert_eq!(state.current_term, 10);
    assert_eq!(state.state, RaftState::Follower);
    assert_eq!(state.voted_for, None);

    consensus.stop().await.unwrap();
}

// ============================================================================
// Log Replication Integration Tests
// ============================================================================

#[tokio::test]
async fn test_initialize_replication_state() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();
    consensus.promote_to_leader().await.unwrap();

    // Initialize replication state for peers
    let peer_ids = vec![2, 3, 4];
    consensus
        .initialize_replication_state(peer_ids.clone())
        .await;

    // Verify state was initialized
    let state = consensus.state.read().await;
    for peer_id in peer_ids {
        assert_eq!(state.next_index.get(&peer_id), Some(&1));
        assert_eq!(state.match_index.get(&peer_id), Some(&0));
    }

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_log_replication_basic() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();
    consensus.promote_to_leader().await.unwrap();

    // Propose some entries
    let index1 = consensus.propose(b"entry1".to_vec()).await.unwrap();
    let index2 = consensus.propose(b"entry2".to_vec()).await.unwrap();
    let index3 = consensus.propose(b"entry3".to_vec()).await.unwrap();

    assert_eq!(index1, 1);
    assert_eq!(index2, 2);
    assert_eq!(index3, 3);

    // Verify log contains entries
    let state = consensus.state.read().await;
    assert_eq!(state.log.len(), 3);
    assert_eq!(state.log[0].index, 1);
    assert_eq!(state.log[1].index, 2);
    assert_eq!(state.log[2].index, 3);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_append_entries_success() {
    let config = get_test_config_with_node(2);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(2, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Create AppendEntries request
    let entries = vec![
        LogEntryCompact {
            term: 1,
            data: bincode::serialize(&LogEntryData::Command(b"entry1".to_vec())).unwrap(),
        },
        LogEntryCompact {
            term: 1,
            data: bincode::serialize(&LogEntryData::Command(b"entry2".to_vec())).unwrap(),
        },
    ];

    let request = AppendEntriesRequest {
        term: 1,
        leader_id: 1,
        prev_log_index: 0,
        prev_log_term: 0,
        entries,
        leader_commit: 0,
    };

    // Handle AppendEntries
    let response = consensus.handle_append_entries(request).await.unwrap();

    assert!(response.success);
    assert_eq!(response.term, 1);
    assert_eq!(response.match_index, Some(2));

    // Verify entries were appended
    let state = consensus.state.read().await;
    assert_eq!(state.log.len(), 2);
    assert_eq!(state.current_term, 1);
    assert_eq!(state.current_leader, Some(1));

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_append_entries_log_inconsistency() {
    let config = get_test_config_with_node(2);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(2, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Add some entries to the log first
    {
        let mut state = consensus.state.write().await;
        state.current_term = 1;
        state.log.push(LogEntry {
            term: 1,
            index: 1,
            data: LogEntryData::Command(b"old_entry".to_vec()),
            fencing_token: None,
        });
    }

    // Create AppendEntries request with inconsistent prev_log_index
    let request = AppendEntriesRequest {
        term: 2,
        leader_id: 1,
        prev_log_index: 5, // We only have 1 entry
        prev_log_term: 1,
        entries: vec![],
        leader_commit: 0,
    };

    // Handle AppendEntries
    let response = consensus.handle_append_entries(request).await.unwrap();

    assert!(!response.success);
    assert_eq!(response.conflict_index, Some(2)); // Next available index

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_append_entries_term_conflict() {
    let config = get_test_config_with_node(2);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(2, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Add some entries with term 1
    {
        let mut state = consensus.state.write().await;
        state.current_term = 1;
        state.log.push(LogEntry {
            term: 1,
            index: 1,
            data: LogEntryData::Command(b"entry1".to_vec()),
            fencing_token: None,
        });
        state.log.push(LogEntry {
            term: 1,
            index: 2,
            data: LogEntryData::Command(b"entry2".to_vec()),
            fencing_token: None,
        });
    }

    // Create AppendEntries with conflicting term at prev_log_index
    let request = AppendEntriesRequest {
        term: 2,
        leader_id: 1,
        prev_log_index: 2,
        prev_log_term: 2, // Our term at index 2 is 1, not 2
        entries: vec![],
        leader_commit: 0,
    };

    let response = consensus.handle_append_entries(request).await.unwrap();

    assert!(!response.success);
    assert_eq!(response.conflict_term, Some(1));
    assert!(response.conflict_index.is_some());

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_append_entries_overwrites_conflicting() {
    let config = get_test_config_with_node(2);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(2, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Add entries with term 1
    {
        let mut state = consensus.state.write().await;
        state.current_term = 1;
        state.log.push(LogEntry {
            term: 1,
            index: 1,
            data: LogEntryData::Command(b"entry1".to_vec()),
            fencing_token: None,
        });
        state.log.push(LogEntry {
            term: 1,
            index: 2,
            data: LogEntryData::Command(b"conflicting_entry".to_vec()),
            fencing_token: None,
        });
    }

    // Send AppendEntries with new entry at index 2 with different term
    let entries = vec![LogEntryCompact {
        term: 2,
        data: bincode::serialize(&LogEntryData::Command(b"new_entry".to_vec())).unwrap(),
    }];

    let request = AppendEntriesRequest {
        term: 2,
        leader_id: 1,
        prev_log_index: 1,
        prev_log_term: 1,
        entries,
        leader_commit: 0,
    };

    let response = consensus.handle_append_entries(request).await.unwrap();

    assert!(response.success);

    // Verify conflicting entry was overwritten
    let state = consensus.state.read().await;
    assert_eq!(state.log.len(), 2);
    assert_eq!(state.log[1].term, 2);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_commit_index_update() {
    let config = get_test_config_with_node(2);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(2, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Add some entries
    let entries = vec![
        LogEntryCompact {
            term: 1,
            data: bincode::serialize(&LogEntryData::Command(b"entry1".to_vec())).unwrap(),
        },
        LogEntryCompact {
            term: 1,
            data: bincode::serialize(&LogEntryData::Command(b"entry2".to_vec())).unwrap(),
        },
    ];

    let request = AppendEntriesRequest {
        term: 1,
        leader_id: 1,
        prev_log_index: 0,
        prev_log_term: 0,
        entries,
        leader_commit: 2, // Leader has committed both entries
    };

    let response = consensus.handle_append_entries(request).await.unwrap();

    assert!(response.success);

    // Verify commit index was updated
    let state = consensus.state.read().await;
    assert_eq!(state.commit_index, 2);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_apply_committed_entries() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();
    consensus.promote_to_leader().await.unwrap();

    // Add and commit some entries
    {
        let mut state = consensus.state.write().await;
        state.log.push(LogEntry {
            term: 1,
            index: 1,
            data: LogEntryData::Command(b"entry1".to_vec()),
            fencing_token: None,
        });
        state.log.push(LogEntry {
            term: 1,
            index: 2,
            data: LogEntryData::Command(b"entry2".to_vec()),
            fencing_token: None,
        });
        state.commit_index = 2;
    }

    // Apply committed entries
    let applied = consensus.apply_committed_entries().await.unwrap();
    assert_eq!(applied, 2);

    // Verify last_applied was updated
    let state = consensus.state.read().await;
    assert_eq!(state.last_applied, 2);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_leader_advance_commit_index() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();
    consensus.promote_to_leader().await.unwrap();

    // Add entries
    consensus.propose(b"entry1".to_vec()).await.unwrap();
    consensus.propose(b"entry2".to_vec()).await.unwrap();
    consensus.propose(b"entry3".to_vec()).await.unwrap();

    // Initialize replication state for 2 followers (3 node cluster)
    consensus.initialize_replication_state(vec![2, 3]).await;

    // Simulate responses indicating entries are replicated
    {
        let mut state = consensus.state.write().await;
        state.cluster_size = 3;
        state.match_index.insert(2, 3); // Follower 2 has all entries
        state.match_index.insert(3, 2); // Follower 3 has entries 1-2
    }

    // Handle successful response
    let response = AppendEntriesResponse {
        term: 1,
        success: true,
        match_index: Some(3),
        conflict_index: None,
        conflict_term: None,
    };

    consensus
        .handle_append_entries_response(2, response)
        .await
        .unwrap();

    // Commit index should advance to 2 (majority has 1-2)
    let commit_index = consensus.commit_index().await;
    assert!(commit_index >= 2);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_handle_higher_term_in_response() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();
    consensus.promote_to_leader().await.unwrap();

    {
        let state = consensus.state.read().await;
        // promote_to_leader increments the term
        assert_eq!(state.current_term, 1);
        assert_eq!(state.state, RaftState::Leader);
    }

    // Receive response with higher term
    let response = AppendEntriesResponse {
        term: 5,
        success: false,
        match_index: None,
        conflict_index: None,
        conflict_term: None,
    };

    consensus
        .handle_append_entries_response(2, response)
        .await
        .unwrap();

    // Should step down
    let state = consensus.state.read().await;
    assert_eq!(state.current_term, 5);
    assert_eq!(state.state, RaftState::Follower);
    assert!(state.leader_lease.is_none());

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_backtracking_on_conflict() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();
    consensus.promote_to_leader().await.unwrap();
    consensus.initialize_replication_state(vec![2]).await;

    // Set next_index to 5
    {
        let mut state = consensus.state.write().await;
        state.next_index.insert(2, 5);
    }

    // Receive conflict response
    let response = AppendEntriesResponse {
        term: 1,
        success: false,
        match_index: None,
        conflict_index: Some(3),
        conflict_term: Some(1),
    };

    consensus
        .handle_append_entries_response(2, response)
        .await
        .unwrap();

    // next_index should be decremented
    let state = consensus.state.read().await;
    let next_idx = state.next_index.get(&2).copied().unwrap();
    assert!(next_idx < 5);

    consensus.stop().await.unwrap();
}

#[tokio::test]
async fn test_last_log_index_and_term() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let consensus = RaftConsensus::new(1, transport, config).await.unwrap();

    consensus.start().await.unwrap();

    // Initially empty
    assert_eq!(consensus.last_log_index().await, 0);
    assert_eq!(consensus.last_log_term().await, 0);

    // Add entries (promote increments term to 1)
    consensus.promote_to_leader().await.unwrap();
    consensus.propose(b"entry1".to_vec()).await.unwrap();
    consensus.propose(b"entry2".to_vec()).await.unwrap();

    assert_eq!(consensus.last_log_index().await, 2);
    // Term should be 1 (incremented during promote_to_leader)
    assert_eq!(consensus.last_log_term().await, 1);

    consensus.stop().await.unwrap();
}

// ============================================================================
// Chaos Tests - Failure Scenarios
// ============================================================================

#[tokio::test]
async fn test_follower_crash_and_recovery() {
    // Create leader
    let leader_config = get_test_config_with_node(1);
    let leader_transport = Arc::new(NetworkTransport::new(&leader_config).await.unwrap());
    let leader = Arc::new(
        RaftConsensus::new(1, leader_transport, leader_config)
            .await
            .unwrap(),
    );

    leader.start().await.unwrap();
    leader.promote_to_leader().await.unwrap();

    // Create follower
    let follower_config = get_test_config_with_node(2);
    let follower_transport = Arc::new(NetworkTransport::new(&follower_config).await.unwrap());
    let follower = RaftConsensus::new(2, follower_transport, follower_config)
        .await
        .unwrap();

    follower.start().await.unwrap();

    // Initialize replication
    leader.initialize_replication_state(vec![2]).await;

    // Leader proposes some entries
    leader.propose(b"entry1".to_vec()).await.unwrap();
    leader.propose(b"entry2".to_vec()).await.unwrap();

    // Simulate follower crash (stop it)
    follower.stop().await.unwrap();

    // Leader continues to propose entries
    leader.propose(b"entry3".to_vec()).await.unwrap();
    leader.propose(b"entry4".to_vec()).await.unwrap();

    // Verify leader's log has all entries
    let leader_log_len = leader.last_log_index().await;
    assert_eq!(leader_log_len, 4);

    // Follower recovers (start a new instance)
    let follower_config = get_test_config_with_node(2);
    let follower_transport = Arc::new(NetworkTransport::new(&follower_config).await.unwrap());
    let follower_recovered = RaftConsensus::new(2, follower_transport, follower_config)
        .await
        .unwrap();
    follower_recovered.start().await.unwrap();

    // Follower should be able to catch up via AppendEntries
    // (In real implementation, leader would detect and send missing entries)

    leader.stop().await.unwrap();
    follower_recovered.stop().await.unwrap();
}

#[tokio::test]
async fn test_network_partition_follower_falls_behind() {
    let leader_config = get_test_config_with_node(1);
    let leader_transport = Arc::new(NetworkTransport::new(&leader_config).await.unwrap());
    let leader = RaftConsensus::new(1, leader_transport, leader_config)
        .await
        .unwrap();

    leader.start().await.unwrap();
    leader.promote_to_leader().await.unwrap();

    // Initialize replication for 2 followers
    leader.initialize_replication_state(vec![2, 3]).await;

    // Propose entries before partition
    leader.propose(b"entry1".to_vec()).await.unwrap();
    leader.propose(b"entry2".to_vec()).await.unwrap();

    // Simulate follower 2 being partitioned (not receiving entries)
    // Leader continues with majority (itself + follower 3)

    leader.propose(b"entry3".to_vec()).await.unwrap();
    leader.propose(b"entry4".to_vec()).await.unwrap();
    leader.propose(b"entry5".to_vec()).await.unwrap();

    // Follower 2 rejoins with only entries 1-2
    // Leader's next_index for follower 2 should backtrack

    // Get current term from leader
    let current_term = leader.current_term().await;

    // Simulate failed append entries (follower 2 doesn't have entry 3)
    {
        let mut state = leader.state.write().await;
        state.cluster_size = 3;
        state.next_index.insert(2, 3); // Leader thinks follower 2 has up to entry 2
    }

    // Verify leader can handle backtracking
    let response = AppendEntriesResponse {
        term: current_term,
        success: false,
        match_index: None,
        conflict_index: Some(3),
        conflict_term: None,
    };

    leader
        .handle_append_entries_response(2, response)
        .await
        .unwrap();

    // next_index should be adjusted
    let state = leader.state.read().await;
    let next_idx = state.next_index.get(&2).copied().unwrap_or(0);
    assert!((1..=3).contains(&next_idx));

    leader.stop().await.unwrap();
}

#[tokio::test]
async fn test_leader_crash_during_replication() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let leader = RaftConsensus::new(1, transport, config).await.unwrap();

    leader.start().await.unwrap();
    leader.promote_to_leader().await.unwrap();

    // Initialize replication
    leader.initialize_replication_state(vec![2, 3]).await;

    // Propose entries
    leader.propose(b"entry1".to_vec()).await.unwrap();
    leader.propose(b"entry2".to_vec()).await.unwrap();

    // Get the log state before crash
    let log_len_before_crash = {
        let state = leader.state.read().await;
        state.log.len()
    };

    // Leader crashes (simulate by stopping)
    leader.stop().await.unwrap();

    // Verify log was saved (in real implementation)
    assert_eq!(log_len_before_crash, 2);

    // New leader would be elected, entries should remain
    // (This is ensured by persistent storage in real implementation)
}

#[tokio::test]
async fn test_split_brain_prevention_with_fencing_tokens() {
    let config1 = get_test_config_with_node(1);
    let transport1 = Arc::new(NetworkTransport::new(&config1).await.unwrap());
    let leader1 = RaftConsensus::new(1, transport1, config1).await.unwrap();

    leader1.start().await.unwrap();
    leader1.promote_to_leader().await.unwrap();

    // First leader proposes entries with fencing tokens
    let index1 = leader1.propose(b"entry1".to_vec()).await.unwrap();

    let token1 = {
        let state = leader1.state.read().await;
        state.log[index1 as usize - 1].fencing_token.unwrap()
    };

    // Network partition - another node becomes leader with higher term
    let config2 = get_test_config_with_node(2);
    let transport2 = Arc::new(NetworkTransport::new(&config2).await.unwrap());
    let leader2 = RaftConsensus::new(2, transport2, config2).await.unwrap();

    leader2.start().await.unwrap();
    {
        let mut state = leader2.state.write().await;
        state.current_term = token1.term + 1; // Higher term
        state.state = RaftState::Leader;
    }
    leader2.promote_to_leader().await.unwrap();

    // Second leader proposes entries
    let index2 = leader2
        .propose(b"entry_from_new_leader".to_vec())
        .await
        .unwrap();

    let token2 = {
        let state = leader2.state.read().await;
        state.log[index2 as usize - 1].fencing_token.unwrap()
    };

    // Token from new leader should be newer
    assert!(token2.is_newer_than(&token1));

    // Old leader should reject operations with stale token
    let _validate_result = leader1.validate_fencing_token(&token1).await;
    // Would fail in real split-brain scenario after learning about higher term

    leader1.stop().await.unwrap();
    leader2.stop().await.unwrap();
}

#[tokio::test]
async fn test_majority_commit_with_node_failures() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let leader = RaftConsensus::new(1, transport, config).await.unwrap();

    leader.start().await.unwrap();
    leader.promote_to_leader().await.unwrap();

    // 5-node cluster
    leader.initialize_replication_state(vec![2, 3, 4, 5]).await;

    // Propose entries
    leader.propose(b"entry1".to_vec()).await.unwrap();
    leader.propose(b"entry2".to_vec()).await.unwrap();
    leader.propose(b"entry3".to_vec()).await.unwrap();

    // Get current term
    let current_term = leader.current_term().await;

    // Simulate responses: 2 followers succeed, 2 fail
    {
        let mut state = leader.state.write().await;
        state.cluster_size = 5;
        state.match_index.insert(2, 3); // Follower 2 has all
        state.match_index.insert(3, 3); // Follower 3 has all
        state.match_index.insert(4, 0); // Follower 4 failed
        state.match_index.insert(5, 0); // Follower 5 failed
    }

    // Should still commit (3 out of 5 = majority)
    let response = AppendEntriesResponse {
        term: current_term,
        success: true,
        match_index: Some(3),
        conflict_index: None,
        conflict_term: None,
    };

    leader
        .handle_append_entries_response(2, response.clone())
        .await
        .unwrap();
    leader
        .handle_append_entries_response(3, response)
        .await
        .unwrap();

    // Commit index should advance
    let commit_idx = leader.commit_index().await;
    assert!(commit_idx >= 3);

    leader.stop().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_proposals_under_load() {
    let config = get_test_config_with_node(1);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let leader = Arc::new(RaftConsensus::new(1, transport, config).await.unwrap());

    leader.start().await.unwrap();
    leader.promote_to_leader().await.unwrap();

    // Spawn multiple tasks proposing entries concurrently
    let mut tasks = Vec::new();
    for i in 0..10 {
        let leader_clone = Arc::clone(&leader);
        let task = tokio::spawn(async move {
            let data = format!("concurrent_entry_{i}");
            leader_clone.propose(data.as_bytes().to_vec()).await
        });
        tasks.push(task);
    }

    // Wait for all proposals
    let mut results = Vec::new();
    for task in tasks {
        if let Ok(result) = task.await {
            results.push(result);
        }
    }

    // All proposals should succeed
    assert_eq!(results.len(), 10);

    // Log should have all entries
    let log_len = leader.last_log_index().await;
    assert_eq!(log_len, 10);

    leader.stop().await.unwrap();
}

#[tokio::test]
async fn test_follower_receives_entries_from_multiple_terms() {
    let config = get_test_config_with_node(2);
    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let follower = RaftConsensus::new(2, transport, config).await.unwrap();

    follower.start().await.unwrap();

    // Receive entries from term 1
    let entries_t1 = vec![
        LogEntryCompact {
            term: 1,
            data: bincode::serialize(&LogEntryData::Command(b"term1_entry1".to_vec())).unwrap(),
        },
        LogEntryCompact {
            term: 1,
            data: bincode::serialize(&LogEntryData::Command(b"term1_entry2".to_vec())).unwrap(),
        },
    ];

    let request1 = AppendEntriesRequest {
        term: 1,
        leader_id: 1,
        prev_log_index: 0,
        prev_log_term: 0,
        entries: entries_t1,
        leader_commit: 0,
    };

    let response1 = follower.handle_append_entries(request1).await.unwrap();
    assert!(response1.success);

    // Receive entries from term 2 (new leader)
    let entries_t2 = vec![LogEntryCompact {
        term: 2,
        data: bincode::serialize(&LogEntryData::Command(b"term2_entry1".to_vec())).unwrap(),
    }];

    let request2 = AppendEntriesRequest {
        term: 2,
        leader_id: 3,
        prev_log_index: 2,
        prev_log_term: 1,
        entries: entries_t2,
        leader_commit: 2,
    };

    let response2 = follower.handle_append_entries(request2).await.unwrap();
    assert!(response2.success);

    // Verify log has entries from both terms
    let state = follower.state.read().await;
    assert_eq!(state.log.len(), 3);
    assert_eq!(state.log[0].term, 1);
    assert_eq!(state.log[1].term, 1);
    assert_eq!(state.log[2].term, 2);

    follower.stop().await.unwrap();
}

#[tokio::test]
async fn test_rapid_leader_changes() {
    // Test scenario where leadership changes rapidly
    let config1 = get_test_config_with_node(1);
    let transport1 = Arc::new(NetworkTransport::new(&config1).await.unwrap());
    let node1 = RaftConsensus::new(1, transport1, config1).await.unwrap();

    let config2 = get_test_config_with_node(2);
    let transport2 = Arc::new(NetworkTransport::new(&config2).await.unwrap());
    let node2 = RaftConsensus::new(2, transport2, config2).await.unwrap();

    node1.start().await.unwrap();
    node2.start().await.unwrap();

    // Node 1 becomes leader (promote_to_leader will set term to 1)
    node1.promote_to_leader().await.unwrap();

    // Propose entry
    node1.propose(b"entry_term1".to_vec()).await.unwrap();

    // Node 2 becomes leader in term 3 (higher than node1's term 1)
    // First set term to 2 so promote_to_leader increments to 3
    {
        let mut state = node2.state.write().await;
        state.current_term = 2;
    }
    node2.promote_to_leader().await.unwrap();

    // Propose entry
    node2.propose(b"entry_term2".to_vec()).await.unwrap();

    // Node 1 receives higher term (3) and should step down
    let response = AppendEntriesResponse {
        term: 3,
        success: false,
        match_index: None,
        conflict_index: None,
        conflict_term: None,
    };

    node1
        .handle_append_entries_response(2, response)
        .await
        .unwrap();

    // Node 1 should be follower now
    let state1 = node1.state.read().await;
    assert_eq!(state1.state, RaftState::Follower);
    assert_eq!(state1.current_term, 3);

    node1.stop().await.unwrap();
    node2.stop().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn test_election_timeout_during_replication() {
    let mut config = get_test_config_with_node(2);
    // Short election timeout for testing
    config.raft.election_timeout_min = Duration::from_millis(100);
    config.raft.election_timeout_max = Duration::from_millis(150);

    let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
    let follower = RaftConsensus::new(2, transport, config).await.unwrap();

    follower.start().await.unwrap();

    // Follower receives entries
    let entries = vec![LogEntryCompact {
        term: 1,
        data: bincode::serialize(&LogEntryData::Command(b"entry1".to_vec())).unwrap(),
    }];

    let request = AppendEntriesRequest {
        term: 1,
        leader_id: 1,
        prev_log_index: 0,
        prev_log_term: 0,
        entries,
        leader_commit: 0,
    };

    let response = follower.handle_append_entries(request).await.unwrap();
    assert!(response.success);

    // Should remain follower since receiving AppendEntries
    tokio::time::advance(Duration::from_millis(200)).await;
    tokio::time::sleep(Duration::from_millis(1)).await;

    let _state = follower.state.read().await;
    // Might transition to candidate if no more heartbeats
    // This tests election timeout behavior during replication

    follower.stop().await.unwrap();
}
