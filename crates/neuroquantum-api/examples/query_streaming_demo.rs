//! # Query Streaming Demo
//!
//! Demonstrates real-time query result streaming over WebSocket connections
//! with progress updates, batch processing, and cancellation support.
//!
//! ## Features Demonstrated
//!
//! 1. **Streaming Query Execution**: Large result sets delivered in batches
//! 2. **Progress Updates**: Real-time execution progress
//! 3. **Query Cancellation**: Ability to cancel long-running queries
//! 4. **Multiple Concurrent Streams**: Handle multiple queries simultaneously
//! 5. **Stream Statistics**: Monitor active streams and performance
//!
//! ## Run the Demo
//!
//! ```bash
//! cargo run --example query_streaming_demo
//! ```

use neuroquantum_api::websocket::{
    QueryStreamer, StreamingConfig, StreamingMessage, StreamingRegistry,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    println!("\n🚀 NeuroQuantumDB - Query Streaming Demo\n");
    println!("========================================\n");

    // Demo 1: Basic streaming query
    demo_basic_streaming().await?;

    // Demo 2: Progress updates
    demo_progress_updates().await?;

    // Demo 3: Query cancellation
    demo_query_cancellation().await?;

    // Demo 4: Concurrent streams
    demo_concurrent_streams().await?;

    // Demo 5: Custom batch sizes
    demo_custom_batch_size().await?;

    println!("\n✅ All demos completed successfully!\n");

    Ok(())
}

/// Demo 1: Basic streaming query
async fn demo_basic_streaming() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Demo 1: Basic Streaming Query");
    println!("─────────────────────────────────\n");

    let config = StreamingConfig {
        batch_size: 50,
        progress_interval: Duration::from_millis(200),
        ..Default::default()
    };

    let registry = Arc::new(StreamingRegistry::new(config.clone()));
    let streamer = QueryStreamer::new(config, registry.clone());

    // Create a mock connection ID
    let conn_id = neuroquantum_api::websocket::ConnectionId::new();

    // Register a stream
    let stream_id = registry
        .register_stream(
            conn_id,
            "SELECT * FROM sensors WHERE temperature > 25".to_string(),
        )
        .await;

    println!("🔍 Stream ID: {}", stream_id);

    // Create mock results
    let results = streamer.create_mock_results(150);

    let mut message_count = 0;
    let send_fn = |msg: StreamingMessage| {
        match &msg {
            StreamingMessage::Started {
                query,
                estimated_rows,
                ..
            } => {
                println!("▶️  Query started: {}", query);
                println!("   Estimated rows: {:?}", estimated_rows);
            }
            StreamingMessage::Batch {
                batch_number,
                rows,
                has_more,
                ..
            } => {
                println!(
                    "📦 Batch #{}: {} rows (more: {})",
                    batch_number,
                    rows.len(),
                    has_more
                );
            }
            StreamingMessage::Progress { progress, .. } => {
                println!(
                    "⏳ Progress: {:.1}% ({} rows, {:.0} rows/sec)",
                    progress.percentage.unwrap_or(0.0),
                    progress.rows_processed,
                    progress.throughput
                );
            }
            StreamingMessage::Completed {
                total_rows,
                execution_time_ms,
                ..
            } => {
                println!(
                    "✅ Completed: {} rows in {} ms",
                    total_rows, execution_time_ms
                );
            }
            _ => {}
        }
        message_count += 1;
        Ok(())
    };

    streamer
        .stream_results(
            stream_id,
            "SELECT * FROM sensors".to_string(),
            results,
            send_fn,
        )
        .await?;

    println!("   Total messages: {}\n", message_count);

    registry.remove_stream(stream_id).await;

    Ok(())
}

/// Demo 2: Progress updates with large dataset
async fn demo_progress_updates() -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 Demo 2: Progress Updates");
    println!("───────────────────────────\n");

    let config = StreamingConfig {
        batch_size: 100,
        progress_interval: Duration::from_millis(100),
        detailed_progress: true,
        ..Default::default()
    };

    let registry = Arc::new(StreamingRegistry::new(config.clone()));
    let streamer = QueryStreamer::new(config, registry.clone());

    let conn_id = neuroquantum_api::websocket::ConnectionId::new();
    let stream_id = registry
        .register_stream(conn_id, "SELECT * FROM large_table".to_string())
        .await;

    let results = streamer.create_mock_results(500);

    let mut progress_count = 0;
    let send_fn = |msg: StreamingMessage| {
        if let StreamingMessage::Progress { progress, .. } = &msg {
            progress_count += 1;
            println!(
                "📊 {} / {} rows ({:.1}%) - ETA: {:?} ms",
                progress.rows_processed,
                progress.estimated_total.unwrap_or(0),
                progress.percentage.unwrap_or(0.0),
                progress.estimated_remaining_ms
            );
        }
        Ok(())
    };

    streamer
        .stream_results(
            stream_id,
            "SELECT * FROM large_table".to_string(),
            results,
            send_fn,
        )
        .await?;

    println!("   Progress updates: {}\n", progress_count);

    registry.remove_stream(stream_id).await;

    Ok(())
}

