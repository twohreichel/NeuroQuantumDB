//! Tests for WebSocket flow control, pubsub, streaming, and connection management
//!
//! These tests validate WebSocket infrastructure components.

use std::time::Duration;

use neuroquantum_api::websocket::{
    ChannelId, ConnectionConfig, ConnectionId, ConnectionManager, ConnectionMetadata,
    ConnectionMetrics, DropPolicy, FlowControlConfig, FlowControlledSender, FlowController,
    FlowState, PubSubManager, QueryStreamId, QueryStreamStatus, QueryStreamer, StreamingConfig,
    StreamingMessage, StreamingRegistry,
};

// =============================================================================
// Flow Control Tests
// =============================================================================

#[tokio::test]
async fn test_flow_controller_normal() {
    let config = FlowControlConfig::default();
    let controller = FlowController::new(config);

    // Small buffer - should allow send
    assert!(controller.can_send(100).await);

    let stats = controller.get_stats().await;
    assert_eq!(stats.flow_state, FlowState::Normal);
}

#[tokio::test]
async fn test_flow_controller_throttled() {
    let config = FlowControlConfig {
        max_buffer_size: 100,
        backpressure_threshold: 0.7,
        ..Default::default()
    };
    let controller = FlowController::new(config);

    // 75% full - should throttle
    assert!(controller.can_send(75).await);

    let stats = controller.get_stats().await;
    assert_eq!(stats.flow_state, FlowState::Throttled);
}

#[tokio::test]
async fn test_flow_controller_paused() {
    let config = FlowControlConfig {
        max_buffer_size: 100,
        pause_threshold: 0.9,
        ..Default::default()
    };
    let controller = FlowController::new(config);

    // 95% full - should pause
    assert!(!controller.can_send(95).await);

    let stats = controller.get_stats().await;
    assert_eq!(stats.flow_state, FlowState::Paused);
}

#[tokio::test]
async fn test_calculate_delay() {
    let config = FlowControlConfig {
        max_buffer_size: 100,
        backpressure_threshold: 0.7,
        pause_threshold: 0.9,
        min_batch_delay: Duration::from_millis(0),
        max_batch_delay: Duration::from_millis(100),
        adaptive_throttling: true,
        ..Default::default()
    };
    let controller = FlowController::new(config);

    // Normal - no delay
    let delay = controller.calculate_delay(50).await;
    assert_eq!(delay, Duration::from_millis(0));

    // Throttled - some delay
    let delay = controller.calculate_delay(80).await;
    assert!(delay > Duration::from_millis(0));
    assert!(delay < Duration::from_millis(100));
}

#[tokio::test]
async fn test_record_metrics() {
    let config = FlowControlConfig::default();
    let controller = FlowController::new(config);

    controller.record_sent().await;
    controller.record_sent().await;
    controller.record_dropped().await;

    let stats = controller.get_stats().await;
    assert_eq!(stats.messages_sent, 2);
    assert_eq!(stats.messages_dropped, 1);
}

#[tokio::test]
async fn test_flow_controlled_sender() {
    let config = FlowControlConfig {
        max_buffer_size: 10,
        ..Default::default()
    };
    let sender = FlowControlledSender::new(config);

    // Send some messages
    for i in 0..5 {
        sender.send(i).await.unwrap();
    }

    assert_eq!(sender.buffer_size().await, 5);

    // Drain messages
    let messages = sender.drain(3).await;
    assert_eq!(messages.len(), 3);
    assert_eq!(sender.buffer_size().await, 2);
}

#[tokio::test]
async fn test_drop_oldest_policy() {
    let config = FlowControlConfig {
        max_buffer_size: 5,
        drop_policy: DropPolicy::DropOldest,
        ..Default::default()
    };
    let sender = FlowControlledSender::new(config);

    // Fill buffer
    for i in 0..5 {
        sender.send(i).await.unwrap();
    }

    // Overflow - should drop oldest
    sender.send(5).await.unwrap();

    let messages = sender.drain(10).await;
    assert_eq!(messages.len(), 5);
    assert_eq!(messages[0], 1); // 0 was dropped
}

#[tokio::test]
async fn test_health_check() {
    let config = FlowControlConfig::default();
    let controller = FlowController::new(config);

    // Initially healthy
    assert!(controller.is_healthy().await);

    // Send and drop messages
    for _ in 0..100 {
        controller.record_sent().await;
    }
    for _ in 0..5 {
        controller.record_dropped().await;
    }

    // Still healthy (5% drop rate)
    assert!(controller.is_healthy().await);

    // Too many drops
    for _ in 0..20 {
        controller.record_dropped().await;
    }

    // Now unhealthy (25% drop rate)
    assert!(!controller.is_healthy().await);
}

