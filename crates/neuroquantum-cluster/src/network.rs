//! Network transport layer for inter-node communication.

use std::collections::HashMap;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::ClusterConfig;
use crate::error::{ClusterError, ClusterResult};
use crate::node::NodeId;

/// Message types for cluster communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    /// Request vote from peers during election
    RequestVote(RequestVoteRequest),
    /// Response to vote request
    RequestVoteResponse(RequestVoteResponse),
    /// Append entries (heartbeat or log replication)
    AppendEntries(AppendEntriesRequest),
    /// Response to append entries
    AppendEntriesResponse(AppendEntriesResponse),
    /// Install snapshot
    InstallSnapshot(InstallSnapshotRequest),
    /// Response to install snapshot
    InstallSnapshotResponse(InstallSnapshotResponse),
    /// Ping for health check
    Ping(PingRequest),
    /// Pong response
    Pong(PongResponse),
}

/// Request vote RPC (Raft election).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteRequest {
    /// Candidate's term
    pub term: u64,
    /// Candidate requesting vote
    pub candidate_id: NodeId,
    /// Index of candidate's last log entry
    pub last_log_index: u64,
    /// Term of candidate's last log entry
    pub last_log_term: u64,
    /// Pre-vote flag (if true, this is a pre-vote request)
    pub is_pre_vote: bool,
}

/// Response to request vote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// Current term, for candidate to update itself
    pub term: u64,
    /// True means candidate received vote
    pub vote_granted: bool,
}

/// Append entries RPC (log replication and heartbeat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: u64,
    /// Leader ID so follower can redirect clients
    pub leader_id: NodeId,
    /// Index of log entry immediately preceding new ones
    pub prev_log_index: u64,
    /// Term of prev_log_index entry
    pub prev_log_term: u64,
    /// Log entries to store (empty for heartbeat)
    pub entries: Vec<LogEntryCompact>,
    /// Leader's commit index
    pub leader_commit: u64,
}

/// Compact log entry for network transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntryCompact {
    /// Term when entry was received by leader
    pub term: u64,
    /// Entry data (serialized)
    pub data: Vec<u8>,
}

/// Response to append entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term, for leader to update itself
    pub term: u64,
    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
    /// Hint for the next index to try (optimization)
    pub match_index: Option<u64>,
}

/// Install snapshot RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    /// Leader's term
    pub term: u64,
    /// Leader ID
    pub leader_id: NodeId,
    /// The snapshot replaces all entries up through and including this index
    pub last_included_index: u64,
    /// Term of last_included_index
    pub last_included_term: u64,
    /// Byte offset where chunk is positioned in the snapshot file
    pub offset: u64,
    /// Raw bytes of the snapshot chunk
    pub data: Vec<u8>,
    /// True if this is the last chunk
    pub done: bool,
}

/// Response to install snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    /// Current term, for leader to update itself
    pub term: u64,
}

/// Simple ping request for health checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    /// Sender node ID
    pub from: NodeId,
    /// Timestamp (for RTT measurement)
    pub timestamp_ms: u64,
}

/// Pong response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongResponse {
    /// Responder node ID
    pub from: NodeId,
    /// Original timestamp
    pub request_timestamp_ms: u64,
    /// Response timestamp
    pub response_timestamp_ms: u64,
}

/// Connection state for a peer.
#[allow(dead_code)]
struct PeerConnection {
    /// Peer node ID (used in full gRPC implementation)
    node_id: NodeId,
    /// Peer address
    addr: SocketAddr,
    /// Whether connection is established
    connected: bool,
    /// Last successful communication time (used in full implementation)
    last_contact_ms: u64,
}

/// Network transport for cluster communication.
#[allow(dead_code)]
pub struct NetworkTransport {
    /// Local node ID
    node_id: NodeId,
    /// Bind address (used when starting gRPC server)
    bind_addr: SocketAddr,
    /// Peer connections
    peers: RwLock<HashMap<NodeId, PeerConnection>>,
    /// Configuration
    config: ClusterConfig,
    /// Running flag
    running: RwLock<bool>,
}

