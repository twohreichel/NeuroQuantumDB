//! Unit tests for shard management and consistent hashing.

use neuroquantum_cluster::config::ClusterConfig;
use neuroquantum_cluster::sharding::{
    ShardInfo, ShardManager, ShardState, ShardTransfer, TransferStatus,
};

#[tokio::test]
async fn test_shard_manager_creation() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config);
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_add_remove_node() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    // Add a node
    let result = manager.add_node(1).await;
    assert!(result.is_ok());

    // Adding same node again should fail
    let result = manager.add_node(1).await;
    assert!(result.is_err());

    // Remove the node
    let result = manager.remove_node(1).await;
    assert!(result.is_ok());

    // Removing non-existent node should fail
    let result = manager.remove_node(1).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_primary_node() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    // Add some nodes
    manager.add_node(1).await.unwrap();
    manager.add_node(2).await.unwrap();
    manager.add_node(3).await.unwrap();

    // Get primary for a key
    let result = manager.get_primary_node(b"test-key").await;
    assert!(result.is_ok());

    // Same key should always map to same primary
    let primary1 = manager.get_primary_node(b"test-key").await.unwrap();
    let primary2 = manager.get_primary_node(b"test-key").await.unwrap();
    assert_eq!(primary1, primary2);
}

#[tokio::test]
async fn test_get_nodes_for_key() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    // Add nodes
    manager.add_node(1).await.unwrap();
    manager.add_node(2).await.unwrap();
    manager.add_node(3).await.unwrap();

    // Get nodes for key
    let nodes = manager.get_nodes_for_key(b"test-key").await.unwrap();

    // Should return replication_factor nodes (or fewer if cluster is smaller)
    assert!(!nodes.is_empty());
    assert!(nodes.len() <= config.sharding.replication_factor as usize);
}

#[tokio::test]
async fn test_consistent_hashing() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    manager.add_node(1).await.unwrap();
    manager.add_node(2).await.unwrap();

    // Record assignments for some keys
    let mut assignments = Vec::new();
    for i in 0..10 {
        let key = format!("key-{i}");
        let primary = manager.get_primary_node(key.as_bytes()).await.unwrap();
        assignments.push((key, primary));
    }

    // Add a new node
    manager.add_node(3).await.unwrap();

    // Check that most keys still map to same node (consistent hashing property)
    let mut unchanged = 0;
    for (key, old_primary) in &assignments {
        let new_primary = manager.get_primary_node(key.as_bytes()).await.unwrap();
        if *old_primary == new_primary {
            unchanged += 1;
        }
    }

    // With consistent hashing, adding a node should only move ~1/N keys
    // At least half should remain unchanged
    assert!(unchanged >= assignments.len() / 2);
}

#[tokio::test]
async fn test_rebalance_lifecycle() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    manager.add_node(1).await.unwrap();
    manager.add_node(2).await.unwrap();

    // Register a shard
    let shard_info = ShardInfo {
        shard_id: 100,
        primary_node: 1,
        replica_nodes: vec![],
        state: ShardState::Active,
        key_count: 500,
        size_bytes: 512 * 1024,
    };
    manager.register_shard(shard_info).await.unwrap();

    // Start rebalance
    let transfers = manager.start_rebalance().await.unwrap();

    // If there are transfers, test the lifecycle
    if !transfers.is_empty() {
        let transfer = &transfers[0];
        let transfer_id = transfer.transfer_id;

        // Start the transfer
        manager.start_transfer(transfer_id).await.unwrap();

        // Update progress
        manager
            .update_transfer_progress(transfer_id, 256 * 1024, 250)
            .await
            .unwrap();

        // Check progress
        let progress = manager.get_rebalance_progress().await;
        assert!(progress.active);
        assert_eq!(progress.total_transfers, transfers.len());
        assert_eq!(progress.in_progress_transfers, 1);

        // Complete the transfer
        manager.complete_transfer(transfer_id).await.unwrap();

        let completed_transfer = manager.get_transfer(transfer_id).await.unwrap();
        assert_eq!(completed_transfer.status, TransferStatus::Completed);
    }

    // Complete rebalance
    manager.complete_rebalance().await.unwrap();
}

