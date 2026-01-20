//! Online Schema Migration Framework
//!
//! This module provides non-blocking schema migration capabilities for
//! production environments, allowing schema changes without downtime.

mod executor;
mod history;
mod parser;
mod progress;

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
pub use executor::{MigrationExecutor, MigrationExecutorConfig};
pub use history::{MigrationHistory, MigrationRecord, MigrationStatus};
pub use parser::{Migration, MigrationDirection, MigrationParser};
pub use progress::{MigrationProgress, ProgressTracker};
use serde::{Deserialize, Serialize};

/// Result of executing a SQL statement
#[derive(Debug, Clone)]
pub struct SqlExecutionResult {
    /// Number of rows affected by the statement
    pub rows_affected: u64,
    /// Optional message from the execution
    pub message: Option<String>,
}

/// Trait for executing SQL statements during migrations.
///
/// This trait allows the migration executor to execute SQL statements
/// against the database without depending on the query engine crate directly.
/// Implementations of this trait can be injected from higher-level crates
/// like `neuroquantum-api` that have access to the full query engine.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use neuroquantum_core::storage::migration::{SqlExecutor, SqlExecutionResult};
///
/// struct MyQSQLExecutor;
///
/// #[async_trait::async_trait]
/// impl SqlExecutor for MyQSQLExecutor {
///     async fn execute_sql(&self, sql: &str) -> anyhow::Result<SqlExecutionResult> {
///         // Execute the SQL statement
///         Ok(SqlExecutionResult {
///             rows_affected: 0,
///             message: None,
///         })
///     }
/// }
/// ```
#[async_trait]
pub trait SqlExecutor: Send + Sync {
    /// Execute a SQL statement and return the result.
    ///
    /// # Arguments
    ///
    /// * `sql` - The SQL statement to execute
    ///
    /// # Returns
    ///
    /// * `Ok(SqlExecutionResult)` - The execution result with rows affected
    /// * `Err(anyhow::Error)` - If the execution fails
    async fn execute_sql(&self, sql: &str) -> anyhow::Result<SqlExecutionResult>;
}

/// Type alias for a boxed SQL executor
pub type BoxedSqlExecutor = Arc<dyn SqlExecutor>;

/// Migration identifier
pub type MigrationId = String;

/// Configuration for migration framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Directory containing migration files
    pub migrations_dir: PathBuf,
    /// Enable dry-run mode (don't apply changes)
    pub dry_run: bool,
    /// Lock timeout in seconds
    pub lock_timeout_secs: u64,
    /// Enable automatic rollback on failure
    pub auto_rollback: bool,
    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            migrations_dir: PathBuf::from("migrations"),
            dry_run: false,
            lock_timeout_secs: 300,
            auto_rollback: true,
            max_concurrent_ops: 4,
        }
    }
}

/// Migration operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub migration_id: MigrationId,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub rows_affected: Option<u64>,
}

/// Safety checks before migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheck {
    pub check_name: String,
    pub passed: bool,
    pub message: String,
}

/// Pre-migration validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub checks: Vec<SafetyCheck>,
    pub estimated_disk_space_mb: Option<u64>,
    pub estimated_duration_secs: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_config_default() {
        let config = MigrationConfig::default();
        assert_eq!(config.migrations_dir, PathBuf::from("migrations"));
        assert!(!config.dry_run);
        assert_eq!(config.lock_timeout_secs, 300);
        assert!(config.auto_rollback);
        assert_eq!(config.max_concurrent_ops, 4);
    }
}
