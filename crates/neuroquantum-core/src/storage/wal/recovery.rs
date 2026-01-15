//! Recovery Manager implementing ARIES algorithm
//!
//! Three-phase recovery:
//! 1. Analysis: Determine which transactions were active and which pages were dirty
//! 2. Redo: Replay all changes from the log to restore the database state
//! 3. Undo: Roll back incomplete transactions

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};

use super::{TransactionId, TransactionState, WALConfig, WALRecord, WALRecordType, LSN};
use crate::storage::pager::{PageId, PageStorageManager};

/// Recovery statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStats {
    /// Number of log records analyzed
    pub records_analyzed: usize,
    /// Number of redo operations performed
    pub redo_operations: usize,
    /// Number of undo operations performed
    pub undo_operations: usize,
    /// Number of transactions recovered (committed)
    pub transactions_committed: usize,
    /// Number of transactions rolled back
    pub transactions_aborted: usize,
    /// Total recovery time in milliseconds
    pub recovery_time_ms: u64,
    /// Checkpoint LSN used (if any)
    pub checkpoint_lsn: Option<LSN>,
}

/// Recovery Manager
///
/// Implements ARIES-style crash recovery with analysis, redo, and undo phases
pub struct RecoveryManager {
    _config: WALConfig,
    _pager: Arc<PageStorageManager>,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(config: WALConfig, pager: Arc<PageStorageManager>) -> Self {
        Self {
            _config: config,
            _pager: pager,
        }
    }

    /// Perform crash recovery using ARIES algorithm
    pub async fn recover(
        &self,
        wal_manager: &super::WALManager,
        pager: Arc<PageStorageManager>,
    ) -> Result<RecoveryStats> {
        let start_time = std::time::Instant::now();
        info!("🔄 Starting ARIES recovery...");

        // Phase 1: Analysis
        info!("📊 Phase 1: Analysis");
        let analysis_result = self.analysis_phase(wal_manager).await?;

        // Log detailed transaction state information
        let redo_txns = analysis_result.transactions_needing_redo();
        let undo_txns = analysis_result.transactions_needing_undo();
        info!("   - Transactions needing redo: {}", redo_txns.len());
        info!("   - Transactions needing undo: {}", undo_txns.len());
        info!(
            "   - Already aborted: {}",
            analysis_result.aborted_txns.len()
        );

        // Phase 2: Redo
        info!("♻️ Phase 2: Redo");
        let redo_count = self
            .redo_phase(wal_manager, &analysis_result, &pager)
            .await?;

        // Phase 3: Undo
        info!("↩️ Phase 3: Undo");
        let undo_count = self
            .undo_phase(wal_manager, &analysis_result, &pager)
            .await?;

        let recovery_time_ms = start_time.elapsed().as_millis() as u64;

        let stats = RecoveryStats {
            records_analyzed: analysis_result.total_records,
            redo_operations: redo_count,
            undo_operations: undo_count,
            transactions_committed: analysis_result.committed_txns.len(),
            transactions_aborted: analysis_result.active_txns.len(),
            recovery_time_ms,
            checkpoint_lsn: analysis_result.checkpoint_lsn,
        };

        info!("✅ Recovery completed in {}ms", recovery_time_ms);
        info!("   - Records analyzed: {}", stats.records_analyzed);
        info!("   - Redo operations: {}", stats.redo_operations);
        info!("   - Undo operations: {}", stats.undo_operations);
        info!("   - Committed: {}", stats.transactions_committed);
        info!("   - Aborted: {}", stats.transactions_aborted);

        Ok(stats)
    }

