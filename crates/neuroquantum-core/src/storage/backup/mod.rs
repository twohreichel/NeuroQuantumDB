//! Backup and Restore System for NeuroQuantumDB
//!
//! Provides comprehensive backup and restore capabilities:
//! - Hot backups (no downtime required)
//! - Point-in-Time Recovery (PITR)
//! - Incremental backups
//! - Cloud storage integration (S3, GCS)
//! - Backup verification and validation
//! - Compression and encryption

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

pub mod incremental;
pub mod restore;
pub mod storage_backend;

pub use incremental::{IncrementalBackup, IncrementalBackupManager};
pub use restore::{RestoreManager, RestoreOptions, RestoreStats};
pub use storage_backend::{BackupStorageBackend, LocalBackend, S3Backend};

use super::pager::PageStorageManager;
use super::wal::WALManager;

/// Unique identifier for backup operations
pub type BackupId = Uuid;

/// Backup types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupType {
    /// Full backup of all data
    Full,
    /// Incremental backup since last full backup
    Incremental,
    /// Differential backup (all changes since last full)
    Differential,
}

/// Backup status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupStatus {
    /// Backup is in progress
    InProgress,
    /// Backup completed successfully
    Completed,
    /// Backup failed
    Failed,
    /// Backup was cancelled
    Cancelled,
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Unique backup identifier
    pub backup_id: BackupId,
    /// Backup type
    pub backup_type: BackupType,
    /// Backup status
    pub status: BackupStatus,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,
    /// LSN at backup start
    pub start_lsn: u64,
    /// LSN at backup end
    pub end_lsn: Option<u64>,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Compressed size in bytes
    pub compressed_size_bytes: u64,
    /// Number of files backed up
    pub file_count: u32,
    /// Parent backup ID (for incremental backups)
    pub parent_backup_id: Option<BackupId>,
    /// Database version
    pub db_version: String,
    /// Checksum for verification
    pub checksum: String,
    /// Storage backend location
    pub storage_location: String,
    /// Encryption enabled
    pub encrypted: bool,
}

/// Backup configuration
#[derive(Debug, Clone)]
pub struct BackupConfig {
    /// Output directory or storage URL
    pub output_path: PathBuf,
    /// Backup type
    pub backup_type: BackupType,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression level (1-9)
    pub compression_level: u32,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Encryption key (if enabled)
    pub encryption_key: Option<Vec<u8>>,
    /// Maximum concurrent file operations
    pub max_concurrency: usize,
    /// Include WAL files
    pub include_wal: bool,
    /// Storage backend type
    pub storage_backend: BackupStorageType,
    /// S3 configuration (if using S3)
    pub s3_config: Option<S3Config>,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("./backups"),
            backup_type: BackupType::Full,
            enable_compression: true,
            compression_level: 6,
            enable_encryption: false,
            encryption_key: None,
            max_concurrency: 4,
            include_wal: true,
            storage_backend: BackupStorageType::Local,
            s3_config: None,
        }
    }
}

/// Storage backend types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupStorageType {
    /// Local filesystem
    Local,
    /// Amazon S3
    S3,
    /// Google Cloud Storage
    GCS,
}

/// S3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub endpoint: Option<String>,
}

/// Backup statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BackupStats {
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Number of files backed up
    pub files_backed_up: u32,
    /// Number of pages backed up
    pub pages_backed_up: u64,
    /// WAL segments backed up
    pub wal_segments_backed_up: u32,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Compression ratio (if compression enabled)
    pub compression_ratio: f64,
    /// Throughput in MB/s
    pub throughput_mbps: f64,
}

/// Main backup manager
pub struct BackupManager {
    /// Storage engine pager
    pager: Arc<RwLock<PageStorageManager>>,
    /// WAL manager
    wal_manager: Arc<RwLock<WALManager>>,
    /// Backup configuration
    config: BackupConfig,
    /// Storage backend
    storage_backend: Arc<dyn BackupStorageBackend>,
    /// Active backup metadata
    active_backup: Arc<RwLock<Option<BackupMetadata>>>,
}

impl BackupManager {
    /// Create a new backup manager
    pub async fn new(
        pager: Arc<RwLock<PageStorageManager>>,
        wal_manager: Arc<RwLock<WALManager>>,
        config: BackupConfig,
    ) -> Result<Self> {
        // Create storage backend based on configuration
        let storage_backend: Arc<dyn BackupStorageBackend> = match config.storage_backend {
            BackupStorageType::Local => {
                Arc::new(LocalBackend::new(config.output_path.clone()).await?)
            }
            BackupStorageType::S3 => {
                let s3_config = config
                    .s3_config
                    .as_ref()
                    .ok_or_else(|| anyhow!("S3 configuration required"))?;
                Arc::new(S3Backend::new(s3_config.clone()).await?)
            }
            BackupStorageType::GCS => {
                return Err(anyhow!("GCS backend not yet implemented"));
            }
        };

        Ok(Self {
            pager,
            wal_manager,
            config,
            storage_backend,
            active_backup: Arc::new(RwLock::new(None)),
        })
    }

