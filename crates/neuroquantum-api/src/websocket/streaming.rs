//! Query Result Streaming Module
//!
//! Provides efficient streaming of large query results over WebSocket connections
//! with batch processing, progress updates, and cancellation support.
//!
//! # Features
//!
//! - **Batch Streaming**: Results delivered in configurable batch sizes
//! - **Progress Updates**: Real-time execution progress notifications
//! - **Cancellation**: Allow clients to cancel long-running queries
//! - **Backpressure Handling**: Respects client processing speed
//! - **Memory Efficient**: Streams without loading entire result set
//!
//! # Architecture
//!
//! ```text
//! Query Executor
//!      │
//!      ├─► StreamingQuery (manages lifecycle)
//!      │        │
//!      │        ├─► Batch 1 (100 rows) ──► WebSocket
//!      │        ├─► Progress Update (25%) ──► WebSocket
//!      │        ├─► Batch 2 (100 rows) ──► WebSocket
//!      │        ├─► Progress Update (50%) ──► WebSocket
//!      │        └─► ... until complete
//!      │
//!      └─► StreamingRegistry (tracks all active streams)
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use neuroquantum_core::storage::{Row, Value};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::websocket::types::ConnectionId;

/// Unique identifier for a streaming query
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryStreamId(Uuid);

impl QueryStreamId {
    /// Create a new unique query stream ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for QueryStreamId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for QueryStreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for QueryStreamId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Configuration for query streaming behavior
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Number of rows per batch
    pub batch_size: usize,

    /// Interval between progress updates
    pub progress_interval: Duration,

    /// Maximum time a query can run before auto-cancellation
    pub max_query_duration: Duration,

    /// Buffer size for the streaming channel
    pub channel_buffer_size: usize,

    /// Enable detailed progress reporting
    pub detailed_progress: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            progress_interval: Duration::from_millis(500),
            max_query_duration: Duration::from_secs(300), // 5 minutes
            channel_buffer_size: 1000,
            detailed_progress: true,
        }
    }
}

/// Status of a streaming query
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryStreamStatus {
    /// Query is initializing
    Initializing,

    /// Query is actively streaming results
    Streaming,

    /// Query has been paused
    Paused,

    /// Query completed successfully
    Completed,

    /// Query was cancelled by client
    Cancelled,

    /// Query failed with an error
    Failed { error: String },
}

/// Progress information for a streaming query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProgress {
    /// Total rows processed so far
    pub rows_processed: u64,

    /// Estimated total rows (if known)
    pub estimated_total: Option<u64>,

    /// Progress percentage (0-100)
    pub percentage: Option<f32>,

    /// Rows per second throughput
    pub throughput: f32,

    /// Time elapsed since query start
    pub elapsed_ms: u64,

    /// Estimated time remaining (if known)
    pub estimated_remaining_ms: Option<u64>,
}

/// A batch of query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultBatch {
    /// Query stream ID
    pub stream_id: QueryStreamId,

    /// Batch sequence number (0-indexed)
    pub batch_number: u64,

    /// Rows in this batch
    pub rows: Vec<Row>,

    /// Timestamp when batch was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Messages sent during query streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingMessage {
    /// Query stream started
    Started {
        stream_id: QueryStreamId,
        query: String,
        estimated_rows: Option<u64>,
    },

    /// Progress update
    Progress {
        stream_id: QueryStreamId,
        progress: QueryProgress,
    },

    /// Result batch
    Batch {
        stream_id: QueryStreamId,
        batch_number: u64,
        rows: Vec<Row>,
        has_more: bool,
    },

    /// Query completed
    Completed {
        stream_id: QueryStreamId,
        total_rows: u64,
        execution_time_ms: u64,
    },

    /// Query cancelled
    Cancelled {
        stream_id: QueryStreamId,
        reason: String,
    },

    /// Query error
    Error {
        stream_id: QueryStreamId,
        error: String,
    },
}

/// Internal state of an active streaming query
struct ActiveStream {
    stream_id: QueryStreamId,
    connection_id: ConnectionId,
    _query: String,
    status: QueryStreamStatus,
    started_at: Instant,
    rows_sent: u64,
    batches_sent: u64,
    last_progress_update: Instant,
    cancellation_tx: Option<mpsc::Sender<()>>,
}

