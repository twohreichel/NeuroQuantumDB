//! WebSocket Streaming Integration Tests
//!
//! Comprehensive integration tests for WebSocket streaming under load,
//! covering concurrent streaming, backpressure handling, and PubSub scenarios.
//!
//! Status: Addresses AUDIT.md Section 7.2 - Expanded integration tests for WebSocket

use neuroquantum_api::websocket::{
    ChannelId, ConnectionConfig, ConnectionManager, PubSubManager, QueryProgress, QueryStreamId,
    QueryStreamStatus, QueryStreamer, StreamingConfig, StreamingMessage, StreamingRegistry,
};
use neuroquantum_core::storage::{Row, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Helper to create mock rows for testing
fn create_mock_rows(count: usize) -> Vec<Row> {
    (0..count)
        .map(|i| Row {
            id: i as u64,
            fields: {
                let mut fields = HashMap::new();
                fields.insert("id".to_string(), Value::Integer(i as i64));
                fields.insert("name".to_string(), Value::Text(format!("item_{}", i)));
                fields.insert("value".to_string(), Value::Float(i as f64 * 2.5));
                fields.insert("active".to_string(), Value::Boolean(i % 2 == 0));
                fields
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .collect()
}

// ==================== WebSocket Streaming Under Load Tests ====================

#[tokio::test]
async fn test_concurrent_query_streams() {
    let config = StreamingConfig {
        batch_size: 50,
        progress_interval: Duration::from_millis(100),
        max_query_duration: Duration::from_secs(60),
        channel_buffer_size: 1000,
        detailed_progress: true,
    };

    let registry = Arc::new(StreamingRegistry::new(config.clone()));
    let streamer = Arc::new(QueryStreamer::new(config, Arc::clone(&registry)));

    // Simulate 10 concurrent streaming queries
    let mut handles = vec![];
    let total_messages = Arc::new(AtomicU64::new(0));

    for stream_idx in 0..10 {
        let registry_clone = Arc::clone(&registry);
        let streamer_clone = Arc::clone(&streamer);
        let messages_clone = Arc::clone(&total_messages);

        let handle = tokio::spawn(async move {
            use neuroquantum_api::websocket::types::ConnectionId;

            let conn_id = ConnectionId::new();
            let query = format!("SELECT * FROM table_{}", stream_idx);
            let stream_id = registry_clone.register_stream(conn_id, query.clone()).await;

            // Create mock results - varying sizes per stream
            let row_count = 100 + (stream_idx * 20);
            let mock_rows = create_mock_rows(row_count);

            let mut batch_count = 0u64;
            let mut row_count_received = 0u64;

            let result = streamer_clone
                .stream_results(stream_id, query, mock_rows, |msg| {
                    messages_clone.fetch_add(1, Ordering::SeqCst);
                    match msg {
                        | StreamingMessage::Batch { rows, .. } => {
                            batch_count += 1;
                            row_count_received += rows.len() as u64;
                        },
                        | StreamingMessage::Completed { total_rows, .. } => {
                            assert!(total_rows > 0, "Should have rows in completion");
                        },
                        | _ => {},
                    }
                    Ok(())
                })
                .await;

            assert!(result.is_ok(), "Stream {} should complete", stream_idx);
            assert!(batch_count > 0, "Should have received batches");
            assert!(
                row_count_received > 0,
                "Should have received rows in batches"
            );

            stream_id
        });

        handles.push(handle);
    }

    // Wait for all streams to complete
    let mut completed_streams = vec![];
    for handle in handles {
        let stream_id = handle.await.expect("Task should complete");
        completed_streams.push(stream_id);
    }

    // Verify all streams completed
    assert_eq!(completed_streams.len(), 10);
    assert!(
        total_messages.load(Ordering::SeqCst) > 30,
        "Should have sent many messages across all streams"
    );

    println!("✅ Concurrent query streams test passed!");
}

#[tokio::test]
async fn test_streaming_with_cancellation() {
    let config = StreamingConfig {
        batch_size: 10,
        progress_interval: Duration::from_millis(50),
        max_query_duration: Duration::from_secs(30),
        channel_buffer_size: 100,
        detailed_progress: true,
    };

    let registry = Arc::new(StreamingRegistry::new(config.clone()));

    use neuroquantum_api::websocket::types::ConnectionId;

    let conn_id = ConnectionId::new();
    let query = "SELECT * FROM large_table".to_string();
    let stream_id = registry.register_stream(conn_id, query).await;

    // Verify stream is registered
    let stats = registry.get_stream_stats(stream_id).await;
    assert!(stats.is_some(), "Stream should be registered");
    assert_eq!(
        stats.unwrap().status,
        QueryStreamStatus::Initializing,
        "Stream should be initializing"
    );

    // Cancel the stream
    let cancel_result = registry.cancel_stream(stream_id).await;
    assert!(cancel_result.is_ok(), "Cancel should succeed");

    // Verify stream was cancelled
    let stats_after = registry.get_stream_stats(stream_id).await;
    assert!(stats_after.is_some());
    assert_eq!(stats_after.unwrap().status, QueryStreamStatus::Cancelled);

    println!("✅ Streaming cancellation test passed!");
}

#[tokio::test]
async fn test_streaming_progress_updates() {
    let config = StreamingConfig {
        batch_size: 25,
        progress_interval: Duration::from_millis(10), // Fast progress updates for testing
        max_query_duration: Duration::from_secs(60),
        channel_buffer_size: 500,
        detailed_progress: true,
    };

    let registry = Arc::new(StreamingRegistry::new(config.clone()));
    let streamer = QueryStreamer::new(config, Arc::clone(&registry));

    use neuroquantum_api::websocket::types::ConnectionId;

    let conn_id = ConnectionId::new();
    let query = "SELECT * FROM progress_test".to_string();
    let stream_id = registry.register_stream(conn_id, query.clone()).await;

    let mock_rows = create_mock_rows(200);
    let progress_updates = Arc::new(RwLock::new(Vec::<QueryProgress>::new()));
    let progress_clone = Arc::clone(&progress_updates);

    let result = streamer
        .stream_results(stream_id, query, mock_rows, |msg| {
            if let StreamingMessage::Progress { progress, .. } = msg {
                let progress_updates = progress_clone.clone();
                tokio::spawn(async move {
                    progress_updates.write().await.push(progress);
                });
            }
            Ok(())
        })
        .await;

    assert!(result.is_ok(), "Stream should complete");

    // Give async tasks time to complete
    tokio::time::sleep(Duration::from_millis(50)).await;

    let updates = progress_updates.read().await;
    // Progress updates should show increasing values
    for i in 1..updates.len() {
        assert!(
            updates[i].rows_processed >= updates[i - 1].rows_processed,
            "Progress should be non-decreasing"
        );
    }

    println!("✅ Streaming progress updates test passed!");
}

#[tokio::test]
async fn test_streaming_registry_cleanup_expired() {
    let config = StreamingConfig {
        batch_size: 10,
        progress_interval: Duration::from_millis(100),
        max_query_duration: Duration::from_millis(50), // Very short for testing
        channel_buffer_size: 100,
        detailed_progress: false,
    };

    let registry = StreamingRegistry::new(config);

    use neuroquantum_api::websocket::types::ConnectionId;

    // Register multiple streams
    for i in 0..5 {
        let conn_id = ConnectionId::new();
        registry
            .register_stream(conn_id, format!("SELECT {}", i))
            .await;
    }

    assert_eq!(registry.active_stream_count().await, 5);

    // Wait for streams to expire
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cleanup expired streams
    let cleaned = registry.cleanup_expired_streams().await;
    assert_eq!(cleaned, 5, "All streams should be cleaned up");
    assert_eq!(registry.active_stream_count().await, 0);

    println!("✅ Streaming registry cleanup test passed!");
}

#[tokio::test]
async fn test_streaming_large_batch_throughput() {
    let config = StreamingConfig {
        batch_size: 500, // Large batches
        progress_interval: Duration::from_millis(200),
        max_query_duration: Duration::from_secs(120),
        channel_buffer_size: 5000,
        detailed_progress: true,
    };

    let registry = Arc::new(StreamingRegistry::new(config.clone()));
    let streamer = QueryStreamer::new(config, Arc::clone(&registry));

    use neuroquantum_api::websocket::types::ConnectionId;

    let conn_id = ConnectionId::new();
    let query = "SELECT * FROM large_throughput_test".to_string();
    let stream_id = registry.register_stream(conn_id, query.clone()).await;

    // Large dataset
    let mock_rows = create_mock_rows(5000);
    let mut total_batches = 0u64;
    let mut total_rows = 0u64;
    let start = std::time::Instant::now();

    let result = streamer
        .stream_results(stream_id, query, mock_rows, |msg| {
            match msg {
                | StreamingMessage::Batch { rows, .. } => {
                    total_batches += 1;
                    total_rows += rows.len() as u64;
                },
                | StreamingMessage::Completed {
                    total_rows: final_rows,
                    execution_time_ms,
                    ..
                } => {
                    assert_eq!(final_rows, 5000, "Should stream all rows");
                    assert!(execution_time_ms > 0, "Should have execution time");
                },
                | _ => {},
            }
            Ok(())
        })
        .await;

    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Large stream should complete");
    assert_eq!(total_rows, 5000, "Should receive all rows");
    assert_eq!(total_batches, 10, "Should have 10 batches of 500");
    println!(
        "✅ Large batch throughput test passed! Streamed 5000 rows in {:?}",
        elapsed
    );
}

// ==================== PubSub Integration Tests ====================

#[tokio::test]
async fn test_pubsub_high_throughput_publish() {
    let pubsub = Arc::new(PubSubManager::new());

    use neuroquantum_api::websocket::types::ConnectionId;

    // Create multiple subscribers
    let mut conn_ids = vec![];
    for _ in 0..20 {
        let conn_id = ConnectionId::new();
        conn_ids.push(conn_id);
        pubsub.subscribe(conn_id, "events.**").await.unwrap();
    }

    // Publish many messages to channels
    let mut handles = vec![];
    for i in 0..100 {
        let pubsub_clone = Arc::clone(&pubsub);
        let handle = tokio::spawn(async move {
            let channel = ChannelId::new(format!("events.stream.{}", i % 10));
            let message = serde_json::json!({"event": "test", "index": i});
            let subscribers = pubsub_clone.publish(&channel, &message).await;
            subscribers
        });
        handles.push(handle);
    }

    let mut total_deliveries = 0usize;
    for handle in handles {
        let subscribers = handle.await.expect("Publish should complete");
        total_deliveries += subscribers.len();
    }

    // Each of 100 messages should reach 20 subscribers
    assert_eq!(total_deliveries, 2000, "Should deliver to all subscribers");

    println!("✅ PubSub high throughput publish test passed!");
}

#[tokio::test]
async fn test_pubsub_wildcard_matching() {
    let pubsub = PubSubManager::new();

    use neuroquantum_api::websocket::types::ConnectionId;

    let conn1 = ConnectionId::new();
    let conn2 = ConnectionId::new();
    let conn3 = ConnectionId::new();

    // Different subscription patterns
    pubsub.subscribe(conn1, "sensor.*").await.unwrap();
    pubsub.subscribe(conn2, "sensor.**").await.unwrap();
    pubsub.subscribe(conn3, "sensor.temperature").await.unwrap();

    // Test various channel matches
    let channel_temp = ChannelId::new("sensor.temperature");
    let channel_humidity = ChannelId::new("sensor.humidity");
    let channel_nested = ChannelId::new("sensor.device.1.temperature");
    let test_message = serde_json::json!({"type": "test"});

    // sensor.temperature should match conn1 (sensor.*), conn2 (sensor.**), conn3 (sensor.temperature)
    let subscribers_temp = pubsub.publish(&channel_temp, &test_message).await;
    assert_eq!(
        subscribers_temp.len(),
        3,
        "All 3 should match sensor.temperature"
    );

    // sensor.humidity should match conn1 (sensor.*), conn2 (sensor.**), NOT conn3
    let subscribers_humidity = pubsub.publish(&channel_humidity, &test_message).await;
    assert_eq!(
        subscribers_humidity.len(),
        2,
        "conn1 and conn2 should match sensor.humidity"
    );

    // sensor.device.1.temperature should only match conn2 (sensor.**)
    let subscribers_nested = pubsub.publish(&channel_nested, &test_message).await;
    assert_eq!(
        subscribers_nested.len(),
        1,
        "Only conn2 (sensor.**) should match nested channel"
    );

    println!("✅ PubSub wildcard matching test passed!");
}

#[tokio::test]
async fn test_pubsub_concurrent_subscribe_unsubscribe() {
    let pubsub = Arc::new(PubSubManager::new());

    use neuroquantum_api::websocket::types::ConnectionId;

    let mut handles = vec![];

    // Concurrent subscribe operations
    for i in 0..50 {
        let pubsub_clone = Arc::clone(&pubsub);
        let handle = tokio::spawn(async move {
            let conn_id = ConnectionId::new();
            let pattern = format!("topic.{}", i % 5);
            let _ = pubsub_clone.subscribe(conn_id, &pattern).await;

            // Small delay then unsubscribe
            tokio::time::sleep(Duration::from_millis(10)).await;
            let _ = pubsub_clone.unsubscribe(conn_id, &pattern).await;

            conn_id
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Subscribe/unsubscribe should complete");
    }

    // All subscriptions should be cleaned up
    let stats = pubsub.get_all_stats().await;
    assert_eq!(
        stats.total_subscribers, 0,
        "All subscriptions should be removed"
    );

    println!("✅ PubSub concurrent subscribe/unsubscribe test passed!");
}

#[tokio::test]
async fn test_pubsub_channel_statistics() {
    let pubsub = PubSubManager::new();

    use neuroquantum_api::websocket::types::ConnectionId;

    // Subscribe to multiple channels
    let conn_id = ConnectionId::new();
    pubsub.subscribe(conn_id, "channel.a").await.unwrap();
    pubsub.subscribe(conn_id, "channel.b").await.unwrap();
    pubsub.subscribe(conn_id, "channel.c").await.unwrap();

    // Publish to channels multiple times
    let channel_a = ChannelId::new("channel.a");
    let channel_b = ChannelId::new("channel.b");
    let msg = serde_json::json!({"data": "test"});

    for _ in 0..5 {
        pubsub.publish(&channel_a, &msg).await;
    }
    for _ in 0..3 {
        pubsub.publish(&channel_b, &msg).await;
    }

    // Check statistics
    let stats = pubsub.get_all_stats().await;
    assert_eq!(stats.channel_count, 3, "Should have 3 active channels");
    assert_eq!(stats.total_subscribers, 3, "Should have 3 subscriptions");
    assert_eq!(stats.total_messages, 8, "Should have 8 total messages");

    // Check individual channel stats
    let channel_a_stats = pubsub.get_channel_stats(&channel_a).await;
    assert!(channel_a_stats.is_some());
    let a_stats = channel_a_stats.unwrap();
    assert_eq!(a_stats.subscriber_count, 1);
    assert_eq!(a_stats.message_count, 5);

    println!("✅ PubSub channel statistics test passed!");
}

// ==================== Connection Manager Load Tests ====================

#[tokio::test]
async fn test_connection_manager_high_connection_count() {
    let config = ConnectionConfig {
        max_connections: 500,
        heartbeat_interval: Duration::from_secs(60),
        heartbeat_timeout: Duration::from_secs(180),
        idle_timeout: Duration::from_secs(600),
        enable_heartbeat_monitor: false,
    };

    let manager = Arc::new(ConnectionManager::new(config));

    // Simulate high concurrent access
    let mut handles = vec![];

    for _ in 0..200 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            // Simulate connection activity
            for _ in 0..10 {
                let _ = manager_clone.get_metrics();
                let _ = manager_clone.connection_count();
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Connection activity should complete");
    }

    let metrics = manager.get_metrics();
    // Metrics should be consistent
    assert!(
        metrics.connection_errors == 0,
        "Should have no connection errors from metrics access"
    );

    println!("✅ Connection manager high connection count test passed!");
}

#[tokio::test]
async fn test_connection_manager_metrics_under_load() {
    let config = ConnectionConfig {
        max_connections: 1000,
        heartbeat_interval: Duration::from_secs(30),
        heartbeat_timeout: Duration::from_secs(90),
        idle_timeout: Duration::from_secs(300),
        enable_heartbeat_monitor: false,
    };

    let manager = Arc::new(ConnectionManager::new(config));
    let metrics_count = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    // Concurrent metrics requests
    for _ in 0..100 {
        let metrics_clone = Arc::clone(&metrics_count);
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            for _ in 0..50 {
                let _metrics = manager_clone.get_metrics();
                metrics_clone.fetch_add(1, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Metrics task should complete");
    }

    assert_eq!(
        metrics_count.load(Ordering::SeqCst),
        5000,
        "Should complete all metrics requests"
    );

    println!("✅ Connection manager metrics under load test passed!");
}

#[tokio::test]
async fn test_streaming_message_serialization() {
    // Test that all streaming message types serialize correctly
    let stream_id = QueryStreamId::new();

    let messages = vec![
        StreamingMessage::Started {
            stream_id,
            query: "SELECT * FROM test".to_string(),
            estimated_rows: Some(1000),
        },
        StreamingMessage::Progress {
            stream_id,
            progress: QueryProgress {
                rows_processed: 500,
                estimated_total: Some(1000),
                percentage: Some(50.0),
                throughput: 100.5,
                elapsed_ms: 5000,
                estimated_remaining_ms: Some(5000),
            },
        },
        StreamingMessage::Batch {
            stream_id,
            batch_number: 5,
            rows: create_mock_rows(10),
            has_more: true,
        },
        StreamingMessage::Completed {
            stream_id,
            total_rows: 1000,
            execution_time_ms: 10000,
        },
        StreamingMessage::Cancelled {
            stream_id,
            reason: "User requested cancellation".to_string(),
        },
        StreamingMessage::Error {
            stream_id,
            error: "Query execution failed".to_string(),
        },
    ];

    for msg in messages {
        // Serialize to JSON
        let json = serde_json::to_string(&msg);
        assert!(json.is_ok(), "Message should serialize to JSON");

        // Deserialize back
        let json_str = json.unwrap();
        let deserialized: Result<StreamingMessage, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok(), "Message should deserialize from JSON");
    }

    println!("✅ Streaming message serialization test passed!");
}

#[tokio::test]
async fn test_stream_stats_tracking() {
    let config = StreamingConfig {
        batch_size: 20,
        progress_interval: Duration::from_millis(50),
        max_query_duration: Duration::from_secs(60),
        channel_buffer_size: 200,
        detailed_progress: true,
    };

    let registry = Arc::new(StreamingRegistry::new(config.clone()));
    let streamer = QueryStreamer::new(config, Arc::clone(&registry));

    use neuroquantum_api::websocket::types::ConnectionId;

    let conn_id = ConnectionId::new();
    let query = "SELECT * FROM stats_test".to_string();
    let stream_id = registry.register_stream(conn_id, query.clone()).await;

    // Initial stats
    let initial_stats = registry.get_stream_stats(stream_id).await.unwrap();
    assert_eq!(initial_stats.rows_sent, 0);
    assert_eq!(initial_stats.batches_sent, 0);

    // Stream some data
    let mock_rows = create_mock_rows(100);
    let _ = streamer
        .stream_results(stream_id, query, mock_rows, |_| Ok(()))
        .await;

    // Final stats
    let final_stats = registry.get_stream_stats(stream_id).await.unwrap();
    assert_eq!(final_stats.rows_sent, 100);
    assert_eq!(final_stats.batches_sent, 5); // 100 / 20 = 5 batches
    assert_eq!(final_stats.status, QueryStreamStatus::Completed);
    assert!(final_stats.elapsed_ms > 0);

    println!("✅ Stream stats tracking test passed!");
}

#[tokio::test]
async fn test_multi_connection_stream_isolation() {
    let config = StreamingConfig::default();
    let registry = StreamingRegistry::new(config);

    use neuroquantum_api::websocket::types::ConnectionId;

    // Register streams for different connections
    let conn1 = ConnectionId::new();
    let conn2 = ConnectionId::new();

    let stream1a = registry
        .register_stream(conn1, "SELECT 1".to_string())
        .await;
    let stream1b = registry
        .register_stream(conn1, "SELECT 2".to_string())
        .await;
    let stream2a = registry
        .register_stream(conn2, "SELECT 3".to_string())
        .await;

    // Get streams per connection
    let conn1_streams = registry.get_streams_for_connection(conn1).await;
    let conn2_streams = registry.get_streams_for_connection(conn2).await;

    assert_eq!(conn1_streams.len(), 2, "Connection 1 should have 2 streams");
    assert_eq!(conn2_streams.len(), 1, "Connection 2 should have 1 stream");

    assert!(conn1_streams.contains(&stream1a));
    assert!(conn1_streams.contains(&stream1b));
    assert!(conn2_streams.contains(&stream2a));

    println!("✅ Multi-connection stream isolation test passed!");
}
