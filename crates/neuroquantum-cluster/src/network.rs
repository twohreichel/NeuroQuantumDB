//! Network transport layer for inter-node communication.
//!
//! This module provides gRPC-based network communication for the cluster,
//! enabling multi-node deployments with the following features:
//!
//! ## Features
//! - **gRPC Server**: Listens for incoming RPC calls from cluster peers
//! - **gRPC Client**: Establishes connections to peer nodes
//! - **Connection Management**: Automatic connection pooling and health tracking
//! - **RPC Methods**:
//!   - `Handshake`: Node discovery and connection establishment
//!   - `AppendEntries`: Raft log replication and heartbeats
//!   - `RequestVote`: Raft leader election
//!   - `Heartbeat`: Health checks
//!   - `InstallSnapshot`: Snapshot transfer for lagging nodes
//!
//! ## Usage
//! ```no_run
//! use std::sync::Arc;
//! use neuroquantum_cluster::network::NetworkTransport;
//! use neuroquantum_cluster::config::ClusterConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ClusterConfig::default();
//! let transport = Arc::new(NetworkTransport::new(&config).await?);
//!
//! // Start the gRPC server and connect to peers
//! transport.start().await?;
//!
//! // Server is now listening and clients are connected
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::{debug, error, info, warn};

use crate::config::ClusterConfig;
use crate::error::{ClusterError, ClusterResult};
use crate::node::NodeId;

// Forward declare RaftConsensus to avoid circular dependency
use std::future::Future;
use std::pin::Pin;

// Type alias for consensus handler callbacks
type RequestVoteHandler = Arc<
    dyn Fn(
            RequestVoteRequest,
        ) -> Pin<Box<dyn Future<Output = ClusterResult<RequestVoteResponse>> + Send>>
        + Send
        + Sync,
>;

// Include generated protobuf code
pub mod proto {
    tonic::include_proto!("neuroquantum.cluster");
}

use proto::cluster_node_client::ClusterNodeClient;
use proto::cluster_node_server::{ClusterNode as ClusterNodeService, ClusterNodeServer};

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
    /// Term of `prev_log_index` entry
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
    /// True if follower contained entry matching `prev_log_index` and `prev_log_term`
    pub success: bool,
    /// Hint for the next index to try (optimization)
    pub match_index: Option<u64>,
    /// Conflict index for faster catchup
    pub conflict_index: Option<u64>,
    /// Conflict term for faster catchup
    pub conflict_term: Option<u64>,
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
    /// Term of `last_included_index`
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
struct PeerConnection {
    /// Peer node ID
    /// Reserved for future peer-to-peer routing and health monitoring
    #[allow(dead_code)]
    node_id: NodeId,
    /// Peer address
    addr: SocketAddr,
    /// Whether connection is established
    connected: bool,
    /// Last successful communication time
    last_contact_ms: u64,
    /// gRPC client for this peer
    client: Option<ClusterNodeClient<tonic::transport::Channel>>,
    /// Protocol version of the peer
    /// Reserved for future version negotiation and compatibility checks
    #[allow(dead_code)]
    protocol_version: u32,
}

/// Network transport for cluster communication.
pub struct NetworkTransport {
    /// Local node ID
    node_id: NodeId,
    /// Bind address for gRPC server
    bind_addr: SocketAddr,
    /// Peer connections
    peers: RwLock<HashMap<NodeId, PeerConnection>>,
    /// Configuration
    config: ClusterConfig,
    /// Running flag
    running: RwLock<bool>,
    /// Server shutdown signal
    server_shutdown: RwLock<Option<tokio::sync::oneshot::Sender<()>>>,
    /// Handler for `RequestVote` RPCs
    request_vote_handler: RwLock<Option<RequestVoteHandler>>,
}

/// gRPC service implementation for cluster node
struct ClusterNodeServiceImpl {
    node_id: NodeId,
    transport: Arc<NetworkTransport>,
}