/// Demo 3: Query cancellation
async fn demo_query_cancellation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🛑 Demo 3: Query Cancellation");
    println!("─────────────────────────────\n");

    let config = StreamingConfig::default();
    let registry = Arc::new(StreamingRegistry::new(config.clone()));
    let streamer = QueryStreamer::new(config, registry.clone());

    let conn_id = neuroquantum_api::websocket::ConnectionId::new();
    let stream_id = registry
        .register_stream(conn_id, "SELECT * FROM huge_table".to_string())
        .await;

    println!("🔍 Starting query...");
    println!("🛑 Simulating cancellation after 2 batches\n");

    let results = streamer.create_mock_results(1000);

    let registry_clone = registry.clone();
    let stream_id_clone = stream_id;

    let send_fn = move |msg: StreamingMessage| {
        if let StreamingMessage::Batch { batch_number, .. } = &msg {
            println!("📦 Batch #{}", batch_number);

            if *batch_number >= 2 {
                println!("\n⚠️  Cancelling query...");
                let registry = registry_clone.clone();
                tokio::spawn(async move { registry.cancel_stream(stream_id_clone).await });
            }
        }
        Ok(())
    };

    match streamer
        .stream_results(
            stream_id,
            "SELECT * FROM huge_table".to_string(),
            results,
            send_fn,
        )
        .await
    {
        Ok(total) => println!("✅ Processed {} rows before cancellation\n", total),
        Err(e) => println!("❌ Query cancelled: {}\n", e),
    }

    let stats = registry.get_stream_stats(stream_id).await;
    if let Some(stats) = stats {
        println!("📊 Final stats:");
        println!("   Status: {:?}", stats.status);
        println!("   Rows sent: {}", stats.rows_sent);
        println!("   Batches sent: {}\n", stats.batches_sent);
    }

    registry.remove_stream(stream_id).await;

    Ok(())
}

/// Demo 4: Multiple concurrent streams
async fn demo_concurrent_streams() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔀 Demo 4: Concurrent Streams");
    println!("─────────────────────────────\n");

    let config = StreamingConfig {
        batch_size: 50,
        ..Default::default()
    };

    let registry = Arc::new(StreamingRegistry::new(config.clone()));
    let streamer = Arc::new(QueryStreamer::new(config, registry.clone()));

    // Start 3 concurrent streams
    let mut handles = vec![];

    for i in 1..=3 {
        let streamer_clone = streamer.clone();
        let registry_clone = registry.clone();

        let handle = tokio::spawn(async move {
            let conn_id = neuroquantum_api::websocket::ConnectionId::new();
            let query = format!("SELECT * FROM table_{}", i);
            let stream_id = registry_clone.register_stream(conn_id, query.clone()).await;

            println!("🔍 Stream {}: Started (ID: {})", i, stream_id);

            let results = streamer_clone.create_mock_results(100 * i);

            let send_fn = move |msg: StreamingMessage| {
                if let StreamingMessage::Completed {
                    total_rows,
                    execution_time_ms,
                    ..
                } = msg
                {
                    println!(
                        "✅ Stream {}: Completed ({} rows in {} ms)",
                        i, total_rows, execution_time_ms
                    );
                }
                Ok(())
            };

            let _ = streamer_clone
                .stream_results(stream_id, query, results, send_fn)
                .await;

            registry_clone.remove_stream(stream_id).await;
        });

        handles.push(handle);
    }

    // Wait for all streams to complete
    for handle in handles {
        handle.await?;
    }

    println!("\n   All streams completed\n");

    Ok(())
}

/// Demo 5: Custom batch sizes
async fn demo_custom_batch_size() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚙️  Demo 5: Custom Batch Sizes");
    println!("──────────────────────────────\n");

    for batch_size in [10, 50, 200] {
        println!("📦 Testing batch size: {}", batch_size);

        let config = StreamingConfig {
            batch_size,
            progress_interval: Duration::from_secs(999), // Disable progress
            ..Default::default()
        };

        let registry = Arc::new(StreamingRegistry::new(config.clone()));
        let streamer = QueryStreamer::new(config, registry.clone());

        let conn_id = neuroquantum_api::websocket::ConnectionId::new();
        let stream_id = registry
            .register_stream(conn_id, "SELECT * FROM test".to_string())
            .await;

        let results = streamer.create_mock_results(250);

        let mut batch_count = 0;
        let send_fn = |msg: StreamingMessage| {
            if matches!(msg, StreamingMessage::Batch { .. }) {
                batch_count += 1;
            }
            Ok(())
        };

        streamer
            .stream_results(
                stream_id,
                "SELECT * FROM test".to_string(),
                results,
                send_fn,
            )
            .await?;

        println!("   Batches created: {}", batch_count);
        println!("   Expected: ~{}\n", 250_usize.div_ceil(batch_size));

        registry.remove_stream(stream_id).await;
    }

    Ok(())
}
