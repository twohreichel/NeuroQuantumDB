//! Pub/Sub Channel System for WebSocket
//!
//! Topic-based message routing with:
//! - Channel creation and management
//! - Subscribe/unsubscribe operations
//! - Wildcard subscriptions (e.g., "sensor.*", "events.**")
//! - Message publishing to channels
//! - Channel statistics and monitoring

use crate::websocket::types::ConnectionId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// A channel identifier (topic name)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(String);

impl ChannelId {
    /// Create a new channel ID
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the channel name
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this channel matches a pattern (supports wildcards)
    pub fn matches(&self, pattern: &str) -> bool {
        // Simple wildcard matching:
        // "*" matches single segment
        // "**" matches multiple segments

        if pattern == "**" {
            return true; // Match everything
        }

        let channel_parts: Vec<&str> = self.0.split('.').collect();
        let pattern_parts: Vec<&str> = pattern.split('.').collect();

        Self::matches_recursive(&channel_parts, &pattern_parts)
    }

    fn matches_recursive(channel: &[&str], pattern: &[&str]) -> bool {
        match (channel.first(), pattern.first()) {
            | (None, None) => true,
            | (Some(_), None) => false,
            | (None, Some(&"**")) if pattern.len() == 1 => true,
            | (None, Some(_)) => false,
            | (Some(_ch), Some(&"**")) => {
                // "**" can match zero or more segments
                if pattern.len() == 1 {
                    return true; // Match rest
                }
                // Try matching rest with and without consuming segment
                Self::matches_recursive(&channel[1..], pattern)
                    || Self::matches_recursive(channel, &pattern[1..])
                    || Self::matches_recursive(&channel[1..], &pattern[1..])
            },
            | (Some(_ch), Some(&"*")) => {
                // "*" matches exactly one segment
                Self::matches_recursive(&channel[1..], &pattern[1..])
            },
            | (Some(ch), Some(pat)) if ch == pat => {
                // Exact match
                Self::matches_recursive(&channel[1..], &pattern[1..])
            },
            | _ => false,
        }
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ChannelId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ChannelId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// A subscription to a channel or pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// The connection that subscribed
    pub connection_id: ConnectionId,

    /// The channel or pattern subscribed to
    pub pattern: String,

    /// Timestamp when subscription was created
    pub subscribed_at: chrono::DateTime<chrono::Utc>,
}

/// A channel with its subscribers
struct Channel {
    /// Channel identifier
    id: ChannelId,

    /// Set of connection IDs subscribed to this channel
    subscribers: Arc<RwLock<HashSet<ConnectionId>>>,

    /// Number of messages published to this channel
    message_count: Arc<RwLock<u64>>,

    /// Timestamp when channel was created
    created_at: chrono::DateTime<chrono::Utc>,
}

impl Channel {
    fn new(id: ChannelId) -> Self {
        Self {
            id,
            subscribers: Arc::new(RwLock::new(HashSet::new())),
            message_count: Arc::new(RwLock::new(0)),
            created_at: chrono::Utc::now(),
        }
    }

    async fn add_subscriber(&self, conn_id: ConnectionId) -> bool {
        let mut subs = self.subscribers.write().await;
        subs.insert(conn_id)
    }

    async fn remove_subscriber(&self, conn_id: ConnectionId) -> bool {
        let mut subs = self.subscribers.write().await;
        subs.remove(&conn_id)
    }

    async fn get_subscribers(&self) -> Vec<ConnectionId> {
        let subs = self.subscribers.read().await;
        subs.iter().copied().collect()
    }

    async fn subscriber_count(&self) -> usize {
        let subs = self.subscribers.read().await;
        subs.len()
    }

    async fn increment_message_count(&self) {
        let mut count = self.message_count.write().await;
        *count += 1;
    }
}

/// Pub/Sub Channel Manager
///
/// Manages channels and subscriptions for topic-based message routing.
pub struct PubSubManager {
    /// All channels indexed by ChannelId
    channels: Arc<DashMap<ChannelId, Arc<Channel>>>,