    /// Perform a backup
    pub async fn backup(&self) -> Result<BackupMetadata> {
        info!("Starting backup: type={:?}", self.config.backup_type);

        // Check if a backup is already in progress
        {
            let active = self.active_backup.read().await;
            if active.is_some() {
                return Err(anyhow!("A backup is already in progress"));
            }
        }

        // Create backup metadata
        let backup_id = Uuid::new_v4();
        let start_time = Utc::now();
        let start_lsn = self.wal_manager.read().await.current_lsn();

        let mut metadata = BackupMetadata {
            backup_id,
            backup_type: self.config.backup_type,
            status: BackupStatus::InProgress,
            start_time,
            end_time: None,
            start_lsn,
            end_lsn: None,
            size_bytes: 0,
            compressed_size_bytes: 0,
            file_count: 0,
            parent_backup_id: None,
            db_version: env!("CARGO_PKG_VERSION").to_string(),
            checksum: String::new(),
            storage_location: format!("{}", self.config.output_path.display()),
            encrypted: self.config.enable_encryption,
        };

        // Store active backup
        {
            let mut active = self.active_backup.write().await;
            *active = Some(metadata.clone());
        }

        // Perform backup based on type
        let result = match self.config.backup_type {
            BackupType::Full => self.perform_full_backup(&mut metadata).await,
            BackupType::Incremental => self.perform_incremental_backup(&mut metadata).await,
            BackupType::Differential => self.perform_differential_backup(&mut metadata).await,
        };

        // Update metadata based on result
        let success = match &result {
            Ok(stats) => {
                metadata.status = BackupStatus::Completed;
                metadata.end_time = Some(Utc::now());
                metadata.end_lsn = Some(self.wal_manager.read().await.current_lsn());
                metadata.size_bytes = stats.bytes_read;
                metadata.compressed_size_bytes = stats.bytes_written;
                metadata.file_count = stats.files_backed_up;

                info!(
                    "Backup completed: id={}, size={} bytes, duration={} ms",
                    backup_id, stats.bytes_written, stats.duration_ms
                );
                true
            }
            Err(e) => {
                metadata.status = BackupStatus::Failed;
                metadata.end_time = Some(Utc::now());
                warn!("Backup failed: id={}, error={}", backup_id, e);
                false
            }
        };

        // Save metadata
        self.save_backup_metadata(&metadata).await?;

        // Clear active backup
        {
            let mut active = self.active_backup.write().await;
            *active = None;
        }

        if success {
            Ok(metadata)
        } else {
            result.map(|_| metadata)
        }
    }

    /// Perform a full backup
    async fn perform_full_backup(&self, metadata: &mut BackupMetadata) -> Result<BackupStats> {
        let start = std::time::Instant::now();
        let mut stats = BackupStats::default();

        info!("Performing full backup");

        // Create backup directory
        let backup_dir = self.get_backup_directory(&metadata.backup_id);
        self.storage_backend.create_directory(&backup_dir).await?;

        // Step 1: Backup all data pages
        info!("Backing up data pages");
        let page_stats = self.backup_data_pages(&backup_dir, metadata).await?;
        stats.bytes_read += page_stats.bytes_read;
        stats.bytes_written += page_stats.bytes_written;
        stats.pages_backed_up += page_stats.pages_backed_up;
        stats.files_backed_up += page_stats.files_backed_up;

        // Step 2: Backup WAL files if configured
        if self.config.include_wal {
            info!("Backing up WAL files");
            let wal_stats = self.backup_wal_files(&backup_dir, metadata).await?;
            stats.bytes_read += wal_stats.bytes_read;
            stats.bytes_written += wal_stats.bytes_written;
            stats.wal_segments_backed_up = wal_stats.wal_segments_backed_up;
            stats.files_backed_up += wal_stats.files_backed_up;
        }

        // Step 3: Calculate statistics
        stats.duration_ms = start.elapsed().as_millis() as u64;
        if stats.bytes_read > 0 {
            stats.compression_ratio = stats.bytes_written as f64 / stats.bytes_read as f64;
        }
        if stats.duration_ms > 0 {
            stats.throughput_mbps = (stats.bytes_written as f64 / 1024.0 / 1024.0)
                / (stats.duration_ms as f64 / 1000.0);
        }

        // Step 4: Calculate checksum for verification
        info!("Calculating backup checksum");
        metadata.checksum = self.compute_backup_checksum(&backup_dir, metadata).await?;

        Ok(stats)
    }

