//! Cluster error types and result definitions.

use std::net::SocketAddr;

use thiserror::Error;

/// Result type for cluster operations.
pub type ClusterResult<T> = Result<T, ClusterError>;

/// Cluster-specific errors.
#[derive(Error, Debug)]
pub enum ClusterError {
    /// Node not found in cluster
    #[error("Node {0} not found in cluster")]
    NodeNotFound(u64),

    /// Leader not elected yet
    #[error("No leader elected in cluster")]
    NoLeader,

    /// Node is not the leader
    #[error("Node {0} is not the leader, leader is {1:?}")]
    NotLeader(u64, Option<u64>),

    /// Failed to connect to peer
    #[error("Failed to connect to peer {0}: {1}")]
    ConnectionFailed(SocketAddr, String),

    /// Raft consensus error
    #[error("Raft consensus error: {0}")]
    RaftError(String),

    /// Network transport error
    #[error("Network transport error: {0}")]
    NetworkError(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Shard not found
    #[error("Shard {0} not found")]
    ShardNotFound(u64),

    /// Shard rebalancing in progress
    #[error("Shard rebalancing in progress, try again later")]
    RebalancingInProgress,

    /// Node already exists
    #[error("Node {0} already exists in cluster")]
    NodeAlreadyExists(u64),

    /// Quorum not reached
    #[error("Quorum not reached: need {needed}, have {have}")]
    QuorumNotReached { needed: usize, have: usize },

    /// Timeout waiting for operation
    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Internal error
    #[error("Internal cluster error: {0}")]
    Internal(String),

    /// Discovery error
    #[error("Service discovery error: {0}")]
    DiscoveryError(String),

    /// Replication error
    #[error("Replication error: {0}")]
    ReplicationError(String),

    /// Lock acquisition failed
    #[error("Failed to acquire lock: {0}")]
    LockError(String),

    /// No quorum available for write operation
    #[error("No quorum available: cannot perform write operation without majority")]
    NoQuorum,

    /// Stale fencing token
    #[error("Stale fencing token: current term {current_term}, received {received_term}")]
    StaleToken {
        current_term: u64,
        received_term: u64,
    },

    /// Leader lease expired
    #[error("Leader lease expired: cannot perform write operations")]
    LeaseExpired,

    /// Network partition detected
    #[error("Network partition detected: node is in minority partition")]
    NetworkPartition,

    /// Invalid state transition
    #[error("Invalid state: expected {expected}, actual {actual}")]
    InvalidState { expected: String, actual: String },

    /// Health check failed
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Protocol version mismatch
    #[error("Protocol version mismatch with node {node_id}: expected >= {expected}, got {actual}")]
    ProtocolVersionMismatch {
        node_id: u64,
        expected: u32,
        actual: u32,
    },

    /// Upgrade in progress
    #[error("Upgrade operation already in progress")]
    UpgradeInProgress,

    /// Minimum healthy nodes requirement not met
    #[error("Minimum healthy nodes requirement not met: {current} < {required}")]
    InsufficientHealthyNodes { current: usize, required: usize },
}

impl From<std::io::Error> for ClusterError {
    fn from(err: std::io::Error) -> Self {
        Self::NetworkError(err.to_string())
    }
}

impl From<bincode::Error> for ClusterError {
    fn from(err: bincode::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<serde_json::Error> for ClusterError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<tonic::Status> for ClusterError {
    fn from(err: tonic::Status) -> Self {
        Self::NetworkError(format!("gRPC error: {}", err))
    }
}

impl From<tonic::transport::Error> for ClusterError {
    fn from(err: tonic::transport::Error) -> Self {
        Self::NetworkError(format!("gRPC transport error: {}", err))
    }
}
