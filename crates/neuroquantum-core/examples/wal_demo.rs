//! Write-Ahead Logging (WAL) Demo
//!
//! Demonstrates the WAL system with ARIES recovery:
//! - Transaction logging
//! - Commit/Abort operations
//! - Checkpointing
//! - Crash recovery simulation

use std::sync::Arc;

use neuroquantum_core::storage::pager::{PageStorageManager, PageType, PagerConfig, SyncMode};
use neuroquantum_core::storage::wal::{RecoveryStats, WALConfig, WALManager};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 NeuroQuantumDB - Write-Ahead Logging (WAL) Demo");
    println!("{}", "=".repeat(60));
    println!();

    // Setup storage directories
    let data_dir = std::env::temp_dir().join("neuroquantum_wal_demo");
    let wal_dir = data_dir.join("wal");

    if data_dir.exists() {
        std::fs::remove_dir_all(&data_dir)?;
    }

    tokio::fs::create_dir_all(&data_dir).await?;
    tokio::fs::create_dir_all(&wal_dir).await?;

    // Configure page storage
    let pager_config = PagerConfig {
        max_file_size: 100 * 1024 * 1024, // 100MB
        enable_checksums: true,
        sync_mode: SyncMode::Commit,
        direct_io: false,
    };

    let db_file = data_dir.join("demo.db");
    let pager = Arc::new(PageStorageManager::new(&db_file, pager_config).await?);

    // Configure WAL
    let wal_config = WALConfig {
        wal_dir: wal_dir.clone(),
        segment_size: 16 * 1024 * 1024, // 16MB segments
        sync_on_write: true,
        buffer_size: 256 * 1024, // 256KB buffer
        checkpoint_interval_secs: 300,
        min_segments_to_keep: 3,
    };

    let wal = WALManager::new(wal_config, Arc::clone(&pager)).await?;

    println!("✅ WAL Manager initialized");
    println!("   Data directory: {}", data_dir.display());
    println!("   WAL directory: {}", wal_dir.display());
    println!();

    // Demo 1: Simple Transaction
    demo_simple_transaction(&wal, &pager).await?;
    println!();

    // Demo 2: Multiple Concurrent Transactions
    demo_concurrent_transactions(&wal, &pager).await?;
    println!();

    // Demo 3: Transaction Abort
    demo_transaction_abort(&wal, &pager).await?;
    println!();

    // Demo 4: Checkpoint
    demo_checkpoint(&wal).await?;
    println!();

    // Demo 5: Crash Recovery Simulation
    demo_crash_recovery(&wal, &pager).await?;
    println!();

    println!("🎉 All WAL demos completed successfully!");
    println!();
    println!("📊 WAL Statistics:");
    println!("   Current LSN: {}", wal.get_current_lsn());
    println!(
        "   Transaction Table: {} active",
        wal.get_transaction_table().await.len()
    );
    println!("   Dirty Pages: {}", wal.get_dirty_page_table().await.len());

    Ok(())
}

async fn demo_simple_transaction(
    wal: &WALManager,
    pager: &Arc<PageStorageManager>,
) -> anyhow::Result<()> {
    println!("📝 Demo 1: Simple Transaction");
    println!("{}", "-".repeat(40));

    // Allocate a page
    let page_id = pager.allocate_page(PageType::Data).await?;
    println!("   Allocated page: {}", page_id.0);

    // Begin transaction
    let tx_id = wal.begin_transaction().await?;
    println!("   Transaction started: {tx_id}");

    // Log updates
    let before = vec![0; 256];
    let after = vec![42; 256];

    let lsn = wal.log_update(tx_id, page_id, 0, before, after).await?;
    println!("   Update logged at LSN: {lsn}");

    // Commit
    wal.commit_transaction(tx_id).await?;
    println!("   ✅ Transaction committed");

    Ok(())
}

