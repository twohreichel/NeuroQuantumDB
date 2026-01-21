//! Core storage types for `NeuroQuantumDB`
//!
//! This module contains the fundamental data types used throughout the storage engine:
//! - `Value`: Generic value type for database operations
//! - `DataType`: Supported column data types
//! - `TableSchema`: Table structure definition
//! - `ColumnDefinition`: Column metadata and constraints
//! - `ForeignKeyConstraint`: Referential integrity constraints

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::id_generation::{AutoIncrementConfig, IdGenerationStrategy};

/// Unique identifier for database rows
pub type RowId = u64;

/// Table schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub version: u32,
    /// Auto-increment configurations for columns (`column_name` -> config)
    #[serde(default)]
    pub auto_increment_columns: HashMap<String, AutoIncrementConfig>,
    /// ID generation strategy for internal row IDs
    #[serde(default)]
    pub id_strategy: IdGenerationStrategy,
    /// Foreign key constraints defined on this table
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKeyConstraint>,
}

impl TableSchema {
    /// Create a new table schema with minimal required fields
    ///
    /// # Example
    /// ```rust
    /// use neuroquantum_core::storage::{TableSchema, ColumnDefinition, DataType};
    ///
    /// let schema = TableSchema::new("users", "id", vec![
    ///     ColumnDefinition::new("id", DataType::BigSerial),
    ///     ColumnDefinition::new("name", DataType::Text),
    /// ]);
    /// ```
    pub fn new(
        name: impl Into<String>,
        primary_key: impl Into<String>,
        columns: Vec<ColumnDefinition>,
    ) -> Self {
        Self {
            name: name.into(),
            columns,
            primary_key: primary_key.into(),
            created_at: chrono::Utc::now(),
            version: 1,
            auto_increment_columns: HashMap::new(),
            id_strategy: IdGenerationStrategy::AutoIncrement,
            foreign_keys: Vec::new(),
        }
    }

    /// Set a custom ID generation strategy
    #[must_use]
    pub const fn with_id_strategy(mut self, strategy: IdGenerationStrategy) -> Self {
        self.id_strategy = strategy;
        self
    }
}

/// Foreign key constraint definition
/// Represents a relationship between tables where the foreign key column(s)
/// in this table reference primary/unique key column(s) in another table
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForeignKeyConstraint {
    /// Optional constraint name for identification and error messages
    pub name: Option<String>,
    /// Column(s) in this table that reference another table
    pub columns: Vec<String>,
    /// The table being referenced
    pub referenced_table: String,
    /// Column(s) in the referenced table
    pub referenced_columns: Vec<String>,
    /// Action to take when referenced row is deleted
    pub on_delete: ReferentialAction,
    /// Action to take when referenced row is updated
    pub on_update: ReferentialAction,
}

/// Referential action for foreign key constraints
/// Specifies what action to take when the referenced row is updated or deleted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReferentialAction {
    /// Reject the delete or update operation (default)
    /// Raises an error if any referencing rows exist
    #[default]
    Restrict,
    /// Automatically delete or update the referencing rows
    /// Propagates the operation to dependent tables
    Cascade,
    /// Set the foreign key column(s) to NULL
    /// Requires the column(s) to be nullable
    SetNull,
    /// Set the foreign key column(s) to their default values
    /// Requires the column(s) to have default values defined
    SetDefault,
    /// Similar to RESTRICT but checked at end of transaction
    /// Allows deferred constraint checking
    NoAction,
}

impl std::fmt::Display for ReferentialAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::Restrict => write!(f, "RESTRICT"),
            | Self::Cascade => write!(f, "CASCADE"),
            | Self::SetNull => write!(f, "SET NULL"),
            | Self::SetDefault => write!(f, "SET DEFAULT"),
            | Self::NoAction => write!(f, "NO ACTION"),
        }
    }
}

/// Column definition in table schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<Value>,
    /// Whether this column auto-increments
    #[serde(default)]
    pub auto_increment: bool,
}

