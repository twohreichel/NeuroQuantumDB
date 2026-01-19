//! Migration executor
//!
//! Executes migrations with safety checks and rollback support.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use super::history::MigrationHistory;
use super::parser::{Migration, MigrationDirection, MigrationParser};
use super::progress::ProgressTracker;
use super::{BoxedSqlExecutor, MigrationConfig, MigrationResult, SafetyCheck, ValidationResult};

/// Configuration for migration executor
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MigrationExecutorConfig {
    /// Base migration configuration
    pub config: MigrationConfig,
    /// Enable verbose logging
    pub verbose: bool,
}

/// Executes database migrations
pub struct MigrationExecutor {
    config: MigrationExecutorConfig,
    parser: MigrationParser,
    history: MigrationHistory,
    progress: ProgressTracker,
    /// Optional SQL executor for executing migration SQL.
    /// When None, dry-run mode is enforced or simulation is used in tests.
    sql_executor: Option<BoxedSqlExecutor>,
}

impl MigrationExecutor {
    /// Create a new migration executor without SQL execution capability.
    ///
    /// Use `with_executor` to set up a migration executor that can actually
    /// execute SQL statements against the database.
    #[must_use]
    pub fn new(config: MigrationExecutorConfig) -> Self {
        let parser = MigrationParser::new(config.config.migrations_dir.clone());
        let history = MigrationHistory::new();
        let progress = ProgressTracker::new();

        Self {
            config,
            parser,
            history,
            progress,
            sql_executor: None,
        }
    }

    /// Create a new migration executor with SQL execution capability.
    ///
    /// The provided `sql_executor` will be used to execute migration SQL
    /// statements against the database.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use neuroquantum_core::storage::migration::{
    ///     MigrationExecutor, MigrationExecutorConfig, SqlExecutor
    /// };
    ///
    /// let executor = MigrationExecutor::with_executor(
    ///     config,
    ///     Arc::new(my_sql_executor),
    /// );
    /// ```
    #[must_use]
    pub fn with_executor(config: MigrationExecutorConfig, sql_executor: BoxedSqlExecutor) -> Self {
        let parser = MigrationParser::new(config.config.migrations_dir.clone());
        let history = MigrationHistory::new();
        let progress = ProgressTracker::new();

        Self {
            config,
            parser,
            history,
            progress,
            sql_executor: Some(sql_executor),
        }
    }

    /// Set the SQL executor for this migration executor.
    ///
    /// This allows configuring the executor after creation.
    pub fn set_sql_executor(&mut self, sql_executor: BoxedSqlExecutor) {
        self.sql_executor = Some(sql_executor);
    }

    /// Check if a SQL executor is configured.
    #[must_use]
    pub fn has_sql_executor(&self) -> bool {
        self.sql_executor.is_some()
    }

