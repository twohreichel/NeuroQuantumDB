//! # Flow Control & Backpressure Demo
//!
//! Demonstrates automatic backpressure handling to prevent overwhelming slow clients.
//!
//! ## Features Demonstrated
//!
//! 1. **Normal Flow**: Fast client with no backpressure
//! 2. **Throttling**: Client slowing down, adaptive delays applied
//! 3. **Pause**: Buffer critically full, sending paused
//! 4. **Drop Oldest**: Buffer overflow, oldest messages dropped
//! 5. **Health Monitoring**: Detect unhealthy connections
//!
//! ## Run the Demo
//!
//! ```bash
//! cargo run --package neuroquantum-api --example flow_control_demo
//! ```

use neuroquantum_api::websocket::{
    DropPolicy, FlowControlConfig, FlowControlledSender, FlowController,
};
use std::time::Duration;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    println!("\n🚀 NeuroQuantumDB - Flow Control & Backpressure Demo\n");
    println!("=====================================================\n");

    // Demo 1: Normal flow (no backpressure)
    demo_normal_flow().await?;

    // Demo 2: Throttling (backpressure applied)
    demo_throttling().await?;

    // Demo 3: Pause (buffer critically full)
    demo_pause().await?;

    // Demo 4: Drop oldest policy
    demo_drop_oldest().await?;

    // Demo 5: Health monitoring
    demo_health_monitoring().await?;

    // Demo 6: Adaptive throttling
    demo_adaptive_throttling().await?;

    println!("\n✅ All flow control demos completed successfully!\n");

    Ok(())
}

/// Demo 1: Normal flow with fast client
async fn demo_normal_flow() -> Result<(), Box<dyn std::error::Error>> {
    println!("🟢 Demo 1: Normal Flow (No Backpressure)");
    println!("─────────────────────────────────────────\n");

    let config = FlowControlConfig {
        max_buffer_size: 1000,
        backpressure_threshold: 0.7,
        ..Default::default()
    };

    let controller = FlowController::new(config);

    // Simulate sending with low buffer
    for i in 0..10 {
        let buffer_size = i * 10; // 0, 10, 20, ..., 90
        let can_send = controller.can_send(buffer_size).await;
        let delay = controller.calculate_delay(buffer_size).await;

        println!(
            "Message {}: buffer {}%, can_send: {}, delay: {:?}",
            i,
            (buffer_size as f32 / 1000.0 * 100.0),
            can_send,
            delay
        );

        controller.record_sent().await;
    }

    let stats = controller.get_stats().await;
    println!("\n📊 Stats:");
    println!("   Flow state: {:?}", stats.flow_state);
    println!("   Messages sent: {}", stats.messages_sent);
    println!("   Fill: {:.1}%\n", stats.fill_percentage * 100.0);

    Ok(())
}

/// Demo 2: Throttling when buffer fills
async fn demo_throttling() -> Result<(), Box<dyn std::error::Error>> {
    println!("🟡 Demo 2: Throttling (Backpressure Applied)");
    println!("──────────────────────────────────────────────\n");

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

    // Simulate buffer filling up
    for buffer_size in (50..=85).step_by(5) {
        let _can_send = controller.can_send(buffer_size).await;
        let delay = controller.calculate_delay(buffer_size).await;
        let stats = controller.get_stats().await;

        println!(
            "Buffer: {}% | State: {:?} | Delay: {}ms",
            buffer_size,
            stats.flow_state,
            delay.as_millis()
        );

        controller.record_sent().await;
    }

    let stats = controller.get_stats().await;
    println!("\n📊 Final Stats:");
    println!("   Throttle events: {}", stats.throttle_events);
    println!("   Current delay: {}ms\n", stats.current_throttle_delay_ms);

    Ok(())
}

/// Demo 3: Pause when buffer is critically full
async fn demo_pause() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔴 Demo 3: Pause (Buffer Critically Full)");
    println!("───────────────────────────────────────────\n");

    let config = FlowControlConfig {
        max_buffer_size: 100,
        pause_threshold: 0.9,
        pause_duration: Duration::from_millis(50),
        ..Default::default()
    };

    let controller = FlowController::new(config);

    // Simulate critically full buffer
    let buffer_size = 95;

    println!("Buffer: {}% full", buffer_size);

    let can_send = controller.can_send(buffer_size).await;
    println!("Can send: {} (paused!)", can_send);

    if !can_send {
        println!("⏸️  Waiting for buffer to drain...");
        let start = std::time::Instant::now();
        controller.wait_if_needed(buffer_size).await;
        println!("✅ Resumed after {:?}\n", start.elapsed());
    }

    let stats = controller.get_stats().await;
    println!("📊 Stats:");
    println!("   Flow state: {:?}", stats.flow_state);
    println!("   Pause events: {}", stats.pause_events);
    println!("   Total pause time: {}ms\n", stats.total_pause_time_ms);

    Ok(())
}