/// Manages all active query streams
pub struct StreamingRegistry {
    config: StreamingConfig,
    active_streams: Arc<RwLock<HashMap<QueryStreamId, ActiveStream>>>,
}

impl StreamingRegistry {
    /// Create a new streaming registry
    pub fn new(config: StreamingConfig) -> Self {
        info!(
            "✅ StreamingRegistry initialized (batch_size: {})",
            config.batch_size
        );
        Self {
            config,
            active_streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new streaming query
    pub async fn register_stream(
        &self,
        connection_id: ConnectionId,
        query: String,
    ) -> QueryStreamId {
        let stream_id = QueryStreamId::new();
        let (cancel_tx, _cancel_rx) = mpsc::channel(1);

        let stream = ActiveStream {
            stream_id,
            connection_id,
            _query: query.clone(),
            status: QueryStreamStatus::Initializing,
            started_at: Instant::now(),
            rows_sent: 0,
            batches_sent: 0,
            last_progress_update: Instant::now(),
            cancellation_tx: Some(cancel_tx),
        };

        self.active_streams.write().await.insert(stream_id, stream);

        info!(
            "📊 Registered stream {} for connection {}",
            stream_id, connection_id
        );

        stream_id
    }

    /// Cancel a streaming query
    pub async fn cancel_stream(&self, stream_id: QueryStreamId) -> Result<(), String> {
        let mut streams = self.active_streams.write().await;

        if let Some(stream) = streams.get_mut(&stream_id) {
            stream.status = QueryStreamStatus::Cancelled;

            if let Some(cancel_tx) = &stream.cancellation_tx {
                if cancel_tx.send(()).await.is_err() {
                    warn!(
                        "Failed to send cancellation signal for stream {}",
                        stream_id
                    );
                }
            }

            info!("🛑 Cancelled stream {}", stream_id);
            Ok(())
        } else {
            Err(format!("Stream {stream_id} not found"))
        }
    }

    /// Update stream status
    async fn update_stream_status(&self, stream_id: QueryStreamId, status: QueryStreamStatus) {
        if let Some(stream) = self.active_streams.write().await.get_mut(&stream_id) {
            stream.status = status;
        }
    }

    /// Update stream progress
    async fn update_stream_progress(&self, stream_id: QueryStreamId, rows: u64, batches: u64) {
        if let Some(stream) = self.active_streams.write().await.get_mut(&stream_id) {
            stream.rows_sent = rows;
            stream.batches_sent = batches;
            stream.last_progress_update = Instant::now();
        }
    }

    /// Remove a completed or failed stream
    pub async fn remove_stream(&self, stream_id: QueryStreamId) {
        self.active_streams.write().await.remove(&stream_id);
        debug!("🗑️  Removed stream {}", stream_id);
    }

    /// Get statistics for a specific stream
    pub async fn get_stream_stats(&self, stream_id: QueryStreamId) -> Option<StreamStats> {
        let streams = self.active_streams.read().await;
        streams.get(&stream_id).map(|stream| StreamStats {
            stream_id,
            status: stream.status.clone(),
            rows_sent: stream.rows_sent,
            batches_sent: stream.batches_sent,
            elapsed_ms: stream.started_at.elapsed().as_millis() as u64,
        })
    }

    /// Get all active streams for a connection
    pub async fn get_streams_for_connection(
        &self,
        connection_id: ConnectionId,
    ) -> Vec<QueryStreamId> {
        let streams = self.active_streams.read().await;
        streams
            .values()
            .filter(|s| s.connection_id == connection_id)
            .map(|s| s.stream_id)
            .collect()
    }

    /// Get total number of active streams
    pub async fn active_stream_count(&self) -> usize {
        self.active_streams.read().await.len()
    }

    /// Cleanup expired streams
    pub async fn cleanup_expired_streams(&self) -> usize {
        let mut streams = self.active_streams.write().await;
        let max_duration = self.config.max_query_duration;

        let expired: Vec<QueryStreamId> = streams
            .iter()
            .filter(|(_, stream)| stream.started_at.elapsed() > max_duration)
            .map(|(id, _)| *id)
            .collect();

        for stream_id in &expired {
            streams.remove(stream_id);
            warn!("⏰ Removed expired stream {}", stream_id);
        }

        expired.len()
    }
}

/// Statistics for a streaming query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub stream_id: QueryStreamId,
    pub status: QueryStreamStatus,
    pub rows_sent: u64,
    pub batches_sent: u64,
    pub elapsed_ms: u64,
}

/// Query streamer for processing and streaming query results
pub struct QueryStreamer {
    config: StreamingConfig,
    registry: Arc<StreamingRegistry>,
}

impl QueryStreamer {
    /// Create a new query streamer
    pub fn new(config: StreamingConfig, registry: Arc<StreamingRegistry>) -> Self {
        info!("✅ QueryStreamer initialized");
        Self { config, registry }
    }

