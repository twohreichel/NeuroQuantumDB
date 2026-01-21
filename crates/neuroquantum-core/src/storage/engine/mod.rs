//! Storage Engine implementation
//!
//! This module contains the `StorageEngine` struct and its implementation,
//! split across multiple submodules for maintainability:
//!
//! - `init`: Initialization and setup
//! - `schema`: DDL operations (CREATE/DROP/ALTER TABLE)
//! - `crud`: DML operations (INSERT/SELECT/UPDATE/DELETE)
//! - `transactions`: Simple transaction management (for backwards compatibility)
//! - `acid_transactions`: Full ACID transaction support with WAL
//! - `persistence`: Disk I/O operations
//! - `recovery`: Crash recovery
//! - `foreign_keys`: FK constraint handling
//! - `query_helpers`: Internal query processing utilities

mod acid_transactions;
mod crud;
mod foreign_keys;
mod init;
mod persistence;
mod query_helpers;
mod recovery;
mod schema;
mod transactions;

// Re-export transaction types for convenience
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use lru::LruCache;
pub use transactions::{BatchOperation, BatchResult};

use super::encryption::EncryptionManager;
use super::row::Row;
use super::stats::{DatabaseMetadata, QueryExecutionStats};
use super::transaction_log::Transaction;
use super::types::RowId;
use crate::dna::{EncodedData, QuantumDNACompressor};
use crate::transaction::TransactionManager;

/// Main storage engine providing persistent file-based storage
///
/// Note: `StorageEngine` is intentionally not Clone. Use `Arc<RwLock<StorageEngine>>`
/// for shared access across multiple tasks/threads. This prevents accidental
/// cloning of large internal data structures and ensures consistent cache state.
pub struct StorageEngine {
    /// Base directory for all database files
    pub(crate) data_dir: PathBuf,

    /// B+ Tree indexes for fast query performance
    pub(crate) indexes: HashMap<String, BTreeMap<String, RowId>>,

    /// Active transaction log for ACID compliance
    pub(crate) transaction_log: Vec<Transaction>,

    /// DNA-compressed data blocks for space efficiency
    pub(crate) compressed_blocks: HashMap<RowId, EncodedData>,

    /// Database metadata
    pub(crate) metadata: DatabaseMetadata,

    /// DNA compressor for data compression
    pub(crate) dna_compressor: QuantumDNACompressor,

    /// Next available row ID
    pub(crate) next_row_id: RowId,

    /// Next available LSN
    pub(crate) next_lsn: super::transaction_log::LSN,

    /// In-memory LRU cache for frequently accessed data
    /// Uses proper LRU eviction strategy for optimal memory management
    pub(crate) row_cache: LruCache<RowId, Row>,

    /// Transaction manager for ACID compliance
    pub(crate) transaction_manager: TransactionManager,

    /// Encryption manager for data-at-rest encryption
    pub(crate) encryption_manager: Option<EncryptionManager>,

    /// Query execution statistics for the last query
    pub(crate) last_query_stats: QueryExecutionStats,
}

impl StorageEngine {
    /// Get the last query execution statistics
    #[must_use]
    pub const fn get_last_query_stats(&self) -> &QueryExecutionStats {
        &self.last_query_stats
    }

    /// Get a reference to the transaction manager
    #[must_use]
    pub const fn get_transaction_manager(&self) -> &TransactionManager {
        &self.transaction_manager
    }

    /// Get the number of tables in the database
    #[must_use]
    pub fn get_table_count(&self) -> usize {
        self.metadata.tables.len()
    }

    /// Get the schema for a specific table
    ///
    /// Returns the table schema if it exists, or None if the table doesn't exist.
    /// This is useful for checking column definitions and default values during INSERT.
    #[must_use]
    pub fn get_table_schema(&self, table_name: &str) -> Option<&super::types::TableSchema> {
        self.metadata.tables.get(table_name)
    }

    /// Get a mutable reference to the schema for a specific table
    ///
    /// Returns the table schema if it exists, or None if the table doesn't exist.
    /// This is useful for modifying auto-increment configurations.
    pub fn get_table_schema_mut(
        &mut self,
        table_name: &str,
    ) -> Option<&mut super::types::TableSchema> {
        self.metadata.tables.get_mut(table_name)
    }
}
