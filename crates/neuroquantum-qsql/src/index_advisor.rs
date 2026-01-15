//! Index Advisor for Automatic Index Recommendations
//!
//! This module provides automatic index recommendations based on query pattern analysis.
//! It tracks query patterns, identifies full table scans, and generates index recommendations
//! with estimated performance improvements.
//!
//! ## Features
//!
//! - **Query Pattern Tracking**: Tracks WHERE clauses, JOIN conditions, and ORDER BY columns
//! - **Frequency Analysis**: Identifies frequently accessed columns

// Index advisor uses expect for mutex locks which should not fail in normal operation.
#![allow(clippy::expect_used)]
//! - **Recommendation Engine**: Generates single-column and composite index recommendations
//! - **Impact Estimation**: Estimates performance improvement from recommended indexes

use crate::ast::{Expression, FromClause, JoinClause, OrderByItem, SelectStatement, Statement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{debug, info};

/// Statistics for a column access pattern
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ColumnStats {
    /// Column name
    pub column_name: String,
    /// Table name
    pub table_name: String,
    /// Number of times used in WHERE clauses
    pub where_count: u64,
    /// Number of times used in JOIN conditions
    pub join_count: u64,
    /// Number of times used in ORDER BY clauses
    pub order_by_count: u64,
    /// Number of times used in GROUP BY clauses
    pub group_by_count: u64,
    /// Most common operators used (e.g., =, >, <, LIKE)
    pub operators: HashMap<String, u64>,
    /// Last access timestamp (Unix ms)
    pub last_access_ms: u64,
}

/// Table-level statistics for query analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TableStats {
    /// Table name
    pub table_name: String,
    /// Total number of queries accessing this table
    pub query_count: u64,
    /// Number of full table scans detected
    pub full_scan_count: u64,
    /// Number of indexed lookups
    pub index_lookup_count: u64,
    /// Columns accessed in this table
    pub columns: HashMap<String, ColumnStats>,
    /// Existing indexes on this table
    pub existing_indexes: Vec<String>,
    /// Last access timestamp (Unix ms)
    pub last_access_ms: u64,
}

/// Priority level for index recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationPriority {
    /// Critical: High-frequency queries with no index support
    Critical,
    /// High: Frequent queries that would benefit significantly
    High,
    /// Medium: Moderate frequency or moderate benefit
    Medium,
    /// Low: Infrequent queries or marginal benefit
    Low,
}

impl std::fmt::Display for RecommendationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::Critical => write!(f, "CRITICAL"),
            | Self::High => write!(f, "HIGH"),
            | Self::Medium => write!(f, "MEDIUM"),
            | Self::Low => write!(f, "LOW"),
        }
    }
}

/// Type of index recommended
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    /// B-tree index (default, good for equality and range queries)
    BTree,
    /// Hash index (good for equality queries only)
    Hash,
    /// Covering index (includes additional columns to avoid table lookups)
    Covering { include_columns: Vec<String> },
    /// Composite index (multi-column)
    Composite,
}

impl std::fmt::Display for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::BTree => write!(f, "BTREE"),
            | Self::Hash => write!(f, "HASH"),
            | Self::Covering { .. } => write!(f, "COVERING"),
            | Self::Composite => write!(f, "COMPOSITE"),
        }
    }
}

/// An index recommendation with impact estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecommendation {
    /// Unique recommendation ID
    pub id: String,
    /// Table name
    pub table_name: String,
    /// Columns to index (in order for composite indexes)
    pub columns: Vec<String>,
    /// Type of index recommended
    pub index_type: IndexType,
    /// Priority level
    pub priority: RecommendationPriority,
    /// Estimated performance improvement (0.0 to 1.0, where 1.0 = 100% improvement)
    pub estimated_improvement: f64,
    /// Number of queries that would benefit
    pub affected_query_count: u64,
    /// SQL statement to create the index
    pub create_statement: String,
    /// Reason for the recommendation
    pub reason: String,
    /// Estimated index size in bytes (approximate)
    pub estimated_size_bytes: u64,
}

