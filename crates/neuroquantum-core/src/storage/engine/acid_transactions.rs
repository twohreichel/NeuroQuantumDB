//! ACID Transaction Operations for `StorageEngine`
//!
//! This module implements full ACID transaction support with:
//! - Write-Ahead Logging (WAL)
//! - Lock-based concurrency control
//! - Isolation levels (Read Committed, Serializable)
//! - Savepoints for partial rollback

use anyhow::{anyhow, Result};
use tracing::{debug, instrument};

use super::StorageEngine;
use crate::storage::query::{DeleteQuery, SelectQuery, UpdateQuery};
use crate::storage::row::Row;
use crate::storage::transaction_log::{Operation, LSN};
use crate::storage::types::RowId;
use crate::transaction::{IsolationLevel, LockType, TransactionId};

impl StorageEngine {
    /// Begin a new ACID transaction
    ///
    /// Starts a new transaction with Read Committed isolation level.
    /// All subsequent operations must use the returned transaction ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction cannot be started.
    #[instrument(level = "debug", skip(self))]
    pub async fn begin_acid_transaction(&self) -> Result<TransactionId> {
        self.transaction_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .map_err(|e| anyhow!("Failed to begin transaction: {e}"))
    }

    /// Begin a transaction with specific isolation level
    ///
    /// # Isolation Levels
    /// - `ReadCommitted`: See only committed data (default)
    /// - `RepeatableRead`: Consistent snapshot for the entire transaction
    /// - `Serializable`: Full isolation, transactions appear sequential
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction cannot be started.
    #[instrument(level = "debug", skip(self), fields(isolation_level = ?isolation_level))]
    pub async fn begin_acid_transaction_with_isolation(
        &self,
        isolation_level: IsolationLevel,
    ) -> Result<TransactionId> {
        self.transaction_manager
            .begin_transaction(isolation_level)
            .await
            .map_err(|e| anyhow!("Failed to begin transaction: {e}"))
    }

    /// Commit a transaction and persist pending writes to disk
    ///
    /// This method:
    /// 1. Retrieves pending operations from the undo log
    /// 2. Writes INSERT operations to disk (updates are already persisted)
    /// 3. Marks the transaction as committed in the WAL
    ///
    /// # Errors
    ///
    /// Returns an error if commit fails or disk writes fail.
    #[instrument(level = "debug", skip(self), fields(tx_id = ?tx_id))]
    pub async fn commit_acid_transaction(&mut self, tx_id: TransactionId) -> Result<()> {
        use crate::transaction::LogRecordType;

        debug!("💾 Committing transaction: {:?}", tx_id);

        // Get the undo log to find pending writes (inserts/updates)
        if let Some(log_entries) = self.transaction_manager.get_undo_log(tx_id).await {
            for entry in &log_entries {
                if let LogRecordType::Update {
                    before_image,
                    table,
                    after_image,
                    ..
                } = &entry.record_type
                {
                    // If there's no before_image, this was an INSERT - write to disk
                    if before_image.is_none() {
                        if let Ok(row) = serde_json::from_slice::<Row>(after_image) {
                            debug!(
                                "COMMIT: Writing inserted row {} to disk for table {}",
                                row.id, table
                            );
                            self.append_row_to_file(table, &row).await?;
                        }
                    }
                    // For updates, the data is already on disk (we update in place)
                }
            }
        }

        // Now complete the transaction commit
        self.transaction_manager
            .commit(tx_id)
            .await
            .map_err(|e| anyhow!("Failed to commit transaction: {e}"))
    }