    /// Connection subscriptions (conn_id -> set of patterns)
    subscriptions: Arc<DashMap<ConnectionId, HashSet<String>>>,

    /// Total number of messages published
    total_messages: Arc<RwLock<u64>>,
}

impl PubSubManager {
    /// Create a new Pub/Sub manager
    pub fn new() -> Self {
        info!("✅ PubSubManager initialized");
        Self {
            channels: Arc::new(DashMap::new()),
            subscriptions: Arc::new(DashMap::new()),
            total_messages: Arc::new(RwLock::new(0)),
        }
    }

    /// Get or create a channel
    fn get_or_create_channel(&self, channel_id: &ChannelId) -> Arc<Channel> {
        self.channels
            .entry(channel_id.clone())
            .or_insert_with(|| {
                debug!("📺 Creating new channel: {}", channel_id);
                Arc::new(Channel::new(channel_id.clone()))
            })
            .clone()
    }

    /// Subscribe a connection to a channel or pattern
    ///
    /// Supports wildcards:
    /// - `sensor.*` matches `sensor.temp`, `sensor.humidity`
    /// - `events.**` matches `events.user`, `events.user.login`, etc.
    pub async fn subscribe(
        &self,
        conn_id: ConnectionId,
        pattern: impl Into<String>,
    ) -> Result<(), PubSubError> {
        let pattern = pattern.into();

        // Add to connection's subscription list
        self.subscriptions
            .entry(conn_id)
            .or_default()
            .insert(pattern.clone());

        // If it's an exact channel (no wildcards), subscribe directly
        if !pattern.contains('*') {
            let channel_id = ChannelId::new(&pattern);
            let channel = self.get_or_create_channel(&channel_id);
            channel.add_subscriber(conn_id).await;
        }

        info!(
            "📡 Connection {} subscribed to pattern: {}",
            conn_id, pattern
        );

        Ok(())
    }

    /// Unsubscribe a connection from a channel or pattern
    pub async fn unsubscribe(
        &self,
        conn_id: ConnectionId,
        pattern: impl Into<String>,
    ) -> Result<(), PubSubError> {
        let pattern = pattern.into();

        // Remove from connection's subscription list
        if let Some(mut subs) = self.subscriptions.get_mut(&conn_id) {
            subs.remove(&pattern);
        }

        // If it's an exact channel, remove from channel subscribers
        if !pattern.contains('*') {
            let channel_id = ChannelId::new(&pattern);
            if let Some(channel) = self.channels.get(&channel_id) {
                channel.remove_subscriber(conn_id).await;
            }
        }

        info!(
            "📴 Connection {} unsubscribed from pattern: {}",
            conn_id, pattern
        );

        Ok(())
    }

    /// Unsubscribe a connection from all channels
    pub async fn unsubscribe_all(&self, conn_id: ConnectionId) -> Result<(), PubSubError> {
        // Get all patterns this connection was subscribed to
        let patterns: Vec<String> = if let Some(subs) = self.subscriptions.get(&conn_id) {
            subs.iter().cloned().collect()
        } else {
            return Ok(());
        };

        // Unsubscribe from each pattern
        for pattern in patterns {
            self.unsubscribe(conn_id, pattern).await?;
        }

        // Remove from subscriptions map
        self.subscriptions.remove(&conn_id);

        info!("📴 Connection {} unsubscribed from all channels", conn_id);

        Ok(())
    }

    /// Get all subscribers for a specific channel
    ///
    /// This includes both exact subscriptions and wildcard matches.
    pub async fn get_subscribers(&self, channel_id: &ChannelId) -> Vec<ConnectionId> {
        let mut result = HashSet::new();

        // Get exact subscribers
        if let Some(channel) = self.channels.get(channel_id) {
            let exact_subs = channel.get_subscribers().await;
            result.extend(exact_subs);
        }

        // Check wildcard subscriptions
        for entry in self.subscriptions.iter() {
            let conn_id = *entry.key();
            let patterns = entry.value();

            for pattern in patterns.iter() {
                if pattern.contains('*') && channel_id.matches(pattern) {
                    result.insert(conn_id);
                }
            }
        }

        result.into_iter().collect()
    }