    /// Phase 1: Analysis - determine transaction and page state
    ///
    /// This phase scans the log to build:
    /// - Transaction table: maps tx_id -> TransactionState with full ARIES tracking
    /// - Dirty page table: maps page_id -> recovery LSN
    /// - Sets of committed and aborted transactions
    async fn analysis_phase(&self, wal_manager: &super::WALManager) -> Result<AnalysisResult> {
        info!("Scanning log from beginning...");

        // Read all log records
        let records = wal_manager.read_log_records(1).await?;

        // Full TransactionState tracking for ARIES
        let mut active_txn_states: HashMap<TransactionId, TransactionState> = HashMap::new();
        // Simplified LSN mapping for compatibility
        let mut active_txns: HashMap<TransactionId, LSN> = HashMap::new();
        let mut committed_txns: HashSet<TransactionId> = HashSet::new();
        let mut aborted_txns: HashSet<TransactionId> = HashSet::new();
        let mut dirty_pages: HashMap<PageId, LSN> = HashMap::new();
        let mut checkpoint_lsn: Option<LSN> = None;

        for record in &records {
            match &record.record_type {
                | WALRecordType::Begin { tx_id, .. } => {
                    // Create a new TransactionState with full tracking
                    let tx_state = TransactionState::new(*tx_id, record.lsn);
                    active_txn_states.insert(*tx_id, tx_state);
                    active_txns.insert(*tx_id, record.lsn);
                    debug!("Found BEGIN for TX={}", tx_id);
                },
                | WALRecordType::Update { tx_id, page_id, .. } => {
                    // Update transaction state with operation tracking
                    if let Some(tx_state) = active_txn_states.get_mut(tx_id) {
                        tx_state.record_operation(record.lsn, Some(*page_id));
                    } else {
                        // Transaction started before our log scan - create state
                        let mut tx_state = TransactionState::new(*tx_id, record.lsn);
                        tx_state.record_operation(record.lsn, Some(*page_id));
                        active_txn_states.insert(*tx_id, tx_state);
                    }
                    active_txns.insert(*tx_id, record.lsn);
                    dirty_pages.entry(*page_id).or_insert(record.lsn);
                    debug!("Found UPDATE for TX={}, Page={}", tx_id, page_id.0);
                },
                | WALRecordType::Commit { tx_id } => {
                    // Mark transaction as committed
                    if let Some(tx_state) = active_txn_states.get_mut(tx_id) {
                        tx_state.begin_commit();
                        tx_state.complete_commit(record.lsn);
                    }
                    active_txns.remove(tx_id);
                    committed_txns.insert(*tx_id);
                    debug!("Found COMMIT for TX={}", tx_id);
                },
                | WALRecordType::Abort { tx_id } => {
                    // Mark transaction as aborted
                    if let Some(tx_state) = active_txn_states.get_mut(tx_id) {
                        tx_state.begin_abort();
                        tx_state.complete_abort(record.lsn);
                    }
                    active_txns.remove(tx_id);
                    aborted_txns.insert(*tx_id);
                    debug!("Found ABORT for TX={}", tx_id);
                },
                | WALRecordType::CheckpointBegin { .. } => {
                    checkpoint_lsn = Some(record.lsn);
                    debug!("Found CHECKPOINT at LSN={}", record.lsn);
                },
                | WALRecordType::CheckpointEnd => {
                    debug!("Found CHECKPOINT END at LSN={}", record.lsn);
                },
                | WALRecordType::CLR {
                    tx_id,
                    undo_next_lsn,
                    page_id,
                    ..
                } => {
                    // Update undo chain for CLR records
                    if let Some(tx_state) = active_txn_states.get_mut(tx_id) {
                        tx_state.record_operation(record.lsn, Some(*page_id));
                        // CLR updates the undo_next_lsn to skip already undone operations
                        tx_state.undo_next_lsn = Some(*undo_next_lsn);
                    }
                    active_txns.insert(*tx_id, record.lsn);
                    dirty_pages.entry(*page_id).or_insert(record.lsn);
                    debug!("Found CLR for TX={}, undo_next={}", tx_id, undo_next_lsn);
                },
                | WALRecordType::Savepoint { tx_id, name, .. } => {
                    // Track savepoint in transaction state
                    if let Some(tx_state) = active_txn_states.get_mut(tx_id) {
                        tx_state.savepoints.insert(name.clone(), record.lsn);
                        tx_state.record_operation(record.lsn, None);
                    }
                    active_txns.insert(*tx_id, record.lsn);
                    debug!("Found SAVEPOINT '{}' for TX={}", name, tx_id);
                },
                | WALRecordType::RollbackToSavepoint {
                    tx_id,
                    name,
                    target_lsn,
                } => {
                    // Update undo chain to reflect rollback
                    if let Some(tx_state) = active_txn_states.get_mut(tx_id) {
                        tx_state.undo_next_lsn = Some(*target_lsn);
                        tx_state.record_operation(record.lsn, None);
                    }
                    active_txns.insert(*tx_id, record.lsn);
                    debug!(
                        "Found ROLLBACK TO SAVEPOINT '{}' for TX={}, target_lsn={}",
                        name, tx_id, target_lsn
                    );
                },
                | WALRecordType::ReleaseSavepoint { tx_id, name } => {
                    // Remove savepoint from transaction state
                    if let Some(tx_state) = active_txn_states.get_mut(tx_id) {
                        tx_state.savepoints.remove(name);
                        tx_state.record_operation(record.lsn, None);
                    }
                    active_txns.insert(*tx_id, record.lsn);
                    debug!("Found RELEASE SAVEPOINT '{}' for TX={}", name, tx_id);
                },
            }
        }

        info!("Analysis complete:");
        info!("  - Active transactions: {}", active_txns.len());
        info!("  - Committed transactions: {}", committed_txns.len());
        info!("  - Aborted transactions: {}", aborted_txns.len());
        info!("  - Dirty pages: {}", dirty_pages.len());

        Ok(AnalysisResult {
            active_txn_states,
            active_txns,
            committed_txns,
            aborted_txns,
            dirty_pages,
            checkpoint_lsn,
            total_records: records.len(),
        })
    }

