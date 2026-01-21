//! Query helper methods for `StorageEngine`
//!
//! Internal utilities for query processing, including filtering, sorting,
//! compression, validation, and caching.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tracing::debug;

use super::StorageEngine;
use crate::dna::{DNACompressor, EncodedData};
use crate::storage::query::{ComparisonOperator, OrderBy, SortDirection, WhereClause};
use crate::storage::row::Row;
use crate::storage::types::{DataType, TableSchema, Value};

impl StorageEngine {
    /// Validate table schema
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Table name is empty
    /// - No columns are defined
    /// - Primary key column doesn't exist
    pub(crate) fn validate_schema(&self, schema: &TableSchema) -> Result<()> {
        if schema.name.is_empty() {
            return Err(anyhow!("Table name cannot be empty"));
        }

        if schema.columns.is_empty() {
            return Err(anyhow!("Table must have at least one column"));
        }

        // Check if primary key exists in columns
        let pk_exists = schema
            .columns
            .iter()
            .any(|col| col.name == schema.primary_key);
        if !pk_exists {
            return Err(anyhow!(
                "Primary key '{}' not found in columns",
                schema.primary_key
            ));
        }

        Ok(())
    }

    /// Validate row against schema
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Column type doesn't match the value type
    /// - Required column is missing
    pub(crate) fn validate_row(&self, schema: &TableSchema, row: &Row) -> Result<()> {
        for column in &schema.columns {
            if let Some(value) = row.fields.get(&column.name) {
                // Type validation
                let valid_type = match (&column.data_type, value) {
                    | (DataType::Integer, Value::Integer(_)) => true,
                    | (DataType::Float, Value::Float(_)) => true,
                    | (DataType::Text, Value::Text(_)) => true,
                    | (DataType::Boolean, Value::Boolean(_)) => true,
                    | (DataType::Timestamp, Value::Timestamp(_)) => true,
                    | (DataType::Binary, Value::Binary(_)) => true,
                    // Serial types store as Integer values
                    | (DataType::BigSerial, Value::Integer(_)) => true,
                    | (DataType::Serial, Value::Integer(_)) => true,
                    | (_, Value::Null) => column.nullable,
                    | _ => false,
                };

                if !valid_type {
                    return Err(anyhow!(
                        "Type mismatch for column '{}': expected {:?}, got {:?}",
                        column.name,
                        column.data_type,
                        value
                    ));
                }
            } else if !column.nullable
                && column.default_value.is_none()
                && !column.auto_increment
                && !matches!(column.data_type, DataType::Serial | DataType::BigSerial)
            {
                return Err(anyhow!("Required column '{}' is missing", column.name));
            }
        }

        Ok(())
    }

    /// Compress row data using DNA compression
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or compression fails.
    pub(crate) async fn compress_row(&mut self, row: &Row) -> Result<EncodedData> {
        // Use bincode for efficient binary serialization (more compatible with HashMap)
        let serialized =
            bincode::serialize(row).map_err(|e| anyhow!("Failed to serialize row: {e}"))?;
        let compressed = self.dna_compressor.compress(&serialized).await?;
        Ok(compressed)
    }

    /// Decompress row data from DNA compression
    ///
    /// This method provides async decompression of DNA-compressed row data,
    /// supporting both modern bincode and legacy JSON formats for backwards
    /// compatibility with older data files.
    ///
    /// # Errors
    ///
    /// Returns an error if decompression or deserialization fails.
    pub(crate) async fn decompress_row(&self, encoded: &EncodedData) -> Result<Row> {
        let decompressed = self.dna_compressor.decompress(encoded).await?;

        // Try bincode first (modern format), fall back to JSON (legacy format)
        if let Ok(row) = bincode::deserialize::<Row>(&decompressed) {
            return Ok(row);
        }

        // Fall back to JSON for legacy compatibility
        serde_json::from_slice::<Row>(&decompressed)
            .map_err(|e| anyhow!("Failed to deserialize row with both bincode and JSON: {e}"))
    }

    /// Update indexes for inserted row
    ///
    /// # Errors
    ///
    /// Returns Ok(()) on success (currently infallible).
    pub(crate) fn update_indexes_for_insert(
        &mut self,
        schema: &TableSchema,
        row: &Row,
    ) -> Result<()> {
        // Update primary key index
        let pk_index_name = format!("{}_{}", schema.name, schema.primary_key);
        if let Some(pk_value) = row.fields.get(&schema.primary_key) {
            let pk_string = self.value_to_string(pk_value);
            if let Some(index) = self.indexes.get_mut(&pk_index_name) {
                index.insert(pk_string, row.id);
            }
        }

        Ok(())
    }

