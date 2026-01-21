//! Test helper functions for the storage module
//!
//! This module provides utility functions for creating test data.

use std::collections::HashMap;

use super::id_generation::IdGenerationStrategy;
use super::row::Row;
use super::types::{ColumnDefinition, DataType, RowId, TableSchema, Value};

/// Create a basic table schema for testing
#[must_use]
pub fn create_test_schema(name: &str) -> TableSchema {
    TableSchema {
        name: name.to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::BigSerial,
                nullable: false,
                default_value: None,
                auto_increment: true,
            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "created_at".to_string(),
                data_type: DataType::Timestamp,
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
    }
}

/// Create a test row with given id and name
#[must_use]
pub fn create_test_row(id: i64, name: &str) -> Row {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Integer(id));
    fields.insert("name".to_string(), Value::text(name));
    fields.insert(
        "created_at".to_string(),
        Value::Timestamp(chrono::Utc::now()),
    );

    Row {
        id: id as RowId,
        fields,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}
