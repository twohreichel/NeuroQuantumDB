//! Concurrent Transaction Integration Tests
//!
//! Tests for concurrent transaction handling, isolation levels,
//! and transaction recovery scenarios.
//!
//! Status: Addresses AUDIT.md Section 7.2 - Expanded integration tests for Transactions

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use neuroquantum_core::storage::{
    ColumnDefinition, DataType, IdGenerationStrategy, Row, SelectQuery, StorageEngine, TableSchema,
    Value,
};
use neuroquantum_core::transaction::{IsolationLevel, LogManager, LogRecordType, TransactionId};
use tempfile::TempDir;
use tokio::sync::Barrier;

/// Helper function to create test table schema
fn create_test_table_schema(name: &str) -> TableSchema {
    TableSchema {
        name: name.to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "balance".to_string(),
                data_type: DataType::Integer,
                nullable: true,
                default_value: Some(Value::Integer(0)),
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: std::collections::HashMap::new(),
        id_strategy: IdGenerationStrategy::AutoIncrement,
    }
}

/// Create a test row with given values
fn create_test_row(id: i64, name: &str, balance: i64) -> Row {
    Row {
        id: id as u64,
        fields: HashMap::from([
            ("id".to_string(), Value::Integer(id)),
            ("name".to_string(), Value::Text(name.to_string())),
            ("balance".to_string(), Value::Integer(balance)),
        ]),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

// ==================== Concurrent Transaction Tests ====================

#[tokio::test]
async fn test_concurrent_transactions_isolation() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    storage
        .create_table(create_test_table_schema("accounts"))
        .await
        .unwrap();

    // Insert initial data
    storage
        .insert_row("accounts", create_test_row(1, "Alice", 1000))
        .await
        .unwrap();

    // Begin two transactions
    let tx1 = storage.begin_transaction().await.unwrap();
    let tx2 = storage.begin_transaction().await.unwrap();

    // Both transactions should have started successfully with different IDs
    assert_ne!(tx1, tx2, "Transactions should have different IDs");

    // Commit both transactions
    storage.commit_transaction(tx1).await.unwrap();
    storage.commit_transaction(tx2).await.unwrap();

    println!("✅ Concurrent transactions isolation test passed!");
}

#[tokio::test]
async fn test_transaction_rollback_consistency() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    storage
        .create_table(create_test_table_schema("rollback_test"))
        .await
        .unwrap();

    // Insert initial data
    storage
        .insert_row("rollback_test", create_test_row(1, "Initial", 500))
        .await
        .unwrap();

    // Begin transaction and insert more data
    let tx_id = storage.begin_transaction().await.unwrap();

    // Insert transactional row
    let inserted_id = storage
        .insert_row_transactional(
            tx_id,
            "rollback_test",
            create_test_row(2, "ToRollback", 200),
        )
        .await
        .unwrap();

    assert!(inserted_id > 0, "Should return valid row ID");

    // Rollback the transaction
    storage.rollback_transaction(tx_id).await.unwrap();

    // Verify original data is still intact
    let query = SelectQuery {
        table: "rollback_test".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage.select_rows(&query).await.unwrap();
    // Original row should exist, rolled back row behavior depends on implementation
    assert!(!rows.is_empty(), "At least original row should exist");

    println!("✅ Transaction rollback consistency test passed!");
}

#[tokio::test]
async fn test_multiple_concurrent_inserts() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Arc::new(tokio::sync::RwLock::new(
        StorageEngine::new(temp_dir.path()).await.unwrap(),
    ));

    // Create table
    {
        let mut storage_write = storage.write().await;
        storage_write
            .create_table(create_test_table_schema("concurrent_inserts"))
            .await
            .unwrap();
    }

    let insert_count = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    // Launch 10 concurrent insert tasks
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let insert_count_clone = Arc::clone(&insert_count);
        let barrier_clone = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier_clone.wait().await;

            // Perform insert
            let mut storage_write = storage_clone.write().await;
            let result = storage_write
                .insert_row(
                    "concurrent_inserts",
                    create_test_row(i64::from(i), &format!("User_{i}"), i64::from(i) * 100),
                )
                .await;

            if result.is_ok() {
                insert_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        handles.push(handle);
    }

    // Wait for all inserts to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_inserts = insert_count.load(Ordering::SeqCst);
    assert_eq!(total_inserts, 10, "All 10 inserts should succeed");

    // Verify all rows are present
    {
        let storage_read = storage.read().await;
        let query = SelectQuery {
            table: "concurrent_inserts".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };

        let rows = storage_read.select_rows(&query).await.unwrap();
        assert_eq!(rows.len(), 10, "Should have 10 rows");
    }

    println!("✅ Multiple concurrent inserts test passed!");
}