    /// Update indexes for deleted row
    ///
    /// # Errors
    ///
    /// Returns Ok(()) on success (currently infallible).
    pub(crate) fn update_indexes_for_delete(
        &mut self,
        schema: &TableSchema,
        row: &Row,
    ) -> Result<()> {
        // Remove from primary key index
        let pk_index_name = format!("{}_{}", schema.name, schema.primary_key);
        if let Some(pk_value) = row.fields.get(&schema.primary_key) {
            let pk_string = self.value_to_string(pk_value);
            if let Some(index) = self.indexes.get_mut(&pk_index_name) {
                index.remove(&pk_string);
            }
        }

        Ok(())
    }

    /// Convert value to string for indexing
    pub(crate) fn value_to_string(&self, value: &Value) -> String {
        match value {
            | Value::Integer(i) => i.to_string(),
            | Value::Float(f) => f.to_string(),
            | Value::Text(s) => s.as_ref().clone(),
            | Value::Boolean(b) => b.to_string(),
            | Value::Timestamp(ts) => ts.to_rfc3339(),
            | Value::Binary(b) => {
                use base64::engine::general_purpose;
                use base64::Engine as _;
                general_purpose::STANDARD.encode(b.as_ref())
            },
            | Value::Null => "NULL".to_string(),
        }
    }

    /// Add row to cache with LRU eviction
    ///
    /// Uses proper LRU (Least Recently Used) eviction strategy:
    /// - When cache is full, the least recently accessed entry is automatically evicted
    /// - Each access (get/put) moves the entry to "most recently used" position
    /// - Provides O(1) amortized time complexity for all operations
    pub(crate) fn add_to_cache(&mut self, row: Row) {
        // LruCache handles eviction automatically when capacity is exceeded
        self.row_cache.put(row.id, row);
    }

    /// Apply WHERE clause to filter rows
    ///
    /// # Errors
    ///
    /// Returns an error if condition evaluation fails.
    pub(crate) fn apply_where_clause(
        &self,
        rows: Vec<Row>,
        where_clause: &WhereClause,
    ) -> Result<Vec<Row>> {
        let mut filtered_rows = Vec::new();

        for row in rows {
            let mut matches = true;

            for condition in &where_clause.conditions {
                if let Some(field_value) = row.fields.get(&condition.field) {
                    let condition_matches = self.evaluate_condition(
                        field_value,
                        &condition.operator,
                        &condition.value,
                    )?;
                    if !condition_matches {
                        matches = false;
                        break;
                    }
                } else {
                    matches = false;
                    break;
                }
            }

            if matches {
                filtered_rows.push(row);
            }
        }

        Ok(filtered_rows)
    }

    /// Evaluate a single condition
    ///
    /// # Errors
    ///
    /// Returns an error if value comparison fails (e.g., incompatible types).
    pub(crate) fn evaluate_condition(
        &self,
        field_value: &Value,
        operator: &ComparisonOperator,
        condition_value: &Value,
    ) -> Result<bool> {
        match operator {
            | ComparisonOperator::Equal => Ok(field_value == condition_value),
            | ComparisonOperator::NotEqual => Ok(field_value != condition_value),
            | ComparisonOperator::LessThan => self
                .compare_values(field_value, condition_value)
                .map(Ordering::is_lt),
            | ComparisonOperator::LessThanOrEqual => self
                .compare_values(field_value, condition_value)
                .map(Ordering::is_le),
            | ComparisonOperator::GreaterThan => self
                .compare_values(field_value, condition_value)
                .map(Ordering::is_gt),
            | ComparisonOperator::GreaterThanOrEqual => self
                .compare_values(field_value, condition_value)
                .map(Ordering::is_ge),
            | ComparisonOperator::Like => {
                if let (Value::Text(field_text), Value::Text(pattern)) =
                    (field_value, condition_value)
                {
                    Ok(field_text.contains(pattern.as_str()))
                } else {
                    Ok(false)
                }
            },
            | ComparisonOperator::In => {
                // For simplicity, treating this as equality for now
                Ok(field_value == condition_value)
            },
        }
    }