    /// Rollback a transaction and undo all changes
    ///
    /// This method:
    /// 1. Retrieves the undo log for the transaction
    /// 2. Applies undo operations in reverse order
    /// 3. Marks the transaction as aborted
    ///
    /// # Errors
    ///
    /// Returns an error if rollback fails.
    pub async fn rollback_acid_transaction(&mut self, tx_id: TransactionId) -> Result<()> {
        use crate::transaction::LogRecordType;

        debug!("🔙 Rolling back transaction: {:?}", tx_id);

        // Get the undo log for this transaction before rolling back
        let undo_log = self.transaction_manager.get_undo_log(tx_id).await;

        // Apply undo operations in reverse order
        if let Some(log_entries) = undo_log {
            for entry in log_entries.iter().rev() {
                if let LogRecordType::Update {
                    before_image, key, ..
                } = &entry.record_type
                {
                    let row_id: RowId = key.parse().unwrap_or(0);
                    if before_image.is_none() {
                        // This was an INSERT - we need to delete the row
                        debug!("ROLLBACK: Deleting inserted row {}", key);
                        // Remove from compressed blocks
                        self.compressed_blocks.remove(&row_id);
                        // Remove from cache
                        self.row_cache.pop(&row_id);
                    } else {
                        // This was an UPDATE or DELETE - restore the before image
                        debug!("ROLLBACK: Restoring before image for row {}", key);
                        if let Some(before) = before_image {
                            if let Ok(row) = serde_json::from_slice::<Row>(before) {
                                let compressed = self.compress_row(&row).await?;
                                self.compressed_blocks.insert(row_id, compressed);
                                self.add_to_cache(row);
                            }
                        }
                    }
                }
            }
        }

        // Now call the transaction manager to complete the rollback
        self.transaction_manager
            .rollback(tx_id)
            .await
            .map_err(|e| anyhow!("Failed to rollback transaction: {e}"))
    }

    /// Rollback transaction to a savepoint
    ///
    /// This allows partial rollback of a transaction, undoing only
    /// operations that occurred after the savepoint.
    ///
    /// # Returns
    ///
    /// The number of operations that were undone.
    ///
    /// # Errors
    ///
    /// Returns an error if the savepoint is not found or rollback fails.
    pub async fn rollback_acid_to_savepoint(
        &mut self,
        tx_id: TransactionId,
        savepoint_lsn: LSN,
    ) -> Result<u64> {
        use crate::transaction::LogRecordType;

        debug!(
            "↩️  Rolling back transaction {:?} to savepoint LSN {}",
            tx_id, savepoint_lsn
        );

        // Get the undo log for this transaction
        let undo_log = self.transaction_manager.get_undo_log(tx_id).await;
        let mut operations_undone = 0u64;

        // Apply undo operations in reverse order for entries after savepoint
        if let Some(log_entries) = undo_log {
            for entry in log_entries.iter().rev() {
                // Only undo operations that occurred after the savepoint
                if entry.lsn <= savepoint_lsn {
                    break;
                }

                operations_undone += 1;

                if let LogRecordType::Update {
                    before_image, key, ..
                } = &entry.record_type
                {
                    let row_id: RowId = key.parse().unwrap_or(0);
                    if before_image.is_none() {
                        // This was an INSERT - we need to delete the row
                        debug!("ROLLBACK TO SAVEPOINT: Deleting inserted row {}", key);
                        self.compressed_blocks.remove(&row_id);
                        self.row_cache.pop(&row_id);
                    } else {
                        // This was an UPDATE or DELETE - restore the before image
                        debug!(
                            "ROLLBACK TO SAVEPOINT: Restoring before image for row {}",
                            key
                        );
                        if let Some(before) = before_image {
                            if let Ok(row) = serde_json::from_slice::<Row>(before) {
                                let compressed = self.compress_row(&row).await?;
                                self.compressed_blocks.insert(row_id, compressed);
                                self.add_to_cache(row);
                            }
                        }
                    }
                }
            }
        }

        // Call the transaction manager to update its internal state
        self.transaction_manager
            .rollback_to_savepoint(tx_id, savepoint_lsn)
            .await
            .map_err(|e| anyhow!("Failed to rollback to savepoint: {e}"))?;

        Ok(operations_undone)
    }

    /// Get the undo log for a transaction
    ///
    /// Returns the list of log records for the transaction, which can be
    /// used for debugging or custom recovery procedures.
    pub async fn get_acid_undo_log(
        &self,
        tx_id: TransactionId,
    ) -> Option<Vec<crate::transaction::LogRecord>> {
        self.transaction_manager.get_undo_log(tx_id).await
    }

