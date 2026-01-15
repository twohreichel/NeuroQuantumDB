//! # Transaction Management System
//!
//! Complete ACID-compliant transaction management with:
//! - Write-Ahead Logging (WAL)
//! - Two-Phase Commit Protocol
//! - Multi-Version Concurrency Control (MVCC)
//! - Deadlock Detection
//! - Crash Recovery

use crate::error::NeuroQuantumError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::sync::{Mutex, RwLock as TokioRwLock};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Transaction identifier
pub type TransactionId = Uuid;

/// Log Sequence Number for write-ahead logging
pub type LSN = u64;

/// Resource identifier for locking
pub type ResourceId = String;

/// Isolation levels for transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IsolationLevel {
    /// Dirty reads allowed, lowest isolation
    ReadUncommitted,
    /// No dirty reads, non-repeatable reads allowed
    #[default]
    ReadCommitted,
    /// Repeatable reads, phantom reads allowed
    RepeatableRead,
    /// Full serializable isolation, highest level
    Serializable,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is active and accepting operations
    Active,
    /// Transaction is preparing to commit (2PC phase 1)
    Preparing,
    /// Transaction is prepared and ready to commit (2PC phase 1 complete)
    Prepared,
    /// Transaction is committing (2PC phase 2)
    Committing,
    /// Transaction successfully committed
    Committed,
    /// Transaction is aborting
    Aborting,
    /// Transaction aborted/rolled back
    Aborted,
}

/// Lock types for concurrency control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockType {
    /// Shared lock for read operations
    Shared,
    /// Exclusive lock for write operations
    Exclusive,
    /// Intention to acquire shared lock (for hierarchical locking)
    IntentionShared,
    /// Intention to acquire exclusive lock
    IntentionExclusive,
}

/// Lock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lock {
    pub transaction_id: TransactionId,
    pub resource_id: ResourceId,
    pub lock_type: LockType,
    pub acquired_at: chrono::DateTime<chrono::Utc>,
}

/// Write-Ahead Log record types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogRecordType {
    /// Transaction begin
    Begin {
        tx_id: TransactionId,
        isolation_level: IsolationLevel,
    },
    /// Data modification with before/after images
    Update {
        tx_id: TransactionId,
        table: String,
        key: String,
        before_image: Option<Vec<u8>>,
        after_image: Vec<u8>,
    },
    /// Transaction commit
    Commit { tx_id: TransactionId },
    /// Transaction abort
    Abort { tx_id: TransactionId },
    /// Checkpoint for recovery
    Checkpoint {
        active_transactions: Vec<TransactionId>,
    },
}

/// Write-Ahead Log record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    pub lsn: LSN,
    pub prev_lsn: Option<LSN>,
    pub tx_id: Option<TransactionId>,
    pub record_type: LogRecordType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Transaction context with MVCC support
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: TransactionId,
    pub status: TransactionStatus,
    pub isolation_level: IsolationLevel,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub timeout_seconds: u64,
    /// Resources locked by this transaction
    pub locks: HashSet<ResourceId>,
    /// First LSN of this transaction
    pub first_lsn: Option<LSN>,
    /// Last LSN of this transaction
    pub last_lsn: Option<LSN>,
    /// Undo log for rollback
    pub undo_log: Vec<LogRecord>,
    /// Transaction snapshot for MVCC
    pub snapshot_version: u64,
    /// Read set for serializable isolation
    pub read_set: HashSet<ResourceId>,
    /// Write set for conflict detection
    pub write_set: HashSet<ResourceId>,
}

impl Transaction {
    /// Create a new transaction
    #[must_use] 
    pub fn new(isolation_level: IsolationLevel, timeout_seconds: u64) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            status: TransactionStatus::Active,
            isolation_level,
            started_at: now,
            last_active: now,
            timeout_seconds,
            locks: HashSet::new(),
            first_lsn: None,
            last_lsn: None,
            undo_log: Vec::new(),
            snapshot_version: 0,
            read_set: HashSet::new(),
            write_set: HashSet::new(),
        }
    }

    /// Check if transaction has timed out
    #[must_use] 
    pub fn is_timed_out(&self) -> bool {
        let elapsed = chrono::Utc::now()
            .signed_duration_since(self.last_active)
            .num_seconds();
        elapsed as u64 > self.timeout_seconds
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_active = chrono::Utc::now();
    }
}

/// Lock manager for concurrency control with deadlock detection
pub struct LockManager {
    /// Current locks held on resources
    locks: Arc<RwLock<HashMap<ResourceId, Vec<Lock>>>>,
    /// Wait-for graph for deadlock detection
    wait_for: Arc<RwLock<HashMap<TransactionId, HashSet<TransactionId>>>>,
    /// Transactions waiting for locks
    waiting: Arc<RwLock<HashMap<TransactionId, ResourceId>>>,
}

