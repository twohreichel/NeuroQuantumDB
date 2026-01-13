//! Incremental Backup System for NeuroQuantumDB
//!
//! Provides efficient incremental backups by only backing up changes since last backup

use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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

    /// Backup WAL segments since given LSN with proper parsing
    async fn backup_wal_since_lsn(&self, since_lsn: u64, backup_dir: &Path) -> Result<BackupStats> {
        let mut stats = BackupStats::default();
        let wal_manager = self.wal_manager.read().await;

        let wal_dir = backup_dir.join("wal");
        self.storage_backend.create_directory(&wal_dir).await?;

        // Get WAL directory
        let source_wal_dir = wal_manager.get_wal_directory();

        info!(
            "Scanning WAL segments from {} for records since LSN {}",
            source_wal_dir.display(),
            since_lsn
        );

        // List WAL segment files
        let mut entries = tokio::fs::read_dir(&source_wal_dir).await?;
        let mut processed_segments = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wal") {
                // Read and parse WAL file
                let wal_data = tokio::fs::read(&path).await?;

                // Parse WAL records to check LSN range
                match self.parse_wal_segment(&wal_data, since_lsn).await {
                    Ok(parsed_data) => {
                        if !parsed_data.is_empty() {
                            // This segment contains records after since_lsn
                            let filename =
                                path.file_name().expect("WAL file should have a filename");
                            let dest_path = wal_dir.join(filename);

                            // Write only the relevant records
                            self.storage_backend
                                .write_file(&dest_path, &parsed_data)
                                .await?;

                            stats.bytes_read += wal_data.len() as u64;
                            stats.bytes_written += parsed_data.len() as u64;
                            stats.wal_segments_backed_up += 1;
                            stats.files_backed_up += 1;

                            info!(
                                "Backed up WAL segment {:?}: {} bytes ({} records)",
                                filename,
                                parsed_data.len(),
                                self.count_records_in_segment(&parsed_data)
                                    .await
                                    .unwrap_or(0)
                            );
                        } else {
                            debug!(
                                "Skipped WAL segment {:?} - no records after LSN {}",
                                path.file_name(),
                                since_lsn
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse WAL segment {:?}: {}. Backing up entire segment.",
                            path.file_name(),
                            e
                        );

                        // Fallback: backup entire segment if parsing fails
                        let filename = path.file_name().expect("WAL file should have a filename");
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

                processed_segments += 1;
            }
        }

        info!(
            "Processed {} WAL segments, backed up {} segments",
            processed_segments, stats.wal_segments_backed_up
        );

        Ok(stats)
    }

    /// Parse WAL segment and filter records by LSN
    async fn parse_wal_segment(&self, data: &[u8], since_lsn: u64) -> Result<Vec<u8>> {
        use crate::storage::wal::WALRecord;
        use bincode;

        let mut filtered_records = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            // Try to deserialize a WAL record
            match bincode::deserialize::<WALRecord>(&data[offset..]) {
                Ok(record) => {
                    // Check if this record is after since_lsn
                    if record.lsn > since_lsn {
                        // Serialize and add to filtered records
                        if let Ok(serialized) = bincode::serialize(&record) {
                            filtered_records.extend_from_slice(&serialized);
                        }
                    }

                    // Calculate record size and move offset
                    if let Ok(serialized) = bincode::serialize(&record) {
                        offset += serialized.len();
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    // End of valid records or corrupted data
                    break;
                }
            }
        }

        Ok(filtered_records)
    }

    /// Count the number of records in a WAL segment
    async fn count_records_in_segment(&self, data: &[u8]) -> Result<usize> {
        use crate::storage::wal::WALRecord;
        use bincode;

        let mut count = 0;
        let mut offset = 0;

        while offset < data.len() {
            match bincode::deserialize::<WALRecord>(&data[offset..]) {
                Ok(record) => {
                    count += 1;
                    if let Ok(serialized) = bincode::serialize(&record) {
                        offset += serialized.len();
                    } else {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        Ok(count)
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