    /// Insert a row within a transaction
    ///
    /// This method:
    /// 1. Acquires an exclusive lock on the table
    /// 2. Logs the operation to WAL before applying
    /// 3. Applies changes to memory only (disk write on commit)
    ///
    /// # Errors
    ///
    /// Returns an error if lock acquisition fails or validation fails.
    pub async fn insert_row_acid(
        &mut self,
        tx_id: TransactionId,
        table: &str,
        mut row: Row,
    ) -> Result<RowId> {
        debug!("➕ Transactional insert into table: {}", table);

        // Acquire exclusive lock on table
        let resource_id = format!("table:{table}");
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Exclusive)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {e}"))?;

        // Get table schema
        let schema = self
            .metadata
            .tables
            .get(table)
            .ok_or_else(|| anyhow!("Table '{table}' does not exist"))?
            .clone();

        // Assign row ID
        row.id = self.next_row_id;
        self.next_row_id += 1;

        // Validate row against schema
        self.validate_row(&schema, &row)?;

        // Compress row data
        let compressed_data = self.compress_row(&row).await?;

        // Log the operation to WAL before applying changes
        let after_image = serde_json::to_vec(&row)?;
        self.transaction_manager
            .log_update(
                tx_id,
                table.to_string(),
                row.id.to_string(),
                None, // No before-image for INSERT
                after_image,
            )
            .await
            .map_err(|e| anyhow!("Failed to log update: {e}"))?;

        // Apply changes to memory only (disk write happens at commit time)
        self.compressed_blocks.insert(row.id, compressed_data);
        self.update_indexes_for_insert(&schema, &row)?;
        self.add_to_cache(row.clone());

        // Log operation to local transaction log (for commit-time disk writes)
        let operation = Operation::Insert {
            table: table.to_string(),
            row_id: row.id,
            data: row.clone(),
        };
        self.log_operation(operation).await?;

