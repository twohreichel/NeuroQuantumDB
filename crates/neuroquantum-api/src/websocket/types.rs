//! WebSocket connection types and data structures
//!
//! Defines core types for connection management, metadata, and status tracking.

use std::sync::Arc;
use std::time::{Duration, Instant};

use actix_ws::Session;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unique identifier for a WebSocket connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    /// Create a new unique connection ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Metadata associated with a connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    /// User identifier (if authenticated)
    pub user_id: Option<String>,

    /// API key or token used for authentication
    pub auth_token: Option<String>,

    /// Client IP address
    pub remote_addr: String,

    /// User-Agent header
    pub user_agent: Option<String>,

    /// Connection timestamp
    pub connected_at: chrono::DateTime<chrono::Utc>,

    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,

    /// Custom metadata (e.g., session info, client version)
    pub custom: std::collections::HashMap<String, String>,
}

impl ConnectionMetadata {
    /// Create new metadata with minimal information
    #[must_use]
    pub fn new(remote_addr: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id: None,
            auth_token: None,
            remote_addr,
            user_agent: None,
            connected_at: now,
            last_activity: now,
            custom: std::collections::HashMap::new(),
        }
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now();
    }

    /// Check if connection has been idle for longer than the given duration
    #[must_use]
    pub fn is_idle(&self, timeout: Duration) -> bool {
        let elapsed = chrono::Utc::now()
            .signed_duration_since(self.last_activity)
            .to_std()
            .unwrap_or(Duration::ZERO);
        elapsed > timeout
    }
}

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Connection is active and healthy
    Active,

    /// Connection is idle (no recent activity)
    Idle,

    /// Connection is being closed gracefully
    Closing,

    /// Connection has been closed
    Closed,

    /// Connection failed due to an error
    Failed,
}

/// A WebSocket connection with its session and metadata
pub struct Connection {
    /// Unique connection identifier
    pub id: ConnectionId,

    /// Actix WebSocket session for sending messages
    pub session: Arc<RwLock<Session>>,

    /// Connection metadata
    pub metadata: Arc<RwLock<ConnectionMetadata>>,

    /// Current connection status
    pub status: Arc<RwLock<ConnectionStatus>>,

    /// Last heartbeat timestamp
    pub last_heartbeat: Arc<RwLock<Instant>>,

    /// Number of messages sent
    pub messages_sent: Arc<RwLock<u64>>,

    /// Number of messages received
    pub messages_received: Arc<RwLock<u64>>,
}

impl Connection {
    /// Create a new connection
    #[must_use]
    pub fn new(session: Session, metadata: ConnectionMetadata) -> Self {
        Self {
            id: ConnectionId::new(),
            session: Arc::new(RwLock::new(session)),
            metadata: Arc::new(RwLock::new(metadata)),
            status: Arc::new(RwLock::new(ConnectionStatus::Active)),
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
            messages_sent: Arc::new(RwLock::new(0)),
            messages_received: Arc::new(RwLock::new(0)),
        }
    }

    /// Send a text message to the client
    pub async fn send_text(&self, text: impl Into<String>) -> Result<(), actix_ws::Closed> {
        let mut session = self.session.write().await;
        let text_string = text.into();
        let result = session.text(text_string).await;

        if result.is_ok() {
            let mut count = self.messages_sent.write().await;
            *count += 1;
        }

        result
    }

    /// Send a JSON message to the client
    pub async fn send_json<T: Serialize>(
        &self,
        data: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(data)?;
        self.send_text(json)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Update heartbeat timestamp
    pub async fn update_heartbeat(&self) {
        let mut last = self.last_heartbeat.write().await;
        *last = Instant::now();

        let mut metadata = self.metadata.write().await;
        metadata.update_activity();
    }

    /// Check if connection needs heartbeat check
    pub async fn needs_heartbeat(&self, interval: Duration) -> bool {
        let last = self.last_heartbeat.read().await;
        last.elapsed() > interval
    }

    /// Check if connection is healthy
    pub async fn is_healthy(&self, timeout: Duration) -> bool {
        let last = self.last_heartbeat.read().await;
        last.elapsed() < timeout
    }

    /// Get current status
    pub async fn get_status(&self) -> ConnectionStatus {
        *self.status.read().await
    }

    /// Set connection status
    pub async fn set_status(&self, new_status: ConnectionStatus) {
        let mut status = self.status.write().await;
        *status = new_status;
    }

    /// Close the connection gracefully
    pub async fn close(&self) -> Result<(), actix_ws::Closed> {
        self.set_status(ConnectionStatus::Closing).await;

        let session = self.session.write().await;
        let result = session.clone().close(None).await;

        drop(session); // Release lock before updating status
        self.set_status(ConnectionStatus::Closed).await;

        result
    }

    /// Increment received message counter
    pub async fn increment_received(&self) {
        let mut count = self.messages_received.write().await;
        *count += 1;

        let mut metadata = self.metadata.write().await;
        metadata.update_activity();
    }

    /// Get connection statistics
    pub async fn get_stats(&self) -> ConnectionStats {
        ConnectionStats {
            id: self.id,
            messages_sent: *self.messages_sent.read().await,
            messages_received: *self.messages_received.read().await,
            status: *self.status.read().await,
            connected_at: self.metadata.read().await.connected_at,
            last_activity: self.metadata.read().await.last_activity,
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub id: ConnectionId,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub status: ConnectionStatus,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}
