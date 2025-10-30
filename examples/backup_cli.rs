//! CLI tool for NeuroQuantumDB Backup and Restore operations
//!
//! Usage:
//!   cargo run --example backup_cli -- backup --db ./data --output ./backups
//!   cargo run --example backup_cli -- restore --backup <id> --output ./restored
//!   cargo run --example backup_cli -- list --backup-dir ./backups
//!   cargo run --example backup_cli -- pitr --backup <id> --time "2025-10-30T12:00:00Z"

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use neuroquantum_core::storage::{
    BackupConfig, BackupManager, BackupStorageType, BackupType, LocalBackend, RestoreManager,
    RestoreOptions, PagerConfig, PageStorageManager, SyncMode, WALConfig, WALManager,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "neuroquantum-backup")]
#[command(about = "NeuroQuantumDB Backup and Restore CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a backup
    Backup {
        /// Database directory
        #[arg(short, long)]
        db: PathBuf,

        /// Output backup directory
        #[arg(short, long)]
        output: PathBuf,

        /// Backup type (full, incremental, differential)
        #[arg(short = 't', long, default_value = "full")]
        backup_type: String,

        /// Enable compression
        #[arg(short, long, default_value = "true")]
        compress: bool,

        /// Compression level (1-9)
        #[arg(long, default_value = "6")]
        compression_level: u32,

        /// Include WAL files
        #[arg(long, default_value = "true")]
        include_wal: bool,
    },

    /// Restore from backup
    Restore {
        /// Backup ID
        #[arg(short, long)]
        backup_id: String,

        /// Backup directory
        #[arg(short = 'i', long)]
        backup_dir: PathBuf,

        /// Output directory for restored database
        #[arg(short, long)]
        output: PathBuf,

        /// Verify before restore
        #[arg(long, default_value = "true")]
        verify: bool,
    },

    /// List all backups
    List {
        /// Backup directory
        #[arg(short, long)]
        backup_dir: PathBuf,
    },

    /// Point-in-time recovery
    Pitr {
        /// Backup ID to start from
        #[arg(short, long)]
        backup_id: String,

        /// Backup directory
        #[arg(short = 'i', long)]
        backup_dir: PathBuf,

        /// Target timestamp (ISO 8601 format)
        #[arg(short, long)]
        time: Option<String>,

        /// Target LSN
        #[arg(short, long)]
        lsn: Option<u64>,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Delete a backup
    Delete {
        /// Backup ID
        #[arg(short, long)]
        backup_id: String,

        /// Backup directory
        #[arg(short = 'i', long)]
        backup_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Backup {
            db,
            output,
            backup_type,
            compress,
            compression_level,
            include_wal,
        } => {
            handle_backup(db, output, backup_type, compress, compression_level, include_wal)
                .await?;
        }
        Commands::Restore {
            backup_id,
            backup_dir,
            output,
            verify,
        } => {
            handle_restore(backup_id, backup_dir, output, verify).await?;
        }
        Commands::List { backup_dir } => {
            handle_list(backup_dir).await?;
        }
        Commands::Pitr {
            backup_id,
            backup_dir,
            time,
            lsn,
            output,
        } => {
            handle_pitr(backup_id, backup_dir, time, lsn, output).await?;
        }
        Commands::Delete {
            backup_id,
            backup_dir,
        } => {
            handle_delete(backup_id, backup_dir).await?;
        }
    }

    Ok(())
}

