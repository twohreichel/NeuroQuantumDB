//! `NeuroQuantumDB` Storage Engine
//! Provides persistent file-based storage with DNA compression, B+ tree indexes,
//! and ACID transaction support for production deployment

pub mod backup;
pub mod btree;
pub mod buffer;
pub mod encryption;
pub mod migration;
pub mod pager;
pub mod wal;

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
pub use backup::{
    BackupConfig, BackupManager, BackupMetadata, BackupStats, BackupStorageBackend,
    BackupStorageType, BackupType, IncrementalBackup, LocalBackend, RestoreManager, RestoreOptions,
    RestoreStats, S3Backend, S3Config,
};
pub use btree::{BTree, BTreeConfig};
pub use buffer::{BufferPoolConfig, BufferPoolManager, BufferPoolStats, EvictionPolicyType};
pub use encryption::{EncryptedData, EncryptionManager};
use lru::LruCache;
pub use migration::{
    BoxedSqlExecutor, Migration, MigrationConfig, MigrationDirection, MigrationExecutor,
    MigrationExecutorConfig, MigrationHistory, MigrationParser, MigrationProgress, MigrationRecord,
    MigrationResult, MigrationStatus, ProgressTracker, SafetyCheck, SqlExecutionResult,
    SqlExecutor, ValidationResult,
};
pub use pager::{PageStorageManager, PagerConfig, StorageStats, SyncMode};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, instrument};
use uuid::Uuid;
pub use wal::{RecoveryStats, WALConfig, WALManager};

use crate::dna::{DNACompressor, EncodedData, QuantumDNACompressor};
use crate::error::CoreError;
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

/// Strategy for automatic ID generation
///
/// This determines how unique identifiers are generated for new rows.
/// Choose based on your use case:
/// - `AutoIncrement`: Best for single-instance, high-performance scenarios
/// - `Uuid`: Best for distributed systems or when IDs should be unpredictable
/// - `Snowflake`: Best for distributed systems requiring sortable IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IdGenerationStrategy {
    /// Sequential auto-incrementing integer (1, 2, 3, ...)
    ///
    /// **Pros:**
    /// - Minimal storage (8 bytes)
    /// - Excellent B+ tree performance (sequential inserts)
    /// - Human-readable and debuggable
    /// - Perfect for synaptic/neural ID references
    ///
    /// **Cons:**
    /// - Predictable (potential security concern for public APIs)
    /// - Single point of generation (not ideal for distributed systems)
    #[default]
    AutoIncrement,

    /// UUID v4 (random 128-bit identifier)
    ///
    /// **Pros:**
    /// - Globally unique without coordination
    /// - Unpredictable (good for security)
    /// - Works in distributed systems
    ///
    /// **Cons:**
    /// - Larger storage (16 bytes)
    /// - Poor B+ tree performance (random distribution causes page splits)
    /// - Not human-readable
    Uuid,

    /// Snowflake-style ID (64-bit time-based with machine ID)
    ///
    /// **Pros:**
    /// - Time-sortable (roughly ordered by creation time)
    /// - Distributed generation with machine ID
    /// - Same storage as auto-increment (8 bytes)
    ///
    /// **Cons:**
    /// - Requires time synchronization
    /// - More complex implementation
    Snowflake {
        /// Machine/node identifier (0-1023)
        machine_id: u16,
    },
}

/// Auto-increment column configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoIncrementConfig {
    /// Column name that uses auto-increment
    pub column_name: String,
    /// Current next value to be assigned
    pub next_value: i64,
    /// Increment step (default: 1)
    pub increment_by: i64,
    /// Minimum value (default: 1)
    pub min_value: i64,
    /// Maximum value (default: `i64::MAX`)
    pub max_value: i64,
    /// Whether to cycle when max is reached
    pub cycle: bool,
}

impl Default for AutoIncrementConfig {
    fn default() -> Self {
        Self {
            column_name: "id".to_string(),
            next_value: 1,
            increment_by: 1,
            min_value: 1,
            max_value: i64::MAX,
            cycle: false,
        }
    }
}

impl AutoIncrementConfig {
    /// Create a new auto-increment config for a column
    pub fn new(column_name: impl Into<String>) -> Self {
        Self {
            column_name: column_name.into(),
            ..Default::default()
        }
    }

    /// Set the starting value
    #[must_use]
    pub const fn start_with(mut self, start: i64) -> Self {
        self.next_value = start;
        self
    }

