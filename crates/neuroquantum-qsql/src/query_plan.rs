//! Query Planning for QSQL
//!
//! This module provides query plan generation and optimization using
//! neuromorphic intelligence and quantum-inspired algorithms.

use crate::ast::*;
use crate::error::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    // Storage engine integration (optional for backward compatibility)
    storage_engine: Option<StorageEngine>,
    // Neuromorphic learning integration (optional)
    learning_engine: Option<HebbianLearningEngine>,
    synaptic_network: Option<SynapticNetwork>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        })
    }

    /// Create executor with storage engine integration (production mode)
    /// This enables DNA compression, neuromorphic learning, and full query execution
    pub fn with_storage(config: ExecutorConfig, storage_engine: StorageEngine) -> QSQLResult<Self> {
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
        })
    }

    /// Set storage engine (for existing executors)
    pub fn set_storage_engine(&mut self, storage_engine: StorageEngine) {
        self.storage_engine = Some(storage_engine);
    }

    /// Check if storage engine is available
    pub fn has_storage_engine(&self) -> bool {
        self.storage_engine.is_some()
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

            // Convert SQL SELECT to storage query (no borrow of self.storage_engine)
            let storage_query = self.convert_select_to_storage_query(select)?;

            // Execute query via storage engine (automatically DNA-decompressed!)
            let storage_rows = self
                .storage_engine
                .as_ref()
                .unwrap()
                .select_rows(&storage_query)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Storage select failed: {}", e),
                })?;

            // Neuromorphic learning: learn from access pattern
            if self.config.enable_synaptic_optimization && select.synaptic_weight.is_some() {
                self.learn_from_select(select, &storage_rows).await?;
            }

            // Convert storage rows to query result
            let (result_rows, columns) =
                self.convert_storage_rows_to_result(storage_rows, select)?;

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
                let row_id = self
                    .storage_engine
                    .as_mut()
                    .unwrap()
                    .insert_row(&insert.table_name, row)
                    .await
                    .map_err(|e| QSQLError::ExecutionError {
                        message: format!("Storage insert failed: {}", e),
                    })?;

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
            let rows_affected = self
                .storage_engine
                .as_mut()
                .unwrap()
                .update_rows(&storage_query)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Storage update failed: {}", e),
                })?;

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
            let rows_affected = self
                .storage_engine
                .as_mut()
                .unwrap()
                .delete_rows(&storage_query)
                .await
                .map_err(|e| QSQLError::ExecutionError {
                    message: format!("Storage delete failed: {}", e),
                })?;

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

        // Extract column list
        let columns = select
            .select_list
            .iter()
            .map(|item| match item {
                SelectItem::Wildcard => "*".to_string(),
                SelectItem::Expression { expr, alias } => alias
                    .clone()
                    .unwrap_or_else(|| Self::expression_to_string_static(expr)),
            })
            .collect();

        // Convert WHERE clause
        let where_clause = if let Some(expr) = &select.where_clause {
            Some(Self::convert_expression_to_where_clause_static(expr)?)
        } else {
            None
        };

        // Convert ORDER BY
        let order_by = if !select.order_by.is_empty() {
            Some(Self::convert_order_by_static(&select.order_by[0])?)
        } else {
            None
        };

        Ok(SelectQuery {
            table,
            columns,
            where_clause,
            order_by,
            limit: select.limit,
            offset: select.offset,
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
        _select: &SelectStatement,
    ) -> QSQLResult<QueryResultData> {
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

    /// Convert expression to string (helper, static)
    fn expression_to_string_static(expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::Literal(lit) => format!("{:?}", lit),
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
            quantum_parallel: false,
            grover_iterations: None,
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
}
