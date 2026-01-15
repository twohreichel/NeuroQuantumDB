//! WebSocket Connection Manager
//!
//! Enterprise-grade connection lifecycle management with:
//! - Automatic registration/unregistration
//! - Heartbeat monitoring with configurable timeouts
//! - Broadcast support for all connections
//! - Connection statistics and metrics
//! - Graceful shutdown

use crate::websocket::metrics::ConnectionMetrics;
use crate::websocket::types::{Connection, ConnectionId, ConnectionMetadata};
use actix_ws::Session;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, error, info, warn};

/// Configuration for the connection manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Maximum number of concurrent connections allowed
    pub max_connections: usize,

    /// Heartbeat check interval
    pub heartbeat_interval: Duration,

    /// Heartbeat timeout (connection is closed if no heartbeat within this time)
    pub heartbeat_timeout: Duration,

    /// Idle connection timeout
    pub idle_timeout: Duration,

    /// Enable automatic heartbeat monitoring
    pub enable_heartbeat_monitor: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_connections: 10_000,
            heartbeat_interval: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(90),
            idle_timeout: Duration::from_secs(300),
            enable_heartbeat_monitor: true,
        }
    }
}

/// WebSocket Connection Manager
///
/// Manages the lifecycle of all WebSocket connections with automatic
/// heartbeat monitoring, metrics tracking, and graceful shutdown.
pub struct ConnectionManager {
    /// Active connections indexed by ConnectionId
    connections: Arc<DashMap<ConnectionId, Arc<Connection>>>,

    /// Connection metrics
    metrics: Arc<ConnectionMetrics>,

    /// Configuration
    config: ConnectionConfig,

    /// Shutdown signal
    shutdown: Arc<RwLock<bool>>,
}

impl ConnectionManager {
    /// Create a new connection manager with the given configuration
    pub fn new(config: ConnectionConfig) -> Self {
        let manager = Self {
            connections: Arc::new(DashMap::new()),
            metrics: Arc::new(ConnectionMetrics::new()),
            config,
            shutdown: Arc::new(RwLock::new(false)),
        };

        // Start heartbeat monitor if enabled
        if manager.config.enable_heartbeat_monitor {
            manager.start_heartbeat_monitor();
        }

        info!(
            "✅ ConnectionManager initialized (max_connections: {})",
            manager.config.max_connections
        );

        manager
    }

    /// Register a new WebSocket connection
    ///
    /// Returns the unique ConnectionId on success, or an error if the
    /// maximum number of connections has been reached.
    pub async fn register(
        &self,
        session: Session,
        metadata: ConnectionMetadata,
    ) -> Result<ConnectionId, ConnectionError> {
        // Check if we've reached the maximum number of connections
        if self.connections.len() >= self.config.max_connections {
            warn!(
                "⚠️  Maximum connections reached ({}/{})",
                self.connections.len(),
                self.config.max_connections
            );
            return Err(ConnectionError::MaxConnectionsReached);
        }

        let connection = Arc::new(Connection::new(session, metadata.clone()));
        let conn_id = connection.id;

        // Insert the connection
        self.connections.insert(conn_id, connection.clone());

        // Update metrics
        self.metrics.increment_total_connections();
        self.metrics.increment_active_connections();

        info!(
            "📡 New WebSocket connection registered: {} (user: {:?}, remote: {})",
            conn_id, metadata.user_id, metadata.remote_addr
        );

        debug!(
            "Active connections: {}/{}",
            self.connections.len(),
            self.config.max_connections
        );

        Ok(conn_id)
    }

    /// Unregister a connection
    ///
    /// Removes the connection from the manager and updates metrics.
    pub async fn unregister(&self, conn_id: ConnectionId) -> Result<(), ConnectionError> {
        if let Some((_, connection)) = self.connections.remove(&conn_id) {
            // Close the connection gracefully
            if let Err(e) = connection.close().await {
                warn!("Failed to close connection {}: {:?}", conn_id, e);
            }

            // Update metrics
            self.metrics.decrement_active_connections();

            info!("👋 Connection unregistered: {}", conn_id);
            debug!("Active connections: {}", self.connections.len());

            Ok(())
        } else {
            Err(ConnectionError::ConnectionNotFound)
        }
    }

    /// Get a connection by ID
    pub fn get_connection(&self, conn_id: ConnectionId) -> Option<Arc<Connection>> {
        self.connections.get(&conn_id).map(|entry| entry.clone())
    }

