//! Write-Ahead Logging (WAL) System for `NeuroQuantumDB`
//!
//! Implements ARIES-style recovery with:
//! - Write-ahead logging with force-at-commit
//! - REDO/UNDO log records
//! - Checkpointing for fast recovery
//! - Crash recovery with analysis, redo, and undo phases
//! - Integration with Buffer Pool Manager

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub mod checkpoint;
pub mod log_writer;
pub mod recovery;

pub use checkpoint::{CheckpointManager, CheckpointRecord};
pub use log_writer::{LogWriter, LogWriterConfig};
pub use recovery::{RecoveryManager, RecoveryStats};

use super::pager::{PageId, PageStorageManager};

/// Log Sequence Number - unique identifier for each log record
pub type LSN = u64;

/// Transaction identifier
pub type TransactionId = Uuid;

/// WAL record types following ARIES protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WALRecordType {
    /// Transaction begin
    Begin {
        tx_id: TransactionId,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Page update with before/after images for undo/redo
    Update {
        tx_id: TransactionId,
        page_id: PageId,
        offset: usize,
        before_image: Vec<u8>,
        after_image: Vec<u8>,
    },
    /// Transaction commit
    Commit { tx_id: TransactionId },
    /// Transaction abort
    Abort { tx_id: TransactionId },
    /// Checkpoint begin
    CheckpointBegin {
        active_transactions: Vec<TransactionId>,
    },
    /// Checkpoint end
    CheckpointEnd,
    /// Compensation Log Record for undo operations
    CLR {
        tx_id: TransactionId,
        undo_next_lsn: LSN,
        page_id: PageId,
        redo_data: Vec<u8>,
    },
    /// Savepoint creation
    Savepoint {
        tx_id: TransactionId,
        name: String,
        /// LSN at the time of savepoint creation for rollback reference
        savepoint_lsn: LSN,
    },
    /// Rollback to savepoint
    RollbackToSavepoint {
        tx_id: TransactionId,
        name: String,
        /// LSN of the savepoint we're rolling back to
        target_lsn: LSN,
    },
    /// Release savepoint
    ReleaseSavepoint { tx_id: TransactionId, name: String },
}

/// WAL record - fundamental unit of the log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALRecord {
    /// Log Sequence Number
    pub lsn: LSN,
    /// Previous LSN for this transaction (for undo chain)
    pub prev_lsn: Option<LSN>,
    /// Transaction ID (None for checkpoint records)
    pub tx_id: Option<TransactionId>,
    /// Record type
    pub record_type: WALRecordType,
    /// Timestamp when record was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Checksum for integrity verification
    pub checksum: u32,
}

impl WALRecord {
    /// Create a new WAL record
    #[must_use]
    pub fn new(
        lsn: LSN,
        prev_lsn: Option<LSN>,
        tx_id: Option<TransactionId>,
        record_type: WALRecordType,
    ) -> Self {
        let mut record = Self {
            lsn,
            prev_lsn,
            tx_id,
            record_type,
            timestamp: chrono::Utc::now(),
            checksum: 0,
        };
        record.checksum = record.compute_checksum();
        record
    }

    /// Compute checksum for the record
    fn compute_checksum(&self) -> u32 {
        // Simple CRC32 checksum
        let data = bincode::serialize(&(
            self.lsn,
            self.prev_lsn,
            self.tx_id,
            &self.record_type,
            self.timestamp.timestamp(),
        ))
        .unwrap_or_default();

        crc32fast::hash(&data)
    }

    /// Verify checksum
    #[must_use]
    pub fn verify_checksum(&self) -> bool {
        let expected = self.compute_checksum();
        self.checksum == expected
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow!("Failed to serialize WAL record: {e}"))
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| anyhow!("Failed to deserialize WAL record: {e}"))
    }
}

/// WAL Manager configuration
#[derive(Debug, Clone)]
pub struct WALConfig {
    /// Directory for WAL files
    pub wal_dir: PathBuf,
    /// Maximum size of a single WAL segment file
    pub segment_size: usize,
    /// Whether to fsync after each write (durability vs performance)
    pub sync_on_write: bool,
    /// Buffer size for log writer
    pub buffer_size: usize,
    /// Checkpoint interval in seconds
    pub checkpoint_interval_secs: u64,
    /// Number of WAL segments to keep for recovery
    pub min_segments_to_keep: usize,
}

impl Default for WALConfig {
    fn default() -> Self {
        Self {
            wal_dir: PathBuf::from("data/wal"),
            segment_size: 64 * 1024 * 1024, // 64 MB
            sync_on_write: true,
            buffer_size: 256 * 1024,       // 256 KB
            checkpoint_interval_secs: 300, // 5 minutes
            min_segments_to_keep: 3,
        }
    }
}

/// Transaction state tracked by WAL for ARIES-style recovery
///
/// This structure maintains comprehensive transaction state information
/// required for proper crash recovery, including:
/// - Transaction boundaries (first/last LSN)
/// - Undo chain navigation (`undo_next_lsn`)
/// - Current transaction status for recovery decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionState {
    /// Unique transaction identifier
    pub tx_id: TransactionId,
    /// Current status of the transaction
    pub status: TransactionStatus,
    /// LSN of the first log record for this transaction
    pub first_lsn: LSN,
    /// LSN of the last log record for this transaction
    pub last_lsn: LSN,
    /// Next LSN to undo (for crash recovery undo chain)
    pub undo_next_lsn: Option<LSN>,
    /// Timestamp when transaction started
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Number of operations performed by this transaction
    pub operation_count: u64,
    /// Pages modified by this transaction (for selective undo)
    pub modified_pages: Vec<PageId>,
    /// Active savepoints with their LSNs (for nested savepoint support)
    pub savepoints: HashMap<String, LSN>,
}

