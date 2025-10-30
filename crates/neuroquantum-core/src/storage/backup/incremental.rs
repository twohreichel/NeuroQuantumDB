//! Incremental Backup System for NeuroQuantumDB
//!
//! Provides efficient incremental backups by only backing up changes since last backup

use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::{BackupMetadata, BackupStats, BackupStorageBackend};
use crate::storage::pager::{PageId, PageStorageManager};
use crate::storage::wal::WALManager;

/// Incremental backup manager
pub struct IncrementalBackup {
    pager: Arc<RwLock<PageStorageManager>>,
    wal_manager: Arc<RwLock<WALManager>>,
    storage_backend: Arc<dyn BackupStorageBackend>,
}

impl IncrementalBackup {
    /// Create a new incremental backup manager
    pub fn new(
        pager: Arc<RwLock<PageStorageManager>>,
        wal_manager: Arc<RwLock<WALManager>>,
        storage_backend: Arc<dyn BackupStorageBackend>,
    ) -> Self {
        Self {
            pager,
            wal_manager,
            storage_backend,
        }
    }

    /// Backup all pages modified since given LSN
    pub async fn backup_since_lsn(
        &self,
        since_lsn: u64,
        metadata: &BackupMetadata,
    ) -> Result<BackupStats> {
        let start = std::time::Instant::now();
        let mut stats = BackupStats::default();

        info!("Starting incremental backup since LSN {}", since_lsn);

        // Get modified pages from WAL
        let modified_pages = self.get_modified_pages_since_lsn(since_lsn).await?;
        info!("Found {} modified pages", modified_pages.len());

        // Create backup directory
        let backup_dir = PathBuf::from(&metadata.storage_location)
            .join(format!("backup_{}", metadata.backup_id));
        self.storage_backend.create_directory(&backup_dir).await?;

        let data_dir = backup_dir.join("data");
        self.storage_backend.create_directory(&data_dir).await?;

        // Backup modified pages
        let pager = self.pager.read().await;

        for page_id in modified_pages {
            if let Ok(page) = pager.read_page(page_id).await {
                let page_bytes = page.serialize()?;

                // Save individual page file
                let page_file = data_dir.join(format!("page_{:016x}.dat", page_id.0));
                self.storage_backend
                    .write_file(&page_file, &page_bytes)
                    .await?;

                stats.bytes_read += page_bytes.len() as u64;
                stats.bytes_written += page_bytes.len() as u64;
                stats.pages_backed_up += 1;
                stats.files_backed_up += 1;
            }
        }

        // Also backup WAL segments since last backup
        let wal_stats = self.backup_wal_since_lsn(since_lsn, &backup_dir).await?;
        stats.bytes_read += wal_stats.bytes_read;
        stats.bytes_written += wal_stats.bytes_written;
        stats.wal_segments_backed_up = wal_stats.wal_segments_backed_up;
        stats.files_backed_up += wal_stats.files_backed_up;

        stats.duration_ms = start.elapsed().as_millis() as u64;
        if stats.duration_ms > 0 {
            stats.throughput_mbps = (stats.bytes_written as f64 / 1024.0 / 1024.0)
                / (stats.duration_ms as f64 / 1000.0);
        }

        info!(
            "Incremental backup completed: {} pages, {} bytes, {} ms",
            stats.pages_backed_up, stats.bytes_written, stats.duration_ms
        );

        Ok(stats)
    }

    /// Get list of pages modified since given LSN
    async fn get_modified_pages_since_lsn(&self, since_lsn: u64) -> Result<HashSet<PageId>> {
        let mut modified_pages = HashSet::new();
        let wal_manager = self.wal_manager.read().await;

        // Get all WAL records since the given LSN
        let records = wal_manager.get_records_since_lsn(since_lsn).await?;

        for record in records {
            // Extract page ID from update records
            match &record.record_type {
                crate::storage::wal::WALRecordType::Update { page_id, .. } => {
                    modified_pages.insert(*page_id);
                }
                crate::storage::wal::WALRecordType::CLR { page_id, .. } => {
                    modified_pages.insert(*page_id);
                }
                _ => {
                    // Ignore other record types
                }
            }
        }

        Ok(modified_pages)
    }

    /// Backup WAL segments since given LSN
    async fn backup_wal_since_lsn(
        &self,
        _since_lsn: u64,
        backup_dir: &PathBuf,
    ) -> Result<BackupStats> {
        let mut stats = BackupStats::default();
        let wal_manager = self.wal_manager.read().await;

        let wal_dir = backup_dir.join("wal");
        self.storage_backend.create_directory(&wal_dir).await?;

        // Get WAL directory
        let source_wal_dir = wal_manager.get_wal_directory();

        // List WAL segment files
        let mut entries = tokio::fs::read_dir(&source_wal_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wal") {
                // Read WAL file to check if it contains records after since_lsn
                let wal_data = tokio::fs::read(&path).await?;

                // Simplified: backup all WAL files
                // In production, would parse and check LSN ranges
                let filename = path.file_name().unwrap();
                let dest_path = wal_dir.join(filename);

                self.storage_backend
                    .write_file(&dest_path, &wal_data)
                    .await?;

                stats.bytes_read += wal_data.len() as u64;
                stats.bytes_written += wal_data.len() as u64;
                stats.wal_segments_backed_up += 1;
                stats.files_backed_up += 1;
            }
        }

        Ok(stats)
    }
}

/// Incremental backup manager for coordinating incremental backups
pub struct IncrementalBackupManager {
    /// Last full backup LSN
    last_full_backup_lsn: u64,
    /// Last incremental backup LSN
    last_incremental_backup_lsn: Option<u64>,
}

impl IncrementalBackupManager {
    /// Create a new incremental backup manager
    pub fn new(last_full_backup_lsn: u64) -> Self {
        Self {
            last_full_backup_lsn,
            last_incremental_backup_lsn: None,
        }
    }

    /// Get LSN to backup from
    pub fn get_backup_since_lsn(&self) -> u64 {
        self.last_incremental_backup_lsn
            .unwrap_or(self.last_full_backup_lsn)
    }

    /// Update last incremental backup LSN
    pub fn update_last_backup_lsn(&mut self, lsn: u64) {
        self.last_incremental_backup_lsn = Some(lsn);
    }

    /// Reset to full backup
    pub fn reset(&mut self, full_backup_lsn: u64) {
        self.last_full_backup_lsn = full_backup_lsn;
        self.last_incremental_backup_lsn = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_backup_manager() {
        let mut manager = IncrementalBackupManager::new(100);
        assert_eq!(manager.get_backup_since_lsn(), 100);

        manager.update_last_backup_lsn(200);
        assert_eq!(manager.get_backup_since_lsn(), 200);

        manager.reset(300);
        assert_eq!(manager.get_backup_since_lsn(), 300);
    }
}