    /// Get all active connection IDs
    pub fn get_all_connection_ids(&self) -> Vec<ConnectionId> {
        self.connections.iter().map(|entry| *entry.key()).collect()
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Send a message to a specific connection
    pub async fn send_to(
        &self,
        conn_id: ConnectionId,
        message: impl Into<String>,
    ) -> Result<(), ConnectionError> {
        let connection = self
            .get_connection(conn_id)
            .ok_or(ConnectionError::ConnectionNotFound)?;

        connection
            .send_text(message)
            .await
            .map_err(|_| ConnectionError::SendFailed)?;

        self.metrics.increment_messages_sent(1);

        Ok(())
    }

    /// Broadcast a message to all active connections
    ///
    /// Returns the number of connections that successfully received the message.
    pub async fn broadcast(&self, message: impl Into<String> + Clone) -> usize {
        let msg = message.into();
        let mut success_count = 0;
        let mut failed_connections = Vec::new();

        for entry in self.connections.iter() {
            let conn_id = *entry.key();
            let connection = entry.value();

            match connection.send_text(msg.clone()).await {
                | Ok(_) => {
                    success_count += 1;
                },
                | Err(e) => {
                    warn!("Failed to broadcast to connection {}: {:?}", conn_id, e);
                    failed_connections.push(conn_id);
                },
            }
        }

        // Remove failed connections
        for conn_id in failed_connections {
            if let Err(e) = self.unregister(conn_id).await {
                error!("Failed to unregister dead connection {}: {:?}", conn_id, e);
            }
        }

        self.metrics.increment_broadcast_messages();
        self.metrics.increment_messages_sent(success_count);

        debug!("📢 Broadcast sent to {} connections", success_count);

        success_count as usize
    }

    /// Broadcast a JSON message to all active connections
    pub async fn broadcast_json<T: Serialize>(&self, data: &T) -> Result<usize, ConnectionError> {
        let json = serde_json::to_string(data)
            .map_err(|e| ConnectionError::SerializationFailed(e.to_string()))?;

        Ok(self.broadcast(json).await)
    }

    /// Send a heartbeat (ping) to a specific connection
    pub async fn send_heartbeat(&self, conn_id: ConnectionId) -> Result<(), ConnectionError> {
        let connection = self
            .get_connection(conn_id)
            .ok_or(ConnectionError::ConnectionNotFound)?;

        let ping_message = serde_json::json!({
            "type": "ping",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        connection
            .send_json(&ping_message)
            .await
            .map_err(|_| ConnectionError::SendFailed)?;

        Ok(())
    }

    /// Handle a heartbeat response (pong) from a client
    pub async fn handle_heartbeat_response(
        &self,
        conn_id: ConnectionId,
    ) -> Result<(), ConnectionError> {
        let connection = self
            .get_connection(conn_id)
            .ok_or(ConnectionError::ConnectionNotFound)?;

        connection.update_heartbeat().await;
        debug!("💓 Heartbeat received from connection: {}", conn_id);

        Ok(())
    }

    /// Start the heartbeat monitor task
    ///
    /// This spawns a background task that periodically checks all connections
    /// for heartbeat timeouts and closes dead connections.
    fn start_heartbeat_monitor(&self) {
        let connections = self.connections.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(config.heartbeat_interval);

            info!(
                "💓 Heartbeat monitor started (interval: {:?}, timeout: {:?})",
                config.heartbeat_interval, config.heartbeat_timeout
            );

            loop {
                interval.tick().await;

                // Check shutdown signal
                if *shutdown.read().await {
                    info!("Heartbeat monitor shutting down");
                    break;
                }

                let mut dead_connections = Vec::new();

                // Check all connections for heartbeat timeout
                for entry in connections.iter() {
                    let conn_id = *entry.key();
                    let connection = entry.value();

                    if !connection.is_healthy(config.heartbeat_timeout).await {
                        warn!(
                            "💀 Connection {} failed heartbeat check (timeout: {:?})",
                            conn_id, config.heartbeat_timeout
                        );
                        dead_connections.push(conn_id);
                        metrics.increment_heartbeat_failures();
                    }
                }

                // Remove dead connections
                for conn_id in dead_connections {
                    if let Some((_, connection)) = connections.remove(&conn_id) {
                        if let Err(e) = connection.close().await {
                            error!("Failed to close dead connection {}: {:?}", conn_id, e);
                        }
                        metrics.decrement_active_connections();
                        info!("🗑️  Dead connection removed: {}", conn_id);
                    }
                }

                debug!(
                    "💓 Heartbeat check completed. Active connections: {}",
                    connections.len()
                );
            }
        });
    }

    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> crate::websocket::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Gracefully shut down the connection manager
    ///
    /// Closes all active connections and stops the heartbeat monitor.
    pub async fn shutdown(&self) {
        info!("🛑 ConnectionManager shutting down...");

        // Signal heartbeat monitor to stop
        *self.shutdown.write().await = true;

        // Close all connections
        let conn_ids: Vec<ConnectionId> = self.connections.iter().map(|e| *e.key()).collect();

        for conn_id in conn_ids {
            if let Err(e) = self.unregister(conn_id).await {
                error!(
                    "Failed to unregister connection {} during shutdown: {:?}",
                    conn_id, e
                );
            }
        }

        info!("✅ ConnectionManager shut down complete");
    }
}

/// Connection-related errors
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Maximum number of connections reached")]
    MaxConnectionsReached,

    #[error("Connection not found")]
    ConnectionNotFound,

    #[error("Failed to send message")]
    SendFailed,

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Connection closed")]
    ConnectionClosed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_config_default() {
        let config = ConnectionConfig::default();
        assert_eq!(config.max_connections, 10_000);
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.heartbeat_timeout, Duration::from_secs(90));
        assert!(config.enable_heartbeat_monitor);
    }

    #[test]
    fn test_connection_error_display() {
        let err = ConnectionError::MaxConnectionsReached;
        assert_eq!(err.to_string(), "Maximum number of connections reached");
    }
}
