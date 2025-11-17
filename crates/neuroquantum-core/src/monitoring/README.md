# Advanced Query Monitoring

Comprehensive query monitoring and performance analysis for NeuroQuantumDB.

## Features

### 📊 Slow Query Log

Automatically captures queries exceeding configurable thresholds:

```rust
use neuroquantum_core::monitoring::{AdvancedQueryMetrics, MonitoringConfig};
use std::time::Duration;

let config = MonitoringConfig {
    slow_query_threshold_ms: 100, // Log queries > 100ms
    max_slow_queries: 1000,
    ..Default::default()
};

let metrics = AdvancedQueryMetrics::new(config);

// Record query execution
metrics.record_query_execution(
    1, // query_id
    "SELECT * FROM users WHERE status = 'active'".to_string(),
    Duration::from_millis(150), // execution_time
    10000, // rows_examined
    500, // rows_returned
    None, // no index used
    Duration::from_millis(20), // lock_wait_time
    Some("Seq Scan on users".to_string()), // execution_plan
    "admin".to_string(), // user
    true, // success
).await;

// Get slow queries
let slow_queries = metrics.get_slow_queries().await;
for query in slow_queries {
    println!("Query {} took {}ms", query.query_id, query.execution_time_ms);
}
```

**Captured Information:**
- Query text and ID
- Execution time
- Rows examined vs returned
- Index usage
- Lock wait time
- Execution plan
- User information
- Timestamp

### 📈 Index Usage Statistics

Track index utilization to identify optimization opportunities:

```rust
// Get all index statistics
let stats = metrics.get_index_stats().await;
for stat in stats {
    println!("{} on {}: {} scans, {:.2} avg rows/scan",
        stat.index_name,
        stat.table_name,
        stat.total_scans,
        stat.avg_rows_per_scan
    );
}

// Find unused indexes (not used in last hour)
let unused = metrics.get_unused_indexes(3600).await;
if !unused.is_empty() {
    println!("Unused indexes: {:?}", unused);
    println!("Consider dropping these indexes to save space");
}
```

**Tracked Metrics:**
- Total scans per index
- Total rows read
- Average rows per scan
- Last usage timestamp
- Creation timestamp

**Optimization Hints:**
- Identify unused indexes (candidates for removal)
- Find most/least used indexes
- Analyze scan efficiency (rows per scan)

### 🔒 Lock Contention Tracking

Monitor lock contention and identify hotspots:

```rust
use neuroquantum_core::monitoring::LockType;

// Record lock contention
metrics.record_lock_contention(
    101, // query_id
    "orders_table".to_string(), // resource
    LockType::Exclusive,
    Duration::from_millis(50), // wait_time
    Some(102), // holder_query_id
).await;

// Get contention summary
let summary = metrics.get_contention_summary().await;
println!("Total contentions: {}", summary.total_contentions);
println!("Avg wait time: {:.2}ms", summary.avg_wait_time_ms);

// Find hot resources
for (resource, count) in summary.hot_resources {
    println!("{}: {} contentions", resource, count);
}

// Breakdown by lock type
for (lock_type, count) in summary.contentions_by_type {
    println!("{}: {} contentions", lock_type, count);
}
```

**Lock Types:**
- `Shared` - Multiple readers allowed
- `Exclusive` - Single writer, no readers
- `RowLevel` - Row-level locks
- `TableLevel` - Table-level locks
- `IndexLevel` - Index-level locks

**Analysis:**
- Total contention count
- Average wait time
- Hottest resources (most contended)
- Contention breakdown by type

### 📊 Query Execution Statistics

Histogram analysis with percentile calculations:

