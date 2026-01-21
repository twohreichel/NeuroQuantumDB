//! CRUD operations for `StorageEngine`
//!
//! This module implements INSERT, SELECT, UPDATE, and DELETE operations.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use tracing::{debug, instrument};

use super::StorageEngine;
use crate::error::CoreError;
use crate::storage::query::{
    ComparisonOperator, Condition, DeleteQuery, SelectQuery, UpdateQuery, WhereClause,
};
use crate::storage::row::Row;
use crate::storage::stats::QueryExecutionStats;
use crate::storage::transaction_log::Operation;
use crate::storage::types::{DataType, RowId, TableSchema, Value};

impl StorageEngine {
    /// Insert a new row into the specified table
    ///
    /// Automatically handles:
    /// - Row ID assignment (internal, always auto-increment)
    /// - `AUTO_INCREMENT` columns (user-defined)
    /// - SERIAL/BIGSERIAL columns (PostgreSQL-style)
    /// - Timestamp fields (`created_at`, `updated_at`)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table doesn't exist
    /// - Row validation fails
    /// - Foreign key constraint is violated
    /// - Compression fails
    ///
    /// # Example
    /// ```ignore
    /// # async fn example() -> anyhow::Result<()> {
    /// # use neuroquantum_core::storage::{StorageEngine, Row, Value};
    /// # let mut storage = StorageEngine::new("./data").await?;
    /// // ID is automatically generated - no need to specify it!
    /// let mut row = Row::new();
    /// row.set("name", Value::Text("Alice".to_string().into()));
    /// row.set("email", Value::Text("alice@example.com".to_string().into()));
    /// let id = storage.insert_row("users", row).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(level = "debug", skip(self, row), fields(table = %table))]
    pub async fn insert_row(&mut self, table: &str, mut row: Row) -> Result<RowId> {
        debug!("➕ Inserting row into table: {}", table);

        // Get table schema (clone to avoid borrow issues)
        let schema = self
            .metadata
            .tables
            .get(table)
            .ok_or_else(|| anyhow!("Table '{table}' does not exist"))?
            .clone();

        // Assign internal row ID (always auto-increment for B+ tree efficiency)
        row.id = self.next_row_id;
        self.next_row_id += 1;

        // Process AUTO_INCREMENT columns
        self.populate_auto_increment_columns(table, &schema, &mut row)?;

        // Apply DEFAULT values for missing columns
        Self::populate_default_values(&schema, &mut row);

        // Validate row against schema
        self.validate_row(&schema, &row)?;

        // Validate foreign key constraints
        self.validate_foreign_key_constraints(&schema, &row).await?;

        // Compress row data using DNA compression
        let compressed_data = self.compress_row(&row).await?;

        // Store compressed data
        self.compressed_blocks.insert(row.id, compressed_data);

        // Update indexes
        self.update_indexes_for_insert(&schema, &row)?;

        // Append to table file with DNA compression (uses reference)
        self.append_row_to_file(table, &row).await?;

        // Log operation (needs clone for ownership)
        let operation = Operation::Insert {
            table: table.to_string(),
            row_id: row.id,
            data: row.clone(),
        };
        self.log_operation(operation).await?;

        // Add to cache (moves row, so do this last)
        let row_id = row.id;
        self.add_to_cache(row);

        debug!("✅ Row inserted with ID: {} (DNA compressed)", row_id);
        Ok(row_id)
    }

    /// Select rows matching the given query
    ///
    /// # Errors
    ///
    /// Returns an error if the table doesn't exist or query execution fails.
    #[instrument(level = "debug", skip(self, query), fields(table = %query.table))]
    pub async fn select_rows(&self, query: &SelectQuery) -> Result<Vec<Row>> {
        let (rows, _stats) = self.select_rows_with_stats(query).await?;
        Ok(rows)
    }

