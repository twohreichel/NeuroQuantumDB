//! # Real-Time Updates Demo
//!
//! Demonstrates the complete real-time update system using WebSocket Pub/Sub
//! with topic-based message routing, wildcard subscriptions, and live data streaming.
//!
//! ## Features Demonstrated
//!
//! 1. **Topic-Based Pub/Sub**: Channel creation and message routing
//! 2. **Wildcard Subscriptions**: Pattern matching with `*` and `**`
//! 3. **Real-Time Notifications**: Database change notifications
//! 4. **Multiple Subscribers**: Concurrent connections to same topics
//! 5. **Channel Statistics**: Monitor active channels and throughput
//! 6. **Connection Management**: Subscribe, unsubscribe, and cleanup
//!
//! ## Biological Inspiration
//!
//! The Pub/Sub system is inspired by neural signaling:
//! - **Channels** → Neural Pathways (specific information routes)
//! - **Subscriptions** → Synaptic Connections (selective listening)
//! - **Wildcards** → Dendritic Integration (pattern recognition)
//! - **Publishing** → Action Potentials (signal propagation)
//!
//! ## Run the Demo
//!
//! ```bash
//! cargo run --example real_time_updates
//! ```

use neuroquantum_api::websocket::pubsub::{ChannelId, PubSubManager};
use neuroquantum_api::websocket::types::ConnectionId;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    println!("\n🔔 NeuroQuantumDB - Real-Time Updates Demo\n");
    println!("══════════════════════════════════════════════════════════════\n");

    // Demo 1: Basic Pub/Sub
    demo_basic_pubsub().await?;

    println!("\n──────────────────────────────────────────────────────────────\n");

    // Demo 2: Wildcard Subscriptions
    demo_wildcard_subscriptions().await?;

    println!("\n──────────────────────────────────────────────────────────────\n");

    // Demo 3: Database Change Notifications
    demo_database_notifications().await?;

    println!("\n──────────────────────────────────────────────────────────────\n");

    // Demo 4: Multiple Subscribers
    demo_multiple_subscribers().await?;

    println!("\n──────────────────────────────────────────────────────────────\n");

    // Demo 5: Channel Statistics
    demo_channel_statistics().await?;

    println!("\n──────────────────────────────────────────────────────────────\n");

    // Demo 6: Pattern Matching
    demo_pattern_matching().await?;

    println!("\n──────────────────────────────────────────────────────────────\n");

    // Demo 7: Hierarchical Topics
    demo_hierarchical_topics().await?;

    println!("\n══════════════════════════════════════════════════════════════\n");

    // Summary
    print_summary();

    Ok(())
}

/// Demo 1: Basic Pub/Sub workflow
async fn demo_basic_pubsub() -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 Demo 1: Basic Pub/Sub Workflow\n");

    let manager = Arc::new(PubSubManager::new());

    // Create a connection and subscribe to a channel
    let conn1 = ConnectionId::new();
    let channel = ChannelId::new("notifications.alerts");

    println!("  ✓ Connection {conn1} created");

    manager.subscribe(conn1, channel.as_str()).await?;
    println!("  ✓ Subscribed to channel: {channel}");

    // Publish a message
    let message = json!({
        "type": "alert",
        "severity": "warning",
        "message": "High CPU usage detected",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let subscribers = manager.publish(&channel, &message).await;
    println!(
        "  ✓ Published message to {} subscriber(s)",
        subscribers.len()
    );
    println!("  📨 Message: {}", serde_json::to_string_pretty(&message)?);

    // Verify subscriber
    assert_eq!(subscribers.len(), 1);
    assert_eq!(subscribers[0], conn1);
    println!("  ✓ Message delivered to Connection {conn1}");

    // Unsubscribe
    manager.unsubscribe(conn1, channel.as_str()).await?;
    println!("  ✓ Unsubscribed from channel");

    // Publish again (no subscribers)
    let subscribers = manager.publish(&channel, &message).await;
    println!(
        "  ✓ Published again: {} subscriber(s) (expected 0)",
        subscribers.len()
    );

    Ok(())
}

/// Demo 2: Wildcard Subscriptions with * and **
async fn demo_wildcard_subscriptions() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Demo 2: Wildcard Subscriptions\n");

    let manager = Arc::new(PubSubManager::new());
    let conn = ConnectionId::new();

    // Subscribe to wildcard patterns
    manager.subscribe(conn, "sensor.*").await?;
    println!("  ✓ Subscribed to pattern: sensor.*");

    manager.subscribe(conn, "events.**").await?;
    println!("  ✓ Subscribed to pattern: events.**\n");

    // Publish to channels matching "sensor.*"
    let channels = vec![
        ChannelId::new("sensor.temperature"),
        ChannelId::new("sensor.humidity"),
        ChannelId::new("sensor.pressure"),
    ];

    println!("  📊 Publishing to sensor channels:");
    for channel in &channels {
        let message = json!({
            "channel": channel.as_str(),
            "value": rand::random::<f64>() * 100.0,
            "unit": "various",
        });
        let subs = manager.publish(channel, &message).await;
        println!("    • {} → {} subscriber(s) ✓", channel, subs.len());
    }

    // Publish to channels matching "events.**"
    let event_channels = vec![
        ChannelId::new("events.user.login"),
        ChannelId::new("events.user.logout"),
        ChannelId::new("events.system.startup"),
    ];

    println!("\n  📡 Publishing to event channels:");
    for channel in &event_channels {
        let message = json!({
            "channel": channel.as_str(),
            "action": "logged",
        });
        let subs = manager.publish(channel, &message).await;
        println!("    • {} → {} subscriber(s) ✓", channel, subs.len());
    }

    // Show connection subscriptions
    let subs = manager.get_connection_subscriptions(conn);
    println!("\n  📋 Connection {conn} subscriptions: {subs:?}");

    Ok(())
}

