//! DDL (Schema) operations for `StorageEngine`
//!
//! This module implements CREATE TABLE, DROP TABLE, and ALTER TABLE operations.

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

use super::StorageEngine;
use crate::storage::id_generation::AutoIncrementConfig;
use crate::storage::query::AlterTableOp;
use crate::storage::row::CompressedRowEntry;
use crate::storage::transaction_log::Operation;
use crate::storage::types::{DataType, TableSchema, Value};

impl StorageEngine {
    /// Create a new table with given schema
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table already exists
    /// - Schema validation fails
    /// - File creation fails
    pub async fn create_table(&mut self, schema: TableSchema) -> Result<()> {
        let table_name = schema.name.clone();
        info!("🔨 Creating table: {}", table_name);

        // Check if table already exists
        if self.metadata.tables.contains_key(&schema.name) {
            return Err(anyhow!("Table '{}' already exists", schema.name));
        }

        // Validate schema
        self.validate_schema(&schema)?;

        // Create table file
        let table_path = self
            .data_dir
            .join("tables")
            .join(format!("{}.nqdb", schema.name));
        fs::File::create(&table_path).await?;

        // Create primary key index
        let index_path = self
            .data_dir
            .join("indexes")
            .join(format!("{}_{}.idx", schema.name, schema.primary_key));
        fs::File::create(&index_path).await?;

        // Add to metadata
        let mut schema_to_store = schema.clone();

        // Initialize auto_increment_columns for columns with auto_increment: true or Serial/BigSerial types
        for column in &schema_to_store.columns {
            if column.auto_increment
                || matches!(column.data_type, DataType::Serial | DataType::BigSerial)
            {
                schema_to_store
                    .auto_increment_columns
                    .insert(column.name.clone(), AutoIncrementConfig::new(&column.name));
            }
        }

        self.metadata
            .tables
            .insert(schema_to_store.name.clone(), schema_to_store.clone());

        // Create index in memory
        self.indexes.insert(
            format!("{}_{}", schema.name, schema.primary_key),
            BTreeMap::new(),
        );

        // Log operation
        let operation = Operation::CreateTable { schema };
        self.log_operation(operation).await?;

        // Save metadata
        self.save_metadata().await?;

        info!("✅ Table '{}' created successfully", table_name);
        Ok(())
    }