// =============================================================================
// PubSub Tests
// =============================================================================

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
    assert_eq!(format!("{channel}"), "test.channel");
}

#[tokio::test]
async fn test_pubsub_manager_creation() {
    let manager = PubSubManager::new();
    let stats = manager.get_all_stats().await;
    assert_eq!(stats.channel_count, 0);
    assert_eq!(stats.total_messages, 0);
}

// =============================================================================
// Streaming Tests
// =============================================================================

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
    let registry = std::sync::Arc::new(StreamingRegistry::new(config.clone()));
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
    let registry = std::sync::Arc::new(StreamingRegistry::new(config.clone()));
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

// =============================================================================
// Connection Manager Tests
// =============================================================================

#[test]
fn test_connection_config_default() {
    let config = ConnectionConfig::default();
    assert_eq!(config.max_connections, 10_000);
    assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
    assert_eq!(config.heartbeat_timeout, Duration::from_secs(90));
    assert!(config.enable_heartbeat_monitor);
}

#[tokio::test]
async fn test_connection_manager_creation() {
    let config = ConnectionConfig::default();
    let manager = ConnectionManager::new(config);

    assert_eq!(manager.connection_count(), 0);

    let metrics = manager.get_metrics();
    assert_eq!(metrics.active_connections, 0);
    assert_eq!(metrics.total_connections, 0);
}

#[tokio::test]
async fn test_connection_metadata_creation() {
    let mut metadata = ConnectionMetadata::new("127.0.0.1:8080".to_string());
    metadata.user_id = Some("user123".to_string());
    metadata.user_agent = Some("Test/1.0".to_string());

    assert_eq!(metadata.user_id, Some("user123".to_string()));
    assert_eq!(metadata.remote_addr, "127.0.0.1:8080");
    assert_eq!(metadata.user_agent, Some("Test/1.0".to_string()));
    assert!(!metadata.is_idle(Duration::from_secs(300)));
}

#[tokio::test]
async fn test_connection_metadata_idle_detection() {
    let mut metadata = ConnectionMetadata::new("127.0.0.1:8080".to_string());

    // Simulate old timestamp
    metadata.last_activity = chrono::Utc::now() - chrono::Duration::seconds(400);

    assert!(metadata.is_idle(Duration::from_secs(300)));
    assert!(!metadata.is_idle(Duration::from_secs(500)));
}

#[tokio::test]
async fn test_connection_metadata_update_activity() {
    let mut metadata = ConnectionMetadata::new("127.0.0.1:8080".to_string());
    let old_timestamp = metadata.last_activity;

    tokio::time::sleep(Duration::from_millis(10)).await;
    metadata.update_activity();

    assert!(metadata.last_activity > old_timestamp);
}

#[tokio::test]
async fn test_metrics_increment_operations() {
    let metrics = ConnectionMetrics::new();

    metrics.increment_total_connections();
    metrics.increment_active_connections();
    metrics.increment_messages_sent(5);
    metrics.increment_messages_received(3);
    metrics.increment_connection_errors();
    metrics.increment_heartbeat_failures();
    metrics.increment_broadcast_messages();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_connections, 1);
    assert_eq!(snapshot.active_connections, 1);
    assert_eq!(snapshot.total_messages_sent, 5);
    assert_eq!(snapshot.total_messages_received, 3);
    assert_eq!(snapshot.connection_errors, 1);
    assert_eq!(snapshot.heartbeat_failures, 1);
    assert_eq!(snapshot.broadcast_messages, 1);
}

#[tokio::test]
async fn test_connection_id_uniqueness() {
    let id1 = ConnectionId::new();
    let id2 = ConnectionId::new();

    assert_ne!(id1, id2);
    assert_ne!(id1.to_string(), id2.to_string());
}

#[tokio::test]
async fn test_connection_config_validation() {
    let config = ConnectionConfig::default();

    // Heartbeat timeout should be greater than interval
    assert!(config.heartbeat_timeout > config.heartbeat_interval);

    // Idle timeout should be reasonable
    assert!(config.idle_timeout > Duration::from_secs(60));

    // Max connections should be positive
    assert!(config.max_connections > 0);
}

#[tokio::test]
async fn test_connection_manager_concurrent_access() {
    use std::sync::Arc;

    let config = ConnectionConfig {
        enable_heartbeat_monitor: false,
        ..Default::default()
    };
    let manager = Arc::new(ConnectionManager::new(config));

    // Spawn multiple tasks that access the manager concurrently
    let mut handles = vec![];
    for _ in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let count = manager_clone.connection_count();
            let _metrics = manager_clone.get_metrics();
            count
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Manager should still be in a valid state
    assert_eq!(manager.connection_count(), 0);
}
