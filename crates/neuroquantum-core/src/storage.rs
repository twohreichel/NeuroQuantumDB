//! NeuroQuantumDB Storage Engine
//! Provides persistent file-based storage with DNA compression, B+ tree indexes,
//! and ACID transaction support for production deployment

pub mod backup;
pub mod btree;
pub mod buffer;
pub mod encryption;
pub mod pager;
pub mod wal;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};
use uuid::Uuid;

pub use backup::{
    BackupConfig, BackupManager, BackupMetadata, BackupStats, BackupStorageBackend,
    BackupStorageType, BackupType, IncrementalBackup, LocalBackend, RestoreManager, RestoreOptions,
    RestoreStats, S3Backend, S3Config,
};
pub use btree::{BTree, BTreeConfig};
pub use buffer::{BufferPoolConfig, BufferPoolManager, BufferPoolStats, EvictionPolicyType};
pub use encryption::{EncryptedData, EncryptionManager};
pub use pager::{PageStorageManager, PagerConfig, StorageStats, SyncMode};
pub use wal::{RecoveryStats, WALConfig, WALManager};

use crate::dna::{DNACompressor, EncodedData, QuantumDNACompressor};
use crate::transaction::{IsolationLevel, TransactionManager};

/// Unique identifier for database rows
pub type RowId = u64;

/// Compressed row entry for binary storage format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompressedRowEntry {
    row_id: RowId,
    compressed_data: EncodedData,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    /// Encrypted wrapper for additional security (optional)
    encrypted_wrapper: Option<EncryptedData>,
    /// Format version for backward compatibility
    format_version: u32,
}

/// Transaction identifier
pub type TransactionId = Uuid;

/// Log Sequence Number for write-ahead logging
pub type LSN = u64;

/// Table schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub version: u32,
}

/// Column definition in table schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<Value>,
}

/// Supported data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataType {
    Integer,
    Float,
    Text,
    Boolean,
    Timestamp,
    Binary,
}

/// Generic value type for database operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Timestamp(chrono::DateTime<chrono::Utc>),
    Binary(Vec<u8>),
    Null,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Text(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Timestamp(ts) => write!(f, "{}", ts.to_rfc3339()),
            Value::Binary(b) => write!(f, "Binary[{} bytes]", b.len()),
            Value::Null => write!(f, "NULL"),
        }
    }
}

/// Database row containing field values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub id: RowId,
    pub fields: HashMap<String, Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Query operations for storage engine
#[derive(Debug, Clone)]
pub struct SelectQuery {
    pub table: String,
    pub columns: Vec<String>,
    pub where_clause: Option<WhereClause>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct InsertQuery {
    pub table: String,
    pub values: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct UpdateQuery {
    pub table: String,
    pub set_values: HashMap<String, Value>,
    pub where_clause: Option<WhereClause>,
}

#[derive(Debug, Clone)]
pub struct DeleteQuery {
    pub table: String,
    pub where_clause: Option<WhereClause>,
}

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: Value,
}

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

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Transaction log entry for ACID compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub operations: Vec<Operation>,
    pub status: TransactionStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub lsn: LSN,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Insert {
        table: String,
        row_id: RowId,
        data: Row,
    },
    Update {
        table: String,
        row_id: RowId,
        old_data: Row,
        new_data: Row,
    },
    Delete {
        table: String,
        row_id: RowId,
        data: Row,
    },
    CreateTable {
        schema: TableSchema,
    },
    DropTable {
        table: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Active,
    Committed,
    Aborted,
}

/// Database metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetadata {
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_backup: Option<chrono::DateTime<chrono::Utc>>,
    pub tables: HashMap<String, TableSchema>,
    pub next_row_id: RowId,
    pub next_lsn: LSN,
}

/// Main storage engine providing persistent file-based storage
#[derive(Clone)]
pub struct StorageEngine {
    /// Base directory for all database files
    data_dir: PathBuf,

    /// B+ Tree indexes for fast query performance
    indexes: HashMap<String, BTreeMap<String, RowId>>,

    /// Active transaction log for ACID compliance
    transaction_log: Vec<Transaction>,

    /// DNA-compressed data blocks for space efficiency
    compressed_blocks: HashMap<RowId, EncodedData>,

    /// Database metadata
    metadata: DatabaseMetadata,

    /// DNA compressor for data compression
    dna_compressor: QuantumDNACompressor,

    /// Next available row ID
    next_row_id: RowId,

    /// Next available LSN
    next_lsn: LSN,

    /// In-memory cache for frequently accessed data
    row_cache: HashMap<RowId, Row>,

    /// Cache size limit
    cache_limit: usize,

    /// Transaction manager for ACID compliance
    transaction_manager: TransactionManager,

    /// Encryption manager for data-at-rest encryption
    encryption_manager: Option<EncryptionManager>,
}

impl StorageEngine {
    /// Create a placeholder storage engine for two-phase initialization
    ///
    /// This is used for synchronous construction of NeuroQuantumDB,
    /// which is then properly initialized with async `init()` method.
    ///
    /// **Important:** This should NOT be used directly for production.
    /// Always follow with proper async initialization via `new()`.
    ///
    /// # Example
    /// ```no_run
    /// let mut db = NeuroQuantumDB::new();
    /// db.init().await?; // Proper async initialization
    /// ```
    #[doc(hidden)] // Hide from public API docs
    pub fn new_placeholder(data_dir: &std::path::Path) -> Self {
        let metadata = DatabaseMetadata {
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now(),
            last_backup: None,
            tables: HashMap::new(),
            next_row_id: 1,
            next_lsn: 1,
        };

        Self {
            data_dir: data_dir.to_path_buf(),
            indexes: HashMap::new(),
            transaction_log: Vec::new(),
            compressed_blocks: HashMap::new(),
            metadata,
            dna_compressor: QuantumDNACompressor::new(),
            next_row_id: 1,
            next_lsn: 1,
            row_cache: HashMap::new(),
            cache_limit: 10000,
            transaction_manager: TransactionManager::new(),
            encryption_manager: None,
        }
    }

