//! Advanced Query Monitoring and Performance Analysis
//!
//! Provides comprehensive query metrics including:
//! - Slow query logging with configurable thresholds
//! - Index usage statistics and optimization hints
//! - Lock contention tracking and analysis
//! - Query execution plans and performance profiling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Comprehensive query metrics for production monitoring
#[derive(Debug, Clone)]
pub struct AdvancedQueryMetrics {
    slow_query_log: Arc<RwLock<SlowQueryLog>>,
    index_usage_stats: Arc<RwLock<IndexUsageStats>>,
    lock_contention_tracker: Arc<RwLock<LockContentionTracker>>,
    query_execution_stats: Arc<RwLock<QueryExecutionStats>>,
    config: MonitoringConfig,
}

/// Configuration for query monitoring
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Threshold in milliseconds for slow query logging
    pub slow_query_threshold_ms: u64,
    /// Maximum number of slow queries to keep in memory
    pub max_slow_queries: usize,
    /// Enable detailed execution plan logging
    pub log_execution_plans: bool,
    /// Enable index usage statistics
    pub track_index_usage: bool,
    /// Enable lock contention tracking
    pub track_lock_contention: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            slow_query_threshold_ms: 100, // 100ms default threshold
            max_slow_queries: 1000,
            log_execution_plans: true,
            track_index_usage: true,
            track_lock_contention: true,
        }
    }
}

/// Slow query log entry with detailed diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQueryEntry {
    pub query_id: u64,
    pub query_text: String,
    pub execution_time_ms: u64,
    pub timestamp: u64,
    pub user: String,
    pub rows_examined: u64,
    pub rows_returned: u64,
    pub index_used: Option<String>,
    pub lock_wait_time_ms: u64,
    pub execution_plan: Option<String>,
}

/// Parameters for recording a query execution
#[derive(Debug, Clone)]
pub struct QueryExecutionParams {
    pub query_id: u64,
    pub query_text: String,
    pub execution_time: Duration,
    pub rows_examined: u64,
    pub rows_returned: u64,
    pub index_used: Option<String>,
    pub lock_wait_time: Duration,
    pub execution_plan: Option<String>,
    pub user: String,
    pub success: bool,
}

/// Slow query log manager
#[derive(Debug, Default)]
pub struct SlowQueryLog {
    entries: Vec<SlowQueryEntry>,
    total_slow_queries: u64,
    queries_by_table: HashMap<String, u64>,
}

impl SlowQueryLog {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            total_slow_queries: 0,
            queries_by_table: HashMap::new(),
        }
    }

    /// Add a slow query entry
    pub fn add_entry(&mut self, entry: SlowQueryEntry, max_entries: usize) {
        self.total_slow_queries += 1;

        // Extract table name from query (simplified)
        if let Some(table) = self.extract_table_name(&entry.query_text) {
            *self.queries_by_table.entry(table).or_insert(0) += 1;
        }

        // Keep only the most recent entries
        if self.entries.len() >= max_entries {
            self.entries.remove(0);
        }

        self.entries.push(entry);
    }

    /// Get all slow query entries
    #[must_use] 
    pub fn get_entries(&self) -> &[SlowQueryEntry] {
        &self.entries
    }

    /// Get total count of slow queries
    #[must_use] 
    pub const fn total_slow_queries(&self) -> u64 {
        self.total_slow_queries
    }

    /// Get slow queries by table
    #[must_use] 
    pub const fn queries_by_table(&self) -> &HashMap<String, u64> {
        &self.queries_by_table
    }

    /// Extract table name from query text (simplified parser)
    fn extract_table_name(&self, query: &str) -> Option<String> {
        let query_lower = query.to_lowercase();
        if let Some(from_pos) = query_lower.find("from ") {
            let after_from = &query_lower[from_pos + 5..];
            let table_name = after_from
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c| c == '(' || c == ')' || c == ',');
            if !table_name.is_empty() {
                return Some(table_name.to_string());
            }
        }
        None
    }
}

/// Index usage statistics entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexUsageEntry {
    pub index_name: String,
    pub table_name: String,
    pub total_scans: u64,
    pub total_rows_read: u64,
    pub avg_rows_per_scan: f64,
    pub last_used: u64,
    pub created_at: u64,
}

/// Index usage statistics tracker
#[derive(Debug, Default)]
pub struct IndexUsageStats {
    indexes: HashMap<String, IndexUsageEntry>,
    unused_indexes: Vec<String>,
}