    /// Drop a table and all its associated data
    ///
    /// This method removes:
    /// - The table schema from metadata
    /// - The table data file (.nqdb)
    /// - All associated index files
    /// - All compressed blocks for rows in the table
    ///
    /// # Arguments
    /// * `table_name` - The name of the table to drop
    /// * `if_exists` - If true, don't return an error if the table doesn't exist
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table doesn't exist (and `if_exists` is false)
    /// - File deletion fails
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> anyhow::Result<()> {
    /// # use neuroquantum_core::storage::StorageEngine;
    /// # let mut storage = StorageEngine::new("./data").await?;
    /// // Drop a table
    /// storage.drop_table("users", false).await?;
    ///
    /// // Drop a table if it exists (won't error if table doesn't exist)
    /// storage.drop_table("maybe_exists", true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn drop_table(&mut self, table_name: &str, if_exists: bool) -> Result<()> {
        info!("🗑️ Dropping table: {}", table_name);

        // Check if table exists
        let schema = match self.metadata.tables.get(table_name) {
            | Some(schema) => schema.clone(),
            | None => {
                if if_exists {
                    debug!(
                        "Table '{}' does not exist, but IF EXISTS specified",
                        table_name
                    );
                    return Ok(());
                }
                return Err(anyhow!("Table '{table_name}' does not exist"));
            },
        };

        // Get all row IDs from this table before removing
        // We need to load the table rows to know which compressed blocks to remove
        let table_rows = match self.load_table_rows(table_name).await {
            | Ok(rows) => rows,
            | Err(e) => {
                debug!(
                    "Warning: Could not load table rows for cleanup during DROP TABLE: {}. Proceeding with schema removal.",
                    e
                );
                Vec::new()
            },
        };
        let row_ids: Vec<_> = table_rows.iter().map(|r| r.id).collect();

        // Remove compressed blocks for all rows in the table
        for row_id in &row_ids {
            self.compressed_blocks.remove(row_id);
            self.row_cache.pop(row_id);
        }

        // Remove all indexes for this table
        let index_keys_to_remove: Vec<String> = self
            .indexes
            .keys()
            .filter(|key| key.starts_with(&format!("{table_name}_")))
            .cloned()
            .collect();

        for key in &index_keys_to_remove {
            self.indexes.remove(key);
        }

        // Delete table data file
        let table_path = self
            .data_dir
            .join("tables")
            .join(format!("{table_name}.nqdb"));
        if table_path.exists() {
            fs::remove_file(&table_path).await.map_err(|e| {
                anyhow!(
                    "Failed to delete table file '{}': {}",
                    table_path.display(),
                    e
                )
            })?;
            debug!("Deleted table file: {}", table_path.display());
        }

        // Delete index files
        let indexes_dir = self.data_dir.join("indexes");
        for index_key in &index_keys_to_remove {
            let index_path = indexes_dir.join(format!("{index_key}.idx"));
            if index_path.exists() {
                fs::remove_file(&index_path).await.map_err(|e| {
                    anyhow!(
                        "Failed to delete index file '{}': {}",
                        index_path.display(),
                        e
                    )
                })?;
                debug!("Deleted index file: {}", index_path.display());
            }
        }

        // Also delete primary key index file (might have different naming)
        let pk_index_path = indexes_dir.join(format!("{}_{}.idx", table_name, schema.primary_key));
        if pk_index_path.exists() {
            if let Err(e) = fs::remove_file(&pk_index_path).await {
                debug!(
                    "Warning: Could not delete primary key index file '{}': {}",
                    pk_index_path.display(),
                    e
                );
            } else {
                debug!(
                    "Deleted primary key index file: {}",
                    pk_index_path.display()
                );
            }
        }

        // Remove table from metadata
        self.metadata.tables.remove(table_name);

        // Log the DROP TABLE operation
        let operation = Operation::DropTable {
            table: table_name.to_string(),
        };
        self.log_operation(operation).await?;

        // Save updated metadata
        self.save_metadata().await?;

        // Save updated compressed blocks
        self.save_compressed_blocks().await?;

        info!(
            "✅ Table '{}' dropped successfully ({} rows removed)",
            table_name,
            row_ids.len()
        );
        Ok(())
    }

