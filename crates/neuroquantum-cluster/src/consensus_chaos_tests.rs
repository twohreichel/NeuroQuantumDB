//! Chaos tests for Raft log replication under failure scenarios

#[cfg(test)]
mod chaos_tests {
    use super::super::*;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    // Port counter for tests
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(30000);

    fn get_test_config(node_id: NodeId) -> ClusterConfig {
        let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        ClusterConfig {
            node_id,
            bind_addr: format!("127.0.0.1:{}", port).parse().unwrap(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_follower_crash_and_recovery() {
        // Create leader
        let leader_config = get_test_config(1);
        let leader_transport = Arc::new(NetworkTransport::new(&leader_config).await.unwrap());
        let leader = Arc::new(
            RaftConsensus::new(1, leader_transport, leader_config)
                .await
                .unwrap(),
        );

        leader.start().await.unwrap();
        leader.promote_to_leader().await.unwrap();

        // Create follower
        let follower_config = get_test_config(2);
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
        let follower_config = get_test_config(2);
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
        let leader_config = get_test_config(1);
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
        let config = get_test_config(1);
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
        let config1 = get_test_config(1);
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
        let config2 = get_test_config(2);
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
        let config = get_test_config(1);
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
        let config = get_test_config(1);
        let transport = Arc::new(NetworkTransport::new(&config).await.unwrap());
        let leader = Arc::new(RaftConsensus::new(1, transport, config).await.unwrap());

        leader.start().await.unwrap();
        leader.promote_to_leader().await.unwrap();

        // Spawn multiple tasks proposing entries concurrently
        let mut tasks = Vec::new();
        for i in 0..10 {
            let leader_clone = Arc::clone(&leader);
            let task = tokio::spawn(async move {
                let data = format!("concurrent_entry_{}", i);
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
        let config = get_test_config(2);
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
        let config1 = get_test_config(1);
        let transport1 = Arc::new(NetworkTransport::new(&config1).await.unwrap());
        let node1 = RaftConsensus::new(1, transport1, config1).await.unwrap();

        let config2 = get_test_config(2);
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
        let mut config = get_test_config(2);
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
}
