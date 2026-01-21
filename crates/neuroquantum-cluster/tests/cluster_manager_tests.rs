//! Unit tests for cluster manager.

use std::sync::atomic::{AtomicU16, Ordering};

use neuroquantum_cluster::cluster_manager::ClusterManager;
use neuroquantum_cluster::config::ClusterConfig;
use neuroquantum_cluster::error::ClusterError;
use neuroquantum_cluster::node::NodeState;

fn get_test_config() -> ClusterConfig {
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(20000);

    let port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    ClusterConfig {
        node_id: 1,
        bind_addr: format!("127.0.0.1:{port}").parse().unwrap(),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_cluster_manager_creation() {
    let config = get_test_config();
    let manager = ClusterManager::new(config).await;
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_cluster_manager_start_stop() {
    let config = get_test_config();
    let manager = ClusterManager::new(config).await.unwrap();

    // Start
    let start_result = manager.start().await;
    assert!(start_result.is_ok());

    // Check status
    let status = manager.status().await;
    assert_eq!(status.local_state, NodeState::Running);

    // Stop
    let stop_result = manager.stop().await;
    assert!(stop_result.is_ok());
}

#[tokio::test]
async fn test_cluster_manager_status() {
    let config = get_test_config();
    let manager = ClusterManager::new(config).await.unwrap();

    manager.start().await.unwrap();

    let status = manager.status().await;
    assert_eq!(status.node_count, 1); // Just self
    assert_eq!(status.healthy_nodes, 1); // Self is healthy
    assert!(status.has_quorum); // Single node has quorum

    manager.stop().await.unwrap();
}

#[tokio::test]
async fn test_propose_not_leader() {
    let config = get_test_config();
    let manager = ClusterManager::new(config).await.unwrap();

    manager.start().await.unwrap();

    // Initially not a leader
    let result = manager.propose(b"test command".to_vec()).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ClusterError::NotLeader(_, _)));

    manager.stop().await.unwrap();
}
