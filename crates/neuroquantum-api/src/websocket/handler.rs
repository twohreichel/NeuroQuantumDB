//! Integrated WebSocket Handler
//!
//! Combines ConnectionManager and PubSubManager for a complete
//! real-time communication solution.

use crate::websocket::{
    manager::{ConnectionError, ConnectionManager},
    pubsub::{ChannelId, PubSubManager},
    types::{ConnectionId, ConnectionMetadata},
};
use actix_ws::{Message, Session};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Subscribe to a channel
    Subscribe { channel: String },

    /// Unsubscribe from a channel
    Unsubscribe { channel: String },

    /// Publish a message to a channel
    Publish {
        channel: String,
        data: serde_json::Value,
    },

    /// Ping (heartbeat)
    Ping { timestamp: Option<String> },

    /// Pong (heartbeat response)
    Pong { timestamp: String },

    /// Query status request
    QueryStatus { query_id: String },

    /// Generic message
    Message { data: serde_json::Value },
}

/// WebSocket response message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsResponse {
    /// Subscription confirmed
    SubscriptionConfirmed { channel: String, timestamp: String },

    /// Unsubscription confirmed
    UnsubscriptionConfirmed { channel: String, timestamp: String },

    /// Message from a channel
    ChannelMessage {
        channel: String,
        data: serde_json::Value,
        timestamp: String,
    },

    /// Pong response
    Pong { timestamp: String },

    /// Query status update
    QueryStatus {
        query_id: String,
        status: String,
        progress: Option<u32>,
    },

    /// Error message
    Error { code: String, message: String },
}

/// Integrated WebSocket service
pub struct WebSocketService {
    connection_manager: Arc<ConnectionManager>,
    pubsub_manager: Arc<PubSubManager>,
}