    /// Alter an existing table structure
    ///
    /// Supports:
    /// - ADD COLUMN: Add a new column with optional default value
    /// - DROP COLUMN: Remove a column from the table
    /// - RENAME COLUMN: Rename an existing column
    /// - MODIFY COLUMN: Change the data type of a column
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table doesn't exist
    /// - Column already exists (for ADD)
    /// - Column doesn't exist (for DROP/RENAME/MODIFY)
    /// - Attempting to drop primary key
    /// - Data conversion fails (for MODIFY)
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> anyhow::Result<()> {
    /// # use neuroquantum_core::storage::{StorageEngine, AlterTableOp, ColumnDefinition, DataType};
    /// # let mut storage = StorageEngine::new("./data").await?;
    /// // Add a new column
    /// storage.alter_table("users", AlterTableOp::AddColumn {
    ///     column: ColumnDefinition::new("email", DataType::Text),
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn alter_table(&mut self, table_name: &str, operation: AlterTableOp) -> Result<()> {
        info!("🔧 Altering table: {} ({:?})", table_name, operation);

        // Get current schema
        let old_schema = self
            .metadata
            .tables
            .get(table_name)
            .ok_or_else(|| anyhow!("Table '{table_name}' does not exist"))?
            .clone();

        // Create new schema based on operation
        let mut new_schema = old_schema.clone();

        match &operation {
            | AlterTableOp::AddColumn { column } => {
                // Check if column already exists
                if new_schema.columns.iter().any(|c| c.name == column.name) {
                    return Err(anyhow!("Column '{}' already exists", column.name));
                }

                // Add the new column
                new_schema.columns.push(column.clone());

                // If it's auto-increment, add to auto_increment_columns
                if column.auto_increment
                    || matches!(column.data_type, DataType::Serial | DataType::BigSerial)
                {
                    new_schema
                        .auto_increment_columns
                        .insert(column.name.clone(), AutoIncrementConfig::new(&column.name));
                }

                // Update all existing rows with default value or NULL
                let default_value = column.default_value.clone().unwrap_or(Value::Null);

                // Collect row IDs to update (avoid borrow issues)
                let row_ids: Vec<_> = self.compressed_blocks.keys().copied().collect();

                for row_id in row_ids {
                    if let Some(encoded_data) = self.compressed_blocks.get(&row_id) {
                        // Decompress the row
                        let mut row_data = self.decompress_row(encoded_data).await?;

                        // Add default value for new column
                        row_data
                            .fields
                            .insert(column.name.clone(), default_value.clone());
                        row_data.updated_at = chrono::Utc::now();

                        // Recompress and store
                        let compressed = self.compress_row(&row_data).await?;
                        self.compressed_blocks.insert(row_id, compressed);

                        // Update cache if present
                        if self.row_cache.contains(&row_id) {
                            self.add_to_cache(row_data);
                        }
                    }
                }
            },
            | AlterTableOp::DropColumn { column_name } => {
                // Check if column exists
                let column_index = new_schema
                    .columns
                    .iter()
                    .position(|c| c.name == *column_name)
                    .ok_or_else(|| anyhow!("Column '{column_name}' does not exist"))?;

                // Prevent dropping primary key
                if *column_name == new_schema.primary_key {
                    return Err(anyhow!("Cannot drop primary key column '{column_name}'"));
                }

                // Remove the column from schema
                new_schema.columns.remove(column_index);

                // Remove from auto_increment_columns if present
                new_schema.auto_increment_columns.remove(column_name);

                // Remove column data from all existing rows
                let row_ids: Vec<_> = self.compressed_blocks.keys().copied().collect();

                for row_id in row_ids {
                    if let Some(encoded_data) = self.compressed_blocks.get(&row_id) {
                        let mut row_data = self.decompress_row(encoded_data).await?;

                        // Remove the column
                        row_data.fields.remove(column_name);
                        row_data.updated_at = chrono::Utc::now();

                        // Recompress and store
                        let compressed = self.compress_row(&row_data).await?;
                        self.compressed_blocks.insert(row_id, compressed);

                        // Update cache if present
                        if self.row_cache.contains(&row_id) {
                            self.add_to_cache(row_data);
                        }
                    }
                }
            },
            | AlterTableOp::RenameColumn { old_name, new_name } => {
                // Check if old column exists
                let column_index = new_schema
                    .columns
                    .iter()
                    .position(|c| c.name == *old_name)
                    .ok_or_else(|| anyhow!("Column '{old_name}' does not exist"))?;

                // Check if new name is not already used
                if new_schema.columns.iter().any(|c| c.name == *new_name) {
                    return Err(anyhow!("Column '{new_name}' already exists"));
                }

                // Update schema
                new_schema.columns[column_index].name = new_name.clone();

                // Update primary key if renamed
                if new_schema.primary_key == *old_name {
                    new_schema.primary_key = new_name.clone();
                }

                // Update auto_increment_columns if present
                if let Some(config) = new_schema.auto_increment_columns.remove(old_name) {
                    let mut new_config = config;
                    new_config.column_name = new_name.clone();
                    new_schema
                        .auto_increment_columns
                        .insert(new_name.clone(), new_config);
                }

                // Rename column in all existing rows
                let row_ids: Vec<_> = self.compressed_blocks.keys().copied().collect();

                for row_id in row_ids {
                    if let Some(encoded_data) = self.compressed_blocks.get(&row_id) {
                        let mut row_data = self.decompress_row(encoded_data).await?;

                        // Rename the field
                        if let Some(value) = row_data.fields.remove(old_name) {
                            row_data.fields.insert(new_name.clone(), value);
                        }
                        row_data.updated_at = chrono::Utc::now();

                        // Recompress and store
                        let compressed = self.compress_row(&row_data).await?;
                        self.compressed_blocks.insert(row_id, compressed);

                        // Update cache if present
                        if self.row_cache.contains(&row_id) {
                            self.add_to_cache(row_data);
                        }
                    }
                }
            },
            | AlterTableOp::ModifyColumn {
                column_name,
                new_data_type,
            } => {
                // Check if column exists
                let column_index = new_schema
                    .columns
                    .iter()
                    .position(|c| c.name == *column_name)
                    .ok_or_else(|| anyhow!("Column '{column_name}' does not exist"))?;

                let old_data_type = new_schema.columns[column_index].data_type.clone();

                // Update column data type
                new_schema.columns[column_index].data_type = new_data_type.clone();

                // Convert data in all existing rows
                let row_ids: Vec<_> = self.compressed_blocks.keys().copied().collect();

                for row_id in row_ids {
                    if let Some(encoded_data) = self.compressed_blocks.get(&row_id) {
                        let mut row_data = self.decompress_row(encoded_data).await?;

                        // Try to convert the value to new data type
                        if let Some(old_value) = row_data.fields.get(column_name) {
                            let new_value =
                                self.convert_value(old_value, &old_data_type, new_data_type)?;
                            row_data.fields.insert(column_name.clone(), new_value);
                        }
                        row_data.updated_at = chrono::Utc::now();

                        // Recompress and store
                        let compressed = self.compress_row(&row_data).await?;
                        self.compressed_blocks.insert(row_id, compressed);

                        // Update cache if present
                        if self.row_cache.contains(&row_id) {
                            self.add_to_cache(row_data);
                        }
                    }
                }
            },
        }

        // Update schema version
        new_schema.version += 1;

        // Log the operation for WAL
        let operation_log = Operation::AlterTable {
            table: table_name.to_string(),
            old_schema: old_schema.clone(),
            new_schema: new_schema.clone(),
        };
        self.log_operation(operation_log).await?;

        // Update metadata with new schema
        self.metadata
            .tables
            .insert(table_name.to_string(), new_schema);

        // Save metadata
        self.save_metadata().await?;

        // Rewrite the table file with updated rows
        self.rewrite_table_file(table_name).await?;

        info!("✅ Table '{}' altered successfully", table_name);
        Ok(())
    }

    /// Reset auto-increment counters for a table
    ///
    /// This is typically called during TRUNCATE TABLE with RESTART IDENTITY.
    /// It resets all identity/serial columns back to their initial values.
    ///
    /// # Arguments
    /// * `table_name` - Name of the table to reset
    ///
    /// # Errors
    ///
    /// Returns an error if the table doesn't exist.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> anyhow::Result<()> {
    /// # use neuroquantum_core::storage::StorageEngine;
    /// # let mut storage = StorageEngine::new("./data").await?;
    /// // Reset auto-increment counters after TRUNCATE
    /// storage.reset_auto_increment("users").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reset_auto_increment(&mut self, table_name: &str) -> Result<()> {
        debug!(
            "🔄 Resetting auto-increment counters for table: {}",
            table_name
        );

        // Get table schema
        let schema = self
            .metadata
            .tables
            .get_mut(table_name)
            .ok_or_else(|| anyhow!("Table '{table_name}' does not exist"))?;

        // Reset all auto-increment column counters
        let mut reset_count = 0;
        for config in schema.auto_increment_columns.values_mut() {
            let old_value = config.next_value;
            config.next_value = config.min_value;
            debug!(
                "Reset auto-increment for column '{}': {} -> {}",
                config.column_name, old_value, config.next_value
            );
            reset_count += 1;
        }

        if reset_count > 0 {
            // Save updated metadata
            self.save_metadata().await?;
            info!(
                "✅ Reset {} auto-increment counter(s) for table '{}'",
                reset_count, table_name
            );
        } else {
            debug!(
                "Table '{}' has no auto-increment columns to reset",
                table_name
            );
        }

        Ok(())
    }