    /// Set the increment step
    #[must_use]
    pub const fn increment_by(mut self, step: i64) -> Self {
        self.increment_by = step;
        self
    }

    /// Generate the next ID and advance the counter
    pub fn next_id(&mut self) -> Result<i64> {
        let current = self.next_value;

        // Check for overflow
        if self.increment_by > 0 && current > self.max_value - self.increment_by {
            if self.cycle {
                self.next_value = self.min_value;
            } else {
                return Err(anyhow!(
                    "Auto-increment column '{}' has reached maximum value {}",
                    self.column_name,
                    self.max_value
                ));
            }
        } else {
            self.next_value = current + self.increment_by;
        }

        Ok(current)
    }
}

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
    AlterTable {
        table: String,
        old_schema: TableSchema,
        new_schema: TableSchema,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Active,
    Committed,
    Aborted,
}

/// Query execution statistics for monitoring and optimization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryExecutionStats {
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
    /// Indexes used in the query
    pub indexes_used: Vec<String>,
    /// Whether index was actually used for query optimization
    pub index_scan: bool,
    /// Number of rows examined from storage
    pub rows_examined: usize,
}

impl QueryExecutionStats {
    #[must_use]
    pub fn cache_hit_rate(&self) -> Option<f32> {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            Some(self.cache_hits as f32 / total as f32)
        } else {
            None
        }
    }
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
///
/// Note: `StorageEngine` is intentionally not Clone. Use `Arc<RwLock<StorageEngine>>`
/// for shared access across multiple tasks/threads. This prevents accidental
/// cloning of large internal data structures and ensures consistent cache state.
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

    /// In-memory LRU cache for frequently accessed data
    /// Uses proper LRU eviction strategy for optimal memory management
    row_cache: LruCache<RowId, Row>,

    /// Transaction manager for ACID compliance
    transaction_manager: TransactionManager,

    /// Encryption manager for data-at-rest encryption
    encryption_manager: Option<EncryptionManager>,

    /// Query execution statistics for the last query
    last_query_stats: QueryExecutionStats,
}

impl StorageEngine {
    /// Create a placeholder storage engine for two-phase initialization
    ///
    /// This is used for synchronous construction of StorageEngine,
    /// which is then properly initialized with async `new()` method.
    ///
    /// **Important:** This should NOT be used directly for production.
    /// Always follow with proper async initialization via `new()`.
    ///
    /// # Example
    /// ```no_run
    /// use neuroquantum_core::storage::StorageEngine;
    /// use std::path::Path;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let data_dir = Path::new("./data");
    /// // Don't use new_placeholder directly - use new() instead
    /// let storage = StorageEngine::new(data_dir).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[doc(hidden)] // Hide from public API docs
    #[must_use]
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
            // SAFETY: 10000 is a non-zero constant
            #[allow(clippy::expect_used)]
            row_cache: LruCache::new(NonZeroUsize::new(10000).expect("10000 is non-zero")),
            transaction_manager: TransactionManager::new(),
            encryption_manager: None,
            last_query_stats: QueryExecutionStats::default(),
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
            .map_err(|e| anyhow!("Failed to initialize transaction manager: {e}"))?;