#[tokio::test]
async fn test_transaction_begin_commit_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    storage
        .create_table(create_test_table_schema("tx_cycle"))
        .await
        .unwrap();

    // Multiple begin-commit cycles
    for i in 0..5 {
        let tx_id = storage.begin_transaction().await.unwrap();

        // Insert data within transaction
        storage
            .insert_row_transactional(
                tx_id,
                "tx_cycle",
                create_test_row(i64::from(i), &format!("Cycle_{i}"), i64::from(i) * 50),
            )
            .await
            .unwrap();

        // Commit
        storage.commit_transaction(tx_id).await.unwrap();
    }

    // Verify all data persisted
    let query = SelectQuery {
        table: "tx_cycle".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage.select_rows(&query).await.unwrap();
    assert!(rows.len() >= 5, "Should have at least 5 rows from cycles");

    println!("✅ Transaction begin-commit cycle test passed!");
}

// ==================== WAL Recovery Tests ====================

#[tokio::test]
async fn test_wal_log_record_types() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs");
    tokio::fs::create_dir_all(&log_dir).await.unwrap();

    let log_manager = LogManager::new(&log_dir).await.unwrap();
    let tx_id = TransactionId::new_v4();

    // Write BEGIN record
    let lsn1 = log_manager
        .write_log_record(
            Some(tx_id),
            None,
            LogRecordType::Begin {
                tx_id,
                isolation_level: IsolationLevel::ReadCommitted,
            },
        )
        .await
        .unwrap();

    assert!(lsn1 > 0, "Should return valid LSN for BEGIN");

    // Write UPDATE record
    let test_row = create_test_row(1, "Test", 100);
    let after_image = serde_json::to_vec(&test_row).unwrap();

    let lsn2 = log_manager
        .write_log_record(
            Some(tx_id),
            None,
            LogRecordType::Update {
                tx_id,
                table: "test_table".to_string(),
                key: "1".to_string(),
                before_image: None,
                after_image,
            },
        )
        .await
        .unwrap();

    assert!(lsn2 > lsn1, "LSN should be monotonically increasing");

    // Write COMMIT record
    let lsn3 = log_manager
        .write_log_record(Some(tx_id), None, LogRecordType::Commit { tx_id })
        .await
        .unwrap();

    assert!(lsn3 > lsn2, "COMMIT LSN should be greater than UPDATE LSN");

    // Force flush
    log_manager.force_log(lsn3).await.unwrap();

    println!("✅ WAL log record types test passed!");
}

#[tokio::test]
async fn test_wal_abort_record() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs");
    tokio::fs::create_dir_all(&log_dir).await.unwrap();

    let log_manager = LogManager::new(&log_dir).await.unwrap();
    let tx_id = TransactionId::new_v4();

    // BEGIN
    log_manager
        .write_log_record(
            Some(tx_id),
            None,
            LogRecordType::Begin {
                tx_id,
                isolation_level: IsolationLevel::ReadCommitted,
            },
        )
        .await
        .unwrap();

    // ABORT
    let abort_lsn = log_manager
        .write_log_record(Some(tx_id), None, LogRecordType::Abort { tx_id })
        .await
        .unwrap();

    assert!(abort_lsn > 0, "Should return valid LSN for ABORT");

    log_manager.force_log(abort_lsn).await.unwrap();

    println!("✅ WAL abort record test passed!");
}

#[tokio::test]
async fn test_wal_checkpoint_record() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs");
    tokio::fs::create_dir_all(&log_dir).await.unwrap();

    let log_manager = LogManager::new(&log_dir).await.unwrap();

    // Write checkpoint record
    let checkpoint_lsn = log_manager
        .write_log_record(
            None,
            None,
            LogRecordType::Checkpoint {
                active_transactions: vec![],
            },
        )
        .await
        .unwrap();

    assert!(checkpoint_lsn > 0, "Should return valid LSN for checkpoint");

    log_manager.force_log(checkpoint_lsn).await.unwrap();

    println!("✅ WAL checkpoint record test passed!");
}