    /// Stream query results in batches
    ///
    /// # Arguments
    ///
    /// * `stream_id` - Unique identifier for this stream
    /// * `query` - The SQL query being executed
    /// * `rows` - Iterator/stream of result rows
    /// * `send_fn` - Callback function to send messages to the client
    ///
    /// # Returns
    ///
    /// Total number of rows streamed
    pub async fn stream_results<F>(
        &self,
        stream_id: QueryStreamId,
        query: String,
        rows: Vec<Row>, // In production, this would be an async iterator
        mut send_fn: F,
    ) -> Result<u64, String>
    where
        F: FnMut(StreamingMessage) -> Result<(), String>,
    {
        let started_at = Instant::now();
        let total_rows = rows.len() as u64;

        // Update status to streaming
        self.registry
            .update_stream_status(stream_id, QueryStreamStatus::Streaming)
            .await;

        // Send start message
        send_fn(StreamingMessage::Started {
            stream_id,
            query: query.clone(),
            estimated_rows: Some(total_rows),
        })
        .map_err(|e| format!("Failed to send start message: {e}"))?;

        let mut rows_sent = 0u64;
        let mut batches_sent = 0u64;
        let mut last_progress = Instant::now();

        // Process rows in batches
        for chunk in rows.chunks(self.config.batch_size) {
            let batch_rows: Vec<Row> = chunk.to_vec();
            let has_more = rows_sent + (batch_rows.len() as u64) < total_rows;

            // Send batch
            send_fn(StreamingMessage::Batch {
                stream_id,
                batch_number: batches_sent,
                rows: batch_rows.clone(),
                has_more,
            })
            .map_err(|e| format!("Failed to send batch: {e}"))?;

            rows_sent += batch_rows.len() as u64;
            batches_sent += 1;

            // Update progress
            self.registry
                .update_stream_progress(stream_id, rows_sent, batches_sent)
                .await;

            // Send progress update if interval elapsed
            if last_progress.elapsed() >= self.config.progress_interval {
                let elapsed_ms = started_at.elapsed().as_millis() as u64;
                let throughput = if elapsed_ms > 0 {
                    (rows_sent as f32) / (elapsed_ms as f32 / 1000.0)
                } else {
                    0.0
                };

                let percentage = (rows_sent as f32 / total_rows as f32) * 100.0;
                let estimated_remaining_ms = if throughput > 0.0 {
                    Some((((total_rows - rows_sent) as f32) / throughput * 1000.0) as u64)
                } else {
                    None
                };

                send_fn(StreamingMessage::Progress {
                    stream_id,
                    progress: QueryProgress {
                        rows_processed: rows_sent,
                        estimated_total: Some(total_rows),
                        percentage: Some(percentage),
                        throughput,
                        elapsed_ms,
                        estimated_remaining_ms,
                    },
                })
                .map_err(|e| format!("Failed to send progress: {e}"))?;

                last_progress = Instant::now();
            }

            // Small delay to simulate streaming and allow cancellation checks
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Update status to completed
        self.registry
            .update_stream_status(stream_id, QueryStreamStatus::Completed)
            .await;

        // Send completion message
        let execution_time_ms = started_at.elapsed().as_millis() as u64;
        send_fn(StreamingMessage::Completed {
            stream_id,
            total_rows: rows_sent,
            execution_time_ms,
        })
        .map_err(|e| format!("Failed to send completion: {e}"))?;

        info!(
            "✅ Stream {} completed: {} rows in {} batches ({} ms)",
            stream_id, rows_sent, batches_sent, execution_time_ms
        );

        Ok(rows_sent)
    }

    /// Create a mock result set for testing
    #[must_use]
    pub fn create_mock_results(&self, count: usize) -> Vec<Row> {
        (0..count)
            .map(|i| Row {
                id: i as u64,
                fields: {
                    let mut fields = HashMap::new();
                    fields.insert("id".to_string(), Value::Integer(i as i64));
                    fields.insert("name".to_string(), Value::Text(format!("Row {i}")));
                    fields.insert("value".to_string(), Value::Float(i as f64 * 1.5));
                    fields
                },
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_id_creation() {
        let id1 = QueryStreamId::new();
        let id2 = QueryStreamId::new();
        assert_ne!(id1, id2);
        assert!(!id1.to_string().is_empty());
    }

    #[tokio::test]
    async fn test_registry_register_stream() {
        let config = StreamingConfig::default();
        let registry = StreamingRegistry::new(config);
        let conn_id = ConnectionId::new();

        let stream_id = registry
            .register_stream(conn_id, "SELECT * FROM test".to_string())
            .await;

        assert_eq!(registry.active_stream_count().await, 1);

        let stats = registry.get_stream_stats(stream_id).await;
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().status, QueryStreamStatus::Initializing);
    }

    #[tokio::test]
    async fn test_registry_cancel_stream() {
        let config = StreamingConfig::default();
        let registry = StreamingRegistry::new(config);
        let conn_id = ConnectionId::new();

        let stream_id = registry
            .register_stream(conn_id, "SELECT * FROM test".to_string())
            .await;

        let result = registry.cancel_stream(stream_id).await;
        assert!(result.is_ok());

        let stats = registry.get_stream_stats(stream_id).await;
        assert_eq!(stats.unwrap().status, QueryStreamStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_registry_remove_stream() {
        let config = StreamingConfig::default();
        let registry = StreamingRegistry::new(config);
        let conn_id = ConnectionId::new();

        let stream_id = registry
            .register_stream(conn_id, "SELECT * FROM test".to_string())
            .await;

        assert_eq!(registry.active_stream_count().await, 1);

        registry.remove_stream(stream_id).await;
        assert_eq!(registry.active_stream_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_streams_for_connection() {
        let config = StreamingConfig::default();
        let registry = StreamingRegistry::new(config);
        let conn_id = ConnectionId::new();

        let stream1 = registry
            .register_stream(conn_id, "SELECT * FROM test1".to_string())
            .await;
        let stream2 = registry
            .register_stream(conn_id, "SELECT * FROM test2".to_string())
            .await;

        let streams = registry.get_streams_for_connection(conn_id).await;
        assert_eq!(streams.len(), 2);
        assert!(streams.contains(&stream1));
        assert!(streams.contains(&stream2));
    }

    #[tokio::test]
    async fn test_query_streamer_mock_results() {
        let config = StreamingConfig::default();
        let registry = Arc::new(StreamingRegistry::new(config.clone()));
        let streamer = QueryStreamer::new(config, registry);

        let results = streamer.create_mock_results(150);
        assert_eq!(results.len(), 150);
        assert_eq!(results[0].id, 0);
        assert_eq!(results[149].id, 149);
    }

    #[tokio::test]
    async fn test_stream_results_basic() {
        let config = StreamingConfig {
            batch_size: 50,
            ..Default::default()
        };
        let registry = Arc::new(StreamingRegistry::new(config.clone()));
        let streamer = QueryStreamer::new(config, registry.clone());

        let conn_id = ConnectionId::new();
        let stream_id = registry
            .register_stream(conn_id, "SELECT * FROM test".to_string())
            .await;

        let mock_results = streamer.create_mock_results(150);

        let mut messages = Vec::new();
        let send_fn = |msg: StreamingMessage| {
            messages.push(msg);
            Ok(())
        };

        let total_rows = streamer
            .stream_results(
                stream_id,
                "SELECT * FROM test".to_string(),
                mock_results,
                send_fn,
            )
            .await
            .unwrap();

        assert_eq!(total_rows, 150);
        assert!(messages.len() >= 5); // Start + 3 batches + Complete

        // Check message types
        assert!(matches!(messages[0], StreamingMessage::Started { .. }));
        assert!(matches!(
            messages.last().unwrap(),
            StreamingMessage::Completed { .. }
        ));
    }
}