/// Configuration for the Index Advisor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexAdvisorConfig {
    /// Minimum query count before recommending an index
    pub min_query_threshold: u64,
    /// Minimum estimated improvement to recommend (0.0 to 1.0)
    pub min_improvement_threshold: f64,
    /// Maximum number of recommendations to return
    pub max_recommendations: usize,
    /// Whether to track query patterns (can be disabled for performance)
    pub enable_tracking: bool,
    /// Maximum number of tables to track
    pub max_tracked_tables: usize,
    /// Maximum number of columns per table to track
    pub max_tracked_columns_per_table: usize,
}

impl Default for IndexAdvisorConfig {
    fn default() -> Self {
        Self {
            min_query_threshold: 10,
            min_improvement_threshold: 0.1, // 10% improvement
            max_recommendations: 20,
            enable_tracking: true,
            max_tracked_tables: 100,
            max_tracked_columns_per_table: 50,
        }
    }
}

/// Index Advisor for automatic index recommendations
///
/// This struct collects query statistics and generates index recommendations
/// based on observed query patterns.
pub struct IndexAdvisor {
    config: IndexAdvisorConfig,
    /// Table statistics (`table_name` -> `TableStats`)
    table_stats: Arc<RwLock<HashMap<String, TableStats>>>,
    /// Total queries analyzed
    total_queries: AtomicU64,
    /// Queries that triggered full table scans
    full_scan_queries: AtomicU64,
}

