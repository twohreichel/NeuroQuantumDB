//! Integrated WebSocket Handler
//!
//! Combines `ConnectionManager` and `PubSubManager` for a complete
//! real-time communication solution with query streaming support.

use crate::websocket::{
    manager::{ConnectionError, ConnectionManager},
    pubsub::{ChannelId, PubSubManager},
    streaming::{
        QueryStreamId, QueryStreamer, StreamingConfig, StreamingMessage, StreamingRegistry,
    },
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

    /// Execute a streaming query
    StreamQuery {
        query: String,
        batch_size: Option<usize>,
    },

    /// Cancel a streaming query
    CancelQuery { stream_id: String },

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

    /// Streaming query started
    QueryStarted {
        stream_id: String,
        query: String,
        estimated_rows: Option<u64>,
    },

    /// Query progress update
    QueryProgress {
        stream_id: String,
        rows_processed: u64,
        percentage: Option<f32>,
        throughput: f32,
    },

    /// Query result batch
    QueryBatch {
        stream_id: String,
        batch_number: u64,
        rows: Vec<serde_json::Value>,
        has_more: bool,
    },

    /// Query completed
    QueryCompleted {
        stream_id: String,
        total_rows: u64,
        execution_time_ms: u64,
    },

    /// Query cancelled
    QueryCancelled { stream_id: String, reason: String },

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
    streaming_registry: Arc<StreamingRegistry>,
    query_streamer: Arc<QueryStreamer>,
    qsql_engine: Option<Arc<tokio::sync::Mutex<neuroquantum_qsql::QSQLEngine>>>,
}

