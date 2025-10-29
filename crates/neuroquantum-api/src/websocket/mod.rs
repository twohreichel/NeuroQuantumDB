//! WebSocket Real-Time Communication Module
//!
//! This module provides enterprise-grade WebSocket support for real-time
//! query updates, notifications, and bidirectional communication.
//!
//! # Architecture
//!
//! - **Connection Manager**: Handles client lifecycle (register, unregister, heartbeat)
//! - **Pub/Sub Channels**: Topic-based message broadcasting
//! - **Query Streaming**: Incremental result delivery with backpressure
//! - **Flow Control**: Automatic rate limiting and buffer management
//!
//! # Example
//!
//! ```rust,no_run
//! use neuroquantum_api::websocket::{ConnectionManager, ConnectionConfig, ConnectionMetadata};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = ConnectionConfig::default();
//!     let manager = ConnectionManager::new(config);
//!     
//!     // Example: Register a new connection (in real use, session comes from actix-ws)
//!     // let metadata = ConnectionMetadata::new("127.0.0.1:8080".to_string());
//!     // let conn_id = manager.register(session, metadata).await.unwrap();
//!
//!     // Broadcast a message to all connected clients
//!     manager.broadcast("Hello, all clients!").await;
//! }
//! ```

pub mod handler;
pub mod manager;
pub mod metrics;
pub mod pubsub;
pub mod types;

#[cfg(test)]
mod tests;

pub use handler::{ServiceStats, WebSocketService, WsMessage, WsResponse};
pub use manager::{ConnectionConfig, ConnectionManager};
pub use metrics::ConnectionMetrics;
pub use pubsub::{ChannelId, ChannelStats, PubSubManager, PubSubStats};
pub use types::{Connection, ConnectionId, ConnectionMetadata, ConnectionStatus};