impl NetworkTransport {
    /// Create a new network transport.
    pub async fn new(config: &ClusterConfig) -> ClusterResult<Self> {
        info!(
            node_id = config.node_id,
            bind_addr = %config.bind_addr,
            "Creating network transport"
        );

        Ok(Self {
            node_id: config.node_id,
            bind_addr: config.bind_addr,
            peers: RwLock::new(HashMap::new()),
            config: config.clone(),
            running: RwLock::new(false),
        })
    }

    /// Start the network transport.
    pub async fn start(&self) -> ClusterResult<()> {
        info!(node_id = self.node_id, "Starting network transport");

        {
            let mut running = self.running.write().await;
            *running = true;
        }

        // In a full implementation:
        // 1. Start gRPC server on bind_addr
        // 2. Connect to known peers
        // 3. Start connection health monitoring

        self.connect_to_peers().await?;

        info!(node_id = self.node_id, "Network transport started");
        Ok(())
    }

    /// Stop the network transport.
    pub async fn stop(&self) -> ClusterResult<()> {
        info!(node_id = self.node_id, "Stopping network transport");

        {
            let mut running = self.running.write().await;
            *running = false;
        }

        // Close all peer connections
        {
            let mut peers = self.peers.write().await;
            peers.clear();
        }

        info!(node_id = self.node_id, "Network transport stopped");
        Ok(())
    }

    /// Send a message to a specific peer.
    pub async fn send(&self, target: NodeId, message: ClusterMessage) -> ClusterResult<()> {
        let peers = self.peers.read().await;

        let peer = peers
            .get(&target)
            .ok_or(ClusterError::NodeNotFound(target))?;

        if !peer.connected {
            return Err(ClusterError::ConnectionFailed(
                peer.addr,
                "Peer not connected".into(),
            ));
        }

        debug!(
            from = self.node_id,
            to = target,
            message_type = ?std::mem::discriminant(&message),
            "Sending message"
        );

        // In a full implementation, this would use gRPC client to send the message

        Ok(())
    }

    /// Broadcast a message to all peers.
    pub async fn broadcast(&self, message: ClusterMessage) -> ClusterResult<()> {
        let peers = self.peers.read().await;

        for (&peer_id, peer) in peers.iter() {
            if peer.connected {
                // Clone message for each peer
                if let Err(e) = self.send(peer_id, message.clone()).await {
                    warn!(
                        from = self.node_id,
                        to = peer_id,
                        error = %e,
                        "Failed to send broadcast message"
                    );
                }
            }
        }

        Ok(())
    }

    /// Connect to known peers.
    async fn connect_to_peers(&self) -> ClusterResult<()> {
        let peers = self.peers.write().await;

        for peer_addr in &self.config.peers {
            debug!(
                node_id = self.node_id,
                peer = peer_addr,
                "Attempting to connect to peer"
            );

            // In a full implementation:
            // 1. Resolve peer address
            // 2. Establish gRPC connection
            // 3. Exchange node IDs
            // 4. Add to peers map
        }

        info!(
            node_id = self.node_id,
            connected_peers = peers.len(),
            "Connected to peers"
        );

        Ok(())
    }

    /// Add a new peer connection.
    pub async fn add_peer(&self, node_id: NodeId, addr: SocketAddr) -> ClusterResult<()> {
        let mut peers = self.peers.write().await;

        if peers.contains_key(&node_id) {
            return Err(ClusterError::NodeAlreadyExists(node_id));
        }

        peers.insert(
            node_id,
            PeerConnection {
                node_id,
                addr,
                connected: false,
                last_contact_ms: 0,
            },
        );

        info!(
            local = self.node_id,
            remote = node_id,
            addr = %addr,
            "Added new peer"
        );

        Ok(())
    }

    /// Remove a peer connection.
    pub async fn remove_peer(&self, node_id: NodeId) -> ClusterResult<()> {
        let mut peers = self.peers.write().await;

        peers
            .remove(&node_id)
            .ok_or(ClusterError::NodeNotFound(node_id))?;

        info!(local = self.node_id, remote = node_id, "Removed peer");

        Ok(())
    }

    /// Get the number of connected peers.
    pub async fn connected_peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.values().filter(|p| p.connected).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
