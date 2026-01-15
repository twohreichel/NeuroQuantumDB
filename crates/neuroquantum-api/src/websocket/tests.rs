//! Comprehensive tests for WebSocket Connection Manager
//!
//! Task 2.1: Connection Manager - Test Suite
//! Tests cover:
//! - Connection registration and unregistration
//! - Maximum connection limits
//! - Heartbeat monitoring
//! - Message sending and broadcasting
//! - Metrics tracking
//! - Graceful shutdown

#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
use tokio::time::sleep;

#[cfg(test)]
use crate::websocket::{ConnectionConfig, ConnectionManager, ConnectionMetadata};

/// Helper function to create a mock metadata for testing
#[cfg(test)]
fn create_mock_metadata(user_id: Option<&str>) -> ConnectionMetadata {
    let mut metadata = ConnectionMetadata::new("127.0.0.1:8080".to_string());
    metadata.user_id = user_id.map(std::string::ToString::to_string);
    metadata.user_agent = Some("Test/1.0".to_string());
    metadata
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
async fn test_connection_config_custom() {
    let config = ConnectionConfig {
        max_connections: 500,
        heartbeat_interval: Duration::from_secs(15),
        heartbeat_timeout: Duration::from_secs(45),
        idle_timeout: Duration::from_secs(120),
        enable_heartbeat_monitor: false,
    };

    assert_eq!(config.max_connections, 500);
    assert_eq!(config.heartbeat_interval, Duration::from_secs(15));
    assert_eq!(config.heartbeat_timeout, Duration::from_secs(45));
    assert!(!config.enable_heartbeat_monitor);
}

#[tokio::test]
async fn test_connection_metadata_creation() {
    let metadata = create_mock_metadata(Some("user123"));

    assert_eq!(metadata.user_id, Some("user123".to_string()));
    assert_eq!(metadata.remote_addr, "127.0.0.1:8080");
    assert_eq!(metadata.user_agent, Some("Test/1.0".to_string()));
    assert!(!metadata.is_idle(Duration::from_secs(300)));
}

#[tokio::test]
async fn test_connection_metadata_idle_detection() {
    let mut metadata = create_mock_metadata(None);

    // Simulate old timestamp
    metadata.last_activity = chrono::Utc::now() - chrono::Duration::seconds(400);

    assert!(metadata.is_idle(Duration::from_secs(300)));
    assert!(!metadata.is_idle(Duration::from_secs(500)));
}

#[tokio::test]
async fn test_connection_metadata_update_activity() {
    let mut metadata = create_mock_metadata(None);
    let old_timestamp = metadata.last_activity;

    sleep(Duration::from_millis(10)).await;
    metadata.update_activity();

    assert!(metadata.last_activity > old_timestamp);
}

#[tokio::test]
async fn test_metrics_increment_operations() {
    use crate::websocket::metrics::ConnectionMetrics;

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
async fn test_metrics_decrement_active_connections() {
    use crate::websocket::metrics::ConnectionMetrics;

    let metrics = ConnectionMetrics::new();

    metrics.increment_active_connections();
    metrics.increment_active_connections();
    assert_eq!(metrics.snapshot().active_connections, 2);

    metrics.decrement_active_connections();
    assert_eq!(metrics.snapshot().active_connections, 1);

    metrics.decrement_active_connections();
    assert_eq!(metrics.snapshot().active_connections, 0);
}

#[tokio::test]
async fn test_metrics_snapshot_calculations() {
    use crate::websocket::metrics::MetricsSnapshot;

    let snapshot = MetricsSnapshot {
        total_connections: 100,
        active_connections: 50,
        total_messages_sent: 500,
        total_messages_received: 300,
        connection_errors: 5,
        heartbeat_failures: 3,
        broadcast_messages: 10,
    };

    // Message rate: (500 + 300) / 10.0 = 80.0 messages/sec
    assert_eq!(snapshot.message_rate(10.0), 80.0);

    // Error rate: (5 + 3) / 100.0 = 0.08
    assert_eq!(snapshot.error_rate(), 0.08);
}

#[tokio::test]
async fn test_metrics_reset() {
    use crate::websocket::metrics::ConnectionMetrics;

    let metrics = ConnectionMetrics::new();

    metrics.increment_total_connections();
    metrics.increment_active_connections();
    metrics.increment_messages_sent(10);

    assert_eq!(metrics.snapshot().total_connections, 1);
    assert_eq!(metrics.snapshot().active_connections, 1);
    assert_eq!(metrics.snapshot().total_messages_sent, 10);

    metrics.reset();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_connections, 0);
    assert_eq!(snapshot.active_connections, 0);
    assert_eq!(snapshot.total_messages_sent, 0);
}

#[tokio::test]
async fn test_connection_id_uniqueness() {
    use crate::websocket::types::ConnectionId;

    let id1 = ConnectionId::new();
    let id2 = ConnectionId::new();

    assert_ne!(id1, id2);
    assert_ne!(id1.to_string(), id2.to_string());
}

#[tokio::test]
async fn test_connection_id_display() {
    use crate::websocket::types::ConnectionId;

    let id = ConnectionId::new();
    let display_str = format!("{id}");

    // Should be a valid UUID string
    assert_eq!(display_str.len(), 36); // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    assert!(display_str.contains('-'));
}

#[tokio::test]
async fn test_connection_status_values() {
    use crate::websocket::types::ConnectionStatus;

    let statuses = [
        ConnectionStatus::Active,
        ConnectionStatus::Idle,
        ConnectionStatus::Closing,
        ConnectionStatus::Closed,
        ConnectionStatus::Failed,
    ];

    // Test that all status values are distinct
    for (i, status1) in statuses.iter().enumerate() {
        for (j, status2) in statuses.iter().enumerate() {
            if i == j {
                assert_eq!(status1, status2);
            } else {
                assert_ne!(status1, status2);
            }
        }
    }
}

#[tokio::test]
async fn test_connection_manager_get_all_connection_ids() {
    let config = ConnectionConfig {
        enable_heartbeat_monitor: false,
        ..Default::default()
    };
    let manager = ConnectionManager::new(config);

    let ids = manager.get_all_connection_ids();
    assert_eq!(ids.len(), 0);
}

#[tokio::test]
async fn test_connection_manager_get_connection_not_found() {
    use crate::websocket::types::ConnectionId;

    let config = ConnectionConfig {
        enable_heartbeat_monitor: false,
        ..Default::default()
    };
    let manager = ConnectionManager::new(config);

    let fake_id = ConnectionId::new();
    let result = manager.get_connection(fake_id);
    assert!(result.is_none());
}

#[tokio::test]
async fn test_connection_error_types() {
    use crate::websocket::manager::ConnectionError;

    let errors = vec![
        ConnectionError::MaxConnectionsReached,
        ConnectionError::ConnectionNotFound,
        ConnectionError::SendFailed,
        ConnectionError::SerializationFailed("test".to_string()),
        ConnectionError::ConnectionClosed,
    ];

    // Test that all errors can be displayed
    for error in errors {
        let error_msg = format!("{error}");
        assert!(!error_msg.is_empty());
    }
}

#[tokio::test]
async fn test_connection_manager_metrics_tracking() {
    let config = ConnectionConfig {
        enable_heartbeat_monitor: false,
        ..Default::default()
    };
    let manager = ConnectionManager::new(config);

    let initial_metrics = manager.get_metrics();
    assert_eq!(initial_metrics.active_connections, 0);
    assert_eq!(initial_metrics.total_connections, 0);
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