    /// Rewrite the entire table file with current data from `compressed_blocks`
    pub(crate) async fn rewrite_table_file(&mut self, table_name: &str) -> Result<()> {
        let table_path = self
            .data_dir
            .join("tables")
            .join(format!("{table_name}.nqdb"));

        // Delete existing file and create new one
        if table_path.exists() {
            fs::remove_file(&table_path).await?;
        }

        // Create file and write all rows with proper binary format
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&table_path)
            .await?;

        // Get all rows for this table (sorted by ID for consistency)
        let mut row_ids: Vec<_> = self.compressed_blocks.keys().copied().collect();
        row_ids.sort_unstable();

        for row_id in row_ids {
            if let Some(encoded_data) = self.compressed_blocks.get(&row_id) {
                // Decompress to get the actual row
                let row = self.decompress_row(encoded_data).await?;

                // Create compressed entry
                let entry = CompressedRowEntry {
                    row_id,
                    compressed_data: encoded_data.clone(),
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    encrypted_wrapper: None,
                    format_version: 1,
                };

                // Serialize the entry
                let serialized = bincode::serialize(&entry)?;

                // Write length prefix (4 bytes)
                let len = serialized.len() as u32;
                file.write_all(&len.to_le_bytes()).await?;

                // Write the serialized entry
                file.write_all(&serialized).await?;
            }
        }

        file.flush().await?;
        Ok(())
    }
}