    /// Compare two values for ordering
    ///
    /// # Errors
    ///
    /// Returns an error if values have incompatible types.
    pub(crate) fn compare_values(&self, a: &Value, b: &Value) -> Result<Ordering> {
        match (a, b) {
            | (Value::Integer(a), Value::Integer(b)) => Ok(a.cmp(b)),
            | (Value::Float(a), Value::Float(b)) => Ok(a.partial_cmp(b).unwrap_or(Ordering::Equal)),
            | (Value::Text(a), Value::Text(b)) => Ok(a.cmp(b)),
            | (Value::Boolean(a), Value::Boolean(b)) => Ok(a.cmp(b)),
            | (Value::Timestamp(a), Value::Timestamp(b)) => Ok(a.cmp(b)),
            | _ => Err(anyhow!("Cannot compare values of different types")),
        }
    }

    /// Apply ORDER BY to sort rows
    ///
    /// # Errors
    ///
    /// Returns Ok(()) on success (sorting is done in-place).
    pub(crate) fn apply_order_by(&self, rows: &mut [Row], order_by: &OrderBy) -> Result<()> {
        rows.sort_by(|a, b| {
            let a_value = a.fields.get(&order_by.field);
            let b_value = b.fields.get(&order_by.field);

            match (a_value, b_value) {
                | (Some(a_val), Some(b_val)) => {
                    let cmp = self.compare_values(a_val, b_val).unwrap_or(Ordering::Equal);
                    match order_by.direction {
                        | SortDirection::Ascending => cmp,
                        | SortDirection::Descending => cmp.reverse(),
                    }
                },
                | (Some(_), None) => Ordering::Less,
                | (None, Some(_)) => Ordering::Greater,
                | (None, None) => Ordering::Equal,
            }
        });

        Ok(())
    }

    /// Project specific columns from rows
    ///
    /// # Errors
    ///
    /// Returns Ok on success (currently infallible).
    pub(crate) fn project_columns(&self, rows: Vec<Row>, columns: &[String]) -> Result<Vec<Row>> {
        let mut projected_rows = Vec::new();

        for mut row in rows {
            let mut new_fields = HashMap::new();

            for column in columns {
                if let Some(value) = row.fields.remove(column) {
                    new_fields.insert(column.clone(), value);
                }
            }

            row.fields = new_fields;
            projected_rows.push(row);
        }

        Ok(projected_rows)
    }

