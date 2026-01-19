#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::too_many_lines,
    clippy::no_effect_underscore_binding,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unused_async,
    clippy::needless_pass_by_ref_mut,
    clippy::similar_names,
    clippy::module_name_repetitions,
    clippy::struct_excessive_bools,
    clippy::single_match_else,
    clippy::if_not_else,
    clippy::match_same_arms,
    clippy::unnested_or_patterns,
    clippy::items_after_statements,
    clippy::redundant_else,
    clippy::match_wildcard_for_single_variants,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::doc_markdown,
    clippy::format_push_string,
    clippy::option_if_let_else,
    clippy::significant_drop_tightening,
    clippy::cognitive_complexity,
    clippy::default_trait_access,
    clippy::return_self_not_must_use,
    clippy::manual_string_new,
    clippy::trivially_copy_pass_by_ref,
    clippy::wildcard_imports,
    clippy::unreadable_literal,
    clippy::struct_field_names,
    clippy::enum_glob_use,
    clippy::ref_option,
    clippy::assigning_clones,
    clippy::needless_pass_by_value,
    clippy::manual_let_else,
    clippy::or_fun_call,
    clippy::if_then_some_else_none,
    clippy::collection_is_never_read,
    clippy::too_long_first_doc_paragraph,
    clippy::useless_let_if_seq
)]
//! # `NeuroQuantum` QSQL Language Implementation
//!
//! This crate provides a brain-inspired query language (QSQL) for `NeuroQuantumDB`
//! that extends SQL with neuromorphic computing capabilities, quantum-inspired
//! optimizations, and natural language processing.
//!
//! ## Features
//!
//! - **Brain-inspired syntax**: Neuromorphic extensions like `NEUROMATCH`, `QUANTUM_JOIN`
//! - **SQL compatibility**: Full backward compatibility with standard SQL
//! - **Natural language processing**: Query understanding and translation
//! - **Neuromorphic optimization**: Synaptic pathway-based query planning
//! - **Quantum-inspired execution**: Grover's search and superposition processing
//! - **ARM64 optimizations**: NEON-SIMD accelerated parsing and execution

pub mod ast;
pub mod error;
pub mod executor;
pub mod explain;
pub mod index_advisor;
pub mod natural_language;
pub mod optimizer;
pub mod parser;
pub mod prepared_statements;
pub mod query_plan;
pub mod query_plan_cache;

// SQL Engine Integration Tests
#[cfg(test)]
pub mod sql_engine_tests;

// Comprehensive test suite
#[cfg(test)]
mod tests;

// Property-based tests for parser robustness
#[cfg(test)]
mod proptest_suite;

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
// Import types from modules to avoid duplicates
use optimizer::{NeuromorphicOptimizer, OptimizerConfig};
// Re-export key types for external use (avoid conflicts)
pub use parser::QSQLParser as Parser;
use parser::{ParserConfig, QSQLParser as ParserQSQLParser};
// Internal use
use query_plan::{ExecutionStrategy, OptimizationMetadata, QueryPlan};
pub use query_plan::{ExecutorConfig, QueryExecutor, QueryResult};
use query_plan_cache::{CachedQueryPlan, QueryPlanCache, QueryPlanCacheConfig};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

// Use the QueryPlan from query_plan module (what the executor expects)
// pub use query_plan::QueryPlan; // Commented out to avoid duplicate definition

/// Main QSQL engine that coordinates parsing, optimization, and execution
pub struct QSQLEngine {
    parser: ParserQSQLParser,
    /// Neuromorphic optimizer for query optimization (reserved for future use)
    #[allow(dead_code)]
    optimizer: NeuromorphicOptimizer,
    executor: QueryExecutor,
    cache: QueryPlanCache,
    metrics: QSQLMetrics,
    /// Index Advisor for automatic index recommendations
    index_advisor: index_advisor::IndexAdvisor,
}

// CachedQueryPlan is now defined in query_plan_cache module