impl TransactionState {
    /// Create a new transaction state
    #[must_use]
    pub fn new(tx_id: TransactionId, first_lsn: LSN) -> Self {
        Self {
            tx_id,
            status: TransactionStatus::Active,
            first_lsn,
            last_lsn: first_lsn,
            undo_next_lsn: None,
            start_time: chrono::Utc::now(),
            operation_count: 0,
            modified_pages: Vec::new(),
            savepoints: HashMap::new(),
        }
    }

    /// Check if transaction is in a terminal state
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Committed | TransactionStatus::Aborted
        )
    }

    /// Check if transaction needs undo during recovery
    #[must_use]
    pub const fn needs_undo(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Active | TransactionStatus::Aborting
        )
    }

    /// Check if transaction needs redo during recovery
    #[must_use]
    pub const fn needs_redo(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Committed | TransactionStatus::Committing
        )
    }

    /// Update the last LSN and increment operation count
    pub fn record_operation(&mut self, lsn: LSN, page_id: Option<PageId>) {
        self.last_lsn = lsn;
        self.undo_next_lsn = Some(lsn);
        self.operation_count += 1;
        if let Some(page) = page_id {
            if !self.modified_pages.contains(&page) {
                self.modified_pages.push(page);
            }
        }
    }

    /// Transition to committing state
    pub fn begin_commit(&mut self) {
        debug_assert_eq!(self.status, TransactionStatus::Active);
        self.status = TransactionStatus::Committing;
    }

    /// Transition to committed state
    pub fn complete_commit(&mut self, lsn: LSN) {
        debug_assert_eq!(self.status, TransactionStatus::Committing);
        self.status = TransactionStatus::Committed;
        self.last_lsn = lsn;
    }

    /// Transition to aborting state
    pub fn begin_abort(&mut self) {
        debug_assert!(matches!(
            self.status,
            TransactionStatus::Active | TransactionStatus::Committing
        ));
        self.status = TransactionStatus::Aborting;
    }

    /// Transition to aborted state
    pub fn complete_abort(&mut self, lsn: LSN) {
        debug_assert_eq!(self.status, TransactionStatus::Aborting);
        self.status = TransactionStatus::Aborted;
        self.last_lsn = lsn;
    }

    /// Get transaction duration
    #[must_use]
    pub fn duration(&self) -> chrono::Duration {
        chrono::Utc::now() - self.start_time
    }

    /// Get transaction summary for monitoring
    #[must_use]
    pub fn summary(&self) -> TransactionSummary {
        TransactionSummary {
            tx_id: self.tx_id,
            status: self.status,
            duration_ms: self.duration().num_milliseconds() as u64,
            operation_count: self.operation_count,
            pages_modified: self.modified_pages.len(),
            lsn_range: (self.first_lsn, self.last_lsn),
        }
    }
}

/// Transaction status for ARIES recovery protocol
///
/// Transaction lifecycle:
/// ```text
/// Active -> Committing -> Committed
///       \-> Aborting  -> Aborted
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is active and can perform operations
    Active,
    /// Transaction is in the process of committing (2PC prepare phase)
    Committing,
    /// Transaction has been successfully committed
    Committed,
    /// Transaction is in the process of being rolled back
    Aborting,
    /// Transaction has been rolled back
    Aborted,
}

impl TransactionStatus {
    /// Check if this status indicates an active transaction
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Committing | Self::Aborting)
    }

    /// Check if this status indicates a completed transaction
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Committed | Self::Aborted)
    }

    /// Get human-readable status string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Active => "ACTIVE",
            | Self::Committing => "COMMITTING",
            | Self::Committed => "COMMITTED",
            | Self::Aborting => "ABORTING",
            | Self::Aborted => "ABORTED",
        }
    }
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Summary of transaction state for monitoring and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSummary {
    /// Transaction ID
    pub tx_id: TransactionId,
    /// Current status
    pub status: TransactionStatus,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Number of operations
    pub operation_count: u64,
    /// Number of pages modified
    pub pages_modified: usize,
    /// LSN range (first, last)
    pub lsn_range: (LSN, LSN),
}

/// Write-Ahead Log Manager
///
/// Manages the write-ahead log with ARIES-style recovery protocol.
/// Ensures durability and atomicity of transactions.
#[derive(Clone)]
pub struct WALManager {
    _config: WALConfig,
    /// Current LSN counter
    next_lsn: Arc<AtomicU64>,
    /// Log writer for appending records
    log_writer: Arc<RwLock<LogWriter>>,
    /// Active transactions
    active_txns: Arc<RwLock<HashMap<TransactionId, TransactionState>>>,
    /// Transaction table: `tx_id` -> `last_lsn`
    transaction_table: Arc<RwLock<HashMap<TransactionId, LSN>>>,
    /// Dirty page table: `page_id` -> `recovery_lsn`
    dirty_page_table: Arc<RwLock<HashMap<PageId, LSN>>>,
    /// Checkpoint manager
    _checkpoint_manager: Arc<CheckpointManager>,
    /// Recovery manager
    recovery_manager: Arc<RecoveryManager>,
}