    /// Select rows matching the given query with execution statistics
    ///
    /// # Errors
    ///
    /// Returns an error if the table doesn't exist or query execution fails.
    #[instrument(level = "debug", skip(self, query), fields(table = %query.table))]
    pub async fn select_rows_with_stats(
        &self,
        query: &SelectQuery,
    ) -> Result<(Vec<Row>, QueryExecutionStats)> {
        debug!("🔍 Selecting rows from table: {}", query.table);

        let mut stats = QueryExecutionStats::default();

        // Get table schema
        let schema = self
            .metadata
            .tables
            .get(&query.table)
            .ok_or_else(|| anyhow!("Table '{}' does not exist", query.table))?;

        // Check if we can use an index for this query
        let index_key = format!("{}_{}", query.table, schema.primary_key);
        if self.indexes.contains_key(&index_key) {
            stats.indexes_used.push(index_key.clone());
            // Index exists but we're doing a full scan for now
            // In a more optimized implementation, we'd use the index for WHERE clauses
            stats.index_scan = false;
        }

        // Load all rows for the table
        let mut rows = self.load_table_rows(&query.table).await?;
        stats.rows_examined = rows.len();

        // Track cache hits/misses during row loading
        for row in &rows {
            if self.row_cache.contains(&row.id) {
                stats.cache_hits += 1;
            } else {
                stats.cache_misses += 1;
            }
        }

        // Apply WHERE clause
        if let Some(where_clause) = &query.where_clause {
            rows = self.apply_where_clause(rows, where_clause)?;
        }

        // Apply ORDER BY
        if let Some(order_by) = &query.order_by {
            self.apply_order_by(&mut rows, order_by)?;
        }

        // Apply LIMIT and OFFSET
        if let Some(offset) = query.offset {
            rows = rows.into_iter().skip(offset as usize).collect();
        }

        if let Some(limit) = query.limit {
            rows.truncate(limit as usize);
        }

        // Project columns
        if !query.columns.is_empty() && !query.columns.contains(&"*".to_string()) {
            rows = self.project_columns(rows, &query.columns)?;
        }

        debug!("✅ Selected {} rows", rows.len());
        Ok((rows, stats))
    }

    /// Update rows matching the given query
    ///
    /// Handles foreign key constraints:
    /// - Validates that new FK values exist in referenced tables
    /// - Applies ON UPDATE actions for tables referencing updated PK values
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table doesn't exist
    /// - Row validation fails
    /// - Foreign key constraint is violated
    #[instrument(level = "debug", skip(self, query), fields(table = %query.table))]
    pub async fn update_rows(&mut self, query: &UpdateQuery) -> Result<u64> {
        debug!("✏️ Updating rows in table: {}", query.table);

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

        // Clone schema for FK validation
        let schema = self
            .metadata
            .tables
            .get(&query.table)
            .ok_or_else(|| {
                CoreError::invalid_operation(&format!("Table '{}' schema not found", query.table))
            })?
            .clone();

        for mut row in existing_rows {
            let old_row = row.clone();

            // Apply updates
            for (field, new_value) in &query.set_values {
                row.fields.insert(field.clone(), new_value.clone());
            }
            row.updated_at = chrono::Utc::now();

            // Validate updated row
            self.validate_row(&schema, &row)?;

            // Validate foreign key constraints for the updated row
            self.validate_foreign_key_constraints(&schema, &row).await?;

            // Handle ON UPDATE actions for tables referencing this table's PK
            self.handle_update_foreign_key_constraints(&query.table, &old_row, &row)
                .await?;

            // Update compressed data
            let compressed_data = self.compress_row(&row).await?;
            self.compressed_blocks.insert(row.id, compressed_data);

            // Keep track of updated rows for file rewrite (need clone here)
            updated_rows.push(row.clone());

            // Log operation (consumes old_row, clones row for new_data)
            let operation = Operation::Update {
                table: query.table.clone(),
                row_id: row.id,
                old_data: old_row,
                new_data: row.clone(),
            };
            self.log_operation(operation).await?;

            // Update cache (moves row, so do this last)
            self.add_to_cache(row);

            updated_count += 1;
        }

        // Rewrite table file with updated data
        if updated_count > 0 {
            self.rewrite_table_file_with_updates(&query.table, &updated_rows)
                .await?;
        }

        debug!("✅ Updated {} rows", updated_count);
        Ok(updated_count)
    }