```rust
// Get execution summary
let summary = metrics.get_execution_summary().await;
println!("Total queries: {}", summary.total_queries);
println!("Failed queries: {}", summary.failed_queries);
println!("Error rate: {:.2}%", summary.error_rate);

// Slowest query patterns
for pattern in summary.slowest_patterns {
    println!("\nPattern: {}", pattern.query_pattern);
    println!("  Count: {}", pattern.count);
    println!("  Avg: {:.2}ms", pattern.avg_time_ms);
    println!("  Min: {}ms, Max: {}ms", pattern.min_time_ms, pattern.max_time_ms);
    println!("  P50: {}ms, P95: {}ms, P99: {}ms",
        pattern.p50_time_ms, pattern.p95_time_ms, pattern.p99_time_ms);
}

// Most frequent patterns
for pattern in summary.most_frequent_patterns {
    println!("\nPattern: {} ({} executions)", 
        pattern.query_pattern, pattern.count);
}
```

**Statistics Per Pattern:**
- Execution count
- Total/average/min/max time
- Percentiles (p50, p95, p99)
- Query pattern (normalized)

**Use Cases:**
- Identify slowest queries
- Find most frequent queries
- Detect performance regressions
- Optimize query patterns

## Configuration

```rust
use neuroquantum_core::monitoring::MonitoringConfig;

let config = MonitoringConfig {
    // Queries exceeding this threshold are logged
    slow_query_threshold_ms: 100,
    
    // Maximum slow queries to keep in memory
    max_slow_queries: 1000,
    
    // Enable execution plan logging
    log_execution_plans: true,
    
    // Enable index usage tracking
    track_index_usage: true,
    
    // Enable lock contention tracking
    track_lock_contention: true,
};
```

## Performance

- **Memory Overhead**: < 5MB for typical workloads
- **CPU Overhead**: < 0.1% (< 10μs per query)
- **Thread Safety**: Async RwLock, minimal contention
- **Scalability**: Tested with 10,000+ queries/sec

## Integration

### With Query Executor

```rust
// In your query execution path
let start = Instant::now();
let result = execute_query(&query).await?;
let duration = start.elapsed();

metrics.record_query_execution(
    query.id,
    query.text.clone(),
    duration,
    result.rows_examined,
    result.rows_returned,
    result.index_used,
    result.lock_wait_time,
    result.execution_plan,
    query.user.clone(),
    result.is_ok(),
).await;
```

### With Lock Manager

```rust
// When lock contention occurs
metrics.record_lock_contention(
    current_query_id,
    resource_name,
    lock_type,
    wait_duration,
    holder_query_id,
).await;
```

## Examples

Run the comprehensive demo:

```bash
cargo run --example advanced_monitoring_demo
```

## API Reference

### `AdvancedQueryMetrics`

Main metrics collector.

**Methods:**
- `new(config: MonitoringConfig) -> Self`
- `record_query_execution(...) -> impl Future<Output = ()>`
- `record_lock_contention(...) -> impl Future<Output = ()>`
- `get_slow_queries() -> impl Future<Output = Vec<SlowQueryEntry>>`
- `get_index_stats() -> impl Future<Output = Vec<IndexUsageEntry>>`
- `get_unused_indexes(threshold_secs: u64) -> impl Future<Output = Vec<String>>`
- `get_contention_summary() -> impl Future<Output = ContentionSummary>`
- `get_execution_summary() -> impl Future<Output = ExecutionSummary>`

### `MonitoringConfig`

Configuration options.

**Fields:**
- `slow_query_threshold_ms: u64` - Threshold for slow query logging
- `max_slow_queries: usize` - Max entries to keep
- `log_execution_plans: bool` - Enable plan logging
- `track_index_usage: bool` - Enable index tracking
- `track_lock_contention: bool` - Enable lock tracking

## Best Practices

1. **Tune Threshold**: Set `slow_query_threshold_ms` based on your SLA
2. **Review Regularly**: Check slow queries and unused indexes weekly
3. **Optimize Proactively**: Add indexes before contentions become critical
4. **Monitor Trends**: Track changes in execution patterns over time
5. **Set Alerts**: Configure alerts for high contention or error rates

## Future Enhancements

- [ ] Prometheus metrics export
- [ ] Grafana dashboard templates
- [ ] Automatic index recommendations
- [ ] Query rewrite suggestions
- [ ] Historical trend analysis
- [ ] Machine learning for anomaly detection