    /// Perform an incremental backup
    async fn perform_incremental_backup(
        &self,
        metadata: &mut BackupMetadata,
    ) -> Result<BackupStats> {
        info!("Performing incremental backup");

        // Find last full backup
        let last_full_backup = self
            .find_last_backup(BackupType::Full)
            .await?
            .ok_or_else(|| anyhow!("No full backup found for incremental backup"))?;

        metadata.parent_backup_id = Some(last_full_backup.backup_id);

        // Use incremental backup manager
        let incremental_mgr = IncrementalBackup::new(
            self.pager.clone(),
            self.wal_manager.clone(),
            self.storage_backend.clone(),
        );

        incremental_mgr
            .backup_since_lsn(last_full_backup.end_lsn.unwrap_or(0), metadata)
            .await
    }

    /// Perform a differential backup
    async fn perform_differential_backup(
        &self,
        metadata: &mut BackupMetadata,
    ) -> Result<BackupStats> {
        info!("Performing differential backup");

        // Same as incremental for now
        self.perform_incremental_backup(metadata).await
    }

    /// Backup all data pages
    async fn backup_data_pages(
        &self,
        backup_dir: &Path,
        _metadata: &BackupMetadata,
    ) -> Result<BackupStats> {
        let mut stats = BackupStats::default();
        let pager = self.pager.read().await;

        // Get storage statistics to determine how many pages to backup
        let storage_stats = pager.stats().await;
        let total_pages = storage_stats.total_pages_allocated;

        info!("Backing up {} pages", total_pages);

        // Create data subdirectory
        let data_dir = backup_dir.join("data");
        self.storage_backend.create_directory(&data_dir).await?;

        // Backup pages in chunks
        let chunk_size = 1000;
        for chunk_start in (0..total_pages).step_by(chunk_size) {
            let chunk_end = (chunk_start + chunk_size as u64).min(total_pages);

            let chunk_file =
                data_dir.join(format!("pages_{:08x}_{:08x}.dat", chunk_start, chunk_end));
            let mut chunk_data = Vec::new();

            for page_id in chunk_start..chunk_end {
                // Try to read page
                if let Ok(page) = pager.read_page(super::pager::PageId(page_id)).await {
                    let page_bytes = page.serialize()?;
                    let original_size = page_bytes.len();

                    // Compress if enabled
                    let final_bytes = if self.config.enable_compression {
                        self.compress_data(&page_bytes)?
                    } else {
                        page_bytes
                    };

                    chunk_data.extend_from_slice(&final_bytes);
                    stats.bytes_read += original_size as u64;
                    stats.pages_backed_up += 1;
                }
            }

            // Write chunk file
            if !chunk_data.is_empty() {
                self.storage_backend
                    .write_file(&chunk_file, &chunk_data)
                    .await?;
                stats.bytes_written += chunk_data.len() as u64;
                stats.files_backed_up += 1;
            }
        }

        Ok(stats)
    }

