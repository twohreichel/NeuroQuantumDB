//! `NeuroQuantumDB` Storage Engine
//!
//! Provides persistent file-based storage with DNA compression, B+ tree indexes,
//! and ACID transaction support for production deployment.
//!
//! # Architecture
//!
//! The storage engine is organized into several submodules:
//!
//! - [`engine`]: Core storage engine implementation
//! - [`types`]: Core type definitions (Value, `DataType`, `TableSchema`, etc.)
//! - [`row`]: Row representation and compressed storage format
//! - [`query`]: Query types for SELECT, INSERT, UPDATE, DELETE
//! - [`id_generation`]: ID generation strategies (`AutoIncrement`, UUID, Snowflake)
//! - [`transaction_log`]: Transaction and operation logging
//! - [`stats`]: Query execution statistics and database metadata
//! - [`encryption`]: Data-at-rest encryption
//! - [`backup`]: Backup and restore functionality
//! - [`btree`]: B+ tree index implementation
//! - [`buffer`]: Buffer pool management
//! - [`pager`]: Page-based storage management
//! - [`wal`]: Write-ahead logging
//! - [`migration`]: Schema migration support
//!
//! # Example
//!
//! ```ignore
//! use neuroquantum_core::storage::{StorageEngine, TableSchema, Row, Value, DataType};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create storage engine
//! let mut storage = StorageEngine::new("./data").await?;
//!
//! // Create a table
//! let schema = TableSchema::new("users", "id", vec![
//!     ColumnDefinition::new("id", DataType::BigSerial),
//!     ColumnDefinition::new("name", DataType::Text),
//! ]);
//! storage.create_table(schema).await?;
//!
//! // Insert a row
//! let mut row = Row::new();
//! row.set("name", Value::text("Alice"));
//! let id = storage.insert_row("users", row).await?;
//! # Ok(())
//! # }
//! ```

// Submodules
pub mod backup;
pub mod btree;
pub mod buffer;
pub mod encryption;
pub mod engine;
pub mod id_generation;
pub mod migration;
pub mod pager;
pub mod query;
pub mod row;
pub mod stats;
pub mod test_helpers;
pub mod transaction_log;
pub mod types;
pub mod wal;

// Re-exports from submodules for convenient access

// Core types
// Backup and restore
pub use backup::{
    BackupConfig, BackupManager, BackupMetadata, BackupStats, BackupStorageBackend,
    BackupStorageType, BackupType, IncrementalBackup, LocalBackend, RestoreManager, RestoreOptions,
    RestoreStats, S3Backend, S3Config,
};
// B+ tree
pub use btree::{BTree, BTreeConfig};
// Buffer pool
pub use buffer::{BufferPoolConfig, BufferPoolManager, BufferPoolStats, EvictionPolicyType};
// Encryption
pub use encryption::{EncryptedData, EncryptionManager};
// Storage engine
pub use engine::{BatchOperation, BatchResult, StorageEngine};
// ID generation
pub use id_generation::{AutoIncrementConfig, IdGenerationStrategy};
// Migration
pub use migration::{
    BoxedSqlExecutor, Migration, MigrationConfig, MigrationDirection, MigrationExecutor,
    MigrationExecutorConfig, MigrationHistory, MigrationParser, MigrationProgress, MigrationRecord,
    MigrationResult, MigrationStatus, ProgressTracker, SafetyCheck, SqlExecutionResult,
    SqlExecutor, ValidationResult,
};
// Pager
pub use pager::{PageStorageManager, PagerConfig, StorageStats, SyncMode};
// Query types
pub use query::{
    AlterTableOp, ComparisonOperator, Condition, DeleteQuery, InsertQuery, OrderBy, SelectQuery,
    SortDirection, UpdateQuery, WhereClause,
};
// Row types
pub use row::Row;
// Statistics and metadata
pub use stats::{DatabaseMetadata, QueryExecutionStats};
// Compressed row entry is pub(crate) for internal use only

// Test helpers (available in all builds for integration tests)
pub use test_helpers::{create_test_row, create_test_schema};
// Transaction log types
pub use transaction_log::{Operation, Transaction, TransactionId, TransactionStatus, LSN};
pub use types::{
    ColumnDefinition, DataType, ForeignKeyConstraint, ReferentialAction, RowId, TableSchema, Value,
};
// WAL
pub use wal::{RecoveryStats, WALConfig, WALManager};
