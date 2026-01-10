//! Migration history tracking
//!
//! Tracks applied migrations in a special table.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::storage::migration::MigrationId;

/// Status of a migration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Migration is pending
    Pending,
    /// Migration is currently running
    Running,
    /// Migration completed successfully
    Completed,
    /// Migration failed
    Failed,
    /// Migration was rolled back
    RolledBack,
}

/// Record of an applied migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Migration identifier
    pub migration_id: MigrationId,
    /// When the migration was applied
    pub applied_at: DateTime<Utc>,
    /// Status of the migration
    pub status: MigrationStatus,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of rows affected
    pub rows_affected: Option<u64>,
    /// Checksum of the migration SQL
    pub checksum: String,
}

/// Manages migration history
pub struct MigrationHistory {
    /// In-memory cache of migration records
    records: RwLock<HashMap<MigrationId, MigrationRecord>>,
}

impl MigrationHistory {
    /// Create a new migration history tracker
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize the migration history table
    pub async fn initialize(&self) -> Result<()> {
        // This would create the migration history table in the database
        // For now, we'll use in-memory storage
        Ok(())
    }

    /// Record a migration as started
    pub async fn record_start(&self, migration_id: MigrationId) -> Result<()> {
        let mut records = self.records.write().await;
        records.insert(
            migration_id.clone(),
            MigrationRecord {
                migration_id,
                applied_at: Utc::now(),
                status: MigrationStatus::Running,
                duration_ms: 0,
                error: None,
                rows_affected: None,
                checksum: String::new(),
            },
        );
        Ok(())
    }

    /// Record a migration as completed
    pub async fn record_complete(
        &self,
        migration_id: MigrationId,
        duration_ms: u64,
        rows_affected: Option<u64>,
        checksum: String,
    ) -> Result<()> {
        let mut records = self.records.write().await;
        if let Some(record) = records.get_mut(&migration_id) {
            record.status = MigrationStatus::Completed;
            record.duration_ms = duration_ms;
            record.rows_affected = rows_affected;
            record.checksum = checksum;
        }
        Ok(())
    }

    /// Record a migration as failed
    pub async fn record_failure(
        &self,
        migration_id: MigrationId,
        error: String,
        duration_ms: u64,
    ) -> Result<()> {
        let mut records = self.records.write().await;
        if let Some(record) = records.get_mut(&migration_id) {
            record.status = MigrationStatus::Failed;
            record.error = Some(error);
            record.duration_ms = duration_ms;
        }
        Ok(())
    }

    /// Record a migration as rolled back
    pub async fn record_rollback(&self, migration_id: MigrationId) -> Result<()> {
        let mut records = self.records.write().await;
        if let Some(record) = records.get_mut(&migration_id) {
            record.status = MigrationStatus::RolledBack;
        }
        Ok(())
    }

    /// Get all migration records
    pub async fn get_all(&self) -> Result<Vec<MigrationRecord>> {
        let records = self.records.read().await;
        Ok(records.values().cloned().collect())
    }

    /// Get a specific migration record
    pub async fn get(&self, migration_id: &MigrationId) -> Result<Option<MigrationRecord>> {
        let records = self.records.read().await;
        Ok(records.get(migration_id).cloned())
    }

    /// Check if a migration has been applied
    pub async fn is_applied(&self, migration_id: &MigrationId) -> Result<bool> {
        let records = self.records.read().await;
        Ok(records
            .get(migration_id)
            .map(|r| r.status == MigrationStatus::Completed)
            .unwrap_or(false))
    }

    /// Get pending migrations from a list
    pub async fn get_pending(
        &self,
        all_migrations: Vec<MigrationId>,
    ) -> Result<Vec<MigrationId>> {
        let records = self.records.read().await;
        Ok(all_migrations
            .into_iter()
            .filter(|id| {
                !records
                    .get(id)
                    .map(|r| r.status == MigrationStatus::Completed)
                    .unwrap_or(false)
            })
            .collect())
    }
}

impl Default for MigrationHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_lifecycle() {
        let history = MigrationHistory::new();
        let migration_id = "001".to_string();

        // Record start
        history.record_start(migration_id.clone()).await.unwrap();
        let record = history.get(&migration_id).await.unwrap().unwrap();
        assert_eq!(record.status, MigrationStatus::Running);

        // Record completion
        history
            .record_complete(migration_id.clone(), 1000, Some(10), "abc123".to_string())
            .await
            .unwrap();
        let record = history.get(&migration_id).await.unwrap().unwrap();
        assert_eq!(record.status, MigrationStatus::Completed);
        assert_eq!(record.duration_ms, 1000);
        assert_eq!(record.rows_affected, Some(10));
    }

    #[tokio::test]
    async fn test_is_applied() {
        let history = MigrationHistory::new();
        let migration_id = "001".to_string();

        // Not applied initially
        assert!(!history.is_applied(&migration_id).await.unwrap());

        // Apply migration
        history.record_start(migration_id.clone()).await.unwrap();
        history
            .record_complete(migration_id.clone(), 1000, None, "abc".to_string())
            .await
            .unwrap();

        // Now applied
        assert!(history.is_applied(&migration_id).await.unwrap());
    }
}