impl IndexUsageStats {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
            unused_indexes: Vec::new(),
        }
    }

    /// Record index usage
    pub fn record_index_scan(&mut self, index_name: String, table_name: String, rows_read: u64) {
        let entry = self
            .indexes
            .entry(index_name.clone())
            .or_insert_with(|| IndexUsageEntry {
                index_name: index_name.clone(),
                table_name,
                total_scans: 0,
                total_rows_read: 0,
                avg_rows_per_scan: 0.0,
                last_used: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                created_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            });

        entry.total_scans += 1;
        entry.total_rows_read += rows_read;
        entry.avg_rows_per_scan = entry.total_rows_read as f64 / entry.total_scans as f64;
        entry.last_used = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Get all index usage statistics
    #[must_use] 
    pub fn get_all_stats(&self) -> Vec<IndexUsageEntry> {
        self.indexes.values().cloned().collect()
    }

    /// Get unused indexes (not used in last N seconds)
    pub fn get_unused_indexes(&mut self, threshold_secs: u64) -> Vec<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.unused_indexes.clear();

        for (name, entry) in &self.indexes {
            if now - entry.last_used > threshold_secs {
                self.unused_indexes.push(name.clone());
            }
        }

        self.unused_indexes.clone()
    }

    /// Get most frequently used indexes
    #[must_use] 
    pub fn get_top_indexes(&self, limit: usize) -> Vec<IndexUsageEntry> {
        let mut entries: Vec<IndexUsageEntry> = self.indexes.values().cloned().collect();
        entries.sort_by(|a, b| b.total_scans.cmp(&a.total_scans));
        entries.truncate(limit);
        entries
    }
}

/// Lock contention event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockContentionEvent {
    pub query_id: u64,
    pub resource: String,
    pub lock_type: LockType,
    pub wait_time_ms: u64,
    pub timestamp: u64,
    pub holder_query_id: Option<u64>,
}

/// Lock type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LockType {
    Shared,
    Exclusive,
    RowLevel,
    TableLevel,
    IndexLevel,
}

impl std::fmt::Display for LockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::Shared => write!(f, "Shared"),
            | Self::Exclusive => write!(f, "Exclusive"),
            | Self::RowLevel => write!(f, "RowLevel"),
            | Self::TableLevel => write!(f, "TableLevel"),
            | Self::IndexLevel => write!(f, "IndexLevel"),
        }
    }
}

/// Lock contention tracker
#[derive(Debug, Default)]
pub struct LockContentionTracker {
    events: Vec<LockContentionEvent>,
    total_contentions: u64,
    total_wait_time_ms: u64,
    contentions_by_resource: HashMap<String, u64>,
    contentions_by_type: HashMap<LockType, u64>,
}

impl LockContentionTracker {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            total_contentions: 0,
            total_wait_time_ms: 0,
            contentions_by_resource: HashMap::new(),
            contentions_by_type: HashMap::new(),
        }
    }

    /// Record a lock contention event
    pub fn record_contention(&mut self, event: LockContentionEvent, max_events: usize) {
        self.total_contentions += 1;
        self.total_wait_time_ms += event.wait_time_ms;

        *self
            .contentions_by_resource
            .entry(event.resource.clone())
            .or_insert(0) += 1;

        *self
            .contentions_by_type
            .entry(event.lock_type.clone())
            .or_insert(0) += 1;

        // Keep only recent events
        if self.events.len() >= max_events {
            self.events.remove(0);
        }

        self.events.push(event);
    }

    /// Get all contention events
    #[must_use] 
    pub fn get_events(&self) -> &[LockContentionEvent] {
        &self.events
    }

    /// Get total contention count
    #[must_use] 
    pub const fn total_contentions(&self) -> u64 {
        self.total_contentions
    }

    /// Get average wait time
    #[must_use] 
    pub fn avg_wait_time_ms(&self) -> f64 {
        if self.total_contentions == 0 {
            0.0
        } else {
            self.total_wait_time_ms as f64 / self.total_contentions as f64
        }
    }

    /// Get contentions by resource
    #[must_use] 
    pub const fn contentions_by_resource(&self) -> &HashMap<String, u64> {
        &self.contentions_by_resource
    }

    /// Get contentions by type
    #[must_use] 
    pub const fn contentions_by_type(&self) -> &HashMap<LockType, u64> {
        &self.contentions_by_type
    }

    /// Get most contended resources
    #[must_use] 
    pub fn get_hot_resources(&self, limit: usize) -> Vec<(String, u64)> {
        let mut resources: Vec<(String, u64)> = self
            .contentions_by_resource
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        resources.sort_by(|a, b| b.1.cmp(&a.1));
        resources.truncate(limit);
        resources
    }
}

