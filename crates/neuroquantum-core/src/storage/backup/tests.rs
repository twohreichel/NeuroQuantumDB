//! Comprehensive tests for Backup & Restore system

#![allow(clippy::significant_drop_tightening)]

use std::sync::Arc;

use anyhow::Result;
use tempfile::TempDir;
use tokio::sync::RwLock;

use crate::storage::pager::{Page, PageId, PageType};
use crate::storage::{
    BackupConfig, BackupManager, BackupStorageBackend, BackupStorageType, BackupType, LocalBackend,
    PageStorageManager, PagerConfig, RestoreManager, RestoreOptions, SyncMode, WALConfig,
    WALManager,
};

/// Helper to create test database
async fn setup_test_db() -> Result<(
    TempDir,
    Arc<RwLock<PageStorageManager>>,
    Arc<RwLock<WALManager>>,
)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("db");
    let wal_path = temp_dir.path().join("wal");

    tokio::fs::create_dir_all(&db_path).await?;
    tokio::fs::create_dir_all(&wal_path).await?;

    let pager_config = PagerConfig {
        max_file_size: 10 * 1024 * 1024,
        enable_checksums: true,
        sync_mode: SyncMode::None,
        direct_io: false,
    };

    let db_file = db_path.join("test.db");
    let pager: Arc<RwLock<PageStorageManager>> = Arc::new(RwLock::new(
        PageStorageManager::new(&db_file, pager_config.clone()).await?,
    ));

    let wal_config = WALConfig {
        wal_dir: wal_path,
        segment_size: 1024 * 1024,
        sync_on_write: false,
        buffer_size: 64 * 1024,
        checkpoint_interval_secs: 60,
        min_segments_to_keep: 2,
        group_commit_delay_ms: 0,
        group_commit_max_records: 1000,
        group_commit_max_bytes: 4 * 1024 * 1024,
    };

    // Create a separate pager instance for WAL
    let pager_for_wal = PageStorageManager::new(&db_file, pager_config).await?;

    let wal_manager: Arc<RwLock<WALManager>> = Arc::new(RwLock::new(
        WALManager::new(wal_config, Arc::new(pager_for_wal)).await?,
    ));

    Ok((temp_dir, pager, wal_manager))
}

#[tokio::test]
async fn test_full_backup_creation() -> Result<()> {
    let (temp_dir, pager, wal_manager) = setup_test_db().await?;

    // Write some test pages
    {
        let pager = pager.write().await;
        for i in 1..=10 {
            let mut page = Page::new(PageId(i), PageType::Data);
            let test_data = format!("Test page {i}");
            page.write_data(0, test_data.as_bytes())?;
            page.update_checksum();
            pager.write_page(&page).await?;
        }
        pager.sync().await?;
    }

    // Create backup
    let backup_path = temp_dir.path().join("backups");
    let config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Full,
        enable_compression: true,
        compression_level: 6,
        enable_encryption: false,
        encryption_key: None,
        max_concurrency: 4,
        include_wal: true,
        storage_backend: BackupStorageType::Local,
        s3_config: None,
    };

    let backup_manager = BackupManager::new(pager, wal_manager, config).await?;
    let metadata = backup_manager.backup().await?;

    // Verify backup
    assert_eq!(metadata.backup_type, BackupType::Full);
    assert!(metadata.file_count > 0);
    assert!(metadata.compressed_size_bytes > 0);

    Ok(())
}