impl WALManager {
    /// Create a new WAL manager
    pub async fn new(config: WALConfig, pager: Arc<PageStorageManager>) -> Result<Self> {
        info!(
            "📝 Initializing WAL Manager at: {}",
            config.wal_dir.display()
        );

        // Create WAL directory if it doesn't exist
        tokio::fs::create_dir_all(&config.wal_dir).await?;

        // Initialize log writer
        let log_writer_config = LogWriterConfig {
            wal_dir: config.wal_dir.clone(),
            segment_size: config.segment_size,
            sync_on_write: config.sync_on_write,
            buffer_size: config.buffer_size,
        };
        let log_writer = LogWriter::new(log_writer_config).await?;

        // Get the last LSN from log writer
        let next_lsn = log_writer.get_next_lsn();

        let checkpoint_manager = Arc::new(CheckpointManager::new(config.clone()));
        let recovery_manager = Arc::new(RecoveryManager::new(config.clone(), Arc::clone(&pager)));

        let manager = Self {
            _config: config,
            next_lsn: Arc::new(AtomicU64::new(next_lsn)),
            log_writer: Arc::new(RwLock::new(log_writer)),
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            transaction_table: Arc::new(RwLock::new(HashMap::new())),
            dirty_page_table: Arc::new(RwLock::new(HashMap::new())),
            _checkpoint_manager: checkpoint_manager,
            recovery_manager,
        };

        info!("✅ WAL Manager initialized with LSN: {}", next_lsn);
        Ok(manager)
    }

    /// Begin a new transaction
    pub async fn begin_transaction(&self) -> Result<TransactionId> {
        let tx_id = Uuid::new_v4();
        let lsn = self.allocate_lsn();

        let record = WALRecord::new(
            lsn,
            None,
            Some(tx_id),
            WALRecordType::Begin {
                tx_id,
                timestamp: chrono::Utc::now(),
            },
        );

        // Write to log
        self.append_log_record(record).await?;

        // Track transaction using the new constructor
        let mut active_txns = self.active_txns.write().await;
        active_txns.insert(tx_id, TransactionState::new(tx_id, lsn));

        let mut tx_table = self.transaction_table.write().await;
        tx_table.insert(tx_id, lsn);

        debug!("🆕 Transaction started: {} (LSN: {})", tx_id, lsn);
        Ok(tx_id)
    }

    /// Log a page update
    pub async fn log_update(
        &self,
        tx_id: TransactionId,
        page_id: PageId,
        offset: usize,
        before_image: Vec<u8>,
        after_image: Vec<u8>,
    ) -> Result<LSN> {
        let lsn = self.allocate_lsn();

        // Get previous LSN for this transaction
        let prev_lsn = {
            let tx_table = self.transaction_table.read().await;
            tx_table.get(&tx_id).copied()
        };

        let record = WALRecord::new(
            lsn,
            prev_lsn,
            Some(tx_id),
            WALRecordType::Update {
                tx_id,
                page_id,
                offset,
                before_image,
                after_image,
            },
        );

        // Write to log
        self.append_log_record(record).await?;

        // Update transaction table
        let mut tx_table = self.transaction_table.write().await;
        tx_table.insert(tx_id, lsn);

        // Update transaction state using the new record_operation method
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            tx_state.record_operation(lsn, Some(page_id));
        }

        // Mark page as dirty
        let mut dirty_pages = self.dirty_page_table.write().await;
        dirty_pages.entry(page_id).or_insert(lsn);

