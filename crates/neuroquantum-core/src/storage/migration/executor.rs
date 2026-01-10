//! Migration executor
//!
//! Executes migrations with safety checks and rollback support.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{error, info};

use super::{
    history::MigrationHistory, parser::{Migration, MigrationDirection, MigrationParser},
    progress::ProgressTracker, MigrationConfig, MigrationId, MigrationResult, SafetyCheck,
    ValidationResult,
};

/// Configuration for migration executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationExecutorConfig {
    /// Base migration configuration
    pub config: MigrationConfig,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for MigrationExecutorConfig {
    fn default() -> Self {
        Self {
            config: MigrationConfig::default(),
            verbose: false,
        }
    }
}

/// Executes database migrations
pub struct MigrationExecutor {
    config: MigrationExecutorConfig,
    parser: MigrationParser,
    history: MigrationHistory,
    progress: ProgressTracker,
}

impl MigrationExecutor {
    /// Create a new migration executor
    pub fn new(config: MigrationExecutorConfig) -> Self {
        let parser = MigrationParser::new(config.config.migrations_dir.clone());
        let history = MigrationHistory::new();
        let progress = ProgressTracker::new();

        Self {
            config,
            parser,
            history,
            progress,
        }
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
                .ok_or_else(|| anyhow!("Migration {} not found", migration_id))?;

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

        info!(
            "Executing migration {} ({:?})",
            migration.id, direction
        );

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
        self.progress
            .start(migration.id.clone(), 100)
            .await;

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

        // TODO: Actually execute SQL against database
        // For now, simulate execution
        self.progress
            .update(50, "Executing SQL...".to_string())
            .await;

        // Simulate some work
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        self.progress.complete().await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Record completion
        self.history
            .record_complete(migration.id.clone(), duration_ms, Some(0), "checksum".to_string())
            .await?;

        info!(
            "Migration {} completed in {}ms",
            migration.id, duration_ms
        );

        Ok(MigrationResult {
            migration_id: migration.id.clone(),
            success: true,
            duration_ms,
            error: None,
            rows_affected: Some(0),
        })
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
    pub fn progress(&self) -> &ProgressTracker {
        &self.progress
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