    /// Create new storage engine instance
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();

        info!("🗄️ Initializing StorageEngine at: {}", data_dir.display());

        // Create directory structure
        Self::create_directory_structure(&data_dir).await?;

        // Initialize DNA compressor
        let dna_compressor = QuantumDNACompressor::new();

        // Load existing metadata or create new
        let metadata = Self::load_or_create_metadata(&data_dir).await?;

        // Initialize transaction manager with real log manager
        let log_dir = data_dir.join("logs");
        let transaction_manager = crate::transaction::TransactionManager::new_async(&log_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize transaction manager: {}", e))?;

        // Initialize encryption manager for data-at-rest encryption
        let encryption_manager = EncryptionManager::new(&data_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize encryption manager: {}", e))?;

        info!(
            "🔐 Encryption-at-rest enabled with key fingerprint: {}",
            encryption_manager.get_key_fingerprint()
        );

        let mut engine = Self {
            data_dir: data_dir.clone(),
            indexes: HashMap::new(),
            transaction_log: Vec::new(),
            compressed_blocks: HashMap::new(),
            metadata,
            dna_compressor,
            next_row_id: 1,
            next_lsn: 1,
            row_cache: HashMap::new(),
            cache_limit: 10000, // 10k rows in cache
            transaction_manager,
            encryption_manager: Some(encryption_manager),
        };

        // Load existing data
        engine.load_from_disk().await?;

        Ok(engine)
    }

    /// Create the required directory structure
    async fn create_directory_structure(data_dir: &Path) -> Result<()> {
        let dirs = [
            data_dir,
            &data_dir.join("tables"),
            &data_dir.join("indexes"),
            &data_dir.join("logs"),
            &data_dir.join("quantum"),
        ];

        for dir in &dirs {
            if !dir.exists() {
                fs::create_dir_all(dir).await.map_err(|e| {
                    anyhow!(
                        "Failed to create directory '{}': {} (Error code: {})",
                        dir.display(),
                        e,
                        e.raw_os_error().unwrap_or(-1)
                    )
                })?;
                info!("📁 Created directory: {}", dir.display());
            } else {
                debug!("📁 Directory already exists: {}", dir.display());
            }
        }

        Ok(())
    }

    /// Load existing metadata or create new
    async fn load_or_create_metadata(data_dir: &Path) -> Result<DatabaseMetadata> {
        let metadata_path = data_dir.join("metadata.json");

        if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path).await?;
            let metadata: DatabaseMetadata = serde_json::from_str(&content)?;
            info!("📋 Loaded existing metadata");
            Ok(metadata)
        } else {
            let metadata = DatabaseMetadata {
                version: "1.0.0".to_string(),
                created_at: chrono::Utc::now(),
                last_backup: None,
                tables: HashMap::new(),
                next_row_id: 1,
                next_lsn: 1,
            };

            // Save metadata
            let content = serde_json::to_string_pretty(&metadata)?;
            fs::write(&metadata_path, content).await?;

            info!("📋 Created new metadata");
            Ok(metadata)
        }
    }

    /// Create a new table with given schema
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
        self.metadata
            .tables
            .insert(schema.name.clone(), schema.clone());

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

    /// Insert a new row into the specified table
    pub async fn insert_row(&mut self, table: &str, mut row: Row) -> Result<RowId> {
        debug!("➕ Inserting row into table: {}", table);

        // Get table schema
        let schema = self
            .metadata
            .tables
            .get(table)
            .ok_or_else(|| anyhow!("Table '{}' does not exist", table))?
            .clone();

        // Assign row ID
        row.id = self.next_row_id;
        self.next_row_id += 1;

        // Validate row against schema
        self.validate_row(&schema, &row)?;

        // Compress row data using DNA compression
        let compressed_data = self.compress_row(&row).await?;

        // Store compressed data
        self.compressed_blocks.insert(row.id, compressed_data);

        // Update indexes
        self.update_indexes_for_insert(&schema, &row)?;

        // Add to cache
        self.add_to_cache(row.clone());

        // Log operation
        let operation = Operation::Insert {
            table: table.to_string(),
            row_id: row.id,
            data: row.clone(),
        };
        self.log_operation(operation).await?;

        // Append to table file with DNA compression
        self.append_row_to_file(table, &row).await?;

        debug!("✅ Row inserted with ID: {} (DNA compressed)", row.id);
        Ok(row.id)
    }

    /// Select rows matching the given query
    pub async fn select_rows(&self, query: &SelectQuery) -> Result<Vec<Row>> {
        debug!("🔍 Selecting rows from table: {}", query.table);

        // Get table schema - unused but kept for future optimization
        let _schema = self
            .metadata
            .tables
            .get(&query.table)
            .ok_or_else(|| anyhow!("Table '{}' does not exist", query.table))?;

        // Load all rows for the table (in a real implementation, this would be optimized)
        let mut rows = self.load_table_rows(&query.table).await?;

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
        Ok(rows)
    }

    /// Update rows matching the given query
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

        for mut row in existing_rows {
            let old_row = row.clone();

            // Apply updates
            for (field, new_value) in &query.set_values {
                row.fields.insert(field.clone(), new_value.clone());
            }
            row.updated_at = chrono::Utc::now();

            // Validate updated row
            let schema = self.metadata.tables.get(&query.table).unwrap();
            self.validate_row(schema, &row)?;

            // Update compressed data
            let compressed_data = self.compress_row(&row).await?;
            self.compressed_blocks.insert(row.id, compressed_data);

            // Update cache
            self.add_to_cache(row.clone());

            // Keep track of updated rows for file rewrite
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

        debug!("✅ Updated {} rows", updated_count);
        Ok(updated_count)
    }

    /// Delete rows matching the given query
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

        for row in rows_to_delete {
            // Keep track of deleted row IDs
            deleted_row_ids.push(row.id);

            // Remove from compressed blocks
            self.compressed_blocks.remove(&row.id);

            // Remove from cache
            self.row_cache.remove(&row.id);

            // Update indexes - clone schema to avoid borrow checker issues
            let schema = self.metadata.tables.get(&query.table).unwrap().clone();
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

    /// Flush all pending changes to disk
    pub async fn flush_to_disk(&mut self) -> Result<()> {
        info!("💾 Flushing all data to disk...");

        // Save metadata
        self.save_metadata().await?;

        // Save transaction log
        self.save_transaction_log().await?;

        // Save indexes
        self.save_indexes().await?;

        // Save compressed blocks
        self.save_compressed_blocks().await?;

        info!("✅ All data flushed to disk successfully");
        Ok(())
    }

    /// Load existing data from disk
    pub async fn load_from_disk(&mut self) -> Result<()> {
        info!("📖 Loading existing data from disk...");

        // Load transaction log
        self.load_transaction_log().await?;

        // Load indexes
        self.load_indexes().await?;

        // Load compressed blocks
        self.load_compressed_blocks().await?;

        // Update next IDs from metadata
        self.next_row_id = self.metadata.next_row_id;
        self.next_lsn = self.metadata.next_lsn;

        info!("✅ Data loaded from disk successfully");
        Ok(())
    }

    /// Store data with a key (used by the main API)
    pub async fn store(&mut self, key: &str, data: &[u8]) -> Result<()> {
        // Create a simple row structure for generic storage
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Integer(self.next_row_id as i64));
        fields.insert("key".to_string(), Value::Text(key.to_string()));
        fields.insert("data".to_string(), Value::Binary(data.to_vec()));

        let row = Row {
            id: self.next_row_id,
            fields,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Ensure we have a generic storage table
        if !self.metadata.tables.contains_key("_storage") {
            let schema = TableSchema {
                name: "_storage".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "key".to_string(),
                        data_type: DataType::Text,
                        nullable: false,
                        default_value: None,
                    },
                    ColumnDefinition {
                        name: "data".to_string(),
                        data_type: DataType::Binary,
                        nullable: false,
                        default_value: None,
                    },
                ],
                primary_key: "id".to_string(),
                created_at: chrono::Utc::now(),
                version: 1,
            };
            self.create_table(schema).await?;
        }

        self.insert_row("_storage", row).await?;
        Ok(())
    }

    /// Retrieve data by key (used by the main API)
    pub async fn retrieve(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Query for the key in the generic storage table
        let query = SelectQuery {
            table: "_storage".to_string(),
            columns: vec!["data".to_string()],
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: "key".to_string(),
                    operator: ComparisonOperator::Equal,
                    value: Value::Text(key.to_string()),
                }],
            }),
            order_by: None,
            limit: Some(1),
            offset: None,
        };

        let rows = self.select_rows(&query).await?;
        if let Some(row) = rows.first() {
            if let Some(Value::Binary(data)) = row.fields.get("data") {
                return Ok(Some(data.clone()));
            }
        }

        Ok(None)
    }

    // === PRIVATE HELPER METHODS ===

    /// Validate table schema
    fn validate_schema(&self, schema: &TableSchema) -> Result<()> {
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
    fn validate_row(&self, schema: &TableSchema, row: &Row) -> Result<()> {
        for column in &schema.columns {
            if let Some(value) = row.fields.get(&column.name) {
                // Type validation
                let valid_type = match (&column.data_type, value) {
                    (DataType::Integer, Value::Integer(_)) => true,
                    (DataType::Float, Value::Float(_)) => true,
                    (DataType::Text, Value::Text(_)) => true,
                    (DataType::Boolean, Value::Boolean(_)) => true,
                    (DataType::Timestamp, Value::Timestamp(_)) => true,
                    (DataType::Binary, Value::Binary(_)) => true,
                    (_, Value::Null) => column.nullable,
                    _ => false,
                };

                if !valid_type {
                    return Err(anyhow!(
                        "Type mismatch for column '{}': expected {:?}, got {:?}",
                        column.name,
                        column.data_type,
                        value
                    ));
                }
            } else if !column.nullable && column.default_value.is_none() {
                return Err(anyhow!("Required column '{}' is missing", column.name));
            }
        }

        Ok(())
    }

    /// Compress row data using DNA compression
    async fn compress_row(&mut self, row: &Row) -> Result<EncodedData> {
        // Use bincode for efficient binary serialization (more compatible with HashMap)
        let serialized =
            bincode::serialize(row).map_err(|e| anyhow!("Failed to serialize row: {}", e))?;
        let compressed = self.dna_compressor.compress(&serialized).await?;
        Ok(compressed)
    }

    /// Decompress row data from DNA compression
    #[allow(dead_code)]
    async fn decompress_row(&mut self, encoded: &EncodedData) -> Result<Row> {
        let decompressed = self.dna_compressor.decompress(encoded).await?;
        let row: Row = bincode::deserialize(&decompressed)
            .map_err(|e| anyhow!("Failed to deserialize row: {}", e))?;
        Ok(row)
    }

    /// Update indexes for inserted row
    fn update_indexes_for_insert(&mut self, schema: &TableSchema, row: &Row) -> Result<()> {
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
    fn update_indexes_for_delete(&mut self, schema: &TableSchema, row: &Row) -> Result<()> {
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
    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Text(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Timestamp(ts) => ts.to_rfc3339(),
            Value::Binary(b) => {
                use base64::{engine::general_purpose, Engine as _};
                general_purpose::STANDARD.encode(b)
            }
            Value::Null => "NULL".to_string(),
        }
    }

    /// Add row to cache with LRU eviction
    fn add_to_cache(&mut self, row: Row) {
        if self.row_cache.len() >= self.cache_limit {
            // Simple eviction: remove oldest entry
            if let Some(first_key) = self.row_cache.keys().next().cloned() {
                self.row_cache.remove(&first_key);
            }
        }
        self.row_cache.insert(row.id, row);
    }

    /// Apply WHERE clause to filter rows
    fn apply_where_clause(&self, rows: Vec<Row>, where_clause: &WhereClause) -> Result<Vec<Row>> {
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
    fn evaluate_condition(
        &self,
        field_value: &Value,
        operator: &ComparisonOperator,
        condition_value: &Value,
    ) -> Result<bool> {
        match operator {
            ComparisonOperator::Equal => Ok(field_value == condition_value),
            ComparisonOperator::NotEqual => Ok(field_value != condition_value),
            ComparisonOperator::LessThan => self
                .compare_values(field_value, condition_value)
                .map(|ord| ord.is_lt()),
            ComparisonOperator::LessThanOrEqual => self
                .compare_values(field_value, condition_value)
                .map(|ord| ord.is_le()),
            ComparisonOperator::GreaterThan => self
                .compare_values(field_value, condition_value)
                .map(|ord| ord.is_gt()),
            ComparisonOperator::GreaterThanOrEqual => self
                .compare_values(field_value, condition_value)
                .map(|ord| ord.is_ge()),
            ComparisonOperator::Like => {
                if let (Value::Text(field_text), Value::Text(pattern)) =
                    (field_value, condition_value)
                {
                    Ok(field_text.contains(pattern))
                } else {
                    Ok(false)
                }
            }
            ComparisonOperator::In => {
                // For simplicity, treating this as equality for now
                Ok(field_value == condition_value)
            }
        }
    }

    /// Compare two values for ordering
    fn compare_values(&self, a: &Value, b: &Value) -> Result<std::cmp::Ordering> {
        use std::cmp::Ordering;

        match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(a.cmp(b)),
            (Value::Float(a), Value::Float(b)) => Ok(a.partial_cmp(b).unwrap_or(Ordering::Equal)),
            (Value::Text(a), Value::Text(b)) => Ok(a.cmp(b)),
            (Value::Boolean(a), Value::Boolean(b)) => Ok(a.cmp(b)),
            (Value::Timestamp(a), Value::Timestamp(b)) => Ok(a.cmp(b)),
            _ => Err(anyhow!("Cannot compare values of different types")),
        }
    }

    /// Apply ORDER BY to sort rows
    fn apply_order_by(&self, rows: &mut [Row], order_by: &OrderBy) -> Result<()> {
        rows.sort_by(|a, b| {
            let a_value = a.fields.get(&order_by.field);
            let b_value = b.fields.get(&order_by.field);

            match (a_value, b_value) {
                (Some(a_val), Some(b_val)) => {
                    let cmp = self
                        .compare_values(a_val, b_val)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    match order_by.direction {
                        SortDirection::Ascending => cmp,
                        SortDirection::Descending => cmp.reverse(),
                    }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        Ok(())
    }

    /// Project specific columns from rows
    fn project_columns(&self, rows: Vec<Row>, columns: &[String]) -> Result<Vec<Row>> {
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

    /// Load all rows for a table with DNA decompression
    async fn load_table_rows(&self, table: &str) -> Result<Vec<Row>> {
        let table_path = self.data_dir.join("tables").join(format!("{}.nqdb", table));

        if !table_path.exists() {
            return Ok(Vec::new());
        }

        let file_content = fs::read(&table_path).await?;

        // If file is empty, return empty vector
        if file_content.is_empty() {
            return Ok(Vec::new());
        }

        let mut rows = Vec::new();
        let mut offset = 0;

        // Try to read as binary compressed format first
        while offset < file_content.len() {
            // Check if we have enough bytes for length prefix
            if offset + 4 > file_content.len() {
                break;
            }

            // Read length prefix
            let len_bytes: [u8; 4] = file_content[offset..offset + 4]
                .try_into()
                .map_err(|_| anyhow!("Failed to read length prefix"))?;
            let entry_len = u32::from_le_bytes(len_bytes) as usize;
            offset += 4;

            // Check if we have enough bytes for entry
            if offset + entry_len > file_content.len() {
                break;
            }

            // Read and deserialize compressed entry
            let entry_bytes = &file_content[offset..offset + entry_len];
            offset += entry_len;

            match bincode::deserialize::<CompressedRowEntry>(entry_bytes) {
                Ok(entry) => {
                    // Get the compressed data (decrypt first if encrypted)
                    let compressed_data = if let Some(ref encrypted_wrapper) =
                        entry.encrypted_wrapper
                    {
                        // Data is encrypted, decrypt it first
                        if let Some(ref encryption_manager) = self.encryption_manager {
                            match encryption_manager.decrypt(encrypted_wrapper) {
                                Ok(decrypted_bytes) => {
                                    // Deserialize the decrypted compressed data
                                    match bincode::deserialize::<EncodedData>(&decrypted_bytes) {
                                        Ok(data) => data,
                                        Err(e) => {
                                            debug!("Failed to deserialize decrypted data: {}", e);
                                            continue;
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("Failed to decrypt row: {}", e);
                                    continue;
                                }
                            }
                        } else {
                            debug!("Encrypted data found but no encryption manager available");
                            continue;
                        }
                    } else {
                        // Data is not encrypted, use directly
                        entry.compressed_data
                    };

                    // Decompress the row data
                    let compressor = self.dna_compressor.clone();
                    match compressor.decompress(&compressed_data).await {
                        Ok(decompressed) => {
                            // Try bincode first (new format), then JSON (legacy format)
                            if let Ok(row) = bincode::deserialize::<Row>(&decompressed) {
                                rows.push(row);
                            } else if let Ok(row) = serde_json::from_slice::<Row>(&decompressed) {
                                rows.push(row);
                            } else {
                                debug!("Failed to deserialize decompressed row with both bincode and JSON");
                                continue;
                            }
                        }
                        Err(e) => {
                            debug!("Failed to decompress row: {}", e);
                            continue;
                        }
                    }
                }
                Err(_) => {
                    // Might be legacy JSON format, try to read as text
                    break;
                }
            }
        }

        // If no rows were read, try legacy JSON format
        if rows.is_empty() && offset == 0 {
            let content = String::from_utf8_lossy(&file_content);
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(row) = serde_json::from_str::<Row>(line) {
                    rows.push(row);
                }
            }
        }

        Ok(rows)
    }

    /// Append row to table file with DNA compression and encryption
    async fn append_row_to_file(&mut self, table: &str, row: &Row) -> Result<()> {
        let table_path = self.data_dir.join("tables").join(format!("{}.nqdb", table));

        // Get or create compressed data for this row
        let compressed_data = if let Some(data) = self.compressed_blocks.get(&row.id) {
            data.clone()
        } else {
            // Compress the row if not already compressed
            let serialized = serde_json::to_vec(row)?;
            let compressed = self.dna_compressor.compress(&serialized).await?;
            self.compressed_blocks.insert(row.id, compressed.clone());
            compressed
        };

        // Optionally encrypt the compressed data
        let encrypted_wrapper = if let Some(ref encryption_manager) = self.encryption_manager {
            // Serialize the compressed data
            let compressed_bytes = bincode::serialize(&compressed_data)
                .map_err(|e| anyhow!("Failed to serialize compressed data: {}", e))?;

            // Encrypt the serialized compressed data
            let encrypted = encryption_manager.encrypt(&compressed_bytes)?;
            Some(encrypted)
        } else {
            None
        };

        // Create a storage entry with metadata, compressed data, and optional encryption
        let storage_entry = CompressedRowEntry {
            row_id: row.id,
            compressed_data,
            created_at: row.created_at,
            updated_at: row.updated_at,
            encrypted_wrapper,
            format_version: 1,
        };

        // Serialize and write the entry
        let entry_bytes = bincode::serialize(&storage_entry)
            .map_err(|e| anyhow!("Failed to serialize storage entry: {}", e))?;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&table_path)
            .await?;

        // Write length prefix (4 bytes) followed by entry data
        let len_bytes = (entry_bytes.len() as u32).to_le_bytes();
        file.write_all(&len_bytes).await?;
        file.write_all(&entry_bytes).await?;
        file.flush().await?;

        // Immediately persist compressed blocks to quantum directory
        self.save_compressed_blocks().await?;

        debug!(
            "💾 Row {} written (DNA compressed{}, {} bytes)",
            row.id,
            if self.encryption_manager.is_some() {
                " + encrypted"
            } else {
                ""
            },
            entry_bytes.len()
        );

        Ok(())
    }

    /// Rewrite table file with updated rows
    async fn rewrite_table_file_with_updates(
        &mut self,
        table: &str,
        updated_rows: &[Row],
    ) -> Result<()> {
        let table_path = self.data_dir.join("tables").join(format!("{}.nqdb", table));

        // Create a HashMap of updated rows for quick lookup
        let updated_map: HashMap<RowId, &Row> = updated_rows.iter().map(|r| (r.id, r)).collect();

        // Load all existing rows
        let existing_rows = self.load_table_rows(table).await?;

        // Create a temporary file
        let temp_path = table_path.with_extension("tmp");
        let mut temp_file = fs::File::create(&temp_path).await?;

        // Write all rows to temporary file (updated ones with new data, others as-is)
        // Using the same binary format as append_row_to_file
        for existing_row in existing_rows {
            let row_to_write = if let Some(updated_row) = updated_map.get(&existing_row.id) {
                updated_row
            } else {
                &existing_row
            };

            // Get or create compressed data for this row
            let compressed_data = if let Some(data) = self.compressed_blocks.get(&row_to_write.id) {
                data.clone()
            } else {
                // Compress the row if not already compressed
                let serialized = serde_json::to_vec(row_to_write)?;
                let compressed = self.dna_compressor.compress(&serialized).await?;
                self.compressed_blocks
                    .insert(row_to_write.id, compressed.clone());
                compressed
            };

            // Optionally encrypt the compressed data
            let encrypted_wrapper = if let Some(ref encryption_manager) = self.encryption_manager {
                // Serialize the compressed data
                let compressed_bytes = bincode::serialize(&compressed_data)
                    .map_err(|e| anyhow!("Failed to serialize compressed data: {}", e))?;

                // Encrypt the serialized compressed data
                let encrypted = encryption_manager.encrypt(&compressed_bytes)?;
                Some(encrypted)
            } else {
                None
            };

            // Create a storage entry with metadata, compressed data, and optional encryption
            let storage_entry = CompressedRowEntry {
                row_id: row_to_write.id,
                compressed_data,
                created_at: row_to_write.created_at,
                updated_at: row_to_write.updated_at,
                encrypted_wrapper,
                format_version: 1,
            };

            // Serialize and write the entry using the same format as append_row_to_file
            let entry_bytes = bincode::serialize(&storage_entry)
                .map_err(|e| anyhow!("Failed to serialize storage entry: {}", e))?;

            // Write length prefix (4 bytes) followed by entry data
            let len_bytes = (entry_bytes.len() as u32).to_le_bytes();
            temp_file.write_all(&len_bytes).await?;
            temp_file.write_all(&entry_bytes).await?;
        }

        temp_file.flush().await?;

        // Replace the original file with the temporary file
        fs::rename(&temp_path, &table_path).await?;

        // Immediately persist compressed blocks to quantum directory
        self.save_compressed_blocks().await?;

        Ok(())
    }

    /// Rewrite table file with deleted rows
    async fn rewrite_table_file_with_deletions(
        &mut self,
        table: &str,
        deleted_row_ids: &[RowId],
    ) -> Result<()> {
        let table_path = self.data_dir.join("tables").join(format!("{}.nqdb", table));

        // Create a temporary file
        let temp_path = table_path.with_extension("tmp");
        let mut temp_file = fs::File::create(&temp_path).await?;

        // Load existing rows
        let existing_rows = self.load_table_rows(table).await?;

        // Write rows that are not deleted to temporary file
        // Using the same binary format as append_row_to_file
        for row in existing_rows {
            if !deleted_row_ids.contains(&row.id) {
                // Get or create compressed data for this row
                let compressed_data = if let Some(data) = self.compressed_blocks.get(&row.id) {
                    data.clone()
                } else {
                    // Compress the row if not already compressed
                    let serialized = serde_json::to_vec(&row)?;
                    let compressed = self.dna_compressor.compress(&serialized).await?;
                    self.compressed_blocks.insert(row.id, compressed.clone());
                    compressed
                };

                // Optionally encrypt the compressed data
                let encrypted_wrapper =
                    if let Some(ref encryption_manager) = self.encryption_manager {
                        // Serialize the compressed data
                        let compressed_bytes = bincode::serialize(&compressed_data)
                            .map_err(|e| anyhow!("Failed to serialize compressed data: {}", e))?;

                        // Encrypt the serialized compressed data
                        let encrypted = encryption_manager.encrypt(&compressed_bytes)?;
                        Some(encrypted)
                    } else {
                        None
                    };

                // Create a storage entry with metadata, compressed data, and optional encryption
                let storage_entry = CompressedRowEntry {
                    row_id: row.id,
                    compressed_data,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    encrypted_wrapper,
                    format_version: 1,
                };

                // Serialize and write the entry using the same format as append_row_to_file
                let entry_bytes = bincode::serialize(&storage_entry)
                    .map_err(|e| anyhow!("Failed to serialize storage entry: {}", e))?;

                // Write length prefix (4 bytes) followed by entry data
                let len_bytes = (entry_bytes.len() as u32).to_le_bytes();
                temp_file.write_all(&len_bytes).await?;
                temp_file.write_all(&entry_bytes).await?;
            }
        }

        temp_file.flush().await?;

        // Replace the original file with the temporary file
        fs::rename(&temp_path, &table_path).await?;

        // Immediately persist compressed blocks to quantum directory
        self.save_compressed_blocks().await?;

        Ok(())
    }

    /// Log an operation for transaction management
    async fn log_operation(&mut self, operation: Operation) -> Result<()> {
        let transaction = Transaction {
            id: TransactionId::new_v4(),
            operations: vec![operation],
            status: TransactionStatus::Committed,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            lsn: self.next_lsn,
        };

        self.next_lsn += 1;

        // Try to push transaction - if serialization fails, log warning but don't fail operation
        match bincode::serialize(&transaction) {
            Ok(_) => {
                self.transaction_log.push(transaction);

                // Keep transaction log size manageable
                if self.transaction_log.len() > 10000 {
                    self.transaction_log.drain(0..5000);
                }
            }
            Err(e) => {
                // Log warning but don't fail the operation
                debug!(
                    "⚠️  Warning: Failed to serialize transaction for logging: {}",
                    e
                );
            }
        }

        Ok(())
    }

    /// Save metadata to disk
    async fn save_metadata(&mut self) -> Result<()> {
        self.metadata.next_row_id = self.next_row_id;
        self.metadata.next_lsn = self.next_lsn;

        let metadata_path = self.data_dir.join("metadata.json");
        let content = serde_json::to_string_pretty(&self.metadata)?;
        fs::write(&metadata_path, content).await?;

        Ok(())
    }

    /// Save transaction log to disk
    async fn save_transaction_log(&self) -> Result<()> {
        let log_path = self.data_dir.join("logs").join("transaction.log");
        let content = serde_json::to_string_pretty(&self.transaction_log)?;
        fs::write(&log_path, content).await?;

        Ok(())
    }

    /// Load transaction log from disk
    async fn load_transaction_log(&mut self) -> Result<()> {
        let log_path = self.data_dir.join("logs").join("transaction.log");

        if log_path.exists() {
            let content = fs::read_to_string(&log_path).await?;
            if !content.trim().is_empty() {
                self.transaction_log = serde_json::from_str(&content)?;
            }
        }

        Ok(())
    }

    /// Save indexes to disk
    async fn save_indexes(&self) -> Result<()> {
        for (index_name, index_data) in &self.indexes {
            let index_path = self
                .data_dir
                .join("indexes")
                .join(format!("{}.idx", index_name));
            let content = serde_json::to_string_pretty(index_data)?;
            fs::write(&index_path, content).await?;
        }

        Ok(())
    }

    /// Load indexes from disk
    async fn load_indexes(&mut self) -> Result<()> {
        let indexes_dir = self.data_dir.join("indexes");

        if !indexes_dir.exists() {
            return Ok(());
        }

        let mut entries = fs::read_dir(&indexes_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("idx") {
                let content = fs::read_to_string(&path).await?;
                if !content.trim().is_empty() {
                    let index_data: BTreeMap<String, RowId> = serde_json::from_str(&content)?;

                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        self.indexes.insert(stem.to_string(), index_data);
                    }
                }
            }
        }

        Ok(())
    }

    /// Save compressed blocks to disk
    async fn save_compressed_blocks(&self) -> Result<()> {
        let blocks_path = self
            .data_dir
            .join("quantum")
            .join("compressed_blocks.qdata");
        // Use bincode instead of JSON because CompressedDNA contains HashMap with Vec<u8> keys
        // which cannot be serialized to JSON (JSON only supports string keys)
        let content = bincode::serialize(&self.compressed_blocks)
            .map_err(|e| anyhow!("Failed to serialize compressed blocks: {}", e))?;
        fs::write(&blocks_path, content).await?;

        Ok(())
    }

    /// Load compressed blocks from disk
    async fn load_compressed_blocks(&mut self) -> Result<()> {
        let blocks_path = self
            .data_dir
            .join("quantum")
            .join("compressed_blocks.qdata");

        if blocks_path.exists() {
            let content = fs::read(&blocks_path).await?;
            if !content.is_empty() {
                // Use bincode to deserialize (consistent with save_compressed_blocks)
                self.compressed_blocks = bincode::deserialize(&content)
                    .map_err(|e| anyhow!("Failed to deserialize compressed blocks: {}", e))?;
            }
        }

        Ok(())
    }

    /// Initialize transaction manager properly after construction
    pub async fn init_transaction_manager(&mut self) -> Result<()> {
        let log_dir = self.data_dir.join("logs");
        self.transaction_manager = crate::transaction::TransactionManager::new_async(&log_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize transaction manager: {}", e))?;

        info!("✅ Transaction manager initialized with ACID support");
        Ok(())
    }

    /// Begin a new transaction
    pub async fn begin_transaction(&self) -> Result<crate::transaction::TransactionId> {
        self.transaction_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .map_err(|e| anyhow!("Failed to begin transaction: {}", e))
    }

    /// Begin a transaction with specific isolation level
    pub async fn begin_transaction_with_isolation(
        &self,
        isolation_level: crate::transaction::IsolationLevel,
    ) -> Result<crate::transaction::TransactionId> {
        self.transaction_manager
            .begin_transaction(isolation_level)
            .await
            .map_err(|e| anyhow!("Failed to begin transaction: {}", e))
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, tx_id: crate::transaction::TransactionId) -> Result<()> {
        self.transaction_manager
            .commit(tx_id)
            .await
            .map_err(|e| anyhow!("Failed to commit transaction: {}", e))
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(
        &self,
        tx_id: crate::transaction::TransactionId,
    ) -> Result<()> {
        self.transaction_manager
            .rollback(tx_id)
            .await
            .map_err(|e| anyhow!("Failed to rollback transaction: {}", e))
    }

    /// Insert a row within a transaction
    pub async fn insert_row_transactional(
        &mut self,
        tx_id: crate::transaction::TransactionId,
        table: &str,
        mut row: Row,
    ) -> Result<RowId> {
        use crate::transaction::LockType;

        debug!("➕ Transactional insert into table: {}", table);

        // Acquire exclusive lock on table
        let resource_id = format!("table:{}", table);
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Exclusive)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {}", e))?;

        // Get table schema
        let schema = self
            .metadata
            .tables
            .get(table)
            .ok_or_else(|| anyhow!("Table '{}' does not exist", table))?
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
            .map_err(|e| anyhow!("Failed to log update: {}", e))?;

        // Apply changes
        self.compressed_blocks.insert(row.id, compressed_data);
        self.update_indexes_for_insert(&schema, &row)?;
        self.add_to_cache(row.clone());

        // Log operation to local transaction log
        let operation = Operation::Insert {
            table: table.to_string(),
            row_id: row.id,
            data: row.clone(),
        };
        self.log_operation(operation).await?;

        // Append to table file
        self.append_row_to_file(table, &row).await?;

        debug!("✅ Row inserted with ID: {} (tx: {:?})", row.id, tx_id);
        Ok(row.id)
    }

    /// Update rows within a transaction
    pub async fn update_rows_transactional(
        &mut self,
        tx_id: crate::transaction::TransactionId,
        query: &UpdateQuery,
    ) -> Result<u64> {
        use crate::transaction::LockType;

        debug!("✏️ Transactional update in table: {}", query.table);

        // Acquire exclusive lock on table
        let resource_id = format!("table:{}", query.table);
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Exclusive)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {}", e))?;

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
            let schema = self.metadata.tables.get(&query.table).unwrap();
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
                .map_err(|e| anyhow!("Failed to log update: {}", e))?;

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
    pub async fn delete_rows_transactional(
        &mut self,
        tx_id: crate::transaction::TransactionId,
        query: &DeleteQuery,
    ) -> Result<u64> {
        use crate::transaction::LockType;

        debug!("🗑️ Transactional delete from table: {}", query.table);

        // Acquire exclusive lock on table
        let resource_id = format!("table:{}", query.table);
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Exclusive)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {}", e))?;

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
                .map_err(|e| anyhow!("Failed to log delete: {}", e))?;

            // Keep track of deleted row IDs
            deleted_row_ids.push(row.id);

            // Apply changes
            self.compressed_blocks.remove(&row.id);
            self.row_cache.remove(&row.id);

            let schema = self.metadata.tables.get(&query.table).unwrap().clone();
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
    pub async fn select_rows_transactional(
        &self,
        tx_id: crate::transaction::TransactionId,
        query: &SelectQuery,
    ) -> Result<Vec<Row>> {
        use crate::transaction::LockType;

        debug!("🔍 Transactional select from table: {}", query.table);

        // Acquire shared lock on table for consistent reads
        let resource_id = format!("table:{}", query.table);
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Shared)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {}", e))?;

        // Perform the select (now protected by lock)
        self.select_rows(query).await
    }

    /// Execute a full transaction with automatic commit/rollback
    pub async fn execute_transaction<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce(
            &mut Self,
            crate::transaction::TransactionId,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + '_>>,
    {
        // Begin transaction
        let tx_id = self.begin_transaction().await?;

        // Execute the transaction function
        let result = f(self, tx_id).await;

        match result {
            Ok(value) => {
                // Commit on success
                self.commit_transaction(tx_id).await?;
                Ok(value)
            }
            Err(e) => {
                // Rollback on error
                let _ = self.rollback_transaction(tx_id).await; // Ignore rollback errors
                Err(e)
            }
        }
    }

    /// Get transaction statistics
    pub async fn get_transaction_statistics(&self) -> crate::transaction::TransactionStatistics {
        self.transaction_manager.get_statistics().await
    }

    /// Write a checkpoint for recovery
    pub async fn checkpoint(&self) -> Result<()> {
        self.transaction_manager
            .checkpoint()
            .await
            .map_err(|e| anyhow!("Failed to write checkpoint: {}", e))
    }

    /// Cleanup timed out transactions
    pub async fn cleanup_timed_out_transactions(&self) -> Result<()> {
        self.transaction_manager
            .cleanup_timed_out_transactions()
            .await
            .map_err(|e| anyhow!("Failed to cleanup transactions: {}", e))
    }

    // === RECOVERY OPERATIONS ===

    /// Apply after-image to storage (REDO operation for recovery)
    pub async fn apply_after_image(
        &mut self,
        table: &str,
        key: &str,
        after_image: &[u8],
    ) -> Result<()> {
        debug!("♻️  Applying after-image (REDO) for {}.{}", table, key);

        // Deserialize the row from after-image
        let row: Row = serde_json::from_slice(after_image)
            .map_err(|e| anyhow!("Failed to deserialize after-image: {}", e))?;

        // Check if table exists
        let schema = self
            .metadata
            .tables
            .get(table)
            .ok_or_else(|| anyhow!("Table '{}' does not exist", table))?
            .clone();

        // Apply the row to storage
        let compressed_data = self.compress_row(&row).await?;
        self.compressed_blocks.insert(row.id, compressed_data);

        // Update indexes
        self.update_indexes_for_insert(&schema, &row)?;

        // Add to cache
        self.add_to_cache(row.clone());

        debug!("✅ REDO applied for row ID: {}", row.id);
        Ok(())
    }

    /// Apply before-image to storage (UNDO operation for recovery)
    pub async fn apply_before_image(
        &mut self,
        table: &str,
        key: &str,
        before_image: Option<&[u8]>,
    ) -> Result<()> {
        debug!("⏪ Applying before-image (UNDO) for {}.{}", table, key);

        if let Some(before_data) = before_image {
            // Deserialize the old row from before-image
            let old_row: Row = serde_json::from_slice(before_data)
                .map_err(|e| anyhow!("Failed to deserialize before-image: {}", e))?;

            // Check if table exists
            let schema = self
                .metadata
                .tables
                .get(table)
                .ok_or_else(|| anyhow!("Table '{}' does not exist", table))?
                .clone();

            // Restore the old row to storage
            let compressed_data = self.compress_row(&old_row).await?;
            self.compressed_blocks.insert(old_row.id, compressed_data);

            // Update indexes to old state
            self.update_indexes_for_insert(&schema, &old_row)?;

            // Update cache
            self.add_to_cache(old_row.clone());

            debug!("✅ UNDO applied for row ID: {}", old_row.id);
        } else {
            // No before-image means this was an INSERT - we need to remove the row
            // Parse row ID from key
            if let Ok(row_id) = key.parse::<RowId>() {
                self.compressed_blocks.remove(&row_id);
                self.row_cache.remove(&row_id);

                debug!("✅ UNDO applied: removed row ID {}", row_id);
            } else {
                tracing::warn!("Could not parse row ID from key: {}", key);
            }
        }

        Ok(())
    }

    /// Apply a log record during recovery (convenience method)
    pub async fn apply_log_record(
        &mut self,
        record: &crate::transaction::LogRecord,
        is_redo: bool,
    ) -> Result<()> {
        use crate::transaction::LogRecordType;

        match &record.record_type {
            LogRecordType::Update {
                table,
                key,
                before_image,
                after_image,
                ..
            } => {
                if is_redo {
                    self.apply_after_image(table, key, after_image).await?;
                } else {
                    self.apply_before_image(table, key, before_image.as_deref())
                        .await?;
                }
            }
            _ => {
                // Other log record types don't need storage application
                debug!("Skipping non-update log record type");
            }
        }

        Ok(())
    }

    /// Perform crash recovery by replaying WAL logs
    /// This integrates the TransactionManager's recovery with storage operations
    pub async fn perform_recovery(&mut self) -> Result<()> {
        use crate::transaction::{LogRecordType, TransactionId};
        use std::collections::HashSet;

        info!("🔄 Starting storage-level crash recovery...");

        // Get the WAL log manager from transaction manager (via shared reference)
        // For now, we'll read the logs directly using the internal log manager
        let log_dir = self.data_dir.join("logs");
        let log_manager = crate::transaction::LogManager::new(&log_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize log manager: {}", e))?;

        let log_records = log_manager
            .read_log()
            .await
            .map_err(|e| anyhow!("Failed to read log: {}", e))?;

        if log_records.is_empty() {
            info!("No log records to recover");
            return Ok(());
        }

        // Phase 1: Analysis - determine which transactions to undo/redo
        let mut active_txs: HashSet<TransactionId> = HashSet::new();
        let mut committed_txs: HashSet<TransactionId> = HashSet::new();

        for record in &log_records {
            match &record.record_type {
                LogRecordType::Begin { tx_id, .. } => {
                    active_txs.insert(*tx_id);
                }
                LogRecordType::Commit { tx_id } => {
                    active_txs.remove(tx_id);
                    committed_txs.insert(*tx_id);
                }
                LogRecordType::Abort { tx_id } => {
                    active_txs.remove(tx_id);
                }
                _ => {}
            }
        }

        info!(
            "Analysis: {} transactions to undo, {} to redo",
            active_txs.len(),
            committed_txs.len()
        );

        // Phase 2: REDO - reapply committed transactions
        for record in &log_records {
            if let Some(tx_id) = record.tx_id {
                if committed_txs.contains(&tx_id) {
                    self.apply_log_record(record, true).await?;
                }
            }
        }

        info!("REDO phase completed");

        // Phase 3: UNDO - rollback uncommitted transactions
        for record in log_records.iter().rev() {
            if let Some(tx_id) = record.tx_id {
                if active_txs.contains(&tx_id) {
                    self.apply_log_record(record, false).await?;
                }
            }
        }

        info!("UNDO phase completed");
        info!("✅ Storage-level crash recovery completed");

        Ok(())
    }
}

// === HELPER FUNCTIONS ===

/// Create a basic table schema for testing
pub fn create_test_schema(name: &str) -> TableSchema {
    TableSchema {
        name: name.to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "created_at".to_string(),
                data_type: DataType::Timestamp,
                nullable: false,
                default_value: None,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
    }
}

/// Create a test row
pub fn create_test_row(id: i64, name: &str) -> Row {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Integer(id));
    fields.insert("name".to_string(), Value::Text(name.to_string()));
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