/// Performance metrics for QSQL operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QSQLMetrics {
    pub queries_parsed: u64,
    pub queries_optimized: u64,
    pub queries_executed: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub neuromorphic_optimizations: u64,
    pub quantum_operations: u64,
    pub natural_language_queries: u64,
    pub average_parse_time: Duration,
    pub average_optimization_time: Duration,
    pub average_execution_time: Duration,
}

impl QSQLEngine {
    /// Create new QSQL engine with default configuration
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            parser: ParserQSQLParser::new(),
            optimizer: NeuromorphicOptimizer::new()?,
            executor: QueryExecutor::new()?,
            cache: QueryPlanCache::new(),
            metrics: QSQLMetrics::default(),
            index_advisor: index_advisor::IndexAdvisor::new(),
        })
    }

    /// Create a QSQL engine with custom configuration
    pub fn with_config(config: QSQLConfig) -> Result<Self> {
        let cache_config = QueryPlanCacheConfig {
            max_entries: config.cache_size,
            ..QueryPlanCacheConfig::default()
        };
        Ok(Self {
            parser: ParserQSQLParser::with_config(config.parser_config)?,
            optimizer: NeuromorphicOptimizer::with_config(config.optimizer_config)?,
            executor: QueryExecutor::with_config(config.executor_config)?,
            cache: QueryPlanCache::with_config(cache_config),
            metrics: QSQLMetrics::default(),
            index_advisor: index_advisor::IndexAdvisor::new(),
        })
    }

    /// Create a QSQL engine with a storage engine for production use.
    /// This ensures queries are executed against the actual storage instead of
    /// returning simulated data.
    ///
    /// Note: Uses `Arc<RwLock<StorageEngine>>` for thread-safe shared access.
    pub fn with_storage(
        storage_engine: Arc<tokio::sync::RwLock<neuroquantum_core::storage::StorageEngine>>,
    ) -> anyhow::Result<Self> {
        let config = ExecutorConfig::default();
        let executor = QueryExecutor::with_storage(config, storage_engine)?;
        Ok(Self {
            parser: ParserQSQLParser::new(),
            optimizer: NeuromorphicOptimizer::new()?,
            executor,
            cache: QueryPlanCache::new(),
            metrics: QSQLMetrics::default(),
            index_advisor: index_advisor::IndexAdvisor::new(),
        })
    }

    /// Set the storage engine for an existing QSQL engine.
    /// This enables production mode query execution against the actual storage.
    pub fn set_storage_engine(
        &mut self,
        storage_engine: Arc<tokio::sync::RwLock<neuroquantum_core::storage::StorageEngine>>,
    ) {
        self.executor.set_storage_engine(storage_engine);
    }

    /// Check if the engine has a storage engine configured for production use.
    pub const fn has_storage_engine(&self) -> bool {
        self.executor.has_storage_engine()
    }

    /// Execute a query with full pipeline processing
    #[instrument(skip(self, query))]
    pub async fn execute_query(&mut self, query: &str) -> Result<QueryResult, anyhow::Error> {
        let start_time = Instant::now();

        // Check for empty or obviously invalid queries first
        if query.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty query"));
        }

        // Check cache first
        if let Some(cached_plan) = self.cache.get(query) {
            self.metrics.cache_hits += 1;

            // Clone the plan to avoid borrowing issues
            let plan_clone = cached_plan.plan.clone();
            let _execution_count = cached_plan.execution_count;

            // Use the cached plan execution method
            let exec_start = Instant::now();
            let result = self.execute_cached_plan(&plan_clone).await?;
            let exec_duration = exec_start.elapsed();

            // Update cached plan statistics after execution
            if let Some(cached_plan) = self.cache.get_mut(query) {
                cached_plan.record_execution(exec_duration);
            }

            self.metrics.average_execution_time = Self::update_average(
                self.metrics.average_execution_time,
                exec_duration,
                self.metrics.queries_executed,
            );
            self.metrics.queries_executed += 1;

            debug!("Query executed from cache in {:?}", start_time.elapsed());
            return Ok(result);
        }

        self.metrics.cache_misses += 1;

        // Parse query - convert parsing errors to anyhow errors for proper propagation
        let parse_start = Instant::now();
        let ast = self
            .parser
            .parse_query(query)
            .map_err(|e| anyhow::anyhow!("Parse error: {e}"))?;
        self.metrics.average_parse_time = Self::update_average(
            self.metrics.average_parse_time,
            parse_start.elapsed(),
            self.metrics.queries_parsed,
        );
        self.metrics.queries_parsed += 1;

        // Track query for index advisor
        self.index_advisor.track_query(&ast);

        // Create a simple query plan directly from AST (bypassing optimizer for now)
        let plan = Arc::new(QueryPlan {
            statement: Arc::new(ast),
            execution_strategy: ExecutionStrategy::Sequential,
            synaptic_pathways: vec![],
            quantum_optimizations: vec![],
            estimated_cost: 100.0,
            optimization_metadata: OptimizationMetadata {
                optimization_time: Duration::from_millis(1),
                iterations_used: 1,
                convergence_achieved: true,
                synaptic_adaptations: 0,
                quantum_optimizations_applied: 0,
            },
        });

        // Execute query
        let exec_start = Instant::now();
        let result = self
            .executor
            .execute(&plan)
            .await
            .map_err(|e| anyhow::anyhow!("Execution error: {e}"))?;
        let exec_duration = exec_start.elapsed();

        self.metrics.average_execution_time = Self::update_average(
            self.metrics.average_execution_time,
            exec_duration,
            self.metrics.queries_executed,
        );
        self.metrics.queries_executed += 1;

        // Cache successful plan
        self.cache_plan(query.to_string(), plan, exec_duration);

        debug!("Query executed in {:?}", start_time.elapsed());
        Ok(result)
    }

    /// Execute a natural language query
    #[instrument(skip(self, natural_query))]
    pub async fn execute_natural_query(&mut self, natural_query: &str) -> Result<QueryResult> {
        self.metrics.natural_language_queries += 1;

        // Convert natural language to QSQL
        let qsql_query = self.parser.natural_language_to_qsql(natural_query)?;
        info!("Translated natural language query to QSQL: {}", qsql_query);

        // Execute the generated QSQL
        self.execute_query(&qsql_query).await
    }

    /// Get current performance metrics
    pub const fn metrics(&self) -> &QSQLMetrics {
        &self.metrics
    }

    /// Reset performance metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = QSQLMetrics::default();
    }

    /// Optimize synaptic pathways based on usage patterns
    #[instrument(skip(self))]
    pub fn optimize_synaptic_pathways(&mut self) -> Result<()> {
        // The new QueryPlanCache handles synaptic strengthening automatically
        // via the record_execution method and Hebbian-inspired eviction
        info!(
            "Synaptic pathway optimization: {} cached plans, {} bytes used",
            self.cache.len(),
            self.cache.current_memory_bytes()
        );
        self.metrics.neuromorphic_optimizations += 1;
        Ok(())
    }

    // =====================================================================
    // Index Advisor Methods
    // =====================================================================

    /// Get index recommendations based on collected query patterns
    ///
    /// Returns a list of recommended indexes ordered by priority.
    /// Each recommendation includes the CREATE INDEX SQL statement.
    pub fn get_index_recommendations(&self) -> Vec<index_advisor::IndexRecommendation> {
        self.index_advisor.get_recommendations()
    }

    /// Get index advisor statistics
    ///
    /// Returns statistics about tracked queries, tables, and columns.
    pub fn get_index_advisor_statistics(&self) -> index_advisor::IndexAdvisorStatistics {
        self.index_advisor.get_statistics()
    }

    /// Register an existing index so it won't be recommended again
    pub fn register_existing_index(&self, table_name: &str, columns: &[String]) {
        self.index_advisor
            .register_existing_index(table_name, columns);
    }

    /// Clear index advisor statistics
    pub fn clear_index_advisor_statistics(&self) {
        self.index_advisor.clear_statistics();
    }

    /// Get statistics for a specific table
    pub fn get_table_index_stats(&self, table_name: &str) -> Option<index_advisor::TableStats> {
        self.index_advisor.get_table_stats(table_name)
    }

    /// Get a reference to the index advisor for advanced usage
    pub const fn index_advisor(&self) -> &index_advisor::IndexAdvisor {
        &self.index_advisor
    }

    // =====================================================================
    // Query Plan Cache Methods
    // =====================================================================

    /// Get cache statistics for monitoring
    pub const fn cache_statistics(&self) -> &query_plan_cache::CacheStatistics {
        self.cache.statistics()
    }

    /// Get current cache memory usage in bytes
    pub const fn cache_memory_bytes(&self) -> usize {
        self.cache.current_memory_bytes()
    }

    /// Get current number of cached plans
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Clear the query plan cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Manually trigger cache eviction to reduce memory to target
    pub fn evict_cache_to_memory(&mut self, target_bytes: usize) {
        self.cache.evict_to_target_memory(target_bytes);
    }

    // Private helper methods

    async fn execute_cached_plan(&mut self, plan: &Arc<QueryPlan>) -> Result<QueryResult> {
        self.executor
            .execute(plan)
            .await
            .map_err(std::convert::Into::into)
    }

    fn cache_plan(&mut self, query: String, plan: Arc<QueryPlan>, duration: Duration) {
        let cached = CachedQueryPlan::new(plan, duration);
        self.cache.insert(query, cached);
    }

    const fn update_average(current: Duration, new: Duration, count: u64) -> Duration {
        if count == 0 {
            new
        } else {
            Duration::from_nanos(
                (current.as_nanos() as u64 * count + new.as_nanos() as u64) / (count + 1),
            )
        }
    }
}