    /// Publish a message to a channel
    ///
    /// Returns the list of connection IDs that should receive the message.
    pub async fn publish(
        &self,
        channel_id: &ChannelId,
        _message: &serde_json::Value,
    ) -> Vec<ConnectionId> {
        // Get all subscribers (exact + wildcard)
        let subscribers = self.get_subscribers(channel_id).await;

        // Update channel message count
        if let Some(channel) = self.channels.get(channel_id) {
            channel.increment_message_count().await;
        } else {
            // Create channel if it doesn't exist
            let channel = self.get_or_create_channel(channel_id);
            channel.increment_message_count().await;
        }

        // Update global message count
        let mut total = self.total_messages.write().await;
        *total += 1;

        debug!(
            "📤 Published message to channel {} ({} subscribers)",
            channel_id,
            subscribers.len()
        );

        subscribers
    }

    /// Get subscriptions for a specific connection
    pub fn get_connection_subscriptions(&self, conn_id: ConnectionId) -> Vec<String> {
        self.subscriptions
            .get(&conn_id)
            .map(|subs| subs.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all active channels
    pub fn get_all_channels(&self) -> Vec<ChannelId> {
        self.channels.iter().map(|e| e.key().clone()).collect()
    }

    /// Get channel statistics
    pub async fn get_channel_stats(&self, channel_id: &ChannelId) -> Option<ChannelStats> {
        if let Some(channel) = self.channels.get(channel_id) {
            Some(ChannelStats {
                channel_id: channel.id.clone(),
                subscriber_count: channel.subscriber_count().await,
                message_count: *channel.message_count.read().await,
                created_at: channel.created_at,
            })
        } else {
            None
        }
    }

    /// Get statistics for all channels
    pub async fn get_all_stats(&self) -> PubSubStats {
        let channel_count = self.channels.len();
        let total_subscribers: usize = {
            let mut total = 0;
            for entry in self.channels.iter() {
                total += entry.value().subscriber_count().await;
            }
            total
        };

        PubSubStats {
            channel_count,
            total_subscribers,
            total_messages: *self.total_messages.read().await,
            active_connections: self.subscriptions.len(),
        }
    }
}

impl Default for PubSubManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for a specific channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub channel_id: ChannelId,
    pub subscriber_count: usize,
    pub message_count: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Overall Pub/Sub statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubStats {
    pub channel_count: usize,
    pub total_subscribers: usize,
    pub total_messages: u64,
    pub active_connections: usize,
}

/// Pub/Sub related errors
#[derive(Debug, thiserror::Error)]
pub enum PubSubError {
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("Already subscribed to: {0}")]
    AlreadySubscribed(String),

    #[error("Not subscribed to: {0}")]
    NotSubscribed(String),

    #[error("Invalid channel name: {0}")]
    InvalidChannelName(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_id_exact_match() {
        let channel = ChannelId::new("sensor.temperature");
        assert!(channel.matches("sensor.temperature"));
        assert!(!channel.matches("sensor.humidity"));
    }

    #[test]
    fn test_channel_id_single_wildcard() {
        let channel = ChannelId::new("sensor.temperature");
        assert!(channel.matches("sensor.*"));
        assert!(channel.matches("*.temperature"));
        assert!(!channel.matches("events.*"));
    }

    #[test]
    fn test_channel_id_multi_wildcard() {
        let channel = ChannelId::new("events.user.login");
        assert!(channel.matches("**"));
        assert!(channel.matches("events.**"));
        assert!(channel.matches("**.login"));
        assert!(!channel.matches("sensor.**"));
    }

    #[test]
    fn test_channel_id_display() {
        let channel = ChannelId::new("test.channel");
        assert_eq!(format!("{}", channel), "test.channel");
    }

    #[tokio::test]
    async fn test_pubsub_manager_creation() {
        let manager = PubSubManager::new();
        let stats = manager.get_all_stats().await;
        assert_eq!(stats.channel_count, 0);
        assert_eq!(stats.total_messages, 0);
    }
}