        debug!(
            "📝 Update logged: TX={}, Page={}, LSN={}",
            tx_id, page_id.0, lsn
        );
        Ok(lsn)
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, tx_id: TransactionId) -> Result<()> {
        // Begin commit phase - update state first
        {
            let mut active_txns = self.active_txns.write().await;
            if let Some(tx_state) = active_txns.get_mut(&tx_id) {
                tx_state.begin_commit();
            }
        }

        let lsn = self.allocate_lsn();

        let prev_lsn = {
            let tx_table = self.transaction_table.read().await;
            tx_table.get(&tx_id).copied()
        };

        let record = WALRecord::new(lsn, prev_lsn, Some(tx_id), WALRecordType::Commit { tx_id });

        // Write commit record and force to disk
        self.append_log_record(record).await?;
        self.flush_log().await?;

        // Complete commit phase - update state with final LSN
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            tx_state.complete_commit(lsn);
        }

        // Remove from transaction table
        let mut tx_table = self.transaction_table.write().await;
        tx_table.remove(&tx_id);

        info!("✅ Transaction committed: {} (LSN: {})", tx_id, lsn);
        Ok(())
    }

    /// Abort a transaction
    pub async fn abort_transaction(&self, tx_id: TransactionId) -> Result<()> {
        // Begin abort phase - update state first
        {
            let mut active_txns = self.active_txns.write().await;
            if let Some(tx_state) = active_txns.get_mut(&tx_id) {
                tx_state.begin_abort();
            }
        }

        let lsn = self.allocate_lsn();

        let prev_lsn = {
            let tx_table = self.transaction_table.read().await;
            tx_table.get(&tx_id).copied()
        };

        let record = WALRecord::new(lsn, prev_lsn, Some(tx_id), WALRecordType::Abort { tx_id });

        // Write abort record
        self.append_log_record(record).await?;

        // Complete abort phase - update state with final LSN
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            tx_state.complete_abort(lsn);
        }

        // Remove from transaction table
        let mut tx_table = self.transaction_table.write().await;
        tx_table.remove(&tx_id);

        warn!("⚠️ Transaction aborted: {} (LSN: {})", tx_id, lsn);
        Ok(())
    }

    /// Create a savepoint within a transaction
    pub async fn create_savepoint(&self, tx_id: TransactionId, name: String) -> Result<LSN> {
        let lsn = self.allocate_lsn();

        // Get previous LSN for this transaction
        let prev_lsn = {
            let tx_table = self.transaction_table.read().await;
            tx_table.get(&tx_id).copied()
        };

        // Create savepoint record
        let record = WALRecord::new(
            lsn,
            prev_lsn,
            Some(tx_id),
            WALRecordType::Savepoint {
                tx_id,
                name: name.clone(),
                savepoint_lsn: lsn,
            },
        );

        // Write to log
        self.append_log_record(record).await?;

        // Update transaction table
        let mut tx_table = self.transaction_table.write().await;
        tx_table.insert(tx_id, lsn);

        // Store savepoint in transaction state
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            tx_state.savepoints.insert(name.clone(), lsn);
            tx_state.record_operation(lsn, None);
        }

        debug!(
            "💾 Savepoint created: {} for TX={} (LSN: {})",
            name, tx_id, lsn
        );
        Ok(lsn)
    }

    /// Rollback transaction to a savepoint
    pub async fn rollback_to_savepoint(&self, tx_id: TransactionId, name: String) -> Result<LSN> {
        // Get the savepoint LSN
        let target_lsn = {
            let active_txns = self.active_txns.read().await;
            let tx_state = active_txns
                .get(&tx_id)
                .ok_or_else(|| anyhow!("Transaction {tx_id} not found"))?;

            *tx_state
                .savepoints
                .get(&name)
                .ok_or_else(|| anyhow!("Savepoint '{name}' not found"))?
        };

        let lsn = self.allocate_lsn();

        let prev_lsn = {
            let tx_table = self.transaction_table.read().await;
            tx_table.get(&tx_id).copied()
        };

        // Create rollback to savepoint record
        let record = WALRecord::new(
            lsn,
            prev_lsn,
            Some(tx_id),
            WALRecordType::RollbackToSavepoint {
                tx_id,
                name: name.clone(),
                target_lsn,
            },
        );

        // Write to log
        self.append_log_record(record).await?;

        // Update transaction table
        let mut tx_table = self.transaction_table.write().await;
        tx_table.insert(tx_id, lsn);

        // Update transaction state
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            // Update undo_next_lsn to point to the savepoint for rollback chain
            tx_state.undo_next_lsn = Some(target_lsn);
            tx_state.last_lsn = lsn;
            tx_state.operation_count += 1;
            // Note: Savepoints are NOT removed after rollback (per SQL standard)
        }

        info!(
            "↩️  Rolled back to savepoint: {} for TX={} (target LSN: {}, current LSN: {})",
            name, tx_id, target_lsn, lsn
        );
        Ok(lsn)
    }

    /// Release a savepoint
    pub async fn release_savepoint(&self, tx_id: TransactionId, name: String) -> Result<LSN> {
        // Verify savepoint exists
        {
            let active_txns = self.active_txns.read().await;
            let tx_state = active_txns
                .get(&tx_id)
                .ok_or_else(|| anyhow!("Transaction {tx_id} not found"))?;

            if !tx_state.savepoints.contains_key(&name) {
                return Err(anyhow!("Savepoint '{name}' not found"));
            }
        }

        let lsn = self.allocate_lsn();

        let prev_lsn = {
            let tx_table = self.transaction_table.read().await;
            tx_table.get(&tx_id).copied()
        };

        // Create release savepoint record
        let record = WALRecord::new(
            lsn,
            prev_lsn,
            Some(tx_id),
            WALRecordType::ReleaseSavepoint {
                tx_id,
                name: name.clone(),
            },
        );

        // Write to log
        self.append_log_record(record).await?;

        // Update transaction table
        let mut tx_table = self.transaction_table.write().await;
        tx_table.insert(tx_id, lsn);

        // Remove savepoint from transaction state
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            tx_state.savepoints.remove(&name);
            tx_state.record_operation(lsn, None);
        }

        debug!(
            "🗑️  Savepoint released: {} for TX={} (LSN: {})",
            name, tx_id, lsn
        );
        Ok(lsn)
    }

    /// Perform a checkpoint
    pub async fn checkpoint(&self) -> Result<LSN> {
        info!("🛑 Starting checkpoint...");

        let active_txns = {
            let txns = self.active_txns.read().await;
            txns.keys().copied().collect::<Vec<_>>()
        };

        let lsn = self.allocate_lsn();

        // Write checkpoint begin record
        let begin_record = WALRecord::new(
            lsn,
            None,
            None,
            WALRecordType::CheckpointBegin {
                active_transactions: active_txns.clone(),
            },
        );
        self.append_log_record(begin_record).await?;

        // Flush all dirty pages
        self.flush_dirty_pages().await?;

        // Write checkpoint end record
        let end_lsn = self.allocate_lsn();
        let end_record = WALRecord::new(end_lsn, None, None, WALRecordType::CheckpointEnd);
        self.append_log_record(end_record).await?;

        // Force log to disk
        self.flush_log().await?;

        info!("✅ Checkpoint completed (LSN: {} - {})", lsn, end_lsn);
        Ok(end_lsn)
    }

    /// Recover from crash
    pub async fn recover(&self, pager: Arc<PageStorageManager>) -> Result<RecoveryStats> {
        info!("🔄 Starting crash recovery...");
        let stats = self.recovery_manager.recover(self, pager).await?;
        info!("✅ Recovery completed: {:?}", stats);
        Ok(stats)
    }

    /// Get current LSN
    #[must_use]
    pub fn get_current_lsn(&self) -> LSN {
        self.next_lsn.load(Ordering::SeqCst)
    }

    /// Allocate next LSN
    fn allocate_lsn(&self) -> LSN {
        self.next_lsn.fetch_add(1, Ordering::SeqCst)
    }

    /// Append a log record
    async fn append_log_record(&self, record: WALRecord) -> Result<()> {
        let mut writer = self.log_writer.write().await;
        writer.append_record(record).await
    }

    /// Flush log to disk
    async fn flush_log(&self) -> Result<()> {
        let mut writer = self.log_writer.write().await;
        writer.flush().await
    }

    /// Flush all dirty pages (called during checkpoint)
    async fn flush_dirty_pages(&self) -> Result<()> {
        // This will be called by the buffer pool manager
        // For now, just clear the dirty page table
        let mut dirty_pages = self.dirty_page_table.write().await;
        dirty_pages.clear();
        Ok(())
    }

    /// Read log records for recovery
    pub async fn read_log_records(&self, start_lsn: LSN) -> Result<Vec<WALRecord>> {
        let writer = self.log_writer.read().await;
        writer.read_records_from(start_lsn).await
    }

    /// Get transaction table (for checkpoint)
    pub async fn get_transaction_table(&self) -> HashMap<TransactionId, LSN> {
        self.transaction_table.read().await.clone()
    }

    /// Get dirty page table (for checkpoint)
    pub async fn get_dirty_page_table(&self) -> HashMap<PageId, LSN> {
        self.dirty_page_table.read().await.clone()
    }

    /// Get current LSN (alias for compatibility)
    #[must_use]
    pub fn current_lsn(&self) -> LSN {
        self.get_current_lsn()
    }

    /// Get WAL directory path
    #[must_use]
    pub fn get_wal_directory(&self) -> PathBuf {
        self._config.wal_dir.clone()
    }

    /// Get all WAL records since a given LSN (for incremental backup)
    pub async fn get_records_since_lsn(&self, since_lsn: LSN) -> Result<Vec<WALRecord>> {
        self.read_log_records(since_lsn).await
    }

    /// Get transaction state for a specific transaction
    pub async fn get_transaction_state(&self, tx_id: TransactionId) -> Option<TransactionState> {
        self.active_txns.read().await.get(&tx_id).cloned()
    }

    /// Get summaries of all active transactions for monitoring
    pub async fn get_active_transaction_summaries(&self) -> Vec<TransactionSummary> {
        self.active_txns
            .read()
            .await
            .values()
            .map(TransactionState::summary)
            .collect()
    }

    /// Get count of active transactions by status
    pub async fn get_transaction_stats(&self) -> TransactionStats {
        let txns = self.active_txns.read().await;
        let mut stats = TransactionStats::default();

        for state in txns.values() {
            match state.status {
                | TransactionStatus::Active => stats.active += 1,
                | TransactionStatus::Committing => stats.committing += 1,
                | TransactionStatus::Committed => stats.committed += 1,
                | TransactionStatus::Aborting => stats.aborting += 1,
                | TransactionStatus::Aborted => stats.aborted += 1,
            }
            stats.total_operations += state.operation_count;
        }

        stats.total_transactions = txns.len();
        stats
    }

    /// Check if a transaction is still active
    pub async fn is_transaction_active(&self, tx_id: TransactionId) -> bool {
        self.active_txns
            .read()
            .await
            .get(&tx_id)
            .is_some_and(|state| !state.is_terminal())
    }

    /// Get transactions that need undo during recovery
    pub async fn get_transactions_needing_undo(&self) -> Vec<TransactionState> {
        self.active_txns
            .read()
            .await
            .values()
            .filter(|state| state.needs_undo())
            .cloned()
            .collect()
    }

    /// Get transactions that need redo during recovery
    pub async fn get_transactions_needing_redo(&self) -> Vec<TransactionState> {
        self.active_txns
            .read()
            .await
            .values()
            .filter(|state| state.needs_redo())
            .cloned()
            .collect()
    }

    /// Get all pages modified by a transaction
    pub async fn get_modified_pages(&self, tx_id: TransactionId) -> Vec<PageId> {
        self.active_txns
            .read()
            .await
            .get(&tx_id)
            .map(|state| state.modified_pages.clone())
            .unwrap_or_default()
    }

    /// Get undo chain for a transaction (for selective rollback)
    pub async fn get_undo_chain(&self, tx_id: TransactionId) -> Option<(LSN, Option<LSN>)> {
        self.active_txns
            .read()
            .await
            .get(&tx_id)
            .map(|state| (state.last_lsn, state.undo_next_lsn))
    }
}