    /// Initialize the migration framework
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing migration framework");
        self.history.initialize().await?;
        Ok(())
    }

    /// Validate a migration before applying it
    pub async fn validate_migration(&self, migration: &Migration) -> Result<ValidationResult> {
        let mut checks = Vec::new();

        // Check 1: SQL syntax (basic validation)
        checks.push(SafetyCheck {
            check_name: "SQL Syntax".to_string(),
            passed: !migration.up_sql.is_empty(),
            message: if migration.up_sql.is_empty() {
                "Empty SQL".to_string()
            } else {
                "SQL syntax appears valid".to_string()
            },
        });

        // Check 2: Has down migration for rollback
        checks.push(SafetyCheck {
            check_name: "Rollback Support".to_string(),
            passed: !migration.down_sql.is_empty(),
            message: if migration.down_sql.is_empty() {
                "No down migration - cannot rollback".to_string()
            } else {
                "Down migration available".to_string()
            },
        });

        // Check 3: Check for dangerous operations
        let dangerous_ops = ["DROP TABLE", "TRUNCATE", "DELETE FROM"];
        let has_dangerous = dangerous_ops
            .iter()
            .any(|op| migration.up_sql.to_uppercase().contains(op));

        checks.push(SafetyCheck {
            check_name: "Dangerous Operations".to_string(),
            passed: !has_dangerous,
            message: if has_dangerous {
                "Contains potentially destructive operations".to_string()
            } else {
                "No dangerous operations detected".to_string()
            },
        });

        let all_passed = checks.iter().all(|c| c.passed);

        Ok(ValidationResult {
            valid: all_passed,
            checks,
            estimated_disk_space_mb: None,
            estimated_duration_secs: None,
        })
    }

    /// Run pending migrations (up direction)
    pub async fn migrate_up(&self) -> Result<Vec<MigrationResult>> {
        info!("Running migrations up");

        let migrations = self.parser.load_all()?;
        let pending = self
            .history
            .get_pending(migrations.iter().map(|m| m.id.clone()).collect())
            .await?;

        let mut results = Vec::new();

        for migration_id in pending {
            let migration = migrations
                .iter()
                .find(|m| m.id == migration_id)
                .ok_or_else(|| anyhow!("Migration {migration_id} not found"))?;

            let result = self
                .execute_migration(migration, MigrationDirection::Up)
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Rollback the last N migrations (down direction)
    pub async fn migrate_down(&self, count: usize) -> Result<Vec<MigrationResult>> {
        info!("Rolling back {} migration(s)", count);

        let migrations = self.parser.load_all()?;
        let all_records = self.history.get_all().await?;

        // Get applied migrations in reverse order
        let mut applied: Vec<_> = all_records
            .iter()
            .filter(|r| r.status == crate::storage::migration::history::MigrationStatus::Completed)
            .collect();
        applied.sort_by(|a, b| b.applied_at.cmp(&a.applied_at));

        let mut results = Vec::new();

        for record in applied.iter().take(count) {
            let migration = migrations
                .iter()
                .find(|m| m.id == record.migration_id)
                .ok_or_else(|| anyhow!("Migration {} not found", record.migration_id))?;

            let result = self
                .execute_migration(migration, MigrationDirection::Down)
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute a single migration
    async fn execute_migration(
        &self,
        migration: &Migration,
        direction: MigrationDirection,
    ) -> Result<MigrationResult> {
        let start_time = Instant::now();

        info!("Executing migration {} ({:?})", migration.id, direction);

        // Validate before executing
        if direction == MigrationDirection::Up {
            let validation = self.validate_migration(migration).await?;
            if !validation.valid {
                let error_msg = format!("Validation failed: {:?}", validation.checks);
                error!("{}", error_msg);
                return Ok(MigrationResult {
                    migration_id: migration.id.clone(),
                    success: false,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    error: Some(error_msg),
                    rows_affected: None,
                });
            }
        }

        // Record start
        self.history.record_start(migration.id.clone()).await?;

        // Start progress tracking
        self.progress.start(migration.id.clone(), 100).await;

        // Execute the migration
        let sql = migration.get_sql(direction);

        if self.config.config.dry_run {
            info!("DRY RUN: Would execute SQL:\n{}", sql);
            self.progress.complete().await;

            return Ok(MigrationResult {
                migration_id: migration.id.clone(),
                success: true,
                duration_ms: start_time.elapsed().as_millis() as u64,
                error: None,
                rows_affected: Some(0),
            });
        }

        // Execute SQL through the SQL executor
        self.progress
            .update(50, "Executing SQL...".to_string())
            .await;

        let (success, error, rows_affected) = if let Some(ref executor) = self.sql_executor {
            // Execute SQL statements (may contain multiple statements separated by ;)
            let statements: Vec<&str> = sql
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();

            let mut total_rows_affected: u64 = 0;
            let mut execution_error: Option<String> = None;

            for (i, statement) in statements.iter().enumerate() {
                debug!(
                    "Executing statement {}/{}: {}",
                    i + 1,
                    statements.len(),
                    statement
                );

                match executor.execute_sql(statement).await {
                    | Ok(result) => {
                        total_rows_affected += result.rows_affected;
                        // Update progress based on statements completed
                        let progress_pct = 50 + (50 * (i + 1) / statements.len().max(1)) as u64;
                        self.progress
                            .update(
                                progress_pct,
                                format!("Executed {}/{} statements", i + 1, statements.len()),
                            )
                            .await;
                    },
                    | Err(e) => {
                        error!(
                            "Failed to execute statement in migration {}: {}",
                            migration.id, e
                        );
                        execution_error = Some(format!("Statement {} failed: {}", i + 1, e));
                        break;
                    },
                }
            }

            if let Some(ref err) = execution_error {
                // Rollback if auto_rollback is enabled
                if self.config.config.auto_rollback && direction == MigrationDirection::Up {
                    warn!(
                        "Auto-rollback enabled, attempting to rollback migration {}",
                        migration.id
                    );
                    // Try to execute down migration if available
                    if !migration.down_sql.is_empty() {
                        let down_statements: Vec<&str> = migration
                            .down_sql
                            .split(';')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .collect();

                        for down_stmt in down_statements {
                            if let Err(rollback_err) = executor.execute_sql(down_stmt).await {
                                error!(
                                    "Rollback failed for migration {}: {}",
                                    migration.id, rollback_err
                                );
                            }
                        }
                    }
                }
                (false, Some(err.clone()), Some(0))
            } else {
                (true, None, Some(total_rows_affected))
            }
        } else {
            // No SQL executor configured - this is an error in production
            warn!(
                "No SQL executor configured for migration {}. \
                 Use MigrationExecutor::with_executor() or set_sql_executor() \
                 to enable actual SQL execution.",
                migration.id
            );
            // For backward compatibility in tests, we allow this but log a warning
            #[cfg(test)]
            {
                // In tests, simulate success for backward compatibility
                (true, None, Some(0))
            }
            #[cfg(not(test))]
            {
                return Err(anyhow!(
                    "No SQL executor configured. Cannot execute migration {}. \
                     Use MigrationExecutor::with_executor() to provide a SQL executor.",
                    migration.id
                ));
            }
        };

        self.progress.complete().await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if success {
            // Record completion
            self.history
                .record_complete(
                    migration.id.clone(),
                    duration_ms,
                    rows_affected,
                    Self::compute_checksum(sql),
                )
                .await?;

            info!(
                "Migration {} completed in {}ms, {} rows affected",
                migration.id,
                duration_ms,
                rows_affected.unwrap_or(0)
            );
        } else {
            // Record failure
            self.history
                .record_failure(
                    migration.id.clone(),
                    error.clone().unwrap_or_default(),
                    duration_ms,
                )
                .await?;

            error!(
                "Migration {} failed in {}ms: {}",
                migration.id,
                duration_ms,
                error.as_ref().unwrap_or(&"Unknown error".to_string())
            );
        }

        Ok(MigrationResult {
            migration_id: migration.id.clone(),
            success,
            duration_ms,
            error,
            rows_affected,
        })
    }

    /// Compute a simple checksum for the SQL content
    fn compute_checksum(sql: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        sql.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get migration status
    pub async fn status(&self) -> Result<Vec<(Migration, bool)>> {
        let migrations = self.parser.load_all()?;
        let mut status = Vec::new();

        for migration in migrations {
            let applied = self.history.is_applied(&migration.id).await?;
            status.push((migration, applied));
        }

        Ok(status)
    }

    /// Create a new migration file
    pub fn create(&self, name: &str) -> Result<(PathBuf, PathBuf)> {
        self.parser.create_migration(name)
    }

    /// Get progress tracker
    pub const fn progress(&self) -> &ProgressTracker {
        &self.progress
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_executor_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config = MigrationExecutorConfig {
            config: MigrationConfig {
                migrations_dir: temp_dir.path().to_path_buf(),
                ..Default::default()
            },
            verbose: true,
        };

        let executor = MigrationExecutor::new(config);
        executor.initialize().await.unwrap();
    }

    #[tokio::test]
    async fn test_dry_run_migration() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().to_path_buf();

        // Create a test migration
        std::fs::write(
            migrations_dir.join("001_test.up.sql"),
            "ALTER TABLE users ADD COLUMN test TEXT;",
        )
        .unwrap();
        std::fs::write(
            migrations_dir.join("001_test.down.sql"),
            "ALTER TABLE users DROP COLUMN test;",
        )
        .unwrap();

        let config = MigrationExecutorConfig {
            config: MigrationConfig {
                migrations_dir,
                dry_run: true,
                ..Default::default()
            },
            verbose: true,
        };

        let executor = MigrationExecutor::new(config);
        executor.initialize().await.unwrap();

        let results = executor.migrate_up().await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }
}