#[tokio::test]
async fn test_transfer_failure() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    manager.add_node(1).await.unwrap();
    manager.add_node(2).await.unwrap();

    // Register a shard
    let shard_info = ShardInfo {
        shard_id: 100,
        primary_node: 1,
        replica_nodes: vec![],
        state: ShardState::Active,
        key_count: 100,
        size_bytes: 1024,
    };
    manager.register_shard(shard_info).await.unwrap();

    // Start rebalance
    let transfers = manager.start_rebalance().await.unwrap();

    if !transfers.is_empty() {
        let transfer_id = transfers[0].transfer_id;

        // Start and then fail the transfer
        manager.start_transfer(transfer_id).await.unwrap();
        manager
            .fail_transfer(transfer_id, "Network error".into())
            .await
            .unwrap();

        let failed_transfer = manager.get_transfer(transfer_id).await.unwrap();
        assert_eq!(failed_transfer.status, TransferStatus::Failed);
        assert!(failed_transfer.error.is_some());

        // Check shard is back to active
        let shard = manager.get_shard(100).await.unwrap();
        assert_eq!(shard.state, ShardState::Active);
    }

    manager.complete_rebalance().await.unwrap();
}

#[tokio::test]
async fn test_cancel_rebalance() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    manager.add_node(1).await.unwrap();
    manager.add_node(2).await.unwrap();

    // Register a shard
    let shard_info = ShardInfo {
        shard_id: 100,
        primary_node: 1,
        replica_nodes: vec![],
        state: ShardState::Active,
        key_count: 100,
        size_bytes: 1024,
    };
    manager.register_shard(shard_info).await.unwrap();

    // Start rebalance
    manager.start_rebalance().await.unwrap();
    assert!(manager.is_rebalancing().await);

    // Cancel rebalance
    manager.cancel_rebalance().await.unwrap();
    assert!(!manager.is_rebalancing().await);

    // Shard should be active again
    let shard = manager.get_shard(100).await.unwrap();
    assert_eq!(shard.state, ShardState::Active);
}

#[tokio::test]
async fn test_node_leave_transfers() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    manager.add_node(1).await.unwrap();
    manager.add_node(2).await.unwrap();
    manager.add_node(3).await.unwrap();

    // Register shards on node 2
    for i in 0..3 {
        let shard_info = ShardInfo {
            shard_id: 100 + i,
            primary_node: 2,
            replica_nodes: vec![],
            state: ShardState::Active,
            key_count: 100,
            size_bytes: 1024,
        };
        manager.register_shard(shard_info).await.unwrap();
    }

    // Calculate transfers when node 2 leaves
    let transfers = manager.calculate_node_leave_transfers(2).await.unwrap();

    // All shards from node 2 should need to be transferred
    assert_eq!(transfers.len(), 3);
    for transfer in &transfers {
        assert_eq!(transfer.source_node, 2);
        assert_ne!(transfer.target_node, 2);
    }
}

#[tokio::test]
async fn test_rebalance_progress() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    // Before rebalancing, progress should show inactive
    let progress = manager.get_rebalance_progress().await;
    assert!(!progress.active);
    assert_eq!(progress.total_transfers, 0);

    manager.add_node(1).await.unwrap();
    manager.add_node(2).await.unwrap();

    // Register some shards
    for i in 0..5 {
        let shard_info = ShardInfo {
            shard_id: i,
            primary_node: 1,
            replica_nodes: vec![],
            state: ShardState::Active,
            key_count: 100,
            size_bytes: 1024,
        };
        manager.register_shard(shard_info).await.unwrap();
    }

    manager.start_rebalance().await.unwrap();

    let progress = manager.get_rebalance_progress().await;
    assert!(progress.active);
    assert!(progress.started_at_ms > 0);

    manager.complete_rebalance().await.unwrap();
}

#[tokio::test]
async fn test_transfer_progress_calculation() {
    let transfer = ShardTransfer {
        transfer_id: 1,
        shard_id: 100,
        source_node: 1,
        target_node: 2,
        status: TransferStatus::InProgress,
        bytes_transferred: 500,
        total_bytes: 1000,
        keys_transferred: 50,
        total_keys: 100,
        started_at_ms: 0,
        completed_at_ms: 0,
        error: None,
    };

    assert!((transfer.progress_percent() - 50.0).abs() < 0.01);

    let completed_transfer = ShardTransfer {
        status: TransferStatus::Completed,
        total_bytes: 0,
        ..transfer.clone()
    };
    assert!((completed_transfer.progress_percent() - 100.0).abs() < 0.01);

    let empty_transfer = ShardTransfer {
        status: TransferStatus::Pending,
        total_bytes: 0,
        bytes_transferred: 0,
        ..transfer
    };
    assert!((empty_transfer.progress_percent() - 0.0).abs() < 0.01);
}

#[tokio::test]
async fn test_rebalance_config() {
    let config = ClusterConfig::default();
    let manager = ShardManager::new(&config).unwrap();

    let rebalance_config = manager.rebalance_config();
    assert!(rebalance_config.auto_rebalance);
    assert_eq!(rebalance_config.max_concurrent_transfers, 2);
}
