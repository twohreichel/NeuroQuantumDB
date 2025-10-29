//! Advanced Monitoring Demo
//!
//! Demonstrates the advanced query monitoring capabilities including:
//! - Slow query logging
//! - Index usage statistics
//! - Lock contention tracking
//! - Query execution histograms

use neuroquantum_core::monitoring::{
    AdvancedQueryMetrics, LockType, MonitoringConfig, QueryExecutionParams,
};
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("=== NeuroQuantumDB Advanced Monitoring Demo ===\n");

    // Initialize monitoring with custom configuration
    let config = MonitoringConfig {
        slow_query_threshold_ms: 50,
        max_slow_queries: 1000,
        log_execution_plans: true,
        track_index_usage: true,
        track_lock_contention: true,
    };

    let metrics = AdvancedQueryMetrics::new(config);

    println!("✅ Monitoring initialized with:");
    println!("   - Slow query threshold: 50ms");
    println!("   - Max slow queries: 1000");
    println!("   - Execution plan logging: enabled");
    println!("   - Index usage tracking: enabled");
    println!("   - Lock contention tracking: enabled\n");

    // Scenario 1: Fast queries (won't be logged as slow)
    println!("📊 Scenario 1: Recording fast queries");
    for i in 1..=10 {
        metrics
            .record_query_execution(QueryExecutionParams {
                query_id: i,
                query_text: format!("SELECT * FROM users WHERE id = {}", i),
                execution_time: Duration::from_millis(5), // Fast query
                rows_examined: 100,
                rows_returned: 10,
                index_used: Some("idx_users_id".to_string()),
                lock_wait_time: Duration::from_millis(0),
                execution_plan: Some("Index Scan using idx_users_id".to_string()),
                user: "admin".to_string(),
                success: true,
            })
            .await;
    }
    println!("   ✓ Recorded 10 fast queries (5ms each)");

    // Scenario 2: Slow queries (will be logged)
    println!("\n📊 Scenario 2: Recording slow queries");
    for i in 11..=15 {
        metrics
            .record_query_execution(QueryExecutionParams {
                query_id: i,
                query_text: format!("SELECT * FROM orders WHERE status = 'pending' AND created_at > '{}'", i),
                execution_time: Duration::from_millis(150), // Slow query
                rows_examined: 10000,
                rows_returned: 500,
                index_used: None, // No index used - full table scan
                lock_wait_time: Duration::from_millis(20),
                execution_plan: Some("Seq Scan on orders (cost=0..1000 rows=500)".to_string()),
                user: "api_user".to_string(),
                success: true,
            })
            .await;
    }
    println!("   ✓ Recorded 5 slow queries (150ms each)");

    // Scenario 3: Lock contention
    println!("\n📊 Scenario 3: Recording lock contention events");
    for i in 1..=5 {
        metrics
            .record_lock_contention(
                100 + i,
                "orders_table".to_string(),
                LockType::Exclusive,
                Duration::from_millis(30 + i * 10),
                Some(200 + i),
            )
            .await;
    }
    println!("   ✓ Recorded 5 lock contention events");

    // Scenario 4: Different lock types
    println!("\n📊 Scenario 4: Recording various lock types");
    metrics
        .record_lock_contention(
            201,
            "users_table".to_string(),
            LockType::Shared,
            Duration::from_millis(5),
            None,
        )
        .await;

    metrics
        .record_lock_contention(
            202,
            "idx_orders_status".to_string(),
            LockType::IndexLevel,
            Duration::from_millis(15),
            Some(203),
        )
        .await;
    println!("   ✓ Recorded different lock types");

    // Get and display slow query report
    println!("\n📋 Slow Query Report:");
    println!("   {:-<80}", "");
    let slow_queries = metrics.get_slow_queries().await;
    println!("   Total slow queries: {}", slow_queries.len());
    for (idx, query) in slow_queries.iter().enumerate().take(3) {
        println!("\n   Query #{}", idx + 1);
        println!("     Query ID: {}", query.query_id);
        println!("     Execution time: {}ms", query.execution_time_ms);
        println!("     Rows examined: {}", query.rows_examined);
        println!("     Rows returned: {}", query.rows_returned);
        println!("     Index used: {:?}", query.index_used);
        println!("     Lock wait time: {}ms", query.lock_wait_time_ms);
        println!("     User: {}", query.user);
        let truncated_query = if query.query_text.len() > 60 {
            format!("{}...", &query.query_text[..60])
        } else {
            query.query_text.clone()
        };
        println!("     Query: {}", truncated_query);
    }

    // Get and display index usage statistics
    println!("\n\n📋 Index Usage Statistics:");
    println!("   {:-<80}", "");
    let index_stats = metrics.get_index_stats().await;
    println!("   Total indexes tracked: {}", index_stats.len());
    for (idx, stat) in index_stats.iter().enumerate() {
        println!("\n   Index #{}", idx + 1);
        println!("     Name: {}", stat.index_name);
        println!("     Table: {}", stat.table_name);
        println!("     Total scans: {}", stat.total_scans);
        println!("     Total rows read: {}", stat.total_rows_read);
        println!("     Avg rows per scan: {:.2}", stat.avg_rows_per_scan);
    }

    // Check for unused indexes
    println!("\n\n📋 Unused Index Analysis:");
    println!("   {:-<80}", "");
    let unused_indexes = metrics.get_unused_indexes(3600).await; // 1 hour threshold
    if unused_indexes.is_empty() {
        println!("   ✅ All indexes are being used");
    } else {
        println!("   ⚠️  Unused indexes (not used in last hour): {:?}", unused_indexes);
    }

    // Get and display lock contention summary
    println!("\n\n📋 Lock Contention Summary:");
    println!("   {:-<80}", "");
    let contention = metrics.get_contention_summary().await;
    println!("   Total contentions: {}", contention.total_contentions);
    println!("   Average wait time: {:.2}ms", contention.avg_wait_time_ms);

    println!("\n   Hot Resources (Top 5):");
    for (resource, count) in contention.hot_resources.iter().take(5) {
        println!("     - {}: {} contentions", resource, count);
    }

    println!("\n   Contentions by Type:");
    for (lock_type, count) in &contention.contentions_by_type {
        println!("     - {}: {} contentions", lock_type, count);
    }

    // Get and display execution summary
    println!("\n\n📋 Query Execution Summary:");
    println!("   {:-<80}", "");
    let execution = metrics.get_execution_summary().await;
    println!("   Total queries: {}", execution.total_queries);
    println!("   Failed queries: {}", execution.failed_queries);
    println!("   Error rate: {:.2}%", execution.error_rate);

    println!("\n   Slowest Query Patterns (Top 3):");
    for (idx, pattern) in execution.slowest_patterns.iter().enumerate().take(3) {
        println!("\n     Pattern #{}", idx + 1);
        let truncated_pattern = if pattern.query_pattern.len() > 60 {
            format!("{}...", &pattern.query_pattern[..60])
        } else {
            pattern.query_pattern.clone()
        };
        println!("       Query: {}", truncated_pattern);
        println!("       Count: {}", pattern.count);
        println!("       Avg time: {:.2}ms", pattern.avg_time_ms);
        println!("       Min time: {}ms", pattern.min_time_ms);
        println!("       Max time: {}ms", pattern.max_time_ms);
        println!("       P50: {}ms, P95: {}ms, P99: {}ms",
                 pattern.p50_time_ms, pattern.p95_time_ms, pattern.p99_time_ms);
    }

    println!("\n   Most Frequent Query Patterns (Top 3):");
    for (idx, pattern) in execution.most_frequent_patterns.iter().enumerate().take(3) {
        println!("\n     Pattern #{}", idx + 1);
        let truncated_pattern = if pattern.query_pattern.len() > 60 {
            format!("{}...", &pattern.query_pattern[..60])
        } else {
            pattern.query_pattern.clone()
        };
        println!("       Query: {}", truncated_pattern);
        println!("       Count: {}", pattern.count);
        println!("       Avg time: {:.2}ms", pattern.avg_time_ms);
    }

    // Performance insights
    println!("\n\n💡 Performance Insights:");
    println!("   {:-<80}", "");

    // Check for missing indexes
    let queries_without_index = slow_queries.iter().filter(|q| q.index_used.is_none()).count();
    if queries_without_index > 0 {
        println!("   ⚠️  {} slow queries without index usage - consider adding indexes",
                 queries_without_index);
    }

    // Check for high lock contention
    if contention.avg_wait_time_ms > 20.0 {
        println!("   ⚠️  High average lock wait time ({:.2}ms) - consider optimizing transactions",
                 contention.avg_wait_time_ms);
    }

    // Check for errors
    if execution.error_rate > 1.0 {
        println!("   ⚠️  Error rate is {:.2}% - investigate failed queries",
                 execution.error_rate);
    } else {
        println!("   ✅ Error rate is acceptable ({:.2}%)", execution.error_rate);
    }

    // Index recommendations
    if !index_stats.is_empty() {
        let max_scans = index_stats.iter().map(|s| s.total_scans).max().unwrap_or(0);
        if max_scans > 0 {
            println!("   ✅ Indexes are being utilized effectively");
        }
    }

    println!("\n=== Demo Complete ===");
}