/// Query execution statistics
#[derive(Debug, Default)]
pub struct QueryExecutionStats {
    query_histogram: HashMap<String, QueryHistogram>,
    total_queries: u64,
    failed_queries: u64,
}

/// Histogram for query execution times
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistogram {
    pub query_pattern: String,
    pub count: u64,
    pub total_time_ms: u64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub avg_time_ms: f64,
    pub p50_time_ms: u64,
    pub p95_time_ms: u64,
    pub p99_time_ms: u64,
    pub execution_times: Vec<u64>, // Keep recent execution times for percentile calculation
}

impl QueryHistogram {
    #[must_use] 
    pub const fn new(query_pattern: String) -> Self {
        Self {
            query_pattern,
            count: 0,
            total_time_ms: 0,
            min_time_ms: u64::MAX,
            max_time_ms: 0,
            avg_time_ms: 0.0,
            p50_time_ms: 0,
            p95_time_ms: 0,
            p99_time_ms: 0,
            execution_times: Vec::new(),
        }
    }

    /// Add an execution time and recalculate statistics
    pub fn add_execution(&mut self, time_ms: u64, max_samples: usize) {
        self.count += 1;
        self.total_time_ms += time_ms;
        self.min_time_ms = self.min_time_ms.min(time_ms);
        self.max_time_ms = self.max_time_ms.max(time_ms);
        self.avg_time_ms = self.total_time_ms as f64 / self.count as f64;

        // Keep recent execution times for percentile calculation
        if self.execution_times.len() >= max_samples {
            self.execution_times.remove(0);
        }
        self.execution_times.push(time_ms);

        // Recalculate percentiles
        self.calculate_percentiles();
    }

    /// Calculate percentile statistics
    fn calculate_percentiles(&mut self) {
        if self.execution_times.is_empty() {
            return;
        }

        let mut sorted = self.execution_times.clone();
        sorted.sort_unstable();

        let len = sorted.len();
        self.p50_time_ms = sorted[len / 2];
        self.p95_time_ms = sorted[(len as f64 * 0.95) as usize];
        self.p99_time_ms = sorted[(len as f64 * 0.99) as usize];
    }
}

