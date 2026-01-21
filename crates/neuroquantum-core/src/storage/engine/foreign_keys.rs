//! Foreign key constraint handling for `StorageEngine`
//!
//! This module implements foreign key validation and referential actions
//! (ON UPDATE/ON DELETE CASCADE, SET NULL, etc.).

use anyhow::{anyhow, Result};

use super::StorageEngine;
use crate::storage::query::{ComparisonOperator, Condition, DeleteQuery, SelectQuery, WhereClause};
use crate::storage::row::Row;
use crate::storage::types::{ForeignKeyConstraint, ReferentialAction, TableSchema, Value};

impl StorageEngine {
    /// Validate foreign key constraints for an INSERT operation
    ///
    /// Checks that all foreign key values reference existing rows in the referenced tables.
    ///
    /// # Errors
    ///
    /// Returns an error if any foreign key constraint is violated.
    pub(crate) async fn validate_foreign_key_constraints(
        &self,
        schema: &TableSchema,
        row: &Row,
    ) -> Result<()> {
        for fk in &schema.foreign_keys {
            // For each foreign key constraint, check if the referenced value exists
            for (i, fk_column) in fk.columns.iter().enumerate() {
                if let Some(fk_value) = row.fields.get(fk_column) {
                    // Skip NULL values (they don't need to reference anything)
                    if *fk_value == Value::Null {
                        continue;
                    }

                    // Get the referenced column name
                    let ref_column = fk
                        .referenced_columns
                        .get(i)
                        .ok_or_else(|| anyhow!("Foreign key column count mismatch"))?;

                    // Check if the referenced table exists
                    let ref_schema =
                        self.metadata
                            .tables
                            .get(&fk.referenced_table)
                            .ok_or_else(|| {
                                anyhow!(
                                    "Foreign key references non-existent table '{}'",
                                    fk.referenced_table
                                )
                            })?;

                    // Check if the referenced value exists
                    let exists = self
                        .check_value_exists(&ref_schema.name, ref_column, fk_value)
                        .await?;

                    if !exists {
                        let constraint_name = fk
                            .name
                            .as_ref()
                            .map(|n| format!(" (constraint '{n}')"))
                            .unwrap_or_default();
                        return Err(anyhow!(
                            "Foreign key violation{}: value {} in column '{}' does not exist in {}.{}",
                            constraint_name,
                            fk_value,
                            fk_column,
                            fk.referenced_table,
                            ref_column
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a specific value exists in a table column
    ///
    /// Used for foreign key constraint validation.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub(crate) async fn check_value_exists(
        &self,
        table: &str,
        column: &str,
        value: &Value,
    ) -> Result<bool> {
        let query = SelectQuery {
            table: table.to_string(),
            columns: vec![column.to_string()],
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: column.to_string(),
                    operator: ComparisonOperator::Equal,
                    value: value.clone(),
                }],
            }),
            order_by: None,
            limit: Some(1),
            offset: None,
        };

        let rows = self.select_rows(&query).await?;
        Ok(!rows.is_empty())
    }

    /// Handle foreign key constraints when updating a row (ON UPDATE actions)
    ///
    /// # Errors
    ///
    /// Returns an error if constraint handling fails.
    pub(crate) async fn handle_update_foreign_key_constraints(
        &mut self,
        table_name: &str,
        old_row: &Row,
        new_row: &Row,
    ) -> Result<()> {
        // Get the schema of the table being updated
        let table_schema = self
            .metadata
            .tables
            .get(table_name)
            .ok_or_else(|| anyhow!("Table '{table_name}' not found"))?
            .clone();

        // Check if any primary key or unique key columns are being changed
        let pk_changed = old_row.fields.get(&table_schema.primary_key)
            != new_row.fields.get(&table_schema.primary_key);

        if !pk_changed {
            // No PK change, no need to check ON UPDATE actions
            return Ok(());
        }

        // Find all tables that reference this table
        let referencing_tables: Vec<_> = self
            .metadata
            .tables
            .iter()
            .filter_map(|(name, schema)| {
                let referencing_fks: Vec<_> = schema
                    .foreign_keys
                    .iter()
                    .filter(|fk| fk.referenced_table == table_name)
                    .cloned()
                    .collect();

                if referencing_fks.is_empty() {
                    None
                } else {
                    Some((name.clone(), referencing_fks))
                }
            })
            .collect();

        for (ref_table_name, foreign_keys) in referencing_tables {
            for fk in foreign_keys {
                self.apply_update_action(&ref_table_name, &fk, &table_schema, old_row, new_row)
                    .await?;
            }
        }

        Ok(())
    }

    /// Apply the ON UPDATE action for a foreign key constraint
    ///
    /// # Errors
    ///
    /// Returns an error if the action cannot be applied.
    pub(crate) async fn apply_update_action(
        &mut self,
        referencing_table: &str,
        fk: &ForeignKeyConstraint,
        referenced_schema: &TableSchema,
        old_row: &Row,
        new_row: &Row,
    ) -> Result<()> {
        // Build the WHERE clause to find referencing rows
        let mut conditions = Vec::new();
        for (i, ref_column) in fk.referenced_columns.iter().enumerate() {
            if let Some(old_value) = old_row.fields.get(ref_column) {
                let fk_column = fk
                    .columns
                    .get(i)
                    .ok_or_else(|| anyhow!("Foreign key column count mismatch"))?;
                conditions.push(Condition {
                    field: fk_column.clone(),
                    operator: ComparisonOperator::Equal,
                    value: old_value.clone(),
                });
            }
        }

        if conditions.is_empty() {
            return Ok(());
        }

        // Find referencing rows
        let select_query = SelectQuery {
            table: referencing_table.to_string(),
            columns: vec!["*".to_string()],
            where_clause: Some(WhereClause { conditions }),
            order_by: None,
            limit: None,
            offset: None,
        };

        let referencing_rows = self.select_rows(&select_query).await?;

        if referencing_rows.is_empty() {
            return Ok(());
        }

        // Apply the appropriate action
        match fk.on_update {
            | ReferentialAction::Restrict | ReferentialAction::NoAction => {
                let constraint_name = fk
                    .name
                    .as_ref()
                    .map(|n| format!(" (constraint '{n}')"))
                    .unwrap_or_default();
                return Err(anyhow!(
                    "Foreign key violation{}: cannot update primary key in '{}' because {} row(s) in '{}' reference this row",
                    constraint_name,
                    referenced_schema.name,
                    referencing_rows.len(),
                    referencing_table
                ));
            },
            | ReferentialAction::Cascade => {
                // Update the foreign key columns in referencing rows to match the new PK
                for ref_row in &referencing_rows {
                    let mut updated_fields = ref_row.fields.clone();
                    for (i, ref_column) in fk.referenced_columns.iter().enumerate() {
                        if let Some(new_value) = new_row.fields.get(ref_column) {
                            let fk_column = fk
                                .columns
                                .get(i)
                                .ok_or_else(|| anyhow!("Foreign key column count mismatch"))?;
                            updated_fields.insert(fk_column.clone(), new_value.clone());
                        }
                    }

                    let updated_row = Row {
                        id: ref_row.id,
                        fields: updated_fields,
                        created_at: ref_row.created_at,
                        updated_at: chrono::Utc::now(),
                    };

                    self.update_row_internal(referencing_table, &updated_row)
                        .await?;
                }
            },
            | ReferentialAction::SetNull => {
                // Set the foreign key columns to NULL
                for ref_row in &referencing_rows {
                    let mut updated_fields = ref_row.fields.clone();
                    for fk_column in &fk.columns {
                        updated_fields.insert(fk_column.clone(), Value::Null);
                    }

                    let updated_row = Row {
                        id: ref_row.id,
                        fields: updated_fields,
                        created_at: ref_row.created_at,
                        updated_at: chrono::Utc::now(),
                    };

                    self.update_row_internal(referencing_table, &updated_row)
                        .await?;
                }
            },
            | ReferentialAction::SetDefault => {
                // Set the foreign key columns to their default values
                let ref_schema = self
                    .metadata
                    .tables
                    .get(referencing_table)
                    .ok_or_else(|| anyhow!("Table '{referencing_table}' not found"))?
                    .clone();

                for ref_row in &referencing_rows {
                    let mut updated_fields = ref_row.fields.clone();
                    for fk_column in &fk.columns {
                        let default_value = ref_schema
                            .columns
                            .iter()
                            .find(|c| c.name == *fk_column)
                            .and_then(|c| c.default_value.clone())
                            .unwrap_or(Value::Null);

                        updated_fields.insert(fk_column.clone(), default_value);
                    }

                    let updated_row = Row {
                        id: ref_row.id,
                        fields: updated_fields,
                        created_at: ref_row.created_at,
                        updated_at: chrono::Utc::now(),
                    };

                    self.update_row_internal(referencing_table, &updated_row)
                        .await?;
                }
            },
        }

        Ok(())
    }

    /// Handle foreign key constraints when deleting a row
    ///
    /// Finds all tables that reference this table and applies the ON DELETE action.
    ///
    /// # Errors
    ///
    /// Returns an error if constraint handling fails.
    pub(crate) async fn handle_delete_foreign_key_constraints(
        &mut self,
        table_name: &str,
        row: &Row,
    ) -> Result<()> {
        // Get the schema of the table being deleted from
        let table_schema = self
            .metadata
            .tables
            .get(table_name)
            .ok_or_else(|| anyhow!("Table '{table_name}' not found"))?
            .clone();

        // Find all tables that reference this table
        let referencing_tables: Vec<_> = self
            .metadata
            .tables
            .iter()
            .filter_map(|(name, schema)| {
                let referencing_fks: Vec<_> = schema
                    .foreign_keys
                    .iter()
                    .filter(|fk| fk.referenced_table == table_name)
                    .cloned()
                    .collect();

                if referencing_fks.is_empty() {
                    None
                } else {
                    Some((name.clone(), referencing_fks))
                }
            })
            .collect();

        for (ref_table_name, foreign_keys) in referencing_tables {
            for fk in foreign_keys {
                // For each foreign key that references this table, apply the ON DELETE action
                self.apply_delete_action(&ref_table_name, &fk, &table_schema, row)
                    .await?;
            }
        }

        Ok(())
    }

    /// Apply the ON DELETE action for a foreign key constraint
    ///
    /// # Errors
    ///
    /// Returns an error if the action cannot be applied.
    pub(crate) async fn apply_delete_action(
        &mut self,
        referencing_table: &str,
        fk: &ForeignKeyConstraint,
        referenced_schema: &TableSchema,
        deleted_row: &Row,
    ) -> Result<()> {
        // Build the WHERE clause to find referencing rows
        let mut conditions = Vec::new();
        for (i, ref_column) in fk.referenced_columns.iter().enumerate() {
            if let Some(deleted_value) = deleted_row.fields.get(ref_column) {
                let fk_column = fk
                    .columns
                    .get(i)
                    .ok_or_else(|| anyhow!("Foreign key column count mismatch"))?;
                conditions.push(Condition {
                    field: fk_column.clone(),
                    operator: ComparisonOperator::Equal,
                    value: deleted_value.clone(),
                });
            }
        }

        if conditions.is_empty() {
            return Ok(()); // No matching columns to check
        }

        // Find referencing rows
        let select_query = SelectQuery {
            table: referencing_table.to_string(),
            columns: vec!["*".to_string()],
            where_clause: Some(WhereClause { conditions }),
            order_by: None,
            limit: None,
            offset: None,
        };

        let referencing_rows = self.select_rows(&select_query).await?;

        if referencing_rows.is_empty() {
            return Ok(()); // No referencing rows, nothing to do
        }

        // Apply the appropriate action
        match fk.on_delete {
            | ReferentialAction::Restrict | ReferentialAction::NoAction => {
                let constraint_name = fk
                    .name
                    .as_ref()
                    .map(|n| format!(" (constraint '{n}')"))
                    .unwrap_or_default();
                return Err(anyhow!(
                    "Foreign key violation{}: cannot delete from '{}' because {} row(s) in '{}' reference this row",
                    constraint_name,
                    referenced_schema.name,
                    referencing_rows.len(),
                    referencing_table
                ));
            },
            | ReferentialAction::Cascade => {
                // Delete all referencing rows (this may trigger further cascades)
                let mut cascade_conditions = Vec::new();
                for (i, ref_column) in fk.referenced_columns.iter().enumerate() {
                    if let Some(deleted_value) = deleted_row.fields.get(ref_column) {
                        let fk_column = fk
                            .columns
                            .get(i)
                            .ok_or_else(|| anyhow!("Foreign key column count mismatch"))?;
                        cascade_conditions.push(Condition {
                            field: fk_column.clone(),
                            operator: ComparisonOperator::Equal,
                            value: deleted_value.clone(),
                        });
                    }
                }

                let cascade_delete = DeleteQuery {
                    table: referencing_table.to_string(),
                    where_clause: Some(WhereClause {
                        conditions: cascade_conditions,
                    }),
                };

                // Recursive delete (handles further cascades)
                // Use Box::pin for async recursion
                Box::pin(self.delete_rows_internal(&cascade_delete)).await?;
            },
            | ReferentialAction::SetNull => {
                // Set the foreign key columns to NULL in referencing rows
                for ref_row in &referencing_rows {
                    let mut updated_fields = ref_row.fields.clone();
                    for fk_column in &fk.columns {
                        updated_fields.insert(fk_column.clone(), Value::Null);
                    }

                    let updated_row = Row {
                        id: ref_row.id,
                        fields: updated_fields,
                        created_at: ref_row.created_at,
                        updated_at: chrono::Utc::now(),
                    };

                    self.update_row_internal(referencing_table, &updated_row)
                        .await?;
                }
            },
            | ReferentialAction::SetDefault => {
                // Set the foreign key columns to their default values
                let ref_schema = self
                    .metadata
                    .tables
                    .get(referencing_table)
                    .ok_or_else(|| anyhow!("Table '{referencing_table}' not found"))?
                    .clone();

                for ref_row in &referencing_rows {
                    let mut updated_fields = ref_row.fields.clone();
                    for fk_column in &fk.columns {
                        // Find the default value for this column
                        let default_value = ref_schema
                            .columns
                            .iter()
                            .find(|c| c.name == *fk_column)
                            .and_then(|c| c.default_value.clone())
                            .unwrap_or(Value::Null);

                        updated_fields.insert(fk_column.clone(), default_value);
                    }

                    let updated_row = Row {
                        id: ref_row.id,
                        fields: updated_fields,
                        created_at: ref_row.created_at,
                        updated_at: chrono::Utc::now(),
                    };

                    self.update_row_internal(referencing_table, &updated_row)
                        .await?;
                }
            },
        }

        Ok(())
    }

    /// Internal method to update a row (used by FK constraint handling)
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub(crate) async fn update_row_internal(&mut self, table: &str, row: &Row) -> Result<()> {
        // Compress and store the updated row
        let compressed_data = self.compress_row(row).await?;
        self.compressed_blocks.insert(row.id, compressed_data);

        // Update cache
        self.row_cache.put(row.id, row.clone());

        // Rewrite the row in the table file
        // For simplicity, we'll reload all rows and rewrite
        let file_path = self.data_dir.join("tables").join(format!("{table}.dat"));
        if file_path.exists() {
            let content = tokio::fs::read_to_string(&file_path).await?;
            let mut rows: Vec<Row> = if content.trim().is_empty() {
                Vec::new()
            } else {
                serde_json::from_str(&content)?
            };

            // Find and update the row
            if let Some(existing) = rows.iter_mut().find(|r| r.id == row.id) {
                *existing = row.clone();
            }

            // Write back
            let content = serde_json::to_string_pretty(&rows)?;
            tokio::fs::write(&file_path, content).await?;
        }

        Ok(())
    }
}