#[tokio::test]
async fn test_backup_and_restore() -> Result<()> {
    let (temp_dir, pager, wal_manager) = setup_test_db().await?;

    // Write test data
    {
        let pager = pager.write().await;
        for i in 1..=5 {
            let mut page = Page::new(PageId(i), PageType::Data);
            let test_data = format!("Backup test page {i}");
            page.write_data(0, test_data.as_bytes())?;
            page.update_checksum();
            pager.write_page(&page).await?;
        }
        pager.sync().await?;
    }

    // Create backup
    let backup_path = temp_dir.path().join("backups");
    let config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Full,
        enable_compression: false,
        compression_level: 6,
        enable_encryption: false,
        encryption_key: None,
        max_concurrency: 4,
        include_wal: false,
        storage_backend: BackupStorageType::Local,
        s3_config: None,
    };

    let backup_manager = BackupManager::new(pager, wal_manager, config).await?;
    let backup_metadata = backup_manager.backup().await?;

    // Restore backup
    let restore_path = temp_dir.path().join("restored");
    let storage_backend = Arc::new(LocalBackend::new(backup_path).await?);

    let restore_options = RestoreOptions {
        backup_id: backup_metadata.backup_id,
        target_time: None,
        target_lsn: None,
        output_path: restore_path.clone(),
        verify_before_restore: true,
        verify_after_restore: true,
        max_concurrency: 4,
    };

    let restore_manager = RestoreManager::new(storage_backend, restore_options);
    let stats = restore_manager.restore().await?;

    // Verify restore
    assert!(stats.verification_passed);
    assert!(stats.files_restored > 0);
    assert!(restore_path.join("data").exists());

    Ok(())
}

#[tokio::test]
async fn test_incremental_backup() -> Result<()> {
    let (temp_dir, pager, wal_manager) = setup_test_db().await?;

    // Initial data
    {
        let pager = pager.write().await;
        for i in 1..=5 {
            let mut page = Page::new(PageId(i), PageType::Data);
            page.write_data(0, b"Initial data")?;
            page.update_checksum();
            pager.write_page(&page).await?;
        }
        pager.sync().await?;
    }

    let backup_path = temp_dir.path().join("backups");

    // Full backup
    let full_config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Full,
        enable_compression: false,
        ..Default::default()
    };

    let full_manager =
        BackupManager::new(Arc::clone(&pager), Arc::clone(&wal_manager), full_config).await?;
    let full_backup = full_manager.backup().await?;

    // Add more data
    {
        let pager = pager.write().await;
        for i in 6..=10 {
            let mut page = Page::new(PageId(i), PageType::Data);
            page.write_data(0, b"New data")?;
            page.update_checksum();
            pager.write_page(&page).await?;
        }
        pager.sync().await?;
    }

    // Incremental backup
    let incr_config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Incremental,
        enable_compression: false,
        ..Default::default()
    };

    let incr_manager = BackupManager::new(pager, wal_manager, incr_config).await?;
    let incr_backup = incr_manager.backup().await?;

    // Verify incremental references full backup
    assert_eq!(incr_backup.backup_type, BackupType::Incremental);
    assert_eq!(incr_backup.parent_backup_id, Some(full_backup.backup_id));

    Ok(())
}

#[tokio::test]
async fn test_backup_list() -> Result<()> {
    let (temp_dir, pager, wal_manager) = setup_test_db().await?;

    let backup_path = temp_dir.path().join("backups");
    let config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Full,
        enable_compression: false,
        ..Default::default()
    };

    // Create multiple backups
    for _ in 0..3 {
        let manager =
            BackupManager::new(Arc::clone(&pager), Arc::clone(&wal_manager), config.clone())
                .await?;
        manager.backup().await?;

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // List backups
    let manager = BackupManager::new(pager, wal_manager, config).await?;
    let backups = manager.list_backups().await?;

    assert_eq!(backups.len(), 3);

    // Verify ordering (newest first)
    for i in 1..backups.len() {
        assert!(backups[i - 1].start_time >= backups[i].start_time);
    }

    Ok(())
}

