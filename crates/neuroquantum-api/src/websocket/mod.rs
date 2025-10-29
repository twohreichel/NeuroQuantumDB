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
//! use neuroquantum_api::websocket::{ConnectionManager, ConnectionConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = ConnectionConfig::default();
//!     let manager = ConnectionManager::new(config);
//!     
//!     // Register a new connection
//!     let conn_id = manager.register(session, metadata).await;
//!     
//!     // Broadcast a message
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
