//! Query types for the storage engine
//!
//! This module defines the query structures used for CRUD operations:
//! - `SelectQuery`: For reading data with filtering, sorting, and pagination
//! - `InsertQuery`: For inserting new rows
//! - `UpdateQuery`: For modifying existing rows
//! - `DeleteQuery`: For removing rows
//! - `AlterTableOp`: For DDL schema modifications

use std::collections::HashMap;

use super::types::{ColumnDefinition, DataType, Value};

/// Query for selecting rows from a table
#[derive(Debug, Clone)]
pub struct SelectQuery {
    pub table: String,
    pub columns: Vec<String>,
    pub where_clause: Option<WhereClause>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Query for inserting a new row
#[derive(Debug, Clone)]
pub struct InsertQuery {
    pub table: String,
    pub values: HashMap<String, Value>,
}

/// Query for updating existing rows
#[derive(Debug, Clone)]
pub struct UpdateQuery {
    pub table: String,
    pub set_values: HashMap<String, Value>,
    pub where_clause: Option<WhereClause>,
}

/// Query for deleting rows
#[derive(Debug, Clone)]
pub struct DeleteQuery {
    pub table: String,
    pub where_clause: Option<WhereClause>,
}

/// ALTER TABLE operations for schema modifications
#[derive(Debug, Clone)]
pub enum AlterTableOp {
    AddColumn {
        column: ColumnDefinition,
    },
    DropColumn {
        column_name: String,
    },
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    ModifyColumn {
        column_name: String,
        new_data_type: DataType,
    },
}

/// WHERE clause for filtering rows
#[derive(Debug, Clone)]
pub struct WhereClause {
    pub conditions: Vec<Condition>,
}

/// A single filter condition
#[derive(Debug, Clone)]
pub struct Condition {
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: Value,
}

/// Comparison operators for conditions
#[derive(Debug, Clone)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Like,
    In,
}

/// ORDER BY clause for sorting results
#[derive(Debug, Clone)]
pub struct OrderBy {
    pub field: String,
    pub direction: SortDirection,
}

/// Sort direction for ORDER BY
#[derive(Debug, Clone)]
pub enum SortDirection {
    Ascending,
    Descending,
}