#[tokio::test]
async fn test_backup_compression() -> Result<()> {
    let (temp_dir, pager, wal_manager) = setup_test_db().await?;

    // Write compressible data
    {
        let pager = pager.write().await;
        for i in 1..=10 {
            let mut page = Page::new(PageId(i), PageType::Data);
            // Repeating pattern - should compress well
            let test_data = "A".repeat(1000);
            page.write_data(0, test_data.as_bytes())?;
            page.update_checksum();
            pager.write_page(&page).await?;
        }
        pager.sync().await?;
    }

    let backup_path = temp_dir.path().join("backups");

    // Uncompressed backup
    let uncompressed_config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Full,
        enable_compression: false,
        include_wal: false,
        ..Default::default()
    };

    let uncompressed_manager = BackupManager::new(
        Arc::clone(&pager),
        Arc::clone(&wal_manager),
        uncompressed_config,
    )
    .await?;
    let uncompressed = uncompressed_manager.backup().await?;

    // Compressed backup
    let compressed_config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Full,
        enable_compression: true,
        compression_level: 9,
        include_wal: false,
        ..Default::default()
    };

    let compressed_manager = BackupManager::new(pager, wal_manager, compressed_config).await?;
    let compressed = compressed_manager.backup().await?;

    // Compressed should be smaller
    assert!(compressed.compressed_size_bytes < uncompressed.compressed_size_bytes);

    Ok(())
}

#[tokio::test]
async fn test_local_backend_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let backend = LocalBackend::new(temp_dir.path().to_path_buf()).await?;

    // Test write and read
    let test_file = temp_dir.path().join("test.dat");
    let test_data = b"Hello, Backup!";

    backend.write_file(&test_file, test_data).await?;
    let read_data = backend.read_file(&test_file).await?;

    assert_eq!(read_data, test_data);

    // Test directory operations
    let test_dir = temp_dir.path().join("subdir");
    backend.create_directory(&test_dir).await?;
    assert!(backend.directory_exists(&test_dir).await?);

    // Test list directory
    backend
        .write_file(&test_dir.join("file1.dat"), b"data1")
        .await?;
    backend
        .write_file(&test_dir.join("file2.dat"), b"data2")
        .await?;

    let files = backend.list_directory(&test_dir).await?;
    assert_eq!(files.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_backup_metadata_persistence() -> Result<()> {
    let (temp_dir, pager, wal_manager) = setup_test_db().await?;

    let backup_path = temp_dir.path().join("backups");
    let config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Full,
        enable_compression: false,
        ..Default::default()
    };

    let manager = BackupManager::new(pager, wal_manager, config.clone()).await?;
    let backup = manager.backup().await?;

    // Read metadata from disk
    let metadata_file = backup_path
        .join(format!("backup_{}", backup.backup_id))
        .join("metadata.json");

    assert!(metadata_file.exists());

    let metadata_json: String = tokio::fs::read_to_string(&metadata_file).await?;
    let loaded_metadata: super::BackupMetadata = serde_json::from_str(&metadata_json)?;

    assert_eq!(loaded_metadata.backup_id, backup.backup_id);
    assert_eq!(loaded_metadata.backup_type, backup.backup_type);

    Ok(())
}

#[tokio::test]
async fn test_restore_verification() -> Result<()> {
    let (temp_dir, pager, wal_manager) = setup_test_db().await?;

    // Create backup
    let backup_path = temp_dir.path().join("backups");
    let config = BackupConfig {
        output_path: backup_path.clone(),
        backup_type: BackupType::Full,
        enable_compression: false,
        include_wal: false,
        ..Default::default()
    };

    let manager = BackupManager::new(pager, wal_manager, config).await?;
    let backup = manager.backup().await?;

    // Restore with verification enabled
    let restore_path = temp_dir.path().join("restored");
    let storage_backend = Arc::new(LocalBackend::new(backup_path).await?);

    let options = RestoreOptions {
        backup_id: backup.backup_id,
        target_time: None,
        target_lsn: None,
        output_path: restore_path,
        verify_before_restore: true,
        verify_after_restore: true,
        max_concurrency: 4,
    };

    let restore_manager = RestoreManager::new(storage_backend, options);
    let stats = restore_manager.restore().await?;

    assert!(stats.verification_passed);

    Ok(())
}