/// Statistics about transaction states for monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionStats {
    /// Total number of tracked transactions
    pub total_transactions: usize,
    /// Number of active transactions
    pub active: usize,
    /// Number of transactions in committing phase
    pub committing: usize,
    /// Number of committed transactions
    pub committed: usize,
    /// Number of transactions in aborting phase
    pub aborting: usize,
    /// Number of aborted transactions
    pub aborted: usize,
    /// Total operations across all transactions
    pub total_operations: u64,
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::storage::pager::{PagerConfig, SyncMode};

    async fn setup_test_env() -> (TempDir, Arc<PageStorageManager>, WALManager) {
        let temp_dir = TempDir::new().unwrap();
        let data_path = temp_dir.path().join("data");
        let wal_path = temp_dir.path().join("wal");

        tokio::fs::create_dir_all(&data_path).await.unwrap();
        tokio::fs::create_dir_all(&wal_path).await.unwrap();

        let pager_config = PagerConfig {
            max_file_size: 10 * 1024 * 1024, // 10MB for tests
            enable_checksums: true,
            sync_mode: SyncMode::None,
            direct_io: false,
        };

        let db_file = data_path.join("test.db");
        let pager = Arc::new(
            PageStorageManager::new(&db_file, pager_config)
                .await
                .unwrap(),
        );

        let wal_config = WALConfig {
            wal_dir: wal_path,
            segment_size: 1024 * 1024,
            sync_on_write: false,
            buffer_size: 64 * 1024,
            checkpoint_interval_secs: 60,
            min_segments_to_keep: 2,
        };

        let wal = WALManager::new(wal_config, Arc::clone(&pager))
            .await
            .unwrap();

        (temp_dir, pager, wal)
    }

    #[tokio::test]
    async fn test_begin_transaction() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();
        assert!(!tx_id.is_nil());

        let tx_table = wal.transaction_table.read().await;
        assert!(tx_table.contains_key(&tx_id));
    }

    #[tokio::test]
    async fn test_commit_transaction() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();
        wal.commit_transaction(tx_id).await.unwrap();

        let tx_table = wal.transaction_table.read().await;
        assert!(!tx_table.contains_key(&tx_id));
    }

    #[tokio::test]
    async fn test_log_update() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        let page_id = PageId(1);
        let before = vec![1, 2, 3];
        let after = vec![4, 5, 6];

        let lsn = wal
            .log_update(tx_id, page_id, 0, before, after)
            .await
            .unwrap();

        assert!(lsn > 0);

        let dirty_pages = wal.dirty_page_table.read().await;
        assert!(dirty_pages.contains_key(&page_id));
    }

    #[tokio::test]
    async fn test_checkpoint() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let _tx_id = wal.begin_transaction().await.unwrap();

        let checkpoint_lsn = wal.checkpoint().await.unwrap();
        assert!(checkpoint_lsn > 0);
    }

    #[tokio::test]
    async fn test_wal_record_serialization() {
        let record = WALRecord::new(
            1,
            None,
            Some(Uuid::new_v4()),
            WALRecordType::Begin {
                tx_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
            },
        );

        let bytes = record.to_bytes().unwrap();
        let deserialized = WALRecord::from_bytes(&bytes).unwrap();

        assert_eq!(record.lsn, deserialized.lsn);
        assert!(deserialized.verify_checksum());
    }

    #[tokio::test]
    async fn test_transaction_state_new() {
        let tx_id = Uuid::new_v4();
        let lsn = 42;
        let state = TransactionState::new(tx_id, lsn);

        assert_eq!(state.tx_id, tx_id);
        assert_eq!(state.status, TransactionStatus::Active);
        assert_eq!(state.first_lsn, lsn);
        assert_eq!(state.last_lsn, lsn);
        assert!(state.undo_next_lsn.is_none());
        assert_eq!(state.operation_count, 0);
        assert!(state.modified_pages.is_empty());
    }

    #[tokio::test]
    async fn test_transaction_state_record_operation() {
        let tx_id = Uuid::new_v4();
        let mut state = TransactionState::new(tx_id, 1);

        let page1 = PageId(100);
        let page2 = PageId(200);

        state.record_operation(2, Some(page1));
        assert_eq!(state.last_lsn, 2);
        assert_eq!(state.undo_next_lsn, Some(2));
        assert_eq!(state.operation_count, 1);
        assert_eq!(state.modified_pages.len(), 1);

        state.record_operation(3, Some(page2));
        assert_eq!(state.last_lsn, 3);
        assert_eq!(state.operation_count, 2);
        assert_eq!(state.modified_pages.len(), 2);

        // Same page shouldn't be added twice
        state.record_operation(4, Some(page1));
        assert_eq!(state.modified_pages.len(), 2);
    }

    #[tokio::test]
    async fn test_transaction_state_commit_lifecycle() {
        let tx_id = Uuid::new_v4();
        let mut state = TransactionState::new(tx_id, 1);

        assert_eq!(state.status, TransactionStatus::Active);
        assert!(!state.is_terminal());

        state.begin_commit();
        assert_eq!(state.status, TransactionStatus::Committing);
        assert!(!state.is_terminal());

        state.complete_commit(10);
        assert_eq!(state.status, TransactionStatus::Committed);
        assert!(state.is_terminal());
        assert_eq!(state.last_lsn, 10);
    }

    #[tokio::test]
    async fn test_transaction_state_abort_lifecycle() {
        let tx_id = Uuid::new_v4();
        let mut state = TransactionState::new(tx_id, 1);

        state.begin_abort();
        assert_eq!(state.status, TransactionStatus::Aborting);
        assert!(!state.is_terminal());

        state.complete_abort(15);
        assert_eq!(state.status, TransactionStatus::Aborted);
        assert!(state.is_terminal());
        assert_eq!(state.last_lsn, 15);
    }

    #[tokio::test]
    async fn test_transaction_state_needs_undo_redo() {
        let tx_id = Uuid::new_v4();

        // Active needs undo, not redo
        let active_state = TransactionState::new(tx_id, 1);
        assert!(active_state.needs_undo());
        assert!(!active_state.needs_redo());

        // Committed needs redo, not undo
        let mut committed_state = TransactionState::new(tx_id, 1);
        committed_state.begin_commit();
        committed_state.complete_commit(10);
        assert!(!committed_state.needs_undo());
        assert!(committed_state.needs_redo());

        // Aborting needs undo
        let mut aborting_state = TransactionState::new(tx_id, 1);
        aborting_state.begin_abort();
        assert!(aborting_state.needs_undo());
        assert!(!aborting_state.needs_redo());
    }

    #[tokio::test]
    async fn test_transaction_state_summary() {
        let tx_id = Uuid::new_v4();
        let mut state = TransactionState::new(tx_id, 1);
        state.record_operation(2, Some(PageId(100)));
        state.record_operation(3, Some(PageId(200)));

        let summary = state.summary();
        assert_eq!(summary.tx_id, tx_id);
        assert_eq!(summary.status, TransactionStatus::Active);
        assert_eq!(summary.operation_count, 2);
        assert_eq!(summary.pages_modified, 2);
        assert_eq!(summary.lsn_range, (1, 3));
    }

    #[tokio::test]
    async fn test_transaction_status_display() {
        assert_eq!(TransactionStatus::Active.as_str(), "ACTIVE");
        assert_eq!(TransactionStatus::Committing.as_str(), "COMMITTING");
        assert_eq!(TransactionStatus::Committed.as_str(), "COMMITTED");
        assert_eq!(TransactionStatus::Aborting.as_str(), "ABORTING");
        assert_eq!(TransactionStatus::Aborted.as_str(), "ABORTED");

        assert!(TransactionStatus::Active.is_active());
        assert!(TransactionStatus::Committing.is_active());
        assert!(!TransactionStatus::Committed.is_active());

        assert!(!TransactionStatus::Active.is_complete());
        assert!(TransactionStatus::Committed.is_complete());
        assert!(TransactionStatus::Aborted.is_complete());
    }

    #[tokio::test]
    async fn test_wal_manager_get_transaction_state() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        let state = wal.get_transaction_state(tx_id).await;
        assert!(state.is_some());

        let state = state.unwrap();
        assert_eq!(state.tx_id, tx_id);
        assert_eq!(state.status, TransactionStatus::Active);
    }

    #[tokio::test]
    async fn test_wal_manager_transaction_stats() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let _tx1 = wal.begin_transaction().await.unwrap();
        let _tx2 = wal.begin_transaction().await.unwrap();

        let stats = wal.get_transaction_stats().await;
        assert_eq!(stats.total_transactions, 2);
        assert_eq!(stats.active, 2);
        assert_eq!(stats.committing, 0);
    }

    #[tokio::test]
    async fn test_wal_manager_is_transaction_active() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();
        assert!(wal.is_transaction_active(tx_id).await);

        wal.commit_transaction(tx_id).await.unwrap();
        // After commit, transaction should be marked as committed but still tracked
        // until cleanup
        let state = wal.get_transaction_state(tx_id).await;
        if let Some(s) = state {
            assert!(!wal.is_transaction_active(tx_id).await || s.is_terminal());
        }
    }

    #[tokio::test]
    async fn test_wal_manager_modified_pages() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        let page1 = PageId(100);
        let page2 = PageId(200);

        wal.log_update(tx_id, page1, 0, vec![0; 10], vec![1; 10])
            .await
            .unwrap();
        wal.log_update(tx_id, page2, 0, vec![0; 10], vec![2; 10])
            .await
            .unwrap();

        let modified = wal.get_modified_pages(tx_id).await;
        assert_eq!(modified.len(), 2);
        assert!(modified.contains(&page1));
        assert!(modified.contains(&page2));
    }

    #[tokio::test]
    async fn test_wal_manager_undo_chain() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        let page_id = PageId(100);
        let lsn1 = wal
            .log_update(tx_id, page_id, 0, vec![0; 10], vec![1; 10])
            .await
            .unwrap();
        let lsn2 = wal
            .log_update(tx_id, page_id, 0, vec![1; 10], vec![2; 10])
            .await
            .unwrap();

        let undo_chain = wal.get_undo_chain(tx_id).await;
        assert!(undo_chain.is_some());

        let (last_lsn, undo_next) = undo_chain.unwrap();
        assert_eq!(last_lsn, lsn2);
        assert!(undo_next.is_some());
        // undo_next should point to the last operation
        assert!(undo_next.unwrap() >= lsn1);
    }

    #[tokio::test]
    async fn test_transaction_state_serialization() {
        let tx_id = Uuid::new_v4();
        let mut state = TransactionState::new(tx_id, 1);
        state.record_operation(2, Some(PageId(100)));

        // Test serialization
        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: TransactionState = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.tx_id, tx_id);
        assert_eq!(deserialized.status, TransactionStatus::Active);
        assert_eq!(deserialized.operation_count, 1);
    }

    #[tokio::test]
    async fn test_savepoint_creation() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        // Create a savepoint
        let savepoint_lsn = wal
            .create_savepoint(tx_id, "sp1".to_string())
            .await
            .unwrap();
        assert!(savepoint_lsn > 0);

        // Verify savepoint is stored in transaction state
        let tx_state = wal.get_transaction_state(tx_id).await.unwrap();
        assert!(tx_state.savepoints.contains_key("sp1"));
        assert_eq!(*tx_state.savepoints.get("sp1").unwrap(), savepoint_lsn);
    }

    #[tokio::test]
    async fn test_multiple_savepoints() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        // Create multiple savepoints
        let sp1_lsn = wal
            .create_savepoint(tx_id, "sp1".to_string())
            .await
            .unwrap();
        let sp2_lsn = wal
            .create_savepoint(tx_id, "sp2".to_string())
            .await
            .unwrap();
        let sp3_lsn = wal
            .create_savepoint(tx_id, "sp3".to_string())
            .await
            .unwrap();

        // Verify all savepoints are stored
        let tx_state = wal.get_transaction_state(tx_id).await.unwrap();
        assert_eq!(tx_state.savepoints.len(), 3);
        assert!(sp1_lsn < sp2_lsn);
        assert!(sp2_lsn < sp3_lsn);
    }

    #[tokio::test]
    async fn test_rollback_to_savepoint() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        // Create savepoint
        let sp_lsn = wal
            .create_savepoint(tx_id, "sp1".to_string())
            .await
            .unwrap();

        // Perform some operations after savepoint
        let page_id = PageId(1);
        wal.log_update(tx_id, page_id, 0, vec![1, 2], vec![3, 4])
            .await
            .unwrap();

        // Rollback to savepoint
        let rollback_lsn = wal
            .rollback_to_savepoint(tx_id, "sp1".to_string())
            .await
            .unwrap();
        assert!(rollback_lsn > sp_lsn);

        // Verify transaction state updated
        let tx_state = wal.get_transaction_state(tx_id).await.unwrap();
        assert_eq!(tx_state.last_lsn, rollback_lsn);
        // Savepoint should still exist after rollback (per SQL standard)
        assert!(tx_state.savepoints.contains_key("sp1"));
    }

    #[tokio::test]
    async fn test_rollback_to_nonexistent_savepoint() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        // Try to rollback to non-existent savepoint
        let result = wal
            .rollback_to_savepoint(tx_id, "nonexistent".to_string())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_release_savepoint() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        // Create savepoint
        wal.create_savepoint(tx_id, "sp1".to_string())
            .await
            .unwrap();

        // Release savepoint
        let release_lsn = wal
            .release_savepoint(tx_id, "sp1".to_string())
            .await
            .unwrap();
        assert!(release_lsn > 0);

        // Verify savepoint is removed
        let tx_state = wal.get_transaction_state(tx_id).await.unwrap();
        assert!(!tx_state.savepoints.contains_key("sp1"));
    }

    #[tokio::test]
    async fn test_nested_savepoints() {
        let (_temp, _pager, wal) = setup_test_env().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        // Create nested savepoints
        let sp1_lsn = wal
            .create_savepoint(tx_id, "sp1".to_string())
            .await
            .unwrap();
        let sp2_lsn = wal
            .create_savepoint(tx_id, "sp2".to_string())
            .await
            .unwrap();
        let sp3_lsn = wal
            .create_savepoint(tx_id, "sp3".to_string())
            .await
            .unwrap();

        // Rollback to middle savepoint
        wal.rollback_to_savepoint(tx_id, "sp2".to_string())
            .await
            .unwrap();

        // Verify all savepoints still exist (per SQL standard)
        let tx_state = wal.get_transaction_state(tx_id).await.unwrap();
        assert!(tx_state.savepoints.contains_key("sp1"));
        assert!(tx_state.savepoints.contains_key("sp2"));
        assert!(tx_state.savepoints.contains_key("sp3"));

        // Verify LSNs are in order
        assert!(sp1_lsn < sp2_lsn);
        assert!(sp2_lsn < sp3_lsn);
    }

    #[tokio::test]
    async fn test_savepoint_wal_record_serialization() {
        let tx_id = Uuid::new_v4();
        let record = WALRecord::new(
            100,
            Some(99),
            Some(tx_id),
            WALRecordType::Savepoint {
                tx_id,
                name: "test_sp".to_string(),
                savepoint_lsn: 100,
            },
        );

        let bytes = record.to_bytes().unwrap();
        let deserialized = WALRecord::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.lsn, 100);
        assert!(deserialized.verify_checksum());

        match deserialized.record_type {
            | WALRecordType::Savepoint { name, .. } => {
                assert_eq!(name, "test_sp");
            },
            | _ => panic!("Expected Savepoint record type"),
        }
    }

    #[tokio::test]
    async fn test_rollback_to_savepoint_wal_record_serialization() {
        let tx_id = Uuid::new_v4();
        let record = WALRecord::new(
            150,
            Some(149),
            Some(tx_id),
            WALRecordType::RollbackToSavepoint {
                tx_id,
                name: "sp1".to_string(),
                target_lsn: 100,
            },
        );

        let bytes = record.to_bytes().unwrap();
        let deserialized = WALRecord::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.lsn, 150);
        assert!(deserialized.verify_checksum());

        match deserialized.record_type {
            | WALRecordType::RollbackToSavepoint {
                name, target_lsn, ..
            } => {
                assert_eq!(name, "sp1");
                assert_eq!(target_lsn, 100);
            },
            | _ => panic!("Expected RollbackToSavepoint record type"),
        }
    }

    #[tokio::test]
    async fn test_release_savepoint_wal_record_serialization() {
        let tx_id = Uuid::new_v4();
        let record = WALRecord::new(
            200,
            Some(199),
            Some(tx_id),
            WALRecordType::ReleaseSavepoint {
                tx_id,
                name: "sp1".to_string(),
            },
        );

        let bytes = record.to_bytes().unwrap();
        let deserialized = WALRecord::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.lsn, 200);
        assert!(deserialized.verify_checksum());

        match deserialized.record_type {
            | WALRecordType::ReleaseSavepoint { name, .. } => {
                assert_eq!(name, "sp1");
            },
            | _ => panic!("Expected ReleaseSavepoint record type"),
        }
    }
}