#[tonic::async_trait]
impl ClusterNodeService for ClusterNodeServiceImpl {
    async fn handshake(
        &self,
        request: tonic::Request<proto::HandshakeRequest>,
    ) -> Result<tonic::Response<proto::HandshakeResponse>, tonic::Status> {
        let req = request.into_inner();
        info!(
            local_node = self.node_id,
            remote_node = req.node_id,
            remote_addr = req.address,
            remote_protocol_version = req.protocol_version,
            "Received handshake request"
        );

        // Parse the remote address
        let remote_addr: SocketAddr = req
            .address
            .parse()
            .map_err(|e| tonic::Status::invalid_argument(format!("Invalid address: {e}")))?;

        // Get our protocol version
        let our_protocol_version = self.transport.config.manager.upgrades.protocol_version;
        let min_compatible = self
            .transport
            .config
            .manager
            .upgrades
            .min_compatible_version;

        // Check protocol compatibility
        if req.protocol_version < min_compatible {
            warn!(
                local_node = self.node_id,
                remote_node = req.node_id,
                remote_version = req.protocol_version,
                min_compatible,
                "Incompatible protocol version"
            );
            return Ok(tonic::Response::new(proto::HandshakeResponse {
                node_id: self.node_id,
                success: false,
                error: format!(
                    "Incompatible protocol version: {} < {}",
                    req.protocol_version, min_compatible
                ),
                protocol_version: our_protocol_version,
            }));
        }

        // Add the peer to our connections
        if let Err(e) = self.transport.add_peer(req.node_id, remote_addr).await {
            warn!(
                local_node = self.node_id,
                remote_node = req.node_id,
                error = %e,
                "Failed to add peer during handshake"
            );
            return Ok(tonic::Response::new(proto::HandshakeResponse {
                node_id: self.node_id,
                success: false,
                error: format!("Failed to add peer: {e}"),
                protocol_version: our_protocol_version,
            }));
        }

        Ok(tonic::Response::new(proto::HandshakeResponse {
            node_id: self.node_id,
            success: true,
            error: String::new(),
            protocol_version: our_protocol_version,
        }))
    }

    async fn append_entries(
        &self,
        request: tonic::Request<proto::AppendEntriesRequest>,
    ) -> Result<tonic::Response<proto::AppendEntriesResponse>, tonic::Status> {
        let req = request.into_inner();
        debug!(
            local_node = self.node_id,
            leader = req.leader_id,
            term = req.term,
            entries = req.entries.len(),
            "Received append entries request"
        );

        // Placeholder response - actual Raft logic would be implemented here
        Ok(tonic::Response::new(proto::AppendEntriesResponse {
            term: req.term,
            success: true,
            last_log_index: req.prev_log_index,
            conflict_index: None,
            conflict_term: None,
        }))
    }

    async fn request_vote(
        &self,
        request: tonic::Request<proto::VoteRequest>,
    ) -> Result<tonic::Response<proto::VoteResponse>, tonic::Status> {
        let req = request.into_inner();
        debug!(
            local_node = self.node_id,
            candidate = req.candidate_id,
            term = req.term,
            is_pre_vote = req.is_pre_vote,
            "Received vote request"
        );

        // Convert proto request to internal format
        let vote_request = RequestVoteRequest {
            term: req.term,
            candidate_id: req.candidate_id,
            last_log_index: req.last_log_index,
            last_log_term: req.last_log_term,
            is_pre_vote: req.is_pre_vote,
        };

        // Call the registered handler if available
        let handler = self.transport.request_vote_handler.read().await;
        if let Some(ref h) = *handler {
            match h(vote_request).await {
                | Ok(response) => {
                    return Ok(tonic::Response::new(proto::VoteResponse {
                        term: response.term,
                        vote_granted: response.vote_granted,
                        is_pre_vote: req.is_pre_vote,
                    }));
                },
                | Err(e) => {
                    warn!(
                        local_node = self.node_id,
                        error = %e,
                        "Failed to process vote request"
                    );
                    return Err(tonic::Status::internal(format!(
                        "Failed to process vote request: {e}"
                    )));
                },
            }
        }

        // Fallback: deny vote if no handler registered
        Ok(tonic::Response::new(proto::VoteResponse {
            term: req.term,
            vote_granted: false,
            is_pre_vote: req.is_pre_vote,
        }))
    }