    /// Delete rows matching the given query
    ///
    /// Handles foreign key constraints according to their ON DELETE actions:
    /// - RESTRICT: Error if rows are referenced by other tables
    /// - CASCADE: Delete referencing rows in dependent tables
    /// - SET NULL: Set foreign key columns to NULL in referencing rows
    /// - SET DEFAULT: Set foreign key columns to default values
    /// - NO ACTION: Same as RESTRICT (checked at end of statement)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table doesn't exist
    /// - Foreign key constraint is violated (for RESTRICT/NO ACTION)
    #[instrument(level = "debug", skip(self, query), fields(table = %query.table))]
    pub async fn delete_rows(&mut self, query: &DeleteQuery) -> Result<u64> {
        debug!("🗑️ Deleting rows from table: {}", query.table);

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

        // Handle foreign key constraints for rows to be deleted
        for row in &rows_to_delete {
            self.handle_delete_foreign_key_constraints(&query.table, row)
                .await?;
        }

        for row in rows_to_delete {
            // Keep track of deleted row IDs
            deleted_row_ids.push(row.id);

            // Remove from compressed blocks
            self.compressed_blocks.remove(&row.id);

            // Remove from LRU cache
            self.row_cache.pop(&row.id);

            // Update indexes - clone schema to avoid borrow checker issues
            let schema = self
                .metadata
                .tables
                .get(&query.table)
                .ok_or_else(|| {
                    CoreError::invalid_operation(&format!(
                        "Table '{}' schema not found",
                        query.table
                    ))
                })?
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

        debug!("✅ Deleted {} rows", deleted_count);
        Ok(deleted_count as u64)
    }

    /// Internal delete method for CASCADE operations
    /// This is a separate method to avoid async recursion issues with the #[instrument] macro
    pub(crate) fn delete_rows_internal<'a>(
        &'a mut self,
        query: &'a DeleteQuery,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64>> + Send + 'a>> {
        Box::pin(async move {
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

            // Handle foreign key constraints for rows to be deleted
            for row in &rows_to_delete {
                self.handle_delete_foreign_key_constraints(&query.table, row)
                    .await?;
            }

            for row in rows_to_delete {
                // Keep track of deleted row IDs
                deleted_row_ids.push(row.id);

                // Remove from compressed blocks
                self.compressed_blocks.remove(&row.id);

                // Remove from LRU cache
                self.row_cache.pop(&row.id);

                // Update indexes - clone schema to avoid borrow checker issues
                let schema = self
                    .metadata
                    .tables
                    .get(&query.table)
                    .ok_or_else(|| {
                        CoreError::invalid_operation(&format!(
                            "Table '{}' schema not found",
                            query.table
                        ))
                    })?
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

            Ok(deleted_count as u64)
        })
    }

    /// Store data with a key (used by the main API)
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub async fn store(&mut self, key: &str, data: &[u8]) -> Result<()> {
        // Create a simple row structure for generic storage
        // Note: 'id' is not set here - it will be auto-generated by insert_row
        let mut fields = HashMap::new();
        fields.insert("key".to_string(), Value::text(key));
        fields.insert("data".to_string(), Value::binary(data));

        let row = Row {
            id: 0, // Will be set by auto-increment
            fields,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Ensure we have a generic storage table
        if !self.metadata.tables.contains_key("_storage") {
            use crate::storage::id_generation::IdGenerationStrategy;
            use crate::storage::types::ColumnDefinition;

            let schema = TableSchema {
                name: "_storage".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: DataType::BigSerial,
                        nullable: false,
                        default_value: None,
                        auto_increment: true,
                    },
                    ColumnDefinition {
                        name: "key".to_string(),
                        data_type: DataType::Text,
                        nullable: false,
                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "data".to_string(),
                        data_type: DataType::Binary,
                        nullable: false,
                        default_value: None,
                        auto_increment: false,
                    },
                ],
                primary_key: "id".to_string(),
                created_at: chrono::Utc::now(),
                version: 1,
                auto_increment_columns: HashMap::new(),
                id_strategy: IdGenerationStrategy::AutoIncrement,
                foreign_keys: Vec::new(),
            };
            self.create_table(schema).await?;
        }

        self.insert_row("_storage", row).await?;
        Ok(())
    }

    /// Retrieve data by key (used by the main API)
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Query for the key in the generic storage table
        let query = SelectQuery {
            table: "_storage".to_string(),
            columns: vec!["data".to_string()],
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: "key".to_string(),
                    operator: ComparisonOperator::Equal,
                    value: Value::text(key),
                }],
            }),
            order_by: None,
            limit: Some(1),
            offset: None,
        };

        let rows = self.select_rows(&query).await?;
        if let Some(row) = rows.first() {
            if let Some(Value::Binary(data)) = row.fields.get("data") {
                return Ok(Some(data.as_ref().clone()));
            }
        }

        Ok(None)
    }
}