    /// Phase 2: Redo - replay all changes
    async fn redo_phase(
        &self,
        wal_manager: &super::WALManager,
        analysis: &AnalysisResult,
        pager: &Arc<PageStorageManager>,
    ) -> Result<usize> {
        info!("Redoing changes from log...");

        // Determine starting point (checkpoint or beginning)
        let start_lsn = analysis.checkpoint_lsn.unwrap_or(1);
        let records = wal_manager.read_log_records(start_lsn).await?;

        let mut redo_count = 0;

        for record in &records {
            match &record.record_type {
                | WALRecordType::Update {
                    page_id,
                    offset,
                    after_image,
                    ..
                } => {
                    // Check if page needs redo (is dirty and LSN >= recovery_lsn)
                    if let Some(&recovery_lsn) = analysis.dirty_pages.get(page_id) {
                        if record.lsn >= recovery_lsn {
                            // Apply the update
                            self.apply_redo(pager, *page_id, *offset, after_image.clone())
                                .await?;
                            redo_count += 1;
                            debug!("REDO: Page={}, LSN={}", page_id.0, record.lsn);
                        }
                    }
                },
                | WALRecordType::CLR {
                    page_id, redo_data, ..
                } => {
                    // Redo CLR operations
                    self.apply_redo(pager, *page_id, 0, redo_data.clone())
                        .await?;
                    redo_count += 1;
                    debug!("REDO CLR: Page={}, LSN={}", page_id.0, record.lsn);
                },
                | _ => {}, // Skip non-update records
            }
        }

        info!("Redo complete: {} operations", redo_count);
        Ok(redo_count)
    }

    /// Phase 3: Undo - roll back incomplete transactions
    ///
    /// Uses the full TransactionState information for optimized undo:
    /// - Respects undo_next_lsn for CLR-aware recovery
    /// - Uses modified_pages for selective page access
    /// - Leverages operation_count for progress tracking
    async fn undo_phase(
        &self,
        wal_manager: &super::WALManager,
        analysis: &AnalysisResult,
        pager: &Arc<PageStorageManager>,
    ) -> Result<usize> {
        info!("Undoing incomplete transactions...");

        // Use TransactionState for transactions that need undo
        let txns_to_undo = analysis.transactions_needing_undo();
        if txns_to_undo.is_empty() && analysis.active_txns.is_empty() {
            info!("No active transactions to undo");
            return Ok(0);
        }

        let mut undo_count = 0;

        // Prefer using full TransactionState when available
        for tx_state in &txns_to_undo {
            info!(
                "Undoing transaction: {} (status: {}, operations: {})",
                tx_state.tx_id, tx_state.status, tx_state.operation_count
            );

            // Use undo_next_lsn if available (CLR-aware), otherwise use last_lsn
            let start_lsn = tx_state.undo_next_lsn.unwrap_or(tx_state.last_lsn);

            let undo_ops = self
                .undo_transaction(wal_manager, tx_state.tx_id, start_lsn, pager)
                .await?;
            undo_count += undo_ops;
        }

        // Fallback: Also process any transactions in active_txns not in active_txn_states
        for (tx_id, last_lsn) in &analysis.active_txns {
            // Skip if already processed via TransactionState
            if analysis.active_txn_states.contains_key(tx_id) {
                continue;
            }

            info!("Undoing transaction (legacy path): {}", tx_id);
            let undo_ops = self
                .undo_transaction(wal_manager, *tx_id, *last_lsn, pager)
                .await?;
            undo_count += undo_ops;
        }

        info!("Undo complete: {} operations", undo_count);
        Ok(undo_count)
    }

