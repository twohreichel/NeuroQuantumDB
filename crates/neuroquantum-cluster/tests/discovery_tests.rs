//! Unit tests for service discovery.

use neuroquantum_cluster::config::{ClusterConfig, DiscoveryConfig, DiscoveryMethod};
use neuroquantum_cluster::discovery::{DiscoveryService, NodeMetadata, NodeRegistration};
use neuroquantum_cluster::error::ClusterError;

#[tokio::test]
async fn test_static_discovery() {
    let config = ClusterConfig {
        node_id: 1,
        discovery: DiscoveryConfig {
            method: DiscoveryMethod::Static,
            static_nodes: vec![
                "127.0.0.1:9001".into(),
                "127.0.0.1:9002".into(),
                "127.0.0.1:9003".into(),
            ],
            ..Default::default()
        },
        ..Default::default()
    };

    let service = DiscoveryService::new(&config).unwrap();
    let peers = service.discover().await.unwrap();

    // Should discover 2 peers (excluding self with node_id 1)
    assert_eq!(peers.len(), 2);
}

#[test]
fn test_node_registration_serialization() {
    let reg = NodeRegistration {
        node_id: 1,
        address: "127.0.0.1:9000".into(),
        metadata: NodeMetadata {
            version: "0.1.0".into(),
            datacenter: Some("us-east-1".into()),
            tags: vec!["primary".into()],
        },
        registered_at: 1234567890,
        ttl_secs: 30,
    };

    let json = serde_json::to_string(&reg).unwrap();
    let parsed: NodeRegistration = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.node_id, 1);
    assert_eq!(parsed.metadata.version, "0.1.0");
}

#[tokio::test]
async fn test_dns_discovery_missing_config() {
    let config = ClusterConfig {
        node_id: 1,
        discovery: DiscoveryConfig {
            method: DiscoveryMethod::Dns,
            dns: None, // Missing DNS config
            ..Default::default()
        },
        ..Default::default()
    };

    let service = DiscoveryService::new(&config).unwrap();
    let result = service.discover().await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ClusterError::ConfigError(_)));
}

#[tokio::test]
async fn test_consul_discovery_missing_config() {
    let config = ClusterConfig {
        node_id: 1,
        discovery: DiscoveryConfig {
            method: DiscoveryMethod::Consul,
            consul: None, // Missing Consul config
            ..Default::default()
        },
        ..Default::default()
    };

    let service = DiscoveryService::new(&config).unwrap();
    let result = service.discover().await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ClusterError::ConfigError(_)));
}

#[tokio::test]
async fn test_etcd_discovery_missing_config() {
    let config = ClusterConfig {
        node_id: 1,
        discovery: DiscoveryConfig {
            method: DiscoveryMethod::Etcd,
            etcd: None, // Missing etcd config
            ..Default::default()
        },
        ..Default::default()
    };

    let service = DiscoveryService::new(&config).unwrap();
    let result = service.discover().await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ClusterError::ConfigError(_)));
}