        debug!(
            "✅ Row inserted with ID: {} (tx: {:?}) - pending commit",
            row.id, tx_id
        );
        Ok(row.id)
    }

    /// Update rows within a transaction
    ///
    /// This method:
    /// 1. Acquires an exclusive lock on the table
    /// 2. Logs before/after images to WAL
    /// 3. Applies changes immediately (for durability)
    ///
    /// # Errors
    ///
    /// Returns an error if lock acquisition fails or validation fails.
    pub async fn update_rows_acid(
        &mut self,
        tx_id: TransactionId,
        query: &UpdateQuery,
    ) -> Result<u64> {
        debug!("✏️ Transactional update in table: {}", query.table);

        // Acquire exclusive lock on table
        let resource_id = format!("table:{}", query.table);
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Exclusive)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {e}"))?;

        // Get existing rows that match the condition
        let select_query = SelectQuery {
            table: query.table.clone(),
            columns: vec!["*".to_string()],
            where_clause: query.where_clause.clone(),
            order_by: None,
            limit: None,
            offset: None,
        };

        let existing_rows = self.select_rows(&select_query).await?;
        let mut updated_count = 0;
        let mut updated_rows = Vec::new();

        for mut row in existing_rows {
            let old_row = row.clone();

            // Serialize before-image for WAL
            let before_image = serde_json::to_vec(&old_row)?;

            // Apply updates
            for (field, new_value) in &query.set_values {
                row.fields.insert(field.clone(), new_value.clone());
            }
            row.updated_at = chrono::Utc::now();

            // Validate updated row
            let schema = self
                .metadata
                .tables
                .get(&query.table)
                .ok_or_else(|| anyhow!("Table '{}' schema not found", query.table))?;
            self.validate_row(schema, &row)?;

            // Serialize after-image for WAL
            let after_image = serde_json::to_vec(&row)?;

            // Log to WAL
            self.transaction_manager
                .log_update(
                    tx_id,
                    query.table.clone(),
                    row.id.to_string(),
                    Some(before_image),
                    after_image,
                )
                .await
                .map_err(|e| anyhow!("Failed to log update: {e}"))?;

            // Apply changes
            let compressed_data = self.compress_row(&row).await?;
            self.compressed_blocks.insert(row.id, compressed_data);
            self.add_to_cache(row.clone());
            updated_rows.push(row.clone());

            // Log operation
            let operation = Operation::Update {
                table: query.table.clone(),
                row_id: row.id,
                old_data: old_row,
                new_data: row,
            };
            self.log_operation(operation).await?;

            updated_count += 1;
        }

        // Rewrite table file with updated data
        if updated_count > 0 {
            self.rewrite_table_file_with_updates(&query.table, &updated_rows)
                .await?;
        }

        debug!("✅ Updated {} rows (tx: {:?})", updated_count, tx_id);
        Ok(updated_count)
    }

    /// Delete rows within a transaction
    ///
    /// This method:
    /// 1. Acquires an exclusive lock on the table
    /// 2. Logs before-images to WAL (empty after-image indicates DELETE)
    /// 3. Applies changes immediately
    ///
    /// # Errors
    ///
    /// Returns an error if lock acquisition fails.
    pub async fn delete_rows_acid(
        &mut self,
        tx_id: TransactionId,
        query: &DeleteQuery,
    ) -> Result<u64> {
        debug!("🗑️ Transactional delete from table: {}", query.table);

        // Acquire exclusive lock on table
        let resource_id = format!("table:{}", query.table);
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Exclusive)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {e}"))?;

        // Get existing rows that match the condition
        let select_query = SelectQuery {
            table: query.table.clone(),
            columns: vec!["*".to_string()],
            where_clause: query.where_clause.clone(),
            order_by: None,
            limit: None,
            offset: None,
        };

        let rows_to_delete = self.select_rows(&select_query).await?;
        let deleted_count = rows_to_delete.len();
        let mut deleted_row_ids = Vec::new();

        for row in rows_to_delete {
            // Serialize before-image for WAL
            let before_image = serde_json::to_vec(&row)?;

            // Log to WAL (DELETE has before-image, empty after-image)
            self.transaction_manager
                .log_update(
                    tx_id,
                    query.table.clone(),
                    row.id.to_string(),
                    Some(before_image),
                    vec![], // Empty after-image indicates DELETE
                )
                .await
                .map_err(|e| anyhow!("Failed to log delete: {e}"))?;

            // Keep track of deleted row IDs
            deleted_row_ids.push(row.id);

            // Apply changes
            self.compressed_blocks.remove(&row.id);
            self.row_cache.pop(&row.id);

            let schema = self
                .metadata
                .tables
                .get(&query.table)
                .ok_or_else(|| anyhow!("Table '{}' schema not found", query.table))?
                .clone();
            self.update_indexes_for_delete(&schema, &row)?;

            // Log operation
            let operation = Operation::Delete {
                table: query.table.clone(),
                row_id: row.id,
                data: row,
            };
            self.log_operation(operation).await?;
        }

        // Rewrite table file without deleted rows
        if deleted_count > 0 {
            self.rewrite_table_file_with_deletions(&query.table, &deleted_row_ids)
                .await?;
        }

        debug!("✅ Deleted {} rows (tx: {:?})", deleted_count, tx_id);
        Ok(deleted_count as u64)
    }

    /// Select rows within a transaction (with appropriate locking)
    ///
    /// Acquires a shared lock on the table to ensure consistent reads.
    ///
    /// # Errors
    ///
    /// Returns an error if lock acquisition fails or query fails.
    pub async fn select_rows_acid(
        &self,
        tx_id: TransactionId,
        query: &SelectQuery,
    ) -> Result<Vec<Row>> {
        debug!("🔍 Transactional select from table: {}", query.table);

        // Acquire shared lock on table for consistent reads
        let resource_id = format!("table:{}", query.table);
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Shared)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {e}"))?;

        // Perform the select (now protected by lock)
        self.select_rows(query).await
    }

    /// Execute a full transaction with automatic commit/rollback
    ///
    /// This is the recommended way to execute transactions. The closure
    /// receives the storage engine and transaction ID, and can perform
    /// any operations. On success, the transaction is automatically committed.
    /// On error, it is automatically rolled back.
    ///
    /// # Example
    /// ```ignore
    /// storage.execute_acid_transaction(|storage, tx_id| {
    ///     Box::pin(async move {
    ///         storage.insert_row_acid(tx_id, "users", row).await?;
    ///         Ok(())
    ///     })
    /// }).await?;
    /// ```
    pub async fn execute_acid_transaction<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce(
            &mut Self,
            TransactionId,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + '_>>,
    {
        // Begin transaction
        let tx_id = self.begin_acid_transaction().await?;

        // Execute the transaction function
        let result = f(self, tx_id).await;

        match result {
            | Ok(value) => {
                // Commit on success
                self.commit_acid_transaction(tx_id).await?;
                Ok(value)
            },
            | Err(e) => {
                // Rollback on error
                let _ = self.rollback_acid_transaction(tx_id).await; // Ignore rollback errors
                Err(e)
            },
        }
    }

    /// Get transaction statistics
    ///
    /// Returns statistics about transaction activity including:
    /// - Total transactions started
    /// - Committed/aborted counts
    /// - Average transaction duration
    /// - Lock wait time
    pub async fn get_acid_transaction_statistics(
        &self,
    ) -> crate::transaction::TransactionStatistics {
        self.transaction_manager.get_statistics().await
    }

    // ================= BACKWARDS COMPATIBILITY ALIASES =================
    // These methods provide backwards compatibility with the old API names.
    // New code should prefer the `_acid` variants for clarity.

    /// Begin a new transaction (alias for `begin_acid_transaction`)
    #[inline]
    pub async fn begin_transaction(&self) -> Result<TransactionId> {
        self.begin_acid_transaction().await
    }

    /// Begin a transaction with specific isolation level (alias for `begin_acid_transaction_with_isolation`)
    #[inline]
    pub async fn begin_transaction_with_isolation(
        &self,
        isolation_level: IsolationLevel,
    ) -> Result<TransactionId> {
        self.begin_acid_transaction_with_isolation(isolation_level)
            .await
    }

    /// Commit a transaction (alias for `commit_acid_transaction`)
    #[inline]
    pub async fn commit_transaction(&mut self, tx_id: TransactionId) -> Result<()> {
        self.commit_acid_transaction(tx_id).await
    }

    /// Rollback a transaction (alias for `rollback_acid_transaction`)
    #[inline]
    pub async fn rollback_transaction(&mut self, tx_id: TransactionId) -> Result<()> {
        self.rollback_acid_transaction(tx_id).await
    }

    /// Rollback to a savepoint (alias for `rollback_acid_to_savepoint`)
    #[inline]
    pub async fn rollback_to_savepoint(
        &mut self,
        tx_id: TransactionId,
        savepoint_lsn: LSN,
    ) -> Result<u64> {
        self.rollback_acid_to_savepoint(tx_id, savepoint_lsn).await
    }

    /// Get the undo log for a transaction (alias for `get_acid_undo_log`)
    #[inline]
    pub async fn get_undo_log(
        &self,
        tx_id: TransactionId,
    ) -> Option<Vec<crate::transaction::LogRecord>> {
        self.get_acid_undo_log(tx_id).await
    }

    /// Insert a row within a transaction (alias for `insert_row_acid`)
    #[inline]
    pub async fn insert_row_transactional(
        &mut self,
        tx_id: TransactionId,
        table: &str,
        row: Row,
    ) -> Result<RowId> {
        self.insert_row_acid(tx_id, table, row).await
    }

    /// Update rows within a transaction (alias for `update_rows_acid`)
    #[inline]
    pub async fn update_rows_transactional(
        &mut self,
        tx_id: TransactionId,
        query: &UpdateQuery,
    ) -> Result<u64> {
        self.update_rows_acid(tx_id, query).await
    }

    /// Delete rows within a transaction (alias for `delete_rows_acid`)
    #[inline]
    pub async fn delete_rows_transactional(
        &mut self,
        tx_id: TransactionId,
        query: &DeleteQuery,
    ) -> Result<u64> {
        self.delete_rows_acid(tx_id, query).await
    }

    /// Select rows within a transaction (alias for `select_rows_acid`)
    #[inline]
    pub async fn select_rows_transactional(
        &self,
        tx_id: TransactionId,
        query: &SelectQuery,
    ) -> Result<Vec<Row>> {
        self.select_rows_acid(tx_id, query).await
    }

    /// Get transaction statistics (alias for `get_acid_transaction_statistics`)
    #[inline]
    pub async fn get_transaction_statistics(&self) -> crate::transaction::TransactionStatistics {
        self.get_acid_transaction_statistics().await
    }
}