/// Demo 3: Database Change Notifications
async fn demo_database_notifications() -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 Demo 3: Database Change Notifications\n");

    let manager = Arc::new(PubSubManager::new());

    // Simulate 3 clients watching different tables
    let client1 = ConnectionId::new();
    let client2 = ConnectionId::new();
    let client3 = ConnectionId::new();

    // Subscribe to specific tables
    manager.subscribe(client1, "db.users.changes").await?;
    println!("  ✓ Client {client1} watching: db.users.changes");

    manager.subscribe(client2, "db.orders.changes").await?;
    println!("  ✓ Client {client2} watching: db.orders.changes");

    // Client 3 watches all database changes
    manager.subscribe(client3, "db.**.changes").await?;
    println!(
        "  ✓ Client {client3} watching: db.**.changes (all tables)\n"
    );

    // Simulate database operations
    let operations = vec![
        (
            "db.users.changes",
            "INSERT",
            json!({"id": 1, "name": "Alice"}),
        ),
        (
            "db.users.changes",
            "UPDATE",
            json!({"id": 1, "email": "alice@example.com"}),
        ),
        (
            "db.orders.changes",
            "INSERT",
            json!({"id": 101, "user_id": 1, "total": 99.99}),
        ),
        ("db.products.changes", "DELETE", json!({"id": 42})),
    ];

    println!("  📝 Simulating database operations:");
    for (channel_str, operation, data) in operations {
        let channel = ChannelId::new(channel_str);
        let message = json!({
            "operation": operation,
            "table": channel_str.split('.').nth(1).unwrap_or("unknown"),
            "data": data,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let subs = manager.publish(&channel, &message).await;
        println!(
            "    • {} {} → {} subscriber(s)",
            operation,
            channel,
            subs.len()
        );

        sleep(Duration::from_millis(100)).await;
    }

    println!("\n  📊 Notification Summary:");
    println!(
        "    • Client {client1} received notifications for: users table"
    );
    println!(
        "    • Client {client2} received notifications for: orders table"
    );
    println!(
        "    • Client {client3} received notifications for: all tables"
    );

    Ok(())
}

/// Demo 4: Multiple Subscribers to Same Channel
async fn demo_multiple_subscribers() -> Result<(), Box<dyn std::error::Error>> {
    println!("👥 Demo 4: Multiple Subscribers\n");

    let manager = Arc::new(PubSubManager::new());
    let channel = ChannelId::new("notifications.broadcast");

    // Create 5 subscribers
    let subscribers: Vec<ConnectionId> = (0..5).map(|_| ConnectionId::new()).collect();

    println!("  📡 Creating 5 subscribers to channel: {channel}\n");
    for (i, conn) in subscribers.iter().enumerate() {
        manager.subscribe(*conn, channel.as_str()).await?;
        println!("    ✓ Subscriber {} connected (ID: {})", i + 1, conn);
    }

    // Publish a broadcast message
    let message = json!({
        "type": "broadcast",
        "message": "System maintenance scheduled for tonight",
        "priority": "high",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    println!("\n  📤 Publishing broadcast message...");
    let delivered = manager.publish(&channel, &message).await;
    println!("  ✓ Message delivered to {} subscriber(s)", delivered.len());
    assert_eq!(delivered.len(), 5);

    // Simulate one subscriber disconnecting
    manager.unsubscribe_all(subscribers[2]).await?;
    println!("\n  ⚠️  Subscriber 3 disconnected");

    // Publish again
    let message = json!({"type": "update", "message": "Maintenance completed"});
    let delivered = manager.publish(&channel, &message).await;
    println!(
        "  ✓ Second message delivered to {} subscriber(s)",
        delivered.len()
    );
    assert_eq!(delivered.len(), 4);

    Ok(())
}

/// Demo 5: Channel Statistics and Monitoring
async fn demo_channel_statistics() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Demo 5: Channel Statistics\n");

    let manager = Arc::new(PubSubManager::new());

    // Create multiple channels with subscribers
    let channels = vec![
        ("metrics.cpu", 3),
        ("metrics.memory", 2),
        ("metrics.disk", 1),
    ];

    println!("  📡 Setting up channels:");
    for (channel_name, subscriber_count) in &channels {
        let channel = ChannelId::new(*channel_name);

        // Add subscribers
        for _ in 0..*subscriber_count {
            let conn = ConnectionId::new();
            manager.subscribe(conn, channel.as_str()).await?;
        }

        println!(
            "    • {channel_name} → {subscriber_count} subscriber(s)"
        );
    }

    // Publish messages
    println!("\n  📤 Publishing messages:");
    for (channel_name, msg_count) in &[
        ("metrics.cpu", 10),
        ("metrics.memory", 5),
        ("metrics.disk", 2),
    ] {
        let channel = ChannelId::new(*channel_name);
        for i in 0..*msg_count {
            let message = json!({"reading": i, "value": rand::random::<f64>() * 100.0});
            manager.publish(&channel, &message).await;
        }
        println!("    • {channel_name} → {msg_count} message(s) sent");
    }

    // Get overall statistics
    println!("\n  📈 Overall Statistics:");
    let stats = manager.get_all_stats().await;
    println!("    • Total Channels: {}", stats.channel_count);
    println!("    • Total Subscribers: {}", stats.total_subscribers);
    println!("    • Total Messages: {}", stats.total_messages);
    println!("    • Active Connections: {}", stats.active_connections);

    // Get per-channel statistics
    println!("\n  📊 Per-Channel Statistics:");
    for (channel_name, _) in &channels {
        let channel = ChannelId::new(*channel_name);
        if let Some(channel_stats) = manager.get_channel_stats(&channel).await {
            println!("    • {channel_name}:");
            println!("      - Subscribers: {}", channel_stats.subscriber_count);
            println!("      - Messages: {}", channel_stats.message_count);
            println!(
                "      - Created: {}",
                channel_stats.created_at.format("%H:%M:%S")
            );
        }
    }

    Ok(())
}

/// Demo 6: Advanced Pattern Matching
async fn demo_pattern_matching() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Demo 6: Advanced Pattern Matching\n");

    // Test various pattern matching scenarios
    let test_cases = vec![
        (
            "sensor.*",
            vec!["sensor.temp", "sensor.humidity"],
            vec!["sensor.room.temp"],
        ),
        (
            "sensor.**",
            vec!["sensor.temp", "sensor.room.temp", "sensor.room.floor.temp"],
            vec!["metrics.cpu"],
        ),
        (
            "**.error",
            vec!["system.error", "app.module.error"],
            vec!["error.system"],
        ),
        (
            "events.*.login",
            vec!["events.user.login", "events.admin.login"],
            vec!["events.login", "events.user.logout"],
        ),
    ];

    for (pattern, should_match, should_not_match) in test_cases {
        println!("  🔍 Testing pattern: {pattern}");

        println!("    ✓ Should match:");
        for channel_name in &should_match {
            let channel = ChannelId::new(*channel_name);
            assert!(
                channel.matches(pattern),
                "{channel_name} should match {pattern}"
            );
            println!("      • {channel_name} ✓");
        }

        println!("    ✗ Should NOT match:");
        for channel_name in &should_not_match {
            let channel = ChannelId::new(*channel_name);
            assert!(
                !channel.matches(pattern),
                "{channel_name} should NOT match {pattern}"
            );
            println!("      • {channel_name} ✓");
        }

        println!();
    }

    Ok(())
}

/// Demo 7: Hierarchical Topics (Neuromorphic Routing)
async fn demo_hierarchical_topics() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌳 Demo 7: Hierarchical Topics (Neuromorphic Routing)\n");
    println!("  Inspired by hierarchical processing in the visual cortex:\n");

    let manager = Arc::new(PubSubManager::new());

    // Create connections at different hierarchy levels
    let visual_cortex = ConnectionId::new(); // V1 - Primary visual
    let parietal_cortex = ConnectionId::new(); // Spatial processing
    let frontal_cortex = ConnectionId::new(); // Executive function

    // Subscribe to different hierarchy levels
    manager
        .subscribe(visual_cortex, "neural.sensory.vision.**")
        .await?;
    println!("  🧠 V1 (Visual Cortex) listening: neural.sensory.vision.**");

    manager
        .subscribe(parietal_cortex, "neural.association.spatial.**")
        .await?;
    println!("  🧠 Parietal Cortex listening: neural.association.spatial.**");

    manager.subscribe(frontal_cortex, "neural.**").await?;
    println!("  🧠 Frontal Cortex listening: neural.** (all neural activity)\n");

    // Simulate neural signals at different processing levels
    let signals = vec![
        (
            "neural.sensory.vision.edge.detection",
            "V1 processing edges",
        ),
        (
            "neural.sensory.vision.color.processing",
            "V1 processing colors",
        ),
        (
            "neural.association.spatial.location",
            "Parietal processing location",
        ),
        (
            "neural.association.spatial.movement",
            "Parietal tracking movement",
        ),
        ("neural.motor.planning", "Motor cortex planning action"),
    ];

    println!("  ⚡ Simulating neural signal propagation:");
    for (channel_name, description) in signals {
        let channel = ChannelId::new(channel_name);
        let message = json!({
            "signal": description,
            "strength": rand::random::<f64>(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let subs = manager.publish(&channel, &message).await;
        println!("    • {} → {} region(s)", channel_name, subs.len());
        println!(
            "      '{}' delivered to {} area(s)",
            description,
            subs.len()
        );

        sleep(Duration::from_millis(50)).await;
    }

    println!("\n  🧠 Neuromorphic Insight:");
    println!("    • V1 receives only visual signals (specialized processing)");
    println!("    • Parietal processes spatial information (selective attention)");
    println!("    • Frontal monitors all activity (executive control)");
    println!("    • This mimics hierarchical brain organization!");

    Ok(())
}

/// Print summary of the real-time update system
fn print_summary() {
    println!("📊 Real-Time Update System Summary\n");

    println!("  ✓ Topic-Based Pub/Sub: Channel-based message routing");
    println!("  ✓ Wildcard Patterns: Single (*) and multi-level (**) matching");
    println!("  ✓ Database Notifications: Real-time change tracking");
    println!("  ✓ Multiple Subscribers: Broadcast to many connections");
    println!("  ✓ Channel Statistics: Monitor throughput and activity");
    println!("  ✓ Pattern Matching: Flexible subscription patterns");
    println!("  ✓ Hierarchical Topics: Neuromorphic routing inspired by brain");

    println!("\n  🔬 Biological Inspiration:");
    println!("    • Channels → Neural Pathways (dedicated information routes)");
    println!("    • Wildcards → Dendritic Integration (pattern recognition)");
    println!("    • Pub/Sub → Neurotransmission (selective signal propagation)");
    println!("    • Hierarchy → Cortical Layers (hierarchical processing)");

    println!("\n  📡 Use Cases:");
    println!("    • Real-time dashboards and monitoring");
    println!("    • Live database change notifications");
    println!("    • IoT sensor data streaming");
    println!("    • Collaborative editing and presence");
    println!("    • Event-driven architectures");
    println!("    • Microservice communication");

    println!("\n  🚀 Production Features:");
    println!("    • Thread-safe with DashMap and RwLock");
    println!("    • O(1) channel lookup and subscription");
    println!("    • Efficient wildcard matching");
    println!("    • Connection lifecycle management");
    println!("    • Comprehensive statistics and metrics");

    println!("\n✨ Real-Time Updates Demo Complete!\n");
}