async fn demo_concurrent_transactions(
    wal: &WALManager,
    pager: &Arc<PageStorageManager>,
) -> anyhow::Result<()> {
    println!("🔀 Demo 2: Concurrent Transactions");
    println!("{}", "-".repeat(40));

    let mut handles = Vec::new();

    for i in 0..3 {
        let wal_clone = Arc::new(wal.clone());
        let pager_clone = Arc::clone(pager);

        let handle = tokio::spawn(async move {
            // Allocate page
            let page_id = pager_clone.allocate_page(PageType::Data).await?;

            // Begin transaction
            let tx_id = wal_clone.begin_transaction().await?;

            // Multiple updates
            for j in 0..5 {
                let before = vec![0; 128];
                let after = vec![(i * 10 + j) as u8; 128];

                wal_clone
                    .log_update(tx_id, page_id, j * 128, before, after)
                    .await?;
            }

            // Commit
            wal_clone.commit_transaction(tx_id).await?;

            Ok::<_, anyhow::Error>((tx_id, page_id))
        });

        handles.push(handle);
    }

    // Wait for all transactions
    for (i, handle) in handles.into_iter().enumerate() {
        let (tx_id, page_id) = handle.await??;
        println!(
            "   Transaction {} committed: {} (page: {})",
            i + 1,
            tx_id,
            page_id.0
        );
    }

    println!("   ✅ All concurrent transactions completed");

    Ok(())
}

async fn demo_transaction_abort(
    wal: &WALManager,
    pager: &Arc<PageStorageManager>,
) -> anyhow::Result<()> {
    println!("↩️  Demo 3: Transaction Abort");
    println!("{}", "-".repeat(40));

    let page_id = pager.allocate_page(PageType::Data).await?;
    let tx_id = wal.begin_transaction().await?;

    println!("   Transaction started: {tx_id}");

    // Log some updates
    for i in 0..3 {
        let before = vec![0; 100];
        let after = vec![99; 100];
        wal.log_update(tx_id, page_id, i * 100, before, after)
            .await?;
    }

    println!("   Logged 3 updates");

    // Abort instead of commit
    wal.abort_transaction(tx_id).await?;
    println!("   ⚠️  Transaction aborted");

    Ok(())
}

async fn demo_checkpoint(wal: &WALManager) -> anyhow::Result<()> {
    println!("🛑 Demo 4: Checkpoint");
    println!("{}", "-".repeat(40));

    let tx_table_before = wal.get_transaction_table().await;
    println!("   Active transactions before: {}", tx_table_before.len());

    let checkpoint_lsn = wal.checkpoint().await?;
    println!("   ✅ Checkpoint completed at LSN: {checkpoint_lsn}");

    let dirty_pages = wal.get_dirty_page_table().await;
    println!("   Dirty pages after checkpoint: {}", dirty_pages.len());

    Ok(())
}

async fn demo_crash_recovery(
    wal: &WALManager,
    pager: &Arc<PageStorageManager>,
) -> anyhow::Result<()> {
    println!("🔄 Demo 5: Crash Recovery Simulation");
    println!("{}", "-".repeat(40));

    // Simulate some transactions
    let page_id = pager.allocate_page(PageType::Data).await?;

    // Committed transaction
    let tx1 = wal.begin_transaction().await?;
    wal.log_update(tx1, page_id, 0, vec![0; 100], vec![1; 100])
        .await?;
    wal.commit_transaction(tx1).await?;
    println!("   Simulated committed transaction: {tx1}");

    // Uncommitted transaction (simulates crash)
    let tx2 = wal.begin_transaction().await?;
    wal.log_update(tx2, page_id, 100, vec![0; 100], vec![2; 100])
        .await?;
    println!("   Simulated uncommitted transaction: {tx2} (crash!)");

    // Perform recovery
    println!("   Starting ARIES recovery...");
    let stats: RecoveryStats = wal.recover(Arc::clone(pager)).await?;

    println!("   ✅ Recovery completed:");
    println!("      - Records analyzed: {}", stats.records_analyzed);
    println!("      - Redo operations: {}", stats.redo_operations);
    println!("      - Undo operations: {}", stats.undo_operations);
    println!(
        "      - Transactions committed: {}",
        stats.transactions_committed
    );
    println!(
        "      - Transactions aborted: {}",
        stats.transactions_aborted
    );
    println!("      - Recovery time: {}ms", stats.recovery_time_ms);

    Ok(())
}