    async fn heartbeat(
        &self,
        request: tonic::Request<proto::HeartbeatRequest>,
    ) -> Result<tonic::Response<proto::HeartbeatResponse>, tonic::Status> {
        let req = request.into_inner();
        debug!(
            local_node = self.node_id,
            from = req.from,
            "Received heartbeat"
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(tonic::Response::new(proto::HeartbeatResponse {
            from: self.node_id,
            timestamp_ms: now,
        }))
    }

    async fn install_snapshot(
        &self,
        request: tonic::Request<proto::InstallSnapshotRequest>,
    ) -> Result<tonic::Response<proto::InstallSnapshotResponse>, tonic::Status> {
        let req = request.into_inner();
        debug!(
            local_node = self.node_id,
            leader = req.leader_id,
            term = req.term,
            last_included_index = req.last_included_index,
            "Received install snapshot request"
        );

        // Placeholder response - actual snapshot logic would be implemented here
        Ok(tonic::Response::new(proto::InstallSnapshotResponse {
            term: req.term,
            success: true,
        }))
    }
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
            server_shutdown: RwLock::new(None),
            request_vote_handler: RwLock::new(None),
        })
    }

    /// Register a handler for `RequestVote` RPCs.
    pub async fn register_request_vote_handler<F>(&self, handler: F)
    where
        F: Fn(
                RequestVoteRequest,
            )
                -> Pin<Box<dyn Future<Output = ClusterResult<RequestVoteResponse>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let mut h = self.request_vote_handler.write().await;
        *h = Some(Arc::new(handler));
    }

    /// Start the network transport.
    pub async fn start(self: Arc<Self>) -> ClusterResult<()> {
        info!(node_id = self.node_id, "Starting network transport");

        {
            let mut running = self.running.write().await;
            *running = true;
        }

        // Start gRPC server
        let bind_addr = self.bind_addr;
        let node_id = self.node_id;
        let transport_clone = Arc::clone(&self);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Store shutdown signal
        {
            let mut shutdown = self.server_shutdown.write().await;
            *shutdown = Some(shutdown_tx);
        }

        // Spawn gRPC server in background
        tokio::spawn(async move {
            let service_impl = ClusterNodeServiceImpl {
                node_id,
                transport: transport_clone,
            };

            let svc = ClusterNodeServer::new(service_impl);

            info!(node_id = node_id, bind_addr = %bind_addr, "Starting gRPC server");

            if let Err(e) = Server::builder()
                .add_service(svc)
                .serve_with_shutdown(bind_addr, async {
                    shutdown_rx.await.ok();
                })
                .await
            {
                error!(node_id = node_id, error = %e, "gRPC server failed");
            }
        });

        // Connect to known peers
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

        // Shutdown gRPC server
        {
            let mut shutdown = self.server_shutdown.write().await;
            if let Some(tx) = shutdown.take() {
                let _ = tx.send(());
                info!(
                    node_id = self.node_id,
                    "Sent shutdown signal to gRPC server"
                );
            }
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
    pub async fn send(&self, target: NodeId, message: &ClusterMessage) -> ClusterResult<()> {
        let mut peers = self.peers.write().await;

        let peer = peers
            .get_mut(&target)
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
            message_type = ?std::mem::discriminant(message),
            "Sending message via gRPC"
        );

        // Use gRPC client to send the message
        if let Some(ref mut client) = peer.client {
            match message {
                | ClusterMessage::Ping(ping_req) => {
                    let req = proto::HeartbeatRequest {
                        from: ping_req.from,
                        timestamp_ms: ping_req.timestamp_ms,
                    };
                    client.heartbeat(req).await.map_err(|e| {
                        ClusterError::ConnectionFailed(
                            peer.addr,
                            format!("gRPC call failed: {e}"),
                        )
                    })?;
                },
                | ClusterMessage::RequestVote(vote_req) => {
                    let req = proto::VoteRequest {
                        term: vote_req.term,
                        candidate_id: vote_req.candidate_id,
                        last_log_index: vote_req.last_log_index,
                        last_log_term: vote_req.last_log_term,
                        is_pre_vote: vote_req.is_pre_vote,
                    };
                    // Send vote request and await response
                    client.request_vote(req).await.map_err(|e| {
                        ClusterError::ConnectionFailed(
                            peer.addr,
                            format!("gRPC call failed: {e}"),
                        )
                    })?;
                },
                | ClusterMessage::AppendEntries(append_req) => {
                    // Convert entries to proto format
                    let proto_entries: Vec<proto::LogEntry> = append_req
                        .entries
                        .iter()
                        .enumerate()
                        .map(|(i, entry)| proto::LogEntry {
                            index: append_req.prev_log_index + 1 + i as u64,
                            term: entry.term,
                            data: entry.data.clone(),
                        })
                        .collect();

                    let req = proto::AppendEntriesRequest {
                        term: append_req.term,
                        leader_id: append_req.leader_id,
                        prev_log_index: append_req.prev_log_index,
                        prev_log_term: append_req.prev_log_term,
                        entries: proto_entries,
                        leader_commit: append_req.leader_commit,
                    };
                    client.append_entries(req).await.map_err(|e| {
                        ClusterError::ConnectionFailed(
                            peer.addr,
                            format!("gRPC call failed: {e}"),
                        )
                    })?;
                },
                | _ => {
                    // Other message types would be handled similarly
                    debug!("Message type not yet implemented for gRPC");
                },
            }

            // Update last contact time
            peer.last_contact_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
        }

        Ok(())
    }

    /// Send `RequestVote` RPC and return the response.
    pub async fn send_request_vote_rpc(
        &self,
        target: NodeId,
        request: RequestVoteRequest,
    ) -> ClusterResult<RequestVoteResponse> {
        let mut peers = self.peers.write().await;

        let peer = peers
            .get_mut(&target)
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
            term = request.term,
            "Sending RequestVote RPC"
        );

        // Use gRPC client to send the request
        if let Some(ref mut client) = peer.client {
            let req = proto::VoteRequest {
                term: request.term,
                candidate_id: request.candidate_id,
                last_log_index: request.last_log_index,
                last_log_term: request.last_log_term,
                is_pre_vote: request.is_pre_vote,
            };

            let response = client
                .request_vote(req)
                .await
                .map_err(|e| {
                    ClusterError::ConnectionFailed(peer.addr, format!("gRPC call failed: {e}"))
                })?
                .into_inner();

            // Update last contact time
            peer.last_contact_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            Ok(RequestVoteResponse {
                term: response.term,
                vote_granted: response.vote_granted,
            })
        } else {
            Err(ClusterError::ConnectionFailed(
                peer.addr,
                "No client available".into(),
            ))
        }
    }

    /// Broadcast a message to all peers.
    pub async fn broadcast(&self, message: ClusterMessage) -> ClusterResult<()> {
        let peers = self.peers.read().await;

        for (&peer_id, peer) in peers.iter() {
            if peer.connected {
                // Pass reference to message - no clone needed
                if let Err(e) = self.send(peer_id, &message).await {
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
        let mut peers = self.peers.write().await;

        for peer_addr in &self.config.peers {
            debug!(
                node_id = self.node_id,
                peer = peer_addr,
                "Attempting to connect to peer"
            );

            // Establish gRPC connection
            let endpoint = format!("http://{peer_addr}");
            match ClusterNodeClient::connect(endpoint.clone()).await {
                | Ok(mut client) => {
                    info!(
                        node_id = self.node_id,
                        peer = peer_addr,
                        "Established gRPC connection"
                    );

                    // Perform handshake to exchange node IDs
                    let handshake_req = proto::HandshakeRequest {
                        node_id: self.node_id,
                        address: self.bind_addr.to_string(),
                        term: 0, // Current term would come from Raft consensus
                        protocol_version: self.config.manager.upgrades.protocol_version,
                    };

                    match client.handshake(handshake_req).await {
                        | Ok(response) => {
                            let resp = response.into_inner();
                            if resp.success {
                                info!(
                                    local_node = self.node_id,
                                    remote_node = resp.node_id,
                                    remote_protocol_version = resp.protocol_version,
                                    "Handshake successful"
                                );

                                // Add peer to connections
                                let peer_socket_addr: SocketAddr = match peer_addr.parse() {
                                    | Ok(addr) => addr,
                                    | Err(e) => {
                                        warn!(
                                            node_id = self.node_id,
                                            peer = peer_addr,
                                            error = %e,
                                            "Failed to parse peer address, skipping"
                                        );
                                        continue;
                                    },
                                };

                                peers.insert(
                                    resp.node_id,
                                    PeerConnection {
                                        node_id: resp.node_id,
                                        addr: peer_socket_addr,
                                        connected: true,
                                        last_contact_ms: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis()
                                            as u64,
                                        client: Some(client),
                                        protocol_version: resp.protocol_version,
                                    },
                                );
                            } else {
                                warn!(
                                    node_id = self.node_id,
                                    peer = peer_addr,
                                    error = resp.error,
                                    "Handshake failed"
                                );
                            }
                        },
                        | Err(e) => {
                            warn!(
                                node_id = self.node_id,
                                peer = peer_addr,
                                error = %e,
                                "Failed to perform handshake"
                            );
                        },
                    }
                },
                | Err(e) => {
                    warn!(
                        node_id = self.node_id,
                        peer = peer_addr,
                        error = %e,
                        "Failed to establish gRPC connection"
                    );
                },
            }
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
                client: None,
                protocol_version: 0, // Will be updated during handshake
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
        use std::net::{IpAddr, Ipv4Addr};

        let config = ClusterConfig {
            node_id: 1,
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
            advertise_addr: None,
            peers: vec![],
            data_dir: std::path::PathBuf::from("/tmp/test"),
            raft: crate::config::RaftConfig::default(),
            network: crate::config::NetworkConfig::default(),
            sharding: crate::config::ShardingConfig::default(),
            discovery: crate::config::DiscoveryConfig::default(),
            manager: crate::config::ClusterManagerConfig::default(),
        };

        let transport = NetworkTransport::new(&config).await;
        assert!(transport.is_ok());

        let transport = transport.unwrap();
        assert_eq!(transport.node_id, 1);
        assert_eq!(transport.bind_addr.port(), 8080);
    }
}
