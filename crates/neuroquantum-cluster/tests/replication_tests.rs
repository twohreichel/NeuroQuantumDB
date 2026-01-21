//! Unit tests for data replication.

use std::time::Duration;

use neuroquantum_cluster::consensus::FencingToken;
use neuroquantum_cluster::error::ClusterError;
use neuroquantum_cluster::replication::{
    ConsistencyLevel, ReplicationAck, ReplicationManager, ReplicationStatus,
};

#[test]
fn test_consistency_level_default() {
    assert_eq!(ConsistencyLevel::default(), ConsistencyLevel::Quorum);
}

#[tokio::test]
async fn test_replication_request() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

    let request_id = manager
        .replicate(
            1,
            b"key".to_vec(),
            b"value".to_vec(),
            vec![2, 3],
            ConsistencyLevel::One,
        )
        .await
        .unwrap();

    assert_eq!(request_id, 1);

    let status = manager.get_status(request_id).await.unwrap();
    assert_eq!(status, ReplicationStatus::InProgress);
}

#[tokio::test]
async fn test_replication_ack() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

    let request_id = manager
        .replicate(
            1,
            b"key".to_vec(),
            b"value".to_vec(),
            vec![2, 3],
            ConsistencyLevel::One,
        )
        .await
        .unwrap();

    let completed = manager
        .process_ack(ReplicationAck {
            request_id,
            node_id: 2,
            success: true,
            error: None,
        })
        .await
        .unwrap();

    assert!(completed);

    let status = manager.get_status(request_id).await.unwrap();
    assert_eq!(status, ReplicationStatus::Succeeded);
}

#[test]
fn test_required_acks() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

    assert_eq!(manager.required_acks(ConsistencyLevel::One), 1);
    assert_eq!(manager.required_acks(ConsistencyLevel::Quorum), 2);
    assert_eq!(manager.required_acks(ConsistencyLevel::All), 3);
}

// Split brain prevention tests
#[tokio::test]
async fn test_replicate_with_fencing_token() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));
    let token = FencingToken::new(1, 0);

    let request_id = manager
        .replicate_with_token(
            1,
            b"key".to_vec(),
            b"value".to_vec(),
            vec![2, 3],
            ConsistencyLevel::Quorum,
            Some(token),
        )
        .await
        .unwrap();

    assert_eq!(request_id, 1);
}

#[tokio::test]
async fn test_validate_token_accepts_newer() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

    // Set initial token
    let token1 = FencingToken::new(1, 0);
    manager.update_highest_token(token1).await;

    // Newer token should be accepted
    let token2 = FencingToken::new(1, 1);
    assert!(manager.validate_token(&token2).await.is_ok());

    // Even newer token from higher term should be accepted
    let token3 = FencingToken::new(2, 0);
    assert!(manager.validate_token(&token3).await.is_ok());
}

#[tokio::test]
async fn test_validate_token_rejects_stale() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

    // Set current token
    let current_token = FencingToken::new(5, 10);
    manager.update_highest_token(current_token).await;

    // Stale token from earlier term should be rejected
    let stale_token = FencingToken::new(4, 0);
    let result = manager.validate_token(&stale_token).await;
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

    // Stale token from same term but lower sequence should be rejected
    let stale_seq = FencingToken::new(5, 5);
    assert!(manager.validate_token(&stale_seq).await.is_err());
}

#[tokio::test]
async fn test_update_highest_token() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

    // Initial update
    let token1 = FencingToken::new(1, 0);
    manager.update_highest_token(token1).await;
    assert_eq!(manager.get_highest_token().await, Some(token1));

    // Update with newer token
    let token2 = FencingToken::new(1, 1);
    manager.update_highest_token(token2).await;
    assert_eq!(manager.get_highest_token().await, Some(token2));

    // Try to update with older token (should not change)
    let old_token = FencingToken::new(1, 0);
    manager.update_highest_token(old_token).await;
    assert_eq!(manager.get_highest_token().await, Some(token2));
}

#[tokio::test]
async fn test_replicate_with_stale_token_fails() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

    // Set current token
    let current = FencingToken::new(5, 0);
    manager.update_highest_token(current).await;

    // Try to replicate with stale token
    let stale = FencingToken::new(4, 0);
    let result = manager
        .replicate_with_token(
            1,
            b"key".to_vec(),
            b"value".to_vec(),
            vec![2, 3],
            ConsistencyLevel::Quorum,
            Some(stale),
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_token_progression() {
    let manager = ReplicationManager::new(1, 3, Duration::from_secs(5));

    // Simulate progression of tokens
    for i in 0..5 {
        let token = FencingToken::new(1, i);
        manager.update_highest_token(token).await;
        assert_eq!(manager.get_highest_token().await.unwrap().sequence, i);
    }

    // Term increases
    let new_term_token = FencingToken::new(2, 0);
    manager.update_highest_token(new_term_token).await;
    assert_eq!(manager.get_highest_token().await.unwrap().term, 2);
}