impl WebSocketService {
    /// Create a new WebSocket service
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        pubsub_manager: Arc<PubSubManager>,
    ) -> Self {
        info!("✅ WebSocketService initialized");
        Self {
            connection_manager,
            pubsub_manager,
        }
    }

    /// Handle a new WebSocket connection
    pub async fn handle_connection(
        &self,
        session: Session,
        mut msg_stream: actix_ws::MessageStream,
        metadata: ConnectionMetadata,
    ) -> Result<(), ConnectionError> {
        // Register the connection
        let conn_id = self.connection_manager.register(session, metadata).await?;

        info!("🔌 WebSocket connection established: {}", conn_id);

        // Process messages
        while let Some(Ok(msg)) = msg_stream.next().await {
            if let Err(e) = self.handle_message(conn_id, msg).await {
                error!("Error handling message for {}: {:?}", conn_id, e);
                break;
            }
        }

        // Cleanup: unsubscribe from all channels
        if let Err(e) = self.pubsub_manager.unsubscribe_all(conn_id).await {
            warn!("Failed to unsubscribe connection {}: {:?}", conn_id, e);
        }

        // Unregister the connection
        if let Err(e) = self.connection_manager.unregister(conn_id).await {
            warn!("Failed to unregister connection {}: {:?}", conn_id, e);
        }

        info!("🔌 WebSocket connection closed: {}", conn_id);

        Ok(())
    }

    /// Handle a single WebSocket message
    async fn handle_message(
        &self,
        conn_id: ConnectionId,
        msg: Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match msg {
            Message::Text(text) => {
                debug!("📨 Received text message from {}: {}", conn_id, text);
                self.handle_text_message(conn_id, &text).await?;
            }
            Message::Binary(data) => {
                debug!(
                    "📦 Received binary message from {} ({} bytes)",
                    conn_id,
                    data.len()
                );
                // Binary messages could be handled here
            }
            Message::Ping(data) => {
                debug!("🏓 Received ping from {}", conn_id);
                if let Some(connection) = self.connection_manager.get_connection(conn_id) {
                    connection.session.write().await.pong(&data).await?;
                }
            }
            Message::Pong(_) => {
                debug!("🏓 Received pong from {}", conn_id);
                self.connection_manager
                    .handle_heartbeat_response(conn_id)
                    .await?;
            }
            Message::Close(reason) => {
                info!("👋 Connection {} closing: {:?}", conn_id, reason);
                return Err("Connection closed".into());
            }
            _ => {
                warn!("⚠️  Unsupported message type from {}", conn_id);
            }
        }

        Ok(())
    }

    /// Handle a text message (JSON-based protocol)
    async fn handle_text_message(
        &self,
        conn_id: ConnectionId,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let msg: WsMessage = serde_json::from_str(text)?;

        match msg {
            WsMessage::Subscribe { channel } => {
                self.handle_subscribe(conn_id, channel).await?;
            }
            WsMessage::Unsubscribe { channel } => {
                self.handle_unsubscribe(conn_id, channel).await?;
            }
            WsMessage::Publish { channel, data } => {
                self.handle_publish(conn_id, channel, data).await?;
            }
            WsMessage::Ping { timestamp } => {
                self.handle_ping(conn_id, timestamp).await?;
            }
            WsMessage::Pong { timestamp: _ } => {
                self.connection_manager
                    .handle_heartbeat_response(conn_id)
                    .await?;
            }
            WsMessage::QueryStatus { query_id } => {
                self.handle_query_status(conn_id, query_id).await?;
            }
            WsMessage::Message { data } => {
                debug!("💬 Received generic message from {}: {:?}", conn_id, data);
            }
        }

        Ok(())
    }

    /// Handle subscribe request
    async fn handle_subscribe(
        &self,
        conn_id: ConnectionId,
        channel: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.pubsub_manager.subscribe(conn_id, &channel).await?;

        let response = WsResponse::SubscriptionConfirmed {
            channel,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.send_to_connection(conn_id, &response).await?;

        Ok(())
    }

    /// Handle unsubscribe request
    async fn handle_unsubscribe(
        &self,
        conn_id: ConnectionId,
        channel: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.pubsub_manager.unsubscribe(conn_id, &channel).await?;

        let response = WsResponse::UnsubscriptionConfirmed {
            channel,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.send_to_connection(conn_id, &response).await?;

        Ok(())
    }

    /// Handle publish request
    async fn handle_publish(
        &self,
        _conn_id: ConnectionId,
        channel: String,
        data: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channel_id = ChannelId::new(channel.clone());
        let subscribers = self.pubsub_manager.publish(&channel_id, &data).await;

        let response = WsResponse::ChannelMessage {
            channel: channel.clone(),
            data: data.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Send to all subscribers
        let subscriber_count = subscribers.len();
        for sub_id in &subscribers {
            if let Err(e) = self.send_to_connection(*sub_id, &response).await {
                warn!("Failed to send to subscriber {}: {:?}", sub_id, e);
            }
        }

        debug!(
            "📤 Published message to channel {}, {} subscribers",
            channel, subscriber_count
        );

        Ok(())
    }

    /// Handle ping request
    async fn handle_ping(
        &self,
        conn_id: ConnectionId,
        _timestamp: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = WsResponse::Pong {
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.send_to_connection(conn_id, &response).await?;
        self.connection_manager
            .handle_heartbeat_response(conn_id)
            .await?;

        Ok(())
    }

    /// Handle query status request
    async fn handle_query_status(
        &self,
        conn_id: ConnectionId,
        query_id: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // This would integrate with the query execution engine
        // For now, return a mock response
        let response = WsResponse::QueryStatus {
            query_id,
            status: "running".to_string(),
            progress: Some(75),
        };

        self.send_to_connection(conn_id, &response).await?;

        Ok(())
    }

    /// Send a response to a specific connection
    async fn send_to_connection(
        &self,
        conn_id: ConnectionId,
        response: &WsResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(connection) = self.connection_manager.get_connection(conn_id) {
            connection.send_json(response).await?;
        }
        Ok(())
    }

    /// Broadcast a message to a channel
    pub async fn broadcast_to_channel(
        &self,
        channel: &str,
        data: serde_json::Value,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let channel_id = ChannelId::new(channel);
        let subscribers = self.pubsub_manager.publish(&channel_id, &data).await;

        let response = WsResponse::ChannelMessage {
            channel: channel.to_string(),
            data,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let mut count = 0;
        for sub_id in &subscribers {
            if self.send_to_connection(*sub_id, &response).await.is_ok() {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get service statistics
    pub async fn get_stats(&self) -> ServiceStats {
        let conn_metrics = self.connection_manager.get_metrics();
        let pubsub_stats = self.pubsub_manager.get_all_stats().await;

        ServiceStats {
            active_connections: conn_metrics.active_connections,
            total_connections: conn_metrics.total_connections,
            total_messages_sent: conn_metrics.total_messages_sent,
            total_messages_received: conn_metrics.total_messages_received,
            active_channels: pubsub_stats.channel_count,
            total_channel_messages: pubsub_stats.total_messages,
        }
    }
}

/// Service statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStats {
    pub active_connections: u64,
    pub total_connections: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub active_channels: usize,
    pub total_channel_messages: u64,
}