        // Initialize encryption manager for data-at-rest encryption
        let encryption_manager = EncryptionManager::new(&data_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize encryption manager: {e}"))?;

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
            // SAFETY: 10000 is a non-zero constant
            #[allow(clippy::expect_used)]
            row_cache: LruCache::new(NonZeroUsize::new(10000).expect("10000 is non-zero")), // 10k rows LRU cache
            transaction_manager,
            encryption_manager: Some(encryption_manager),
            last_query_stats: QueryExecutionStats::default(),
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
            if dir.exists() {
                debug!("📁 Directory already exists: {}", dir.display());
            } else {
                fs::create_dir_all(dir).await.map_err(|e| {
                    anyhow!(
                        "Failed to create directory '{}': {} (Error code: {})",
                        dir.display(),
                        e,
                        e.raw_os_error().unwrap_or(-1)
                    )
                })?;
                info!("📁 Created directory: {}", dir.display());
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
        let row_ids: Vec<RowId> = table_rows.iter().map(|r| r.id).collect();

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
                let row_ids: Vec<RowId> = self.compressed_blocks.keys().copied().collect();

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
                let row_ids: Vec<RowId> = self.compressed_blocks.keys().copied().collect();

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
                let row_ids: Vec<RowId> = self.compressed_blocks.keys().copied().collect();

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
                let row_ids: Vec<RowId> = self.compressed_blocks.keys().copied().collect();

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
    /// # Returns
    /// * `Ok(())` - If successful
    /// * `Err` - If table doesn't exist
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
    async fn rewrite_table_file(&mut self, table_name: &str) -> Result<()> {
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
        let mut row_ids: Vec<RowId> = self.compressed_blocks.keys().copied().collect();
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

    /// Convert a value from one data type to another
    fn convert_value(
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

    /// Insert a new row into the specified table
    ///
    /// Automatically handles:
    /// - Row ID assignment (internal, always auto-increment)
    /// - `AUTO_INCREMENT` columns (user-defined)
    /// - SERIAL/BIGSERIAL columns (PostgreSQL-style)
    /// - Timestamp fields (`created_at`, `updated_at`)
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

    /// Populate auto-increment columns with generated values
    ///
    /// This handles columns marked as:
    /// - `AUTO_INCREMENT` (MySQL-style)
    /// - SERIAL/BIGSERIAL (PostgreSQL-style)
    /// - GENERATED AS IDENTITY (SQL standard)
    fn populate_auto_increment_columns(
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
    fn populate_default_values(schema: &TableSchema, row: &mut Row) {
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

    /// Select rows matching the given query
    #[instrument(level = "debug", skip(self, query), fields(table = %query.table))]
    pub async fn select_rows(&self, query: &SelectQuery) -> Result<Vec<Row>> {
        let (rows, _stats) = self.select_rows_with_stats(query).await?;
        Ok(rows)
    }

    /// Select rows matching the given query with execution statistics
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
    pub fn get_table_schema(&self, table_name: &str) -> Option<&TableSchema> {
        self.metadata.tables.get(table_name)
    }

    /// Get a mutable reference to the schema for a specific table
    ///
    /// Returns the table schema if it exists, or None if the table doesn't exist.
    /// This is useful for modifying auto-increment configurations.
    pub fn get_table_schema_mut(&mut self, table_name: &str) -> Option<&mut TableSchema> {
        self.metadata.tables.get_mut(table_name)
    }

    /// Update rows matching the given query
    ///
    /// Handles foreign key constraints:
    /// - Validates that new FK values exist in referenced tables
    /// - Applies ON UPDATE actions for tables referencing updated PK values
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

    /// Handle foreign key constraints when updating a row (ON UPDATE actions)
    async fn handle_update_foreign_key_constraints(
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
    async fn apply_update_action(
        &mut self,
        referencing_table: &str,
        fk: &ForeignKeyConstraint,
        _referenced_schema: &TableSchema,
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
                    _referenced_schema.name,
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

    /// Delete rows matching the given query
    ///
    /// Handles foreign key constraints according to their ON DELETE actions:
    /// - RESTRICT: Error if rows are referenced by other tables
    /// - CASCADE: Delete referencing rows in dependent tables
    /// - SET NULL: Set foreign key columns to NULL in referencing rows
    /// - SET DEFAULT: Set foreign key columns to default values
    /// - NO ACTION: Same as RESTRICT (checked at end of statement)
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
    fn delete_rows_internal<'a>(
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

    /// Handle foreign key constraints when deleting a row
    ///
    /// Finds all tables that reference this table and applies the ON DELETE action
    async fn handle_delete_foreign_key_constraints(
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
    async fn apply_delete_action(
        &mut self,
        referencing_table: &str,
        fk: &ForeignKeyConstraint,
        _referenced_schema: &TableSchema,
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
                    _referenced_schema.name,
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
    async fn update_row_internal(&mut self, table: &str, row: &Row) -> Result<()> {
        // Compress and store the updated row
        let compressed_data = self.compress_row(row).await?;
        self.compressed_blocks.insert(row.id, compressed_data);

        // Update cache
        self.row_cache.put(row.id, row.clone());

        // Rewrite the row in the table file
        // For simplicity, we'll reload all rows and rewrite
        let file_path = self.data_dir.join("tables").join(format!("{table}.dat"));
        if file_path.exists() {
            let content = fs::read_to_string(&file_path).await?;
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
            fs::write(&file_path, content).await?;
        }

        Ok(())
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
            let schema = TableSchema {
                name: "_storage".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: DataType::BigSerial, // Auto-increment ID
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

    /// Validate foreign key constraints for an INSERT operation
    ///
    /// Checks that all foreign key values reference existing rows in the referenced tables.
    /// Returns an error if any foreign key constraint is violated.
    async fn validate_foreign_key_constraints(
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
    /// Used for foreign key constraint validation
    async fn check_value_exists(&self, table: &str, column: &str, value: &Value) -> Result<bool> {
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

    /// Compress row data using DNA compression
    async fn compress_row(&mut self, row: &Row) -> Result<EncodedData> {
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
    async fn decompress_row(&self, encoded: &EncodedData) -> Result<Row> {
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
    fn add_to_cache(&mut self, row: Row) {
        // LruCache handles eviction automatically when capacity is exceeded
        // put() returns the evicted entry if any (we don't need it)
        self.row_cache.put(row.id, row);
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
            | ComparisonOperator::Equal => Ok(field_value == condition_value),
            | ComparisonOperator::NotEqual => Ok(field_value != condition_value),
            | ComparisonOperator::LessThan => self
                .compare_values(field_value, condition_value)
                .map(std::cmp::Ordering::is_lt),
            | ComparisonOperator::LessThanOrEqual => self
                .compare_values(field_value, condition_value)
                .map(std::cmp::Ordering::is_le),
            | ComparisonOperator::GreaterThan => self
                .compare_values(field_value, condition_value)
                .map(std::cmp::Ordering::is_gt),
            | ComparisonOperator::GreaterThanOrEqual => self
                .compare_values(field_value, condition_value)
                .map(std::cmp::Ordering::is_ge),
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
    fn compare_values(&self, a: &Value, b: &Value) -> Result<std::cmp::Ordering> {
        use std::cmp::Ordering;

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
    fn apply_order_by(&self, rows: &mut [Row], order_by: &OrderBy) -> Result<()> {
        rows.sort_by(|a, b| {
            let a_value = a.fields.get(&order_by.field);
            let b_value = b.fields.get(&order_by.field);

            match (a_value, b_value) {
                | (Some(a_val), Some(b_val)) => {
                    let cmp = self
                        .compare_values(a_val, b_val)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    match order_by.direction {
                        | SortDirection::Ascending => cmp,
                        | SortDirection::Descending => cmp.reverse(),
                    }
                },
                | (Some(_), None) => std::cmp::Ordering::Less,
                | (None, Some(_)) => std::cmp::Ordering::Greater,
                | (None, None) => std::cmp::Ordering::Equal,
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
        let table_path = self.data_dir.join("tables").join(format!("{table}.nqdb"));

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
                | Ok(entry) => {
                    // Get the compressed data (decrypt first if encrypted)
                    let compressed_data = if let Some(ref encrypted_wrapper) =
                        entry.encrypted_wrapper
                    {
                        // Data is encrypted, decrypt it first
                        if let Some(ref encryption_manager) = self.encryption_manager {
                            match encryption_manager.decrypt(encrypted_wrapper) {
                                | Ok(decrypted_bytes) => {
                                    // Deserialize the decrypted compressed data
                                    match bincode::deserialize::<EncodedData>(&decrypted_bytes) {
                                        | Ok(data) => data,
                                        | Err(e) => {
                                            debug!("Failed to deserialize decrypted data: {}", e);
                                            continue;
                                        },
                                    }
                                },
                                | Err(e) => {
                                    debug!("Failed to decrypt row: {}", e);
                                    continue;
                                },
                            }
                        } else {
                            debug!("Encrypted data found but no encryption manager available");
                            continue;
                        }
                    } else {
                        // Data is not encrypted, use directly
                        entry.compressed_data
                    };

                    // Decompress the row data using the async decompress_row method
                    match self.decompress_row(&compressed_data).await {
                        | Ok(row) => {
                            rows.push(row);
                        },
                        | Err(e) => {
                            debug!("Failed to decompress row: {}", e);
                        },
                    }
                },
                | Err(_) => {
                    // Might be legacy JSON format, try to read as text
                    break;
                },
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
        let table_path = self.data_dir.join("tables").join(format!("{table}.nqdb"));

        // Get or create compressed data for this row
        let compressed_data = if let Some(data) = self.compressed_blocks.get(&row.id) {
            data.clone()
        } else {
            // Compress the row if not already compressed
            // Use bincode for consistency with compress_row() and decompress_row()
            let serialized =
                bincode::serialize(row).map_err(|e| anyhow!("Failed to serialize row: {e}"))?;
            let compressed = self.dna_compressor.compress(&serialized).await?;
            self.compressed_blocks.insert(row.id, compressed.clone());
            compressed
        };

        // Optionally encrypt the compressed data
        let encrypted_wrapper = if let Some(ref encryption_manager) = self.encryption_manager {
            // Serialize the compressed data
            let compressed_bytes = bincode::serialize(&compressed_data)
                .map_err(|e| anyhow!("Failed to serialize compressed data: {e}"))?;

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
            .map_err(|e| anyhow!("Failed to serialize storage entry: {e}"))?;

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
        let table_path = self.data_dir.join("tables").join(format!("{table}.nqdb"));

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
                // Use bincode for consistency with compress_row() and decompress_row()
                let serialized = bincode::serialize(row_to_write)
                    .map_err(|e| anyhow!("Failed to serialize row: {e}"))?;
                let compressed = self.dna_compressor.compress(&serialized).await?;
                self.compressed_blocks
                    .insert(row_to_write.id, compressed.clone());
                compressed
            };

            // Optionally encrypt the compressed data
            let encrypted_wrapper = if let Some(ref encryption_manager) = self.encryption_manager {
                // Serialize the compressed data
                let compressed_bytes = bincode::serialize(&compressed_data)
                    .map_err(|e| anyhow!("Failed to serialize compressed data: {e}"))?;

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
                .map_err(|e| anyhow!("Failed to serialize storage entry: {e}"))?;

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
        let table_path = self.data_dir.join("tables").join(format!("{table}.nqdb"));

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
                    // Use bincode for consistency with compress_row() and decompress_row()
                    let serialized = bincode::serialize(&row)
                        .map_err(|e| anyhow!("Failed to serialize row: {e}"))?;
                    let compressed = self.dna_compressor.compress(&serialized).await?;
                    self.compressed_blocks.insert(row.id, compressed.clone());
                    compressed
                };

                // Optionally encrypt the compressed data
                let encrypted_wrapper =
                    if let Some(ref encryption_manager) = self.encryption_manager {
                        // Serialize the compressed data
                        let compressed_bytes = bincode::serialize(&compressed_data)
                            .map_err(|e| anyhow!("Failed to serialize compressed data: {e}"))?;

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
                    .map_err(|e| anyhow!("Failed to serialize storage entry: {e}"))?;

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
            | Ok(_) => {
                self.transaction_log.push(transaction);

                // Keep transaction log size manageable
                if self.transaction_log.len() > 10000 {
                    self.transaction_log.drain(0..5000);
                }
            },
            | Err(e) => {
                // Log warning but don't fail the operation
                debug!(
                    "⚠️  Warning: Failed to serialize transaction for logging: {}",
                    e
                );
            },
        }

        Ok(())
    }

    /// Save metadata to disk
    ///
    /// Persists the current database metadata including table schemas,
    /// auto-increment counters, and row/LSN counters.
    pub async fn save_metadata(&mut self) -> Result<()> {
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
                .join(format!("{index_name}.idx"));
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
            .map_err(|e| anyhow!("Failed to serialize compressed blocks: {e}"))?;
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
                    .map_err(|e| anyhow!("Failed to deserialize compressed blocks: {e}"))?;
            }
        }

        Ok(())
    }

    /// Initialize transaction manager properly after construction
    pub async fn init_transaction_manager(&mut self) -> Result<()> {
        let log_dir = self.data_dir.join("logs");
        self.transaction_manager = crate::transaction::TransactionManager::new_async(&log_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize transaction manager: {e}"))?;

        info!("✅ Transaction manager initialized with ACID support");
        Ok(())
    }

    /// Begin a new transaction
    #[instrument(level = "debug", skip(self))]
    pub async fn begin_transaction(&self) -> Result<crate::transaction::TransactionId> {
        self.transaction_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .map_err(|e| anyhow!("Failed to begin transaction: {e}"))
    }

    /// Begin a transaction with specific isolation level
    #[instrument(level = "debug", skip(self), fields(isolation_level = ?isolation_level))]
    pub async fn begin_transaction_with_isolation(
        &self,
        isolation_level: crate::transaction::IsolationLevel,
    ) -> Result<crate::transaction::TransactionId> {
        self.transaction_manager
            .begin_transaction(isolation_level)
            .await
            .map_err(|e| anyhow!("Failed to begin transaction: {e}"))
    }

    /// Commit a transaction and persist pending writes to disk
    #[instrument(level = "debug", skip(self), fields(tx_id = ?tx_id))]
    pub async fn commit_transaction(
        &mut self,
        tx_id: crate::transaction::TransactionId,
    ) -> Result<()> {
        use crate::transaction::LogRecordType;

        debug!("💾 Committing transaction: {:?}", tx_id);

        // Get the undo log to find pending writes (inserts/updates)
        if let Some(log_entries) = self.transaction_manager.get_undo_log(tx_id).await {
            for entry in &log_entries {
                if let LogRecordType::Update {
                    before_image,
                    table,
                    after_image,
                    ..
                } = &entry.record_type
                {
                    // If there's no before_image, this was an INSERT - write to disk
                    if before_image.is_none() {
                        if let Ok(row) = serde_json::from_slice::<Row>(after_image) {
                            debug!(
                                "COMMIT: Writing inserted row {} to disk for table {}",
                                row.id, table
                            );
                            self.append_row_to_file(table, &row).await?;
                        }
                    }
                    // For updates, the data is already on disk (we update in place)
                }
            }
        }

        // Now complete the transaction commit
        self.transaction_manager
            .commit(tx_id)
            .await
            .map_err(|e| anyhow!("Failed to commit transaction: {e}"))
    }

    /// Rollback a transaction and undo all changes
    pub async fn rollback_transaction(
        &mut self,
        tx_id: crate::transaction::TransactionId,
    ) -> Result<()> {
        use crate::transaction::LogRecordType;

        debug!("🔙 Rolling back transaction: {:?}", tx_id);

        // Get the undo log for this transaction before rolling back
        let undo_log = self.transaction_manager.get_undo_log(tx_id).await;

        // Apply undo operations in reverse order
        if let Some(log_entries) = undo_log {
            for entry in log_entries.iter().rev() {
                if let LogRecordType::Update {
                    before_image, key, ..
                } = &entry.record_type
                {
                    let row_id: RowId = key.parse().unwrap_or(0);
                    if before_image.is_none() {
                        // This was an INSERT - we need to delete the row
                        debug!("ROLLBACK: Deleting inserted row {}", key);
                        // Remove from compressed blocks
                        self.compressed_blocks.remove(&row_id);
                        // Remove from cache
                        self.row_cache.pop(&row_id);
                        // Note: Removing from disk file is complex; in a real implementation
                        // we would mark the row as deleted or use a tombstone approach
                    } else {
                        // This was an UPDATE or DELETE - restore the before image
                        debug!("ROLLBACK: Restoring before image for row {}", key);
                        if let Some(before) = before_image {
                            if let Ok(row) = serde_json::from_slice::<Row>(before) {
                                let compressed = self.compress_row(&row).await?;
                                self.compressed_blocks.insert(row_id, compressed);
                                self.add_to_cache(row);
                            }
                        }
                    }
                }
            }
        }

        // Now call the transaction manager to complete the rollback
        self.transaction_manager
            .rollback(tx_id)
            .await
            .map_err(|e| anyhow!("Failed to rollback transaction: {e}"))
    }

    /// Rollback transaction to a savepoint
    pub async fn rollback_to_savepoint(
        &mut self,
        tx_id: crate::transaction::TransactionId,
        savepoint_lsn: LSN,
    ) -> Result<u64> {
        use crate::transaction::LogRecordType;

        debug!(
            "↩️  Rolling back transaction {:?} to savepoint LSN {}",
            tx_id, savepoint_lsn
        );

        // Get the undo log for this transaction
        let undo_log = self.transaction_manager.get_undo_log(tx_id).await;
        let mut operations_undone = 0u64;

        // Apply undo operations in reverse order for entries after savepoint
        if let Some(log_entries) = undo_log {
            for entry in log_entries.iter().rev() {
                // Only undo operations that occurred after the savepoint
                if entry.lsn <= savepoint_lsn {
                    break;
                }

                operations_undone += 1;

                if let LogRecordType::Update {
                    before_image, key, ..
                } = &entry.record_type
                {
                    let row_id: RowId = key.parse().unwrap_or(0);
                    if before_image.is_none() {
                        // This was an INSERT - we need to delete the row
                        debug!("ROLLBACK TO SAVEPOINT: Deleting inserted row {}", key);
                        self.compressed_blocks.remove(&row_id);
                        self.row_cache.pop(&row_id);
                    } else {
                        // This was an UPDATE or DELETE - restore the before image
                        debug!(
                            "ROLLBACK TO SAVEPOINT: Restoring before image for row {}",
                            key
                        );
                        if let Some(before) = before_image {
                            if let Ok(row) = serde_json::from_slice::<Row>(before) {
                                let compressed = self.compress_row(&row).await?;
                                self.compressed_blocks.insert(row_id, compressed);
                                self.add_to_cache(row);
                            }
                        }
                    }
                }
            }
        }

        // Call the transaction manager to update its internal state
        self.transaction_manager
            .rollback_to_savepoint(tx_id, savepoint_lsn)
            .await
            .map_err(|e| anyhow!("Failed to rollback to savepoint: {e}"))?;

        Ok(operations_undone)
    }

    /// Get the undo log for a transaction
    pub async fn get_undo_log(
        &self,
        tx_id: crate::transaction::TransactionId,
    ) -> Option<Vec<crate::transaction::LogRecord>> {
        self.transaction_manager.get_undo_log(tx_id).await
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
        let resource_id = format!("table:{table}");
        self.transaction_manager
            .acquire_lock(tx_id, resource_id.clone(), LockType::Exclusive)
            .await
            .map_err(|e| anyhow!("Failed to acquire lock: {e}"))?;

        // Get table schema
        let schema = self
            .metadata
            .tables
            .get(table)
            .ok_or_else(|| anyhow!("Table '{table}' does not exist"))?
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
            .map_err(|e| anyhow!("Failed to log update: {e}"))?;

        // Apply changes to memory only (disk write happens at commit time)
        self.compressed_blocks.insert(row.id, compressed_data);
        self.update_indexes_for_insert(&schema, &row)?;
        self.add_to_cache(row.clone());

        // Log operation to local transaction log (for commit-time disk writes)
        let operation = Operation::Insert {
            table: table.to_string(),
            row_id: row.id,
            data: row.clone(),
        };
        self.log_operation(operation).await?;

        // NOTE: Do NOT append to table file here - this happens at commit time
        // For transactional inserts, we keep data in memory until commit
        // This enables proper rollback support

        debug!(
            "✅ Row inserted with ID: {} (tx: {:?}) - pending commit",
            row.id, tx_id
        );
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
            .map_err(|e| anyhow!("Failed to acquire lock: {e}"))?;

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
            let schema = self.metadata.tables.get(&query.table).ok_or_else(|| {
                CoreError::invalid_operation(&format!("Table '{}' schema not found", query.table))
            })?;
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
                .map_err(|e| anyhow!("Failed to log update: {e}"))?;

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
            .map_err(|e| anyhow!("Failed to acquire lock: {e}"))?;

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
                .map_err(|e| anyhow!("Failed to log delete: {e}"))?;

            // Keep track of deleted row IDs
            deleted_row_ids.push(row.id);

            // Apply changes
            self.compressed_blocks.remove(&row.id);
            self.row_cache.pop(&row.id);

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
            .map_err(|e| anyhow!("Failed to acquire lock: {e}"))?;

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
            | Ok(value) => {
                // Commit on success
                self.commit_transaction(tx_id).await?;
                Ok(value)
            },
            | Err(e) => {
                // Rollback on error
                let _ = self.rollback_transaction(tx_id).await; // Ignore rollback errors
                Err(e)
            },
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
            .map_err(|e| anyhow!("Failed to write checkpoint: {e}"))
    }

    /// Cleanup timed out transactions
    pub async fn cleanup_timed_out_transactions(&self) -> Result<()> {
        self.transaction_manager
            .cleanup_timed_out_transactions()
            .await
            .map_err(|e| anyhow!("Failed to cleanup transactions: {e}"))
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
            .map_err(|e| anyhow!("Failed to deserialize after-image: {e}"))?;

        // Check if table exists
        let schema = self
            .metadata
            .tables
            .get(table)
            .ok_or_else(|| anyhow!("Table '{table}' does not exist"))?
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
                .map_err(|e| anyhow!("Failed to deserialize before-image: {e}"))?;

            // Check if table exists
            let schema = self
                .metadata
                .tables
                .get(table)
                .ok_or_else(|| anyhow!("Table '{table}' does not exist"))?
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
                self.row_cache.pop(&row_id);

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
            | LogRecordType::Update {
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
            },
            | _ => {
                // Other log record types don't need storage application
                debug!("Skipping non-update log record type");
            },
        }

        Ok(())
    }

    /// Perform crash recovery by replaying WAL logs
    /// This integrates the `TransactionManager`'s recovery with storage operations
    pub async fn perform_recovery(&mut self) -> Result<()> {
        use std::collections::HashSet;

        use crate::transaction::{LogRecordType, TransactionId};

        info!("🔄 Starting storage-level crash recovery...");

        // Get the WAL log manager from transaction manager (via shared reference)
        // For now, we'll read the logs directly using the internal log manager
        let log_dir = self.data_dir.join("logs");
        let log_manager = crate::transaction::LogManager::new(&log_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize log manager: {e}"))?;

        let log_records = log_manager
            .read_log()
            .await
            .map_err(|e| anyhow!("Failed to read log: {e}"))?;

        if log_records.is_empty() {
            info!("No log records to recover");
            return Ok(());
        }

        // Phase 1: Analysis - determine which transactions to undo/redo
        let mut active_txs: HashSet<TransactionId> = HashSet::new();
        let mut committed_txs: HashSet<TransactionId> = HashSet::new();

        for record in &log_records {
            match &record.record_type {
                | LogRecordType::Begin { tx_id, .. } => {
                    active_txs.insert(*tx_id);
                },
                | LogRecordType::Commit { tx_id } => {
                    active_txs.remove(tx_id);
                    committed_txs.insert(*tx_id);
                },
                | LogRecordType::Abort { tx_id } => {
                    active_txs.remove(tx_id);
                },
                | _ => {},
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
#[must_use]
pub fn create_test_schema(name: &str) -> TableSchema {
    TableSchema {
        name: name.to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::BigSerial, // Auto-increment ID
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

/// Builder for creating `ColumnDefinition` with sensible defaults
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

/// Builder for creating `TableSchema` with sensible defaults
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

/// Create a test row
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

#[cfg(test)]
mod auto_increment_tests {
    use super::*;

    #[test]
    fn test_auto_increment_config_default() {
        let config = AutoIncrementConfig::default();
        assert_eq!(config.next_value, 1);
        assert_eq!(config.increment_by, 1);
        assert_eq!(config.min_value, 1);
        assert_eq!(config.max_value, i64::MAX);
        assert!(!config.cycle);
    }

    #[test]
    fn test_auto_increment_next_id() {
        let mut config = AutoIncrementConfig::new("id");

        // Generate sequential IDs
        assert_eq!(config.next_id().unwrap(), 1);
        assert_eq!(config.next_id().unwrap(), 2);
        assert_eq!(config.next_id().unwrap(), 3);
        assert_eq!(config.next_value, 4);
    }

    #[test]
    fn test_auto_increment_custom_start() {
        let mut config = AutoIncrementConfig::new("user_id").start_with(1000);

        assert_eq!(config.next_id().unwrap(), 1000);
        assert_eq!(config.next_id().unwrap(), 1001);
    }

    #[test]
    fn test_auto_increment_custom_increment() {
        let mut config = AutoIncrementConfig::new("id").increment_by(10);

        assert_eq!(config.next_id().unwrap(), 1);
        assert_eq!(config.next_id().unwrap(), 11);
        assert_eq!(config.next_id().unwrap(), 21);
    }

    #[test]
    fn test_id_generation_strategy_default() {
        let strategy = IdGenerationStrategy::default();
        assert_eq!(strategy, IdGenerationStrategy::AutoIncrement);
    }

    #[test]
    fn test_column_definition_with_auto_increment() {
        let col = ColumnDefinition {
            name: "id".to_string(),
            data_type: DataType::BigSerial,
            nullable: false,
            default_value: None,
            auto_increment: true,
        };

        assert!(col.auto_increment);
        assert_eq!(col.data_type, DataType::BigSerial);
    }

    #[test]
    fn test_table_schema_with_auto_increment() {
        let schema = TableSchema {
            name: "users".to_string(),
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
            ],
            primary_key: "id".to_string(),
            created_at: chrono::Utc::now(),
            version: 1,
            auto_increment_columns: HashMap::new(),
            id_strategy: IdGenerationStrategy::AutoIncrement,
            foreign_keys: Vec::new(),
        };

        // Verify id column is marked as auto_increment
        let id_col = schema.columns.iter().find(|c| c.name == "id").unwrap();
        assert!(id_col.auto_increment);
        assert_eq!(id_col.data_type, DataType::BigSerial);
    }

    #[test]
    fn test_serial_data_types() {
        // Serial should represent auto-increment integer types
        assert_ne!(DataType::Serial, DataType::Integer);
        assert_ne!(DataType::BigSerial, DataType::Integer);
    }
}