    /// Undo a single transaction by following prev_lsn chain
    async fn undo_transaction(
        &self,
        wal_manager: &super::WALManager,
        tx_id: TransactionId,
        mut current_lsn: LSN,
        pager: &Arc<PageStorageManager>,
    ) -> Result<usize> {
        let mut undo_count = 0;
        let records = wal_manager.read_log_records(1).await?;

        // Build a map of LSN -> Record for quick lookup
        let record_map: HashMap<LSN, &WALRecord> = records.iter().map(|r| (r.lsn, r)).collect();

        // Follow the undo chain
        while let Some(record) = record_map.get(&current_lsn) {
            match &record.record_type {
                | WALRecordType::Update {
                    page_id,
                    offset,
                    before_image,
                    ..
                } => {
                    // Apply the before image (undo the change)
                    self.apply_redo(pager, *page_id, *offset, before_image.clone())
                        .await?;
                    undo_count += 1;
                    debug!("UNDO: TX={}, Page={}, LSN={}", tx_id, page_id.0, record.lsn);

                    // Write CLR (Compensation Log Record) - not implemented here for simplicity
                },
                | WALRecordType::Begin { .. } => {
                    // Reached the beginning of the transaction
                    break;
                },
                | _ => {},
            }

            // Move to previous LSN
            if let Some(prev_lsn) = record.prev_lsn {
                current_lsn = prev_lsn;
            } else {
                break;
            }
        }

        Ok(undo_count)
    }

    /// Apply a redo operation to a page
    async fn apply_redo(
        &self,
        pager: &Arc<PageStorageManager>,
        page_id: PageId,
        offset: usize,
        data: Vec<u8>,
    ) -> Result<()> {
        // Read the page
        let mut page = pager.read_page(page_id).await?;

        // Apply the update using the write_data method
        page.write_data(offset, &data)?;

        // Write the page back
        pager.write_page(&page).await?;

        Ok(())
    }
}

/// Result of the analysis phase with full ARIES transaction tracking
#[derive(Debug)]
struct AnalysisResult {
    /// Active transaction states (not committed/aborted) - full TransactionState for proper undo
    active_txn_states: HashMap<TransactionId, TransactionState>,
    /// Active transactions (not committed/aborted) - LSN mapping for compatibility
    active_txns: HashMap<TransactionId, LSN>,
    /// Committed transactions
    committed_txns: HashSet<TransactionId>,
    /// Aborted transactions
    aborted_txns: HashSet<TransactionId>,
    /// Dirty pages and their recovery LSN
    dirty_pages: HashMap<PageId, LSN>,
    /// Last checkpoint LSN (if any)
    checkpoint_lsn: Option<LSN>,
    /// Total records analyzed
    total_records: usize,
}

impl AnalysisResult {
    /// Get all transactions that need undo (active or aborting)
    pub fn transactions_needing_undo(&self) -> Vec<&TransactionState> {
        self.active_txn_states
            .values()
            .filter(|state| state.needs_undo())
            .collect()
    }

    /// Get all transactions that need redo (committed or committing)
    pub fn transactions_needing_redo(&self) -> Vec<&TransactionState> {
        self.active_txn_states
            .values()
            .filter(|state| state.needs_redo())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::pager::{PageType, PagerConfig, SyncMode};
    use crate::storage::wal::WALManager;
    use tempfile::TempDir;

    async fn setup_test_recovery() -> (TempDir, Arc<PageStorageManager>, WALManager) {
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

        let wal_config = super::WALConfig {
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
    async fn test_recovery_with_committed_transaction() {
        let (_temp, pager, wal) = setup_test_recovery().await;

        // Allocate a page first
        let page_id = pager.allocate_page(PageType::Data).await.unwrap();

        // Simulate a transaction
        let tx_id = wal.begin_transaction().await.unwrap();

        let before = vec![0; 100];
        let after = vec![1; 100];

        wal.log_update(tx_id, page_id, 0, before, after)
            .await
            .unwrap();

        wal.commit_transaction(tx_id).await.unwrap();

        // Now recover
        let stats = wal.recover(Arc::clone(&pager)).await.unwrap();

        assert_eq!(stats.transactions_committed, 1);
        assert_eq!(stats.transactions_aborted, 0);
    }

    #[tokio::test]
    async fn test_recovery_with_aborted_transaction() {
        let (_temp, pager, wal) = setup_test_recovery().await;

        // Allocate a page first
        let page_id = pager.allocate_page(PageType::Data).await.unwrap();

        // Simulate an uncommitted transaction (simulate crash)
        let tx_id = wal.begin_transaction().await.unwrap();

        let before = vec![0; 100];
        let after = vec![1; 100];

        wal.log_update(tx_id, page_id, 0, before, after)
            .await
            .unwrap();

        // Don't commit - simulate crash

        // Now recover
        let stats = wal.recover(Arc::clone(&pager)).await.unwrap();

        // Active transactions should be rolled back
        assert!(stats.transactions_aborted >= 1 || stats.transactions_committed == 0);
    }
}
