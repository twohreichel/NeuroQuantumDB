//! Flow Control and Backpressure Management
//!
//! Implements automatic backpressure handling to prevent overwhelming slow clients
//! with query results. Monitors buffer sizes and applies throttling strategies.
//!
//! # Features
//!
//! - **Buffer Monitoring**: Track client receive buffer fill levels
//! - **Automatic Throttling**: Slow down when buffer exceeds threshold
//! - **Drop Policies**: Configurable strategies when buffer is full
//! - **Metrics**: Real-time backpressure statistics
//! - **Fair Scheduling**: Prevent fast clients from starving slow ones
//!
//! # Architecture
//!
//! ```text
//! Query Streamer
//!      │
//!      ├─► FlowController
//!      │        │
//!      │        ├─► Monitor buffer size
//!      │        ├─► Apply throttling
//!      │        └─► Track metrics
//!      │             │
//!      │             ▼
//!      └────────► WebSocket Send
//!                   (with backpressure)
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Flow control configuration
#[derive(Debug, Clone)]
pub struct FlowControlConfig {
    /// Maximum buffer size (in messages)
    pub max_buffer_size: usize,

    /// Threshold to start applying backpressure (0.0-1.0)
    pub backpressure_threshold: f32,

    /// Threshold to pause sending (0.0-1.0)
    pub pause_threshold: f32,

    /// Drop policy when buffer is full
    pub drop_policy: DropPolicy,

    /// Time to wait when paused
    pub pause_duration: Duration,

    /// Enable adaptive throttling
    pub adaptive_throttling: bool,

    /// Minimum delay between batches
    pub min_batch_delay: Duration,

    /// Maximum delay between batches
    pub max_batch_delay: Duration,
}

impl Default for FlowControlConfig {
    fn default() -> Self {
        Self {
            max_buffer_size: 1000,
            backpressure_threshold: 0.7, // Start slowing at 70% full
            pause_threshold: 0.9,        // Pause at 90% full
            drop_policy: DropPolicy::DropOldest,
            pause_duration: Duration::from_millis(50),
            adaptive_throttling: true,
            min_batch_delay: Duration::from_millis(0),
            max_batch_delay: Duration::from_millis(500),
        }
    }
}

/// Policy for dropping messages when buffer is full
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DropPolicy {
    /// Drop oldest messages first (FIFO)
    DropOldest,

    /// Drop newest messages (reject new)
    DropNewest,

    /// Block until space is available
    Block,

    /// Drop all and reset
    DropAll,
}

/// Flow control state for a connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowState {
    /// Normal flow, no backpressure
    Normal,

    /// Backpressure applied (throttling)
    Throttled,

    /// Sending paused
    Paused,

    /// Buffer full, dropping messages
    Dropping,
}

/// Flow control statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowControlStats {
    /// Current buffer size
    pub buffer_size: usize,

    /// Maximum buffer size
    pub max_buffer_size: usize,

    /// Buffer fill percentage (0.0-1.0)
    pub fill_percentage: f32,

    /// Current flow state
    pub flow_state: FlowState,

    /// Total messages sent
    pub messages_sent: u64,

    /// Total messages dropped
    pub messages_dropped: u64,

    /// Total time spent paused (ms)
    pub total_pause_time_ms: u64,

    /// Current throttle delay (ms)
    pub current_throttle_delay_ms: u64,

    /// Number of throttling events
    pub throttle_events: u64,

    /// Number of pause events
    pub pause_events: u64,
}

impl Default for FlowControlStats {
    fn default() -> Self {
        Self {
            buffer_size: 0,
            max_buffer_size: 1000,
            fill_percentage: 0.0,
            flow_state: FlowState::Normal,
            messages_sent: 0,
            messages_dropped: 0,
            total_pause_time_ms: 0,
            current_throttle_delay_ms: 0,
            throttle_events: 0,
            pause_events: 0,
        }
    }
}

/// Flow controller for managing backpressure
pub struct FlowController {
    config: FlowControlConfig,
    stats: Arc<RwLock<FlowControlStats>>,
    last_send: Arc<RwLock<Instant>>,
}

