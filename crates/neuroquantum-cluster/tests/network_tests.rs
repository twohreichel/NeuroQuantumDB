//! Unit tests for network transport layer.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use neuroquantum_cluster::config::{
    ClusterConfig, ClusterManagerConfig, DiscoveryConfig, NetworkConfig, RaftConfig, ShardingConfig,
};
use neuroquantum_cluster::network::{
    proto, ClusterMessage, NetworkTransport, PingRequest, RequestVoteRequest,
};

#[test]
fn test_message_serialization() {
    let msg = ClusterMessage::Ping(PingRequest {
        from: 1,
        timestamp_ms: 12345,
    });

    let serialized = serde_json::to_string(&msg).unwrap();
    let deserialized: ClusterMessage = serde_json::from_str(&serialized).unwrap();

    if let ClusterMessage::Ping(ping) = deserialized {
        assert_eq!(ping.from, 1);
        assert_eq!(ping.timestamp_ms, 12345);
    } else {
        panic!("Wrong message type after deserialization");
    }
}

#[test]
fn test_request_vote_serialization() {
    let req = RequestVoteRequest {
        term: 5,
        candidate_id: 2,
        last_log_index: 100,
        last_log_term: 4,
        is_pre_vote: true,
    };

    let serialized = bincode::serialize(&req).unwrap();
    let deserialized: RequestVoteRequest = bincode::deserialize(&serialized).unwrap();

    assert_eq!(deserialized.term, 5);
    assert_eq!(deserialized.candidate_id, 2);
    assert!(deserialized.is_pre_vote);
}

#[tokio::test]
async fn test_proto_handshake_request() {
    // Test that proto messages can be created correctly
    let handshake_req = proto::HandshakeRequest {
        node_id: 1,
        address: "127.0.0.1:8080".to_string(),
        term: 0,
        protocol_version: 1,
    };

    assert_eq!(handshake_req.node_id, 1);
    assert_eq!(handshake_req.address, "127.0.0.1:8080");
    assert_eq!(handshake_req.term, 0);
    assert_eq!(handshake_req.protocol_version, 1);
}

#[tokio::test]
async fn test_proto_heartbeat() {
    // Test heartbeat proto message
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let heartbeat_req = proto::HeartbeatRequest {
        from: 1,
        timestamp_ms: now,
    };

    assert_eq!(heartbeat_req.from, 1);
    assert!(heartbeat_req.timestamp_ms > 0);
}

#[tokio::test]
async fn test_proto_vote_request() {
    // Test vote request proto message
    let vote_req = proto::VoteRequest {
        term: 5,
        candidate_id: 2,
        last_log_index: 100,
        last_log_term: 4,
        is_pre_vote: false,
    };

    assert_eq!(vote_req.term, 5);
    assert_eq!(vote_req.candidate_id, 2);
    assert_eq!(vote_req.last_log_index, 100);
    assert!(!vote_req.is_pre_vote);
}

#[tokio::test]
async fn test_network_transport_creation() {
    let config = ClusterConfig {
        node_id: 1,
        bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
        advertise_addr: None,
        peers: vec![],
        data_dir: PathBuf::from("/tmp/test"),
        raft: RaftConfig::default(),
        network: NetworkConfig::default(),
        sharding: ShardingConfig::default(),
        discovery: DiscoveryConfig::default(),
        manager: ClusterManagerConfig::default(),
    };

    let transport = NetworkTransport::new(&config).await;
    assert!(transport.is_ok());

    let transport = transport.unwrap();
    assert_eq!(transport.node_id(), 1);
    assert_eq!(transport.bind_addr().port(), 8080);
}