    /// Convert a value from one data type to another
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion is not supported or fails.
    pub(crate) fn convert_value(
        &self,
        value: &Value,
        _old_type: &DataType,
        new_type: &DataType,
    ) -> Result<Value> {
        match (value, new_type) {
            // NULL remains NULL for any type
            | (Value::Null, _) => Ok(Value::Null),

            // Integer conversions
            | (Value::Integer(i), DataType::Integer | DataType::Serial | DataType::BigSerial) => {
                Ok(Value::Integer(*i))
            },
            | (Value::Integer(i), DataType::Float) => Ok(Value::Float(*i as f64)),
            | (Value::Integer(i), DataType::Text) => Ok(Value::text(i.to_string())),
            | (Value::Integer(i), DataType::Boolean) => Ok(Value::Boolean(*i != 0)),

            // Float conversions
            | (Value::Float(f), DataType::Float) => Ok(Value::Float(*f)),
            | (Value::Float(f), DataType::Integer | DataType::Serial | DataType::BigSerial) => {
                Ok(Value::Integer(*f as i64))
            },
            | (Value::Float(f), DataType::Text) => Ok(Value::text(f.to_string())),

            // Text conversions
            | (Value::Text(s), DataType::Text) => Ok(Value::Text(s.clone())),
            | (Value::Text(s), DataType::Integer | DataType::Serial | DataType::BigSerial) => {
                let parsed = s
                    .parse::<i64>()
                    .map_err(|e| anyhow!("Cannot convert '{s}' to Integer: {e}"))?;
                // Validate integer bounds based on type
                match new_type {
                    | DataType::Integer | DataType::Serial => {
                        if parsed < i64::from(i32::MIN) || parsed > i64::from(i32::MAX) {
                            return Err(anyhow!(
                                "Value {} is out of range for Integer (must be between {} and {})",
                                parsed,
                                i32::MIN,
                                i32::MAX
                            ));
                        }
                    },
                    | DataType::BigSerial => {
                        // BigSerial uses i64, no additional bounds check needed
                    },
                    | _ => {},
                }
                Ok(Value::Integer(parsed))
            },
            | (Value::Text(s), DataType::Float) => s
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|e| anyhow!("Cannot convert '{s}' to Float: {e}")),
            | (Value::Text(s), DataType::Boolean) => match s.to_lowercase().as_str() {
                | "true" | "t" | "yes" | "y" | "1" => Ok(Value::Boolean(true)),
                | "false" | "f" | "no" | "n" | "0" => Ok(Value::Boolean(false)),
                | _ => Err(anyhow!("Cannot convert '{s}' to Boolean")),
            },

            // Boolean conversions
            | (Value::Boolean(b), DataType::Boolean) => Ok(Value::Boolean(*b)),
            | (Value::Boolean(b), DataType::Integer | DataType::Serial | DataType::BigSerial) => {
                Ok(Value::Integer(i64::from(*b)))
            },
            | (Value::Boolean(b), DataType::Text) => Ok(Value::text(b.to_string())),

            // Timestamp conversions
            | (Value::Timestamp(ts), DataType::Timestamp) => Ok(Value::Timestamp(*ts)),
            | (Value::Timestamp(ts), DataType::Text) => Ok(Value::text(ts.to_rfc3339())),

            // Binary conversions
            | (Value::Binary(b), DataType::Binary) => Ok(Value::Binary(Arc::clone(b))),
            | (Value::Binary(b), DataType::Text) => {
                Ok(Value::text(format!("Binary[{} bytes]", b.len())))
            },

            // Catch-all for unsupported conversions
            | _ => Err(anyhow!("Cannot convert {value:?} to {new_type:?}")),
        }
    }

    /// Populate auto-increment columns with generated values
    ///
    /// This handles columns marked as:
    /// - `AUTO_INCREMENT` (MySQL-style)
    /// - SERIAL/BIGSERIAL (PostgreSQL-style)
    /// - GENERATED AS IDENTITY (SQL standard)
    ///
    /// # Errors
    ///
    /// Returns an error if auto-increment value generation fails.
    pub(crate) fn populate_auto_increment_columns(
        &mut self,
        table: &str,
        schema: &TableSchema,
        row: &mut Row,
    ) -> Result<()> {
        for column in &schema.columns {
            // Skip if value is already provided and is not NULL
            if let Some(value) = row.fields.get(&column.name) {
                if *value != Value::Null {
                    continue;
                }
            }

            // Check if this column needs auto-increment
            let needs_auto_increment = column.auto_increment
                || matches!(column.data_type, DataType::Serial | DataType::BigSerial);

            if needs_auto_increment {
                // Get or create auto-increment config for this column
                let next_value = if let Some(config) = self
                    .metadata
                    .tables
                    .get_mut(table)
                    .and_then(|s| s.auto_increment_columns.get_mut(&column.name))
                {
                    config.next_id()?
                } else {
                    // First time: initialize auto-increment for this column
                    use crate::storage::id_generation::AutoIncrementConfig;
                    let mut config = AutoIncrementConfig::new(&column.name);
                    let value = config.next_id()?;

                    // Store config in schema
                    if let Some(schema) = self.metadata.tables.get_mut(table) {
                        schema
                            .auto_increment_columns
                            .insert(column.name.clone(), config);
                    }
                    value
                };

                // Set the auto-generated value
                row.fields
                    .insert(column.name.clone(), Value::Integer(next_value));

                debug!(
                    "🔢 Auto-generated value {} for column '{}.{}'",
                    next_value, table, column.name
                );
            }
        }

        Ok(())
    }

    /// Populate missing columns with their default values
    ///
    /// This function fills in any columns that are missing from the row
    /// with their defined default values from the schema. This is called
    /// during INSERT to ensure all columns have appropriate values.
    pub(crate) fn populate_default_values(schema: &TableSchema, row: &mut Row) {
        for column in &schema.columns {
            // Skip if value is already provided
            if row.fields.contains_key(&column.name) {
                continue;
            }

            // Apply default value if available
            if let Some(default_value) = &column.default_value {
                row.fields
                    .insert(column.name.clone(), default_value.clone());
                debug!(
                    "📝 Applied default value {:?} for column '{}'",
                    default_value, column.name
                );
            }
        }
    }
}