impl WebSocketService {
    /// Create a new WebSocket service
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        pubsub_manager: Arc<PubSubManager>,
    ) -> Self {
        let streaming_config = StreamingConfig::default();
        let streaming_registry = Arc::new(StreamingRegistry::new(streaming_config.clone()));
        let query_streamer = Arc::new(QueryStreamer::new(
            streaming_config,
            streaming_registry.clone(),
        ));

        info!("✅ WebSocketService initialized with streaming support");
        Self {
            connection_manager,
            pubsub_manager,
            streaming_registry,
            query_streamer,
            qsql_engine: None,
        }
    }

    /// Set the QSQL engine for real query execution
    pub fn set_qsql_engine(
        &mut self,
        engine: Arc<tokio::sync::Mutex<neuroquantum_qsql::QSQLEngine>>,
    ) {
        self.qsql_engine = Some(engine);
        info!("✅ QSQL engine attached to WebSocketService");
    }

    /// Create a new WebSocket service with QSQL engine
    pub fn with_qsql_engine(
        connection_manager: Arc<ConnectionManager>,
        pubsub_manager: Arc<PubSubManager>,
        qsql_engine: Arc<tokio::sync::Mutex<neuroquantum_qsql::QSQLEngine>>,
    ) -> Self {
        let streaming_config = StreamingConfig::default();
        let streaming_registry = Arc::new(StreamingRegistry::new(streaming_config.clone()));
        let query_streamer = Arc::new(QueryStreamer::new(
            streaming_config,
            streaming_registry.clone(),
        ));

        info!("✅ WebSocketService initialized with QSQL engine support");
        Self {
            connection_manager,
            pubsub_manager,
            streaming_registry,
            query_streamer,
            qsql_engine: Some(qsql_engine),
        }
    }

    /// Create a new WebSocket service with custom streaming config
    pub fn with_streaming_config(
        connection_manager: Arc<ConnectionManager>,
        pubsub_manager: Arc<PubSubManager>,
        streaming_config: StreamingConfig,
    ) -> Self {
        let streaming_registry = Arc::new(StreamingRegistry::new(streaming_config.clone()));
        let query_streamer = Arc::new(QueryStreamer::new(
            streaming_config,
            streaming_registry.clone(),
        ));

        info!("✅ WebSocketService initialized with custom streaming config");
        Self {
            connection_manager,
            pubsub_manager,
            streaming_registry,
            query_streamer,
            qsql_engine: None,
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
            | Message::Text(text) => {
                debug!("📨 Received text message from {}: {}", conn_id, text);
                self.handle_text_message(conn_id, &text).await?;
            },
            | Message::Binary(data) => {
                debug!(
                    "📦 Received binary message from {} ({} bytes)",
                    conn_id,
                    data.len()
                );
                // Binary messages could be handled here
            },
            | Message::Ping(data) => {
                debug!("🏓 Received ping from {}", conn_id);
                if let Some(connection) = self.connection_manager.get_connection(conn_id) {
                    connection.session.write().await.pong(&data).await?;
                }
            },
            | Message::Pong(_) => {
                debug!("🏓 Received pong from {}", conn_id);
                self.connection_manager
                    .handle_heartbeat_response(conn_id)
                    .await?;
            },
            | Message::Close(reason) => {
                info!("👋 Connection {} closing: {:?}", conn_id, reason);
                return Err("Connection closed".into());
            },
            | _ => {
                warn!("⚠️  Unsupported message type from {}", conn_id);
            },
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

        // Record incoming WebSocket message
        let msg_type = match &msg {
            | WsMessage::Subscribe { .. } => "subscribe",
            | WsMessage::Unsubscribe { .. } => "unsubscribe",
            | WsMessage::Publish { .. } => "publish",
            | WsMessage::StreamQuery { .. } => "stream_query",
            | WsMessage::CancelQuery { .. } => "cancel_query",
            | WsMessage::Ping { .. } => "ping",
            | WsMessage::Pong { .. } => "pong",
            | WsMessage::QueryStatus { .. } => "query_status",
            | WsMessage::Message { .. } => "message",
        };
        crate::metrics::record_websocket_message("received", msg_type);

        match msg {
            | WsMessage::Subscribe { channel } => {
                self.handle_subscribe(conn_id, channel).await?;
            },
            | WsMessage::Unsubscribe { channel } => {
                self.handle_unsubscribe(conn_id, channel).await?;
            },
            | WsMessage::Publish { channel, data } => {
                self.handle_publish(conn_id, channel, data).await?;
            },
            | WsMessage::StreamQuery { query, batch_size } => {
                self.handle_stream_query(conn_id, query, batch_size).await?;
            },
            | WsMessage::CancelQuery { stream_id } => {
                self.handle_cancel_query(conn_id, stream_id).await?;
            },
            | WsMessage::Ping { timestamp } => {
                self.handle_ping(conn_id, timestamp).await?;
            },
            | WsMessage::Pong { timestamp: _ } => {
                self.connection_manager
                    .handle_heartbeat_response(conn_id)
                    .await?;
            },
            | WsMessage::QueryStatus { query_id } => {
                self.handle_query_status(conn_id, query_id).await?;
            },
            | WsMessage::Message { data } => {
                debug!("💬 Received generic message from {}: {:?}", conn_id, data);
            },
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

    /// Handle streaming query request
    async fn handle_stream_query(
        &self,
        conn_id: ConnectionId,
        query: String,
        _batch_size: Option<usize>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Register the stream
        let stream_id = self
            .streaming_registry
            .register_stream(conn_id, query.clone())
            .await;

        info!(
            "🔍 Starting streaming query {} for connection {}",
            stream_id, conn_id
        );

        // Execute real query if QSQL engine is available, otherwise use mock data
        let query_results = if let Some(qsql_engine) = &self.qsql_engine {
            match qsql_engine.lock().await.execute_query(&query).await {
                | Ok(result) => {
                    // Convert QueryResult rows to Row format
                    result
                        .rows
                        .into_iter()
                        .map(|row_map| {
                            use neuroquantum_core::storage::{Row, Value};
                            use neuroquantum_qsql::query_plan::QueryValue;

                            let mut fields = std::collections::HashMap::new();
                            let mut id_value = 0u64;

                            for (key, value) in row_map {
                                // Extract id if present
                                if key == "id" {
                                    if let QueryValue::Integer(i) = value {
                                        id_value = i as u64;
                                    }
                                }

                                // Convert QueryValue to Value
                                let converted_value = match value {
                                    | QueryValue::Null => Value::Null,
                                    | QueryValue::Boolean(b) => Value::Boolean(b),
                                    | QueryValue::Integer(i) => Value::Integer(i),
                                    | QueryValue::Float(f) => Value::Float(f),
                                    | QueryValue::String(s) => Value::Text(s),
                                    | QueryValue::Blob(b) => Value::Binary(b),
                                    | QueryValue::DNASequence(s) => Value::Text(s),
                                    | QueryValue::SynapticWeight(w) => Value::Float(f64::from(w)),
                                    | QueryValue::QuantumState(s) => Value::Text(s),
                                };
                                fields.insert(key, converted_value);
                            }

                            Row {
                                id: id_value,
                                fields,
                                created_at: chrono::Utc::now(),
                                updated_at: chrono::Utc::now(),
                            }
                        })
                        .collect()
                },
                | Err(e) => {
                    error!("❌ Query execution failed: {}", e);
                    // Send error to client
                    let error_response = WsResponse::Error {
                        code: "QUERY_FAILED".to_string(),
                        message: format!("Query execution failed: {e}"),
                    };
                    self.send_to_connection(conn_id, &error_response).await?;
                    self.streaming_registry.remove_stream(stream_id).await;
                    return Ok(());
                },
            }
        } else {
            // Fallback to mock data if no QSQL engine available
            warn!("⚠️  No QSQL engine available, using mock data");
            self.query_streamer.create_mock_results(500)
        };

        // Clone required variables for async task
        let conn_id_clone = conn_id;
        let stream_id_clone = stream_id;
        let query_clone = query.clone();
        let connection_manager = self.connection_manager.clone();
        let query_streamer = self.query_streamer.clone();
        let streaming_registry = self.streaming_registry.clone();

        // Spawn streaming task
        tokio::spawn(async move {
            // Define send function that writes to WebSocket
            let send_fn = |msg: StreamingMessage| -> Result<(), String> {
                let response = match msg {
                    | StreamingMessage::Started {
                        stream_id,
                        query,
                        estimated_rows,
                    } => WsResponse::QueryStarted {
                        stream_id: stream_id.to_string(),
                        query,
                        estimated_rows,
                    },
                    | StreamingMessage::Progress {
                        stream_id,
                        progress,
                    } => WsResponse::QueryProgress {
                        stream_id: stream_id.to_string(),
                        rows_processed: progress.rows_processed,
                        percentage: progress.percentage,
                        throughput: progress.throughput,
                    },
                    | StreamingMessage::Batch {
                        stream_id,
                        batch_number,
                        rows,
                        has_more,
                    } => {
                        // Convert rows to JSON
                        let json_rows: Vec<serde_json::Value> = rows
                            .iter()
                            .map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null))
                            .collect();

                        WsResponse::QueryBatch {
                            stream_id: stream_id.to_string(),
                            batch_number,
                            rows: json_rows,
                            has_more,
                        }
                    },
                    | StreamingMessage::Completed {
                        stream_id,
                        total_rows,
                        execution_time_ms,
                    } => WsResponse::QueryCompleted {
                        stream_id: stream_id.to_string(),
                        total_rows,
                        execution_time_ms,
                    },
                    | StreamingMessage::Cancelled { stream_id, reason } => {
                        WsResponse::QueryCancelled {
                            stream_id: stream_id.to_string(),
                            reason,
                        }
                    },
                    | StreamingMessage::Error { stream_id, error } => WsResponse::Error {
                        code: "STREAM_ERROR".to_string(),
                        message: format!("Stream {stream_id}: {error}"),
                    },
                };

                // Send to connection
                if let Some(connection) = connection_manager.get_connection(conn_id_clone) {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(async { connection.send_json(&response).await })
                    })
                    .map_err(|e| format!("Failed to send: {e}"))?;
                }

                Ok(())
            };

            // Stream the results (using real query results instead of mock data)
            match query_streamer
                .stream_results(stream_id_clone, query_clone, query_results, send_fn)
                .await
            {
                | Ok(total) => {
                    info!(
                        "✅ Completed streaming query {}: {} rows",
                        stream_id_clone, total
                    );
                },
                | Err(e) => {
                    error!("❌ Streaming query {} failed: {}", stream_id_clone, e);
                },
            }

            // Clean up
            streaming_registry.remove_stream(stream_id_clone).await;
        });

        Ok(())
    }

    /// Handle query cancellation request
    async fn handle_cancel_query(
        &self,
        conn_id: ConnectionId,
        stream_id_str: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Parse stream ID
        let stream_id = match uuid::Uuid::parse_str(&stream_id_str) {
            | Ok(uuid) => QueryStreamId::from(uuid),
            | Err(_) => {
                let response = WsResponse::Error {
                    code: "INVALID_STREAM_ID".to_string(),
                    message: format!("Invalid stream ID: {stream_id_str}"),
                };
                self.send_to_connection(conn_id, &response).await?;
                return Ok(());
            },
        };

        // Cancel the stream
        match self.streaming_registry.cancel_stream(stream_id).await {
            | Ok(()) => {
                info!(
                    "🛑 Cancelled query stream {} for connection {}",
                    stream_id, conn_id
                );
                let response = WsResponse::QueryCancelled {
                    stream_id: stream_id_str,
                    reason: "Cancelled by user".to_string(),
                };
                self.send_to_connection(conn_id, &response).await?;
            },
            | Err(e) => {
                warn!("Failed to cancel stream {}: {}", stream_id, e);
                let response = WsResponse::Error {
                    code: "CANCEL_FAILED".to_string(),
                    message: e,
                };
                self.send_to_connection(conn_id, &response).await?;
            },
        }

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

            // Record outgoing WebSocket message
            let msg_type = match response {
                | WsResponse::SubscriptionConfirmed { .. } => "subscription_confirmed",
                | WsResponse::UnsubscriptionConfirmed { .. } => "unsubscription_confirmed",
                | WsResponse::ChannelMessage { .. } => "channel_message",
                | WsResponse::QueryStarted { .. } => "query_started",
                | WsResponse::QueryProgress { .. } => "query_progress",
                | WsResponse::QueryBatch { .. } => "query_batch",
                | WsResponse::QueryCompleted { .. } => "query_completed",
                | WsResponse::QueryCancelled { .. } => "query_cancelled",
                | WsResponse::Pong { .. } => "pong",
                | WsResponse::QueryStatus { .. } => "query_status",
                | WsResponse::Error { .. } => "error",
            };
            crate::metrics::record_websocket_message("sent", msg_type);
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
        let active_streams = self.streaming_registry.active_stream_count().await;

        ServiceStats {
            active_connections: conn_metrics.active_connections,
            total_connections: conn_metrics.total_connections,
            total_messages_sent: conn_metrics.total_messages_sent,
            total_messages_received: conn_metrics.total_messages_received,
            active_channels: pubsub_stats.channel_count,
            total_channel_messages: pubsub_stats.total_messages,
            active_streams,
        }
    }

    /// Get streaming registry for advanced operations
    #[must_use]
    pub fn streaming_registry(&self) -> Arc<StreamingRegistry> {
        self.streaming_registry.clone()
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
    pub active_streams: usize,
}