impl IndexAdvisor {
    /// Create a new Index Advisor with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(IndexAdvisorConfig::default())
    }

    /// Create a new Index Advisor with custom configuration
    #[must_use]
    pub fn with_config(config: IndexAdvisorConfig) -> Self {
        Self {
            config,
            table_stats: Arc::new(RwLock::new(HashMap::new())),
            total_queries: AtomicU64::new(0),
            full_scan_queries: AtomicU64::new(0),
        }
    }

    /// Track a query for pattern analysis
    ///
    /// This method analyzes the query AST and updates internal statistics
    /// about column usage patterns.
    pub fn track_query(&self, statement: &Statement) {
        if !self.config.enable_tracking {
            return;
        }

        self.total_queries.fetch_add(1, Ordering::Relaxed);

        match statement {
            | Statement::Select(select) => self.track_select(select),
            | Statement::Update(update) => {
                // Track WHERE clause from UPDATE
                if let Some(ref where_clause) = update.where_clause {
                    self.track_expression_columns(where_clause, &update.table_name, "WHERE");
                }
            },
            | Statement::Delete(delete) => {
                // Track WHERE clause from DELETE
                if let Some(ref where_clause) = delete.where_clause {
                    self.track_expression_columns(where_clause, &delete.table_name, "WHERE");
                }
            },
            | _ => {}, // Other statement types don't need index recommendations
        }
    }

    /// Track a SELECT statement
    fn track_select(&self, select: &SelectStatement) {
        let now_ms = Instant::now().elapsed().as_millis() as u64;

        // Get table name from FROM clause
        let table_name = if let Some(ref from) = select.from {
            self.get_primary_table_name(from)
        } else {
            return; // No FROM clause, nothing to track
        };

        // Track WHERE clause columns
        if let Some(ref where_clause) = select.where_clause {
            self.track_expression_columns(where_clause, &table_name, "WHERE");
        } else {
            // No WHERE clause = potential full table scan
            self.full_scan_queries.fetch_add(1, Ordering::Relaxed);
            self.increment_full_scan(&table_name);
        }

        // Track JOIN conditions
        if let Some(ref from) = select.from {
            for join in &from.joins {
                self.track_join(join, &table_name);
            }
        }

        // Track ORDER BY columns
        for order_item in &select.order_by {
            self.track_order_by(order_item, &table_name, now_ms);
        }

        // Track GROUP BY columns
        for group_expr in &select.group_by {
            self.track_expression_columns(group_expr, &table_name, "GROUP_BY");
        }
    }

    /// Get the primary table name from a FROM clause
    fn get_primary_table_name(&self, from: &FromClause) -> String {
        if let Some(first_relation) = from.relations.first() {
            first_relation
                .alias
                .clone()
                .unwrap_or(first_relation.name.clone())
        } else {
            "unknown".to_string()
        }
    }

    /// Track columns used in an expression
    fn track_expression_columns(&self, expr: &Expression, table_name: &str, context: &str) {
        match expr {
            | Expression::Identifier(col_name) => {
                self.increment_column_usage(table_name, col_name, context, None);
            },
            | Expression::BinaryOp {
                left,
                operator,
                right,
            } => {
                // Track the operator used
                let op_str = format!("{operator:?}");

                // Left side is often the column
                if let Expression::Identifier(col_name) = left.as_ref() {
                    self.increment_column_usage(table_name, col_name, context, Some(&op_str));
                }

                // Recurse for nested expressions
                self.track_expression_columns(left, table_name, context);
                self.track_expression_columns(right, table_name, context);
            },
            | Expression::FunctionCall { args, .. } => {
                for arg in args {
                    self.track_expression_columns(arg, table_name, context);
                }
            },
            | Expression::InList { expr, list, .. } => {
                self.track_expression_columns(expr, table_name, context);
                for item in list {
                    self.track_expression_columns(item, table_name, context);
                }
            },
            | _ => {}, // Other expression types
        }
    }

    /// Track a JOIN clause
    fn track_join(&self, join: &JoinClause, primary_table: &str) {
        if let Some(ref condition) = join.condition {
            // Track the join table
            let join_table = join
                .relation
                .alias
                .clone()
                .unwrap_or(join.relation.name.clone());

            self.track_expression_columns(condition, primary_table, "JOIN");
            self.track_expression_columns(condition, &join_table, "JOIN");
        }
    }

    /// Track an ORDER BY item
    fn track_order_by(&self, order_item: &OrderByItem, table_name: &str, now_ms: u64) {
        if let Expression::Identifier(col_name) = &order_item.expression {
            self.increment_column_usage(table_name, col_name, "ORDER_BY", None);
        }
        // Update last access
        let _ = now_ms; // Will be used when updating stats
    }

    /// Increment usage count for a column
    fn increment_column_usage(
        &self,
        table_name: &str,
        column_name: &str,
        context: &str,
        operator: Option<&str>,
    ) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let mut stats = self
            .table_stats
            .write()
            .expect("table_stats RwLock poisoned");

        // Check table limit
        if !stats.contains_key(table_name) && stats.len() >= self.config.max_tracked_tables {
            return;
        }

        let table_stats = stats
            .entry(table_name.to_string())
            .or_insert_with(|| TableStats {
                table_name: table_name.to_string(),
                ..Default::default()
            });

        table_stats.query_count += 1;
        table_stats.last_access_ms = now_ms;

        // Check column limit
        if !table_stats.columns.contains_key(column_name)
            && table_stats.columns.len() >= self.config.max_tracked_columns_per_table
        {
            return;
        }

        let col_stats = table_stats
            .columns
            .entry(column_name.to_string())
            .or_insert_with(|| ColumnStats {
                column_name: column_name.to_string(),
                table_name: table_name.to_string(),
                ..Default::default()
            });

        col_stats.last_access_ms = now_ms;

        match context {
            | "WHERE" => col_stats.where_count += 1,
            | "JOIN" => col_stats.join_count += 1,
            | "ORDER_BY" => col_stats.order_by_count += 1,
            | "GROUP_BY" => col_stats.group_by_count += 1,
            | _ => {},
        }

        if let Some(op) = operator {
            *col_stats.operators.entry(op.to_string()).or_insert(0) += 1;
        }
    }

    /// Increment full scan count for a table
    fn increment_full_scan(&self, table_name: &str) {
        let mut stats = self
            .table_stats
            .write()
            .expect("table_stats RwLock poisoned");
        if let Some(table_stats) = stats.get_mut(table_name) {
            table_stats.full_scan_count += 1;
        }
    }

    /// Register an existing index on a table
    pub fn register_existing_index(&self, table_name: &str, index_columns: &[String]) {
        let mut stats = self
            .table_stats
            .write()
            .expect("table_stats RwLock poisoned");
        let table_stats = stats
            .entry(table_name.to_string())
            .or_insert_with(|| TableStats {
                table_name: table_name.to_string(),
                ..Default::default()
            });

        let index_desc = index_columns.join(",");
        if !table_stats.existing_indexes.contains(&index_desc) {
            table_stats.existing_indexes.push(index_desc);
        }
    }

    /// Generate index recommendations based on collected statistics
    pub fn get_recommendations(&self) -> Vec<IndexRecommendation> {
        let stats = self
            .table_stats
            .read()
            .expect("table_stats RwLock poisoned");
        let mut recommendations = Vec::new();

        for (table_name, table_stats) in stats.iter() {
            // Skip tables with few queries
            if table_stats.query_count < self.config.min_query_threshold {
                continue;
            }

            // Analyze columns for potential indexes
            let mut column_scores: Vec<(&String, &ColumnStats, f64)> = table_stats
                .columns
                .iter()
                .map(|(col_name, col_stats)| {
                    let score = self.calculate_column_score(col_stats, table_stats);
                    (col_name, col_stats, score)
                })
                .filter(|(_, _, score)| *score > 0.0)
                .collect();

            // Sort by score descending
            column_scores
                .sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

            // Generate recommendations for top columns
            for (col_name, col_stats, score) in column_scores.iter().take(5) {
                // Check if index already exists for this column
                if self.has_existing_index(table_stats, &[(*col_name).clone()]) {
                    continue;
                }

                let recommendation = self.create_recommendation(
                    table_name,
                    vec![(*col_name).clone()],
                    col_stats,
                    *score,
                    table_stats,
                );

                if recommendation.estimated_improvement >= self.config.min_improvement_threshold {
                    recommendations.push(recommendation);
                }
            }

            // Look for composite index opportunities
            if let Some(composite_rec) =
                self.find_composite_index_opportunity(table_name, table_stats)
            {
                recommendations.push(composite_rec);
            }
        }

        // Sort by priority and improvement
        recommendations.sort_by(|a, b| {
            let priority_cmp = (a.priority as u8).cmp(&(b.priority as u8));
            if priority_cmp == std::cmp::Ordering::Equal {
                b.estimated_improvement
                    .partial_cmp(&a.estimated_improvement)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                priority_cmp
            }
        });

        // Limit recommendations
        recommendations.truncate(self.config.max_recommendations);

        info!(
            "Generated {} index recommendations from {} tables",
            recommendations.len(),
            stats.len()
        );

        recommendations
    }

    /// Calculate a score for a column based on its usage patterns
    fn calculate_column_score(&self, col_stats: &ColumnStats, table_stats: &TableStats) -> f64 {
        let total_usage = col_stats.where_count
            + col_stats.join_count * 2 // JOINs are weighted higher
            + col_stats.order_by_count
            + col_stats.group_by_count;

        if total_usage == 0 {
            return 0.0;
        }

        // Base score from usage frequency
        let frequency_score = (total_usage as f64) / (table_stats.query_count as f64).max(1.0);

        // Boost for equality operators (better index utilization)
        let equality_boost = col_stats.operators.get("Equal").copied().unwrap_or(0) as f64 * 0.1;

        // Boost for JOIN columns (often critical for performance)
        let join_boost = if col_stats.join_count > 0 { 0.3 } else { 0.0 };

        // Penalty for LIKE with leading wildcard (less useful for B-tree indexes)
        let like_penalty = col_stats.operators.get("Like").copied().unwrap_or(0) as f64 * 0.05;

        (frequency_score + equality_boost + join_boost - like_penalty).max(0.0)
    }

    /// Check if an index already exists for given columns
    fn has_existing_index(&self, table_stats: &TableStats, columns: &[String]) -> bool {
        let columns_str = columns.join(",");
        table_stats
            .existing_indexes
            .iter()
            .any(|idx| idx == &columns_str || idx.starts_with(&format!("{columns_str},")))
    }

    /// Create an index recommendation
    fn create_recommendation(
        &self,
        table_name: &str,
        columns: Vec<String>,
        col_stats: &ColumnStats,
        score: f64,
        table_stats: &TableStats,
    ) -> IndexRecommendation {
        let index_name = format!("idx_{}_{}_advisor", table_name, columns.join("_"));

        let columns_sql = columns.join(", ");
        let create_statement = format!("CREATE INDEX {index_name} ON {table_name} ({columns_sql})");

        // Estimate improvement based on full scan ratio and column usage
        let full_scan_ratio = if table_stats.query_count > 0 {
            table_stats.full_scan_count as f64 / table_stats.query_count as f64
        } else {
            0.0
        };

        // Higher score = more improvement expected
        let estimated_improvement = score.mul_add(0.5, full_scan_ratio * 0.3).min(0.95);

        let priority = if estimated_improvement > 0.7 {
            RecommendationPriority::Critical
        } else if estimated_improvement > 0.4 {
            RecommendationPriority::High
        } else if estimated_improvement > 0.2 {
            RecommendationPriority::Medium
        } else {
            RecommendationPriority::Low
        };

        let affected_query_count =
            col_stats.where_count + col_stats.join_count + col_stats.order_by_count;

        // Determine index type - B-tree is most versatile for general use
        // (could use Hash for purely equality operations, but B-tree handles ranges too)
        let index_type = IndexType::BTree;

        let reason = self.generate_reason(col_stats, table_stats);

        // Rough size estimate (assuming 8 bytes per indexed value + overhead)
        let estimated_size_bytes = table_stats.query_count * 16; // Very rough estimate

        // Generate unique ID using timestamp and column info
        let id = format!(
            "idx_{}_{}_{:x}",
            table_name,
            columns.join("_"),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0)
        );

        IndexRecommendation {
            id,
            table_name: table_name.to_string(),
            columns,
            index_type,
            priority,
            estimated_improvement,
            affected_query_count,
            create_statement,
            reason,
            estimated_size_bytes,
        }
    }

    /// Generate a human-readable reason for the recommendation
    fn generate_reason(&self, col_stats: &ColumnStats, table_stats: &TableStats) -> String {
        let mut reasons = Vec::new();

        if col_stats.where_count > 0 {
            reasons.push(format!(
                "Used in WHERE clause {} times",
                col_stats.where_count
            ));
        }

        if col_stats.join_count > 0 {
            reasons.push(format!(
                "Used in JOIN conditions {} times",
                col_stats.join_count
            ));
        }

        if col_stats.order_by_count > 0 {
            reasons.push(format!(
                "Used in ORDER BY {} times",
                col_stats.order_by_count
            ));
        }

        if table_stats.full_scan_count > 0 {
            reasons.push(format!(
                "Table has {} full scans",
                table_stats.full_scan_count
            ));
        }

        if reasons.is_empty() {
            "Column frequently accessed".to_string()
        } else {
            reasons.join("; ")
        }
    }

    /// Find composite index opportunities (columns often used together)
    fn find_composite_index_opportunity(
        &self,
        table_name: &str,
        table_stats: &TableStats,
    ) -> Option<IndexRecommendation> {
        // Find columns frequently used in JOINs and WHERE together
        let mut join_columns: Vec<&String> = table_stats
            .columns
            .iter()
            .filter(|(_, stats)| stats.join_count > 0)
            .map(|(name, _)| name)
            .collect();

        let mut where_columns: Vec<&String> = table_stats
            .columns
            .iter()
            .filter(|(_, stats)| stats.where_count > self.config.min_query_threshold / 2)
            .map(|(name, _)| name)
            .collect();

        // If we have both JOIN and WHERE columns, suggest composite
        if !join_columns.is_empty() && !where_columns.is_empty() {
            join_columns.sort();
            where_columns.sort();

            // Take first JOIN column + first WHERE column
            let composite_columns = vec![
                join_columns[0].clone(),
                where_columns
                    .iter()
                    .find(|c| **c != join_columns[0])
                    .map_or_else(|| where_columns[0].clone(), |c| (*c).clone()),
            ];

            // Don't recommend if it's the same column twice
            if composite_columns[0] == composite_columns[1] {
                return None;
            }

            // Check if composite index already exists
            if self.has_existing_index(table_stats, &composite_columns) {
                return None;
            }

            let index_name = format!(
                "idx_{}_{}_composite_advisor",
                table_name,
                composite_columns.join("_")
            );

            let columns_sql = composite_columns.join(", ");
            let create_statement =
                format!("CREATE INDEX {index_name} ON {table_name} ({columns_sql})");

            // Generate unique ID for composite index recommendation
            let id = format!(
                "idx_{}_composite_{:x}",
                table_name,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0)
            );

            return Some(IndexRecommendation {
                id,
                table_name: table_name.to_string(),
                columns: composite_columns,
                index_type: IndexType::Composite,
                priority: RecommendationPriority::High,
                estimated_improvement: 0.5,
                affected_query_count: table_stats.query_count,
                create_statement,
                reason: "Columns frequently used together in JOINs and WHERE clauses".to_string(),
                estimated_size_bytes: table_stats.query_count * 24,
            });
        }

        None
    }

    /// Get current statistics summary
    pub fn get_statistics(&self) -> IndexAdvisorStatistics {
        let stats = self
            .table_stats
            .read()
            .expect("table_stats RwLock poisoned");

        let mut total_columns_tracked = 0;
        for table_stats in stats.values() {
            total_columns_tracked += table_stats.columns.len();
        }

        IndexAdvisorStatistics {
            total_queries_analyzed: self.total_queries.load(Ordering::Relaxed),
            full_scan_queries: self.full_scan_queries.load(Ordering::Relaxed),
            tables_tracked: stats.len(),
            columns_tracked: total_columns_tracked,
        }
    }

    /// Clear all collected statistics
    pub fn clear_statistics(&self) {
        let mut stats = self
            .table_stats
            .write()
            .expect("table_stats RwLock poisoned");
        stats.clear();
        self.total_queries.store(0, Ordering::Relaxed);
        self.full_scan_queries.store(0, Ordering::Relaxed);
        debug!("Index advisor statistics cleared");
    }

    /// Get table statistics for a specific table
    pub fn get_table_stats(&self, table_name: &str) -> Option<TableStats> {
        let stats = self
            .table_stats
            .read()
            .expect("table_stats RwLock poisoned");
        stats.get(table_name).cloned()
    }
}