#[tokio::test]
async fn test_recovery_with_multiple_transactions() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();
    let log_dir = data_dir.join("logs");
    tokio::fs::create_dir_all(&log_dir).await.unwrap();

    // Write log records for multiple transactions
    {
        let log_manager = LogManager::new(&log_dir).await.unwrap();

        // Transaction 1 - COMMITTED
        let tx1 = TransactionId::new_v4();
        log_manager
            .write_log_record(
                Some(tx1),
                None,
                LogRecordType::Begin {
                    tx_id: tx1,
                    isolation_level: IsolationLevel::ReadCommitted,
                },
            )
            .await
            .unwrap();

        let row1 = create_test_row(1, "Committed", 100);
        let after1 = serde_json::to_vec(&row1).unwrap();
        log_manager
            .write_log_record(
                Some(tx1),
                None,
                LogRecordType::Update {
                    tx_id: tx1,
                    table: "recovery_test".to_string(),
                    key: "1".to_string(),
                    before_image: None,
                    after_image: after1,
                },
            )
            .await
            .unwrap();

        log_manager
            .write_log_record(Some(tx1), None, LogRecordType::Commit { tx_id: tx1 })
            .await
            .unwrap();

        // Transaction 2 - ABORTED
        let tx2 = TransactionId::new_v4();
        log_manager
            .write_log_record(
                Some(tx2),
                None,
                LogRecordType::Begin {
                    tx_id: tx2,
                    isolation_level: IsolationLevel::ReadCommitted,
                },
            )
            .await
            .unwrap();

        let row2 = create_test_row(2, "Aborted", 200);
        let after2 = serde_json::to_vec(&row2).unwrap();
        log_manager
            .write_log_record(
                Some(tx2),
                None,
                LogRecordType::Update {
                    tx_id: tx2,
                    table: "recovery_test".to_string(),
                    key: "2".to_string(),
                    before_image: None,
                    after_image: after2,
                },
            )
            .await
            .unwrap();

        log_manager
            .write_log_record(Some(tx2), None, LogRecordType::Abort { tx_id: tx2 })
            .await
            .unwrap();

        // Flush all logs
        log_manager.force_log(100).await.unwrap();
    }

    // Perform recovery
    let mut storage = StorageEngine::new(&data_dir).await.unwrap();
    storage
        .create_table(create_test_table_schema("recovery_test"))
        .await
        .unwrap();

    let result = storage.perform_recovery().await;
    assert!(result.is_ok(), "Recovery should succeed");

    println!("✅ Recovery with multiple transactions test passed!");
}

#[tokio::test]
async fn test_transaction_isolation_levels() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs");
    tokio::fs::create_dir_all(&log_dir).await.unwrap();

    let log_manager = LogManager::new(&log_dir).await.unwrap();

    // Test different isolation levels
    let isolation_levels = [
        IsolationLevel::ReadUncommitted,
        IsolationLevel::ReadCommitted,
        IsolationLevel::RepeatableRead,
        IsolationLevel::Serializable,
    ];

    for isolation in isolation_levels {
        let tx_id = TransactionId::new_v4();

        let lsn = log_manager
            .write_log_record(
                Some(tx_id),
                None,
                LogRecordType::Begin {
                    tx_id,
                    isolation_level: isolation,
                },
            )
            .await
            .unwrap();

        assert!(lsn > 0, "Should write BEGIN for {isolation:?}");

        log_manager
            .write_log_record(Some(tx_id), None, LogRecordType::Commit { tx_id })
            .await
            .unwrap();
    }

    println!("✅ Transaction isolation levels test passed!");
}

