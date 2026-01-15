//! Checkpoint Manager for WAL
//!
//! Manages periodic checkpoints for fast recovery:
//! - Fuzzy checkpointing (doesn't block writes)
//! - Tracks active transactions
//! - Coordinates with buffer pool for dirty page flushing

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use super::{TransactionId, WALConfig, LSN};
use crate::storage::pager::PageId;

/// Checkpoint record metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    /// LSN of checkpoint begin
    pub checkpoint_lsn: LSN,
    /// Active transactions at checkpoint time
    pub active_transactions: Vec<TransactionId>,
    /// Transaction table: tx_id -> last_lsn
    pub transaction_table: HashMap<TransactionId, LSN>,
    /// Dirty page table: page_id -> recovery_lsn
    pub dirty_page_table: HashMap<PageId, LSN>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Checkpoint Manager
///
/// Coordinates checkpoint operations to enable fast recovery
pub struct CheckpointManager {
    config: WALConfig,
    /// Last checkpoint LSN
    last_checkpoint_lsn: Option<LSN>,
    /// Last checkpoint time
    last_checkpoint_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(config: WALConfig) -> Self {
        Self {
            config,
            last_checkpoint_lsn: None,
            last_checkpoint_time: None,
        }
    }

    /// Check if a checkpoint is needed based on time interval
    pub fn should_checkpoint(&self) -> bool {
        match self.last_checkpoint_time {
            | None => true, // Never checkpointed
            | Some(last_time) => {
                let elapsed = chrono::Utc::now()
                    .signed_duration_since(last_time)
                    .num_seconds();
                elapsed as u64 >= self.config.checkpoint_interval_secs
            },
        }
    }

    /// Record that a checkpoint was completed
    pub fn record_checkpoint(&mut self, checkpoint_lsn: LSN) {
        self.last_checkpoint_lsn = Some(checkpoint_lsn);
        self.last_checkpoint_time = Some(chrono::Utc::now());
        info!("✅ Checkpoint recorded: LSN={}", checkpoint_lsn);
    }

    /// Get the last checkpoint LSN
    pub fn get_last_checkpoint_lsn(&self) -> Option<LSN> {
        self.last_checkpoint_lsn
    }

    /// Create a checkpoint record
    pub fn create_checkpoint_record(
        checkpoint_lsn: LSN,
        active_transactions: Vec<TransactionId>,
        transaction_table: HashMap<TransactionId, LSN>,
        dirty_page_table: HashMap<PageId, LSN>,
    ) -> CheckpointRecord {
        CheckpointRecord {
            checkpoint_lsn,
            active_transactions,
            transaction_table,
            dirty_page_table,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Serialize checkpoint to bytes for storage
    pub fn serialize_checkpoint(record: &CheckpointRecord) -> Result<Vec<u8>> {
        Ok(bincode::serialize(record)?)
    }

    /// Deserialize checkpoint from bytes
    pub fn deserialize_checkpoint(data: &[u8]) -> Result<CheckpointRecord> {
        Ok(bincode::deserialize(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn test_should_checkpoint() {
        let config = WALConfig {
            wal_dir: PathBuf::from("test"),
            segment_size: 1024,
            sync_on_write: false,
            buffer_size: 1024,
            checkpoint_interval_secs: 5,
            min_segments_to_keep: 2,
        };

        let manager = CheckpointManager::new(config);
        assert!(manager.should_checkpoint()); // Never checkpointed
    }

    #[test]
    fn test_checkpoint_record_serialization() {
        let tx_id = Uuid::new_v4();
        let mut tx_table = HashMap::new();
        tx_table.insert(tx_id, 100);

        let mut dirty_pages = HashMap::new();
        dirty_pages.insert(PageId(1), 50);

        let record =
            CheckpointManager::create_checkpoint_record(1000, vec![tx_id], tx_table, dirty_pages);

        let bytes = CheckpointManager::serialize_checkpoint(&record).unwrap();
        let deserialized = CheckpointManager::deserialize_checkpoint(&bytes).unwrap();

        assert_eq!(record.checkpoint_lsn, deserialized.checkpoint_lsn);
        assert_eq!(record.active_transactions, deserialized.active_transactions);
    }
}