impl QueryExecutionStats {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            query_histogram: HashMap::new(),
            total_queries: 0,
            failed_queries: 0,
        }
    }

    /// Record query execution
    pub fn record_execution(&mut self, query_pattern: String, time_ms: u64, success: bool) {
        self.total_queries += 1;

        if !success {
            self.failed_queries += 1;
        }

        let histogram = self
            .query_histogram
            .entry(query_pattern.clone())
            .or_insert_with(|| QueryHistogram::new(query_pattern));

        histogram.add_execution(time_ms, 1000); // Keep last 1000 executions
    }

    /// Get all query histograms
    #[must_use] 
    pub fn get_all_histograms(&self) -> Vec<QueryHistogram> {
        self.query_histogram.values().cloned().collect()
    }

    /// Get slowest query patterns
    #[must_use] 
    pub fn get_slowest_patterns(&self, limit: usize) -> Vec<QueryHistogram> {
        let mut histograms: Vec<QueryHistogram> = self.query_histogram.values().cloned().collect();
        histograms.sort_by(|a, b| {
            b.avg_time_ms
                .partial_cmp(&a.avg_time_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        histograms.truncate(limit);
        histograms
    }

    /// Get most frequent query patterns
    #[must_use] 
    pub fn get_most_frequent_patterns(&self, limit: usize) -> Vec<QueryHistogram> {
        let mut histograms: Vec<QueryHistogram> = self.query_histogram.values().cloned().collect();
        histograms.sort_by(|a, b| b.count.cmp(&a.count));
        histograms.truncate(limit);
        histograms
    }

    /// Get total queries
    #[must_use] 
    pub const fn total_queries(&self) -> u64 {
        self.total_queries
    }

    /// Get failed queries count
    #[must_use] 
    pub const fn failed_queries(&self) -> u64 {
        self.failed_queries
    }

    /// Get error rate percentage
    #[must_use] 
    pub fn error_rate(&self) -> f64 {
        if self.total_queries == 0 {
            0.0
        } else {
            (self.failed_queries as f64 / self.total_queries as f64) * 100.0
        }
    }
}

impl AdvancedQueryMetrics {
    /// Create a new advanced query metrics collector
    #[must_use] 
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            slow_query_log: Arc::new(RwLock::new(SlowQueryLog::new())),
            index_usage_stats: Arc::new(RwLock::new(IndexUsageStats::new())),
            lock_contention_tracker: Arc::new(RwLock::new(LockContentionTracker::new())),
            query_execution_stats: Arc::new(RwLock::new(QueryExecutionStats::new())),
            config,
        }
    }

    /// Record a query execution
    pub async fn record_query_execution(&self, params: QueryExecutionParams) {
        let execution_time_ms = params.execution_time.as_millis() as u64;

        // Record execution statistics
        let query_pattern = self.normalize_query(&params.query_text);
        self.query_execution_stats.write().await.record_execution(
            query_pattern,
            execution_time_ms,
            params.success,
        );

        // Log slow queries
        if execution_time_ms >= self.config.slow_query_threshold_ms {
            let table_name = self.extract_table_from_query(&params.query_text);
            let entry = SlowQueryEntry {
                query_id: params.query_id,
                query_text: params.query_text,
                execution_time_ms,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                user: params.user,
                rows_examined: params.rows_examined,
                rows_returned: params.rows_returned,
                index_used: params.index_used.clone(),
                lock_wait_time_ms: params.lock_wait_time.as_millis() as u64,
                execution_plan: params.execution_plan,
            };

            self.slow_query_log
                .write()
                .await
                .add_entry(entry, self.config.max_slow_queries);

            // Record index usage
            if self.config.track_index_usage {
                if let Some(index_name) = params.index_used {
                    self.index_usage_stats.write().await.record_index_scan(
                        index_name,
                        table_name,
                        params.rows_examined,
                    );
                }
            }
        }
    }

    /// Record lock contention event
    pub async fn record_lock_contention(
        &self,
        query_id: u64,
        resource: String,
        lock_type: LockType,
        wait_time: Duration,
        holder_query_id: Option<u64>,
    ) {
        if !self.config.track_lock_contention {
            return;
        }

        let event = LockContentionEvent {
            query_id,
            resource,
            lock_type,
            wait_time_ms: wait_time.as_millis() as u64,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            holder_query_id,
        };

        self.lock_contention_tracker
            .write()
            .await
            .record_contention(event, 10000); // Keep last 10k events
    }

    /// Get slow query log entries
    pub async fn get_slow_queries(&self) -> Vec<SlowQueryEntry> {
        self.slow_query_log.read().await.get_entries().to_vec()
    }

    /// Get index usage statistics
    pub async fn get_index_stats(&self) -> Vec<IndexUsageEntry> {
        self.index_usage_stats.read().await.get_all_stats()
    }

    /// Get unused indexes
    pub async fn get_unused_indexes(&self, threshold_secs: u64) -> Vec<String> {
        self.index_usage_stats
            .write()
            .await
            .get_unused_indexes(threshold_secs)
    }

    /// Get lock contention summary
    pub async fn get_contention_summary(&self) -> ContentionSummary {
        let tracker = self.lock_contention_tracker.read().await;
        ContentionSummary {
            total_contentions: tracker.total_contentions(),
            avg_wait_time_ms: tracker.avg_wait_time_ms(),
            hot_resources: tracker.get_hot_resources(10),
            contentions_by_type: tracker
                .contentions_by_type()
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
        }
    }

    /// Get query execution summary
    pub async fn get_execution_summary(&self) -> ExecutionSummary {
        let stats = self.query_execution_stats.read().await;
        ExecutionSummary {
            total_queries: stats.total_queries(),
            failed_queries: stats.failed_queries(),
            error_rate: stats.error_rate(),
            slowest_patterns: stats.get_slowest_patterns(10),
            most_frequent_patterns: stats.get_most_frequent_patterns(10),
        }
    }

    /// Normalize query for pattern matching
    fn normalize_query(&self, query: &str) -> String {
        // Simplified normalization: replace literals with placeholders
        let normalized = query
            .to_lowercase()
            .split_whitespace()
            .map(|word| {
                if word.parse::<i64>().is_ok()
                    || word.parse::<f64>().is_ok()
                    || (word.starts_with('\'') && word.ends_with('\''))
                {
                    "?"
                } else {
                    word
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Truncate if too long
        if normalized.len() > 200 {
            format!("{}...", &normalized[..200])
        } else {
            normalized
        }
    }

    /// Extract table name from query
    fn extract_table_from_query(&self, query: &str) -> String {
        let query_lower = query.to_lowercase();
        if let Some(from_pos) = query_lower.find("from ") {
            let after_from = &query_lower[from_pos + 5..];
            let table_name = after_from
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c| c == '(' || c == ')' || c == ',');
            if !table_name.is_empty() {
                return table_name.to_string();
            }
        }
        "unknown".to_string()
    }
}

/// Lock contention summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentionSummary {
    pub total_contentions: u64,
    pub avg_wait_time_ms: f64,
    pub hot_resources: Vec<(String, u64)>,
    pub contentions_by_type: HashMap<String, u64>,
}

/// Query execution summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub total_queries: u64,
    pub failed_queries: u64,
    pub error_rate: f64,
    pub slowest_patterns: Vec<QueryHistogram>,
    pub most_frequent_patterns: Vec<QueryHistogram>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_slow_query_logging() {
        let config = MonitoringConfig {
            slow_query_threshold_ms: 50,
            max_slow_queries: 100,
            ..Default::default()
        };

        let metrics = AdvancedQueryMetrics::new(config);

        // Record a slow query
        metrics
            .record_query_execution(QueryExecutionParams {
                query_id: 1,
                query_text: "SELECT * FROM users WHERE id = 1".to_string(),
                execution_time: Duration::from_millis(100),
                rows_examined: 1000,
                rows_returned: 100,
                index_used: Some("idx_users_id".to_string()),
                lock_wait_time: Duration::from_millis(10),
                execution_plan: Some("SeqScan on users".to_string()),
                user: "admin".to_string(),
                success: true,
            })
            .await;

        let slow_queries = metrics.get_slow_queries().await;
        assert_eq!(slow_queries.len(), 1);
        assert_eq!(slow_queries[0].query_id, 1);
    }

    #[tokio::test]
    async fn test_index_usage_tracking() {
        let metrics = AdvancedQueryMetrics::new(MonitoringConfig::default());

        // Record index usage
        metrics.index_usage_stats.write().await.record_index_scan(
            "idx_users_id".to_string(),
            "users".to_string(),
            100,
        );

        let stats = metrics.get_index_stats().await;
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].index_name, "idx_users_id");
        assert_eq!(stats[0].total_scans, 1);
    }

    #[tokio::test]
    async fn test_lock_contention_tracking() {
        let metrics = AdvancedQueryMetrics::new(MonitoringConfig::default());

        // Record contention
        metrics
            .record_lock_contention(
                1,
                "users_table".to_string(),
                LockType::Exclusive,
                Duration::from_millis(50),
                Some(2),
            )
            .await;

        let summary = metrics.get_contention_summary().await;
        assert_eq!(summary.total_contentions, 1);
        assert!(summary.avg_wait_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_query_histogram() {
        let mut histogram = QueryHistogram::new("SELECT * FROM users WHERE id = ?".to_string());

        histogram.add_execution(10, 1000);
        histogram.add_execution(20, 1000);
        histogram.add_execution(30, 1000);
        histogram.add_execution(40, 1000);
        histogram.add_execution(50, 1000);

        assert_eq!(histogram.count, 5);
        assert_eq!(histogram.min_time_ms, 10);
        assert_eq!(histogram.max_time_ms, 50);
        assert_eq!(histogram.avg_time_ms, 30.0);
    }

    #[tokio::test]
    async fn test_unused_indexes() {
        let metrics = AdvancedQueryMetrics::new(MonitoringConfig::default());

        // Add an index that was used a long time ago
        let mut stats = metrics.index_usage_stats.write().await;
        stats.record_index_scan("old_index".to_string(), "users".to_string(), 100);

        // Manually set last_used to a very old timestamp
        if let Some(entry) = stats.indexes.get_mut("old_index") {
            entry.last_used = 0; // Set to epoch
        }

        drop(stats); // Release lock

        // Get unused indexes (threshold: 1 second)
        let unused = metrics.get_unused_indexes(1).await;
        assert!(!unused.is_empty());
    }
}