impl FlowController {
    /// Create a new flow controller
    pub fn new(config: FlowControlConfig) -> Self {
        info!(
            "✅ FlowController initialized (max_buffer: {}, threshold: {:.1}%)",
            config.max_buffer_size,
            config.backpressure_threshold * 100.0
        );

        let stats = FlowControlStats {
            max_buffer_size: config.max_buffer_size,
            ..Default::default()
        };

        Self {
            config,
            stats: Arc::new(RwLock::new(stats)),
            last_send: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Check if we can send a message
    pub async fn can_send(&self, buffer_size: usize) -> bool {
        let fill_percentage = buffer_size as f32 / self.config.max_buffer_size as f32;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.buffer_size = buffer_size;
        stats.fill_percentage = fill_percentage;

        if fill_percentage >= self.config.pause_threshold {
            stats.flow_state = FlowState::Paused;
            false
        } else if fill_percentage >= self.config.backpressure_threshold {
            stats.flow_state = FlowState::Throttled;
            true
        } else {
            stats.flow_state = FlowState::Normal;
            true
        }
    }

    /// Calculate delay before sending next batch
    pub async fn calculate_delay(&self, buffer_size: usize) -> Duration {
        let fill_percentage = buffer_size as f32 / self.config.max_buffer_size as f32;

        if !self.config.adaptive_throttling {
            return self.config.min_batch_delay;
        }

        if fill_percentage < self.config.backpressure_threshold {
            // No backpressure
            self.config.min_batch_delay
        } else if fill_percentage >= self.config.pause_threshold {
            // Paused - wait longer
            let mut stats = self.stats.write().await;
            stats.pause_events += 1;
            self.config.pause_duration
        } else {
            // Throttled - linear interpolation between min and max delay
            let throttle_factor = (fill_percentage - self.config.backpressure_threshold)
                / (self.config.pause_threshold - self.config.backpressure_threshold);

            let delay_ms = throttle_factor.mul_add(
                self.config.max_batch_delay.as_millis() as f32
                    - self.config.min_batch_delay.as_millis() as f32,
                self.config.min_batch_delay.as_millis() as f32,
            );

            let delay = Duration::from_millis(delay_ms as u64);

            let mut stats = self.stats.write().await;
            stats.throttle_events += 1;
            stats.current_throttle_delay_ms = delay_ms as u64;

            debug!(
                "🐌 Throttling: buffer {}% full, delay {}ms",
                fill_percentage * 100.0,
                delay_ms
            );

            delay
        }
    }

    /// Record a sent message
    pub async fn record_sent(&self) {
        let mut stats = self.stats.write().await;
        stats.messages_sent += 1;
        *self.last_send.write().await = Instant::now();
    }

    /// Record a dropped message
    pub async fn record_dropped(&self) {
        let mut stats = self.stats.write().await;
        stats.messages_dropped += 1;

        if stats.messages_dropped % 100 == 0 {
            warn!(
                "⚠️  {} messages dropped due to backpressure",
                stats.messages_dropped
            );
        }
    }

    /// Handle buffer full condition
    pub async fn handle_buffer_full(&self, buffer_size: usize) -> FlowAction {
        let mut stats = self.stats.write().await;
        stats.buffer_size = buffer_size;
        stats.flow_state = FlowState::Dropping;

        match self.config.drop_policy {
            | DropPolicy::DropOldest => {
                debug!("📉 Buffer full, dropping oldest messages");
                FlowAction::DropOldest
            },
            | DropPolicy::DropNewest => {
                debug!("📉 Buffer full, dropping newest messages");
                FlowAction::DropNewest
            },
            | DropPolicy::Block => {
                debug!("⏸️  Buffer full, blocking");
                stats.pause_events += 1;
                FlowAction::Block(self.config.pause_duration)
            },
            | DropPolicy::DropAll => {
                warn!("🗑️  Buffer full, dropping all messages");
                FlowAction::DropAll
            },
        }
    }

    /// Wait for appropriate throttle delay
    pub async fn wait_if_needed(&self, buffer_size: usize) {
        let delay = self.calculate_delay(buffer_size).await;

        if delay > Duration::from_millis(0) {
            let pause_start = Instant::now();
            tokio::time::sleep(delay).await;

            let mut stats = self.stats.write().await;
            stats.total_pause_time_ms += pause_start.elapsed().as_millis() as u64;
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> FlowControlStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = FlowControlStats {
            max_buffer_size: stats.max_buffer_size,
            ..Default::default()
        };
        info!("📊 Flow control stats reset");
    }

    /// Check if connection is healthy (not constantly dropping)
    pub async fn is_healthy(&self) -> bool {
        let stats = self.stats.read().await;

        if stats.messages_sent == 0 {
            return true;
        }

        let drop_rate = stats.messages_dropped as f32 / stats.messages_sent as f32;
        drop_rate < 0.1 // Less than 10% drop rate
    }

    /// Get flow control recommendation
    pub async fn get_recommendation(&self, buffer_size: usize) -> FlowRecommendation {
        let fill_percentage = buffer_size as f32 / self.config.max_buffer_size as f32;
        let stats = self.stats.read().await;

        if fill_percentage >= self.config.pause_threshold {
            FlowRecommendation {
                action: "pause".to_string(),
                reason: "Buffer critically full".to_string(),
                buffer_fill: fill_percentage,
                suggested_action: "Wait for client to catch up or increase buffer size".to_string(),
            }
        } else if fill_percentage >= self.config.backpressure_threshold {
            FlowRecommendation {
                action: "throttle".to_string(),
                reason: "Buffer filling up".to_string(),
                buffer_fill: fill_percentage,
                suggested_action: "Reduce send rate".to_string(),
            }
        } else if stats.messages_dropped > 100 {
            FlowRecommendation {
                action: "warning".to_string(),
                reason: format!("{} messages dropped", stats.messages_dropped),
                buffer_fill: fill_percentage,
                suggested_action: "Client may be too slow, consider disconnecting".to_string(),
            }
        } else {
            FlowRecommendation {
                action: "ok".to_string(),
                reason: "Normal flow".to_string(),
                buffer_fill: fill_percentage,
                suggested_action: "Continue".to_string(),
            }
        }
    }
}

/// Action to take based on flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowAction {
    /// Continue sending normally
    Continue,

    /// Drop oldest messages
    DropOldest,

    /// Drop newest messages
    DropNewest,

    /// Drop all messages in buffer
    DropAll,

    /// Block for specified duration
    Block(Duration),
}

/// Flow control recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRecommendation {
    pub action: String,
    pub reason: String,
    pub buffer_fill: f32,
    pub suggested_action: String,
}

/// Buffered sender with flow control
pub struct FlowControlledSender<T> {
    controller: Arc<FlowController>,
    buffer: Arc<RwLock<Vec<T>>>,
}

impl<T: Clone> FlowControlledSender<T> {
    /// Create a new flow-controlled sender
    #[must_use]
    pub fn new(config: FlowControlConfig) -> Self {
        Self {
            controller: Arc::new(FlowController::new(config)),
            buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Send a message with flow control
    pub async fn send(&self, message: T) -> Result<(), String> {
        let buffer_size = self.buffer.read().await.len();

        // Check if we can send
        if !self.controller.can_send(buffer_size).await {
            // Buffer is paused, wait
            self.controller.wait_if_needed(buffer_size).await;
        }

        // Add to buffer
        let mut buffer = self.buffer.write().await;

        if buffer.len() >= self.controller.config.max_buffer_size {
            // Handle buffer full
            let action = self.controller.handle_buffer_full(buffer.len()).await;

            match action {
                | FlowAction::DropOldest => {
                    if !buffer.is_empty() {
                        buffer.remove(0);
                        self.controller.record_dropped().await;
                    }
                },
                | FlowAction::DropNewest => {
                    self.controller.record_dropped().await;
                    return Err("Message dropped (buffer full)".to_string());
                },
                | FlowAction::DropAll => {
                    let dropped = buffer.len();
                    buffer.clear();
                    for _ in 0..dropped {
                        self.controller.record_dropped().await;
                    }
                },
                | FlowAction::Block(duration) => {
                    drop(buffer); // Release lock while waiting
                    tokio::time::sleep(duration).await;
                    // Retry with Box::pin to avoid infinite recursion
                    return Box::pin(self.send(message)).await;
                },
                | FlowAction::Continue => {},
            }
        }

        buffer.push(message);
        self.controller.record_sent().await;

        Ok(())
    }

    /// Drain messages from buffer
    pub async fn drain(&self, count: usize) -> Vec<T> {
        let mut buffer = self.buffer.write().await;
        let drain_count = count.min(buffer.len());
        buffer.drain(0..drain_count).collect()
    }

    /// Get current buffer size
    pub async fn buffer_size(&self) -> usize {
        self.buffer.read().await.len()
    }

    /// Get flow control statistics
    pub async fn stats(&self) -> FlowControlStats {
        self.controller.get_stats().await
    }

    /// Check if sender is healthy
    pub async fn is_healthy(&self) -> bool {
        self.controller.is_healthy().await
    }
}
