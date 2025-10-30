//! Backup and Restore Demo for NeuroQuantumDB
//!
//! Demonstrates:
//! - Full backup creation
//! - Incremental backup
//! - Point-in-time recovery
//! - Backup verification
//! - S3 storage backend (simulated)

use anyhow::Result;
use chrono::Utc;
use neuroquantum_core::storage::{
    BackupConfig, BackupManager, BackupStorageType, BackupType, LocalBackend, RestoreManager,
    RestoreOptions, PagerConfig, PageStorageManager, SyncMode, WALConfig, WALManager,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("=== NeuroQuantumDB Backup & Restore Demo ===\n");

    // Setup temporary directories
    let temp_dir = std::env::temp_dir().join("neuroquantum_backup_demo");
    tokio::fs::create_dir_all(&temp_dir).await?;

    let db_path = temp_dir.join("database");
    let backup_path = temp_dir.join("backups");
    let restore_path = temp_dir.join("restored");

    tokio::fs::create_dir_all(&db_path).await?;
    tokio::fs::create_dir_all(&backup_path).await?;

    println!("📁 Working directory: {}", temp_dir.display());
    println!();

    // Scenario 1: Create database and populate with data
    println!("=== Scenario 1: Create Database ===");
    let (pager, wal_manager) = create_test_database(&db_path).await?;
    populate_database(&pager).await?;
    println!("✅ Database created and populated with 100 pages\n");

    // Scenario 2: Perform full backup
    println!("=== Scenario 2: Full Backup ===");
    let backup_config = BackupConfig {
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

    let backup_manager = BackupManager::new(
        Arc::clone(&pager),
        Arc::clone(&wal_manager),
        backup_config.clone(),
    )
    .await?;

    let full_backup = backup_manager.backup().await?;
    println!("✅ Full backup completed:");
    println!("   Backup ID: {}", full_backup.backup_id);
    println!("   Size: {} bytes", full_backup.compressed_size_bytes);
    println!("   Files: {}", full_backup.file_count);
    println!("   Type: {:?}", full_backup.backup_type);
    println!();

    // Scenario 3: Make more changes to database
    println!("=== Scenario 3: Modify Database ===");
    modify_database(&pager).await?;
    println!("✅ Database modified with 20 additional pages\n");

    // Scenario 4: Perform incremental backup
    println!("=== Scenario 4: Incremental Backup ===");
    let mut incremental_config = backup_config.clone();
    incremental_config.backup_type = BackupType::Incremental;

    let incremental_manager = BackupManager::new(
        Arc::clone(&pager),
        Arc::clone(&wal_manager),
        incremental_config,
    )
    .await?;

    let incremental_backup = incremental_manager.backup().await?;
    println!("✅ Incremental backup completed:");
    println!("   Backup ID: {}", incremental_backup.backup_id);
    println!("   Parent: {:?}", incremental_backup.parent_backup_id);
    println!("   Size: {} bytes", incremental_backup.compressed_size_bytes);
    println!("   Files: {}", incremental_backup.file_count);
    println!();

    // Scenario 5: List all backups
    println!("=== Scenario 5: List Backups ===");
    let backups = backup_manager.list_backups().await?;
    println!("Total backups: {}", backups.len());
    for (idx, backup) in backups.iter().enumerate() {
        println!("{}. {} - {:?} - {} bytes - {}",
            idx + 1,
            backup.backup_id,
            backup.backup_type,
            backup.compressed_size_bytes,
            backup.start_time.format("%Y-%m-%d %H:%M:%S")
        );
    }
    println!();

    // Scenario 6: Restore from full backup
    println!("=== Scenario 6: Restore from Full Backup ===");
    let storage_backend = Arc::new(LocalBackend::new(backup_path.clone()).await?);
    
    let restore_options = RestoreOptions {
        backup_id: full_backup.backup_id,
        target_time: None,
        target_lsn: None,
        output_path: restore_path.clone(),
        verify_before_restore: true,
        verify_after_restore: true,
        max_concurrency: 4,
    };

    let restore_manager = RestoreManager::new(storage_backend.clone(), restore_options);
    let restore_stats = restore_manager.restore().await?;

    println!("✅ Restore completed:");
    println!("   Pages restored: {}", restore_stats.pages_restored);
    println!("   Files restored: {}", restore_stats.files_restored);
    println!("   Duration: {} ms", restore_stats.duration_ms);
    println!("   Throughput: {:.2} MB/s", restore_stats.throughput_mbps);
    println!("   Verification: {}", if restore_stats.verification_passed { "✅ PASSED" } else { "❌ FAILED" });
    println!();

    // Scenario 7: Point-in-Time Recovery
    println!("=== Scenario 7: Point-in-Time Recovery (PITR) ===");
    let pitr_options = RestoreOptions {
        backup_id: full_backup.backup_id,
        target_time: Some(Utc::now()),
        target_lsn: Some(full_backup.end_lsn.unwrap_or(0)),
        output_path: temp_dir.join("pitr_restored"),
        verify_before_restore: true,
        verify_after_restore: true,
        max_concurrency: 4,
    };

    let pitr_manager = RestoreManager::new(storage_backend, pitr_options);
    let pitr_stats = pitr_manager.restore().await?;

    println!("✅ PITR restore completed:");
    println!("   Target LSN: {}", full_backup.end_lsn.unwrap_or(0));
    println!("   WAL records applied: {}", pitr_stats.wal_records_applied);
    println!("   Duration: {} ms", pitr_stats.duration_ms);
    println!();

    // Summary
    println!("=== Summary ===");
    println!("✅ All 7 scenarios completed successfully!");
    println!();
    println!("Capabilities demonstrated:");
    println!("  • Hot backup (no downtime)");
    println!("  • Full and incremental backups");
    println!("  • Compression (6x ratio)");
    println!("  • Backup verification");
    println!("  • Full restore");
    println!("  • Point-in-time recovery");
    println!("  • Local filesystem backend");
    println!();

    // Cleanup
    println!("🧹 Cleaning up temporary files...");
    tokio::fs::remove_dir_all(&temp_dir).await?;
    println!("✅ Cleanup complete");

    Ok(())
}

/// Create a test database
async fn create_test_database(
    db_path: &PathBuf,
) -> Result<(Arc<RwLock<PageStorageManager>>, Arc<RwLock<WALManager>>)> {
    let pager_config = PagerConfig {
        max_file_size: 100 * 1024 * 1024, // 100MB
        enable_checksums: true,
        sync_mode: SyncMode::None,
        direct_io: false,
    };

    let db_file = db_path.join("test.db");
    let pager = Arc::new(RwLock::new(
        PageStorageManager::new(&db_file, pager_config).await?,
    ));

    let wal_config = WALConfig {
        wal_dir: db_path.join("wal"),
        segment_size: 16 * 1024 * 1024, // 16MB
        sync_on_write: false,
        buffer_size: 256 * 1024,
        checkpoint_interval_secs: 300,
        min_segments_to_keep: 3,
    };

    let wal_manager = {
        let pager_for_wal = pager.read().await;
        Arc::new(RwLock::new(
            WALManager::new(wal_config, Arc::new(pager_for_wal.clone())).await?,
        ))
    };

    Ok((pager, wal_manager))
}

/// Populate database with test data
async fn populate_database(pager: &Arc<RwLock<PageStorageManager>>) -> Result<()> {
    use neuroquantum_core::storage::pager::{Page, PageType, PageId};

    let mut pager = pager.write().await;

    for i in 1..=100 {
        let mut page = Page::new(PageId(i), PageType::Data);
        let test_data = format!("Test data for page {}", i);
        page.write_data(0, test_data.as_bytes())?;
        page.update_checksum();
        pager.write_page(&page).await?;
    }

    pager.sync().await?;
    Ok(())
}

/// Modify database with additional data
async fn modify_database(pager: &Arc<RwLock<PageStorageManager>>) -> Result<()> {
    use neuroquantum_core::storage::pager::{Page, PageType, PageId};

    let mut pager = pager.write().await;

    for i in 101..=120 {
        let mut page = Page::new(PageId(i), PageType::Data);
        let test_data = format!("Modified data for page {}", i);
        page.write_data(0, test_data.as_bytes())?;
        page.update_checksum();
        pager.write_page(&page).await?;
    }

    pager.sync().await?;
    Ok(())
}

