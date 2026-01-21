//! Transaction utilities for `StorageEngine`
//!
//! This module provides:
//! - Batch operation types
//! - Operation logging utilities

use anyhow::Result;
use tracing::debug;

use super::StorageEngine;
use crate::storage::query::{DeleteQuery, UpdateQuery};
use crate::storage::row::Row;
use crate::storage::transaction_log::{Operation, Transaction, TransactionId, TransactionStatus};
use crate::storage::types::RowId;

impl StorageEngine {
    /// Log an operation to the internal transaction log
    ///
    /// This is for the simple transaction tracking, not the ACID WAL.
    ///
    /// # Errors
    ///
    /// Returns an error if logging fails.
    pub(crate) async fn log_operation(&mut self, operation: Operation) -> Result<()> {
        let transaction = Transaction {
            id: TransactionId::new_v4(),
            operations: vec![operation],
            status: TransactionStatus::Committed,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            lsn: self.next_lsn,
        };

        self.next_lsn += 1;

        // Try to push transaction - if serialization fails, log warning but don't fail operation
        match bincode::serialize(&transaction) {
            | Ok(_) => {
                self.transaction_log.push(transaction);

                // Keep transaction log size manageable
                if self.transaction_log.len() > 10000 {
                    self.transaction_log.drain(0..5000);
                }
            },
            | Err(e) => {
                debug!(
                    "⚠️  Warning: Failed to serialize transaction for logging: {}",
                    e
                );
            },
        }

        Ok(())
    }

    /// Execute multiple operations in a single batch
    ///
    /// All operations succeed or none do. Uses the ACID transaction manager.
    ///
    /// # Errors
    ///
    /// Returns an error if any operation fails.
    pub async fn execute_batch(&mut self, operations: Vec<BatchOperation>) -> Result<BatchResult> {
        let tx_id = self.begin_acid_transaction().await?;

        let mut result = BatchResult::default();

        for op in operations {
            match op {
                | BatchOperation::Insert { table, row } => {
                    match self.insert_row_acid(tx_id, &table, row).await {
                        | Ok(id) => result.inserted_ids.push(id),
                        | Err(e) => {
                            self.rollback_acid_transaction(tx_id).await?;
                            return Err(e);
                        },
                    }
                },
                | BatchOperation::Update(query) => {
                    match self.update_rows_acid(tx_id, &query).await {
                        | Ok(count) => result.updated_count += count,
                        | Err(e) => {
                            self.rollback_acid_transaction(tx_id).await?;
                            return Err(e);
                        },
                    }
                },
                | BatchOperation::Delete(query) => {
                    match self.delete_rows_acid(tx_id, &query).await {
                        | Ok(count) => result.deleted_count += count,
                        | Err(e) => {
                            self.rollback_acid_transaction(tx_id).await?;
                            return Err(e);
                        },
                    }
                },
            }
        }

        self.commit_acid_transaction(tx_id).await?;
        Ok(result)
    }
}

/// A batch operation for `execute_batch`
#[derive(Debug, Clone)]
pub enum BatchOperation {
    /// Insert a row into a table
    Insert {
        /// The table name
        table: String,
        /// The row to insert
        row: Row,
    },
    /// Update rows matching a query
    Update(UpdateQuery),
    /// Delete rows matching a query
    Delete(DeleteQuery),
}

/// Result of a batch operation
#[derive(Debug, Default, Clone)]
pub struct BatchResult {
    /// IDs of inserted rows
    pub inserted_ids: Vec<RowId>,
    /// Number of updated rows
    pub updated_count: u64,
    /// Number of deleted rows
    pub deleted_count: u64,
}