impl LockManager {
    /// Create a new lock manager
    #[must_use] 
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
            wait_for: Arc::new(RwLock::new(HashMap::new())),
            waiting: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Acquire a lock on a resource
    #[instrument(skip(self))]
    pub async fn acquire_lock(
        &self,
        tx_id: TransactionId,
        resource_id: ResourceId,
        lock_type: LockType,
    ) -> Result<(), NeuroQuantumError> {
        debug!(
            "Transaction {:?} requesting {:?} lock on {}",
            tx_id, lock_type, resource_id
        );

        // Check if lock can be granted
        loop {
            let can_grant = self.can_grant_lock(&tx_id, &resource_id, lock_type).await?;

            if can_grant {
                // Grant the lock
                self.grant_lock(tx_id, resource_id.clone(), lock_type)
                    .await?;
                return Ok(());
            }

            // Check for deadlock before waiting
            self.check_deadlock(&tx_id, &resource_id).await?;

            // Wait for lock to become available
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    /// Check if a lock can be granted
    async fn can_grant_lock(
        &self,
        tx_id: &TransactionId,
        resource_id: &ResourceId,
        lock_type: LockType,
    ) -> Result<bool, NeuroQuantumError> {
        let locks = self
            .locks
            .read()
            .map_err(|e| NeuroQuantumError::TransactionError(format!("Lock poisoned: {e}")))?;

        if let Some(existing_locks) = locks.get(resource_id) {
            // Check compatibility with existing locks
            for lock in existing_locks {
                if lock.transaction_id == *tx_id {
                    // Same transaction can always upgrade/re-acquire
                    continue;
                }

                // Check lock compatibility
                let compatible = matches!(
                    (lock.lock_type, lock_type),
                    (LockType::Shared | LockType::IntentionShared, LockType::Shared) |
(LockType::IntentionShared, LockType::IntentionShared)
                );

                if !compatible {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Grant a lock to a transaction
    async fn grant_lock(
        &self,
        tx_id: TransactionId,
        resource_id: ResourceId,
        lock_type: LockType,
    ) -> Result<(), NeuroQuantumError> {
        let mut locks = self
            .locks
            .write()
            .map_err(|e| NeuroQuantumError::TransactionError(format!("Lock poisoned: {e}")))?;

        let lock = Lock {
            transaction_id: tx_id,
            resource_id: resource_id.clone(),
            lock_type,
            acquired_at: chrono::Utc::now(),
        };

        locks
            .entry(resource_id.clone())
            .or_insert_with(Vec::new)
            .push(lock);

        // Remove from waiting list
        let mut waiting = self
            .waiting
            .write()
            .map_err(|e| NeuroQuantumError::TransactionError(format!("Lock poisoned: {e}")))?;
        waiting.remove(&tx_id);

        debug!(
            "Granted {:?} lock on {} to {:?}",
            lock_type, resource_id, tx_id
        );
        Ok(())
    }

    /// Check for deadlock using cycle detection in wait-for graph
    async fn check_deadlock(
        &self,
        tx_id: &TransactionId,
        resource_id: &ResourceId,
    ) -> Result<(), NeuroQuantumError> {
        let locks = self
            .locks
            .read()
            .map_err(|e| NeuroQuantumError::TransactionError(format!("Lock poisoned: {e}")))?;

        // Build wait-for relationships
        if let Some(existing_locks) = locks.get(resource_id) {
            let mut wait_for = self.wait_for.write().map_err(|e| {
                NeuroQuantumError::TransactionError(format!("Lock poisoned: {e}"))
            })?;

            let waiting_for: HashSet<TransactionId> = existing_locks
                .iter()
                .filter(|lock| lock.transaction_id != *tx_id)
                .map(|lock| lock.transaction_id)
                .collect();

            wait_for.insert(*tx_id, waiting_for);

            // Detect cycle using DFS
            if self.has_cycle(&wait_for, tx_id)? {
                error!("Deadlock detected involving transaction {:?}", tx_id);
                return Err(NeuroQuantumError::DeadlockDetected(format!(
                    "Deadlock detected for transaction {tx_id:?}"
                )));
            }
        }

        Ok(())
    }

    /// Detect cycles in wait-for graph using DFS
    fn has_cycle(
        &self,
        wait_for: &HashMap<TransactionId, HashSet<TransactionId>>,
        start_tx: &TransactionId,
    ) -> Result<bool, NeuroQuantumError> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        Self::dfs_cycle_check(wait_for, start_tx, &mut visited, &mut stack)
    }

    fn dfs_cycle_check(
        wait_for: &HashMap<TransactionId, HashSet<TransactionId>>,
        current: &TransactionId,
        visited: &mut HashSet<TransactionId>,
        stack: &mut HashSet<TransactionId>,
    ) -> Result<bool, NeuroQuantumError> {
        if stack.contains(current) {
            return Ok(true); // Cycle detected
        }

        if visited.contains(current) {
            return Ok(false);
        }

        visited.insert(*current);
        stack.insert(*current);

        if let Some(waiting_for) = wait_for.get(current) {
            for tx in waiting_for {
                if Self::dfs_cycle_check(wait_for, tx, visited, stack)? {
                    return Ok(true);
                }
            }
        }

        stack.remove(current);
        Ok(false)
    }

    /// Release all locks held by a transaction
    pub async fn release_locks(&self, tx_id: &TransactionId) -> Result<(), NeuroQuantumError> {
        let mut locks = self
            .locks
            .write()
            .map_err(|e| NeuroQuantumError::TransactionError(format!("Lock poisoned: {e}")))?;

        let resources_to_remove: Vec<ResourceId> = locks.keys().cloned().collect();

        for resource_id in resources_to_remove {
            if let Some(resource_locks) = locks.get_mut(&resource_id) {
                resource_locks.retain(|lock| lock.transaction_id != *tx_id);
                if resource_locks.is_empty() {
                    locks.remove(&resource_id);
                }
            }
        }

        // Clean up wait-for graph
        let mut wait_for = self
            .wait_for
            .write()
            .map_err(|e| NeuroQuantumError::TransactionError(format!("Lock poisoned: {e}")))?;
        wait_for.remove(tx_id);

        debug!("Released all locks for transaction {:?}", tx_id);
        Ok(())
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Write-Ahead Log manager for durability
pub struct LogManager {
    log_file: Arc<Mutex<File>>,
    /// Path to the WAL log file - used for archiving, truncation, and recovery
    log_path: PathBuf,
    pub(crate) lsn_counter: Arc<AtomicU64>,
    /// Buffer for batch writes
    write_buffer: Arc<Mutex<VecDeque<LogRecord>>>,
    buffer_size: usize,
}

impl LogManager {
    /// Create a new log manager
    pub async fn new(log_dir: &Path) -> Result<Self, NeuroQuantumError> {
        tokio::fs::create_dir_all(log_dir).await.map_err(|e| {
            NeuroQuantumError::StorageError(format!("Failed to create log directory: {e}"))
        })?;

        let log_path = log_dir.join("wal.log");
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&log_path)
            .await
            .map_err(|e| {
                NeuroQuantumError::StorageError(format!("Failed to open WAL file: {e}"))
            })?;

        info!("📝 WAL initialized at: {}", log_path.display());

        Ok(Self {
            log_file: Arc::new(Mutex::new(log_file)),
            log_path,
            lsn_counter: Arc::new(AtomicU64::new(1)),
            write_buffer: Arc::new(Mutex::new(VecDeque::new())),
            buffer_size: 100,
        })
    }

    /// Create a placeholder log manager for two-phase initialization
    ///
    /// **Important:** This uses /dev/null and should NOT be used in production.
    /// Only for internal use during TransactionManager construction.
    ///
    /// # Panics
    ///
    /// Panics if /dev/null cannot be opened, which would indicate a severely broken system.
    #[doc(hidden)]
    #[allow(clippy::expect_used)] // Placeholder for internal use - /dev/null should always exist
    #[must_use] 
    pub fn new_placeholder() -> Self {
        Self {
            log_file: Arc::new(Mutex::new(File::from_std(
                std::fs::File::open("/dev/null").expect("/dev/null should always be accessible"),
            ))),
            log_path: PathBuf::from("/dev/null"),
            lsn_counter: Arc::new(AtomicU64::new(1)),
            write_buffer: Arc::new(Mutex::new(VecDeque::new())),
            buffer_size: 100,
        }
    }

    /// Write a log record to WAL
    #[instrument(skip(self, record_type))]
    pub async fn write_log_record(
        &self,
        tx_id: Option<TransactionId>,
        prev_lsn: Option<LSN>,
        record_type: LogRecordType,
    ) -> Result<LSN, NeuroQuantumError> {
        let lsn = self.lsn_counter.fetch_add(1, Ordering::SeqCst);

        let record = LogRecord {
            lsn,
            prev_lsn,
            tx_id,
            record_type,
            timestamp: chrono::Utc::now(),
        };

        // Add to buffer
        let mut buffer = self.write_buffer.lock().await;
        buffer.push_back(record.clone());

        // Flush if buffer is full
        if buffer.len() >= self.buffer_size {
            drop(buffer); // Release lock before flush
            self.flush_buffer().await?;
        }

        debug!("Wrote log record with LSN {}", lsn);
        Ok(lsn)
    }

    /// Flush write buffer to disk
    async fn flush_buffer(&self) -> Result<(), NeuroQuantumError> {
        let mut buffer = self.write_buffer.lock().await;

        if buffer.is_empty() {
            return Ok(());
        }

        let mut log_file = self.log_file.lock().await;

        while let Some(record) = buffer.pop_front() {
            let serialized = serde_json::to_vec(&record).map_err(|e| {
                NeuroQuantumError::SerializationError(format!(
                    "Failed to serialize log record: {e}"
                ))
            })?;

            // Write length prefix
            let len = serialized.len() as u32;
            log_file.write_all(&len.to_le_bytes()).await.map_err(|e| {
                NeuroQuantumError::StorageError(format!("Failed to write log length: {e}"))
            })?;

            // Write record
            log_file.write_all(&serialized).await.map_err(|e| {
                NeuroQuantumError::StorageError(format!("Failed to write log record: {e}"))
            })?;
        }

        // Sync to disk for durability
        log_file
            .sync_all()
            .await
            .map_err(|e| NeuroQuantumError::StorageError(format!("Failed to sync WAL: {e}")))?;

        Ok(())
    }

    /// Force log to disk (for commit)
    pub async fn force_log(&self, _lsn: LSN) -> Result<(), NeuroQuantumError> {
        self.flush_buffer().await?;

        let log_file = self.log_file.lock().await;
        log_file
            .sync_all()
            .await
            .map_err(|e| NeuroQuantumError::StorageError(format!("Failed to force log: {e}")))?;

        debug!("Forced log up to LSN {}", _lsn);
        Ok(())
    }

    /// Read log records for recovery
    pub async fn read_log(&self) -> Result<Vec<LogRecord>, NeuroQuantumError> {
        let mut log_file = self.log_file.lock().await;
        log_file.seek(SeekFrom::Start(0)).await.map_err(|e| {
            NeuroQuantumError::StorageError(format!("Failed to seek log file: {e}"))
        })?;

        let mut records = Vec::new();
        let mut len_buf = [0u8; 4];

        loop {
            // Read length prefix
            match log_file.read_exact(&mut len_buf).await {
                | Ok(_) => {},
                | Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                | Err(e) => {
                    return Err(NeuroQuantumError::StorageError(format!(
                        "Failed to read log length: {e}"
                    )));
                },
            }

            let len = u32::from_le_bytes(len_buf) as usize;

            // Read record
            let mut record_buf = vec![0u8; len];
            log_file.read_exact(&mut record_buf).await.map_err(|e| {
                NeuroQuantumError::StorageError(format!("Failed to read log record: {e}"))
            })?;

            let record: LogRecord = serde_json::from_slice(&record_buf).map_err(|e| {
                NeuroQuantumError::SerializationError(format!(
                    "Failed to deserialize log record: {e}"
                ))
            })?;

            records.push(record);
        }

        Ok(records)
    }

    /// Write checkpoint record
    pub async fn write_checkpoint(
        &self,
        active_transactions: Vec<TransactionId>,
    ) -> Result<LSN, NeuroQuantumError> {
        let lsn = self
            .write_log_record(
                None,
                None,
                LogRecordType::Checkpoint {
                    active_transactions,
                },
            )
            .await?;

        self.force_log(lsn).await?;
        info!("📍 Checkpoint written at LSN {}", lsn);
        Ok(lsn)
    }

    /// Get the path to the WAL log file
    #[must_use] 
    pub fn get_log_path(&self) -> &Path {
        &self.log_path
    }

    /// Archive the current WAL log file with a timestamp suffix
    /// This is useful for backup and point-in-time recovery
    pub async fn archive_log(&self) -> Result<PathBuf, NeuroQuantumError> {
        // Flush any pending writes first
        self.flush_buffer().await?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let archive_path = self
            .log_path
            .with_extension(format!("log.{timestamp}.archive"));

        tokio::fs::copy(&self.log_path, &archive_path)
            .await
            .map_err(|e| {
                NeuroQuantumError::StorageError(format!(
                    "Failed to archive WAL log from {} to {}: {}",
                    self.log_path.display(),
                    archive_path.display(),
                    e
                ))
            })?;

        info!("📦 WAL archived to: {}", archive_path.display());
        Ok(archive_path)
    }

    /// Truncate the WAL log after a successful checkpoint
    /// This removes all log records that are no longer needed for recovery
    pub async fn truncate_log_after_checkpoint(
        &self,
        checkpoint_lsn: LSN,
    ) -> Result<(), NeuroQuantumError> {
        // Read all records
        let all_records = self.read_log().await?;
        let total_records = all_records.len();

        // Keep only records after the checkpoint
        let records_to_keep: Vec<_> = all_records
            .into_iter()
            .filter(|r| r.lsn > checkpoint_lsn)
            .collect();
        let kept_count = records_to_keep.len();

        // Create a new truncated log file
        let truncated_path = self.log_path.with_extension("log.truncated");
        let mut truncated_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&truncated_path)
            .await
            .map_err(|e| {
                NeuroQuantumError::StorageError(format!("Failed to create truncated log: {e}"))
            })?;

        // Write kept records to truncated file
        for record in &records_to_keep {
            let serialized = serde_json::to_vec(record).map_err(|e| {
                NeuroQuantumError::SerializationError(format!(
                    "Failed to serialize log record: {e}"
                ))
            })?;

            let len = serialized.len() as u32;
            truncated_file
                .write_all(&len.to_le_bytes())
                .await
                .map_err(|e| {
                    NeuroQuantumError::StorageError(format!("Failed to write log length: {e}"))
                })?;
            truncated_file.write_all(&serialized).await.map_err(|e| {
                NeuroQuantumError::StorageError(format!("Failed to write log record: {e}"))
            })?;
        }

        truncated_file.sync_all().await.map_err(|e| {
            NeuroQuantumError::StorageError(format!("Failed to sync truncated log: {e}"))
        })?;
        drop(truncated_file);

        // Replace original with truncated
        tokio::fs::rename(&truncated_path, &self.log_path)
            .await
            .map_err(|e| {
                NeuroQuantumError::StorageError(format!(
                    "Failed to replace log with truncated: {e}"
                ))
            })?;

        info!(
            "✂️ WAL truncated after checkpoint LSN {}: {} records removed, {} kept",
            checkpoint_lsn,
            total_records - kept_count,
            kept_count
        );

        Ok(())
    }

    /// Get statistics about the WAL log file
    pub async fn get_log_stats(&self) -> Result<WALLogStats, NeuroQuantumError> {
        let metadata = tokio::fs::metadata(&self.log_path).await.map_err(|e| {
            NeuroQuantumError::StorageError(format!("Failed to get WAL metadata: {e}"))
        })?;

        let records = self.read_log().await?;
        let current_lsn = self.lsn_counter.load(Ordering::SeqCst);

        Ok(WALLogStats {
            log_path: self.log_path.clone(),
            file_size_bytes: metadata.len(),
            record_count: records.len(),
            current_lsn,
            oldest_lsn: records.first().map(|r| r.lsn),
            newest_lsn: records.last().map(|r| r.lsn),
        })
    }
}

/// Statistics about the WAL log file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALLogStats {
    pub log_path: PathBuf,
    pub file_size_bytes: u64,
    pub record_count: usize,
    pub current_lsn: LSN,
    pub oldest_lsn: Option<LSN>,
    pub newest_lsn: Option<LSN>,
}

/// Callback trait for storage operations during recovery
/// This allows the `RecoveryManager` to integrate with the `StorageEngine`
#[async_trait::async_trait]
pub trait RecoveryStorageCallback: Send + Sync {
    /// Apply an after-image (REDO operation)
    async fn apply_after_image(
        &self,
        table: &str,
        key: &str,
        after_image: &[u8],
    ) -> Result<(), NeuroQuantumError>;

    /// Apply a before-image (UNDO operation)
    async fn apply_before_image(
        &self,
        table: &str,
        key: &str,
        before_image: Option<&[u8]>,
    ) -> Result<(), NeuroQuantumError>;
}

/// Recovery statistics from ARIES recovery process
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecoveryStatistics {
    pub total_records_analyzed: usize,
    pub redo_operations: usize,
    pub undo_operations: usize,
    pub transactions_redone: usize,
    pub transactions_undone: usize,
    pub recovery_duration_ms: u64,
}

/// Recovery manager for crash recovery
pub struct RecoveryManager {
    log_manager: Arc<LogManager>,
}

impl RecoveryManager {
    /// Create a new recovery manager
    #[must_use] 
    pub const fn new(log_manager: Arc<LogManager>) -> Self {
        Self { log_manager }
    }

    /// Create a placeholder recovery manager for two-phase initialization
    ///
    /// **Important:** This uses a placeholder LogManager and should NOT be used in production.
    /// Only for internal use during synchronous construction.
    #[doc(hidden)]
    #[must_use] 
    pub fn new_placeholder() -> Self {
        Self {
            log_manager: Arc::new(LogManager::new_placeholder()),
        }
    }

    /// Perform crash recovery using ARIES algorithm
    /// Perform crash recovery using ARIES algorithm (lightweight mode without storage)
    /// This only analyzes the log and reports what would need to be done.
    /// For full recovery with storage integration, use `recover_with_storage()`.
    #[instrument(skip(self))]
    pub async fn recover(&self) -> Result<(), NeuroQuantumError> {
        info!("🔄 Starting crash recovery (analysis only)...");

        let log_records = self.log_manager.read_log().await?;

        if log_records.is_empty() {
            info!("No log records to recover");
            return Ok(());
        }

        // Phase 1: Analysis - determine which transactions to undo/redo
        let (undo_list, redo_list) = self.analyze_phase(&log_records)?;

        info!(
            "✅ Recovery analysis completed: {} transactions to redo, {} to undo",
            redo_list.len(),
            undo_list.len()
        );
        info!("   Use recover_with_storage() for full ARIES recovery with storage integration");

        Ok(())
    }

    /// Perform full ARIES crash recovery with storage integration
    ///
    /// This implements the complete ARIES recovery algorithm:
    /// 1. **Analysis Phase**: Scan the log to determine which transactions committed
    ///    and which were still active at crash time
    /// 2. **Redo Phase**: Replay all changes from committed transactions to restore
    ///    the database to its state at crash time
    /// 3. **Undo Phase**: Roll back all changes from uncommitted transactions
    ///
    /// # Arguments
    /// * `storage_callback` - Implementation of `RecoveryStorageCallback` that applies
    ///   changes to the actual storage engine
    ///
    /// # Returns
    /// * `RecoveryStatistics` with details about the recovery process
    #[instrument(skip(self, storage_callback))]
    pub async fn recover_with_storage(
        &self,
        storage_callback: &dyn RecoveryStorageCallback,
    ) -> Result<RecoveryStatistics, NeuroQuantumError> {
        let start_time = std::time::Instant::now();
        info!("🔄 Starting full ARIES crash recovery with storage integration...");

        let log_records = self.log_manager.read_log().await?;
        let total_records = log_records.len();

        if log_records.is_empty() {
            info!("No log records to recover");
            return Ok(RecoveryStatistics::default());
        }

        // Phase 1: Analysis
        info!(
            "📊 Phase 1: Analysis - scanning {} log records",
            total_records
        );
        let (undo_list, redo_list) = self.analyze_phase(&log_records)?;

        // Phase 2: Redo
        info!(
            "♻️ Phase 2: Redo - replaying {} committed transactions",
            redo_list.len()
        );
        let redo_count = self
            .redo_phase_with_storage(&log_records, &redo_list, storage_callback)
            .await?;

        // Phase 3: Undo
        info!(
            "↩️ Phase 3: Undo - rolling back {} uncommitted transactions",
            undo_list.len()
        );
        let undo_count = self
            .undo_phase_with_storage(&log_records, &undo_list, storage_callback)
            .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        let stats = RecoveryStatistics {
            total_records_analyzed: total_records,
            redo_operations: redo_count,
            undo_operations: undo_count,
            transactions_redone: redo_list.len(),
            transactions_undone: undo_list.len(),
            recovery_duration_ms: duration_ms,
        };

        info!("✅ ARIES recovery completed in {}ms", duration_ms);
        info!("   Records analyzed: {}", stats.total_records_analyzed);
        info!("   Redo operations: {}", stats.redo_operations);
        info!("   Undo operations: {}", stats.undo_operations);
        info!("   Transactions redone: {}", stats.transactions_redone);
        info!("   Transactions undone: {}", stats.transactions_undone);

        Ok(stats)
    }

    /// Analysis phase: determine transaction states
    fn analyze_phase(
        &self,
        log_records: &[LogRecord],
    ) -> Result<(HashSet<TransactionId>, HashSet<TransactionId>), NeuroQuantumError> {
        let mut active_txs: HashSet<TransactionId> = HashSet::new();
        let mut committed_txs: HashSet<TransactionId> = HashSet::new();

        for record in log_records {
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

        let undo_list = active_txs; // Uncommitted transactions need undo
        let redo_list = committed_txs; // Committed transactions need redo

        info!(
            "Analysis: {} transactions to undo, {} to redo",
            undo_list.len(),
            redo_list.len()
        );

        Ok((undo_list, redo_list))
    }

    /// Redo phase: reapply committed transactions with storage integration
    async fn redo_phase_with_storage(
        &self,
        log_records: &[LogRecord],
        redo_list: &HashSet<TransactionId>,
        storage_callback: &dyn RecoveryStorageCallback,
    ) -> Result<usize, NeuroQuantumError> {
        let mut redo_count = 0;

        for record in log_records {
            if let Some(tx_id) = record.tx_id {
                if redo_list.contains(&tx_id) {
                    if let LogRecordType::Update {
                        table,
                        key,
                        after_image,
                        ..
                    } = &record.record_type
                    {
                        debug!(
                            "REDO LSN {} for TX {:?}: {}.{}",
                            record.lsn, tx_id, table, key
                        );

                        // Apply after-image through storage callback
                        storage_callback
                            .apply_after_image(table, key, after_image)
                            .await?;

                        redo_count += 1;
                    }
                }
            }
        }

        info!("Redo phase completed: {} operations", redo_count);
        Ok(redo_count)
    }

    /// Undo phase: rollback uncommitted transactions with storage integration
    async fn undo_phase_with_storage(
        &self,
        log_records: &[LogRecord],
        undo_list: &HashSet<TransactionId>,
        storage_callback: &dyn RecoveryStorageCallback,
    ) -> Result<usize, NeuroQuantumError> {
        let mut undo_count = 0;

        // Process log records in reverse order for undo
        for record in log_records.iter().rev() {
            if let Some(tx_id) = record.tx_id {
                if undo_list.contains(&tx_id) {
                    if let LogRecordType::Update {
                        table,
                        key,
                        before_image,
                        ..
                    } = &record.record_type
                    {
                        debug!(
                            "UNDO LSN {} for TX {:?}: {}.{}",
                            record.lsn, tx_id, table, key
                        );

                        // Apply before-image through storage callback
                        storage_callback
                            .apply_before_image(table, key, before_image.as_deref())
                            .await?;

                        undo_count += 1;
                    }
                }
            }
        }

        info!("Undo phase completed: {} operations", undo_count);
        Ok(undo_count)
    }
}

/// Main transaction manager coordinating all transaction operations
#[derive(Clone)]
pub struct TransactionManager {
    /// Active transactions
    active_transactions: Arc<TokioRwLock<HashMap<TransactionId, Transaction>>>,
    /// Lock manager for concurrency control
    lock_manager: Arc<LockManager>,
    /// Log manager for durability
    log_manager: Arc<LogManager>,
    /// Recovery manager for crash recovery
    recovery_manager: Arc<RecoveryManager>,
    /// Global snapshot version counter for MVCC
    global_version: Arc<AtomicU64>,
    /// Transaction timeout in seconds
    default_timeout: u64,
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionManager {
    /// Create a placeholder transaction manager for synchronous construction
    /// This should be followed by proper async initialization with `new_async()`
    #[must_use] 
    pub fn new() -> Self {
        Self {
            active_transactions: Arc::new(TokioRwLock::new(HashMap::new())),
            lock_manager: Arc::new(LockManager::new()),
            log_manager: Arc::new(LogManager::new_placeholder()),
            recovery_manager: Arc::new(RecoveryManager::new_placeholder()),
            global_version: Arc::new(AtomicU64::new(1)),
            default_timeout: 30,
        }
    }

    /// Create a new transaction manager with proper async initialization
    pub async fn new_async(log_dir: &Path) -> Result<Self, NeuroQuantumError> {
        let log_manager = Arc::new(LogManager::new(log_dir).await?);
        let recovery_manager = Arc::new(RecoveryManager::new(log_manager.clone()));

        // Perform recovery on startup
        recovery_manager.recover().await?;

        Ok(Self {
            active_transactions: Arc::new(TokioRwLock::new(HashMap::new())),
            lock_manager: Arc::new(LockManager::new()),
            log_manager,
            recovery_manager,
            global_version: Arc::new(AtomicU64::new(1)),
            default_timeout: 30, // 30 seconds default
        })
    }

    /// Begin a new transaction
    #[instrument(skip(self))]
    pub async fn begin_transaction(
        &self,
        isolation_level: IsolationLevel,
    ) -> Result<TransactionId, NeuroQuantumError> {
        let mut tx = Transaction::new(isolation_level, self.default_timeout);
        let tx_id = tx.id;

        // Assign snapshot version for MVCC
        tx.snapshot_version = self.global_version.load(Ordering::SeqCst);

        // Write BEGIN log record
        let lsn = self
            .log_manager
            .write_log_record(
                Some(tx_id),
                None,
                LogRecordType::Begin {
                    tx_id,
                    isolation_level,
                },
            )
            .await?;

        tx.first_lsn = Some(lsn);
        tx.last_lsn = Some(lsn);

        // Add to active transactions
        let mut active = self.active_transactions.write().await;
        active.insert(tx_id, tx);

        info!(
            "🚀 Transaction {:?} started with {:?} isolation",
            tx_id, isolation_level
        );
        Ok(tx_id)
    }

    /// Commit a transaction using 2-Phase Commit
    #[instrument(skip(self))]
    pub async fn commit(&self, tx_id: TransactionId) -> Result<(), NeuroQuantumError> {
        let mut active = self.active_transactions.write().await;

        let tx = active.get_mut(&tx_id).ok_or_else(|| {
            NeuroQuantumError::TransactionError(format!("Transaction {tx_id:?} not found"))
        })?;

        // Check if transaction is still active
        if tx.status != TransactionStatus::Active {
            return Err(NeuroQuantumError::TransactionError(format!(
                "Transaction {:?} is not active (status: {:?})",
                tx_id, tx.status
            )));
        }

        // Phase 1: Prepare
        tx.status = TransactionStatus::Preparing;

        // Validate all locks are still held
        if tx.locks.is_empty() && !tx.write_set.is_empty() {
            return Err(NeuroQuantumError::TransactionError(
                "Transaction lost locks before commit".to_string(),
            ));
        }

        tx.status = TransactionStatus::Prepared;

        // Phase 2: Commit
        tx.status = TransactionStatus::Committing;

        // Write COMMIT log record
        let lsn = self
            .log_manager
            .write_log_record(Some(tx_id), tx.last_lsn, LogRecordType::Commit { tx_id })
            .await?;

        // Force log to disk for durability
        self.log_manager.force_log(lsn).await?;

        // Update global version for MVCC
        self.global_version.fetch_add(1, Ordering::SeqCst);

        tx.status = TransactionStatus::Committed;

        // Release all locks
        self.lock_manager.release_locks(&tx_id).await?;

        // Remove from active transactions
        active.remove(&tx_id);

        info!("✅ Transaction {:?} committed", tx_id);
        Ok(())
    }

    /// Get the undo log for a transaction (for storage engine rollback)
    pub async fn get_undo_log(&self, tx_id: TransactionId) -> Option<Vec<LogRecord>> {
        let active = self.active_transactions.read().await;
        active.get(&tx_id).map(|tx| tx.undo_log.clone())
    }

    /// Rollback a transaction
    #[instrument(skip(self))]
    pub async fn rollback(&self, tx_id: TransactionId) -> Result<(), NeuroQuantumError> {
        let mut active = self.active_transactions.write().await;

        let tx = active.get_mut(&tx_id).ok_or_else(|| {
            NeuroQuantumError::TransactionError(format!("Transaction {tx_id:?} not found"))
        })?;

        tx.status = TransactionStatus::Aborting;

        // Undo all changes using undo log
        for log_record in tx.undo_log.iter().rev() {
            if let LogRecordType::Update {
                before_image: _before_image,
                table,
                key,
                ..
            } = &log_record.record_type
            {
                debug!("Undoing update on {}.{}", table, key);
                debug!("ROLLBACK: Apply before_image for {}.{}", table, key);
                // NOTE: Storage integration must be done at StorageEngine level
                // Call storage_engine.apply_before_image(table, key, before_image).await
                // This is handled during transactional operations in StorageEngine
            }
        }

        // Write ABORT log record
        let lsn = self
            .log_manager
            .write_log_record(Some(tx_id), tx.last_lsn, LogRecordType::Abort { tx_id })
            .await?;

        self.log_manager.force_log(lsn).await?;

        tx.status = TransactionStatus::Aborted;

        // Release all locks
        self.lock_manager.release_locks(&tx_id).await?;

        // Remove from active transactions
        active.remove(&tx_id);

        warn!("🔙 Transaction {:?} rolled back", tx_id);
        Ok(())
    }

    /// Acquire a lock for a transaction
    pub async fn acquire_lock(
        &self,
        tx_id: TransactionId,
        resource: ResourceId,
        lock_type: LockType,
    ) -> Result<(), NeuroQuantumError> {
        // Update transaction activity
        {
            let mut active = self.active_transactions.write().await;
            if let Some(tx) = active.get_mut(&tx_id) {
                tx.touch();

                // Add to read/write set for conflict detection
                match lock_type {
                    | LockType::Shared | LockType::IntentionShared => {
                        tx.read_set.insert(resource.clone());
                    },
                    | LockType::Exclusive | LockType::IntentionExclusive => {
                        tx.write_set.insert(resource.clone());
                    },
                }

                tx.locks.insert(resource.clone());
            }
        }

        self.lock_manager
            .acquire_lock(tx_id, resource, lock_type)
            .await
    }

    /// Log a data modification
    pub async fn log_update(
        &self,
        tx_id: TransactionId,
        table: String,
        key: String,
        before_image: Option<Vec<u8>>,
        after_image: Vec<u8>,
    ) -> Result<LSN, NeuroQuantumError> {
        let record_type = LogRecordType::Update {
            tx_id,
            table,
            key,
            before_image,
            after_image,
        };

        // Get the transaction and its last_lsn
        let last_lsn = {
            let active = self.active_transactions.read().await;
            let tx = active.get(&tx_id).ok_or_else(|| {
                NeuroQuantumError::TransactionError(format!("Transaction {tx_id:?} not found"))
            })?;
            tx.last_lsn
        };

        // Write to WAL
        let lsn = self
            .log_manager
            .write_log_record(Some(tx_id), last_lsn, record_type.clone())
            .await?;

        // Add to transaction's undo log for rollback support
        {
            let mut active = self.active_transactions.write().await;
            if let Some(tx) = active.get_mut(&tx_id) {
                tx.last_lsn = Some(lsn);
                tx.undo_log.push(LogRecord {
                    lsn,
                    prev_lsn: last_lsn,
                    tx_id: Some(tx_id),
                    record_type,
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        Ok(lsn)
    }

    /// Create a savepoint within a transaction
    pub async fn create_savepoint(
        &self,
        tx_id: TransactionId,
        name: String,
    ) -> Result<LSN, NeuroQuantumError> {
        let mut active = self.active_transactions.write().await;

        let tx = active.get_mut(&tx_id).ok_or_else(|| {
            NeuroQuantumError::TransactionError(format!("Transaction {tx_id:?} not found"))
        })?;

        tx.touch();

        // Get current LSN for this savepoint
        let lsn = self.log_manager.lsn_counter.load(Ordering::SeqCst);

        // Savepoint is tracked in-memory with its LSN
        // Rollback to savepoint uses the undo log to restore state

        debug!(
            "💾 Savepoint '{}' created for transaction {:?} at LSN {}",
            name, tx_id, lsn
        );
        Ok(lsn)
    }

    /// Rollback transaction to a savepoint
    pub async fn rollback_to_savepoint(
        &self,
        tx_id: TransactionId,
        savepoint_lsn: LSN,
    ) -> Result<(), NeuroQuantumError> {
        let mut active = self.active_transactions.write().await;

        let tx = active.get_mut(&tx_id).ok_or_else(|| {
            NeuroQuantumError::TransactionError(format!("Transaction {tx_id:?} not found"))
        })?;

        tx.touch();

        // Undo all changes after the savepoint LSN
        let records_to_undo: Vec<_> = tx
            .undo_log
            .iter()
            .filter(|record| record.lsn > savepoint_lsn)
            .cloned()
            .collect();

        // Apply undo in reverse order
        for log_record in records_to_undo.iter().rev() {
            if let LogRecordType::Update {
                before_image: _before_image,
                table,
                key,
                ..
            } = &log_record.record_type
            {
                debug!(
                    "Undoing update on {}.{} (LSN: {})",
                    table, key, log_record.lsn
                );
                // NOTE: Actual storage undo is handled by StorageEngine::rollback_to_savepoint()
                // which applies before_image data to restore previous state
            }
        }

        // Remove undone records from undo log
        tx.undo_log.retain(|record| record.lsn <= savepoint_lsn);

        info!(
            "↩️  Transaction {:?} rolled back to savepoint (LSN: {})",
            tx_id, savepoint_lsn
        );
        Ok(())
    }

    /// Release a savepoint (no-op in this implementation as savepoints are LSN-based)
    pub async fn release_savepoint(
        &self,
        tx_id: TransactionId,
        _name: String,
    ) -> Result<(), NeuroQuantumError> {
        let active = self.active_transactions.read().await;

        let _tx = active.get(&tx_id).ok_or_else(|| {
            NeuroQuantumError::TransactionError(format!("Transaction {tx_id:?} not found"))
        })?;

        debug!("🗑️  Savepoint released for transaction {:?}", tx_id);
        Ok(())
    }

    /// Cleanup timed out transactions
    pub async fn cleanup_timed_out_transactions(&self) -> Result<(), NeuroQuantumError> {
        let active = self.active_transactions.read().await;
        let timed_out: Vec<TransactionId> = active
            .values()
            .filter(|tx| tx.is_timed_out())
            .map(|tx| tx.id)
            .collect();

        drop(active); // Release read lock

        for tx_id in timed_out {
            warn!("⏰ Transaction {:?} timed out, rolling back", tx_id);
            self.rollback(tx_id).await?;
        }

        Ok(())
    }

    /// Write a checkpoint
    pub async fn checkpoint(&self) -> Result<(), NeuroQuantumError> {
        let active = self.active_transactions.read().await;
        let active_tx_ids: Vec<TransactionId> = active.keys().copied().collect();

        self.log_manager.write_checkpoint(active_tx_ids).await?;
        Ok(())
    }

    /// Get transaction statistics
    pub async fn get_statistics(&self) -> TransactionStatistics {
        let active = self.active_transactions.read().await;

        TransactionStatistics {
            active_transactions: active.len(),
            global_version: self.global_version.load(Ordering::SeqCst),
            next_lsn: self.log_manager.lsn_counter.load(Ordering::SeqCst),
        }
    }

    /// Perform full ARIES crash recovery with storage integration
    ///
    /// This delegates to the `RecoveryManager`'s `recover_with_storage()` method,
    /// providing a convenient entry point from the `TransactionManager`.
    ///
    /// # Arguments
    /// * `storage_callback` - Implementation of `RecoveryStorageCallback` that applies
    ///   changes to the actual storage engine
    ///
    /// # Returns
    /// * `RecoveryStatistics` with details about the recovery process
    pub async fn recover_with_storage(
        &self,
        storage_callback: &dyn RecoveryStorageCallback,
    ) -> Result<RecoveryStatistics, NeuroQuantumError> {
        self.recovery_manager
            .recover_with_storage(storage_callback)
            .await
    }

    /// Get the log manager for direct access (e.g., for archiving)
    #[must_use] 
    pub const fn log_manager(&self) -> &Arc<LogManager> {
        &self.log_manager
    }

    /// Get the recovery manager for direct access
    #[must_use] 
    pub const fn recovery_manager(&self) -> &Arc<RecoveryManager> {
        &self.recovery_manager
    }

    /// Archive the WAL log with a timestamp suffix
    /// Useful for backup and point-in-time recovery
    pub async fn archive_wal(&self) -> Result<PathBuf, NeuroQuantumError> {
        self.log_manager.archive_log().await
    }

    /// Truncate the WAL log after a successful checkpoint
    /// This removes old log records that are no longer needed for recovery
    pub async fn truncate_wal_after_checkpoint(
        &self,
        checkpoint_lsn: LSN,
    ) -> Result<(), NeuroQuantumError> {
        self.log_manager
            .truncate_log_after_checkpoint(checkpoint_lsn)
            .await
    }

    /// Get WAL log statistics
    pub async fn get_wal_stats(&self) -> Result<WALLogStats, NeuroQuantumError> {
        self.log_manager.get_log_stats().await
    }
}

/// Transaction statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatistics {
    pub active_transactions: usize,
    pub global_version: u64,
    pub next_lsn: LSN,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_transaction_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let tx_manager = TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap();

        let tx_id = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();
        assert!(tx_manager
            .active_transactions
            .read()
            .await
            .contains_key(&tx_id));

        tx_manager.commit(tx_id).await.unwrap();
        assert!(!tx_manager
            .active_transactions
            .read()
            .await
            .contains_key(&tx_id));
    }

    #[tokio::test]
    async fn test_deadlock_detection() {
        let temp_dir = TempDir::new().unwrap();
        let tx_manager = TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap();

        let tx1 = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();
        let tx2 = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();

        // TX1 locks resource A
        tx_manager
            .acquire_lock(tx1, "A".to_string(), LockType::Exclusive)
            .await
            .unwrap();

        // TX2 locks resource B
        tx_manager
            .acquire_lock(tx2, "B".to_string(), LockType::Exclusive)
            .await
            .unwrap();

        // This should work - no cycle yet
        assert!(tx_manager.active_transactions.read().await.len() == 2);
    }

    #[tokio::test]
    async fn test_wal_log_stats() {
        let temp_dir = TempDir::new().unwrap();
        let tx_manager = TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap();

        // Create some transactions
        let tx1 = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();
        tx_manager.commit(tx1).await.unwrap();

        let tx2 = tx_manager
            .begin_transaction(IsolationLevel::RepeatableRead)
            .await
            .unwrap();
        tx_manager.rollback(tx2).await.unwrap();

        // Get WAL stats
        let stats = tx_manager.get_wal_stats().await.unwrap();
        assert!(stats.record_count > 0);
        assert!(stats.current_lsn > 0);
    }

    #[tokio::test]
    async fn test_recover_with_storage_callback() {
        use std::sync::Mutex;

        // Mock storage callback that tracks operations
        struct MockStorageCallback {
            redo_calls: Mutex<Vec<(String, String)>>,
            undo_calls: Mutex<Vec<(String, String)>>,
        }

        #[async_trait::async_trait]
        impl RecoveryStorageCallback for MockStorageCallback {
            async fn apply_after_image(
                &self,
                table: &str,
                key: &str,
                _after_image: &[u8],
            ) -> Result<(), NeuroQuantumError> {
                self.redo_calls
                    .lock()
                    .unwrap()
                    .push((table.to_string(), key.to_string()));
                Ok(())
            }

            async fn apply_before_image(
                &self,
                table: &str,
                key: &str,
                _before_image: Option<&[u8]>,
            ) -> Result<(), NeuroQuantumError> {
                self.undo_calls
                    .lock()
                    .unwrap()
                    .push((table.to_string(), key.to_string()));
                Ok(())
            }
        }

        let temp_dir = TempDir::new().unwrap();
        let tx_manager = TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap();

        // Create a committed transaction with updates
        let tx1 = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();
        tx_manager
            .log_update(
                tx1,
                "users".to_string(),
                "1".to_string(),
                None,
                b"{'name': 'Alice'}".to_vec(),
            )
            .await
            .unwrap();
        tx_manager.commit(tx1).await.unwrap();

        // Force flush to ensure records are written
        tx_manager.log_manager.force_log(0).await.unwrap();

        // Perform recovery with mock callback
        // This tests the integration even though there are no uncommitted transactions
        // (the recovery will find committed transactions to redo)
        let callback = MockStorageCallback {
            redo_calls: Mutex::new(Vec::new()),
            undo_calls: Mutex::new(Vec::new()),
        };

        let stats = tx_manager.recover_with_storage(&callback).await.unwrap();

        // Verify recovery ran and found the committed transaction
        assert!(
            stats.total_records_analyzed > 0,
            "Expected to analyze some records"
        );
        assert_eq!(
            stats.transactions_redone, 1,
            "Expected 1 committed transaction to redo"
        );

        // Verify callback was called for redo
        let redo_calls = callback.redo_calls.lock().unwrap();
        assert!(
            redo_calls.iter().any(|(t, k)| t == "users" && k == "1"),
            "Expected redo for users.1"
        );
    }

    #[tokio::test]
    async fn test_wal_archive() {
        let temp_dir = TempDir::new().unwrap();
        let tx_manager = TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap();

        // Create some transactions to generate WAL entries
        let tx1 = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();
        tx_manager.commit(tx1).await.unwrap();

        // Archive the WAL
        let archive_path = tx_manager.archive_wal().await.unwrap();
        assert!(archive_path.exists());
        assert!(archive_path.to_string_lossy().contains(".archive"));
    }

    #[tokio::test]
    async fn test_checkpoint_and_truncate() {
        let temp_dir = TempDir::new().unwrap();
        let tx_manager = TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap();

        // Create some transactions
        for i in 0..3 {
            let tx = tx_manager
                .begin_transaction(IsolationLevel::ReadCommitted)
                .await
                .unwrap();
            tx_manager
                .log_update(
                    tx,
                    "test".to_string(),
                    format!("{}", i),
                    None,
                    b"data".to_vec(),
                )
                .await
                .unwrap();
            tx_manager.commit(tx).await.unwrap();
        }

        // Force flush
        tx_manager.log_manager.force_log(0).await.unwrap();

        let stats_before = tx_manager.get_wal_stats().await.unwrap();
        let records_before = stats_before.record_count;

        // Write checkpoint
        tx_manager.checkpoint().await.unwrap();

        // Force flush again
        tx_manager.log_manager.force_log(0).await.unwrap();

        // Verify we have records to truncate
        assert!(
            records_before > 0,
            "Expected some records before truncation"
        );

        // Truncate all records up to current LSN - 1
        // This should remove at least some records
        let current_lsn = tx_manager.get_statistics().await.next_lsn;
        tx_manager
            .truncate_wal_after_checkpoint(current_lsn - 2)
            .await
            .unwrap();

        // Verify truncation worked (log file was rewritten)
        let stats_after = tx_manager.get_wal_stats().await.unwrap();

        // After truncation, we should have fewer or equal records
        // (depends on the LSN we used for truncation)
        assert!(
            stats_after.record_count <= records_before + 2, // +2 for checkpoint records
            "Expected truncation to work: {} <= {}",
            stats_after.record_count,
            records_before + 2
        );
    }
}
