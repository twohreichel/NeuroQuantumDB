//! # NeuroQuantum QSQL Language Implementation
//!
//! This crate provides a brain-inspired query language (QSQL) for NeuroQuantumDB
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
pub mod natural_language;
pub mod optimizer;
pub mod parser;
pub mod query_plan;

// SQL Engine Integration Tests
#[cfg(test)]
pub mod sql_engine_tests;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, instrument, warn};

// Import types from modules to avoid duplicates
use optimizer::{NeuromorphicOptimizer, OptimizerConfig};
use parser::{ParserConfig, QSQLParser as ParserQSQLParser};
use query_plan::{
    ExecutionStrategy, ExecutorConfig, OptimizationMetadata, QueryExecutor, QueryPlan, QueryResult,
};

// Use the QueryPlan from query_plan module (what the executor expects)
// pub use query_plan::QueryPlan; // Commented out to avoid duplicate definition

/// Main QSQL engine that coordinates parsing, optimization, and execution
pub struct QSQLEngine {
    parser: ParserQSQLParser,
    optimizer: NeuromorphicOptimizer,
    executor: QueryExecutor,
    cache: HashMap<String, CachedQueryPlan>,
    metrics: QSQLMetrics,
}

/// Cached query plan with execution statistics
#[derive(Debug, Clone, Serialize)]
pub struct CachedQueryPlan {
    pub plan: QueryPlan,
    pub execution_count: u64,
    pub average_duration: Duration,
    pub synaptic_strength: f32,
    #[serde(skip)]
    pub last_accessed: Instant,
}

impl<'de> Deserialize<'de> for CachedQueryPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CachedQueryPlanHelper {
            plan: QueryPlan,
            execution_count: u64,
            average_duration: Duration,
            synaptic_strength: f32,
        }

        let helper = CachedQueryPlanHelper::deserialize(deserializer)?;
        Ok(CachedQueryPlan {
            plan: helper.plan,
            execution_count: helper.execution_count,
            average_duration: helper.average_duration,
            synaptic_strength: helper.synaptic_strength,
            last_accessed: Instant::now(),
        })
    }
}

impl Default for CachedQueryPlan {
    fn default() -> Self {
        use ast::{SelectStatement, Statement};

        Self {
            plan: QueryPlan {
                statement: Statement::Select(SelectStatement {
                    select_list: vec![],
                    from: None,
                    where_clause: None,
                    group_by: vec![],
                    having: None,
                    order_by: vec![],
                    limit: None,
                    offset: None,
                    synaptic_weight: None,
                    plasticity_threshold: None,
                    quantum_parallel: false,
                    grover_iterations: None,
                }),
                execution_strategy: ExecutionStrategy::Sequential,
                synaptic_pathways: vec![],
                quantum_optimizations: vec![],
                estimated_cost: 0.0,
                optimization_metadata: OptimizationMetadata {
                    optimization_time: Duration::from_millis(0),
                    iterations_used: 0,
                    convergence_achieved: false,
                    synaptic_adaptations: 0,
                    quantum_optimizations_applied: 0,
                },
            },
            execution_count: 0,
            average_duration: Duration::from_millis(0),
            synaptic_strength: 0.0,
            last_accessed: Instant::now(),
        }
    }
}

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
    /// Create a new QSQL engine with default configuration
    pub fn new() -> Result<Self> {
        Ok(Self {
            parser: ParserQSQLParser::new(),
            optimizer: NeuromorphicOptimizer::new()?,
            executor: QueryExecutor::new()?,
            cache: HashMap::new(),
            metrics: QSQLMetrics::default(),
        })
    }

    /// Create a QSQL engine with custom configuration
    pub fn with_config(config: QSQLConfig) -> Result<Self> {
        Ok(Self {
            parser: ParserQSQLParser::with_config(config.parser_config)?,
            optimizer: NeuromorphicOptimizer::with_config(config.optimizer_config)?,
            executor: QueryExecutor::with_config(config.executor_config)?,
            cache: HashMap::with_capacity(config.cache_size),
            metrics: QSQLMetrics::default(),
        })
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
            let execution_count = cached_plan.execution_count;

            // Use the cached plan execution method
            let exec_start = Instant::now();
            let result = self.execute_cached_plan(&plan_clone).await?;
            let exec_duration = exec_start.elapsed();

            // Update cached plan statistics after execution
            if let Some(cached_plan) = self.cache.get_mut(query) {
                cached_plan.execution_count += 1;
                cached_plan.last_accessed = Instant::now();
                cached_plan.average_duration = Self::update_average(
                    cached_plan.average_duration,
                    exec_duration,
                    execution_count,
                );
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
            .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
        self.metrics.average_parse_time = Self::update_average(
            self.metrics.average_parse_time,
            parse_start.elapsed(),
            self.metrics.queries_parsed,
        );
        self.metrics.queries_parsed += 1;

        // Create a simple query plan directly from AST (bypassing optimizer for now)
        let plan = QueryPlan {
            statement: ast,
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
        };

        // Execute query
        let exec_start = Instant::now();
        let result = self
            .executor
            .execute(&plan)
            .await
            .map_err(|e| anyhow::anyhow!("Execution error: {}", e))?;
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
    pub fn metrics(&self) -> &QSQLMetrics {
        &self.metrics
    }

    /// Reset performance metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = QSQLMetrics::default();
    }

    /// Optimize synaptic pathways based on usage patterns
    #[instrument(skip(self))]
    pub fn optimize_synaptic_pathways(&mut self) -> Result<()> {
        // Strengthen frequently used query patterns
        for cached_plan in self.cache.values_mut() {
            if cached_plan.execution_count > 10 {
                cached_plan.synaptic_strength = (cached_plan.synaptic_strength * 1.1).min(1.0);
                debug!("Strengthened synaptic pathway for query pattern");
            }
        }

        // Update optimizer with learned patterns
        self.optimizer.update_synaptic_weights(&self.cache)?;
        self.metrics.neuromorphic_optimizations += 1;

        Ok(())
    }

    // Private helper methods

    async fn execute_cached_plan(&mut self, plan: &QueryPlan) -> Result<QueryResult> {
        self.executor.execute(plan).await.map_err(|e| e.into())
    }

    fn cache_plan(&mut self, query: String, plan: QueryPlan, duration: Duration) {
        let cached = CachedQueryPlan {
            plan,
            execution_count: 1,
            average_duration: duration,
            synaptic_strength: 0.5, // Initial synaptic strength
            last_accessed: Instant::now(),
        };

        self.cache.insert(query, cached);
    }

    fn update_average(current: Duration, new: Duration, count: u64) -> Duration {
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
            Ok(engine) => engine,
            Err(_) => {
                // Fallback to a minimal engine if creation fails
                QSQLEngine {
                    parser: ParserQSQLParser::default(),
                    optimizer: NeuromorphicOptimizer::default(),
                    executor: QueryExecutor::default(),
                    cache: HashMap::new(),
                    metrics: QSQLMetrics::default(),
                }
            }
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

// Public API exports
pub use ast::*;
pub use error::*;
pub use natural_language::NaturalLanguageProcessor;
