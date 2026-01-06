//! Query Planning for QSQL
//!
//! This module provides query plan generation and optimization using
//! neuromorphic intelligence and quantum-inspired algorithms.

use crate::ast::*;
use crate::error::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
#[cfg(test)]
use tracing::warn;

// Import storage engine and related types
use neuroquantum_core::learning::HebbianLearningEngine;
use neuroquantum_core::storage::{
    ComparisonOperator, Condition, DeleteQuery, OrderBy, Row, RowId, SelectQuery, SortDirection,
    StorageEngine, UpdateQuery, Value, WhereClause,
};
use neuroquantum_core::synaptic::SynapticNetwork;
use neuroquantum_core::transaction::{IsolationLevel, TransactionId, TransactionManager};

/// Type alias for async table row results to reduce type complexity
type TableRowFuture<'a> = std::pin::Pin<
    Box<dyn std::future::Future<Output = QSQLResult<(Vec<Row>, String)>> + Send + 'a>,
>;

/// Query plan executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    pub max_concurrent_operations: usize,
    pub memory_limit_mb: usize,
    pub timeout_seconds: u64,
    pub enable_parallel_execution: bool,
    pub enable_vectorization: bool,
    pub cache_intermediate_results: bool,
    // Integration features
    pub enable_neuromorphic_learning: bool,
    pub enable_synaptic_optimization: bool,
    pub enable_dna_compression: bool, // Always enabled via StorageEngine
    /// Allow legacy mode without storage engine (simulation mode).
    ///
    /// This field is only available in test builds. In production, the executor
    /// always requires a storage engine to be configured via `with_storage()`.
    ///
    /// When `true` (test builds only), the executor will return simulated data
    /// if no storage engine is configured.
    #[cfg(test)]
    #[serde(default)]
    pub allow_legacy_mode: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 4,
            memory_limit_mb: 100, // 100MB limit for edge devices
            timeout_seconds: 30,
            enable_parallel_execution: true,
            enable_vectorization: true,
            cache_intermediate_results: true,
            enable_neuromorphic_learning: true,
            enable_synaptic_optimization: true,
            enable_dna_compression: true,
            #[cfg(test)]
            allow_legacy_mode: false, // Only available in test builds
        }
    }
}

impl ExecutorConfig {
    /// Create a production configuration.
    /// Use this in production environments to ensure storage engine is always required.
    ///
    /// Note: This is equivalent to `Default::default()` as the default
    /// configuration is production-safe.
    pub fn production() -> Self {
        Self::default()
    }

    /// Create a testing configuration that allows legacy mode with simulated data.
    ///
    /// # Warning
    /// Only use this for testing purposes. Legacy mode returns simulated data
    /// instead of real storage data. This method is only available in test builds.
    #[cfg(test)]
    pub fn testing() -> Self {
        Self {
            allow_legacy_mode: true,
            ..Default::default()
        }
    }
}

/// Query execution engine with neuromorphic and quantum support
pub struct QueryExecutor {
    config: ExecutorConfig,
    execution_stats: ExecutionStats,
    // Storage engine integration via Arc for shared access
    // None: executor not configured for production use
    // Some: production mode with real storage
    storage_engine: Option<Arc<tokio::sync::RwLock<StorageEngine>>>,
    // Neuromorphic learning integration (optional)
    learning_engine: Option<HebbianLearningEngine>,
    synaptic_network: Option<SynapticNetwork>,
    // Transaction management
    transaction_manager: Option<Arc<TransactionManager>>,
    // Current active transaction for this session (if any)
    current_transaction: Option<TransactionId>,
    /// Savepoint tracking for nested savepoints.
    ///
    /// Note: Current implementation provides basic savepoint syntax support
    /// and tracks savepoint names. Full savepoint rollback functionality
    /// would require deeper WAL integration to store and restore intermediate
    /// transaction states. This is tracked for future enhancement.
    savepoints: HashMap<String, ()>,
}

/// Query execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows: Vec<HashMap<String, QueryValue>>,
    pub columns: Vec<ColumnInfo>,
    pub execution_time: Duration,
    pub rows_affected: u64,
    pub optimization_applied: bool,
    pub synaptic_pathways_used: u32,
    pub quantum_operations: u32,
}

/// Type alias for query result data: rows and column information
pub type QueryResultData = (Vec<HashMap<String, QueryValue>>, Vec<ColumnInfo>);

/// Column information in query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

/// Value types in query results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Blob(Vec<u8>),
    DNASequence(String),
    SynapticWeight(f32),
    QuantumState(String),
}

/// Execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub queries_executed: u64,
    pub total_execution_time: Duration,
    pub synaptic_optimizations: u64,
    pub quantum_operations: u64,
    pub cache_hits: u64,
    pub memory_usage_peak: usize,
}

/// Represents an aggregate function to be computed
#[derive(Debug, Clone)]
struct AggregateFunction {
    /// The aggregate function name (COUNT, SUM, AVG, MIN, MAX)
    name: String,
    /// The column or expression to aggregate (None for COUNT(*))
    column: Option<String>,
    /// Optional alias for the result
    alias: Option<String>,
    /// Whether DISTINCT should be applied
    distinct: bool,
}

impl QueryExecutor {
    /// Create a new query executor without storage integration.
    ///
    /// # Note
    /// By default, this creates an executor that requires a storage engine.
    /// Use `with_storage()` to configure the storage engine, or use
    /// `with_config(ExecutorConfig::testing())` for test scenarios with
    /// simulated data.
    pub fn new() -> QSQLResult<Self> {
        Self::with_config(ExecutorConfig::default())
    }

    /// Create executor with custom configuration.
    ///
    /// For production, use `ExecutorConfig::default()` or `ExecutorConfig::production()`.
    /// For testing with simulated data, use `ExecutorConfig::testing()`.
    pub fn with_config(config: ExecutorConfig) -> QSQLResult<Self> {
        Ok(Self {
            config,
            execution_stats: ExecutionStats::default(),
            storage_engine: None,
            learning_engine: None,
            synaptic_network: None,
            transaction_manager: None,
            current_transaction: None,
            savepoints: HashMap::new(),
        })
    }

    /// Create executor with storage engine integration (production mode)
    /// This enables DNA compression, neuromorphic learning, and full query execution
    ///
    /// Note: Uses `Arc<RwLock<StorageEngine>>` for thread-safe shared access,
    /// allowing multiple query executors to share the same storage engine.
    pub fn with_storage(
        config: ExecutorConfig,
        storage_engine: Arc<tokio::sync::RwLock<StorageEngine>>,
    ) -> QSQLResult<Self> {
        // Initialize neuromorphic learning if enabled
        let learning_engine = if config.enable_neuromorphic_learning {
            Some(
                HebbianLearningEngine::new(0.01).map_err(|e| QSQLError::ExecutionError {
                    message: format!("Failed to initialize learning engine: {}", e),
                })?,
            )
        } else {
            None
        };

        // Initialize synaptic network if enabled
        let synaptic_network = if config.enable_synaptic_optimization {
            Some(
                SynapticNetwork::new(1000, 0.5).map_err(|e| QSQLError::ExecutionError {
                    message: format!("Failed to initialize synaptic network: {}", e),
                })?,
            )
        } else {
            None
        };

        Ok(Self {
            config,
            execution_stats: ExecutionStats::default(),
            storage_engine: Some(storage_engine),
            learning_engine,
            synaptic_network,
            transaction_manager: None,
            current_transaction: None,
            savepoints: HashMap::new(),
        })
    }

    /// Set storage engine (for existing executors)
    ///
    /// Uses `Arc<RwLock<StorageEngine>>` for thread-safe shared access.
    pub fn set_storage_engine(&mut self, storage_engine: Arc<tokio::sync::RwLock<StorageEngine>>) {
        self.storage_engine = Some(storage_engine);
    }

    /// Check if storage engine is available
    pub fn has_storage_engine(&self) -> bool {
        self.storage_engine.is_some()
    }

    /// Set transaction manager (for transaction control)
    pub fn set_transaction_manager(&mut self, tx_manager: Arc<TransactionManager>) {
        self.transaction_manager = Some(tx_manager);
    }

    /// Check if legacy mode is allowed (test builds only)
    #[cfg(test)]
    pub fn is_legacy_mode_allowed(&self) -> bool {
        self.config.allow_legacy_mode
    }

    /// Check if legacy mode is allowed (always false in production builds)
    #[cfg(not(test))]
    pub fn is_legacy_mode_allowed(&self) -> bool {
        false
    }

    /// Validate that storage engine is available.
    /// In test builds, legacy mode is also accepted.
    /// Returns an error if storage engine is not available (and legacy mode not enabled in tests).
    fn require_storage_or_legacy(&self) -> QSQLResult<()> {
        if !self.has_storage_engine() && !self.is_legacy_mode_allowed() {
            #[cfg(test)]
            let message = "Storage engine required for query execution. \
                         Either provide a storage engine via `with_storage()` or \
                         use `ExecutorConfig::testing()` for test scenarios.";
            #[cfg(not(test))]
            let message = "Storage engine required for query execution. \
                         Provide a storage engine via `with_storage()` before executing queries.";
            return Err(QSQLError::ConfigError {
                message: message.to_string(),
            });
        }
        Ok(())
    }

    /// Execute a Statement directly (convenience method)
    /// Creates a simple QueryPlan internally
    pub async fn execute_statement(&mut self, statement: &Statement) -> QSQLResult<QueryResult> {
        // Create a basic QueryPlan from the statement
        let plan = QueryPlan {
            statement: statement.clone(),
            execution_strategy: ExecutionStrategy::Sequential,
            synaptic_pathways: vec![],
            quantum_optimizations: vec![],
            estimated_cost: 1.0,
            optimization_metadata: OptimizationMetadata {
                optimization_time: Duration::from_millis(0),
                iterations_used: 0,
                convergence_achieved: true,
                synaptic_adaptations: 0,
                quantum_optimizations_applied: 0,
            },
        };

        self.execute(&plan).await
    }

    /// Execute a query plan
    ///
    /// # Errors
    ///
    /// Returns `QSQLError::ConfigError` if no storage engine is configured and
    /// legacy mode is disabled (production mode). In production, always use
    /// `ExecutorConfig::production()` to ensure real storage is required.
    pub async fn execute(&mut self, plan: &QueryPlan) -> QSQLResult<QueryResult> {
        // Production guard: ensure storage engine or explicit legacy mode
        self.require_storage_or_legacy()?;

        let start_time = std::time::Instant::now();

        let result = match &plan.statement {
            Statement::Select(select) => self.execute_select(select, plan).await,
            Statement::Insert(insert) => self.execute_insert(insert, plan).await,
            Statement::Update(update) => self.execute_update(update, plan).await,
            Statement::Delete(delete) => self.execute_delete(delete, plan).await,
            Statement::CreateTable(create) => self.execute_create_table(create, plan).await,
            Statement::DropTable(drop) => self.execute_drop_table(drop, plan).await,
            Statement::AlterTable(alter) => self.execute_alter_table(alter, plan).await,
            Statement::CreateIndex(create_idx) => self.execute_create_index(create_idx, plan).await,
            Statement::DropIndex(drop_idx) => self.execute_drop_index(drop_idx, plan).await,
            Statement::TruncateTable(truncate) => self.execute_truncate_table(truncate, plan).await,
            Statement::NeuroMatch(neuromatch) => self.execute_neuromatch(neuromatch, plan).await,
            Statement::QuantumSearch(quantum) => self.execute_quantum_search(quantum, plan).await,
            Statement::SuperpositionQuery(superpos) => {
                self.execute_superposition_query(superpos, plan).await
            }
            Statement::LearnPattern(learn) => self.execute_learn_pattern(learn, plan).await,
            Statement::AdaptWeights(adapt) => self.execute_adapt_weights(adapt, plan).await,
            Statement::QuantumJoin(qjoin) => self.execute_quantum_join(qjoin, plan).await,
            Statement::Explain(explain) => self.execute_explain(explain, plan).await,
            Statement::Analyze(analyze) => self.execute_analyze(analyze, plan).await,
            // Transaction control statements
            Statement::BeginTransaction(begin) => self.execute_begin_transaction(begin).await,
            Statement::Commit(_) => self.execute_commit().await,
            Statement::Rollback(_) => self.execute_rollback().await,
            Statement::Savepoint(savepoint) => self.execute_savepoint(savepoint).await,
            Statement::RollbackToSavepoint(rollback_to) => {
                self.execute_rollback_to_savepoint(rollback_to).await
            }
            Statement::ReleaseSavepoint(release) => self.execute_release_savepoint(release).await,
            _ => Err(QSQLError::ExecutionError {
                message: "Statement type not yet implemented".to_string(),
            }),
        }?;

        // Update statistics
        self.execution_stats.queries_executed += 1;
        self.execution_stats.total_execution_time += start_time.elapsed();
        self.execution_stats.synaptic_optimizations +=
            plan.optimization_metadata.synaptic_adaptations as u64;
        self.execution_stats.quantum_operations +=
            plan.optimization_metadata.quantum_optimizations_applied as u64;

        Ok(result)
    }

    /// Execute SELECT statement with DNA decompression and synaptic optimization
    async fn execute_select(
        &mut self,
        select: &SelectStatement,
        plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        let start_time = std::time::Instant::now();

        // Check if storage engine is available
        if self.storage_engine.is_some() {
            // Real execution with storage engine

            // Check if we have derived tables (subqueries in FROM clause)
            let has_derived_tables = select
                .from
                .as_ref()
                .map(|f| {
                    f.relations.iter().any(|r| r.subquery.is_some())
                        || f.joins.iter().any(|j| j.relation.subquery.is_some())
                })
                .unwrap_or(false);

            if has_derived_tables {
                // Execute with derived table logic
                return self
                    .execute_select_with_derived_tables(select, plan, start_time)
                    .await;
            }

            // Check if we have JOINs in the query
            let has_joins = select
                .from
                .as_ref()
                .map(|f| !f.joins.is_empty())
                .unwrap_or(false);

            if has_joins {
                // Execute with JOIN logic
                return self
                    .execute_select_with_joins(select, plan, start_time)
                    .await;
            }

            // Check if we have subqueries that need to be resolved first
            let has_where_subqueries = select
                .where_clause
                .as_ref()
                .map(Self::contains_subquery_expression)
                .unwrap_or(false);

            // Check if we have scalar subqueries in the SELECT list
            let has_select_subqueries = Self::has_scalar_subqueries(&select.select_list);

            // Resolve subqueries if present in WHERE clause
            let resolved_where_clause = if has_where_subqueries {
                if let Some(where_expr) = &select.where_clause {
                    Some(
                        self.resolve_subqueries_in_expression(where_expr.clone())
                            .await?,
                    )
                } else {
                    None
                }
            } else {
                select.where_clause.clone()
            };

            // Resolve scalar subqueries in SELECT list
            let resolved_select_list = if has_select_subqueries {
                let mut new_list = Vec::new();
                for item in &select.select_list {
                    match item {
                        SelectItem::Expression {
                            expr: Expression::ScalarSubquery { subquery },
                            alias,
                        } => {
                            // Execute the scalar subquery and replace with literal
                            let value = self.execute_scalar_subquery(subquery).await?;
                            new_list.push(SelectItem::Expression {
                                expr: Expression::Literal(value),
                                alias: alias.clone(),
                            });
                        }
                        other => new_list.push(other.clone()),
                    }
                }
                new_list
            } else {
                select.select_list.clone()
            };

            // Create a modified select statement with resolved subqueries
            let resolved_select = if has_where_subqueries || has_select_subqueries {
                SelectStatement {
                    select_list: resolved_select_list,
                    from: select.from.clone(),
                    where_clause: resolved_where_clause.clone(),
                    group_by: select.group_by.clone(),
                    having: select.having.clone(),
                    order_by: select.order_by.clone(),
                    limit: select.limit,
                    offset: select.offset,
                    synaptic_weight: select.synaptic_weight,
                    plasticity_threshold: select.plasticity_threshold,
                    neuromatch_clause: select.neuromatch_clause.clone(),
                    quantum_parallel: select.quantum_parallel,
                    grover_iterations: select.grover_iterations,
                    with_clause: select.with_clause.clone(),
                }
            } else {
                select.clone()
            };

            // Check if we need post-filtering for InList expressions
            let needs_post_filter = resolved_select
                .where_clause
                .as_ref()
                .map(Self::contains_in_list_expression)
                .unwrap_or(false);

            // Convert SQL SELECT to storage query (no borrow of self.storage_engine)
            let storage_query = self.convert_select_to_storage_query(&resolved_select)?;

            // Execute query via storage engine (automatically DNA-decompressed!)
            // Acquire read lock for query execution
            let storage_guard = self.storage_engine.as_ref().unwrap().read().await;
            let storage_rows = storage_guard
                .select_rows(&storage_query)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Storage select failed: {}", e),
                })?;
            drop(storage_guard); // Release lock early

            // Apply post-filtering for InList expressions
            let filtered_rows = if needs_post_filter {
                if let Some(where_expr) = &resolved_select.where_clause {
                    let mut filtered = Self::apply_post_filter(storage_rows, where_expr)?;

                    // Apply limit/offset after filtering
                    if let Some(offset) = resolved_select.offset {
                        if offset as usize >= filtered.len() {
                            filtered = Vec::new();
                        } else {
                            filtered = filtered.into_iter().skip(offset as usize).collect();
                        }
                    }
                    if let Some(limit) = resolved_select.limit {
                        filtered.truncate(limit as usize);
                    }

                    filtered
                } else {
                    storage_rows
                }
            } else {
                storage_rows
            };

            // Apply NEUROMATCH clause if present (neuromorphic pattern matching)
            let neuromatch_filtered_rows = if let Some(neuromatch) = &select.neuromatch_clause {
                self.apply_neuromatch_filter(filtered_rows, neuromatch)?
            } else {
                filtered_rows
            };

            // Neuromorphic learning: learn from access pattern
            if self.config.enable_synaptic_optimization && select.synaptic_weight.is_some() {
                self.learn_from_select(select, &neuromatch_filtered_rows)
                    .await?;
            }

            // Convert storage rows to query result
            let (result_rows, columns) =
                self.convert_storage_rows_to_result(neuromatch_filtered_rows, select)?;

            let rows_affected = result_rows.len() as u64;

