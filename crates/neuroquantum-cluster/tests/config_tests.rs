//! Unit tests for cluster configuration.

use std::time::Duration;

use neuroquantum_cluster::config::ClusterConfig;

#[test]
fn test_default_config() {
    let config = ClusterConfig::default();
    assert_eq!(config.node_id, 1);
    assert!(config.validate().is_ok());
}

#[test]
fn test_builder() {
    let config = ClusterConfig::builder()
        .node_id(42)
        .bind_addr("127.0.0.1:9001".parse().unwrap())
        .peers(vec!["127.0.0.1:9002".into(), "127.0.0.1:9003".into()])
        .replication_factor(3)
        .build()
        .unwrap();

    assert_eq!(config.node_id, 42);
    assert_eq!(config.peers.len(), 2);
    assert_eq!(config.sharding.replication_factor, 3);
}

#[test]
fn test_invalid_node_id() {
    let result = ClusterConfig::builder().node_id(0).build();
    assert!(result.is_err());
}

#[test]
fn test_invalid_election_timeout() {
    let result = ClusterConfig::builder()
        .election_timeout(Duration::from_millis(500), Duration::from_millis(300))
        .build();
    assert!(result.is_err());
}
