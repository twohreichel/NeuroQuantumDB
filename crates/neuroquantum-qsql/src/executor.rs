//! Query Executor for QSQL
//!
//! This module provides the execution engine that runs optimized query plans
//! with neuromorphic and quantum-inspired processing capabilities.

// Re-export main executor components
pub use crate::query_plan::{QueryExecutor, QueryResult, QueryValue, ExecutorConfig, ExecutionStats, ColumnInfo};