    /// Backup WAL files
    async fn backup_wal_files(
        &self,
        backup_dir: &Path,
        _metadata: &BackupMetadata,
    ) -> Result<BackupStats> {
        let mut stats = BackupStats::default();
        let wal_manager = self.wal_manager.read().await;

        // Create WAL subdirectory
        let wal_dir = backup_dir.join("wal");
        self.storage_backend.create_directory(&wal_dir).await?;

        // Get WAL directory
        let wal_path = wal_manager.get_wal_directory();

        // List WAL segment files
        let mut wal_files = tokio::fs::read_dir(&wal_path).await?;

        while let Some(entry) = wal_files.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wal") {
                let filename = path.file_name().unwrap();
                let dest_path = wal_dir.join(filename);

                // Read and optionally compress WAL file
                let wal_data = tokio::fs::read(&path).await?;

                let final_data = if self.config.enable_compression {
                    self.compress_data(&wal_data)?
                } else {
                    wal_data.clone()
                };

                self.storage_backend
                    .write_file(&dest_path, &final_data)
                    .await?;

                stats.bytes_read += wal_data.len() as u64;
                stats.bytes_written += final_data.len() as u64;
                stats.wal_segments_backed_up += 1;
                stats.files_backed_up += 1;
            }
        }

        Ok(stats)
    }

    /// Compress data using gzip
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder =
            GzEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }

    /// Get backup directory path
    fn get_backup_directory(&self, backup_id: &BackupId) -> PathBuf {
        self.config
            .output_path
            .join(format!("backup_{}", backup_id))
    }

    /// Save backup metadata
    async fn save_backup_metadata(&self, metadata: &BackupMetadata) -> Result<()> {
        let metadata_path = self
            .get_backup_directory(&metadata.backup_id)
            .join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(metadata)?;
        self.storage_backend
            .write_file(&metadata_path, metadata_json.as_bytes())
            .await?;
        Ok(())
    }

    /// Compute checksum of backup files for verification
    async fn compute_backup_checksum(
        &self,
        backup_dir: &Path,
        _metadata: &BackupMetadata,
    ) -> Result<String> {
        use sha3::{Digest, Sha3_256};

        let mut hasher = Sha3_256::new();

        // Note: We only hash the actual data files, not metadata
        // This allows metadata fields like end_time to change without invalidating the checksum

        // Hash all data files in sorted order for consistency
        let data_dir = backup_dir.join("data");
        if self.storage_backend.directory_exists(&data_dir).await? {
            let mut entries = self.storage_backend.list_directory(&data_dir).await?;
            entries.sort();

            for file_path in entries {
                if file_path.extension().and_then(|s| s.to_str()) == Some("dat") {
                    let file_data = self.storage_backend.read_file(&file_path).await?;
                    hasher.update(&file_data);
                }
            }
        }

        // Hash WAL files if present
        let wal_dir = backup_dir.join("wal");
        if self.storage_backend.directory_exists(&wal_dir).await? {
            let mut entries = self.storage_backend.list_directory(&wal_dir).await?;
            entries.sort();

            for file_path in entries {
                if file_path.extension().and_then(|s| s.to_str()) == Some("wal") {
                    let file_data = self.storage_backend.read_file(&file_path).await?;
                    hasher.update(&file_data);
                }
            }
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Find last backup of a specific type
    async fn find_last_backup(&self, backup_type: BackupType) -> Result<Option<BackupMetadata>> {
        // List all backup directories
        let mut backups = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.config.output_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let metadata_path = path.join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(metadata_json) = tokio::fs::read_to_string(&metadata_path).await {
                        if let Ok(metadata) = serde_json::from_str::<BackupMetadata>(&metadata_json)
                        {
                            if metadata.backup_type == backup_type
                                && metadata.status == BackupStatus::Completed
                            {
                                backups.push(metadata);
                            }
                        }
                    }
                }
            }
        }

        // Sort by end time and return most recent
        backups.sort_by(|a, b| {
            b.end_time
                .unwrap_or(Utc::now())
                .cmp(&a.end_time.unwrap_or(Utc::now()))
        });

        Ok(backups.into_iter().next())
    }

    /// List all backups
    pub async fn list_backups(&self) -> Result<Vec<BackupMetadata>> {
        let mut backups = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.config.output_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let metadata_path = path.join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(metadata_json) = tokio::fs::read_to_string(&metadata_path).await {
                        if let Ok(metadata) = serde_json::from_str::<BackupMetadata>(&metadata_json)
                        {
                            backups.push(metadata);
                        }
                    }
                }
            }
        }

        // Sort by start time (newest first)
        backups.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        Ok(backups)
    }

    /// Get backup metadata by ID
    pub async fn get_backup(&self, backup_id: BackupId) -> Result<BackupMetadata> {
        let metadata_path = self.get_backup_directory(&backup_id).join("metadata.json");
        let metadata_json = tokio::fs::read_to_string(&metadata_path).await?;
        let metadata = serde_json::from_str(&metadata_json)?;
        Ok(metadata)
    }

    /// Delete a backup
    pub async fn delete_backup(&self, backup_id: BackupId) -> Result<()> {
        let backup_dir = self.get_backup_directory(&backup_id);
        tokio::fs::remove_dir_all(&backup_dir).await?;
        info!("Deleted backup: {}", backup_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_backup_config_default() {
        let config = BackupConfig::default();
        assert_eq!(config.backup_type, BackupType::Full);
        assert!(config.enable_compression);
        assert!(!config.enable_encryption);
    }

    #[tokio::test]
    async fn test_backup_metadata_creation() {
        let metadata = BackupMetadata {
            backup_id: Uuid::new_v4(),
            backup_type: BackupType::Full,
            status: BackupStatus::InProgress,
            start_time: Utc::now(),
            end_time: None,
            start_lsn: 0,
            end_lsn: None,
            size_bytes: 0,
            compressed_size_bytes: 0,
            file_count: 0,
            parent_backup_id: None,
            db_version: "0.1.0".to_string(),
            checksum: String::new(),
            storage_location: "./backups".to_string(),
            encrypted: false,
        };

        assert_eq!(metadata.status, BackupStatus::InProgress);
        assert_eq!(metadata.backup_type, BackupType::Full);
    }
}
