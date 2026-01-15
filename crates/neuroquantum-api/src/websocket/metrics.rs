//! Connection metrics and monitoring
//!
//! Provides comprehensive metrics for WebSocket connections, including
//! connection counts, message rates, and error tracking.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Metrics for WebSocket connections
#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    /// Total number of connections ever established
    pub total_connections: Arc<AtomicU64>,

    /// Current number of active connections
    pub active_connections: Arc<AtomicU64>,

    /// Total messages sent to clients
    pub total_messages_sent: Arc<AtomicU64>,

    /// Total messages received from clients
    pub total_messages_received: Arc<AtomicU64>,

    /// Total number of connection errors
    pub connection_errors: Arc<AtomicU64>,

    /// Total number of heartbeat failures
    pub heartbeat_failures: Arc<AtomicU64>,

    /// Total number of broadcast messages
    pub broadcast_messages: Arc<AtomicU64>,
}

impl ConnectionMetrics {
    /// Create a new metrics instance
    #[must_use] 
    pub fn new() -> Self {
        Self {
            total_connections: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
            total_messages_sent: Arc::new(AtomicU64::new(0)),
            total_messages_received: Arc::new(AtomicU64::new(0)),
            connection_errors: Arc::new(AtomicU64::new(0)),
            heartbeat_failures: Arc::new(AtomicU64::new(0)),
            broadcast_messages: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increment total connections counter
    pub fn increment_total_connections(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment active connections counter
    pub fn increment_active_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections counter
    pub fn decrement_active_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment messages sent counter
    pub fn increment_messages_sent(&self, count: u64) {
        self.total_messages_sent.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment messages received counter
    pub fn increment_messages_received(&self, count: u64) {
        self.total_messages_received
            .fetch_add(count, Ordering::Relaxed);
    }

    /// Increment connection errors counter
    pub fn increment_connection_errors(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment heartbeat failures counter
    pub fn increment_heartbeat_failures(&self) {
        self.heartbeat_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment broadcast messages counter
    pub fn increment_broadcast_messages(&self) {
        self.broadcast_messages.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics
    #[must_use] 
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_connections: self.total_connections.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_messages_sent: self.total_messages_sent.load(Ordering::Relaxed),
            total_messages_received: self.total_messages_received.load(Ordering::Relaxed),
            connection_errors: self.connection_errors.load(Ordering::Relaxed),
            heartbeat_failures: self.heartbeat_failures.load(Ordering::Relaxed),
            broadcast_messages: self.broadcast_messages.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics (useful for testing)
    pub fn reset(&self) {
        self.total_connections.store(0, Ordering::Relaxed);
        self.active_connections.store(0, Ordering::Relaxed);
        self.total_messages_sent.store(0, Ordering::Relaxed);
        self.total_messages_received.store(0, Ordering::Relaxed);
        self.connection_errors.store(0, Ordering::Relaxed);
        self.heartbeat_failures.store(0, Ordering::Relaxed);
        self.broadcast_messages.store(0, Ordering::Relaxed);
    }
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// A snapshot of metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub connection_errors: u64,
    pub heartbeat_failures: u64,
    pub broadcast_messages: u64,
}

impl MetricsSnapshot {
    /// Calculate message rate (messages per second)
    #[must_use] 
    pub fn message_rate(&self, duration_secs: f64) -> f64 {
        if duration_secs > 0.0 {
            (self.total_messages_sent + self.total_messages_received) as f64 / duration_secs
        } else {
            0.0
        }
    }

    /// Calculate error rate (errors per connection)
    #[must_use] 
    pub fn error_rate(&self) -> f64 {
        if self.total_connections > 0 {
            (self.connection_errors + self.heartbeat_failures) as f64
                / self.total_connections as f64
        } else {
            0.0
        }
    }
}