impl Default for QSQLEngine {
    fn default() -> Self {
        match Self::new() {
            | Ok(engine) => engine,
            | Err(_) => {
                // Fallback to a minimal engine if creation fails
                Self {
                    parser: ParserQSQLParser::default(),
                    optimizer: NeuromorphicOptimizer::default(),
                    executor: QueryExecutor::default(),
                    cache: QueryPlanCache::new(),
                    metrics: QSQLMetrics::default(),
                    index_advisor: index_advisor::IndexAdvisor::new(),
                }
            },
        }
    }
}

/// Configuration for QSQL engine components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QSQLConfig {
    pub parser_config: ParserConfig,
    pub optimizer_config: OptimizerConfig,
    pub executor_config: ExecutorConfig,
    pub cache_size: usize,
    pub enable_natural_language: bool,
    pub enable_quantum_optimization: bool,
    pub synaptic_learning_rate: f32,
}

impl Default for QSQLConfig {
    fn default() -> Self {
        Self {
            parser_config: ParserConfig::default(),
            optimizer_config: OptimizerConfig::default(),
            executor_config: ExecutorConfig::default(),
            cache_size: 1000,
            enable_natural_language: true,
            enable_quantum_optimization: true,
            synaptic_learning_rate: 0.01,
        }
    }
}

impl QSQLConfig {
    /// Create a testing configuration that uses mock/simulated storage
    /// instead of requiring a real storage engine.
    #[cfg(test)]
    #[must_use]
    pub fn testing() -> Self {
        Self {
            parser_config: ParserConfig::default(),
            optimizer_config: OptimizerConfig::default(),
            executor_config: ExecutorConfig::testing(),
            cache_size: 100,
            enable_natural_language: true,
            enable_quantum_optimization: false,
            synaptic_learning_rate: 0.01,
        }
    }
}

// Public API exports
pub use ast::*;
pub use error::*;
pub use index_advisor::{
    ColumnStats, IndexAdvisor, IndexAdvisorConfig, IndexAdvisorStatistics, IndexRecommendation,
    IndexType, RecommendationPriority, TableStats,
};
pub use natural_language::NaturalLanguageProcessor;
pub use prepared_statements::{
    PreparedStatement, PreparedStatementManager, PreparedStatementStats,
    PreparedStatementsStatistics,
};
pub use query_plan_cache::CacheStatistics;