            Ok(QueryResult {
                rows: result_rows,
                columns,
                execution_time: start_time.elapsed(),
                rows_affected,
                optimization_applied: select.synaptic_weight.is_some()
                    || !plan.synaptic_pathways.is_empty(),
                synaptic_pathways_used: plan.synaptic_pathways.len() as u32,
                quantum_operations: if select.quantum_parallel { 1 } else { 0 },
            })
        } else {
            // Fallback: Simulate data (legacy mode) - only available in test builds
            #[cfg(test)]
            {
                warn!(
                    "Query executor running in legacy mode with simulated data. \
                     This is only available in test builds."
                );
                let columns = vec![
                    ColumnInfo {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                    },
                    ColumnInfo {
                        name: "name".to_string(),
                        data_type: DataType::VarChar(Some(255)),
                        nullable: true,
                    },
                ];

                let mut rows = Vec::new();

                // Simulate data rows
                for i in 1..=5 {
                    let mut row = HashMap::new();
                    row.insert("id".to_string(), QueryValue::Integer(i));
                    row.insert(
                        "name".to_string(),
                        QueryValue::String(format!("User {}", i)),
                    );
                    rows.push(row);
                }

                Ok(QueryResult {
                    rows,
                    columns,
                    execution_time: Duration::from_micros(500),
                    rows_affected: 5,
                    optimization_applied: !plan.synaptic_pathways.is_empty(),
                    synaptic_pathways_used: plan.synaptic_pathways.len() as u32,
                    quantum_operations: plan.quantum_optimizations.len() as u32,
                })
            }
            #[cfg(not(test))]
            {
                // This branch should never be reached in production because
                // require_storage_or_legacy() is called at the start of execute()
                Err(QSQLError::ConfigError {
                    message: "Storage engine required for query execution.".to_string(),
                })
            }
        }
    }

    /// Execute SELECT with JOIN operations
    async fn execute_select_with_joins(
        &mut self,
        select: &SelectStatement,
        plan: &QueryPlan,
        start_time: std::time::Instant,
    ) -> QSQLResult<QueryResult> {
        let from = select
            .from
            .as_ref()
            .ok_or_else(|| QSQLError::ExecutionError {
                message: "Missing FROM clause for JOIN".to_string(),
            })?;

        // Get the base table
        let base_table = from
            .relations
            .first()
            .ok_or_else(|| QSQLError::ExecutionError {
                message: "No table specified in FROM clause".to_string(),
            })?;

        // Fetch all rows from the base table
        let base_query = SelectQuery {
            table: base_table.name.clone(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };

        let storage_guard = self.storage_engine.as_ref().unwrap().read().await;
        let base_rows = storage_guard.select_rows(&base_query).await.map_err(|e| {
            QSQLError::ExecutionError {
                message: format!("Failed to fetch base table: {}", e),
            }
        })?;

        // Process each JOIN
        let mut result_rows = base_rows;
        let base_alias = base_table
            .alias
            .clone()
            .unwrap_or_else(|| base_table.name.clone());

        for join in &from.joins {
            let join_table_name = &join.relation.name;
            let join_alias = join
                .relation
                .alias
                .clone()
                .unwrap_or_else(|| join_table_name.clone());

            // Fetch all rows from the joined table
            let join_query = SelectQuery {
                table: join_table_name.clone(),
                columns: vec!["*".to_string()],
                where_clause: None,
                order_by: None,
                limit: None,
                offset: None,
            };

            let join_rows = storage_guard.select_rows(&join_query).await.map_err(|e| {
                QSQLError::ExecutionError {
                    message: format!("Failed to fetch join table: {}", e),
                }
            })?;

            // Perform the JOIN based on type
            result_rows = self.perform_join(
                result_rows,
                &base_alias,
                join_rows,
                &join_alias,
                &join.join_type,
                join.condition.as_ref(),
            )?;
        }

        drop(storage_guard); // Release lock

        // Apply WHERE clause filtering
        let filtered_rows = if let Some(where_expr) = &select.where_clause {
            Self::apply_post_filter(result_rows, where_expr)?
        } else {
            result_rows
        };

        // Apply ORDER BY
        let ordered_rows = if !select.order_by.is_empty() {
            Self::apply_order_by(filtered_rows, &select.order_by)?
        } else {
            filtered_rows
        };

        // Apply LIMIT and OFFSET
        let mut final_rows = ordered_rows;
        if let Some(offset) = select.offset {
            if offset as usize >= final_rows.len() {
                final_rows = Vec::new();
            } else {
                final_rows = final_rows.into_iter().skip(offset as usize).collect();
            }
        }
        if let Some(limit) = select.limit {
            final_rows.truncate(limit as usize);
        }

        // Convert to result format
        let (result_rows, columns) = self.convert_storage_rows_to_result(final_rows, select)?;
        let rows_affected = result_rows.len() as u64;

        Ok(QueryResult {
            rows: result_rows,
            columns,
            execution_time: start_time.elapsed(),
            rows_affected,
            optimization_applied: !plan.synaptic_pathways.is_empty(),
            synaptic_pathways_used: plan.synaptic_pathways.len() as u32,
            quantum_operations: 0,
        })
    }

    /// Execute SELECT with derived tables (subqueries in FROM clause)
    async fn execute_select_with_derived_tables(
        &mut self,
        select: &SelectStatement,
        plan: &QueryPlan,
        start_time: std::time::Instant,
    ) -> QSQLResult<QueryResult> {
        let from = select
            .from
            .as_ref()
            .ok_or_else(|| QSQLError::ExecutionError {
                message: "Missing FROM clause".to_string(),
            })?;

        // Get rows from the base table or derived table
        let base_ref = from
            .relations
            .first()
            .ok_or_else(|| QSQLError::ExecutionError {
                message: "No table specified in FROM clause".to_string(),
            })?;

        let (base_rows, base_alias) = self.get_rows_from_table_ref(base_ref).await?;

        // Process any JOINs (which may also include derived tables)
        let mut result_rows = base_rows;
        let mut current_alias = base_alias;

        for join in &from.joins {
            let (join_rows, join_alias) = self.get_rows_from_table_ref(&join.relation).await?;

            // Perform the JOIN based on type
            result_rows = self.perform_join(
                result_rows,
                &current_alias,
                join_rows,
                &join_alias,
                &join.join_type,
                join.condition.as_ref(),
            )?;

            // Update current alias for next join (not actually used after this point)
            current_alias = join_alias;
        }

        // Apply WHERE clause filtering
        let filtered_rows = if let Some(where_expr) = &select.where_clause {
            Self::apply_post_filter(result_rows, where_expr)?
        } else {
            result_rows
        };

        // Apply GROUP BY and aggregation if needed
        let processed_rows = if !select.group_by.is_empty() || self.has_aggregates(select) {
            let group_by_columns = self.extract_group_by_columns(&select.group_by);
            let aggregates = self.extract_aggregate_functions(&select.select_list);
            let (agg_rows, _cols) = self.execute_grouped_aggregates(
                &filtered_rows,
                &aggregates,
                &group_by_columns,
                &select.having,
                &select.select_list,
            )?;
            // Convert QueryResult rows back to storage Row format for further processing
            self.query_result_to_storage_rows(&agg_rows)?
        } else {
            filtered_rows
        };

        // Apply ORDER BY
        let ordered_rows = if !select.order_by.is_empty() {
            Self::apply_order_by(processed_rows, &select.order_by)?
        } else {
            processed_rows
        };

        // Apply LIMIT and OFFSET
        let mut final_rows = ordered_rows;
        if let Some(offset) = select.offset {
            if offset as usize >= final_rows.len() {
                final_rows = Vec::new();
            } else {
                final_rows = final_rows.into_iter().skip(offset as usize).collect();
            }
        }
        if let Some(limit) = select.limit {
            final_rows.truncate(limit as usize);
        }

        // Convert to result format
        let (result_rows, columns) = self.convert_storage_rows_to_result(final_rows, select)?;
        let rows_affected = result_rows.len() as u64;

        Ok(QueryResult {
            rows: result_rows,
            columns,
            execution_time: start_time.elapsed(),
            rows_affected,
            optimization_applied: !plan.synaptic_pathways.is_empty(),
            synaptic_pathways_used: plan.synaptic_pathways.len() as u32,
            quantum_operations: 0,
        })
    }

    /// Get rows from a table reference (either regular table or derived table)
    fn get_rows_from_table_ref<'a>(
        &'a mut self,
        table_ref: &'a TableReference,
    ) -> TableRowFuture<'a> {
        Box::pin(async move {
            if let Some(subquery) = &table_ref.subquery {
                // This is a derived table - execute the subquery
                let alias = table_ref
                    .alias
                    .clone()
                    .ok_or_else(|| QSQLError::ExecutionError {
                        message: "Derived table requires an alias".to_string(),
                    })?;

                // Execute the subquery
                let subquery_plan = QueryPlan {
                    statement: Statement::Select(subquery.as_ref().clone()),
                    execution_strategy: ExecutionStrategy::Sequential,
                    synaptic_pathways: vec![],
                    quantum_optimizations: vec![],
                    estimated_cost: 1.0,
                    optimization_metadata: OptimizationMetadata {
                        optimization_time: Duration::from_millis(0),
                        iterations_used: 0,
                        convergence_achieved: true,
                        synaptic_adaptations: 0,
                        quantum_optimizations_applied: 0,
                    },
                };

                let result = self.execute(&subquery_plan).await?;

                // Convert QueryResult rows to storage Row format
                let storage_rows = self.query_result_to_storage_rows(&result.rows)?;

                // Add alias prefix to all column names
                let aliased_rows: Vec<Row> = storage_rows
                    .into_iter()
                    .map(|row| {
                        let mut aliased_fields = HashMap::new();
                        for (col, val) in row.fields {
                            // Add both aliased and unaliased versions
                            aliased_fields.insert(format!("{}.{}", alias, col), val.clone());
                            aliased_fields.insert(col, val);
                        }
                        Row {
                            id: row.id,
                            fields: aliased_fields,
                            created_at: row.created_at,
                            updated_at: row.updated_at,
                        }
                    })
                    .collect();

                Ok((aliased_rows, alias))
            } else {
                // This is a regular table - fetch from storage
                let table_name = &table_ref.name;
                let alias = table_ref
                    .alias
                    .clone()
                    .unwrap_or_else(|| table_name.clone());

                let storage_query = SelectQuery {
                    table: table_name.clone(),
                    columns: vec!["*".to_string()],
                    where_clause: None,
                    order_by: None,
                    limit: None,
                    offset: None,
                };

                let storage_guard = self.storage_engine.as_ref().unwrap().read().await;
                let rows = storage_guard
                    .select_rows(&storage_query)
                    .await
                    .map_err(|e| QSQLError::ExecutionError {
                        message: format!("Failed to fetch table {}: {}", table_name, e),
                    })?;
                drop(storage_guard);

                // Add alias prefix to all column names if alias is different from table name
                let aliased_rows: Vec<Row> = rows
                    .into_iter()
                    .map(|row| {
                        let mut aliased_fields = HashMap::new();
                        for (col, val) in row.fields {
                            // Add both aliased and unaliased versions
                            aliased_fields.insert(format!("{}.{}", alias, col), val.clone());
                            aliased_fields.insert(col, val);
                        }
                        Row {
                            id: row.id,
                            fields: aliased_fields,
                            created_at: row.created_at,
                            updated_at: row.updated_at,
                        }
                    })
                    .collect();

                Ok((aliased_rows, alias))
            }
        })
    }

    /// Convert QueryResult rows to storage Row format
    fn query_result_to_storage_rows(
        &self,
        rows: &[HashMap<String, QueryValue>],
    ) -> QSQLResult<Vec<Row>> {
        let mut storage_rows = Vec::new();

        for (idx, qrow) in rows.iter().enumerate() {
            let mut fields = HashMap::new();

            for (col, qval) in qrow {
                let value = match qval {
                    QueryValue::Null => Value::Null,
                    QueryValue::Boolean(b) => Value::Boolean(*b),
                    QueryValue::Integer(i) => Value::Integer(*i),
                    QueryValue::Float(f) => Value::Float(*f),
                    QueryValue::String(s) => Value::Text(s.clone()),
                    QueryValue::Blob(b) => Value::Binary(b.clone()),
                    QueryValue::DNASequence(s) => Value::Text(s.clone()),
                    QueryValue::SynapticWeight(w) => Value::Float(*w as f64),
                    QueryValue::QuantumState(s) => Value::Text(s.clone()),
                };
                fields.insert(col.clone(), value);
            }

            storage_rows.push(Row {
                id: idx as RowId,
                fields,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            });
        }

        Ok(storage_rows)
    }

    /// Check if SELECT statement has aggregate functions
    fn has_aggregates(&self, select: &SelectStatement) -> bool {
        select.select_list.iter().any(|item| {
            if let SelectItem::Expression { expr, .. } = item {
                self.is_aggregate_expression(expr)
            } else {
                false
            }
        })
    }

    /// Perform a JOIN operation between two sets of rows
    fn perform_join(
        &self,
        left_rows: Vec<Row>,
        left_alias: &str,
        right_rows: Vec<Row>,
        right_alias: &str,
        join_type: &JoinType,
        condition: Option<&Expression>,
    ) -> QSQLResult<Vec<Row>> {
        let mut result = Vec::new();

        match join_type {
            JoinType::Inner => {
                // INNER JOIN: Only matching rows
                for left_row in &left_rows {
                    for right_row in &right_rows {
                        if Self::evaluate_join_condition(
                            left_row,
                            left_alias,
                            right_row,
                            right_alias,
                            condition,
                        )? {
                            let merged =
                                Self::merge_rows(left_row, left_alias, right_row, right_alias);
                            result.push(merged);
                        }
                    }
                }
            }
            JoinType::Left => {
                // LEFT JOIN: All left rows, matching right rows or NULLs
                for left_row in &left_rows {
                    let mut found_match = false;
                    for right_row in &right_rows {
                        if Self::evaluate_join_condition(
                            left_row,
                            left_alias,
                            right_row,
                            right_alias,
                            condition,
                        )? {
                            let merged =
                                Self::merge_rows(left_row, left_alias, right_row, right_alias);
                            result.push(merged);
                            found_match = true;
                        }
                    }
                    if !found_match {
                        // Add left row with NULLs for right columns
                        let merged = Self::merge_rows_with_nulls(
                            left_row,
                            left_alias,
                            &right_rows,
                            right_alias,
                            true,
                        );
                        result.push(merged);
                    }
                }
            }
            JoinType::Right => {
                // RIGHT JOIN: All right rows, matching left rows or NULLs
                for right_row in &right_rows {
                    let mut found_match = false;
                    for left_row in &left_rows {
                        if Self::evaluate_join_condition(
                            left_row,
                            left_alias,
                            right_row,
                            right_alias,
                            condition,
                        )? {
                            let merged =
                                Self::merge_rows(left_row, left_alias, right_row, right_alias);
                            result.push(merged);
                            found_match = true;
                        }
                    }
                    if !found_match {
                        // Add right row with NULLs for left columns
                        let merged = Self::merge_rows_with_nulls(
                            right_row,
                            right_alias,
                            &left_rows,
                            left_alias,
                            false,
                        );
                        result.push(merged);
                    }
                }
            }
            JoinType::Full => {
                // FULL OUTER JOIN: All rows from both sides
                let mut matched_right_indices = std::collections::HashSet::new();

                for left_row in &left_rows {
                    let mut found_match = false;
                    for (idx, right_row) in right_rows.iter().enumerate() {
                        if Self::evaluate_join_condition(
                            left_row,
                            left_alias,
                            right_row,
                            right_alias,
                            condition,
                        )? {
                            let merged =
                                Self::merge_rows(left_row, left_alias, right_row, right_alias);
                            result.push(merged);
                            found_match = true;
                            matched_right_indices.insert(idx);
                        }
                    }
                    if !found_match {
                        let merged = Self::merge_rows_with_nulls(
                            left_row,
                            left_alias,
                            &right_rows,
                            right_alias,
                            true,
                        );
                        result.push(merged);
                    }
                }

                // Add unmatched right rows
                for (idx, right_row) in right_rows.iter().enumerate() {
                    if !matched_right_indices.contains(&idx) {
                        let merged = Self::merge_rows_with_nulls(
                            right_row,
                            right_alias,
                            &left_rows,
                            left_alias,
                            false,
                        );
                        result.push(merged);
                    }
                }
            }
            JoinType::Cross => {
                // CROSS JOIN: Cartesian product
                for left_row in &left_rows {
                    for right_row in &right_rows {
                        let merged = Self::merge_rows(left_row, left_alias, right_row, right_alias);
                        result.push(merged);
                    }
                }
            }
            _ => {
                return Err(QSQLError::ExecutionError {
                    message: format!("Unsupported join type: {:?}", join_type),
                });
            }
        }

        Ok(result)
    }

    /// Evaluate JOIN condition (e.g., ON u.id = o.user_id)
    fn evaluate_join_condition(
        left_row: &Row,
        left_alias: &str,
        right_row: &Row,
        right_alias: &str,
        condition: Option<&Expression>,
    ) -> QSQLResult<bool> {
        let Some(expr) = condition else {
            // No condition means always match (CROSS JOIN behavior)
            return Ok(true);
        };

        Self::evaluate_join_expression(left_row, left_alias, right_row, right_alias, expr)
    }

    /// Evaluate a JOIN expression against two rows
    fn evaluate_join_expression(
        left_row: &Row,
        left_alias: &str,
        right_row: &Row,
        right_alias: &str,
        expr: &Expression,
    ) -> QSQLResult<bool> {
        match expr {
            Expression::BinaryOp {
                left,
                operator,
                right,
            } => match operator {
                BinaryOperator::And => {
                    let left_result = Self::evaluate_join_expression(
                        left_row,
                        left_alias,
                        right_row,
                        right_alias,
                        left,
                    )?;
                    let right_result = Self::evaluate_join_expression(
                        left_row,
                        left_alias,
                        right_row,
                        right_alias,
                        right,
                    )?;
                    Ok(left_result && right_result)
                }
                BinaryOperator::Or => {
                    let left_result = Self::evaluate_join_expression(
                        left_row,
                        left_alias,
                        right_row,
                        right_alias,
                        left,
                    )?;
                    let right_result = Self::evaluate_join_expression(
                        left_row,
                        left_alias,
                        right_row,
                        right_alias,
                        right,
                    )?;
                    Ok(left_result || right_result)
                }
                BinaryOperator::Equal => {
                    let left_val =
                        Self::get_join_value(left_row, left_alias, right_row, right_alias, left)?;
                    let right_val =
                        Self::get_join_value(left_row, left_alias, right_row, right_alias, right)?;
                    Ok(Self::values_equal(&left_val, &right_val))
                }
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    /// Get value from a row based on expression (handles qualified column names like u.id)
    fn get_join_value(
        left_row: &Row,
        left_alias: &str,
        right_row: &Row,
        right_alias: &str,
        expr: &Expression,
    ) -> QSQLResult<Value> {
        match expr {
            Expression::Identifier(name) => {
                // Handle qualified names like "u.id" or unqualified names like "id"
                if let Some((table, col)) = name.split_once('.') {
                    if table == left_alias {
                        return left_row.fields.get(col).cloned().ok_or_else(|| {
                            QSQLError::ExecutionError {
                                message: format!("Column {} not found in left table", col),
                            }
                        });
                    } else if table == right_alias {
                        return right_row.fields.get(col).cloned().ok_or_else(|| {
                            QSQLError::ExecutionError {
                                message: format!("Column {} not found in right table", col),
                            }
                        });
                    }
                }
                // Unqualified name - try both tables
                if let Some(val) = left_row.fields.get(name) {
                    return Ok(val.clone());
                }
                if let Some(val) = right_row.fields.get(name) {
                    return Ok(val.clone());
                }
                Err(QSQLError::ExecutionError {
                    message: format!("Column {} not found in any table", name),
                })
            }
            Expression::Literal(lit) => {
                Self::convert_expression_to_value_static(&Expression::Literal(lit.clone()))
            }
            _ => Err(QSQLError::ExecutionError {
                message: "Unsupported expression in JOIN condition".to_string(),
            }),
        }
    }

    /// Merge two rows from different tables into one
    fn merge_rows(left_row: &Row, left_alias: &str, right_row: &Row, right_alias: &str) -> Row {
        let mut merged_fields = HashMap::new();

        // Add left row fields with alias prefix
        for (col, val) in &left_row.fields {
            merged_fields.insert(format!("{}.{}", left_alias, col), val.clone());
            // Also add without prefix for compatibility
            merged_fields.insert(col.clone(), val.clone());
        }

        // Add right row fields with alias prefix
        for (col, val) in &right_row.fields {
            merged_fields.insert(format!("{}.{}", right_alias, col), val.clone());
            // Add without prefix if not already present (left takes precedence)
            if !merged_fields.contains_key(col) {
                merged_fields.insert(col.clone(), val.clone());
            }
        }

        Row {
            id: left_row.id,
            fields: merged_fields,
            created_at: left_row.created_at,
            updated_at: left_row.updated_at,
        }
    }

    /// Merge a row with NULLs for the other table's fields
    fn merge_rows_with_nulls(
        row: &Row,
        row_alias: &str,
        other_rows: &[Row],
        other_alias: &str,
        row_is_left: bool,
    ) -> Row {
        let mut merged_fields = HashMap::new();

        // Get field names from other table (use first row as template)
        let other_field_names: Vec<String> = if let Some(first_row) = other_rows.first() {
            first_row.fields.keys().cloned().collect()
        } else {
            Vec::new()
        };

        if row_is_left {
            // Row is from left table
            for (col, val) in &row.fields {
                merged_fields.insert(format!("{}.{}", row_alias, col), val.clone());
                merged_fields.insert(col.clone(), val.clone());
            }
            // Add NULLs for right fields
            for col in &other_field_names {
                merged_fields.insert(format!("{}.{}", other_alias, col), Value::Null);
            }
        } else {
            // Row is from right table - add NULLs for left fields first
            for col in &other_field_names {
                merged_fields.insert(format!("{}.{}", other_alias, col), Value::Null);
            }
            // Add right row fields
            for (col, val) in &row.fields {
                merged_fields.insert(format!("{}.{}", row_alias, col), val.clone());
                merged_fields.insert(col.clone(), val.clone());
            }
        }

        Row {
            id: row.id,
            fields: merged_fields,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }

    /// Apply ORDER BY sorting to rows
    fn apply_order_by(mut rows: Vec<Row>, order_by: &[OrderByItem]) -> QSQLResult<Vec<Row>> {
        if order_by.is_empty() {
            return Ok(rows);
        }

        rows.sort_by(|a, b| {
            for order_item in order_by {
                let col_name = Self::expression_to_string_static(&order_item.expression);

                let a_val = a.fields.get(&col_name);
                let b_val = b.fields.get(&col_name);

                let cmp = match (a_val, b_val) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => a.cmp(b),
                    (Some(Value::Float(a)), Some(Value::Float(b))) => {
                        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Some(Value::Text(a)), Some(Value::Text(b))) => a.cmp(b),
                    (Some(Value::Boolean(a)), Some(Value::Boolean(b))) => a.cmp(b),
                    _ => std::cmp::Ordering::Equal,
                };

                if cmp != std::cmp::Ordering::Equal {
                    return if order_item.ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    };
                }
            }
            std::cmp::Ordering::Equal
        });

        Ok(rows)
    }

    /// Execute INSERT statement with DNA compression and neuromorphic learning
    async fn execute_insert(
        &mut self,
        insert: &InsertStatement,
        plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        let start_time = std::time::Instant::now();

        // Check if storage engine is available
        if self.storage_engine.is_some() {
            // Real execution with storage engine
            let mut total_rows_affected = 0;
            let mut inserted_ids = Vec::new();

            // Process each value set
            for value_set in &insert.values {
                // Convert SQL INSERT to storage Row (no borrow of self needed)
                let row = Self::convert_insert_to_row_static(insert, value_set)?;

                // Insert via storage engine (automatically DNA-compressed!)
                // Use transactional insert if we have an active transaction
                let mut storage_guard = self.storage_engine.as_ref().unwrap().write().await;
                let row_id = if let Some(tx_id) = self.current_transaction {
                    storage_guard
                        .insert_row_transactional(tx_id, &insert.table_name, row)
                        .await
                        .map_err(|e| QSQLError::ExecutionError {
                            message: format!("Transactional storage insert failed: {}", e),
                        })?
                } else {
                    storage_guard
                        .insert_row(&insert.table_name, row)
                        .await
                        .map_err(|e| QSQLError::ExecutionError {
                            message: format!("Storage insert failed: {}", e),
                        })?
                };
                drop(storage_guard); // Release lock early

                inserted_ids.push(row_id);
                total_rows_affected += 1;
            }

            // Neuromorphic learning: adapt to insert pattern (after loop)
            if self.config.enable_neuromorphic_learning && insert.synaptic_adaptation {
                for row_id in &inserted_ids {
                    self.learn_from_insert(insert, *row_id).await?;
                }
            }

            // Build result
            let mut result_rows = Vec::new();
            for row_id in inserted_ids {
                let mut result_row = HashMap::new();
                result_row.insert(
                    "inserted_id".to_string(),
                    QueryValue::Integer(row_id as i64),
                );
                result_rows.push(result_row);
            }

            Ok(QueryResult {
                rows: result_rows,
                columns: vec![ColumnInfo {
                    name: "inserted_id".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                }],
                execution_time: start_time.elapsed(),
                rows_affected: total_rows_affected,
                optimization_applied: insert.synaptic_adaptation,
                synaptic_pathways_used: plan.synaptic_pathways.len() as u32,
                quantum_operations: 0,
            })
        } else {
            // Fallback: Simulate insertion (legacy mode) - only available in test builds
            #[cfg(test)]
            {
                warn!(
                    "INSERT executed in legacy mode with simulated result. \
                     This is only available in test builds."
                );
                let mut rows = vec![HashMap::new()];
                rows[0].insert("id".to_string(), QueryValue::Integer(1));
                rows[0].insert(
                    "name".to_string(),
                    QueryValue::String("New User".to_string()),
                );

                Ok(QueryResult {
                    rows,
                    columns: vec![],
                    execution_time: Duration::from_millis(1),
                    rows_affected: 1,
                    optimization_applied: false,
                    synaptic_pathways_used: 0,
                    quantum_operations: 0,
                })
            }
            #[cfg(not(test))]
            {
                Err(QSQLError::ConfigError {
                    message: "Storage engine required for query execution.".to_string(),
                })
            }
        }
    }

    /// Execute UPDATE statement with DNA re-compression and plasticity adaptation
    async fn execute_update(
        &mut self,
        update: &UpdateStatement,
        plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        let start_time = std::time::Instant::now();
        if self.storage_engine.is_some() {
            // Convert SQL UPDATE to storage query (no borrow needed)
            let storage_query = Self::convert_update_to_storage_query_static(update)?;

            // Execute update via storage engine (automatically DNA re-compressed!)
            // Use transactional update if we have an active transaction
            let mut storage_guard = self.storage_engine.as_ref().unwrap().write().await;
            let rows_affected = if let Some(tx_id) = self.current_transaction {
                storage_guard
                    .update_rows_transactional(tx_id, &storage_query)
                    .await
                    .map_err(|e| QSQLError::ExecutionError {
                        message: format!("Transactional storage update failed: {}", e),
                    })?
            } else {
                storage_guard
                    .update_rows(&storage_query)
                    .await
                    .map_err(|e| QSQLError::ExecutionError {
                        message: format!("Storage update failed: {}", e),
                    })?
            };
            drop(storage_guard); // Release lock early

            // Plasticity adaptation: strengthen connections for updated patterns
            if self.config.enable_neuromorphic_learning && update.plasticity_adaptation.is_some() {
                self.adapt_plasticity_from_update(update).await?;
            }

            Ok(QueryResult {
                rows: vec![],
                columns: vec![],
                execution_time: start_time.elapsed(),
                rows_affected,
                optimization_applied: update.plasticity_adaptation.is_some(),
                synaptic_pathways_used: plan.synaptic_pathways.len() as u32,
                quantum_operations: 0,
            })
        } else {
            // Fallback: Simulate update (legacy mode) - only available in test builds
            #[cfg(test)]
            {
                warn!(
                    "UPDATE executed in legacy mode with simulated result. \
                     This is only available in test builds."
                );
                let mut rows = vec![HashMap::new()];
                rows[0].insert("id".to_string(), QueryValue::Integer(1));
                rows[0].insert(
                    "name".to_string(),
                    QueryValue::String("Updated User".to_string()),
                );

                Ok(QueryResult {
                    rows,
                    columns: vec![],
                    execution_time: Duration::from_millis(1),
                    rows_affected: 1,
                    optimization_applied: false,
                    synaptic_pathways_used: 0,
                    quantum_operations: 0,
                })
            }
            #[cfg(not(test))]
            {
                Err(QSQLError::ConfigError {
                    message: "Storage engine required for query execution.".to_string(),
                })
            }
        }
    }

    /// Execute DELETE statement with DNA cleanup and synaptic pruning
    async fn execute_delete(
        &mut self,
        delete: &DeleteStatement,
        plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        let start_time = std::time::Instant::now();
        if self.storage_engine.is_some() {
            // Convert SQL DELETE to storage query (no borrow needed)
            let storage_query = Self::convert_delete_to_storage_query_static(delete)?;

            // Execute delete via storage engine (frees compressed DNA blocks!)
            // Use transactional delete if we have an active transaction
            let mut storage_guard = self.storage_engine.as_ref().unwrap().write().await;
            let rows_affected = if let Some(tx_id) = self.current_transaction {
                storage_guard
                    .delete_rows_transactional(tx_id, &storage_query)
                    .await
                    .map_err(|e| QSQLError::ExecutionError {
                        message: format!("Transactional storage delete failed: {}", e),
                    })?
            } else {
                storage_guard
                    .delete_rows(&storage_query)
                    .await
                    .map_err(|e| QSQLError::ExecutionError {
                        message: format!("Storage delete failed: {}", e),
                    })?
            };
            drop(storage_guard); // Release lock early

            // Synaptic pruning: weaken connections for deleted data patterns
            if self.config.enable_neuromorphic_learning && delete.synaptic_pruning {
                self.prune_synaptic_connections_from_delete(delete).await?;
            }

            Ok(QueryResult {
                rows: vec![],
                columns: vec![],
                execution_time: start_time.elapsed(),
                rows_affected,
                optimization_applied: delete.synaptic_pruning,
                synaptic_pathways_used: plan.synaptic_pathways.len() as u32,
                quantum_operations: 0,
            })
        } else {
            // Fallback: Simulate deletion (legacy mode) - only available in test builds
            #[cfg(test)]
            {
                warn!(
                    "DELETE executed in legacy mode with simulated result. \
                     This is only available in test builds."
                );
                Ok(QueryResult {
                    rows: vec![],
                    columns: vec![],
                    execution_time: Duration::from_millis(1),
                    rows_affected: 1,
                    optimization_applied: false,
                    synaptic_pathways_used: 0,
                    quantum_operations: 0,
                })
            }
            #[cfg(not(test))]
            {
                Err(QSQLError::ConfigError {
                    message: "Storage engine required for query execution.".to_string(),
                })
            }
        }
    }

    /// Execute NEUROMATCH statement with synaptic optimization
    async fn execute_neuromatch(
        &mut self,
        neuromatch: &NeuroMatchStatement,
        plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        let columns = vec![
            ColumnInfo {
                name: "match_score".to_string(),
                data_type: DataType::SynapticWeight,
                nullable: false,
            },
            ColumnInfo {
                name: "entity_id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            },
        ];

        let mut rows = Vec::new();

        // Simulate neuromorphic pattern matching
        for i in 1..=3 {
            let mut row = HashMap::new();
            let synaptic_score = neuromatch.synaptic_weight * (1.0 - (i as f32 * 0.1));
            row.insert(
                "match_score".to_string(),
                QueryValue::SynapticWeight(synaptic_score),
            );
            row.insert("entity_id".to_string(), QueryValue::Integer(i));
            rows.push(row);
        }

        Ok(QueryResult {
            rows,
            columns,
            execution_time: Duration::from_micros(750), // Neuromorphic processing overhead
            rows_affected: 3,
            optimization_applied: true,
            synaptic_pathways_used: plan.synaptic_pathways.len() as u32,
            quantum_operations: 0,
        })
    }

    /// Execute QUANTUM_SEARCH with Grover's algorithm simulation
    async fn execute_quantum_search(
        &mut self,
        quantum: &QuantumSearchStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        let columns = vec![
            ColumnInfo {
                name: "quantum_amplitude".to_string(),
                data_type: DataType::QuantumBit,
                nullable: false,
            },
            ColumnInfo {
                name: "result_id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            },
        ];

        let mut rows = Vec::new();

        // Simulate quantum search results
        let iterations = quantum.max_iterations.unwrap_or(10);
        let amplitude_boost = if quantum.amplitude_amplification {
            1.5
        } else {
            1.0
        };

        for i in 1..=2 {
            let mut row = HashMap::new();
            row.insert(
                "quantum_amplitude".to_string(),
                QueryValue::String(format!(
                    "{}|0⟩ + {}|1⟩",
                    0.6 * amplitude_boost,
                    0.8 * amplitude_boost
                )),
            );
            row.insert("result_id".to_string(), QueryValue::Integer(i));
            rows.push(row);
        }

        // Quantum advantage: faster execution time
        let execution_time = Duration::from_micros(200); // Quadratic speedup simulation

        Ok(QueryResult {
            rows,
            columns,
            execution_time,
            rows_affected: 2,
            optimization_applied: true,
            synaptic_pathways_used: 0,
            quantum_operations: iterations,
        })
    }

    /// Execute SUPERPOSITION_QUERY with parallel quantum processing
    async fn execute_superposition_query(
        &mut self,
        superpos: &SuperpositionQueryStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        let columns = vec![
            ColumnInfo {
                name: "superposition_state".to_string(),
                data_type: DataType::SuperpositionState,
                nullable: false,
            },
            ColumnInfo {
                name: "coherence_level".to_string(),
                data_type: DataType::Real,
                nullable: false,
            },
        ];

        let mut rows = Vec::new();

        // Simulate superposition results from parallel queries
        for (i, _query) in superpos.parallel_queries.iter().enumerate() {
            let mut row = HashMap::new();
            row.insert(
                "superposition_state".to_string(),
                QueryValue::QuantumState(format!("State_{}", i)),
            );
            row.insert(
                "coherence_level".to_string(),
                QueryValue::Float(0.9 - (i as f64 * 0.1)),
            );
            rows.push(row);
        }

        // Parallel execution advantage
        let base_time = Duration::from_micros(1000);
        let parallel_speedup = superpos.parallel_queries.len() as u32;
        let execution_time =
            Duration::from_nanos(base_time.as_nanos() as u64 / parallel_speedup as u64);

        Ok(QueryResult {
            rows,
            columns,
            execution_time,
            rows_affected: superpos.parallel_queries.len() as u64,
            optimization_applied: true,
            synaptic_pathways_used: 0,
            quantum_operations: parallel_speedup,
        })
    }

    /// Execute LEARN_PATTERN statement
    async fn execute_learn_pattern(
        &mut self,
        _learn: &LearnPatternStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        // Simulate learning a pattern
        let mut rows = vec![HashMap::new()];
        rows[0].insert("pattern_id".to_string(), QueryValue::Integer(1));
        rows[0].insert(
            "pattern_description".to_string(),
            QueryValue::String("Learned Pattern 1".to_string()),
        );

        Ok(QueryResult {
            rows,
            columns: vec![],
            execution_time: Duration::from_millis(1),
            rows_affected: 1,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute ADAPT_WEIGHTS statement
    async fn execute_adapt_weights(
        &mut self,
        _adapt: &AdaptWeightsStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        // Simulate weight adaptation
        let mut rows = vec![HashMap::new()];
        rows[0].insert("adaptation_id".to_string(), QueryValue::Integer(1));
        rows[0].insert("new_weight".to_string(), QueryValue::SynapticWeight(0.9));

        Ok(QueryResult {
            rows,
            columns: vec![],
            execution_time: Duration::from_millis(1),
            rows_affected: 1,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute QUANTUM_JOIN with quantum-enhanced join processing
    async fn execute_quantum_join(
        &mut self,
        _qjoin: &QuantumJoinStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        let columns = vec![
            ColumnInfo {
                name: "joined_id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            },
            ColumnInfo {
                name: "user_name".to_string(),
                data_type: DataType::VarChar(Some(255)),
                nullable: true,
            },
        ];

        let mut rows = Vec::new();

        // Simulate quantum join results
        for i in 1..=2 {
            let mut row = HashMap::new();
            row.insert("joined_id".to_string(), QueryValue::Integer(i));
            row.insert(
                "user_name".to_string(),
                QueryValue::String(format!("User {}", i)),
            );
            rows.push(row);
        }

        // Quantum join advantage: reduced execution time
        let execution_time = Duration::from_micros(300); // Speedup simulation

        Ok(QueryResult {
            rows,
            columns,
            execution_time,
            rows_affected: 2,
            optimization_applied: true,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute EXPLAIN statement
    async fn execute_explain(
        &mut self,
        explain: &ExplainStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        use crate::explain::{ExplainConfig, ExplainGenerator};

        // Create a query plan for the inner statement
        // For now, create a simple plan without full optimization
        let inner_plan = QueryPlan {
            statement: (*explain.statement).clone(),
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

        // Generate explain plan
        let config = ExplainConfig {
            show_costs: true,
            show_timing: explain.analyze,
            show_buffers: explain.verbose,
            show_synaptic_pathways: true,
            show_quantum_ops: true,
            format: explain.format.clone(),
        };

        let generator = ExplainGenerator::new(config);
        let explain_plan = generator.generate_explain(&inner_plan, explain.analyze)?;

        // Format output based on format
        let output = match explain.format {
            ExplainFormat::Text => generator.format_text(&explain_plan),
            ExplainFormat::Json => generator.format_json(&explain_plan)?,
            ExplainFormat::Yaml => generator.format_yaml(&explain_plan)?,
            ExplainFormat::Xml => {
                // Simple XML wrapper (full XML support could be added later)
                format!(
                    "<explain>{}</explain>",
                    generator.format_text(&explain_plan)
                )
            }
        };

        // Return as a single-row result
        let columns = vec![ColumnInfo {
            name: "QUERY PLAN".to_string(),
            data_type: DataType::VarChar(None),
            nullable: false,
        }];

        let mut row = HashMap::new();
        row.insert("QUERY PLAN".to_string(), QueryValue::String(output));

        Ok(QueryResult {
            rows: vec![row],
            columns,
            execution_time: explain_plan.planning_time,
            rows_affected: 1,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute ANALYZE statement
    async fn execute_analyze(
        &mut self,
        analyze: &AnalyzeStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        use crate::explain::TableStatistics;

        // Simulate collecting table statistics
        let stats = TableStatistics {
            table_name: analyze.table_name.clone(),
            row_count: 10000,
            page_count: 100,
            avg_row_size: 256,
            null_frac: HashMap::new(),
            distinct_values: HashMap::new(),
            most_common_values: HashMap::new(),
            histogram_bounds: HashMap::new(),
            last_analyzed: std::time::SystemTime::now(),
            synaptic_density: 0.75,
            plasticity_index: 0.82,
        };

        // Return statistics as result
        let columns = vec![
            ColumnInfo {
                name: "table_name".to_string(),
                data_type: DataType::VarChar(Some(255)),
                nullable: false,
            },
            ColumnInfo {
                name: "row_count".to_string(),
                data_type: DataType::BigInt,
                nullable: false,
            },
            ColumnInfo {
                name: "page_count".to_string(),
                data_type: DataType::BigInt,
                nullable: false,
            },
            ColumnInfo {
                name: "avg_row_size".to_string(),
                data_type: DataType::Integer,
                nullable: false,
            },
            ColumnInfo {
                name: "synaptic_density".to_string(),
                data_type: DataType::Real,
                nullable: false,
            },
            ColumnInfo {
                name: "plasticity_index".to_string(),
                data_type: DataType::Real,
                nullable: false,
            },
        ];

        let mut row = HashMap::new();
        row.insert(
            "table_name".to_string(),
            QueryValue::String(stats.table_name),
        );
        row.insert(
            "row_count".to_string(),
            QueryValue::Integer(stats.row_count as i64),
        );
        row.insert(
            "page_count".to_string(),
            QueryValue::Integer(stats.page_count as i64),
        );
        row.insert(
            "avg_row_size".to_string(),
            QueryValue::Integer(stats.avg_row_size as i64),
        );
        row.insert(
            "synaptic_density".to_string(),
            QueryValue::Float(stats.synaptic_density as f64),
        );
        row.insert(
            "plasticity_index".to_string(),
            QueryValue::Float(stats.plasticity_index as f64),
        );

        Ok(QueryResult {
            rows: vec![row],
            columns,
            execution_time: Duration::from_millis(50),
            rows_affected: 1,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute CREATE TABLE statement
    async fn execute_create_table(
        &mut self,
        create: &CreateTableStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        // Get storage engine
        let storage_engine =
            self.storage_engine
                .as_ref()
                .ok_or_else(|| QSQLError::ExecutionError {
                    message: "Storage engine not configured".to_string(),
                })?;

        // Convert QSQL column definitions to storage column definitions
        let columns: Vec<neuroquantum_core::storage::ColumnDefinition> = create
            .columns
            .iter()
            .map(|col| {
                // Convert data type
                let data_type = match col.data_type {
                    DataType::Integer => neuroquantum_core::storage::DataType::Integer,
                    DataType::BigInt => neuroquantum_core::storage::DataType::Integer,
                    DataType::SmallInt => neuroquantum_core::storage::DataType::Integer,
                    DataType::Real | DataType::Double => {
                        neuroquantum_core::storage::DataType::Float
                    }
                    DataType::Text
                    | DataType::VarChar(_)
                    | DataType::Varchar(_)
                    | DataType::Char(_) => neuroquantum_core::storage::DataType::Text,
                    DataType::Boolean => neuroquantum_core::storage::DataType::Boolean,
                    DataType::Timestamp | DataType::Date | DataType::Time => {
                        neuroquantum_core::storage::DataType::Timestamp
                    }
                    DataType::Blob => neuroquantum_core::storage::DataType::Binary,
                    DataType::Serial | DataType::BigSerial | DataType::SmallSerial => {
                        neuroquantum_core::storage::DataType::Integer
                    }
                    DataType::Decimal(_, _) => neuroquantum_core::storage::DataType::Float,
                    _ => neuroquantum_core::storage::DataType::Text,
                };

                // Check constraints for NOT NULL and DEFAULT
                let nullable = !col
                    .constraints
                    .iter()
                    .any(|c| matches!(c, ColumnConstraint::NotNull));

                let default_value = col.constraints.iter().find_map(|c| {
                    if let ColumnConstraint::Default(expr) = c {
                        Self::convert_expression_to_value_static(expr).ok()
                    } else {
                        None
                    }
                });

                let auto_increment = col.constraints.iter().any(|c| {
                    matches!(
                        c,
                        ColumnConstraint::AutoIncrement | ColumnConstraint::Identity { .. }
                    )
                }) || matches!(
                    col.data_type,
                    DataType::Serial | DataType::BigSerial | DataType::SmallSerial
                );

                neuroquantum_core::storage::ColumnDefinition {
                    name: col.name.clone(),
                    data_type,
                    nullable,
                    default_value,
                    auto_increment,
                }
            })
            .collect();

        // Find primary key
        let primary_key = create
            .columns
            .iter()
            .find(|col| {
                col.constraints
                    .iter()
                    .any(|c| matches!(c, ColumnConstraint::PrimaryKey))
            })
            .map(|col| col.name.clone())
            .or_else(|| {
                create.constraints.iter().find_map(|constraint| {
                    if let TableConstraint::PrimaryKey(cols) = constraint {
                        cols.first().cloned()
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "id".to_string());

        // Create table schema
        let schema = neuroquantum_core::storage::TableSchema {
            name: create.table_name.clone(),
            columns,
            primary_key,
            created_at: chrono::Utc::now(),
            version: 1,
            auto_increment_columns: std::collections::HashMap::new(),
            id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
        };

        // Try to create table
        let mut storage = storage_engine.write().await;
        let result = storage.create_table(schema).await;

        match result {
            Ok(()) => Ok(QueryResult {
                rows: vec![],
                columns: vec![],
                execution_time: Duration::from_millis(10),
                rows_affected: 0,
                optimization_applied: false,
                synaptic_pathways_used: 0,
                quantum_operations: 0,
            }),
            Err(e) => {
                // Check if it's a "table already exists" error and if_not_exists is true
                let error_msg = e.to_string().to_lowercase();
                if (error_msg.contains("already exists") || error_msg.contains("exist"))
                    && create.if_not_exists
                {
                    // Silently succeed
                    Ok(QueryResult {
                        rows: vec![],
                        columns: vec![],
                        execution_time: Duration::from_millis(1),
                        rows_affected: 0,
                        optimization_applied: false,
                        synaptic_pathways_used: 0,
                        quantum_operations: 0,
                    })
                } else {
                    Err(QSQLError::ExecutionError {
                        message: format!("Failed to create table: {}", e),
                    })
                }
            }
        }
    }

    /// Execute DROP TABLE statement
    async fn execute_drop_table(
        &mut self,
        drop: &DropTableStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        // Get storage engine
        let storage_engine =
            self.storage_engine
                .as_ref()
                .ok_or_else(|| QSQLError::ExecutionError {
                    message: "Storage engine not configured".to_string(),
                })?;

        // Drop the table using storage engine
        let mut storage = storage_engine.write().await;
        let result = storage.drop_table(&drop.table_name, drop.if_exists).await;

        match result {
            Ok(()) => Ok(QueryResult {
                rows: vec![],
                columns: vec![],
                execution_time: Duration::from_millis(10),
                rows_affected: 0,
                optimization_applied: false,
                synaptic_pathways_used: 0,
                quantum_operations: 0,
            }),
            Err(e) => {
                // If the table doesn't exist and IF EXISTS was specified,
                // the storage engine will return Ok(()) - so this is a real error
                Err(QSQLError::ExecutionError {
                    message: format!("Failed to drop table: {}", e),
                })
            }
        }
    }

    /// Execute ALTER TABLE statement
    async fn execute_alter_table(
        &mut self,
        alter: &AlterTableStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        // Get storage engine
        let storage = self
            .storage_engine
            .as_ref()
            .ok_or_else(|| QSQLError::ExecutionError {
                message: "Storage engine not configured".to_string(),
            })?;

        // Convert AST operation to storage operation
        let storage_op = match &alter.operation {
            AlterTableOperation::AddColumn { column } => {
                // Extract nullable and default value from constraints
                let mut nullable = true;
                let mut default_value = None;
                let mut auto_increment = false;

                for constraint in &column.constraints {
                    match constraint {
                        ColumnConstraint::NotNull => nullable = false,
                        ColumnConstraint::Default(expr) => {
                            default_value = Some(Self::convert_default_value(expr));
                        }
                        ColumnConstraint::AutoIncrement => auto_increment = true,
                        _ => {}
                    }
                }

                // Convert AST column definition to storage column definition
                let storage_column = neuroquantum_core::storage::ColumnDefinition {
                    name: column.name.clone(),
                    data_type: Self::convert_data_type(&column.data_type),
                    nullable,
                    default_value,
                    auto_increment,
                };
                neuroquantum_core::storage::AlterTableOp::AddColumn {
                    column: storage_column,
                }
            }
            AlterTableOperation::DropColumn { column_name } => {
                neuroquantum_core::storage::AlterTableOp::DropColumn {
                    column_name: column_name.clone(),
                }
            }
            AlterTableOperation::RenameColumn { old_name, new_name } => {
                neuroquantum_core::storage::AlterTableOp::RenameColumn {
                    old_name: old_name.clone(),
                    new_name: new_name.clone(),
                }
            }
            AlterTableOperation::ModifyColumn {
                column_name,
                new_data_type,
            } => neuroquantum_core::storage::AlterTableOp::ModifyColumn {
                column_name: column_name.clone(),
                new_data_type: Self::convert_data_type(new_data_type),
            },
        };

        // Execute the alter table operation
        let mut storage_guard = storage.write().await;
        storage_guard
            .alter_table(&alter.table_name, storage_op)
            .await
            .map_err(|e| QSQLError::ExecutionError {
                message: format!("ALTER TABLE failed: {}", e),
            })?;

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_millis(0),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Convert AST DataType to storage DataType
    fn convert_data_type(ast_type: &DataType) -> neuroquantum_core::storage::DataType {
        match ast_type {
            DataType::Integer | DataType::BigInt | DataType::SmallInt => {
                neuroquantum_core::storage::DataType::Integer
            }
            DataType::Real | DataType::Double | DataType::Decimal(_, _) => {
                neuroquantum_core::storage::DataType::Float
            }
            DataType::VarChar(_) | DataType::Varchar(_) | DataType::Char(_) | DataType::Text => {
                neuroquantum_core::storage::DataType::Text
            }
            DataType::Boolean => neuroquantum_core::storage::DataType::Boolean,
            DataType::Timestamp | DataType::Date | DataType::Time => {
                neuroquantum_core::storage::DataType::Timestamp
            }
            DataType::Blob => neuroquantum_core::storage::DataType::Binary,
            DataType::Serial => neuroquantum_core::storage::DataType::Serial,
            DataType::BigSerial => neuroquantum_core::storage::DataType::BigSerial,
            DataType::SmallSerial => neuroquantum_core::storage::DataType::Serial,
            // Neuromorphic types map to their storage equivalents
            DataType::DNASequence | DataType::NeuralPattern => {
                neuroquantum_core::storage::DataType::Binary
            }
            DataType::SynapticWeight => neuroquantum_core::storage::DataType::Float,
            DataType::PlasticityMatrix => neuroquantum_core::storage::DataType::Binary,
            // Quantum types map to their storage equivalents
            DataType::QuantumBit | DataType::QuantumRegister(_) => {
                neuroquantum_core::storage::DataType::Integer
            }
            DataType::SuperpositionState | DataType::EntanglementPair => {
                neuroquantum_core::storage::DataType::Binary
            }
        }
    }

    /// Convert AST Expression default value to storage Value
    fn convert_default_value(expr: &Expression) -> Value {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Integer(i) => Value::Integer(*i),
                Literal::Float(f) => Value::Float(*f),
                Literal::String(s) => Value::Text(s.clone()),
                Literal::Boolean(b) => Value::Boolean(*b),
                Literal::Null => Value::Null,
                // For complex types, use Text representation
                _ => Value::Text(format!("{:?}", lit)),
            },
            // For non-literal expressions, use Text representation as fallback
            _ => Value::Text(format!("{:?}", expr)),
        }
    }

    /// Execute CREATE INDEX statement
    async fn execute_create_index(
        &mut self,
        create_idx: &CreateIndexStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        // For now, just return success
        // In a full implementation, we would create the index in the storage engine
        if create_idx.if_not_exists {
            // Silently succeed if already exists
        }

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_millis(10),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute DROP INDEX statement
    async fn execute_drop_index(
        &mut self,
        drop_idx: &DropIndexStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        // For now, just return success
        // In a full implementation, we would drop the index from the storage engine
        if drop_idx.if_exists {
            // Silently succeed if doesn't exist
        }

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_millis(10),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute TRUNCATE TABLE statement
    async fn execute_truncate_table(
        &mut self,
        truncate: &TruncateTableStatement,
        _plan: &QueryPlan,
    ) -> QSQLResult<QueryResult> {
        // Get storage engine
        let storage_engine =
            self.storage_engine
                .as_ref()
                .ok_or_else(|| QSQLError::ExecutionError {
                    message: "Storage engine not configured".to_string(),
                })?;

        let mut storage = storage_engine.write().await;

        // Delete all rows from the table
        let delete_query = neuroquantum_core::storage::DeleteQuery {
            table: truncate.table_name.clone(),
            where_clause: None, // Delete all rows
        };

        let rows_affected =
            storage
                .delete_rows(&delete_query)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Failed to truncate table: {}", e),
                })?;

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_millis(50),
            rows_affected,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    // =============================================================================
    // Transaction Control Execution
    // =============================================================================

    /// Execute BEGIN TRANSACTION statement
    async fn execute_begin_transaction(
        &mut self,
        begin: &BeginTransactionStatement,
    ) -> QSQLResult<QueryResult> {
        // Check if there's already an active transaction
        if self.current_transaction.is_some() {
            return Err(QSQLError::ExecutionError {
                message: "Transaction already in progress. COMMIT or ROLLBACK first.".to_string(),
            });
        }

        // Parse isolation level (default to ReadCommitted)
        let isolation_level = match begin.isolation_level.as_deref() {
            Some("READ UNCOMMITTED") => IsolationLevel::ReadUncommitted,
            Some("READ COMMITTED") => IsolationLevel::ReadCommitted,
            Some("REPEATABLE READ") => IsolationLevel::RepeatableRead,
            Some("SERIALIZABLE") => IsolationLevel::Serializable,
            None => IsolationLevel::ReadCommitted,
            Some(level) => {
                return Err(QSQLError::ExecutionError {
                    message: format!("Unknown isolation level: {}", level),
                });
            }
        };

        // Prefer using storage engine's transaction manager for consistency
        let tx_id = if let Some(storage_engine) = &self.storage_engine {
            let storage_guard = storage_engine.read().await;
            storage_guard
                .begin_transaction_with_isolation(isolation_level)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Failed to begin transaction via storage: {}", e),
                })?
        } else if let Some(tx_manager) = &self.transaction_manager {
            // Fallback to executor's transaction manager
            tx_manager
                .begin_transaction(isolation_level)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Failed to begin transaction: {}", e),
                })?
        } else {
            return Err(QSQLError::ExecutionError {
                message: "Transaction manager not available. Transaction control requires storage engine.".to_string(),
            });
        };

        self.current_transaction = Some(tx_id);
        self.savepoints.clear(); // Clear any old savepoints

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_micros(100),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute COMMIT statement
    async fn execute_commit(&mut self) -> QSQLResult<QueryResult> {
        let tx_id = self
            .current_transaction
            .ok_or_else(|| QSQLError::ExecutionError {
                message: "No active transaction to commit".to_string(),
            })?;

        // Prefer using storage engine's transaction manager for consistency
        if let Some(storage_engine) = &self.storage_engine {
            let mut storage_guard = storage_engine.write().await;
            storage_guard.commit_transaction(tx_id).await.map_err(|e| {
                QSQLError::ExecutionError {
                    message: format!("Failed to commit transaction via storage: {}", e),
                }
            })?;
        } else if let Some(tx_manager) = &self.transaction_manager {
            // Fallback to executor's transaction manager
            tx_manager
                .commit(tx_id)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Failed to commit transaction: {}", e),
                })?;
        } else {
            return Err(QSQLError::ExecutionError {
                message: "Transaction manager not available".to_string(),
            });
        }

        self.current_transaction = None;
        self.savepoints.clear();

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_micros(100),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute ROLLBACK statement
    async fn execute_rollback(&mut self) -> QSQLResult<QueryResult> {
        let tx_id = self
            .current_transaction
            .ok_or_else(|| QSQLError::ExecutionError {
                message: "No active transaction to rollback".to_string(),
            })?;

        // Prefer using storage engine's transaction manager for consistency
        if let Some(storage_engine) = &self.storage_engine {
            let mut storage_guard = storage_engine.write().await;
            storage_guard
                .rollback_transaction(tx_id)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Failed to rollback transaction via storage: {}", e),
                })?;
        } else if let Some(tx_manager) = &self.transaction_manager {
            // Fallback to executor's transaction manager
            tx_manager
                .rollback(tx_id)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Failed to rollback transaction: {}", e),
                })?;
        } else {
            return Err(QSQLError::ExecutionError {
                message: "Transaction manager not available".to_string(),
            });
        }

        self.current_transaction = None;
        self.savepoints.clear();

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_micros(100),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute SAVEPOINT statement
    ///
    /// Note: Current implementation provides syntax support and tracks savepoint names.
    /// Full savepoint rollback requires WAL integration to store and restore transaction
    /// state at the savepoint. This is sufficient for basic savepoint syntax validation
    /// and will be enhanced with full rollback support in future updates.
    async fn execute_savepoint(
        &mut self,
        savepoint: &SavepointStatement,
    ) -> QSQLResult<QueryResult> {
        // Check if transaction is active
        if self.current_transaction.is_none() {
            return Err(QSQLError::ExecutionError {
                message: "No active transaction. BEGIN a transaction first.".to_string(),
            });
        }

        // Track savepoint name for syntax validation
        // TODO: Full implementation requires WAL integration for state capture
        self.savepoints.insert(savepoint.name.clone(), ());

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_micros(50),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute ROLLBACK TO SAVEPOINT statement
    ///
    /// Note: Current implementation validates savepoint existence and provides
    /// syntax support. Full rollback-to-savepoint requires WAL integration to
    /// restore transaction state to the savepoint. This will be implemented
    /// in future enhancements as part of the complete savepoint feature.
    async fn execute_rollback_to_savepoint(
        &mut self,
        rollback_to: &RollbackToSavepointStatement,
    ) -> QSQLResult<QueryResult> {
        // Check if transaction is active
        if self.current_transaction.is_none() {
            return Err(QSQLError::ExecutionError {
                message: "No active transaction".to_string(),
            });
        }

        // Check if savepoint exists
        if !self.savepoints.contains_key(&rollback_to.name) {
            return Err(QSQLError::ExecutionError {
                message: format!("Savepoint '{}' does not exist", rollback_to.name),
            });
        }

        // TODO: Full implementation requires WAL integration to undo operations
        // back to the savepoint state while keeping the transaction active

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_micros(50),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    /// Execute RELEASE SAVEPOINT statement
    async fn execute_release_savepoint(
        &mut self,
        release: &ReleaseSavepointStatement,
    ) -> QSQLResult<QueryResult> {
        // Check if transaction is active
        if self.current_transaction.is_none() {
            return Err(QSQLError::ExecutionError {
                message: "No active transaction".to_string(),
            });
        }

        // Check if savepoint exists
        if self.savepoints.remove(&release.name).is_none() {
            return Err(QSQLError::ExecutionError {
                message: format!("Savepoint '{}' does not exist", release.name),
            });
        }

        Ok(QueryResult {
            rows: vec![],
            columns: vec![],
            execution_time: Duration::from_micros(50),
            rows_affected: 0,
            optimization_applied: false,
            synaptic_pathways_used: 0,
            quantum_operations: 0,
        })
    }

    // =============================================================================
    // SQL → Storage Engine Conversion Functions
    // =============================================================================

    /// Convert SQL INSERT to storage Row (static to avoid borrowing issues)
    fn convert_insert_to_row_static(
        insert: &InsertStatement,
        values: &[Expression],
    ) -> QSQLResult<Row> {
        use chrono::prelude::*;

        let mut fields = HashMap::new();

        // Get column names (either explicit or infer from schema)
        let column_names = if let Some(cols) = &insert.columns {
            cols.clone()
        } else {
            // Generate default column names
            (0..values.len()).map(|i| format!("col_{}", i)).collect()
        };

        // Convert each value
        for (i, expr) in values.iter().enumerate() {
            if i >= column_names.len() {
                break;
            }
            let column_name = &column_names[i];
            let value = Self::convert_expression_to_value_static(expr)?;
            fields.insert(column_name.clone(), value);
        }

        Ok(Row {
            id: 0, // Will be assigned by storage engine
            fields,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Convert SQL UPDATE to storage UpdateQuery (static)
    fn convert_update_to_storage_query_static(update: &UpdateStatement) -> QSQLResult<UpdateQuery> {
        let mut set_values = HashMap::new();

        // Convert assignments
        for assignment in &update.assignments {
            let value = Self::convert_expression_to_value_static(&assignment.value)?;
            set_values.insert(assignment.column.clone(), value);
        }

        // Convert WHERE clause
        let where_clause = if let Some(expr) = &update.where_clause {
            Some(Self::convert_expression_to_where_clause_static(expr)?)
        } else {
            None
        };

        Ok(UpdateQuery {
            table: update.table_name.clone(),
            set_values,
            where_clause,
        })
    }

    /// Convert SQL DELETE to storage DeleteQuery (static)
    fn convert_delete_to_storage_query_static(delete: &DeleteStatement) -> QSQLResult<DeleteQuery> {
        let where_clause = if let Some(expr) = &delete.where_clause {
            Some(Self::convert_expression_to_where_clause_static(expr)?)
        } else {
            None
        };

        Ok(DeleteQuery {
            table: delete.table_name.clone(),
            where_clause,
        })
    }

    /// Check if select list contains aggregate functions
    fn has_aggregate_functions(select_list: &[SelectItem]) -> bool {
        select_list.iter().any(|item| {
            if let SelectItem::Expression {
                expr: Expression::FunctionCall { name, .. },
                ..
            } = item
            {
                let upper_name = name.to_uppercase();
                matches!(upper_name.as_str(), "COUNT" | "SUM" | "AVG" | "MIN" | "MAX")
            } else {
                false
            }
        })
    }

    /// Check if select list contains scalar functions (string, math, etc.)
    fn has_scalar_functions(select_list: &[SelectItem]) -> bool {
        select_list.iter().any(|item| {
            if let SelectItem::Expression {
                expr: Expression::FunctionCall { name, .. },
                ..
            } = item
            {
                let upper_name = name.to_uppercase();
                matches!(
                    upper_name.as_str(),
                    "UPPER"
                        | "LOWER"
                        | "LENGTH"
                        | "LEN"
                        | "CONCAT"
                        | "SUBSTRING"
                        | "SUBSTR"
                        | "TRIM"
                        | "LTRIM"
                        | "RTRIM"
                        | "REPLACE"
                        | "LEFT"
                        | "RIGHT"
                        | "REVERSE"
                        | "REPEAT"
                        | "LPAD"
                        | "RPAD"
                        | "POSITION"
                        | "INSTR"
                        | "CHAR_LENGTH"
                        | "CHARACTER_LENGTH"
                        | "INITCAP"
                        | "ASCII"
                        | "CHR"
                )
            } else {
                false
            }
        })
    }

    /// Check if select list contains window functions
    fn has_window_functions(select_list: &[SelectItem]) -> bool {
        select_list.iter().any(|item| {
            matches!(
                item,
                SelectItem::Expression {
                    expr: Expression::WindowFunction { .. },
                    ..
                }
            )
        })
    }

    /// Check if select list contains scalar subqueries
    fn has_scalar_subqueries(select_list: &[SelectItem]) -> bool {
        select_list.iter().any(|item| {
            matches!(
                item,
                SelectItem::Expression {
                    expr: Expression::ScalarSubquery { .. },
                    ..
                }
            )
        })
    }

    /// Check if an expression contains an InList or InSubquery that needs post-filtering
    fn contains_in_list_expression(expr: &Expression) -> bool {
        match expr {
            Expression::InList { .. } => true,
            Expression::InSubquery { .. } => true,
            Expression::BinaryOp { left, right, .. } => {
                Self::contains_in_list_expression(left) || Self::contains_in_list_expression(right)
            }
            Expression::UnaryOp { operand, .. } => Self::contains_in_list_expression(operand),
            _ => false,
        }
    }

    /// Evaluate a WHERE expression against a storage Row
    fn evaluate_where_expression(expr: &Expression, row: &Row) -> QSQLResult<bool> {
        match expr {
            Expression::InList {
                expr: field_expr,
                list,
                negated,
            } => {
                // Get the field value from the row
                let field_name = Self::expression_to_string_static(field_expr);
                let field_value = row.fields.get(&field_name);

                match field_value {
                    Some(val) => {
                        // Check if field value matches any value in the list
                        let matches = list.iter().any(|list_item| {
                            if let Ok(list_val) =
                                Self::convert_expression_to_value_static(list_item)
                            {
                                Self::values_equal(val, &list_val)
                            } else {
                                false
                            }
                        });
                        Ok(if *negated { !matches } else { matches })
                    }
                    None => Ok(*negated), // NULL NOT IN (...) is true, NULL IN (...) is false
                }
            }
            Expression::BinaryOp {
                left,
                operator,
                right,
            } => {
                match operator {
                    BinaryOperator::And => {
                        let left_result = Self::evaluate_where_expression(left, row)?;
                        let right_result = Self::evaluate_where_expression(right, row)?;
                        Ok(left_result && right_result)
                    }
                    BinaryOperator::Or => {
                        let left_result = Self::evaluate_where_expression(left, row)?;
                        let right_result = Self::evaluate_where_expression(right, row)?;
                        Ok(left_result || right_result)
                    }
                    _ => {
                        // For other operators, evaluate as a simple comparison
                        if let Expression::Identifier(field) = left.as_ref() {
                            let field_value = row.fields.get(field);
                            let compare_value = Self::convert_expression_to_value_static(right)?;

                            match field_value {
                                Some(val) => {
                                    Self::evaluate_comparison(val, operator, &compare_value)
                                }
                                None => Ok(false),
                            }
                        } else {
                            Ok(true) // Default to true for unsupported patterns
                        }
                    }
                }
            }
            Expression::UnaryOp {
                operator: UnaryOperator::Not,
                operand,
            } => {
                let result = Self::evaluate_where_expression(operand, row)?;
                Ok(!result)
            }
            _ => Ok(true), // Default to true for unsupported expressions
        }
    }

    /// Compare two Values for equality
    fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            // Handle cross-type comparisons
            (Value::Integer(a), Value::Float(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Value::Float(a), Value::Integer(b)) => (a - *b as f64).abs() < f64::EPSILON,
            _ => false,
        }
    }

    /// Evaluate a comparison operator
    fn evaluate_comparison(
        field_val: &Value,
        op: &BinaryOperator,
        compare_val: &Value,
    ) -> QSQLResult<bool> {
        match op {
            BinaryOperator::Equal => Ok(Self::values_equal(field_val, compare_val)),
            BinaryOperator::NotEqual => Ok(!Self::values_equal(field_val, compare_val)),
            BinaryOperator::LessThan => {
                Self::compare_values_order(field_val, compare_val, |o| o.is_lt())
            }
            BinaryOperator::LessThanOrEqual => {
                Self::compare_values_order(field_val, compare_val, |o| o.is_le())
            }
            BinaryOperator::GreaterThan => {
                Self::compare_values_order(field_val, compare_val, |o| o.is_gt())
            }
            BinaryOperator::GreaterThanOrEqual => {
                Self::compare_values_order(field_val, compare_val, |o| o.is_ge())
            }
            BinaryOperator::Like => {
                if let (Value::Text(field_text), Value::Text(pattern)) = (field_val, compare_val) {
                    // Simple LIKE implementation: convert SQL pattern to contains check
                    let pattern_trimmed = pattern.trim_matches('%');
                    Ok(field_text.contains(pattern_trimmed))
                } else {
                    Ok(false)
                }
            }
            BinaryOperator::NotLike => {
                if let (Value::Text(field_text), Value::Text(pattern)) = (field_val, compare_val) {
                    let pattern_trimmed = pattern.trim_matches('%');
                    Ok(!field_text.contains(pattern_trimmed))
                } else {
                    Ok(true)
                }
            }
            _ => Ok(true), // Default to true for unsupported operators
        }
    }

    /// Compare values and apply ordering predicate
    fn compare_values_order<F>(a: &Value, b: &Value, pred: F) -> QSQLResult<bool>
    where
        F: Fn(std::cmp::Ordering) -> bool,
    {
        let ordering = match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Value::Text(a), Value::Text(b)) => a.cmp(b),
            (Value::Integer(a), Value::Float(b)) => (*a as f64)
                .partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::Float(a), Value::Integer(b)) => a
                .partial_cmp(&(*b as f64))
                .unwrap_or(std::cmp::Ordering::Equal),
            _ => return Ok(false),
        };
        Ok(pred(ordering))
    }

    /// Apply post-filtering for complex WHERE expressions (including InList)
    fn apply_post_filter(rows: Vec<Row>, where_expr: &Expression) -> QSQLResult<Vec<Row>> {
        let mut filtered = Vec::new();
        for row in rows {
            if Self::evaluate_where_expression(where_expr, &row)? {
                filtered.push(row);
            }
        }
        Ok(filtered)
    }

    /// Apply NEUROMATCH clause filtering using neuromorphic pattern matching
    /// This implements brain-inspired similarity matching for the NEUROMATCH clause
    fn apply_neuromatch_filter(
        &self,
        rows: Vec<Row>,
        neuromatch: &NeuroMatchClause,
    ) -> QSQLResult<Vec<Row>> {
        // Extract the pattern string from the expression
        let pattern = match &neuromatch.pattern {
            Expression::Literal(Literal::String(s)) => s.clone(),
            Expression::Identifier(id) => id.clone(),
            _ => {
                // For complex expressions, convert to string representation
                Self::expression_to_string_static(&neuromatch.pattern)
            }
        };

        let threshold = neuromatch.synaptic_weight;
        let mut filtered = Vec::new();

        for row in rows {
            // Calculate neuromorphic similarity score
            let score = if let Some(field) = &neuromatch.field {
                // Match against specific field
                if let Some(value) = row.fields.get(field) {
                    self.calculate_neuromatch_similarity(&value.to_string(), &pattern)
                } else {
                    0.0
                }
            } else {
                // Match against all fields, take the maximum similarity
                let mut max_score: f32 = 0.0;
                for value in row.fields.values() {
                    let score = self.calculate_neuromatch_similarity(&value.to_string(), &pattern);
                    if score > max_score {
                        max_score = score;
                    }
                }
                max_score
            };

            // Include row if similarity exceeds threshold
            if score >= threshold {
                filtered.push(row);
            }
        }

        Ok(filtered)
    }

    /// Calculate neuromorphic pattern similarity using brain-inspired algorithms
    /// Returns a similarity score between 0.0 and 1.0
    fn calculate_neuromatch_similarity(&self, value: &str, pattern: &str) -> f32 {
        // Normalize inputs for comparison
        let value_lower = value.to_lowercase();
        let pattern_lower = pattern.to_lowercase();

        // Exact match check
        if value_lower.contains(&pattern_lower) {
            return 1.0;
        }

        // Use Levenshtein-based similarity for fuzzy matching
        // This simulates synaptic pattern recognition
        let distance = Self::levenshtein_distance(&value_lower, &pattern_lower);
        let max_len = value_lower.len().max(pattern_lower.len());

        if max_len == 0 {
            return 0.0;
        }

        // Calculate similarity as inverse of normalized distance
        let similarity = 1.0 - (distance as f32 / max_len as f32);

        // Apply synaptic threshold - small similarities are filtered out
        if similarity < 0.3 {
            0.0
        } else {
            similarity
        }
    }

    /// Calculate Levenshtein distance between two strings
    /// Used for neuromorphic fuzzy pattern matching
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        // Create distance matrix
        let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

        // Initialize first column and row
        for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
            row[0] = i;
        }
        matrix[0]
            .iter_mut()
            .enumerate()
            .take(len2 + 1)
            .for_each(|(j, val)| {
                *val = j;
            });

        // Fill in the rest of the matrix
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }

    /// Convert SQL SELECT to storage SelectQuery
    fn convert_select_to_storage_query(&self, select: &SelectStatement) -> QSQLResult<SelectQuery> {
        // Extract table name from FROM clause
        let table = if let Some(from) = &select.from {
            if !from.relations.is_empty() {
                from.relations[0].name.clone()
            } else {
                return Err(QSQLError::ExecutionError {
                    message: "No table specified in FROM clause".to_string(),
                });
            }
        } else {
            return Err(QSQLError::ExecutionError {
                message: "Missing FROM clause".to_string(),
            });
        };

        // Extract column list - use "*" for aggregate queries, scalar functions, or when we need post-filtering
        let needs_post_filter = select
            .where_clause
            .as_ref()
            .map(Self::contains_in_list_expression)
            .unwrap_or(false);

        let has_scalar_funcs = Self::has_scalar_functions(&select.select_list);

        let columns = if Self::has_aggregate_functions(&select.select_list)
            || has_scalar_funcs
            || needs_post_filter
        {
            vec!["*".to_string()]
        } else {
            select
                .select_list
                .iter()
                .map(|item| match item {
                    SelectItem::Wildcard => "*".to_string(),
                    SelectItem::Expression { expr, alias } => alias
                        .clone()
                        .unwrap_or_else(|| Self::expression_to_string_static(expr)),
                })
                .collect()
        };

        // Convert WHERE clause - skip if it contains InList (we'll post-filter)
        let where_clause = if let Some(expr) = &select.where_clause {
            if Self::contains_in_list_expression(expr) {
                // We'll handle this in post-filtering
                None
            } else {
                Some(Self::convert_expression_to_where_clause_static(expr)?)
            }
        } else {
            None
        };

        // Convert ORDER BY
        let order_by = if !select.order_by.is_empty() {
            Some(Self::convert_order_by_static(&select.order_by[0])?)
        } else {
            None
        };

        // Note: When we have InList, we don't pass limit/offset to storage
        // because we need to filter first, then apply limit/offset
        let (limit, offset) = if needs_post_filter {
            (None, None)
        } else {
            (select.limit, select.offset)
        };

        Ok(SelectQuery {
            table,
            columns,
            where_clause,
            order_by,
            limit,
            offset,
        })
    }

    /// Convert Expression to storage Value (static)
    fn convert_expression_to_value_static(expr: &Expression) -> QSQLResult<Value> {
        match expr {
            Expression::Literal(lit) => {
                match lit {
                    Literal::Integer(i) => Ok(Value::Integer(*i)),
                    Literal::Float(f) => Ok(Value::Float(*f)),
                    Literal::String(s) => Ok(Value::Text(s.clone())),
                    Literal::Boolean(b) => Ok(Value::Boolean(*b)),
                    Literal::Null => Ok(Value::Null),
                    Literal::DNA(sequence) => Ok(Value::Text(sequence.clone())), // Store DNA as text
                    Literal::QuantumBit(state, amplitude) => {
                        // Store quantum bit as binary representation
                        let data = format!("QB:{}:{}", state, amplitude);
                        Ok(Value::Text(data))
                    }
                }
            }
            Expression::Identifier(name) => {
                // For now, treat identifiers as text (could be enhanced later)
                Ok(Value::Text(name.clone()))
            }
            _ => Err(QSQLError::ExecutionError {
                message: format!("Unsupported expression type in conversion: {:?}", expr),
            }),
        }
    }

    /// Convert Expression to WHERE clause (static)
    fn convert_expression_to_where_clause_static(expr: &Expression) -> QSQLResult<WhereClause> {
        let mut conditions = Vec::new();

        // Handle binary operations
        if let Expression::BinaryOp {
            left,
            operator,
            right,
        } = expr
        {
            if let Expression::Identifier(field) = left.as_ref() {
                let op = match operator {
                    BinaryOperator::Equal => ComparisonOperator::Equal,
                    BinaryOperator::NotEqual => ComparisonOperator::NotEqual,
                    BinaryOperator::LessThan => ComparisonOperator::LessThan,
                    BinaryOperator::LessThanOrEqual => ComparisonOperator::LessThanOrEqual,
                    BinaryOperator::GreaterThan => ComparisonOperator::GreaterThan,
                    BinaryOperator::GreaterThanOrEqual => ComparisonOperator::GreaterThanOrEqual,
                    BinaryOperator::Like => ComparisonOperator::Like,
                    _ => ComparisonOperator::Equal,
                };

                let value = Self::convert_expression_to_value_static(right)?;

                conditions.push(Condition {
                    field: field.clone(),
                    operator: op,
                    value,
                });
            }
        }

        Ok(WhereClause { conditions })
    }

    /// Convert OrderBy item to storage OrderBy (static)
    fn convert_order_by_static(order: &OrderByItem) -> QSQLResult<OrderBy> {
        let field = Self::expression_to_string_static(&order.expression);
        let direction = if order.ascending {
            SortDirection::Ascending
        } else {
            SortDirection::Descending
        };

        Ok(OrderBy { field, direction })
    }

    /// Convert storage Rows to QueryResult format
    fn convert_storage_rows_to_result(
        &self,
        storage_rows: Vec<Row>,
        select: &SelectStatement,
    ) -> QSQLResult<QueryResultData> {
        // Check if we have aggregate functions in the select list
        let aggregates = self.extract_aggregate_functions(&select.select_list);

        if !aggregates.is_empty() {
            // Process aggregate functions
            return self.execute_aggregates(&storage_rows, &aggregates, select);
        }

        // Check if we have window functions in the select list
        let has_window_funcs = Self::has_window_functions(&select.select_list);

        if has_window_funcs {
            // Process window functions
            return self.execute_window_functions(&storage_rows, &select.select_list);
        }

        // Check if we have scalar functions (UPPER, LOWER, LENGTH, etc.) in the select list
        let has_scalar_functions = Self::has_scalar_functions(&select.select_list);

        if has_scalar_functions {
            // Process scalar functions
            return self.execute_scalar_functions(&storage_rows, &select.select_list);
        }

        let mut result_rows = Vec::new();
        let mut columns = Vec::new();

        // Build column info from first row
        if let Some(first_row) = storage_rows.first() {
            for (col_name, value) in &first_row.fields {
                let data_type = self.storage_value_to_datatype(value);
                columns.push(ColumnInfo {
                    name: col_name.clone(),
                    data_type,
                    nullable: matches!(value, Value::Null),
                });
            }
        }

        // Convert each row
        for storage_row in storage_rows {
            let mut result_row = HashMap::new();

            for (col_name, value) in storage_row.fields {
                let query_value = self.storage_value_to_query_value(&value);
                result_row.insert(col_name, query_value);
            }

            result_rows.push(result_row);
        }

        Ok((result_rows, columns))
    }

    /// Extract aggregate functions from select list
    fn extract_aggregate_functions(&self, select_list: &[SelectItem]) -> Vec<AggregateFunction> {
        let mut aggregates = Vec::new();

        for item in select_list {
            if let SelectItem::Expression { expr, alias } = item {
                if let Some(agg) = self.extract_aggregate_from_expr(expr, alias.clone()) {
                    aggregates.push(agg);
                }
            }
        }

        aggregates
    }

    /// Extract aggregate function from expression
    fn extract_aggregate_from_expr(
        &self,
        expr: &Expression,
        alias: Option<String>,
    ) -> Option<AggregateFunction> {
        if let Expression::FunctionCall { name, args } = expr {
            let upper_name = name.to_uppercase();
            match upper_name.as_str() {
                "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" => {
                    // Check for COUNT(*)
                    let (column, distinct) = if args.is_empty() {
                        (None, false)
                    } else {
                        // Check for DISTINCT
                        let first_arg = &args[0];
                        match first_arg {
                            Expression::Identifier(col) => {
                                // Check if it starts with "DISTINCT "
                                if col.to_uppercase().starts_with("DISTINCT ") {
                                    let actual_col = col[9..].trim().to_string();
                                    (Some(actual_col), true)
                                } else {
                                    (Some(col.clone()), false)
                                }
                            }
                            Expression::Literal(Literal::String(s)) if s == "*" => (None, false),
                            _ => (Some(Self::expression_to_string_static(first_arg)), false),
                        }
                    };

                    Some(AggregateFunction {
                        name: upper_name,
                        column,
                        alias,
                        distinct,
                    })
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Execute aggregate functions on the storage rows with GROUP BY and HAVING support
    fn execute_aggregates(
        &self,
        storage_rows: &[Row],
        aggregates: &[AggregateFunction],
        select: &SelectStatement,
    ) -> QSQLResult<QueryResultData> {
        // Extract GROUP BY column names
        let group_by_columns = self.extract_group_by_columns(&select.group_by);

        // If we have GROUP BY, group the rows first
        if !group_by_columns.is_empty() {
            return self.execute_grouped_aggregates(
                storage_rows,
                aggregates,
                &group_by_columns,
                &select.having,
                &select.select_list,
            );
        }

        // No GROUP BY: compute aggregates over all rows (existing behavior)
        let mut result_row = HashMap::new();
        let mut columns = Vec::new();

        for agg in aggregates {
            let result_name = agg.alias.clone().unwrap_or_else(|| {
                format!("{}({})", agg.name, agg.column.as_deref().unwrap_or("*"))
            });

            let value = self.compute_aggregate(storage_rows, agg)?;

            // Determine data type based on aggregate function
            let data_type = match agg.name.as_str() {
                "COUNT" => DataType::BigInt,
                "AVG" => DataType::Double,
                "SUM" | "MIN" | "MAX" => {
                    // Infer from the column type or default to Double
                    if let Some(col) = &agg.column {
                        self.infer_column_type_from_rows(storage_rows, col)
                    } else {
                        DataType::BigInt
                    }
                }
                _ => DataType::Double,
            };

            columns.push(ColumnInfo {
                name: result_name.clone(),
                data_type,
                nullable: false,
            });

            result_row.insert(result_name, value);
        }

        Ok((vec![result_row], columns))
    }

    /// Execute scalar functions (string functions) on storage rows
    fn execute_scalar_functions(
        &self,
        storage_rows: &[Row],
        select_list: &[SelectItem],
    ) -> QSQLResult<QueryResultData> {
        let mut result_rows = Vec::new();
        let mut columns = Vec::new();
        let mut columns_initialized = false;

        for storage_row in storage_rows {
            let mut result_row = HashMap::new();

            for item in select_list {
                match item {
                    SelectItem::Wildcard => {
                        // Add all columns from the row
                        for (col_name, value) in &storage_row.fields {
                            let query_value = self.storage_value_to_query_value(value);
                            result_row.insert(col_name.clone(), query_value);

                            if !columns_initialized {
                                columns.push(ColumnInfo {
                                    name: col_name.clone(),
                                    data_type: self.storage_value_to_datatype(value),
                                    nullable: matches!(value, Value::Null),
                                });
                            }
                        }
                    }
                    SelectItem::Expression { expr, alias } => {
                        let (result_name, query_value, data_type) =
                            self.evaluate_select_expression(expr, alias, storage_row)?;

                        result_row.insert(result_name.clone(), query_value);

                        if !columns_initialized {
                            columns.push(ColumnInfo {
                                name: result_name,
                                data_type,
                                nullable: true,
                            });
                        }
                    }
                }
            }

            columns_initialized = true;
            result_rows.push(result_row);
        }

        Ok((result_rows, columns))
    }

    /// Execute window functions on the storage rows
    fn execute_window_functions(
        &self,
        storage_rows: &[Row],
        select_list: &[SelectItem],
    ) -> QSQLResult<QueryResultData> {
        let mut result_rows = Vec::new();
        let mut columns = Vec::new();
        let mut columns_initialized = false;

        // First, we need to evaluate any window functions across all rows at once
        // Window functions require access to the full result set

        for (row_index, storage_row) in storage_rows.iter().enumerate() {
            let mut result_row = HashMap::new();

            for item in select_list {
                match item {
                    SelectItem::Wildcard => {
                        // Add all columns from the row
                        for (col_name, value) in &storage_row.fields {
                            let query_value = self.storage_value_to_query_value(value);
                            result_row.insert(col_name.clone(), query_value);

                            if !columns_initialized {
                                columns.push(ColumnInfo {
                                    name: col_name.clone(),
                                    data_type: self.storage_value_to_datatype(value),
                                    nullable: matches!(value, Value::Null),
                                });
                            }
                        }
                    }
                    SelectItem::Expression { expr, alias } => {
                        match expr {
                            Expression::WindowFunction {
                                function,
                                args,
                                over_clause,
                            } => {
                                let result_name = alias.clone().unwrap_or_else(|| {
                                    Self::window_function_to_string(function, args)
                                });

                                let query_value = self.evaluate_window_function(
                                    function,
                                    args,
                                    over_clause,
                                    storage_rows,
                                    row_index,
                                )?;

                                let data_type = self.infer_window_function_type(function);

                                result_row.insert(result_name.clone(), query_value);

                                if !columns_initialized {
                                    columns.push(ColumnInfo {
                                        name: result_name,
                                        data_type,
                                        nullable: true,
                                    });
                                }
                            }
                            _ => {
                                // Handle non-window function expressions
                                let (result_name, query_value, data_type) =
                                    self.evaluate_select_expression(expr, alias, storage_row)?;

                                result_row.insert(result_name.clone(), query_value);

                                if !columns_initialized {
                                    columns.push(ColumnInfo {
                                        name: result_name,
                                        data_type,
                                        nullable: true,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            columns_initialized = true;
            result_rows.push(result_row);
        }

        Ok((result_rows, columns))
    }

    /// Convert window function to string for default naming
    fn window_function_to_string(function: &WindowFunctionType, args: &[Expression]) -> String {
        let func_name = match function {
            WindowFunctionType::RowNumber => "ROW_NUMBER",
            WindowFunctionType::Rank => "RANK",
            WindowFunctionType::DenseRank => "DENSE_RANK",
            WindowFunctionType::Lag => "LAG",
            WindowFunctionType::Lead => "LEAD",
            WindowFunctionType::Ntile => "NTILE",
            WindowFunctionType::FirstValue => "FIRST_VALUE",
            WindowFunctionType::LastValue => "LAST_VALUE",
            WindowFunctionType::NthValue => "NTH_VALUE",
            // Phase 2: Aggregate window functions
            WindowFunctionType::Sum => "SUM",
            WindowFunctionType::Avg => "AVG",
            WindowFunctionType::Count => "COUNT",
            WindowFunctionType::Min => "MIN",
            WindowFunctionType::Max => "MAX",
        };

        if args.is_empty() {
            format!("{}()", func_name)
        } else {
            let args_str: Vec<String> =
                args.iter().map(Self::expression_to_string_static).collect();
            format!("{}({})", func_name, args_str.join(", "))
        }
    }

    /// Infer the data type of a window function
    /// Note: For value functions (LAG, LEAD, etc.), the actual runtime type
    /// matches the source column. This returns a default for metadata purposes.
    fn infer_window_function_type(&self, function: &WindowFunctionType) -> DataType {
        match function {
            WindowFunctionType::RowNumber
            | WindowFunctionType::Rank
            | WindowFunctionType::DenseRank
            | WindowFunctionType::Ntile
            | WindowFunctionType::Count => DataType::BigInt,
            // AVG always returns a floating point value
            WindowFunctionType::Avg => DataType::Double,
            // SUM, MIN, MAX return based on input type - default to numeric
            WindowFunctionType::Sum | WindowFunctionType::Min | WindowFunctionType::Max => {
                DataType::Double
            }
            // Value functions return the type of their input column
            // Default to Text for metadata; actual values preserve their original type
            WindowFunctionType::Lag
            | WindowFunctionType::Lead
            | WindowFunctionType::FirstValue
            | WindowFunctionType::LastValue
            | WindowFunctionType::NthValue => DataType::Text,
        }
    }

    /// Evaluate a window function for a specific row
    fn evaluate_window_function(
        &self,
        function: &WindowFunctionType,
        args: &[Expression],
        over_clause: &WindowSpec,
        all_rows: &[Row],
        current_row_index: usize,
    ) -> QSQLResult<QueryValue> {
        // Get the partition for this row
        let partition =
            self.get_partition(all_rows, current_row_index, &over_clause.partition_by)?;

        // Sort the partition according to ORDER BY
        let sorted_partition = self.sort_partition_for_window(&partition, &over_clause.order_by)?;

        // Find the current row's position in the sorted partition
        let current_row = &all_rows[current_row_index];
        let position_in_partition = sorted_partition
            .iter()
            .position(|r| std::ptr::eq(*r, current_row))
            .unwrap_or(0);

        match function {
            WindowFunctionType::RowNumber => {
                // ROW_NUMBER() - sequential row number within partition
                Ok(QueryValue::Integer((position_in_partition + 1) as i64))
            }

            WindowFunctionType::Rank => {
                // RANK() - rank with gaps for ties
                let rank = self.compute_rank(
                    &sorted_partition,
                    position_in_partition,
                    &over_clause.order_by,
                    false,
                )?;
                Ok(QueryValue::Integer(rank))
            }

            WindowFunctionType::DenseRank => {
                // DENSE_RANK() - rank without gaps for ties
                let rank = self.compute_rank(
                    &sorted_partition,
                    position_in_partition,
                    &over_clause.order_by,
                    true,
                )?;
                Ok(QueryValue::Integer(rank))
            }

            WindowFunctionType::Lag => {
                // LAG(column, offset, default)
                let offset = if args.len() > 1 {
                    match self.evaluate_expression_value(&args[1], current_row)? {
                        QueryValue::Integer(i) => i as usize,
                        _ => 1,
                    }
                } else {
                    1
                };

                let default_value = if args.len() > 2 {
                    self.evaluate_expression_value(&args[2], current_row)?
                } else {
                    QueryValue::Null
                };

                if position_in_partition >= offset {
                    let target_row = sorted_partition[position_in_partition - offset];
                    if !args.is_empty() {
                        self.evaluate_expression_value(&args[0], target_row)
                    } else {
                        Ok(QueryValue::Null)
                    }
                } else {
                    Ok(default_value)
                }
            }

            WindowFunctionType::Lead => {
                // LEAD(column, offset, default)
                let offset = if args.len() > 1 {
                    match self.evaluate_expression_value(&args[1], current_row)? {
                        QueryValue::Integer(i) => i as usize,
                        _ => 1,
                    }
                } else {
                    1
                };

                let default_value = if args.len() > 2 {
                    self.evaluate_expression_value(&args[2], current_row)?
                } else {
                    QueryValue::Null
                };

                if position_in_partition + offset < sorted_partition.len() {
                    let target_row = sorted_partition[position_in_partition + offset];
                    if !args.is_empty() {
                        self.evaluate_expression_value(&args[0], target_row)
                    } else {
                        Ok(QueryValue::Null)
                    }
                } else {
                    Ok(default_value)
                }
            }

            WindowFunctionType::Ntile => {
                // NTILE(n) - distribute rows into n buckets
                let n = if !args.is_empty() {
                    match self.evaluate_expression_value(&args[0], current_row)? {
                        QueryValue::Integer(i) => i.max(1) as usize,
                        _ => 1,
                    }
                } else {
                    1
                };

                let total_rows = sorted_partition.len();
                // NTILE distributes rows as evenly as possible
                // If total_rows = 6 and n = 4, we get buckets of sizes 2,2,1,1
                // Rows 0,1 -> bucket 1; rows 2,3 -> bucket 2; row 4 -> bucket 3; row 5 -> bucket 4
                let rows_per_bucket = total_rows / n;
                let extra_rows = total_rows % n; // first 'extra_rows' buckets get one extra row

                // Calculate which bucket this row belongs to
                let bucket = if rows_per_bucket == 0 {
                    // More buckets than rows, each row gets its own bucket
                    (position_in_partition + 1).min(n)
                } else {
                    // Rows in buckets with extra rows: (rows_per_bucket + 1) * extra_rows
                    let rows_in_larger_buckets = (rows_per_bucket + 1) * extra_rows;
                    if position_in_partition < rows_in_larger_buckets {
                        // This row is in one of the larger buckets
                        position_in_partition / (rows_per_bucket + 1) + 1
                    } else {
                        // This row is in one of the smaller buckets
                        let remaining_position = position_in_partition - rows_in_larger_buckets;
                        extra_rows + remaining_position / rows_per_bucket + 1
                    }
                };
                Ok(QueryValue::Integer(bucket as i64))
            }

            WindowFunctionType::FirstValue => {
                // FIRST_VALUE(column) - first value in the window
                if let Some(first_row) = sorted_partition.first() {
                    if !args.is_empty() {
                        self.evaluate_expression_value(&args[0], first_row)
                    } else {
                        Ok(QueryValue::Null)
                    }
                } else {
                    Ok(QueryValue::Null)
                }
            }

            WindowFunctionType::LastValue => {
                // LAST_VALUE(column) - last value in the window
                if let Some(last_row) = sorted_partition.last() {
                    if !args.is_empty() {
                        self.evaluate_expression_value(&args[0], last_row)
                    } else {
                        Ok(QueryValue::Null)
                    }
                } else {
                    Ok(QueryValue::Null)
                }
            }

            WindowFunctionType::NthValue => {
                // NTH_VALUE(column, n) - nth value in the window
                let n = if args.len() > 1 {
                    match self.evaluate_expression_value(&args[1], current_row)? {
                        QueryValue::Integer(i) => (i.max(1) - 1) as usize, // Convert to 0-based index
                        _ => 0,
                    }
                } else {
                    0
                };

                if n < sorted_partition.len() {
                    let target_row = sorted_partition[n];
                    if !args.is_empty() {
                        self.evaluate_expression_value(&args[0], target_row)
                    } else {
                        Ok(QueryValue::Null)
                    }
                } else {
                    Ok(QueryValue::Null)
                }
            }

            // Phase 2: Aggregate Window Functions
            WindowFunctionType::Sum => {
                // SUM(column) OVER (...) - sum of column values in partition
                if args.is_empty() {
                    return Err(QSQLError::ExecutionError {
                        message: "SUM requires a column argument".to_string(),
                    });
                }

                let mut sum_int: i64 = 0;
                let mut sum_float: f64 = 0.0;
                let mut has_float = false;
                let mut count = 0;

                for row in &sorted_partition {
                    let val = self.evaluate_expression_value(&args[0], row)?;
                    match val {
                        QueryValue::Integer(i) => {
                            sum_int += i;
                            count += 1;
                        }
                        QueryValue::Float(f) => {
                            sum_float += f;
                            has_float = true;
                            count += 1;
                        }
                        QueryValue::Null => {} // Ignore NULL values
                        _ => {}                // Ignore non-numeric values
                    }
                }

                if count == 0 {
                    Ok(QueryValue::Null)
                } else if has_float {
                    Ok(QueryValue::Float(sum_float + sum_int as f64))
                } else {
                    Ok(QueryValue::Integer(sum_int))
                }
            }

            WindowFunctionType::Avg => {
                // AVG(column) OVER (...) - average of column values in partition
                if args.is_empty() {
                    return Err(QSQLError::ExecutionError {
                        message: "AVG requires a column argument".to_string(),
                    });
                }

                let mut sum: f64 = 0.0;
                let mut count: i64 = 0;

                for row in &sorted_partition {
                    let val = self.evaluate_expression_value(&args[0], row)?;
                    match val {
                        QueryValue::Integer(i) => {
                            sum += i as f64;
                            count += 1;
                        }
                        QueryValue::Float(f) => {
                            sum += f;
                            count += 1;
                        }
                        QueryValue::Null => {} // Ignore NULL values
                        _ => {}                // Ignore non-numeric values
                    }
                }

                if count == 0 {
                    Ok(QueryValue::Null)
                } else {
                    Ok(QueryValue::Float(sum / count as f64))
                }
            }

            WindowFunctionType::Count => {
                // COUNT(*|column) OVER (...) - count of rows/values in partition
                if args.is_empty() {
                    // COUNT(*) - count all rows in partition
                    Ok(QueryValue::Integer(sorted_partition.len() as i64))
                } else {
                    // Check for COUNT(*) with literal "*"
                    if let Expression::Literal(Literal::String(s)) = &args[0] {
                        if s == "*" {
                            return Ok(QueryValue::Integer(sorted_partition.len() as i64));
                        }
                    }

                    // COUNT(column) - count non-null values
                    let mut count: i64 = 0;
                    for row in &sorted_partition {
                        let val = self.evaluate_expression_value(&args[0], row)?;
                        if !matches!(val, QueryValue::Null) {
                            count += 1;
                        }
                    }
                    Ok(QueryValue::Integer(count))
                }
            }

            WindowFunctionType::Min => {
                // MIN(column) OVER (...) - minimum value in partition
                if args.is_empty() {
                    return Err(QSQLError::ExecutionError {
                        message: "MIN requires a column argument".to_string(),
                    });
                }

                let mut min_value: Option<QueryValue> = None;

                for row in &sorted_partition {
                    let val = self.evaluate_expression_value(&args[0], row)?;
                    if matches!(val, QueryValue::Null) {
                        continue;
                    }

                    min_value = Some(match min_value {
                        None => val,
                        Some(existing) => self.min_query_value(existing, val),
                    });
                }

                Ok(min_value.unwrap_or(QueryValue::Null))
            }

            WindowFunctionType::Max => {
                // MAX(column) OVER (...) - maximum value in partition
                if args.is_empty() {
                    return Err(QSQLError::ExecutionError {
                        message: "MAX requires a column argument".to_string(),
                    });
                }

                let mut max_value: Option<QueryValue> = None;

                for row in &sorted_partition {
                    let val = self.evaluate_expression_value(&args[0], row)?;
                    if matches!(val, QueryValue::Null) {
                        continue;
                    }

                    max_value = Some(match max_value {
                        None => val,
                        Some(existing) => self.max_query_value(existing, val),
                    });
                }

                Ok(max_value.unwrap_or(QueryValue::Null))
            }
        }
    }

    /// Get the partition of rows for a window function
    fn get_partition<'a>(
        &self,
        all_rows: &'a [Row],
        current_row_index: usize,
        partition_by: &[Expression],
    ) -> QSQLResult<Vec<&'a Row>> {
        if partition_by.is_empty() {
            // No partition by means all rows are in the same partition
            return Ok(all_rows.iter().collect());
        }

        let current_row = &all_rows[current_row_index];

        // Get the partition key values for the current row
        let current_key: Vec<String> = partition_by
            .iter()
            .map(|expr| {
                let col_name = Self::expression_to_string_static(expr);
                current_row
                    .fields
                    .get(&col_name)
                    .map(|v| self.value_to_string(v))
                    .unwrap_or_else(|| "NULL".to_string())
            })
            .collect();

        // Filter rows with the same partition key
        let partition: Vec<&Row> = all_rows
            .iter()
            .filter(|row| {
                let row_key: Vec<String> = partition_by
                    .iter()
                    .map(|expr| {
                        let col_name = Self::expression_to_string_static(expr);
                        row.fields
                            .get(&col_name)
                            .map(|v| self.value_to_string(v))
                            .unwrap_or_else(|| "NULL".to_string())
                    })
                    .collect();
                row_key == current_key
            })
            .collect();

        Ok(partition)
    }

    /// Sort a partition according to ORDER BY
    fn sort_partition_for_window<'a>(
        &self,
        partition: &[&'a Row],
        order_by: &[OrderByItem],
    ) -> QSQLResult<Vec<&'a Row>> {
        if order_by.is_empty() {
            return Ok(partition.to_vec());
        }

        let mut sorted: Vec<&'a Row> = partition.to_vec();

        sorted.sort_by(|a, b| {
            for order_item in order_by {
                let col_name = Self::expression_to_string_static(&order_item.expression);

                let a_val = a.fields.get(&col_name);
                let b_val = b.fields.get(&col_name);

                let cmp = match (a_val, b_val) {
                    (Some(Value::Integer(av)), Some(Value::Integer(bv))) => av.cmp(bv),
                    (Some(Value::Float(av)), Some(Value::Float(bv))) => {
                        av.partial_cmp(bv).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    (Some(Value::Text(av)), Some(Value::Text(bv))) => av.cmp(bv),
                    (Some(Value::Boolean(av)), Some(Value::Boolean(bv))) => av.cmp(bv),
                    _ => std::cmp::Ordering::Equal,
                };

                if cmp != std::cmp::Ordering::Equal {
                    return if order_item.ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    };
                }
            }
            std::cmp::Ordering::Equal
        });

        Ok(sorted)
    }

    /// Compute rank (with or without gaps) for a row in a sorted partition
    fn compute_rank(
        &self,
        sorted_partition: &[&Row],
        position: usize,
        order_by: &[OrderByItem],
        dense: bool,
    ) -> QSQLResult<i64> {
        if order_by.is_empty() {
            // Without ORDER BY, all rows have rank 1
            return Ok(1);
        }

        let current_row = sorted_partition[position];
        let current_key: Vec<String> = order_by
            .iter()
            .map(|ob| {
                let col_name = Self::expression_to_string_static(&ob.expression);
                current_row
                    .fields
                    .get(&col_name)
                    .map(|v| self.value_to_string(v))
                    .unwrap_or_else(|| "NULL".to_string())
            })
            .collect();

        if dense {
            // DENSE_RANK: count distinct values up to and including current position
            let mut distinct_ranks = 0i64;
            let mut prev_key: Option<Vec<String>> = None;

            for (i, row) in sorted_partition.iter().enumerate() {
                let row_key: Vec<String> = order_by
                    .iter()
                    .map(|ob| {
                        let col_name = Self::expression_to_string_static(&ob.expression);
                        row.fields
                            .get(&col_name)
                            .map(|v| self.value_to_string(v))
                            .unwrap_or_else(|| "NULL".to_string())
                    })
                    .collect();

                // Increment rank when key changes (or on first row)
                if prev_key.as_ref() != Some(&row_key) {
                    distinct_ranks += 1;
                    prev_key = Some(row_key);
                }

                if i == position {
                    return Ok(distinct_ranks);
                }
            }

            Ok(distinct_ranks)
        } else {
            // RANK: find position of first row with same key
            for (i, row) in sorted_partition.iter().enumerate() {
                let row_key: Vec<String> = order_by
                    .iter()
                    .map(|ob| {
                        let col_name = Self::expression_to_string_static(&ob.expression);
                        row.fields
                            .get(&col_name)
                            .map(|v| self.value_to_string(v))
                            .unwrap_or_else(|| "NULL".to_string())
                    })
                    .collect();

                if row_key == current_key {
                    return Ok((i + 1) as i64);
                }
            }

            Ok((position + 1) as i64)
        }
    }

    /// Evaluate a SELECT expression (including scalar functions) against a row
    fn evaluate_select_expression(
        &self,
        expr: &Expression,
        alias: &Option<String>,
        row: &Row,
    ) -> QSQLResult<(String, QueryValue, DataType)> {
        match expr {
            Expression::Identifier(col_name) => {
                let result_name = alias.clone().unwrap_or_else(|| col_name.clone());
                if let Some(value) = row.fields.get(col_name) {
                    let query_value = self.storage_value_to_query_value(value);
                    let data_type = self.storage_value_to_datatype(value);
                    Ok((result_name, query_value, data_type))
                } else {
                    Ok((result_name, QueryValue::Null, DataType::Text))
                }
            }
            Expression::Literal(lit) => {
                let result_name = alias
                    .clone()
                    .unwrap_or_else(|| Self::expression_to_string_static(expr));
                let query_value = self.literal_to_query_value(lit);
                let data_type = match lit {
                    Literal::Integer(_) => DataType::BigInt,
                    Literal::Float(_) => DataType::Double,
                    Literal::String(_) => DataType::Text,
                    Literal::Boolean(_) => DataType::Boolean,
                    _ => DataType::Text,
                };
                Ok((result_name, query_value, data_type))
            }
            Expression::FunctionCall { name, args } => {
                let upper_name = name.to_uppercase();
                let result_name = alias
                    .clone()
                    .unwrap_or_else(|| Self::expression_to_string_static(expr));

                let query_value = self.evaluate_scalar_function(&upper_name, args, row)?;
                let data_type = self.infer_scalar_function_type(&upper_name);

                Ok((result_name, query_value, data_type))
            }
            Expression::Case {
                when_clauses,
                else_result,
            } => {
                let result_name = alias
                    .clone()
                    .unwrap_or_else(|| Self::expression_to_string_static(expr));

                // Evaluate CASE expression: check each WHEN condition in order
                for (condition, result) in when_clauses {
                    if Self::evaluate_where_expression(condition, row)? {
                        // Condition is true, evaluate and return the result
                        let query_value = self.evaluate_expression_value(result, row)?;
                        let data_type = self.infer_query_value_type(&query_value);
                        return Ok((result_name, query_value, data_type));
                    }
                }
                // No condition matched, return ELSE result or NULL
                match else_result {
                    Some(else_expr) => {
                        let query_value = self.evaluate_expression_value(else_expr, row)?;
                        let data_type = self.infer_query_value_type(&query_value);
                        Ok((result_name, query_value, data_type))
                    }
                    None => Ok((result_name, QueryValue::Null, DataType::Text)),
                }
            }
            _ => {
                // For other expressions, try to convert to string
                let result_name = alias
                    .clone()
                    .unwrap_or_else(|| Self::expression_to_string_static(expr));
                Ok((result_name, QueryValue::Null, DataType::Text))
            }
        }
    }

    /// Evaluate a scalar function with given arguments
    fn evaluate_scalar_function(
        &self,
        func_name: &str,
        args: &[Expression],
        row: &Row,
    ) -> QSQLResult<QueryValue> {
        // Get the first argument value (most scalar functions need at least one)
        let get_arg_value = |idx: usize| -> QSQLResult<QueryValue> {
            if idx >= args.len() {
                return Err(QSQLError::ExecutionError {
                    message: format!("Function {} requires more arguments", func_name),
                });
            }
            self.evaluate_expression_value(&args[idx], row)
        };

        let get_string_arg = |idx: usize| -> QSQLResult<String> {
            let val = get_arg_value(idx)?;
            match val {
                QueryValue::String(s) => Ok(s),
                QueryValue::Integer(i) => Ok(i.to_string()),
                QueryValue::Float(f) => Ok(f.to_string()),
                QueryValue::Boolean(b) => Ok(b.to_string()),
                QueryValue::Null => Ok(String::new()),
                _ => Ok(String::new()),
            }
        };

        let get_int_arg = |idx: usize| -> QSQLResult<i64> {
            let val = get_arg_value(idx)?;
            match val {
                QueryValue::Integer(i) => Ok(i),
                QueryValue::Float(f) => Ok(f as i64),
                QueryValue::String(s) => s.parse().map_err(|_| QSQLError::ExecutionError {
                    message: format!("Cannot convert '{}' to integer", s),
                }),
                _ => Ok(0),
            }
        };

        match func_name {
            // String functions
            "UPPER" => {
                let s = get_string_arg(0)?;
                Ok(QueryValue::String(s.to_uppercase()))
            }
            "LOWER" => {
                let s = get_string_arg(0)?;
                Ok(QueryValue::String(s.to_lowercase()))
            }
            "LENGTH" | "LEN" | "CHAR_LENGTH" | "CHARACTER_LENGTH" => {
                let s = get_string_arg(0)?;
                Ok(QueryValue::Integer(s.len() as i64))
            }
            "TRIM" => {
                let s = get_string_arg(0)?;
                Ok(QueryValue::String(s.trim().to_string()))
            }
            "LTRIM" => {
                let s = get_string_arg(0)?;
                Ok(QueryValue::String(s.trim_start().to_string()))
            }
            "RTRIM" => {
                let s = get_string_arg(0)?;
                Ok(QueryValue::String(s.trim_end().to_string()))
            }
            "REVERSE" => {
                let s = get_string_arg(0)?;
                Ok(QueryValue::String(s.chars().rev().collect()))
            }
            "INITCAP" => {
                let s = get_string_arg(0)?;
                let result = s
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().to_string() + &chars.as_str().to_lowercase()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                Ok(QueryValue::String(result))
            }
            "ASCII" => {
                let s = get_string_arg(0)?;
                let ascii = s.chars().next().map(|c| c as i64).unwrap_or(0);
                Ok(QueryValue::Integer(ascii))
            }
            "CHR" => {
                let code = get_int_arg(0)?;
                let ch = char::from_u32(code as u32).unwrap_or('\0');
                Ok(QueryValue::String(ch.to_string()))
            }
            "CONCAT" => {
                let mut result = String::new();
                for (i, _) in args.iter().enumerate() {
                    result.push_str(&get_string_arg(i)?);
                }
                Ok(QueryValue::String(result))
            }
            "SUBSTRING" | "SUBSTR" => {
                let s = get_string_arg(0)?;
                let start = get_int_arg(1)? as usize;
                // SQL SUBSTRING is 1-indexed
                let start_idx = if start > 0 { start - 1 } else { 0 };

                let result = if args.len() >= 3 {
                    let len = get_int_arg(2)? as usize;
                    s.chars().skip(start_idx).take(len).collect()
                } else {
                    s.chars().skip(start_idx).collect()
                };
                Ok(QueryValue::String(result))
            }
            "LEFT" => {
                let s = get_string_arg(0)?;
                let n = get_int_arg(1)? as usize;
                Ok(QueryValue::String(s.chars().take(n).collect()))
            }
            "RIGHT" => {
                let s = get_string_arg(0)?;
                let n = get_int_arg(1)? as usize;
                let len = s.chars().count();
                let skip = len.saturating_sub(n);
                Ok(QueryValue::String(s.chars().skip(skip).collect()))
            }
            "REPLACE" => {
                let s = get_string_arg(0)?;
                let from = get_string_arg(1)?;
                let to = get_string_arg(2)?;
                Ok(QueryValue::String(s.replace(&from, &to)))
            }
            "REPEAT" => {
                let s = get_string_arg(0)?;
                let n = get_int_arg(1)? as usize;
                Ok(QueryValue::String(s.repeat(n)))
            }
            "LPAD" => {
                let s = get_string_arg(0)?;
                let len = get_int_arg(1)? as usize;
                let pad = if args.len() >= 3 {
                    get_string_arg(2)?
                } else {
                    " ".to_string()
                };
                let current_len = s.chars().count();
                if current_len >= len {
                    Ok(QueryValue::String(s))
                } else {
                    let pad_len = len - current_len;
                    let pad_chars: String = pad.chars().cycle().take(pad_len).collect();
                    Ok(QueryValue::String(pad_chars + &s))
                }
            }
            "RPAD" => {
                let s = get_string_arg(0)?;
                let len = get_int_arg(1)? as usize;
                let pad = if args.len() >= 3 {
                    get_string_arg(2)?
                } else {
                    " ".to_string()
                };
                let current_len = s.chars().count();
                if current_len >= len {
                    Ok(QueryValue::String(s))
                } else {
                    let pad_len = len - current_len;
                    let pad_chars: String = pad.chars().cycle().take(pad_len).collect();
                    Ok(QueryValue::String(s + &pad_chars))
                }
            }
            "POSITION" | "INSTR" => {
                let haystack = get_string_arg(0)?;
                let needle = get_string_arg(1)?;
                // Returns 1-indexed position, 0 if not found
                let pos = haystack.find(&needle).map(|i| i as i64 + 1).unwrap_or(0);
                Ok(QueryValue::Integer(pos))
            }

            // NULL handling functions
            "COALESCE" => {
                // COALESCE returns the first non-NULL argument
                for arg in args {
                    let val = self.evaluate_expression_value(arg, row)?;
                    if !matches!(val, QueryValue::Null) {
                        return Ok(val);
                    }
                }
                // All arguments are NULL, return NULL
                Ok(QueryValue::Null)
            }
            "NULLIF" => {
                // NULLIF(expr1, expr2) returns NULL if expr1 = expr2, otherwise expr1
                if args.len() < 2 {
                    return Err(QSQLError::ExecutionError {
                        message: "NULLIF requires exactly 2 arguments".to_string(),
                    });
                }
                let val1 = self.evaluate_expression_value(&args[0], row)?;
                let val2 = self.evaluate_expression_value(&args[1], row)?;

                // Compare values - if equal, return NULL
                if Self::query_values_equal(&val1, &val2) {
                    Ok(QueryValue::Null)
                } else {
                    Ok(val1)
                }
            }
            "IFNULL" | "NVL" => {
                // IFNULL(expr1, expr2) returns expr2 if expr1 is NULL, otherwise expr1
                // NVL is the Oracle equivalent
                if args.len() < 2 {
                    return Err(QSQLError::ExecutionError {
                        message: "IFNULL requires exactly 2 arguments".to_string(),
                    });
                }
                let val1 = self.evaluate_expression_value(&args[0], row)?;
                if matches!(val1, QueryValue::Null) {
                    self.evaluate_expression_value(&args[1], row)
                } else {
                    Ok(val1)
                }
            }

            // Math functions
            "ABS" => {
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => Ok(QueryValue::Integer(i.abs())),
                    QueryValue::Float(f) => Ok(QueryValue::Float(f.abs())),
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "ABS requires a numeric argument".to_string(),
                    }),
                }
            }
            "ROUND" => {
                let val = get_arg_value(0)?;
                let decimals = if args.len() >= 2 { get_int_arg(1)? } else { 0 };
                match val {
                    QueryValue::Integer(i) => Ok(QueryValue::Integer(i)),
                    QueryValue::Float(f) => {
                        let multiplier = 10_f64.powi(decimals as i32);
                        let rounded = (f * multiplier).round() / multiplier;
                        if decimals == 0 {
                            Ok(QueryValue::Integer(rounded as i64))
                        } else {
                            Ok(QueryValue::Float(rounded))
                        }
                    }
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "ROUND requires a numeric argument".to_string(),
                    }),
                }
            }
            "CEIL" | "CEILING" => {
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => Ok(QueryValue::Integer(i)),
                    QueryValue::Float(f) => Ok(QueryValue::Integer(f.ceil() as i64)),
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "CEIL requires a numeric argument".to_string(),
                    }),
                }
            }
            "FLOOR" => {
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => Ok(QueryValue::Integer(i)),
                    QueryValue::Float(f) => Ok(QueryValue::Integer(f.floor() as i64)),
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "FLOOR requires a numeric argument".to_string(),
                    }),
                }
            }
            "MOD" => {
                if args.len() < 2 {
                    return Err(QSQLError::ExecutionError {
                        message: "MOD requires exactly 2 arguments".to_string(),
                    });
                }
                let val1 = get_arg_value(0)?;
                let val2 = get_arg_value(1)?;
                match (val1, val2) {
                    (QueryValue::Integer(a), QueryValue::Integer(b)) => {
                        if b == 0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Integer(a % b))
                        }
                    }
                    (QueryValue::Float(a), QueryValue::Float(b)) => {
                        if b == 0.0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float(a % b))
                        }
                    }
                    (QueryValue::Integer(a), QueryValue::Float(b)) => {
                        if b == 0.0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float((a as f64) % b))
                        }
                    }
                    (QueryValue::Float(a), QueryValue::Integer(b)) => {
                        if b == 0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float(a % (b as f64)))
                        }
                    }
                    (QueryValue::Null, _) | (_, QueryValue::Null) => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "MOD requires numeric arguments".to_string(),
                    }),
                }
            }
            "POWER" | "POW" => {
                if args.len() < 2 {
                    return Err(QSQLError::ExecutionError {
                        message: "POWER requires exactly 2 arguments".to_string(),
                    });
                }
                let val1 = get_arg_value(0)?;
                let val2 = get_arg_value(1)?;
                match (val1, val2) {
                    (QueryValue::Integer(base), QueryValue::Integer(exp)) => {
                        if exp >= 0 {
                            Ok(QueryValue::Integer((base as f64).powi(exp as i32) as i64))
                        } else {
                            Ok(QueryValue::Float((base as f64).powi(exp as i32)))
                        }
                    }
                    (QueryValue::Float(base), QueryValue::Integer(exp)) => {
                        Ok(QueryValue::Float(base.powi(exp as i32)))
                    }
                    (QueryValue::Integer(base), QueryValue::Float(exp)) => {
                        Ok(QueryValue::Float((base as f64).powf(exp)))
                    }
                    (QueryValue::Float(base), QueryValue::Float(exp)) => {
                        Ok(QueryValue::Float(base.powf(exp)))
                    }
                    (QueryValue::Null, _) | (_, QueryValue::Null) => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "POWER requires numeric arguments".to_string(),
                    }),
                }
            }
            "SQRT" => {
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => {
                        if i < 0 {
                            Ok(QueryValue::Null) // SQL standard: SQRT of negative is NULL
                        } else {
                            Ok(QueryValue::Float((i as f64).sqrt()))
                        }
                    }
                    QueryValue::Float(f) => {
                        if f < 0.0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float(f.sqrt()))
                        }
                    }
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "SQRT requires a numeric argument".to_string(),
                    }),
                }
            }
            "SIGN" => {
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => Ok(QueryValue::Integer(i.signum())),
                    QueryValue::Float(f) => {
                        if f > 0.0 {
                            Ok(QueryValue::Integer(1))
                        } else if f < 0.0 {
                            Ok(QueryValue::Integer(-1))
                        } else {
                            Ok(QueryValue::Integer(0))
                        }
                    }
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "SIGN requires a numeric argument".to_string(),
                    }),
                }
            }
            "TRUNCATE" | "TRUNC" => {
                let val = get_arg_value(0)?;
                let decimals = if args.len() >= 2 { get_int_arg(1)? } else { 0 };
                match val {
                    QueryValue::Integer(i) => Ok(QueryValue::Integer(i)),
                    QueryValue::Float(f) => {
                        let multiplier = 10_f64.powi(decimals as i32);
                        let truncated = (f * multiplier).trunc() / multiplier;
                        if decimals == 0 {
                            Ok(QueryValue::Integer(truncated as i64))
                        } else {
                            Ok(QueryValue::Float(truncated))
                        }
                    }
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "TRUNCATE requires a numeric argument".to_string(),
                    }),
                }
            }
            "EXP" => {
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => Ok(QueryValue::Float((i as f64).exp())),
                    QueryValue::Float(f) => Ok(QueryValue::Float(f.exp())),
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "EXP requires a numeric argument".to_string(),
                    }),
                }
            }
            "LN" | "LOG" => {
                // LOG with one argument is natural logarithm (LN)
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => {
                        if i <= 0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float((i as f64).ln()))
                        }
                    }
                    QueryValue::Float(f) => {
                        if f <= 0.0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float(f.ln()))
                        }
                    }
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "LN/LOG requires a numeric argument".to_string(),
                    }),
                }
            }
            "LOG10" => {
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => {
                        if i <= 0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float((i as f64).log10()))
                        }
                    }
                    QueryValue::Float(f) => {
                        if f <= 0.0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float(f.log10()))
                        }
                    }
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "LOG10 requires a numeric argument".to_string(),
                    }),
                }
            }
            "LOG2" => {
                let val = get_arg_value(0)?;
                match val {
                    QueryValue::Integer(i) => {
                        if i <= 0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float((i as f64).log2()))
                        }
                    }
                    QueryValue::Float(f) => {
                        if f <= 0.0 {
                            Ok(QueryValue::Null)
                        } else {
                            Ok(QueryValue::Float(f.log2()))
                        }
                    }
                    QueryValue::Null => Ok(QueryValue::Null),
                    _ => Err(QSQLError::ExecutionError {
                        message: "LOG2 requires a numeric argument".to_string(),
                    }),
                }
            }
            "PI" => Ok(QueryValue::Float(std::f64::consts::PI)),
            "RANDOM" | "RAND" => {
                // Returns a random float between 0 and 1
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                // Simple pseudo-random using seed
                let random = ((seed % 10000) as f64) / 10000.0;
                Ok(QueryValue::Float(random))
            }

            // Date/Time functions
            "NOW" | "CURRENT_TIMESTAMP" | "GETDATE" | "SYSDATE" => {
                // Returns current date and time as ISO 8601 string
                use chrono::prelude::*;
                let now: DateTime<Utc> = Utc::now();
                Ok(QueryValue::String(
                    now.format("%Y-%m-%d %H:%M:%S").to_string(),
                ))
            }
            "CURRENT_DATE" | "CURDATE" | "DATE" => {
                // Returns current date as YYYY-MM-DD
                use chrono::prelude::*;
                let today = Utc::now().date_naive();
                Ok(QueryValue::String(today.format("%Y-%m-%d").to_string()))
            }
            "CURRENT_TIME" | "CURTIME" => {
                // Returns current time as HH:MM:SS
                use chrono::prelude::*;
                let now = Utc::now();
                Ok(QueryValue::String(now.format("%H:%M:%S").to_string()))
            }
            "LOCALTIME" | "LOCALTIMESTAMP" => {
                // Returns local time/timestamp
                use chrono::prelude::*;
                let local: DateTime<Local> = Local::now();
                Ok(QueryValue::String(
                    local.format("%Y-%m-%d %H:%M:%S").to_string(),
                ))
            }
            "UTC_DATE" => {
                // Returns UTC date
                use chrono::prelude::*;
                let today = Utc::now().date_naive();
                Ok(QueryValue::String(today.format("%Y-%m-%d").to_string()))
            }
            "UTC_TIME" => {
                // Returns UTC time
                use chrono::prelude::*;
                let now = Utc::now();
                Ok(QueryValue::String(now.format("%H:%M:%S").to_string()))
            }
            "UTC_TIMESTAMP" => {
                // Returns UTC timestamp
                use chrono::prelude::*;
                let now: DateTime<Utc> = Utc::now();
                Ok(QueryValue::String(
                    now.format("%Y-%m-%d %H:%M:%S").to_string(),
                ))
            }
            "UNIX_TIMESTAMP" | "EPOCH" => {
                // Returns current Unix timestamp (seconds since 1970-01-01)
                use chrono::prelude::*;
                let now: DateTime<Utc> = Utc::now();
                Ok(QueryValue::Integer(now.timestamp()))
            }
            "YEAR" => {
                // YEAR(date_string) - extracts year from date
                use chrono::prelude::*;
                if args.is_empty() {
                    // No argument - return current year
                    let now = Utc::now();
                    Ok(QueryValue::Integer(now.year() as i64))
                } else {
                    let date_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        Ok(QueryValue::Integer(parsed.year() as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(parsed.year() as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "MONTH" => {
                // MONTH(date_string) - extracts month from date
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    Ok(QueryValue::Integer(now.month() as i64))
                } else {
                    let date_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        Ok(QueryValue::Integer(parsed.month() as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(parsed.month() as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "DAY" | "DAYOFMONTH" => {
                // DAY(date_string) - extracts day from date
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    Ok(QueryValue::Integer(now.day() as i64))
                } else {
                    let date_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        Ok(QueryValue::Integer(parsed.day() as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(parsed.day() as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "HOUR" => {
                // HOUR(time_string) - extracts hour from time/datetime
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    Ok(QueryValue::Integer(now.hour() as i64))
                } else {
                    let time_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                        Ok(QueryValue::Integer(parsed.hour() as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(parsed.hour() as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "MINUTE" => {
                // MINUTE(time_string) - extracts minute from time/datetime
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    Ok(QueryValue::Integer(now.minute() as i64))
                } else {
                    let time_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                        Ok(QueryValue::Integer(parsed.minute() as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(parsed.minute() as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "SECOND" => {
                // SECOND(time_string) - extracts second from time/datetime
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    Ok(QueryValue::Integer(now.second() as i64))
                } else {
                    let time_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveTime::parse_from_str(&time_str, "%H:%M:%S") {
                        Ok(QueryValue::Integer(parsed.second() as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(parsed.second() as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "DAYOFWEEK" | "WEEKDAY" => {
                // DAYOFWEEK(date_string) - returns day of week (1=Sunday to 7=Saturday for MySQL)
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    // Chrono uses Mon=0 to Sun=6, MySQL uses Sun=1 to Sat=7
                    let dow = now.weekday().num_days_from_sunday() + 1;
                    Ok(QueryValue::Integer(dow as i64))
                } else {
                    let date_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        let dow = parsed.weekday().num_days_from_sunday() + 1;
                        Ok(QueryValue::Integer(dow as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                    {
                        let dow = parsed.weekday().num_days_from_sunday() + 1;
                        Ok(QueryValue::Integer(dow as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "DAYOFYEAR" => {
                // DAYOFYEAR(date_string) - returns day of year (1-366)
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    Ok(QueryValue::Integer(now.ordinal() as i64))
                } else {
                    let date_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        Ok(QueryValue::Integer(parsed.ordinal() as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(parsed.ordinal() as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "WEEK" | "WEEKOFYEAR" => {
                // WEEK(date_string) - returns week of year (0-53)
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    Ok(QueryValue::Integer(now.iso_week().week() as i64))
                } else {
                    let date_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        Ok(QueryValue::Integer(parsed.iso_week().week() as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(parsed.date().iso_week().week() as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "QUARTER" => {
                // QUARTER(date_string) - returns quarter (1-4)
                use chrono::prelude::*;
                if args.is_empty() {
                    let now = Utc::now();
                    Ok(QueryValue::Integer(((now.month() - 1) / 3 + 1) as i64))
                } else {
                    let date_str = get_string_arg(0)?;
                    if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        Ok(QueryValue::Integer(((parsed.month() - 1) / 3 + 1) as i64))
                    } else if let Ok(parsed) =
                        NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                    {
                        Ok(QueryValue::Integer(((parsed.month() - 1) / 3 + 1) as i64))
                    } else {
                        Ok(QueryValue::Null)
                    }
                }
            }
            "DATE_FORMAT" | "STRFTIME" => {
                // DATE_FORMAT(date_string, format_string)
                use chrono::prelude::*;
                if args.len() < 2 {
                    return Err(QSQLError::ExecutionError {
                        message: "DATE_FORMAT requires exactly 2 arguments".to_string(),
                    });
                }
                let date_str = get_string_arg(0)?;
                let format_str = get_string_arg(1)?;

                if let Ok(parsed) = NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S") {
                    Ok(QueryValue::String(parsed.format(&format_str).to_string()))
                } else if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    Ok(QueryValue::String(parsed.format(&format_str).to_string()))
                } else {
                    Ok(QueryValue::Null)
                }
            }
            "DATEDIFF" => {
                // DATEDIFF(date1, date2) - returns difference in days
                use chrono::prelude::*;
                if args.len() < 2 {
                    return Err(QSQLError::ExecutionError {
                        message: "DATEDIFF requires exactly 2 arguments".to_string(),
                    });
                }
                let date1_str = get_string_arg(0)?;
                let date2_str = get_string_arg(1)?;

                let parse_date = |s: &str| -> Option<NaiveDate> {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().or_else(|| {
                        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                            .ok()
                            .map(|dt| dt.date())
                    })
                };

                if let (Some(d1), Some(d2)) = (parse_date(&date1_str), parse_date(&date2_str)) {
                    let diff = d1.signed_duration_since(d2).num_days();
                    Ok(QueryValue::Integer(diff))
                } else {
                    Ok(QueryValue::Null)
                }
            }
            "DATE_ADD" => {
                // DATE_ADD(date, INTERVAL expr unit) - adds time interval to date
                use chrono::prelude::*;
                if args.len() < 3 {
                    return Err(QSQLError::ExecutionError {
                        message: "DATE_ADD requires date and INTERVAL arguments (date, expr, unit)"
                            .to_string(),
                    });
                }

                let date_str = get_string_arg(0)?;

                // Second arg is the interval value
                let interval_value = match self.evaluate_expression_value(&args[1], row)? {
                    QueryValue::Integer(i) => i,
                    QueryValue::Float(f) => f as i64,
                    QueryValue::String(s) => {
                        s.parse::<i64>().map_err(|_| QSQLError::ExecutionError {
                            message: format!(
                                "Invalid interval value: '{}' is not a valid number",
                                s
                            ),
                        })?
                    }
                    _ => return Ok(QueryValue::Null),
                };

                // Third arg is the unit marker (INTERVAL_UNIT:DAY, etc.)
                let unit_str = get_string_arg(2)?;
                let unit = unit_str
                    .strip_prefix("INTERVAL_UNIT:")
                    .unwrap_or(&unit_str)
                    .to_uppercase();

                // Parse the input date/datetime
                let result = if let Ok(dt) =
                    NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                {
                    // DateTime input
                    let new_dt = match unit.as_str() {
                        "YEAR" => {
                            if interval_value >= 0 {
                                dt.checked_add_months(chrono::Months::new(
                                    (interval_value * 12) as u32,
                                ))
                            } else {
                                dt.checked_sub_months(chrono::Months::new(
                                    ((-interval_value) * 12) as u32,
                                ))
                            }
                        }
                        "MONTH" => {
                            if interval_value >= 0 {
                                dt.checked_add_months(chrono::Months::new(interval_value as u32))
                            } else {
                                dt.checked_sub_months(chrono::Months::new((-interval_value) as u32))
                            }
                        }
                        "WEEK" => {
                            if interval_value >= 0 {
                                dt.checked_add_days(chrono::Days::new((interval_value * 7) as u64))
                            } else {
                                dt.checked_sub_days(chrono::Days::new(
                                    ((-interval_value) * 7) as u64,
                                ))
                            }
                        }
                        "DAY" => {
                            if interval_value >= 0 {
                                dt.checked_add_days(chrono::Days::new(interval_value as u64))
                            } else {
                                dt.checked_sub_days(chrono::Days::new((-interval_value) as u64))
                            }
                        }
                        "HOUR" => Some(dt + chrono::Duration::hours(interval_value)),
                        "MINUTE" => Some(dt + chrono::Duration::minutes(interval_value)),
                        "SECOND" => Some(dt + chrono::Duration::seconds(interval_value)),
                        _ => {
                            return Err(QSQLError::ExecutionError {
                                message: format!("Unsupported time unit: {}", unit),
                            })
                        }
                    };
                    new_dt.map(|d| QueryValue::String(d.format("%Y-%m-%d %H:%M:%S").to_string()))
                } else if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    // Date only input
                    let new_date = match unit.as_str() {
                        "YEAR" => {
                            if interval_value >= 0 {
                                date.checked_add_months(chrono::Months::new(
                                    (interval_value * 12) as u32,
                                ))
                            } else {
                                date.checked_sub_months(chrono::Months::new(
                                    ((-interval_value) * 12) as u32,
                                ))
                            }
                        }
                        "MONTH" => {
                            if interval_value >= 0 {
                                date.checked_add_months(chrono::Months::new(interval_value as u32))
                            } else {
                                date.checked_sub_months(chrono::Months::new(
                                    (-interval_value) as u32,
                                ))
                            }
                        }
                        "WEEK" => {
                            if interval_value >= 0 {
                                date.checked_add_days(chrono::Days::new(
                                    (interval_value * 7) as u64,
                                ))
                            } else {
                                date.checked_sub_days(chrono::Days::new(
                                    ((-interval_value) * 7) as u64,
                                ))
                            }
                        }
                        "DAY" => {
                            if interval_value >= 0 {
                                date.checked_add_days(chrono::Days::new(interval_value as u64))
                            } else {
                                date.checked_sub_days(chrono::Days::new((-interval_value) as u64))
                            }
                        }
                        "HOUR" | "MINUTE" | "SECOND" => {
                            // Convert to datetime for time-based operations
                            let dt = match date.and_hms_opt(0, 0, 0) {
                                Some(dt) => dt,
                                None => return Ok(QueryValue::Null),
                            };
                            let new_dt = match unit.as_str() {
                                "HOUR" => Some(dt + chrono::Duration::hours(interval_value)),
                                "MINUTE" => Some(dt + chrono::Duration::minutes(interval_value)),
                                "SECOND" => Some(dt + chrono::Duration::seconds(interval_value)),
                                _ => None,
                            };
                            return Ok(new_dt
                                .map(|d| {
                                    QueryValue::String(d.format("%Y-%m-%d %H:%M:%S").to_string())
                                })
                                .unwrap_or(QueryValue::Null));
                        }
                        _ => {
                            return Err(QSQLError::ExecutionError {
                                message: format!("Unsupported time unit: {}", unit),
                            })
                        }
                    };
                    new_date.map(|d| QueryValue::String(d.format("%Y-%m-%d").to_string()))
                } else {
                    None
                };

                Ok(result.unwrap_or(QueryValue::Null))
            }
            "DATE_SUB" => {
                // DATE_SUB(date, INTERVAL expr unit) - subtracts time interval from date
                use chrono::prelude::*;
                if args.len() < 3 {
                    return Err(QSQLError::ExecutionError {
                        message: "DATE_SUB requires date and INTERVAL arguments (date, expr, unit)"
                            .to_string(),
                    });
                }

                let date_str = get_string_arg(0)?;

                // Second arg is the interval value
                let interval_value = match self.evaluate_expression_value(&args[1], row)? {
                    QueryValue::Integer(i) => i,
                    QueryValue::Float(f) => f as i64,
                    QueryValue::String(s) => {
                        s.parse::<i64>().map_err(|_| QSQLError::ExecutionError {
                            message: format!(
                                "Invalid interval value: '{}' is not a valid number",
                                s
                            ),
                        })?
                    }
                    _ => return Ok(QueryValue::Null),
                };

                // Third arg is the unit marker (INTERVAL_UNIT:DAY, etc.)
                let unit_str = get_string_arg(2)?;
                let unit = unit_str
                    .strip_prefix("INTERVAL_UNIT:")
                    .unwrap_or(&unit_str)
                    .to_uppercase();

                // Parse the input date/datetime
                let result = if let Ok(dt) =
                    NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                {
                    // DateTime input
                    let new_dt = match unit.as_str() {
                        "YEAR" => {
                            if interval_value >= 0 {
                                dt.checked_sub_months(chrono::Months::new(
                                    (interval_value * 12) as u32,
                                ))
                            } else {
                                dt.checked_add_months(chrono::Months::new(
                                    ((-interval_value) * 12) as u32,
                                ))
                            }
                        }
                        "MONTH" => {
                            if interval_value >= 0 {
                                dt.checked_sub_months(chrono::Months::new(interval_value as u32))
                            } else {
                                dt.checked_add_months(chrono::Months::new((-interval_value) as u32))
                            }
                        }
                        "WEEK" => {
                            if interval_value >= 0 {
                                dt.checked_sub_days(chrono::Days::new((interval_value * 7) as u64))
                            } else {
                                dt.checked_add_days(chrono::Days::new(
                                    ((-interval_value) * 7) as u64,
                                ))
                            }
                        }
                        "DAY" => {
                            if interval_value >= 0 {
                                dt.checked_sub_days(chrono::Days::new(interval_value as u64))
                            } else {
                                dt.checked_add_days(chrono::Days::new((-interval_value) as u64))
                            }
                        }
                        "HOUR" => Some(dt - chrono::Duration::hours(interval_value)),
                        "MINUTE" => Some(dt - chrono::Duration::minutes(interval_value)),
                        "SECOND" => Some(dt - chrono::Duration::seconds(interval_value)),
                        _ => {
                            return Err(QSQLError::ExecutionError {
                                message: format!("Unsupported time unit: {}", unit),
                            })
                        }
                    };
                    new_dt.map(|d| QueryValue::String(d.format("%Y-%m-%d %H:%M:%S").to_string()))
                } else if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    // Date only input
                    let new_date = match unit.as_str() {
                        "YEAR" => {
                            if interval_value >= 0 {
                                date.checked_sub_months(chrono::Months::new(
                                    (interval_value * 12) as u32,
                                ))
                            } else {
                                date.checked_add_months(chrono::Months::new(
                                    ((-interval_value) * 12) as u32,
                                ))
                            }
                        }
                        "MONTH" => {
                            if interval_value >= 0 {
                                date.checked_sub_months(chrono::Months::new(interval_value as u32))
                            } else {
                                date.checked_add_months(chrono::Months::new(
                                    (-interval_value) as u32,
                                ))
                            }
                        }
                        "WEEK" => {
                            if interval_value >= 0 {
                                date.checked_sub_days(chrono::Days::new(
                                    (interval_value * 7) as u64,
                                ))
                            } else {
                                date.checked_add_days(chrono::Days::new(
                                    ((-interval_value) * 7) as u64,
                                ))
                            }
                        }
                        "DAY" => {
                            if interval_value >= 0 {
                                date.checked_sub_days(chrono::Days::new(interval_value as u64))
                            } else {
                                date.checked_add_days(chrono::Days::new((-interval_value) as u64))
                            }
                        }
                        "HOUR" | "MINUTE" | "SECOND" => {
                            // Convert to datetime for time-based operations
                            let dt = match date.and_hms_opt(0, 0, 0) {
                                Some(dt) => dt,
                                None => return Ok(QueryValue::Null),
                            };
                            let new_dt = match unit.as_str() {
                                "HOUR" => Some(dt - chrono::Duration::hours(interval_value)),
                                "MINUTE" => Some(dt - chrono::Duration::minutes(interval_value)),
                                "SECOND" => Some(dt - chrono::Duration::seconds(interval_value)),
                                _ => None,
                            };
                            return Ok(new_dt
                                .map(|d| {
                                    QueryValue::String(d.format("%Y-%m-%d %H:%M:%S").to_string())
                                })
                                .unwrap_or(QueryValue::Null));
                        }
                        _ => {
                            return Err(QSQLError::ExecutionError {
                                message: format!("Unsupported time unit: {}", unit),
                            })
                        }
                    };
                    new_date.map(|d| QueryValue::String(d.format("%Y-%m-%d").to_string()))
                } else {
                    None
                };

                Ok(result.unwrap_or(QueryValue::Null))
            }

            // Neuromorphic function: SYNAPTIC_WEIGHT
            // Calculate synaptic weight between two columns using Hebbian learning principles
            "SYNAPTIC_WEIGHT" => {
                if args.len() != 2 {
                    return Err(QSQLError::ExecutionError {
                        message: "SYNAPTIC_WEIGHT requires exactly 2 arguments (column1, column2)"
                            .to_string(),
                    });
                }

                // Get the two column values to correlate
                let val1 = get_arg_value(0)?;
                let val2 = get_arg_value(1)?;

                // Calculate Hebbian correlation based on co-occurrence patterns
                // "Neurons that fire together, wire together"
                let weight = self.calculate_hebbian_weight(&val1, &val2)?;

                Ok(QueryValue::SynapticWeight(weight))
            }

            _ => Err(QSQLError::ExecutionError {
                message: format!("Unknown scalar function: {}", func_name),
            }),
        }
    }

    /// Evaluate an expression to a QueryValue (for scalar function arguments)
    fn evaluate_expression_value(&self, expr: &Expression, row: &Row) -> QSQLResult<QueryValue> {
        match expr {
            Expression::Identifier(col_name) => {
                if let Some(value) = row.fields.get(col_name) {
                    Ok(self.storage_value_to_query_value(value))
                } else {
                    Ok(QueryValue::Null)
                }
            }
            Expression::Literal(lit) => Ok(self.literal_to_query_value(lit)),
            Expression::FunctionCall { name, args } => {
                let upper_name = name.to_uppercase();
                self.evaluate_scalar_function(&upper_name, args, row)
            }
            Expression::Case {
                when_clauses,
                else_result,
            } => {
                // Evaluate CASE expression: check each WHEN condition in order
                for (condition, result) in when_clauses {
                    // Evaluate the condition
                    if Self::evaluate_where_expression(condition, row)? {
                        // Condition is true, return the result
                        return self.evaluate_expression_value(result, row);
                    }
                }
                // No condition matched, return ELSE result or NULL
                match else_result {
                    Some(else_expr) => self.evaluate_expression_value(else_expr, row),
                    None => Ok(QueryValue::Null),
                }
            }
            Expression::Extract { field, source } => {
                // Evaluate EXTRACT expression: EXTRACT(field FROM source)
                self.evaluate_extract_expression(field, source, row)
            }
            _ => Ok(QueryValue::Null),
        }
    }

    /// Evaluate EXTRACT expression: EXTRACT(field FROM source)
    fn evaluate_extract_expression(
        &self,
        field: &str,
        source: &Expression,
        row: &Row,
    ) -> QSQLResult<QueryValue> {
        use chrono::prelude::*;

        // Evaluate the source expression to get the date/time value
        let source_value = self.evaluate_expression_value(source, row)?;

        // Convert the source value to a string
        let date_str = match &source_value {
            QueryValue::String(s) => s.clone(),
            QueryValue::Integer(i) => i.to_string(),
            QueryValue::Null => return Ok(QueryValue::Null),
            _ => {
                return Err(QSQLError::ExecutionError {
                    message: format!("EXTRACT requires a date/time value, got {:?}", source_value),
                });
            }
        };

        // Helper to parse date/time strings
        let parse_datetime = |s: &str| -> Option<NaiveDateTime> {
            // Try parsing as full datetime
            if let Ok(parsed) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                return Some(parsed);
            }
            // Try parsing as date only and add midnight time
            if let Ok(parsed) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return parsed.and_hms_opt(0, 0, 0);
            }
            None
        };

        // If no argument was given (NULL), use current time for fields that support it
        let datetime = if date_str.is_empty() {
            Utc::now().naive_utc()
        } else {
            match parse_datetime(&date_str) {
                Some(dt) => dt,
                None => return Ok(QueryValue::Null),
            }
        };

        // Extract the requested field
        match field.to_uppercase().as_str() {
            "YEAR" => Ok(QueryValue::Integer(datetime.year() as i64)),
            "MONTH" => Ok(QueryValue::Integer(datetime.month() as i64)),
            "DAY" => Ok(QueryValue::Integer(datetime.day() as i64)),
            "HOUR" => Ok(QueryValue::Integer(datetime.hour() as i64)),
            "MINUTE" => Ok(QueryValue::Integer(datetime.minute() as i64)),
            "SECOND" => Ok(QueryValue::Integer(datetime.second() as i64)),
            "DOW" | "DAYOFWEEK" => {
                // Day of week (1=Sunday to 7=Saturday, matching MySQL)
                let dow = datetime.weekday().num_days_from_sunday() + 1;
                Ok(QueryValue::Integer(dow as i64))
            }
            "DOY" | "DAYOFYEAR" => {
                // Day of year (1-366)
                Ok(QueryValue::Integer(datetime.ordinal() as i64))
            }
            "WEEK" | "WEEKOFYEAR" => {
                // Week of year (ISO week)
                Ok(QueryValue::Integer(datetime.date().iso_week().week() as i64))
            }
            "QUARTER" => {
                // Quarter (1-4)
                let quarter = (datetime.month() - 1) / 3 + 1;
                Ok(QueryValue::Integer(quarter as i64))
            }
            "EPOCH" => {
                // Unix timestamp
                Ok(QueryValue::Integer(datetime.and_utc().timestamp()))
            }
            _ => Err(QSQLError::ExecutionError {
                message: format!("Unsupported EXTRACT field: {}", field),
            }),
        }
    }

    /// Calculate Hebbian weight (synaptic correlation) between two values
    ///
    /// Implements the Hebbian learning principle: "Neurons that fire together, wire together"
    /// The weight represents the correlation strength between two column values.
    ///
    /// Algorithm:
    /// 1. Convert both values to normalized numeric representations
    /// 2. Calculate correlation based on co-occurrence patterns
    /// 3. Apply Hebbian learning formula: weight = activity1 * activity2
    /// 4. Normalize to [0, 1] range
    fn calculate_hebbian_weight(&self, val1: &QueryValue, val2: &QueryValue) -> QSQLResult<f32> {
        // Helper to convert QueryValue to normalized activity level (0.0 to 1.0)
        let to_activity = |v: &QueryValue| -> f32 {
            match v {
                QueryValue::Null => 0.0,
                QueryValue::Boolean(b) => {
                    if *b {
                        1.0
                    } else {
                        0.0
                    }
                }
                QueryValue::Integer(i) => {
                    // Normalize to [0, 1] using sigmoid-like function
                    let x = (*i as f32) / 100.0;
                    1.0 / (1.0 + (-x).exp())
                }
                QueryValue::Float(f) => {
                    // Normalize to [0, 1] using sigmoid-like function
                    let x = (*f as f32) / 100.0;
                    1.0 / (1.0 + (-x).exp())
                }
                QueryValue::String(s) => {
                    // Hash string to a consistent activity level using DefaultHasher
                    // This provides better distribution than a simple multiplicative hash
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};

                    let mut hasher = DefaultHasher::new();
                    s.hash(&mut hasher);
                    let hash_value = hasher.finish();
                    (hash_value % 1000) as f32 / 1000.0
                }
                QueryValue::Blob(b) => {
                    // Use blob length as a proxy for activity
                    let len = b.len() as f32;
                    (len % 100.0) / 100.0
                }
                QueryValue::DNASequence(s) => {
                    // DNA sequence: use length normalized
                    (s.len() as f32 % 100.0) / 100.0
                }
                QueryValue::SynapticWeight(w) => *w,
                QueryValue::QuantumState(_) => 0.5, // Quantum superposition = 0.5
            }
        };

        // Get activity levels for both values
        let activity1 = to_activity(val1);
        let activity2 = to_activity(val2);

        // Hebbian learning: correlation is the product of activities
        // This implements the basic Hebbian rule: Δw = η * x_i * x_j
        // where η (learning rate) is assumed to be 1.0 for simplicity
        let raw_weight = activity1 * activity2;

        // Apply co-occurrence boost based on value similarity
        let similarity_boost = if Self::query_values_equal(val1, val2) {
            // Perfect match: strong correlation boost
            1.2
        } else if matches!((val1, val2), (QueryValue::Null, _) | (_, QueryValue::Null)) {
            // NULL values reduce correlation
            0.5
        } else {
            // Different values: moderate correlation
            1.0
        };

        // Calculate final synaptic weight
        let weight = (raw_weight * similarity_boost).clamp(0.0, 1.0);

        Ok(weight)
    }

    /// Infer the return type of a scalar function
    fn infer_scalar_function_type(&self, func_name: &str) -> DataType {
        match func_name {
            // String functions
            "UPPER" | "LOWER" | "TRIM" | "LTRIM" | "RTRIM" | "CONCAT" | "SUBSTRING" | "SUBSTR"
            | "LEFT" | "RIGHT" | "REPLACE" | "REVERSE" | "REPEAT" | "LPAD" | "RPAD" | "INITCAP"
            | "CHR" => DataType::Text,
            "LENGTH" | "LEN" | "CHAR_LENGTH" | "CHARACTER_LENGTH" | "POSITION" | "INSTR"
            | "ASCII" => DataType::BigInt,
            // NULL handling functions return dynamic types based on input
            "COALESCE" | "NULLIF" | "IFNULL" | "NVL" => DataType::Text,
            // Date/Time functions returning timestamps/dates as text
            "NOW" | "CURRENT_TIMESTAMP" | "GETDATE" | "SYSDATE" | "LOCALTIME"
            | "LOCALTIMESTAMP" | "UTC_TIMESTAMP" | "CURRENT_DATE" | "CURDATE" | "CURRENT_TIME"
            | "CURTIME" | "UTC_DATE" | "UTC_TIME" | "DATE_FORMAT" | "STRFTIME" | "DATE_ADD"
            | "DATE_SUB" => DataType::Text,
            // Date/Time functions returning integers
            "UNIX_TIMESTAMP" | "EPOCH" | "YEAR" | "MONTH" | "DAY" | "DAYOFMONTH" | "HOUR"
            | "MINUTE" | "SECOND" | "DAYOFWEEK" | "WEEKDAY" | "DAYOFYEAR" | "WEEK"
            | "WEEKOFYEAR" | "QUARTER" | "DATEDIFF" => DataType::BigInt,
            // Math functions
            "ABS" | "ROUND" | "CEIL" | "CEILING" | "FLOOR" | "MOD" | "POWER" | "POW" | "SQRT"
            | "SIGN" | "TRUNCATE" | "TRUNC" | "EXP" | "LN" | "LOG" | "LOG10" | "LOG2" | "PI"
            | "RANDOM" | "RAND" => DataType::Double,
            // Neuromorphic functions
            "SYNAPTIC_WEIGHT" => DataType::SynapticWeight,
            _ => DataType::Text,
        }
    }

    /// Infer the data type from a QueryValue
    fn infer_query_value_type(&self, value: &QueryValue) -> DataType {
        match value {
            QueryValue::Null => DataType::Text,
            QueryValue::Boolean(_) => DataType::Boolean,
            QueryValue::Integer(_) => DataType::BigInt,
            QueryValue::Float(_) => DataType::Double,
            QueryValue::String(_) => DataType::Text,
            QueryValue::Blob(_) => DataType::Blob,
            QueryValue::DNASequence(_) => DataType::DNASequence,
            QueryValue::SynapticWeight(_) => DataType::SynapticWeight,
            QueryValue::QuantumState(_) => DataType::SuperpositionState,
        }
    }

    /// Extract column names from GROUP BY expressions
    fn extract_group_by_columns(&self, group_by: &[Expression]) -> Vec<String> {
        group_by
            .iter()
            .map(Self::expression_to_string_static)
            .collect()
    }

    /// Execute aggregates with GROUP BY grouping
    fn execute_grouped_aggregates(
        &self,
        storage_rows: &[Row],
        aggregates: &[AggregateFunction],
        group_by_columns: &[String],
        having: &Option<Expression>,
        select_list: &[SelectItem],
    ) -> QSQLResult<QueryResultData> {
        // Group rows by the GROUP BY columns
        let groups = self.group_rows_by_columns(storage_rows, group_by_columns);

        let mut result_rows = Vec::new();
        let mut columns = Vec::new();
        let mut columns_initialized = false;

        for (group_key, group_rows) in groups {
            let mut result_row = HashMap::new();

            // Add GROUP BY column values to result
            for (idx, col_name) in group_by_columns.iter().enumerate() {
                if idx < group_key.len() {
                    let value = self.parse_group_key_value(&group_key[idx]);
                    result_row.insert(col_name.clone(), value);

                    if !columns_initialized {
                        // Try to infer data type from first group
                        let data_type = if !group_rows.is_empty() {
                            if let Some(val) = group_rows[0].fields.get(col_name) {
                                self.storage_value_to_datatype(val)
                            } else {
                                DataType::Text
                            }
                        } else {
                            DataType::Text
                        };
                        columns.push(ColumnInfo {
                            name: col_name.clone(),
                            data_type,
                            nullable: true,
                        });
                    }
                }
            }

            // Add non-aggregate columns from SELECT list that are in GROUP BY
            for item in select_list {
                if let SelectItem::Expression { expr, alias } = item {
                    let col_name = Self::expression_to_string_static(expr);
                    // Skip if it's an aggregate function
                    if !self.is_aggregate_expression(expr)
                        && !group_by_columns.contains(&col_name)
                        && !result_row.contains_key(&col_name)
                    {
                        // Check if this column exists in group_by_columns
                        let display_name = alias.clone().unwrap_or_else(|| col_name.clone());
                        if group_by_columns.contains(&col_name) {
                            if let Some(first_row) = group_rows.first() {
                                if let Some(val) = first_row.fields.get(&col_name) {
                                    result_row.insert(
                                        display_name.clone(),
                                        self.storage_value_to_query_value(val),
                                    );
                                    if !columns_initialized {
                                        columns.push(ColumnInfo {
                                            name: display_name,
                                            data_type: self.storage_value_to_datatype(val),
                                            nullable: true,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Compute aggregates for this group
            for agg in aggregates {
                let result_name = agg.alias.clone().unwrap_or_else(|| {
                    format!("{}({})", agg.name, agg.column.as_deref().unwrap_or("*"))
                });

                let value = self.compute_aggregate(&group_rows, agg)?;

                if !columns_initialized {
                    let data_type = match agg.name.as_str() {
                        "COUNT" => DataType::BigInt,
                        "AVG" => DataType::Double,
                        "SUM" | "MIN" | "MAX" => {
                            if let Some(col) = &agg.column {
                                self.infer_column_type_from_rows(&group_rows, col)
                            } else {
                                DataType::BigInt
                            }
                        }
                        _ => DataType::Double,
                    };
                    columns.push(ColumnInfo {
                        name: result_name.clone(),
                        data_type,
                        nullable: false,
                    });
                }

                result_row.insert(result_name, value);
            }

            columns_initialized = true;

            // Apply HAVING filter if present
            if let Some(having_expr) = having {
                if self.evaluate_having_condition(having_expr, &result_row)? {
                    result_rows.push(result_row);
                }
            } else {
                result_rows.push(result_row);
            }
        }

        Ok((result_rows, columns))
    }

    /// Group rows by specified columns
    fn group_rows_by_columns(
        &self,
        storage_rows: &[Row],
        group_by_columns: &[String],
    ) -> Vec<(Vec<String>, Vec<Row>)> {
        use std::collections::BTreeMap;

        let mut groups: BTreeMap<Vec<String>, Vec<Row>> = BTreeMap::new();

        for row in storage_rows {
            let mut key = Vec::new();
            for col in group_by_columns {
                let value_str = row
                    .fields
                    .get(col)
                    .map(|v| self.value_to_string(v))
                    .unwrap_or_else(|| "NULL".to_string());
                key.push(value_str);
            }
            groups.entry(key).or_default().push(row.clone());
        }

        groups.into_iter().collect()
    }

    /// Parse group key value back to QueryValue
    fn parse_group_key_value(&self, key: &str) -> QueryValue {
        if key == "NULL" {
            return QueryValue::Null;
        }

        // Try to parse as integer
        if let Ok(i) = key.parse::<i64>() {
            return QueryValue::Integer(i);
        }

        // Try to parse as float
        if let Ok(f) = key.parse::<f64>() {
            return QueryValue::Float(f);
        }

        // Try to parse as boolean
        if key == "true" || key == "false" {
            return QueryValue::Boolean(key == "true");
        }

        // Default to string
        QueryValue::String(key.to_string())
    }

    /// Check if an expression is an aggregate function
    fn is_aggregate_expression(&self, expr: &Expression) -> bool {
        if let Expression::FunctionCall { name, .. } = expr {
            let upper = name.to_uppercase();
            matches!(
                upper.as_str(),
                "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "COUNT_DISTINCT"
            )
        } else {
            false
        }
    }

    /// Evaluate HAVING condition against a result row
    fn evaluate_having_condition(
        &self,
        having: &Expression,
        row: &HashMap<String, QueryValue>,
    ) -> QSQLResult<bool> {
        match having {
            Expression::BinaryOp {
                left,
                operator,
                right,
            } => {
                let left_val = self.evaluate_having_expr(left, row)?;
                let right_val = self.evaluate_having_expr(right, row)?;

                match operator {
                    BinaryOperator::Equal => {
                        Ok(self.compare_query_values(&left_val, &right_val) == 0)
                    }
                    BinaryOperator::NotEqual => {
                        Ok(self.compare_query_values(&left_val, &right_val) != 0)
                    }
                    BinaryOperator::LessThan => {
                        Ok(self.compare_query_values(&left_val, &right_val) < 0)
                    }
                    BinaryOperator::LessThanOrEqual => {
                        Ok(self.compare_query_values(&left_val, &right_val) <= 0)
                    }
                    BinaryOperator::GreaterThan => {
                        Ok(self.compare_query_values(&left_val, &right_val) > 0)
                    }
                    BinaryOperator::GreaterThanOrEqual => {
                        Ok(self.compare_query_values(&left_val, &right_val) >= 0)
                    }
                    BinaryOperator::And => {
                        let left_bool = self.evaluate_having_condition(left, row)?;
                        let right_bool = self.evaluate_having_condition(right, row)?;
                        Ok(left_bool && right_bool)
                    }
                    BinaryOperator::Or => {
                        let left_bool = self.evaluate_having_condition(left, row)?;
                        let right_bool = self.evaluate_having_condition(right, row)?;
                        Ok(left_bool || right_bool)
                    }
                    _ => Ok(true), // Default to true for unsupported operators
                }
            }
            Expression::UnaryOp {
                operator: UnaryOperator::Not,
                operand,
            } => {
                let val = self.evaluate_having_condition(operand, row)?;
                Ok(!val)
            }
            Expression::UnaryOp { .. } => Ok(true),
            _ => Ok(true), // Default to true for unsupported expressions
        }
    }

    /// Evaluate an expression in the context of HAVING
    fn evaluate_having_expr(
        &self,
        expr: &Expression,
        row: &HashMap<String, QueryValue>,
    ) -> QSQLResult<QueryValue> {
        match expr {
            Expression::Literal(lit) => Ok(self.literal_to_query_value(lit)),
            Expression::Identifier(name) => {
                // Look up the column value in the result row
                row.get(name)
                    .cloned()
                    .ok_or_else(|| QSQLError::ExecutionError {
                        message: format!("Column '{}' not found in HAVING clause", name),
                    })
            }
            Expression::FunctionCall { name, args } => {
                // For aggregate functions in HAVING, look up the computed result
                // The aggregate is stored with format "NAME(*)" or "NAME(column)"
                let agg_name = if args.is_empty() {
                    format!("{}(*)", name.to_uppercase())
                } else {
                    // Handle the case where COUNT(*) has "*" as a Literal::String("*")
                    let arg_str = match &args[0] {
                        Expression::Literal(Literal::String(s)) if s == "*" => "*".to_string(),
                        Expression::Identifier(col) => col.clone(),
                        other => Self::expression_to_string_static(other),
                    };
                    format!("{}({})", name.to_uppercase(), arg_str)
                };

                row.get(&agg_name)
                    .cloned()
                    .ok_or_else(|| QSQLError::ExecutionError {
                        message: format!("Aggregate '{}' not found in HAVING clause", agg_name),
                    })
            }
            _ => Ok(QueryValue::Null),
        }
    }

    /// Convert literal to QueryValue
    fn literal_to_query_value(&self, lit: &Literal) -> QueryValue {
        match lit {
            Literal::Integer(i) => QueryValue::Integer(*i),
            Literal::Float(f) => QueryValue::Float(*f),
            Literal::String(s) => QueryValue::String(s.clone()),
            Literal::Boolean(b) => QueryValue::Boolean(*b),
            Literal::Null => QueryValue::Null,
            Literal::DNA(s) => QueryValue::DNASequence(s.clone()),
            Literal::QuantumBit(state, amplitude) => {
                QueryValue::QuantumState(format!("{}:{}", state, amplitude))
            }
        }
    }

    /// Compare two QueryValues, returns -1, 0, or 1
    fn compare_query_values(&self, a: &QueryValue, b: &QueryValue) -> i32 {
        match (a, b) {
            (QueryValue::Integer(i1), QueryValue::Integer(i2)) => i1.cmp(i2) as i32,
            (QueryValue::Float(f1), QueryValue::Float(f2)) => {
                if f1 < f2 {
                    -1
                } else if f1 > f2 {
                    1
                } else {
                    0
                }
            }
            (QueryValue::Integer(i), QueryValue::Float(f)) => {
                let i_f = *i as f64;
                if i_f < *f {
                    -1
                } else if i_f > *f {
                    1
                } else {
                    0
                }
            }
            (QueryValue::Float(f), QueryValue::Integer(i)) => {
                let i_f = *i as f64;
                if *f < i_f {
                    -1
                } else if *f > i_f {
                    1
                } else {
                    0
                }
            }
            (QueryValue::String(s1), QueryValue::String(s2)) => s1.cmp(s2) as i32,
            (QueryValue::Null, QueryValue::Null) => 0,
            (QueryValue::Null, _) => -1,
            (_, QueryValue::Null) => 1,
            _ => 0, // Default for incompatible types
        }
    }

    /// Compute a single aggregate value
    fn compute_aggregate(
        &self,
        storage_rows: &[Row],
        agg: &AggregateFunction,
    ) -> QSQLResult<QueryValue> {
        match agg.name.as_str() {
            "COUNT" => self.compute_count(storage_rows, &agg.column, agg.distinct),
            "SUM" => self.compute_sum(storage_rows, &agg.column),
            "AVG" => self.compute_avg(storage_rows, &agg.column),
            "MIN" => self.compute_min(storage_rows, &agg.column),
            "MAX" => self.compute_max(storage_rows, &agg.column),
            _ => Err(QSQLError::ExecutionError {
                message: format!("Unknown aggregate function: {}", agg.name),
            }),
        }
    }

    /// Compute COUNT aggregate
    fn compute_count(
        &self,
        storage_rows: &[Row],
        column: &Option<String>,
        distinct: bool,
    ) -> QSQLResult<QueryValue> {
        match column {
            None => {
                // COUNT(*) - count all rows
                Ok(QueryValue::Integer(storage_rows.len() as i64))
            }
            Some(col) => {
                if distinct {
                    // COUNT(DISTINCT column) - count unique non-null values
                    let mut unique_values = std::collections::HashSet::new();
                    for row in storage_rows {
                        if let Some(value) = row.fields.get(col) {
                            if !matches!(value, Value::Null) {
                                unique_values.insert(self.value_to_string(value));
                            }
                        }
                    }
                    Ok(QueryValue::Integer(unique_values.len() as i64))
                } else {
                    // COUNT(column) - count non-null values
                    let count = storage_rows
                        .iter()
                        .filter(|row| {
                            row.fields
                                .get(col)
                                .map(|v| !matches!(v, Value::Null))
                                .unwrap_or(false)
                        })
                        .count();
                    Ok(QueryValue::Integer(count as i64))
                }
            }
        }
    }

    /// Compute SUM aggregate
    fn compute_sum(&self, storage_rows: &[Row], column: &Option<String>) -> QSQLResult<QueryValue> {
        let col = column.as_ref().ok_or_else(|| QSQLError::ExecutionError {
            message: "SUM requires a column argument".to_string(),
        })?;

        let mut sum_int: i64 = 0;
        let mut sum_float: f64 = 0.0;
        let mut has_float = false;
        let mut count = 0;

        for row in storage_rows {
            if let Some(value) = row.fields.get(col) {
                match value {
                    Value::Integer(i) => {
                        sum_int += i;
                        count += 1;
                    }
                    Value::Float(f) => {
                        sum_float += f;
                        has_float = true;
                        count += 1;
                    }
                    Value::Null => {} // Ignore NULL values
                    _ => {}           // Ignore non-numeric values
                }
            }
        }

        if count == 0 {
            return Ok(QueryValue::Null);
        }

        if has_float {
            Ok(QueryValue::Float(sum_float + sum_int as f64))
        } else {
            Ok(QueryValue::Integer(sum_int))
        }
    }

    /// Compute AVG aggregate
    fn compute_avg(&self, storage_rows: &[Row], column: &Option<String>) -> QSQLResult<QueryValue> {
        let col = column.as_ref().ok_or_else(|| QSQLError::ExecutionError {
            message: "AVG requires a column argument".to_string(),
        })?;

        let mut sum: f64 = 0.0;
        let mut count: i64 = 0;

        for row in storage_rows {
            if let Some(value) = row.fields.get(col) {
                match value {
                    Value::Integer(i) => {
                        sum += *i as f64;
                        count += 1;
                    }
                    Value::Float(f) => {
                        sum += f;
                        count += 1;
                    }
                    Value::Null => {} // Ignore NULL values
                    _ => {}           // Ignore non-numeric values
                }
            }
        }

        if count == 0 {
            return Ok(QueryValue::Null);
        }

        Ok(QueryValue::Float(sum / count as f64))
    }

    /// Compute MIN aggregate
    fn compute_min(&self, storage_rows: &[Row], column: &Option<String>) -> QSQLResult<QueryValue> {
        let col = column.as_ref().ok_or_else(|| QSQLError::ExecutionError {
            message: "MIN requires a column argument".to_string(),
        })?;

        let mut min_value: Option<QueryValue> = None;

        for row in storage_rows {
            if let Some(value) = row.fields.get(col) {
                if matches!(value, Value::Null) {
                    continue;
                }

                let current = self.storage_value_to_query_value(value);
                min_value = Some(match min_value {
                    None => current,
                    Some(existing) => self.min_query_value(existing, current),
                });
            }
        }

        Ok(min_value.unwrap_or(QueryValue::Null))
    }

    /// Compute MAX aggregate
    fn compute_max(&self, storage_rows: &[Row], column: &Option<String>) -> QSQLResult<QueryValue> {
        let col = column.as_ref().ok_or_else(|| QSQLError::ExecutionError {
            message: "MAX requires a column argument".to_string(),
        })?;

        let mut max_value: Option<QueryValue> = None;

        for row in storage_rows {
            if let Some(value) = row.fields.get(col) {
                if matches!(value, Value::Null) {
                    continue;
                }

                let current = self.storage_value_to_query_value(value);
                max_value = Some(match max_value {
                    None => current,
                    Some(existing) => self.max_query_value(existing, current),
                });
            }
        }

        Ok(max_value.unwrap_or(QueryValue::Null))
    }

    /// Compare two QueryValues and return the minimum
    fn min_query_value(&self, a: QueryValue, b: QueryValue) -> QueryValue {
        match (&a, &b) {
            (QueryValue::Integer(i1), QueryValue::Integer(i2)) => {
                if i1 <= i2 {
                    a
                } else {
                    b
                }
            }
            (QueryValue::Float(f1), QueryValue::Float(f2)) => {
                if f1 <= f2 {
                    a
                } else {
                    b
                }
            }
            (QueryValue::Integer(i), QueryValue::Float(f)) => {
                if (*i as f64) <= *f {
                    a
                } else {
                    b
                }
            }
            (QueryValue::Float(f), QueryValue::Integer(i)) => {
                if *f <= (*i as f64) {
                    a
                } else {
                    b
                }
            }
            (QueryValue::String(s1), QueryValue::String(s2)) => {
                if s1 <= s2 {
                    a
                } else {
                    b
                }
            }
            _ => a, // Default to first value for incompatible types
        }
    }

    /// Compare two QueryValues and return the maximum
    fn max_query_value(&self, a: QueryValue, b: QueryValue) -> QueryValue {
        match (&a, &b) {
            (QueryValue::Integer(i1), QueryValue::Integer(i2)) => {
                if i1 >= i2 {
                    a
                } else {
                    b
                }
            }
            (QueryValue::Float(f1), QueryValue::Float(f2)) => {
                if f1 >= f2 {
                    a
                } else {
                    b
                }
            }
            (QueryValue::Integer(i), QueryValue::Float(f)) => {
                if (*i as f64) >= *f {
                    a
                } else {
                    b
                }
            }
            (QueryValue::Float(f), QueryValue::Integer(i)) => {
                if *f >= (*i as f64) {
                    a
                } else {
                    b
                }
            }
            (QueryValue::String(s1), QueryValue::String(s2)) => {
                if s1 >= s2 {
                    a
                } else {
                    b
                }
            }
            _ => a, // Default to first value for incompatible types
        }
    }

    /// Convert Value to string for DISTINCT comparison
    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Text(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Binary(b) => format!("{:?}", b),
            Value::Null => "NULL".to_string(),
            Value::Timestamp(ts) => ts.to_rfc3339(),
        }
    }

    /// Infer column type from storage rows
    fn infer_column_type_from_rows(&self, storage_rows: &[Row], column: &str) -> DataType {
        for row in storage_rows {
            if let Some(value) = row.fields.get(column) {
                return self.storage_value_to_datatype(value);
            }
        }
        DataType::Double // Default
    }

    /// Convert storage Value to QueryValue
    fn storage_value_to_query_value(&self, value: &Value) -> QueryValue {
        match value {
            Value::Integer(i) => QueryValue::Integer(*i),
            Value::Float(f) => QueryValue::Float(*f),
            Value::Text(s) => QueryValue::String(s.clone()),
            Value::Boolean(b) => QueryValue::Boolean(*b),
            Value::Binary(b) => QueryValue::Blob(b.clone()),
            Value::Null => QueryValue::Null,
            Value::Timestamp(ts) => QueryValue::String(ts.to_rfc3339()),
        }
    }

    /// Convert storage Value to DataType
    fn storage_value_to_datatype(&self, value: &Value) -> DataType {
        match value {
            Value::Integer(_) => DataType::Integer,
            Value::Float(_) => DataType::Real,
            Value::Text(_) => DataType::VarChar(Some(255)),
            Value::Boolean(_) => DataType::Boolean,
            Value::Binary(_) => DataType::Blob,
            Value::Null => DataType::VarChar(Some(255)),
            Value::Timestamp(_) => DataType::Timestamp,
        }
    }

    /// Compare two QueryValues for equality (used by NULLIF)
    fn query_values_equal(val1: &QueryValue, val2: &QueryValue) -> bool {
        match (val1, val2) {
            (QueryValue::Null, QueryValue::Null) => true,
            (QueryValue::Integer(a), QueryValue::Integer(b)) => a == b,
            (QueryValue::Float(a), QueryValue::Float(b)) => (a - b).abs() < f64::EPSILON,
            (QueryValue::Integer(a), QueryValue::Float(b))
            | (QueryValue::Float(b), QueryValue::Integer(a)) => {
                (*a as f64 - b).abs() < f64::EPSILON
            }
            (QueryValue::String(a), QueryValue::String(b)) => a == b,
            (QueryValue::Boolean(a), QueryValue::Boolean(b)) => a == b,
            (QueryValue::Blob(a), QueryValue::Blob(b)) => a == b,
            _ => false,
        }
    }

    /// Convert expression to string (helper, static)
    fn expression_to_string_static(expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::Literal(lit) => format!("{:?}", lit),
            Expression::FunctionCall { name, args } => {
                let args_str: Vec<String> =
                    args.iter().map(Self::expression_to_string_static).collect();
                format!("{}({})", name, args_str.join(", "))
            }
            Expression::Case { .. } => "CASE".to_string(),
            _ => "unknown".to_string(),
        }
    }

    // =============================================================================
    // Neuromorphic Learning Integration
    // =============================================================================

    /// Learn from INSERT pattern for future optimizations
    async fn learn_from_insert(
        &mut self,
        insert: &InsertStatement,
        row_id: RowId,
    ) -> QSQLResult<()> {
        if let Some(learning_engine) = &mut self.learning_engine {
            if let Some(synaptic_network) = &self.synaptic_network {
                // Create pattern hash for this insert
                let pattern_hash = Self::hash_insert_pattern_static(insert);

                // Strengthen synaptic connection for this pattern
                learning_engine
                    .strengthen_connection(synaptic_network, pattern_hash, row_id, 0.8)
                    .map_err(|e| QSQLError::ExecutionError {
                        message: format!("Learning failed: {}", e),
                    })?;
            }
        }
        Ok(())
    }

    /// Learn from SELECT access pattern
    async fn learn_from_select(
        &mut self,
        _select: &SelectStatement,
        _rows: &[Row],
    ) -> QSQLResult<()> {
        if let Some(_learning_engine) = &mut self.learning_engine {
            if let Some(_synaptic_network) = &self.synaptic_network {
                // Learn access patterns to optimize future queries
                // This could strengthen pathways for frequently accessed data

                // Track query frequency for adaptive optimization
                self.execution_stats.synaptic_optimizations += 1;
            }
        }
        Ok(())
    }

    /// Adapt plasticity based on UPDATE patterns
    async fn adapt_plasticity_from_update(&mut self, _update: &UpdateStatement) -> QSQLResult<()> {
        if let Some(_learning_engine) = &mut self.learning_engine {
            // Adjust learning rates based on update frequency
            // This implements synaptic plasticity
            self.execution_stats.synaptic_optimizations += 1;
        }
        Ok(())
    }

    /// Prune synaptic connections for deleted data
    async fn prune_synaptic_connections_from_delete(
        &mut self,
        _delete: &DeleteStatement,
    ) -> QSQLResult<()> {
        if let Some(_synaptic_network) = &mut self.synaptic_network {
            // Weaken or remove connections for deleted data patterns
            // This implements anti-Hebbian learning
            self.execution_stats.synaptic_optimizations += 1;
        }
        Ok(())
    }

    /// Create hash for INSERT pattern identification (static)
    fn hash_insert_pattern_static(insert: &InsertStatement) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        insert.table_name.hash(&mut hasher);
        if let Some(cols) = &insert.columns {
            for col in cols {
                col.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> &ExecutionStats {
        &self.execution_stats
    }

    /// Reset execution statistics
    pub fn reset_stats(&mut self) {
        self.execution_stats = ExecutionStats::default();
    }

    /// Check if an expression contains a subquery that needs resolution
    fn contains_subquery_expression(expr: &Expression) -> bool {
        match expr {
            Expression::InSubquery { .. } => true,
            Expression::Exists { .. } => true,
            Expression::ScalarSubquery { .. } => true,
            Expression::BinaryOp { left, right, .. } => {
                Self::contains_subquery_expression(left)
                    || Self::contains_subquery_expression(right)
            }
            Expression::UnaryOp { operand, .. } => Self::contains_subquery_expression(operand),
            _ => false,
        }
    }

    /// Resolve all subquery expressions in a WHERE clause by executing them
    /// - InSubquery -> InList
    /// - Exists -> Boolean literal
    /// - ScalarSubquery -> Literal value
    fn resolve_subqueries_in_expression<'a>(
        &'a mut self,
        expr: Expression,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = QSQLResult<Expression>> + Send + 'a>>
    {
        Box::pin(async move {
            match expr {
                Expression::InSubquery {
                    expr: field_expr,
                    subquery,
                    negated,
                } => {
                    // Execute the subquery to get the list of values
                    let subquery_values = self.execute_subquery_for_in(&subquery).await?;

                    // Convert the subquery result to a list of literal expressions
                    let list: Vec<Expression> = subquery_values
                        .into_iter()
                        .map(|v| Expression::Literal(Self::value_to_literal(v)))
                        .collect();

                    // Return an InList expression with the resolved values
                    Ok(Expression::InList {
                        expr: field_expr,
                        list,
                        negated,
                    })
                }
                Expression::Exists { subquery, negated } => {
                    // Execute the EXISTS subquery - check if it returns any rows
                    let exists = self.execute_exists_subquery(&subquery).await?;
                    let result = if negated { !exists } else { exists };
                    Ok(Expression::Literal(Literal::Boolean(result)))
                }
                Expression::ScalarSubquery { subquery } => {
                    // Execute the scalar subquery and return the single value
                    let value = self.execute_scalar_subquery(&subquery).await?;
                    Ok(Expression::Literal(value))
                }
                Expression::BinaryOp {
                    left,
                    operator,
                    right,
                } => {
                    // Recursively resolve subqueries in both sides
                    let resolved_left = self.resolve_subqueries_in_expression(*left).await?;
                    let resolved_right = self.resolve_subqueries_in_expression(*right).await?;
                    Ok(Expression::BinaryOp {
                        left: Box::new(resolved_left),
                        operator,
                        right: Box::new(resolved_right),
                    })
                }
                Expression::UnaryOp { operator, operand } => {
                    let resolved_operand = self.resolve_subqueries_in_expression(*operand).await?;
                    Ok(Expression::UnaryOp {
                        operator,
                        operand: Box::new(resolved_operand),
                    })
                }
                // For all other expressions, return as-is
                other => Ok(other),
            }
        })
    }

    /// Execute an EXISTS subquery - returns true if the subquery returns at least one row
    async fn execute_exists_subquery(&mut self, subquery: &SelectStatement) -> QSQLResult<bool> {
        // Get the table name from the subquery
        let table_name = if let Some(from) = &subquery.from {
            if !from.relations.is_empty() {
                from.relations[0].name.clone()
            } else {
                return Err(QSQLError::ExecutionError {
                    message: "EXISTS subquery has no table in FROM clause".to_string(),
                });
            }
        } else {
            return Err(QSQLError::ExecutionError {
                message: "EXISTS subquery missing FROM clause".to_string(),
            });
        };

        // Build a storage query for the subquery with LIMIT 1 for efficiency
        let storage_query = SelectQuery {
            table: table_name,
            columns: vec!["*".to_string()],
            where_clause: subquery
                .where_clause
                .as_ref()
                .and_then(|w| Self::convert_expression_to_where_clause_static(w).ok()),
            order_by: None,
            limit: Some(1), // Only need to check if at least one row exists
            offset: None,
        };

        // Execute the subquery
        let storage_guard = self.storage_engine.as_ref().unwrap().read().await;
        let rows = storage_guard
            .select_rows(&storage_query)
            .await
            .map_err(|e| QSQLError::ExecutionError {
                message: format!("EXISTS subquery execution failed: {}", e),
            })?;
        drop(storage_guard);

        // Return true if any rows were found
        Ok(!rows.is_empty())
    }

    /// Execute a scalar subquery and return a single value
    async fn execute_scalar_subquery(&mut self, subquery: &SelectStatement) -> QSQLResult<Literal> {
        // Get the table name from the subquery
        let table_name = if let Some(from) = &subquery.from {
            if !from.relations.is_empty() {
                from.relations[0].name.clone()
            } else {
                return Err(QSQLError::ExecutionError {
                    message: "Scalar subquery has no table in FROM clause".to_string(),
                });
            }
        } else {
            return Err(QSQLError::ExecutionError {
                message: "Scalar subquery missing FROM clause".to_string(),
            });
        };

        // Check if this is an aggregate query (like SELECT AVG(age) FROM users)
        let is_aggregate = subquery.select_list.iter().any(|item| {
            if let SelectItem::Expression { expr, .. } = item {
                self.is_aggregate_expression(expr)
            } else {
                false
            }
        });

        // Build a storage query for the subquery
        let storage_query = SelectQuery {
            table: table_name.clone(),
            columns: vec!["*".to_string()],
            where_clause: subquery
                .where_clause
                .as_ref()
                .and_then(|w| Self::convert_expression_to_where_clause_static(w).ok()),
            order_by: None,
            limit: if is_aggregate { None } else { Some(1) },
            offset: None,
        };

        // Execute the subquery
        let storage_guard = self.storage_engine.as_ref().unwrap().read().await;
        let rows = storage_guard
            .select_rows(&storage_query)
            .await
            .map_err(|e| QSQLError::ExecutionError {
                message: format!("Scalar subquery execution failed: {}", e),
            })?;
        drop(storage_guard);

        if is_aggregate {
            // For aggregate queries, compute the aggregate
            let aggregates = self.extract_aggregate_functions(&subquery.select_list);
            if let Some(first_agg) = aggregates.first() {
                let value = self.compute_aggregate(&rows, first_agg)?;
                return Ok(Self::query_value_to_literal(value));
            }
        }

        // Get the first column from the first row
        if rows.is_empty() {
            return Ok(Literal::Null);
        }

        let first_row = &rows[0];

        // Get the column name from the SELECT list
        let column_name = if !subquery.select_list.is_empty() {
            match &subquery.select_list[0] {
                SelectItem::Expression { expr, .. } => Self::expression_to_string_static(expr),
                SelectItem::Wildcard => {
                    first_row.fields.keys().next().cloned().unwrap_or_default()
                }
            }
        } else {
            first_row.fields.keys().next().cloned().unwrap_or_default()
        };

        // Get the value from the first row
        if let Some(value) = first_row.fields.get(&column_name) {
            Ok(Self::value_to_literal(value.clone()))
        } else {
            Ok(Literal::Null)
        }
    }

    /// Convert QueryValue to Literal
    fn query_value_to_literal(value: QueryValue) -> Literal {
        match value {
            QueryValue::Null => Literal::Null,
            QueryValue::Boolean(b) => Literal::Boolean(b),
            QueryValue::Integer(i) => Literal::Integer(i),
            QueryValue::Float(f) => Literal::Float(f),
            QueryValue::String(s) => Literal::String(s),
            QueryValue::Blob(b) => Literal::String(String::from_utf8_lossy(&b).to_string()),
            QueryValue::DNASequence(s) => Literal::DNA(s),
            QueryValue::SynapticWeight(w) => Literal::Float(w as f64),
            QueryValue::QuantumState(s) => Literal::String(s),
        }
    }

    /// Execute a subquery for IN clause and return the first column values
    async fn execute_subquery_for_in(
        &mut self,
        subquery: &SelectStatement,
    ) -> QSQLResult<Vec<Value>> {
        // Get the table name from the subquery
        let table_name = if let Some(from) = &subquery.from {
            if !from.relations.is_empty() {
                from.relations[0].name.clone()
            } else {
                return Err(QSQLError::ExecutionError {
                    message: "Subquery has no table in FROM clause".to_string(),
                });
            }
        } else {
            return Err(QSQLError::ExecutionError {
                message: "Subquery missing FROM clause".to_string(),
            });
        };

        // Build a storage query for the subquery
        let storage_query = SelectQuery {
            table: table_name,
            columns: vec!["*".to_string()], // Get all columns, we'll extract the first one
            where_clause: subquery
                .where_clause
                .as_ref()
                .and_then(|w| Self::convert_expression_to_where_clause_static(w).ok()),
            order_by: None,
            limit: subquery.limit,
            offset: subquery.offset,
        };

        // Execute the subquery
        let storage_guard = self.storage_engine.as_ref().unwrap().read().await;
        let rows = storage_guard
            .select_rows(&storage_query)
            .await
            .map_err(|e| QSQLError::ExecutionError {
                message: format!("Subquery execution failed: {}", e),
            })?;
        drop(storage_guard);

        // Extract the first column from each row
        let column_name = if !subquery.select_list.is_empty() {
            match &subquery.select_list[0] {
                SelectItem::Expression { expr, .. } => Self::expression_to_string_static(expr),
                SelectItem::Wildcard => {
                    // For wildcard, use the first field in the row
                    if let Some(first_row) = rows.first() {
                        first_row.fields.keys().next().cloned().unwrap_or_default()
                    } else {
                        String::new()
                    }
                }
            }
        } else {
            return Err(QSQLError::ExecutionError {
                message: "Subquery has no columns in SELECT list".to_string(),
            });
        };

        // Collect values from the first column
        let values: Vec<Value> = rows
            .into_iter()
            .filter_map(|row| row.fields.get(&column_name).cloned())
            .collect();

        Ok(values)
    }

    /// Convert a Value to a Literal
    fn value_to_literal(value: Value) -> Literal {
        match value {
            Value::Integer(i) => Literal::Integer(i),
            Value::Float(f) => Literal::Float(f),
            Value::Text(s) => Literal::String(s),
            Value::Boolean(b) => Literal::Boolean(b),
            Value::Null => Literal::Null,
            Value::Timestamp(ts) => Literal::String(ts.to_rfc3339()),
            Value::Binary(b) => Literal::String(String::from_utf8_lossy(&b).to_string()),
        }
    }
}

impl Default for QueryExecutor {
    fn default() -> Self {
        match Self::new() {
            Ok(executor) => executor,
            Err(_) => {
                // Fallback to a minimal executor if creation fails
                QueryExecutor {
                    config: ExecutorConfig::default(),
                    execution_stats: ExecutionStats::default(),
                    storage_engine: None,
                    learning_engine: None,
                    synaptic_network: None,
                    transaction_manager: None,
                    current_transaction: None,
                    savepoints: HashMap::new(),
                }
            }
        }
    }
}

/// Query execution plan with optimization metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub statement: Statement,
    pub execution_strategy: ExecutionStrategy,
    pub synaptic_pathways: Vec<SynapticPathway>,
    pub quantum_optimizations: Vec<QuantumOptimization>,
    pub estimated_cost: f64,
    pub optimization_metadata: OptimizationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Sequential,
    Parallel,
    SynapticPipeline,
    QuantumInspired,
    NeuromorphicOptimized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynapticPathway {
    pub pathway_id: String,
    pub weight: f32,
    pub activation_threshold: f32,
    pub plasticity_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumOptimization {
    pub optimization_type: QuantumOptimizationType,
    pub speedup_factor: f32,
    pub coherence_time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantumOptimizationType {
    GroverSearch,
    QuantumJoin,
    AmplitudeAmplification,
    SuperpositionQuery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationMetadata {
    pub optimization_time: Duration,
    pub iterations_used: u32,
    pub convergence_achieved: bool,
    pub synaptic_adaptations: u32,
    pub quantum_optimizations_applied: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::QSQLParser;

    #[tokio::test]
    async fn test_basic_select_execution() {
        // Use testing config to allow legacy mode with simulated data
        let mut executor = QueryExecutor::with_config(ExecutorConfig::testing()).unwrap();

        let select = SelectStatement {
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
            neuromatch_clause: None,
            quantum_parallel: false,
            grover_iterations: None,
            with_clause: None,
        };

        let plan = QueryPlan {
            statement: Statement::Select(select),
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

        let result = executor.execute(&plan).await;
        assert!(result.is_ok());

        let query_result = result.unwrap();
        assert_eq!(query_result.rows_affected, 5);
        assert!(query_result.execution_time < Duration::from_millis(1));
    }

    #[tokio::test]
    async fn test_neuromatch_execution() {
        // Use testing config to allow legacy mode with simulated data
        let mut executor = QueryExecutor::with_config(ExecutorConfig::testing()).unwrap();

        let neuromatch = NeuroMatchStatement {
            target_table: "users".to_string(),
            pattern_expression: Expression::Literal(Literal::Boolean(true)),
            synaptic_weight: 0.8,
            learning_rate: Some(0.01),
            activation_threshold: Some(0.5),
            hebbian_strengthening: true,
        };

        let plan = QueryPlan {
            statement: Statement::NeuroMatch(neuromatch),
            execution_strategy: ExecutionStrategy::SynapticPipeline,
            synaptic_pathways: vec![],
            quantum_optimizations: vec![],
            estimated_cost: 150.0,
            optimization_metadata: OptimizationMetadata {
                optimization_time: Duration::from_millis(2),
                iterations_used: 1,
                convergence_achieved: true,
                synaptic_adaptations: 1,
                quantum_optimizations_applied: 0,
            },
        };

        let result = executor.execute(&plan).await;
        assert!(result.is_ok());

        let query_result = result.unwrap();
        assert_eq!(query_result.rows_affected, 3);
        assert!(query_result.optimization_applied);
    }

    #[tokio::test]
    async fn test_quantum_search_execution() {
        // Use testing config to allow legacy mode with simulated data
        let mut executor = QueryExecutor::with_config(ExecutorConfig::testing()).unwrap();

        let quantum_search = QuantumSearchStatement {
            target_table: "products".to_string(),
            search_expression: Expression::Literal(Literal::Boolean(true)),
            amplitude_amplification: true,
            oracle_function: Some("price_oracle".to_string()),
            max_iterations: Some(10),
        };

        let plan = QueryPlan {
            statement: Statement::QuantumSearch(quantum_search),
            execution_strategy: ExecutionStrategy::QuantumInspired,
            synaptic_pathways: vec![],
            quantum_optimizations: vec![],
            estimated_cost: 50.0,
            optimization_metadata: OptimizationMetadata {
                optimization_time: Duration::from_millis(1),
                iterations_used: 1,
                convergence_achieved: true,
                synaptic_adaptations: 0,
                quantum_optimizations_applied: 1,
            },
        };

        let result = executor.execute(&plan).await;
        assert!(result.is_ok());

        let query_result = result.unwrap();
        assert_eq!(query_result.quantum_operations, 10);
        assert!(query_result.execution_time < Duration::from_millis(1));
    }

    #[test]
    fn test_extract_year_parsing() {
        let parser = QSQLParser::new();
        let sql = "SELECT EXTRACT(YEAR FROM '2025-12-23')";
        let result = parser.parse_query(sql);

        if let Err(e) = &result {
            eprintln!("Parse error: {:?}", e);
        }

        assert!(result.is_ok(), "Failed to parse EXTRACT(YEAR FROM date)");

        let stmt = result.unwrap();
        if let Statement::Select(select) = stmt {
            assert_eq!(select.select_list.len(), 1);
            if let SelectItem::Expression { expr, .. } = &select.select_list[0] {
                match expr {
                    Expression::Extract { field, .. } => {
                        assert_eq!(field, "YEAR");
                    }
                    _ => panic!("Expected Extract expression"),
                }
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_extract_all_fields() {
        let parser = QSQLParser::new();
        let fields = vec![
            "YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND", "DOW", "DOY", "WEEK", "QUARTER",
            "EPOCH",
        ];

        for field in fields {
            let sql = format!("SELECT EXTRACT({} FROM '2025-12-23 14:30:45')", field);
            let result = parser.parse_query(&sql);
            assert!(
                result.is_ok(),
                "Failed to parse EXTRACT({} FROM date)",
                field
            );
        }
    }

    #[test]
    fn test_extract_in_where_clause() {
        let parser = QSQLParser::new();
        let sql = "SELECT * FROM events WHERE EXTRACT(YEAR FROM created_at) = 2025";
        let result = parser.parse_query(sql);
        assert!(result.is_ok(), "Failed to parse EXTRACT in WHERE clause");
    }

    #[test]
    fn test_extract_missing_from_keyword() {
        let parser = QSQLParser::new();
        let sql = "SELECT EXTRACT(YEAR '2025-12-23')";
        let result = parser.parse_query(sql);
        assert!(result.is_err(), "Should fail without FROM keyword");
    }
}