async fn handle_backup(
    db_path: PathBuf,
    output: PathBuf,
    backup_type_str: String,
    compress: bool,
    compression_level: u32,
    include_wal: bool,
) -> Result<()> {
    info!("Starting backup operation");

    let backup_type = match backup_type_str.as_str() {
        "full" => BackupType::Full,
        "incremental" => BackupType::Incremental,
        "differential" => BackupType::Differential,
        _ => return Err(anyhow!("Invalid backup type: {}", backup_type_str)),
    };

    // Initialize database components
    let (pager, wal_manager) = init_database(&db_path).await?;

    // Create backup configuration
    let config = BackupConfig {
        output_path: output.clone(),
        backup_type,
        enable_compression: compress,
        compression_level,
        enable_encryption: false,
        encryption_key: None,
        max_concurrency: 4,
        include_wal,
        storage_backend: BackupStorageType::Local,
        s3_config: None,
    };

    // Perform backup
    let backup_manager = BackupManager::new(pager, wal_manager, config).await?;
    let metadata = backup_manager.backup().await?;

    println!("\n✅ Backup completed successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Backup ID:       {}", metadata.backup_id);
    println!("Type:            {:?}", metadata.backup_type);
    println!("Start Time:      {}", metadata.start_time.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("End Time:        {}", metadata.end_time.unwrap().format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Size:            {} bytes", metadata.size_bytes);
    println!("Compressed Size: {} bytes", metadata.compressed_size_bytes);
    println!("Files:           {}", metadata.file_count);
    println!("Location:        {}", output.display());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}

async fn handle_restore(
    backup_id: String,
    backup_dir: PathBuf,
    output: PathBuf,
    verify: bool,
) -> Result<()> {
    info!("Starting restore operation");

    let backup_uuid = uuid::Uuid::parse_str(&backup_id)?;
    let storage_backend = Arc::new(LocalBackend::new(backup_dir).await?);

    let options = RestoreOptions {
        backup_id: backup_uuid,
        target_time: None,
        target_lsn: None,
        output_path: output.clone(),
        verify_before_restore: verify,
        verify_after_restore: verify,
        max_concurrency: 4,
    };

    let restore_manager = RestoreManager::new(storage_backend, options);
    let stats = restore_manager.restore().await?;

    println!("\n✅ Restore completed successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Backup ID:       {}", backup_id);
    println!("Pages Restored:  {}", stats.pages_restored);
    println!("Files Restored:  {}", stats.files_restored);
    println!("Duration:        {} ms", stats.duration_ms);
    println!("Throughput:      {:.2} MB/s", stats.throughput_mbps);
    println!("Verification:    {}", if stats.verification_passed { "✅ PASSED" } else { "❌ FAILED" });
    println!("Output:          {}", output.display());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}

async fn handle_list(backup_dir: PathBuf) -> Result<()> {
    info!("Listing backups");

    // Create a temporary backup manager to list backups
    let temp_config = BackupConfig {
        output_path: backup_dir.clone(),
        ..Default::default()
    };

    // We need a dummy pager and WAL manager for the backup manager
    // In production, this should be refactored
    let temp_db = std::env::temp_dir().join("neuroquantum_list_temp");
    tokio::fs::create_dir_all(&temp_db).await?;
    let (pager, wal_manager) = init_database(&temp_db).await?;

    let backup_manager = BackupManager::new(pager, wal_manager, temp_config).await?;
    let backups = backup_manager.list_backups().await?;

    println!("\n📋 Available Backups");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("{:<38} {:<15} {:<12} {:<20}", "Backup ID", "Type", "Size (MB)", "Date");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for backup in &backups {
        let size_mb = backup.compressed_size_bytes as f64 / (1024.0 * 1024.0);
        println!(
            "{:<38} {:<15} {:<12.2} {:<20}",
            backup.backup_id,
            format!("{:?}", backup.backup_type),
            size_mb,
            backup.start_time.format("%Y-%m-%d %H:%M:%S")
        );
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total: {} backup(s)\n", backups.len());

    // Cleanup temp directory
    let _ = tokio::fs::remove_dir_all(&temp_db).await;

    Ok(())
}

async fn handle_pitr(
    backup_id: String,
    backup_dir: PathBuf,
    time_str: Option<String>,
    lsn: Option<u64>,
    output: PathBuf,
) -> Result<()> {
    info!("Starting point-in-time recovery");

    let backup_uuid = uuid::Uuid::parse_str(&backup_id)?;
    let storage_backend = Arc::new(LocalBackend::new(backup_dir).await?);

    let target_time = if let Some(time) = time_str {
        Some(DateTime::parse_from_rfc3339(&time)?.with_timezone(&Utc))
    } else {
        None
    };

    let options = RestoreOptions {
        backup_id: backup_uuid,
        target_time,
        target_lsn: lsn,
        output_path: output.clone(),
        verify_before_restore: true,
        verify_after_restore: true,
        max_concurrency: 4,
    };

    let restore_manager = RestoreManager::new(storage_backend, options);
    let stats = restore_manager.restore().await?;

    println!("\n✅ Point-in-time recovery completed!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Backup ID:         {}", backup_id);
    if let Some(t) = target_time {
        println!("Target Time:       {}", t.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    if let Some(l) = lsn {
        println!("Target LSN:        {}", l);
    }
    println!("WAL Records:       {}", stats.wal_records_applied);
    println!("Duration:          {} ms", stats.duration_ms);
    println!("Output:            {}", output.display());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}

async fn handle_delete(backup_id: String, backup_dir: PathBuf) -> Result<()> {
    info!("Deleting backup");

    let backup_uuid = uuid::Uuid::parse_str(&backup_id)?;

    // Create backup manager to delete
    let temp_config = BackupConfig {
        output_path: backup_dir.clone(),
        ..Default::default()
    };

    let temp_db = std::env::temp_dir().join("neuroquantum_delete_temp");
    tokio::fs::create_dir_all(&temp_db).await?;
    let (pager, wal_manager) = init_database(&temp_db).await?;

    let backup_manager = BackupManager::new(pager, wal_manager, temp_config).await?;
    backup_manager.delete_backup(backup_uuid).await?;

    println!("\n✅ Backup deleted successfully!");
    println!("Backup ID: {}\n", backup_id);

    // Cleanup temp directory
    let _ = tokio::fs::remove_dir_all(&temp_db).await;

    Ok(())
}

/// Initialize database components
async fn init_database(
    db_path: &PathBuf,
) -> Result<(Arc<RwLock<PageStorageManager>>, Arc<RwLock<WALManager>>)> {
    let pager_config = PagerConfig {
        max_file_size: 10 * 1024 * 1024 * 1024, // 10GB
        enable_checksums: true,
        sync_mode: SyncMode::Commit,
        direct_io: false,
    };

    let db_file = db_path.join("neuroquantum.db");
    let pager = Arc::new(RwLock::new(
        PageStorageManager::new(&db_file, pager_config).await?,
    ));

    let wal_config = WALConfig {
        wal_dir: db_path.join("wal"),
        segment_size: 64 * 1024 * 1024, // 64MB
        sync_on_write: true,
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