/// Demo 4: Drop oldest messages when buffer overflows
async fn demo_drop_oldest() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗑️  Demo 4: Drop Oldest Policy");
    println!("───────────────────────────────\n");

    let config = FlowControlConfig {
        max_buffer_size: 10,
        drop_policy: DropPolicy::DropOldest,
        ..Default::default()
    };

    let sender: FlowControlledSender<String> = FlowControlledSender::new(config);

    // Fill buffer
    println!("Sending 10 messages (buffer capacity: 10):");
    for i in 0..10 {
        sender.send(format!("Message {}", i)).await?;
        println!("  Sent: Message {}", i);
    }

    println!("\nBuffer full! Sending 3 more messages:");

    // Overflow - should drop oldest
    for i in 10..13 {
        let result = sender.send(format!("Message {}", i)).await;
        println!("  Sent: Message {} (oldest dropped)", i);
        if let Err(e) = result {
            println!("    Error: {}", e);
        }
    }

    let messages = sender.drain(15).await;
    println!("\nDrained messages ({} total):", messages.len());
    for (i, msg) in messages.iter().enumerate() {
        println!("  [{}] {}", i, msg);
    }

    let stats = sender.stats().await;
    println!("\n📊 Stats:");
    println!("   Messages sent: {}", stats.messages_sent);
    println!("   Messages dropped: {}", stats.messages_dropped);
    println!(
        "   Drop rate: {:.1}%\n",
        stats.messages_dropped as f32 / stats.messages_sent as f32 * 100.0
    );

    Ok(())
}

/// Demo 5: Health monitoring
async fn demo_health_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    println!("❤️  Demo 5: Health Monitoring");
    println!("──────────────────────────────\n");

    let config = FlowControlConfig {
        max_buffer_size: 10,
        drop_policy: DropPolicy::DropOldest,
        ..Default::default()
    };

    let sender: FlowControlledSender<i32> = FlowControlledSender::new(config);

    // Healthy scenario
    println!("Scenario 1: Healthy connection (low drop rate)");
    for i in 0..100 {
        sender.send(i).await?;
    }
    sender.drain(95).await; // Drain most messages

    let healthy = sender.is_healthy().await;
    let stats = sender.stats().await;

    println!("  Messages sent: {}", stats.messages_sent);
    println!("  Messages dropped: {}", stats.messages_dropped);
    println!("  Healthy: {} ✅\n", healthy);

    // Unhealthy scenario
    println!("Scenario 2: Unhealthy connection (high drop rate)");
    for i in 0..50 {
        sender.send(i).await.ok(); // Ignore errors
    }
    // Don't drain - cause drops

    let healthy = sender.is_healthy().await;
    let stats = sender.stats().await;

    println!("  Messages sent: {}", stats.messages_sent);
    println!("  Messages dropped: {}", stats.messages_dropped);
    println!(
        "  Drop rate: {:.1}%",
        stats.messages_dropped as f32 / stats.messages_sent as f32 * 100.0
    );
    println!(
        "  Healthy: {} {}\n",
        healthy,
        if healthy { "✅" } else { "❌" }
    );

    Ok(())
}

/// Demo 6: Adaptive throttling
async fn demo_adaptive_throttling() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Demo 6: Adaptive Throttling");
    println!("────────────────────────────────\n");

    let config = FlowControlConfig {
        max_buffer_size: 100,
        backpressure_threshold: 0.5,
        pause_threshold: 0.9,
        min_batch_delay: Duration::from_millis(0),
        max_batch_delay: Duration::from_millis(200),
        adaptive_throttling: true,
        ..Default::default()
    };

    let controller = FlowController::new(config);

    println!("Simulating gradual buffer fill with adaptive delays:\n");
    println!(
        "{:>10} | {:>10} | {:>12} | {:>10}",
        "Buffer %", "State", "Delay (ms)", "Action"
    );
    println!("{}", "-".repeat(50));

    for buffer_pct in (0..=100).step_by(10) {
        let buffer_size = buffer_pct;
        let delay = controller.calculate_delay(buffer_size).await;
        let stats = controller.get_stats().await;

        let action = if buffer_pct < 50 {
            "Normal"
        } else if buffer_pct < 90 {
            "Throttle"
        } else {
            "Pause"
        };

        println!(
            "{:>9}% | {:>10?} | {:>10} | {:>10}",
            buffer_pct,
            stats.flow_state,
            delay.as_millis(),
            action
        );
    }

    println!("\n📈 Throttling increases linearly with buffer fill level\n");

    Ok(())
}