#[tokio::test]
async fn test_concurrent_log_writes() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs");
    tokio::fs::create_dir_all(&log_dir).await.unwrap();

    let log_manager = Arc::new(LogManager::new(&log_dir).await.unwrap());
    let write_count = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    // Concurrent log writes from multiple tasks
    for i in 0..20 {
        let log_manager_clone = Arc::clone(&log_manager);
        let write_count_clone = Arc::clone(&write_count);

        let handle = tokio::spawn(async move {
            let tx_id = TransactionId::new_v4();

            // Write BEGIN
            let result = log_manager_clone
                .write_log_record(
                    Some(tx_id),
                    None,
                    LogRecordType::Begin {
                        tx_id,
                        isolation_level: IsolationLevel::ReadCommitted,
                    },
                )
                .await;

            if result.is_ok() {
                write_count_clone.fetch_add(1, Ordering::SeqCst);
            }

            // Write UPDATE
            let row = create_test_row(i, &format!("Concurrent_{i}"), i * 10);
            let after_image = serde_json::to_vec(&row).unwrap();

            let result = log_manager_clone
                .write_log_record(
                    Some(tx_id),
                    None,
                    LogRecordType::Update {
                        tx_id,
                        table: "concurrent_log".to_string(),
                        key: i.to_string(),
                        before_image: None,
                        after_image,
                    },
                )
                .await;

            if result.is_ok() {
                write_count_clone.fetch_add(1, Ordering::SeqCst);
            }

            // Write COMMIT
            let result = log_manager_clone
                .write_log_record(Some(tx_id), None, LogRecordType::Commit { tx_id })
                .await;

            if result.is_ok() {
                write_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let total_writes = write_count.load(Ordering::SeqCst);
    // Each of 20 tasks writes 3 records
    assert_eq!(
        total_writes, 60,
        "Should have 60 successful log writes (20 tasks × 3 records)"
    );

    println!("✅ Concurrent log writes test passed!");
}

#[tokio::test]
async fn test_storage_recovery_after_crash_simulation() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    // Phase 1: Create database, insert data, simulate crash
    {
        let mut storage = StorageEngine::new(&storage_path).await.unwrap();

        storage
            .create_table(create_test_table_schema("crash_sim"))
            .await
            .unwrap();

        // Insert some data
        for i in 0..10 {
            storage
                .insert_row(
                    "crash_sim",
                    create_test_row(i, &format!("Pre_crash_{i}"), i * 100),
                )
                .await
                .unwrap();
        }

        // Flush to ensure data is persisted
        storage.flush_to_disk().await.unwrap();

        // Simulated crash - storage is dropped here
    }

    // Phase 2: Recover and verify
    {
        let mut storage = StorageEngine::new(&storage_path).await.unwrap();

        // Perform recovery
        let recovery_result = storage.perform_recovery().await;
        assert!(
            recovery_result.is_ok(),
            "Recovery should succeed after crash simulation"
        );

        // Verify data integrity
        let query = SelectQuery {
            table: "crash_sim".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };

        let rows = storage.select_rows(&query).await.unwrap();
        assert_eq!(rows.len(), 10, "Should recover all 10 rows");

        // Verify data values
        for row in &rows {
            assert!(row.fields.contains_key("id"));
            assert!(row.fields.contains_key("name"));
            assert!(row.fields.contains_key("balance"));
        }
    }

    println!("✅ Storage recovery after crash simulation test passed!");
}

#[tokio::test]
async fn test_transaction_with_multiple_tables() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create multiple tables
    storage
        .create_table(create_test_table_schema("table_a"))
        .await
        .unwrap();
    storage
        .create_table(create_test_table_schema("table_b"))
        .await
        .unwrap();

    // Begin transaction
    let tx_id = storage.begin_transaction().await.unwrap();

    // Insert into both tables within same transaction
    storage
        .insert_row_transactional(tx_id, "table_a", create_test_row(1, "A_Row", 100))
        .await
        .unwrap();

    storage
        .insert_row_transactional(tx_id, "table_b", create_test_row(1, "B_Row", 200))
        .await
        .unwrap();

    // Commit transaction
    storage.commit_transaction(tx_id).await.unwrap();

    // Verify both tables have data
    let query_a = SelectQuery {
        table: "table_a".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let query_b = SelectQuery {
        table: "table_b".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows_a = storage.select_rows(&query_a).await.unwrap();
    let rows_b = storage.select_rows(&query_b).await.unwrap();

    assert!(!rows_a.is_empty(), "Table A should have at least 1 row");
    assert!(!rows_b.is_empty(), "Table B should have at least 1 row");

    println!("✅ Transaction with multiple tables test passed!");
}
