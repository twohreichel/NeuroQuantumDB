//! Integration tests for WAL system
//!
//! Tests the complete WAL implementation including:
//! - Transaction logging
//! - Commit/Abort operations
//! - Checkpoint creation
//! - Crash recovery (ARIES algorithm)

#[cfg(test)]
mod wal_integration_tests {
    use neuroquantum_core::storage::{
        pager::{PageId, PageStorageManager, PagerConfig, SyncMode},
        wal::{WALConfig, WALManager},
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn setup_wal_test() -> (TempDir, Arc<PageStorageManager>, WALManager) {
        let temp_dir = TempDir::new().unwrap();
        let data_path = temp_dir.path().join("data");
        let wal_path = temp_dir.path().join("wal");

        tokio::fs::create_dir_all(&data_path).await.unwrap();
        tokio::fs::create_dir_all(&wal_path).await.unwrap();

        let pager_config = PagerConfig {
            file_path: data_path.join("test.db"),
            page_size: 4096,
            cache_size: 100,
            sync_mode: SyncMode::None,
        };

        let pager = Arc::new(PageStorageManager::new(pager_config).await.unwrap());

        let wal_config = WALConfig {
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
    async fn test_wal_basic_transaction() {
        let (_temp, _pager, wal) = setup_wal_test().await;

        // Begin transaction
        let tx_id = wal.begin_transaction().await.unwrap();
        assert!(!tx_id.is_nil());

        // Log an update
        let page_id = PageId(1);
        let before = vec![0; 100];
        let after = vec![1; 100];

        let lsn = wal
            .log_update(tx_id, page_id, 0, before, after)
            .await
            .unwrap();

        assert!(lsn > 0);

        // Commit transaction
        wal.commit_transaction(tx_id).await.unwrap();

        println!("✅ Basic transaction test passed");
    }

    #[tokio::test]
    async fn test_wal_multiple_transactions() {
        let (_temp, _pager, wal) = setup_wal_test().await;

        let mut tx_ids = Vec::new();

        // Start multiple transactions
        for _ in 0..5 {
            let tx_id = wal.begin_transaction().await.unwrap();
            tx_ids.push(tx_id);
        }

        // Log updates for each transaction
        for (i, tx_id) in tx_ids.iter().enumerate() {
            let page_id = PageId(i as u64 + 1);
            let before = vec![0; 100];
            let after = vec![(i + 1) as u8; 100];

            wal.log_update(*tx_id, page_id, 0, before, after)
                .await
                .unwrap();
        }

        // Commit all transactions
        for tx_id in &tx_ids {
            wal.commit_transaction(*tx_id).await.unwrap();
        }

        println!("✅ Multiple transactions test passed");
    }

    #[tokio::test]
    async fn test_wal_abort_transaction() {
        let (_temp, _pager, wal) = setup_wal_test().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        let page_id = PageId(1);
        let before = vec![0; 100];
        let after = vec![1; 100];

        wal.log_update(tx_id, page_id, 0, before, after)
            .await
            .unwrap();

        // Abort instead of commit
        wal.abort_transaction(tx_id).await.unwrap();

        println!("✅ Abort transaction test passed");
    }

    #[tokio::test]
    async fn test_wal_checkpoint() {
        let (_temp, _pager, wal) = setup_wal_test().await;

        // Create some transactions
        for i in 0..3 {
            let tx_id = wal.begin_transaction().await.unwrap();
            let page_id = PageId(i + 1);
            let before = vec![0; 100];
            let after = vec![1; 100];

            wal.log_update(tx_id, page_id, 0, before, after)
                .await
                .unwrap();
            wal.commit_transaction(tx_id).await.unwrap();
        }

        // Perform checkpoint
        let checkpoint_lsn = wal.checkpoint().await.unwrap();
        assert!(checkpoint_lsn > 0);

        println!("✅ Checkpoint test passed (LSN: {})", checkpoint_lsn);
    }

    #[tokio::test]
    async fn test_wal_recovery_committed_transaction() {
        let (_temp, pager, wal) = setup_wal_test().await;

        // Start and commit a transaction
        let tx_id = wal.begin_transaction().await.unwrap();
        let page_id = PageId(1);
        let before = vec![0; 100];
        let after = vec![42; 100];

        wal.log_update(tx_id, page_id, 0, before.clone(), after.clone())
            .await
            .unwrap();
        wal.commit_transaction(tx_id).await.unwrap();

        // Simulate crash and recovery
        let stats = wal.recover(Arc::clone(&pager)).await.unwrap();

        assert_eq!(stats.transactions_committed, 1);
        assert_eq!(stats.transactions_aborted, 0);
        assert!(stats.redo_operations >= 1);

        println!("✅ Recovery with committed transaction test passed");
        println!("   Recovery stats: {:?}", stats);
    }

    #[tokio::test]
    async fn test_wal_recovery_uncommitted_transaction() {
        let (_temp, pager, wal) = setup_wal_test().await;

        // Start transaction but don't commit (simulate crash)
        let tx_id = wal.begin_transaction().await.unwrap();
        let page_id = PageId(1);
        let before = vec![0; 100];
        let after = vec![42; 100];

        wal.log_update(tx_id, page_id, 0, before, after)
            .await
            .unwrap();

        // Don't commit - simulate crash

        // Recovery should abort this transaction
        let stats = wal.recover(Arc::clone(&pager)).await.unwrap();

        assert_eq!(stats.transactions_aborted, 1);
        assert!(stats.undo_operations > 0);

        println!("✅ Recovery with uncommitted transaction test passed");
        println!("   Recovery stats: {:?}", stats);
    }

    #[tokio::test]
    async fn test_wal_recovery_with_checkpoint() {
        let (_temp, pager, wal) = setup_wal_test().await;

        // Create transactions before checkpoint
        for i in 0..2 {
            let tx_id = wal.begin_transaction().await.unwrap();
            let page_id = PageId(i + 1);
            wal.log_update(tx_id, page_id, 0, vec![0; 100], vec![1; 100])
                .await
                .unwrap();
            wal.commit_transaction(tx_id).await.unwrap();
        }

        // Checkpoint
        wal.checkpoint().await.unwrap();

        // More transactions after checkpoint
        for i in 2..4 {
            let tx_id = wal.begin_transaction().await.unwrap();
            let page_id = PageId(i + 1);
            wal.log_update(tx_id, page_id, 0, vec![0; 100], vec![2; 100])
                .await
                .unwrap();
            wal.commit_transaction(tx_id).await.unwrap();
        }

        // Recovery should use checkpoint
        let stats = wal.recover(Arc::clone(&pager)).await.unwrap();

        assert!(stats.checkpoint_lsn.is_some());
        assert_eq!(stats.transactions_committed, 4);

        println!("✅ Recovery with checkpoint test passed");
        println!("   Checkpoint LSN: {:?}", stats.checkpoint_lsn);
    }

    #[tokio::test]
    async fn test_wal_lsn_ordering() {
        let (_temp, _pager, wal) = setup_wal_test().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        let mut lsns = Vec::new();

        // Log multiple updates
        for i in 0..10 {
            let page_id = PageId(1);
            let lsn = wal
                .log_update(tx_id, page_id, i * 10, vec![0; 10], vec![1; 10])
                .await
                .unwrap();
            lsns.push(lsn);
        }

        // Verify LSNs are monotonically increasing
        for i in 1..lsns.len() {
            assert!(lsns[i] > lsns[i - 1], "LSNs must be monotonically increasing");
        }

        wal.commit_transaction(tx_id).await.unwrap();

        println!("✅ LSN ordering test passed");
    }

    #[tokio::test]
    async fn test_wal_concurrent_transactions() {
        let (_temp, _pager, wal) = setup_wal_test().await;
        let wal = Arc::new(wal);

        let mut handles = Vec::new();

        // Spawn multiple concurrent transactions
        for i in 0..5 {
            let wal_clone = Arc::clone(&wal);
            let handle = tokio::spawn(async move {
                let tx_id = wal_clone.begin_transaction().await.unwrap();
                let page_id = PageId(i + 1);

                for j in 0..3 {
                    wal_clone
                        .log_update(
                            tx_id,
                            page_id,
                            j * 100,
                            vec![0; 50],
                            vec![(i + j) as u8; 50],
                        )
                        .await
                        .unwrap();
                }

                wal_clone.commit_transaction(tx_id).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all transactions to complete
        for handle in handles {
            handle.await.unwrap();
        }

        println!("✅ Concurrent transactions test passed");
    }

    #[tokio::test]
    async fn test_wal_transaction_table() {
        let (_temp, _pager, wal) = setup_wal_test().await;

        // Start multiple transactions
        let tx1 = wal.begin_transaction().await.unwrap();
        let tx2 = wal.begin_transaction().await.unwrap();

        // Log updates
        wal.log_update(tx1, PageId(1), 0, vec![0; 100], vec![1; 100])
            .await
            .unwrap();
        wal.log_update(tx2, PageId(2), 0, vec![0; 100], vec![2; 100])
            .await
            .unwrap();

        // Check transaction table
        let tx_table = wal.get_transaction_table().await;
        assert_eq!(tx_table.len(), 2);
        assert!(tx_table.contains_key(&tx1));
        assert!(tx_table.contains_key(&tx2));

        // Commit one transaction
        wal.commit_transaction(tx1).await.unwrap();

        let tx_table = wal.get_transaction_table().await;
        assert_eq!(tx_table.len(), 1);
        assert!(!tx_table.contains_key(&tx1));
        assert!(tx_table.contains_key(&tx2));

        println!("✅ Transaction table test passed");
    }

    #[tokio::test]
    async fn test_wal_dirty_page_tracking() {
        let (_temp, _pager, wal) = setup_wal_test().await;

        let tx_id = wal.begin_transaction().await.unwrap();

        // Update multiple pages
        let pages = vec![PageId(1), PageId(2), PageId(3)];

        for page_id in &pages {
            wal.log_update(tx_id, *page_id, 0, vec![0; 100], vec![1; 100])
                .await
                .unwrap();
        }

        // Check dirty page table
        let dirty_pages = wal.get_dirty_page_table().await;
        assert_eq!(dirty_pages.len(), 3);

        for page_id in &pages {
            assert!(dirty_pages.contains_key(page_id));
        }

        wal.commit_transaction(tx_id).await.unwrap();

        println!("✅ Dirty page tracking test passed");
    }
}