impl Default for IndexAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics from the Index Advisor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexAdvisorStatistics {
    /// Total number of queries analyzed
    pub total_queries_analyzed: u64,
    /// Number of queries that resulted in full table scans
    pub full_scan_queries: u64,
    /// Number of tables being tracked
    pub tables_tracked: usize,
    /// Total number of columns being tracked across all tables
    pub columns_tracked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn create_test_select(table: &str, where_column: Option<&str>) -> Statement {
        let where_clause = where_column.map(|col| Expression::BinaryOp {
            left: Box::new(Expression::Identifier(col.to_string())),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Literal(Literal::Integer(1))),
        });

        Statement::Select(SelectStatement {
            select_list: vec![SelectItem::Wildcard],
            from: Some(FromClause {
                relations: vec![TableReference {
                    name: table.to_string(),
                    alias: None,
                    synaptic_weight: None,
                    quantum_state: None,
                    subquery: None,
                }],
                joins: vec![],
            }),
            where_clause,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            synaptic_weight: None,
            plasticity_threshold: None,
            neuromatch_clause: None,
            quantum_parallel: false,
            grover_iterations: None,
            with_clause: None,
            union_clause: None,
        })
    }

    #[test]
    fn test_index_advisor_creation() {
        let advisor = IndexAdvisor::new();
        let stats = advisor.get_statistics();
        assert_eq!(stats.total_queries_analyzed, 0);
        assert_eq!(stats.tables_tracked, 0);
    }

    #[test]
    fn test_track_simple_select() {
        let advisor = IndexAdvisor::new();
        let query = create_test_select("users", Some("id"));

        advisor.track_query(&query);

        let stats = advisor.get_statistics();
        assert_eq!(stats.total_queries_analyzed, 1);
        assert_eq!(stats.tables_tracked, 1);

        let table_stats = advisor.get_table_stats("users").unwrap();
        // query_count is incremented per column access, not per query
        assert!(table_stats.query_count >= 1);
        assert!(table_stats.columns.contains_key("id"));
    }

    #[test]
    fn test_track_full_table_scan() {
        let advisor = IndexAdvisor::new();
        let query = create_test_select("users", None); // No WHERE clause

        advisor.track_query(&query);

        let stats = advisor.get_statistics();
        assert_eq!(stats.full_scan_queries, 1);
    }

    #[test]
    fn test_generate_recommendations() {
        let config = IndexAdvisorConfig {
            min_query_threshold: 2, // Lower threshold for testing
            ..Default::default()
        };

        let advisor = IndexAdvisor::with_config(config);

        // Track multiple queries on the same column
        for _ in 0..5 {
            let query = create_test_select("users", Some("email"));
            advisor.track_query(&query);
        }

        let recommendations = advisor.get_recommendations();

        // Should recommend an index on email column
        assert!(!recommendations.is_empty());
        assert!(recommendations
            .iter()
            .any(|r| r.table_name == "users" && r.columns.contains(&"email".to_string())));
    }

    #[test]
    fn test_existing_index_not_recommended() {
        let config = IndexAdvisorConfig {
            min_query_threshold: 2,
            ..Default::default()
        };

        let advisor = IndexAdvisor::with_config(config);

        // Register existing index
        advisor.register_existing_index("users", &["email".to_string()]);

        // Track queries
        for _ in 0..5 {
            let query = create_test_select("users", Some("email"));
            advisor.track_query(&query);
        }

        let recommendations = advisor.get_recommendations();

        // Should NOT recommend index on email since it already exists
        assert!(!recommendations
            .iter()
            .any(|r| r.table_name == "users" && r.columns.len() == 1 && r.columns[0] == "email"));
    }

    #[test]
    fn test_clear_statistics() {
        let advisor = IndexAdvisor::new();

        for _ in 0..3 {
            let query = create_test_select("users", Some("id"));
            advisor.track_query(&query);
        }

        assert!(advisor.get_statistics().total_queries_analyzed > 0);

        advisor.clear_statistics();

        let stats = advisor.get_statistics();
        assert_eq!(stats.total_queries_analyzed, 0);
        assert_eq!(stats.tables_tracked, 0);
    }

    #[test]
    fn test_recommendation_priority() {
        let config = IndexAdvisorConfig {
            min_query_threshold: 1,
            ..Default::default()
        };

        let advisor = IndexAdvisor::with_config(config);

        // Track many queries to create high-priority recommendation
        for _ in 0..100 {
            let query = create_test_select("orders", Some("customer_id"));
            advisor.track_query(&query);
        }

        let recommendations = advisor.get_recommendations();

        assert!(!recommendations.is_empty());
        // First recommendation should have high priority due to frequency
        assert!(matches!(
            recommendations[0].priority,
            RecommendationPriority::Critical | RecommendationPriority::High
        ));
    }

    #[test]
    fn test_create_statement_format() {
        let config = IndexAdvisorConfig {
            min_query_threshold: 1,
            ..Default::default()
        };

        let advisor = IndexAdvisor::with_config(config);

        for _ in 0..5 {
            let query = create_test_select("products", Some("category"));
            advisor.track_query(&query);
        }

        let recommendations = advisor.get_recommendations();

        if let Some(rec) = recommendations.first() {
            assert!(rec.create_statement.starts_with("CREATE INDEX"));
            assert!(rec.create_statement.contains("products"));
            assert!(rec.create_statement.contains("category"));
        }
    }

    #[test]
    fn test_tracking_disabled() {
        let config = IndexAdvisorConfig {
            enable_tracking: false,
            ..Default::default()
        };

        let advisor = IndexAdvisor::with_config(config);

        for _ in 0..10 {
            let query = create_test_select("users", Some("id"));
            advisor.track_query(&query);
        }

        let stats = advisor.get_statistics();
        // Tracking is disabled, so nothing should be recorded
        assert_eq!(stats.tables_tracked, 0);
    }
}