impl ColumnDefinition {
    /// Create a new column definition with minimal required fields
    ///
    /// # Example
    /// ```rust
    /// use neuroquantum_core::storage::{ColumnDefinition, DataType};
    ///
    /// let col = ColumnDefinition::new("name", DataType::Text);
    /// assert!(!col.nullable);
    /// assert!(!col.auto_increment);
    /// ```
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        let name = name.into();
        let auto_increment = matches!(data_type, DataType::Serial | DataType::BigSerial);
        Self {
            name,
            data_type,
            nullable: false,
            default_value: None,
            auto_increment,
        }
    }

    /// Set the column as nullable
    #[must_use]
    pub const fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    /// Set a default value for the column
    #[must_use]
    pub fn with_default(mut self, value: Value) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Explicitly set auto-increment behavior
    #[must_use]
    pub const fn auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self
    }
}

/// Supported data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Float,
    Text,
    Boolean,
    Timestamp,
    Binary,
    /// Auto-incrementing integer (SERIAL in `PostgreSQL`)
    Serial,
    /// Auto-incrementing big integer (BIGSERIAL in `PostgreSQL`)
    BigSerial,
}

/// Generic value type for database operations
///
/// Uses `Arc` for `Text` and `Binary` variants to enable cheap cloning (reference counting only).
/// This significantly reduces memory allocations and improves performance for JOIN operations
/// and CTE (Common Table Expression) execution where rows are frequently cloned.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    /// Text value with shared ownership for cheap cloning
    #[serde(with = "arc_string_serde")]
    Text(Arc<String>),
    Boolean(bool),
    Timestamp(chrono::DateTime<chrono::Utc>),
    /// Binary data with shared ownership for cheap cloning
    #[serde(with = "arc_vec_serde")]
    Binary(Arc<Vec<u8>>),
    Null,
}

/// Custom Clone implementation that leverages Arc's cheap reference counting
impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            | Self::Integer(i) => Self::Integer(*i),
            | Self::Float(f) => Self::Float(*f),
            | Self::Text(s) => Self::Text(Arc::clone(s)),
            | Self::Boolean(b) => Self::Boolean(*b),
            | Self::Timestamp(ts) => Self::Timestamp(*ts),
            | Self::Binary(b) => Self::Binary(Arc::clone(b)),
            | Self::Null => Self::Null,
        }
    }
}

/// Serde helper module for `Arc<String>`
mod arc_string_serde {
    use std::sync::Arc;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Arc<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.as_str().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Arc::new(s))
    }
}

/// Serde helper module for `Arc<Vec<u8>>`
mod arc_vec_serde {
    use std::sync::Arc;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Arc<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Vec::<u8>::deserialize(deserializer)?;
        Ok(Arc::new(v))
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::Integer(i) => write!(f, "{i}"),
            | Self::Float(fl) => write!(f, "{fl}"),
            | Self::Text(s) => write!(f, "{s}"),
            | Self::Boolean(b) => write!(f, "{b}"),
            | Self::Timestamp(ts) => write!(f, "{}", ts.to_rfc3339()),
            | Self::Binary(b) => write!(f, "Binary[{} bytes]", b.len()),
            | Self::Null => write!(f, "NULL"),
        }
    }
}

impl Value {
    /// Create a new Text value from a string
    ///
    /// # Example
    /// ```
    /// use neuroquantum_core::storage::Value;
    /// let text = Value::text("Hello, World!");
    /// ```
    #[inline]
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(Arc::new(s.into()))
    }

    /// Create a new Binary value from bytes
    ///
    /// # Example
    /// ```
    /// use neuroquantum_core::storage::Value;
    /// let binary = Value::binary(vec![0x01, 0x02, 0x03]);
    /// ```
    #[inline]
    pub fn binary(data: impl Into<Vec<u8>>) -> Self {
        Self::Binary(Arc::new(data.into()))
    }

    /// Get the inner text as a reference if this is a Text value
    #[inline]
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            | Self::Text(s) => Some(s.as_str()),
            | _ => None,
        }
    }

    /// Get the inner binary data as a reference if this is a Binary value
    #[inline]
    #[must_use]
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            | Self::Binary(b) => Some(b.as_slice()),
            | _ => None,
        }
    }
}
