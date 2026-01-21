//! Recovery operations for `StorageEngine`
//!
//! This module implements crash recovery using Write-Ahead Logging (WAL):
//! - REDO: Replay committed transactions
//! - UNDO: Rollback uncommitted transactions
//! - Checkpoint integration

use std::collections::HashSet;

use anyhow::{anyhow, Result};
use tracing::{debug, info};

use super::StorageEngine;
use crate::storage::row::Row;
use crate::storage::transaction_log::Operation;
use crate::storage::types::RowId;
use crate::transaction::{LogRecordType, TransactionId};

impl StorageEngine {
    /// Apply redo operation (replay committed changes)
    ///
    /// Note: This is kept for future use in recovery operations.
    /// Currently changes are persisted immediately, making REDO unnecessary.
    #[allow(dead_code)]
    pub(crate) const fn apply_redo(&mut self, _operation: &Operation) -> Result<()> {
        // REDO operations restore the after-image
        // In our current implementation, changes are persisted immediately,
        // so REDO is typically not needed during recovery.
        // This method exists for future optimizations where writes are deferred.
        Ok(())
    }

    /// Apply undo operation (rollback uncommitted changes)
    ///
    /// Note: This is kept for future use in recovery operations.
    #[allow(dead_code)]
    pub(crate) fn apply_undo(&mut self, operation: &Operation) -> Result<()> {
        match operation {
            | Operation::Insert { row_id, .. } => {
                // Undo INSERT by removing the row
                self.compressed_blocks.remove(row_id);
                self.row_cache.pop(row_id);
            },
            | Operation::Update {
                row_id, old_data, ..
            } => {
                // Undo UPDATE by restoring old data to cache
                // Note: compressed data restoration would need async context
                self.add_to_cache(old_data.clone());
                debug!("Undoing update for row {}", row_id);
            },
            | Operation::Delete { row_id, data, .. } => {
                // Undo DELETE by restoring the row to cache
                self.add_to_cache(data.clone());
                debug!("Undoing delete for row {}", row_id);
            },
            | Operation::CreateTable { schema } => {
                // Undo CREATE TABLE by removing from metadata
                self.metadata.tables.remove(&schema.name);
                debug!("Undoing create table: {}", schema.name);
            },
            | Operation::DropTable { table } => {
                // Cannot fully undo DROP TABLE without the schema
                // This should be handled at a higher level with full schema backup
                debug!(
                    "Cannot fully undo drop table: {} (schema not preserved)",
                    table
                );
            },
            | Operation::AlterTable {
                table, old_schema, ..
            } => {
                // Undo ALTER TABLE by restoring the original schema
                self.metadata
                    .tables
                    .insert(table.clone(), old_schema.clone());
                debug!("Undoing alter table: {} (restored original schema)", table);
            },
        }
        Ok(())
    }

    /// Apply after-image to storage (REDO operation for recovery)
    pub async fn apply_after_image(
        &mut self,
        table: &str,
        key: &str,
        after_image: &[u8],
    ) -> Result<()> {
        debug!("♻️  Applying after-image (REDO) for {}.{}", table, key);

        // Deserialize the row from after-image
        let row: Row = serde_json::from_slice(after_image)
            .map_err(|e| anyhow!("Failed to deserialize after-image: {e}"))?;

        // Check if table exists
        let schema = self
            .metadata
            .tables
            .get(table)
            .ok_or_else(|| anyhow!("Table '{table}' does not exist"))?
            .clone();

        // Apply the row to storage
        let compressed_data = self.compress_row(&row).await?;
        self.compressed_blocks.insert(row.id, compressed_data);

        // Update indexes
        self.update_indexes_for_insert(&schema, &row)?;

        // Add to cache
        self.add_to_cache(row.clone());

        debug!("✅ REDO applied for row ID: {}", row.id);
        Ok(())
    }

