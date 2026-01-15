//! Comprehensive integration tests for Raft log replication

#[cfg(test)]
mod replication_tests {
    use super::super::*;
    use std::sync::atomic::{AtomicU16, Ordering};
    use std::sync::Arc;

    // Port counter for tests
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(20000);

    fn get_test_config(node_id: NodeId) -> ClusterConfig {
        let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
        ClusterConfig {
            node_id,
            bind_addr: format!("127.0.0.1:{port}").parse().unwrap(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_initialize_replication_state() {
        let config = get_test_config(1);
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
        let config = get_test_config(1);
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
        let config = get_test_config(2);
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
        let config = get_test_config(2);
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
        let config = get_test_config(2);
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
        let config = get_test_config(2);
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
        let config = get_test_config(2);
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
        let config = get_test_config(1);
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
        let config = get_test_config(1);
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
        let config = get_test_config(1);
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
        let config = get_test_config(1);
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
        let config = get_test_config(1);
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
}
