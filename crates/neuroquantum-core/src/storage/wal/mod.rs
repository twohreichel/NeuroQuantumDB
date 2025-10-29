//! Write-Ahead Logging (WAL) System for NeuroQuantumDB
//!
//! Implements ARIES-style recovery with:
//! - Write-ahead logging with force-at-commit
//! - REDO/UNDO log records
//! - Checkpointing for fast recovery
//! - Crash recovery with analysis, redo, and undo phases
//! - Integration with Buffer Pool Manager

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
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
    Commit {
        tx_id: TransactionId,
    },
    /// Transaction abort
    Abort {
        tx_id: TransactionId,
    },
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
    pub fn verify_checksum(&self) -> bool {
        let expected = self.compute_checksum();
        self.checksum == expected
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| anyhow!("Failed to serialize WAL record: {}", e))
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| anyhow!("Failed to deserialize WAL record: {}", e))
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
            buffer_size: 256 * 1024, // 256 KB
            checkpoint_interval_secs: 300, // 5 minutes
            min_segments_to_keep: 3,
        }
    }
}

/// Transaction state tracked by WAL
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TransactionState {
    tx_id: TransactionId,
    status: TransactionStatus,
    first_lsn: LSN,
    last_lsn: LSN,
    undo_next_lsn: Option<LSN>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum TransactionStatus {
    Active,
    Committing,
    Committed,
    Aborting,
    Aborted,
}

/// Write-Ahead Log Manager
///
/// Manages the write-ahead log with ARIES-style recovery protocol.
/// Ensures durability and atomicity of transactions.
#[derive(Clone)]
pub struct WALManager {
    config: WALConfig,
    /// Current LSN counter
    next_lsn: Arc<AtomicU64>,
    /// Log writer for appending records
    log_writer: Arc<RwLock<LogWriter>>,
    /// Active transactions
    active_txns: Arc<RwLock<HashMap<TransactionId, TransactionState>>>,
    /// Transaction table: tx_id -> last_lsn
    transaction_table: Arc<RwLock<HashMap<TransactionId, LSN>>>,
    /// Dirty page table: page_id -> recovery_lsn
    dirty_page_table: Arc<RwLock<HashMap<PageId, LSN>>>,
    /// Checkpoint manager
    checkpoint_manager: Arc<CheckpointManager>,
    /// Recovery manager
    recovery_manager: Arc<RecoveryManager>,
}

impl WALManager {
    /// Create a new WAL manager
    pub async fn new(
        config: WALConfig,
        pager: Arc<PageStorageManager>,
    ) -> Result<Self> {
        info!("📝 Initializing WAL Manager at: {}", config.wal_dir.display());

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
        let recovery_manager = Arc::new(RecoveryManager::new(
            config.clone(),
            Arc::clone(&pager),
        ));

        let manager = Self {
            config,
            next_lsn: Arc::new(AtomicU64::new(next_lsn)),
            log_writer: Arc::new(RwLock::new(log_writer)),
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            transaction_table: Arc::new(RwLock::new(HashMap::new())),
            dirty_page_table: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_manager,
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

        // Track transaction
        let mut active_txns = self.active_txns.write().await;
        active_txns.insert(
            tx_id,
            TransactionState {
                tx_id,
                status: TransactionStatus::Active,
                first_lsn: lsn,
                last_lsn: lsn,
                undo_next_lsn: None,
            },
        );

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

        // Update transaction state
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            tx_state.last_lsn = lsn;
        }

        // Mark page as dirty
        let mut dirty_pages = self.dirty_page_table.write().await;
        dirty_pages.entry(page_id).or_insert(lsn);

        debug!("📝 Update logged: TX={}, Page={}, LSN={}", tx_id, page_id.0, lsn);
        Ok(lsn)
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, tx_id: TransactionId) -> Result<()> {
        let lsn = self.allocate_lsn();

        let prev_lsn = {
            let tx_table = self.transaction_table.read().await;
            tx_table.get(&tx_id).copied()
        };

        let record = WALRecord::new(
            lsn,
            prev_lsn,
            Some(tx_id),
            WALRecordType::Commit { tx_id },
        );

        // Write commit record and force to disk
        self.append_log_record(record).await?;
        self.flush_log().await?;

        // Update transaction state
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            tx_state.status = TransactionStatus::Committed;
            tx_state.last_lsn = lsn;
        }

        // Remove from transaction table
        let mut tx_table = self.transaction_table.write().await;
        tx_table.remove(&tx_id);

        info!("✅ Transaction committed: {} (LSN: {})", tx_id, lsn);
        Ok(())
    }

    /// Abort a transaction
    pub async fn abort_transaction(&self, tx_id: TransactionId) -> Result<()> {
        let lsn = self.allocate_lsn();

        let prev_lsn = {
            let tx_table = self.transaction_table.read().await;
            tx_table.get(&tx_id).copied()
        };

        let record = WALRecord::new(
            lsn,
            prev_lsn,
            Some(tx_id),
            WALRecordType::Abort { tx_id },
        );

        // Write abort record
        self.append_log_record(record).await?;

        // Update transaction state
        let mut active_txns = self.active_txns.write().await;
        if let Some(tx_state) = active_txns.get_mut(&tx_id) {
            tx_state.status = TransactionStatus::Aborted;
            tx_state.last_lsn = lsn;
        }

        // Remove from transaction table
        let mut tx_table = self.transaction_table.write().await;
        tx_table.remove(&tx_id);

        warn!("⚠️ Transaction aborted: {} (LSN: {})", tx_id, lsn);
        Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::pager::{PagerConfig, SyncMode};
    use tempfile::TempDir;

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
        let pager = Arc::new(PageStorageManager::new(&db_file, pager_config).await.unwrap());

        let wal_config = WALConfig {
            wal_dir: wal_path,
            segment_size: 1024 * 1024,
            sync_on_write: false,
            buffer_size: 64 * 1024,
            checkpoint_interval_secs: 60,
            min_segments_to_keep: 2,
        };

        let wal = WALManager::new(wal_config, Arc::clone(&pager)).await.unwrap();

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

        let tx_id = wal.begin_transaction().await.unwrap();

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
}