    /// Apply before-image to storage (UNDO operation for recovery)
    pub async fn apply_before_image(
        &mut self,
        table: &str,
        key: &str,
        before_image: Option<&[u8]>,
    ) -> Result<()> {
        debug!("⏪ Applying before-image (UNDO) for {}.{}", table, key);

        if let Some(before_data) = before_image {
            // Deserialize the old row from before-image
            let old_row: Row = serde_json::from_slice(before_data)
                .map_err(|e| anyhow!("Failed to deserialize before-image: {e}"))?;

            // Check if table exists
            let schema = self
                .metadata
                .tables
                .get(table)
                .ok_or_else(|| anyhow!("Table '{table}' does not exist"))?
                .clone();

            // Restore the old row to storage
            let compressed_data = self.compress_row(&old_row).await?;
            self.compressed_blocks.insert(old_row.id, compressed_data);

            // Update indexes to old state
            self.update_indexes_for_insert(&schema, &old_row)?;

            // Update cache
            self.add_to_cache(old_row.clone());

            debug!("✅ UNDO applied for row ID: {}", old_row.id);
        } else {
            // No before-image means this was an INSERT - we need to remove the row
            // Parse row ID from key
            if let Ok(row_id) = key.parse::<RowId>() {
                self.compressed_blocks.remove(&row_id);
                self.row_cache.pop(&row_id);

                debug!("✅ UNDO applied: removed row ID {}", row_id);
            } else {
                tracing::warn!("Could not parse row ID from key: {}", key);
            }
        }

        Ok(())
    }

    /// Apply a log record during recovery (convenience method)
    pub async fn apply_log_record(
        &mut self,
        record: &crate::transaction::LogRecord,
        is_redo: bool,
    ) -> Result<()> {
        match &record.record_type {
            | LogRecordType::Update {
                table,
                key,
                before_image,
                after_image,
                ..
            } => {
                if is_redo {
                    self.apply_after_image(table, key, after_image).await?;
                } else {
                    self.apply_before_image(table, key, before_image.as_deref())
                        .await?;
                }
            },
            | _ => {
                // Other log record types don't need storage application
                debug!("Skipping non-update log record type");
            },
        }

        Ok(())
    }

    /// Perform crash recovery by replaying WAL logs
    ///
    /// This implements the ARIES-style recovery algorithm:
    /// 1. **Analysis Phase**: Scan log to identify active and committed transactions
    /// 2. **REDO Phase**: Replay all changes from committed transactions
    /// 3. **UNDO Phase**: Rollback all changes from uncommitted transactions
    pub async fn perform_recovery(&mut self) -> Result<()> {
        info!("🔄 Starting storage-level crash recovery...");

        // Get the WAL log manager from transaction manager
        let log_dir = self.data_dir.join("logs");
        let log_manager = crate::transaction::LogManager::new(&log_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize log manager: {e}"))?;

        let log_records = log_manager
            .read_log()
            .await
            .map_err(|e| anyhow!("Failed to read log: {e}"))?;

        if log_records.is_empty() {
            info!("No log records to recover");
            return Ok(());
        }

        // Phase 1: Analysis - determine which transactions to undo/redo
        let mut active_txs: HashSet<TransactionId> = HashSet::new();
        let mut committed_txs: HashSet<TransactionId> = HashSet::new();

        for record in &log_records {
            match &record.record_type {
                | LogRecordType::Begin { tx_id, .. } => {
                    active_txs.insert(*tx_id);
                },
                | LogRecordType::Commit { tx_id } => {
                    active_txs.remove(tx_id);
                    committed_txs.insert(*tx_id);
                },
                | LogRecordType::Abort { tx_id } => {
                    active_txs.remove(tx_id);
                },
                | _ => {},
            }
        }

        info!(
            "Analysis: {} transactions to undo, {} to redo",
            active_txs.len(),
            committed_txs.len()
        );

        // Phase 2: REDO - reapply committed transactions
        for record in &log_records {
            if let Some(tx_id) = record.tx_id {
                if committed_txs.contains(&tx_id) {
                    self.apply_log_record(record, true).await?;
                }
            }
        }

        info!("REDO phase completed");

        // Phase 3: UNDO - rollback uncommitted transactions
        for record in log_records.iter().rev() {
            if let Some(tx_id) = record.tx_id {
                if active_txs.contains(&tx_id) {
                    self.apply_log_record(record, false).await?;
                }
            }
        }

        info!("UNDO phase completed");
        info!("✅ Storage-level crash recovery completed");

        Ok(())
    }

    /// Write a checkpoint for recovery
    ///
    /// A checkpoint records the current state of all active transactions
    /// and forces all dirty pages to disk. This reduces recovery time by
    /// establishing a known-good starting point.
    pub async fn checkpoint(&self) -> Result<()> {
        self.transaction_manager
            .checkpoint()
            .await
            .map_err(|e| anyhow!("Failed to write checkpoint: {e}"))
    }

    /// Cleanup timed out transactions
    ///
    /// Automatically rolls back transactions that have exceeded their timeout.
    pub async fn cleanup_timed_out_transactions(&self) -> Result<()> {
        self.transaction_manager
            .cleanup_timed_out_transactions()
            .await
            .map_err(|e| anyhow!("Failed to cleanup transactions: {e}"))
    }
}
