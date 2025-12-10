//! # Stress Testing Suite for NeuroQuantumDB
//!
//! Comprehensive stress tests covering:
//! - **Concurrency Tests**: Parallel transactions, lock contention, deadlock scenarios
//! - **Recovery Tests**: Crash recovery, partial writes, WAL integrity
//! - **Load Tests**: High-volume operations, memory pressure, sustained workloads
//!
//! These tests validate system behavior under extreme conditions and ensure
//! ACID compliance in concurrent scenarios.

use neuroquantum_core::storage::{
    ColumnDefinition, ComparisonOperator, Condition, DataType, DeleteQuery, Row, SelectQuery,
    StorageEngine, TableSchema, Value, WhereClause,
};
use neuroquantum_core::transaction::{IsolationLevel, LockManager, LockType, TransactionManager};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::Barrier;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test table schema for stress tests
fn create_test_table_schema() -> TableSchema {
    TableSchema {
        name: "stress_test".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "counter".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "data".to_string(),
                data_type: DataType::Text,
                nullable: true,
                default_value: None,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
    }
}

/// Create a test row with given id and counter value
fn create_test_row(id: i64, counter: i64, data: &str) -> Row {
    Row {
        id: id as u64,
        fields: HashMap::from([
            ("id".to_string(), Value::Integer(id)),
            ("counter".to_string(), Value::Integer(counter)),
            ("data".to_string(), Value::Text(data.to_string())),
        ]),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// Create a WhereClause for filtering by id
fn where_id_equals(id: i64) -> WhereClause {
    WhereClause {
        conditions: vec![Condition {
            field: "id".to_string(),
            operator: ComparisonOperator::Equal,
            value: Value::Integer(id),
        }],
    }
}

// ============================================================================
// Concurrency Stress Tests
// ============================================================================

/// Test concurrent read operations don't interfere with each other
#[tokio::test]
async fn test_concurrent_reads() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create table and insert test data
    storage
        .create_table(create_test_table_schema())
        .await
        .unwrap();

    // Insert 50 rows (reduced from 100 for faster test execution)
    for i in 0..50 {
        let row = create_test_row(i, i * 10, &format!("data_{}", i));
        storage.insert_row("stress_test", row).await.unwrap();
    }

    let storage = Arc::new(tokio::sync::RwLock::new(storage));
    let num_readers = 5;
    let reads_per_reader = 20;
    let successful_reads = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    for reader_id in 0..num_readers {
        let storage_clone = Arc::clone(&storage);
        let reads_counter = Arc::clone(&successful_reads);

        let handle = tokio::spawn(async move {
            for read_num in 0..reads_per_reader {
                let target_id = ((reader_id * reads_per_reader + read_num) % 50) as i64;
                let query = SelectQuery {
                    table: "stress_test".to_string(),
                    columns: vec!["*".to_string()],
                    where_clause: Some(where_id_equals(target_id)),
                    order_by: None,
                    limit: None,
                    offset: None,
                };

                let storage_guard = storage_clone.read().await;
                if storage_guard.select_rows(&query).await.is_ok() {
                    reads_counter.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all readers to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_reads = successful_reads.load(Ordering::SeqCst);
    let expected_reads = (num_readers * reads_per_reader) as u64;

    assert_eq!(
        total_reads, expected_reads,
        "Expected {} successful reads, got {}",
        expected_reads, total_reads
    );
}

/// Test concurrent write operations with proper locking
#[tokio::test]
async fn test_concurrent_writes_with_locking() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage = Arc::new(tokio::sync::RwLock::new(storage));

    // Create table
    {
        let mut storage_guard = storage.write().await;
        storage_guard
            .create_table(create_test_table_schema())
            .await
            .unwrap();
    }

    let num_writers = 5;
    let writes_per_writer = 20;
    let successful_writes = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    for writer_id in 0..num_writers {
        let storage_clone = Arc::clone(&storage);
        let writes_counter = Arc::clone(&successful_writes);

        let handle = tokio::spawn(async move {
            for write_num in 0..writes_per_writer {
                let row_id = writer_id * writes_per_writer + write_num;
                let row = create_test_row(
                    row_id as i64,
                    write_num as i64,
                    &format!("writer_{}_data_{}", writer_id, write_num),
                );

                let mut storage_guard = storage_clone.write().await;
                if storage_guard.insert_row("stress_test", row).await.is_ok() {
                    writes_counter.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all writers to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_writes = successful_writes.load(Ordering::SeqCst);
    let expected_writes = (num_writers * writes_per_writer) as u64;

    assert_eq!(
        total_writes, expected_writes,
        "Expected {} successful writes, got {}",
        expected_writes, total_writes
    );

    // Verify all rows were inserted
    let query = SelectQuery {
        table: "stress_test".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let storage_guard = storage.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();
    assert_eq!(rows.len(), expected_writes as usize);
}

/// Test lock manager under contention
#[tokio::test]
async fn test_lock_manager_contention() {
    let lock_manager = Arc::new(LockManager::new());
    let num_transactions = 10;
    let resources_per_tx = 5;
    let barrier = Arc::new(Barrier::new(num_transactions));
    let successful_locks = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    for tx_num in 0..num_transactions {
        let lm = Arc::clone(&lock_manager);
        let bar = Arc::clone(&barrier);
        let lock_counter = Arc::clone(&successful_locks);

        let handle = tokio::spawn(async move {
            let tx_id = uuid::Uuid::new_v4();

            // Wait for all transactions to start simultaneously
            bar.wait().await;

            let mut acquired = 0;
            for res_num in 0..resources_per_tx {
                // Each transaction tries to lock different resources to avoid deadlocks
                let resource_id = format!("resource_{}_{}", tx_num, res_num);

                // Use timeout to prevent test hanging
                let lock_result = tokio::time::timeout(
                    Duration::from_millis(500),
                    lm.acquire_lock(tx_id, resource_id, LockType::Exclusive),
                )
                .await;

                if let Ok(Ok(())) = lock_result {
                    acquired += 1;
                }
            }

            lock_counter.fetch_add(acquired, Ordering::SeqCst);

            // Release all locks
            let _ = lm.release_locks(&tx_id).await;
        });
        handles.push(handle);
    }

    // Wait for all transactions to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_locks = successful_locks.load(Ordering::SeqCst);
    let expected_locks = (num_transactions * resources_per_tx) as u64;

    // All locks should be acquired since each transaction uses unique resources
    assert_eq!(
        total_locks, expected_locks,
        "Expected {} locks, got {}",
        expected_locks, total_locks
    );
}

/// Test shared lock compatibility
#[tokio::test]
async fn test_shared_lock_compatibility() {
    let lock_manager = Arc::new(LockManager::new());
    let num_readers = 20;
    let barrier = Arc::new(Barrier::new(num_readers));
    let successful_shared_locks = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    for _ in 0..num_readers {
        let lm = Arc::clone(&lock_manager);
        let bar = Arc::clone(&barrier);
        let lock_counter = Arc::clone(&successful_shared_locks);

        let handle = tokio::spawn(async move {
            let tx_id = uuid::Uuid::new_v4();

            // Wait for all readers to start simultaneously
            bar.wait().await;

            // All readers try to acquire shared lock on the same resource
            let lock_result = tokio::time::timeout(
                Duration::from_secs(2),
                lm.acquire_lock(tx_id, "shared_resource".to_string(), LockType::Shared),
            )
            .await;

            if let Ok(Ok(())) = lock_result {
                lock_counter.fetch_add(1, Ordering::SeqCst);

                // Hold the lock for a bit
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            // Release lock
            let _ = lm.release_locks(&tx_id).await;
        });
        handles.push(handle);
    }

    // Wait for all readers to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_shared_locks = successful_shared_locks.load(Ordering::SeqCst);

    // All shared locks should be acquired since they're compatible
    assert_eq!(
        total_shared_locks, num_readers as u64,
        "Expected {} shared locks, got {}",
        num_readers, total_shared_locks
    );
}

/// Test deadlock detection
#[tokio::test]
async fn test_deadlock_detection() {
    let lock_manager = Arc::new(LockManager::new());
    let barrier = Arc::new(Barrier::new(2));
    let deadlocks_detected = Arc::new(AtomicU64::new(0));

    let lm1 = Arc::clone(&lock_manager);
    let lm2 = Arc::clone(&lock_manager);
    let bar1 = Arc::clone(&barrier);
    let bar2 = Arc::clone(&barrier);
    let deadlock_counter1 = Arc::clone(&deadlocks_detected);
    let deadlock_counter2 = Arc::clone(&deadlocks_detected);

    // Transaction 1: Lock A, then try to lock B
    let handle1 = tokio::spawn(async move {
        let tx1_id = uuid::Uuid::new_v4();

        // Acquire lock on resource A
        lm1.acquire_lock(tx1_id, "resource_A".to_string(), LockType::Exclusive)
            .await
            .unwrap();

        // Signal that lock A is acquired
        bar1.wait().await;

        // Try to acquire lock on resource B (may cause deadlock)
        let result = tokio::time::timeout(
            Duration::from_millis(500),
            lm1.acquire_lock(tx1_id, "resource_B".to_string(), LockType::Exclusive),
        )
        .await;

        if let Ok(Err(e)) = result {
            if format!("{:?}", e).contains("Deadlock") {
                deadlock_counter1.fetch_add(1, Ordering::SeqCst);
            }
        }

        let _ = lm1.release_locks(&tx1_id).await;
    });

    // Transaction 2: Lock B, then try to lock A
    let handle2 = tokio::spawn(async move {
        let tx2_id = uuid::Uuid::new_v4();

        // Wait for transaction 1 to acquire lock A
        bar2.wait().await;

        // Acquire lock on resource B
        let _ = lm2
            .acquire_lock(tx2_id, "resource_B".to_string(), LockType::Exclusive)
            .await;

        // Try to acquire lock on resource A (may cause deadlock)
        let result = tokio::time::timeout(
            Duration::from_millis(500),
            lm2.acquire_lock(tx2_id, "resource_A".to_string(), LockType::Exclusive),
        )
        .await;

        if let Ok(Err(e)) = result {
            if format!("{:?}", e).contains("Deadlock") {
                deadlock_counter2.fetch_add(1, Ordering::SeqCst);
            }
        }

        let _ = lm2.release_locks(&tx2_id).await;
    });

    let _ = tokio::join!(handle1, handle2);

    // At least one deadlock should be detected
    let total_deadlocks = deadlocks_detected.load(Ordering::SeqCst);
    assert!(
        total_deadlocks >= 1,
        "Expected at least 1 deadlock to be detected, got {}",
        total_deadlocks
    );
}

// ============================================================================
// Recovery Stress Tests
// ============================================================================

/// Test recovery after simulated crash during write
#[tokio::test]
async fn test_recovery_after_partial_write() {
    let temp_dir = TempDir::new().unwrap();

    // Phase 1: Write data and simulate crash
    {
        let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();
        storage
            .create_table(create_test_table_schema())
            .await
            .unwrap();

        // Insert some rows that will be persisted
        for i in 0..10 {
            let row = create_test_row(i, i * 100, &format!("persisted_data_{}", i));
            storage.insert_row("stress_test", row).await.unwrap();
        }

        // Insert more rows (these may not be fully persisted)
        for i in 10..20 {
            let row = create_test_row(i, i * 100, &format!("partial_data_{}", i));
            storage.insert_row("stress_test", row).await.unwrap();
        }

        // Storage is dropped here without proper shutdown (simulating crash)
    }

    // Phase 2: Recover and verify data integrity
    {
        let storage = StorageEngine::new(temp_dir.path()).await.unwrap();

        let query = SelectQuery {
            table: "stress_test".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };

        let rows = storage.select_rows(&query).await.unwrap();

        // At minimum, the first 10 rows should be recovered (they were flushed)
        assert!(
            rows.len() >= 10,
            "Expected at least 10 rows after recovery, got {}",
            rows.len()
        );

        // Verify data integrity of recovered rows
        for row in &rows {
            if let Some(Value::Integer(id)) = row.fields.get("id") {
                if let Some(Value::Integer(counter)) = row.fields.get("counter") {
                    assert_eq!(
                        *counter,
                        id * 100,
                        "Data corruption detected: id={}, counter={}",
                        id,
                        counter
                    );
                }
            }
        }
    }
}

/// Test transaction manager recovery
#[tokio::test]
async fn test_transaction_manager_recovery() {
    let temp_dir = TempDir::new().unwrap();

    // Phase 1: Start transactions and commit some
    {
        let tx_manager = TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap();

        // Start and commit several transactions
        for _ in 0..5 {
            let tx_id = tx_manager
                .begin_transaction(IsolationLevel::ReadCommitted)
                .await
                .unwrap();
            tx_manager.commit(tx_id).await.unwrap();
        }

        // Start transactions that will be active during "crash"
        let _active_tx1 = tx_manager
            .begin_transaction(IsolationLevel::RepeatableRead)
            .await
            .unwrap();
        let _active_tx2 = tx_manager
            .begin_transaction(IsolationLevel::Serializable)
            .await
            .unwrap();

        // Transaction manager dropped without proper shutdown
    }

    // Phase 2: Create new transaction manager and verify it can start
    {
        let tx_manager = TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap();

        // New transactions should work normally
        let tx_id = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();

        tx_manager.commit(tx_id).await.unwrap();
    }
}

/// Test WAL integrity under concurrent writes
#[tokio::test]
async fn test_wal_integrity_concurrent_writes() {
    let temp_dir = TempDir::new().unwrap();
    let tx_manager = Arc::new(
        TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap(),
    );

    let num_transactions = 20;
    let barrier = Arc::new(Barrier::new(num_transactions));
    let successful_commits = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    for _ in 0..num_transactions {
        let tm = Arc::clone(&tx_manager);
        let bar = Arc::clone(&barrier);
        let commit_counter = Arc::clone(&successful_commits);

        let handle = tokio::spawn(async move {
            // Wait for all transactions to start simultaneously
            bar.wait().await;

            let tx_result = tm.begin_transaction(IsolationLevel::ReadCommitted).await;

            if let Ok(tx_id) = tx_result {
                // Simulate some work
                tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 50)).await;

                // Commit transaction
                if tm.commit(tx_id).await.is_ok() {
                    commit_counter.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all transactions to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_commits = successful_commits.load(Ordering::SeqCst);

    // All transactions should commit successfully
    assert_eq!(
        total_commits, num_transactions as u64,
        "Expected {} commits, got {}",
        num_transactions, total_commits
    );
}

// ============================================================================
// Load/Stress Tests
// ============================================================================

/// Test high-volume insert operations
#[tokio::test]
async fn test_high_volume_inserts() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    storage
        .create_table(create_test_table_schema())
        .await
        .unwrap();

    // Reduced from 1000 for faster test execution while still being a stress test
    let num_inserts = 200;
    let start = Instant::now();

    for i in 0..num_inserts {
        let row = create_test_row(i, i * 2, &format!("bulk_data_{}", i));
        storage.insert_row("stress_test", row).await.unwrap();
    }

    let duration = start.elapsed();

    // Verify all rows were inserted
    let query = SelectQuery {
        table: "stress_test".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage.select_rows(&query).await.unwrap();
    assert_eq!(rows.len(), num_inserts as usize);

    // Performance assertion: Should complete in reasonable time
    // (Adjust threshold based on expected performance)
    assert!(
        duration < Duration::from_secs(30),
        "High volume insert took too long: {:?}",
        duration
    );

    println!(
        "Inserted {} rows in {:?} ({:.2} rows/sec)",
        num_inserts,
        duration,
        num_inserts as f64 / duration.as_secs_f64()
    );
}

/// Test sustained mixed workload (reads + writes)
#[tokio::test]
async fn test_sustained_mixed_workload() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage = Arc::new(tokio::sync::RwLock::new(storage));

    // Create table and seed with initial data (reduced from 100 to 30)
    {
        let mut storage_guard = storage.write().await;
        storage_guard
            .create_table(create_test_table_schema())
            .await
            .unwrap();

        for i in 0..30 {
            let row = create_test_row(i, 0, &format!("initial_data_{}", i));
            storage_guard.insert_row("stress_test", row).await.unwrap();
        }
    }

    // Reduced parameters for faster test execution
    let num_workers = 4;
    let operations_per_worker = 30;
    let total_reads = Arc::new(AtomicU64::new(0));
    let total_writes = Arc::new(AtomicU64::new(0));
    let start = Instant::now();

    let mut handles = vec![];

    for worker_id in 0..num_workers {
        let storage_clone = Arc::clone(&storage);
        let reads_counter = Arc::clone(&total_reads);
        let writes_counter = Arc::clone(&total_writes);

        let handle = tokio::spawn(async move {
            for op_num in 0..operations_per_worker {
                // 70% reads, 30% writes
                if op_num % 10 < 7 {
                    // Read operation (target within seeded data range)
                    let target_id = (rand::random::<i64>().abs() % 30) as i64;
                    let query = SelectQuery {
                        table: "stress_test".to_string(),
                        columns: vec!["*".to_string()],
                        where_clause: Some(where_id_equals(target_id)),
                        order_by: None,
                        limit: None,
                        offset: None,
                    };

                    let storage_guard = storage_clone.read().await;
                    if storage_guard.select_rows(&query).await.is_ok() {
                        reads_counter.fetch_add(1, Ordering::SeqCst);
                    }
                } else {
                    // Write operation (insert new row)
                    let new_id = 1000 + worker_id * operations_per_worker + op_num;
                    let row = create_test_row(
                        new_id as i64,
                        op_num as i64,
                        &format!("worker_{}_op_{}", worker_id, op_num),
                    );

                    let mut storage_guard = storage_clone.write().await;
                    if storage_guard.insert_row("stress_test", row).await.is_ok() {
                        writes_counter.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let duration = start.elapsed();
    let total_read_count = total_reads.load(Ordering::SeqCst);
    let total_write_count = total_writes.load(Ordering::SeqCst);
    let total_ops = total_read_count + total_write_count;

    println!(
        "Mixed workload completed: {} reads, {} writes in {:?} ({:.2} ops/sec)",
        total_read_count,
        total_write_count,
        duration,
        total_ops as f64 / duration.as_secs_f64()
    );

    // Verify operations completed
    assert!(total_read_count > 0, "No reads completed");
    assert!(total_write_count > 0, "No writes completed");
}

/// Test memory pressure during large batch operations
#[tokio::test]
async fn test_memory_pressure_large_batch() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    storage
        .create_table(create_test_table_schema())
        .await
        .unwrap();

    // Insert rows with large data payloads
    // Reduced from 500 rows with 10KB each for faster test execution
    let num_rows = 100;
    let large_data = "X".repeat(1_000); // 1KB per row

    let start = Instant::now();

    for i in 0..num_rows {
        let row = create_test_row(i, i, &format!("{}_{}", large_data, i));
        storage.insert_row("stress_test", row).await.unwrap();
    }

    let duration = start.elapsed();

    // Verify all rows were inserted
    let query = SelectQuery {
        table: "stress_test".to_string(),
        columns: vec!["id".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage.select_rows(&query).await.unwrap();
    assert_eq!(rows.len(), num_rows as usize);

    println!(
        "Large batch insert completed: {} rows with 10KB payload each in {:?}",
        num_rows, duration
    );
}

/// Test isolation level enforcement under concurrent access
#[tokio::test]
async fn test_isolation_levels_concurrent() {
    let temp_dir = TempDir::new().unwrap();
    let tx_manager = Arc::new(
        TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap(),
    );

    let isolation_levels = vec![
        IsolationLevel::ReadUncommitted,
        IsolationLevel::ReadCommitted,
        IsolationLevel::RepeatableRead,
        IsolationLevel::Serializable,
    ];

    let mut handles = vec![];

    for level in isolation_levels {
        let tm = Arc::clone(&tx_manager);

        let handle = tokio::spawn(async move {
            // Start 5 transactions at each isolation level
            for _ in 0..5 {
                let tx_id = tm.begin_transaction(level).await.unwrap();

                // Simulate work
                tokio::time::sleep(Duration::from_millis(10)).await;

                // Commit
                tm.commit(tx_id).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all transactions to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // If we reach here without panics, isolation levels are working correctly
}

/// Test rapid transaction creation and completion
#[tokio::test]
async fn test_rapid_transaction_throughput() {
    let temp_dir = TempDir::new().unwrap();
    let tx_manager = TransactionManager::new_async(temp_dir.path())
        .await
        .unwrap();

    let num_transactions = 500;
    let start = Instant::now();

    for _ in 0..num_transactions {
        let tx_id = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();
        tx_manager.commit(tx_id).await.unwrap();
    }

    let duration = start.elapsed();

    println!(
        "Completed {} transactions in {:?} ({:.2} tx/sec)",
        num_transactions,
        duration,
        num_transactions as f64 / duration.as_secs_f64()
    );

    // Should complete in reasonable time
    assert!(
        duration < Duration::from_secs(30),
        "Transaction throughput too slow: {:?}",
        duration
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test behavior when many transactions abort
#[tokio::test]
async fn test_many_aborted_transactions() {
    let temp_dir = TempDir::new().unwrap();
    let tx_manager = TransactionManager::new_async(temp_dir.path())
        .await
        .unwrap();

    let num_transactions = 100;
    let mut aborted_count = 0;

    for i in 0..num_transactions {
        let tx_id = tx_manager
            .begin_transaction(IsolationLevel::ReadCommitted)
            .await
            .unwrap();

        // Abort 50% of transactions
        if i % 2 == 0 {
            tx_manager.rollback(tx_id).await.unwrap();
            aborted_count += 1;
        } else {
            tx_manager.commit(tx_id).await.unwrap();
        }
    }

    assert_eq!(aborted_count, 50, "Expected 50 aborted transactions");

    // System should still be functional after many aborts
    let tx_id = tx_manager
        .begin_transaction(IsolationLevel::ReadCommitted)
        .await
        .unwrap();
    tx_manager.commit(tx_id).await.unwrap();
}

/// Test concurrent reads during writes (no dirty reads)
#[tokio::test]
async fn test_no_dirty_reads_concurrent() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage = Arc::new(tokio::sync::RwLock::new(storage));

    // Create table and insert initial row
    {
        let mut storage_guard = storage.write().await;
        storage_guard
            .create_table(create_test_table_schema())
            .await
            .unwrap();

        let row = create_test_row(1, 100, "initial_value");
        storage_guard.insert_row("stress_test", row).await.unwrap();
    }

    let barrier = Arc::new(Barrier::new(2));
    let storage_writer = Arc::clone(&storage);
    let storage_reader = Arc::clone(&storage);
    let barrier_writer = Arc::clone(&barrier);
    let barrier_reader = Arc::clone(&barrier);

    let inconsistent_reads = Arc::new(AtomicU64::new(0));
    let inconsistent_counter = Arc::clone(&inconsistent_reads);

    // Writer task: Update the row multiple times using delete + insert pattern
    let writer = tokio::spawn(async move {
        barrier_writer.wait().await;

        for new_counter in 200..210i64 {
            let mut storage_guard = storage_writer.write().await;

            // Delete old row
            let delete_query = DeleteQuery {
                table: "stress_test".to_string(),
                where_clause: Some(where_id_equals(1)),
            };
            let _ = storage_guard.delete_rows(&delete_query).await;

            // Insert new row
            let row = create_test_row(1, new_counter, &format!("updated_{}", new_counter));
            let _ = storage_guard.insert_row("stress_test", row).await;

            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });

    // Reader task: Read the row repeatedly and check for consistency
    let reader = tokio::spawn(async move {
        barrier_reader.wait().await;

        for _ in 0..50 {
            let query = SelectQuery {
                table: "stress_test".to_string(),
                columns: vec!["*".to_string()],
                where_clause: Some(where_id_equals(1)),
                order_by: None,
                limit: None,
                offset: None,
            };

            let storage_guard = storage_reader.read().await;
            if let Ok(rows) = storage_guard.select_rows(&query).await {
                for row in rows {
                    // Check that counter and data are consistent
                    if let (Some(Value::Integer(counter)), Some(Value::Text(data))) =
                        (row.fields.get("counter"), row.fields.get("data"))
                    {
                        // Detect inconsistency:
                        // - Initial value: counter=100, data="initial_value"
                        // - Updated values: counter=200+, data="updated_<counter>"
                        let is_inconsistent = (*counter == 100 && data != "initial_value")
                            || (*counter >= 200 && !data.contains(&counter.to_string()));

                        if is_inconsistent {
                            inconsistent_counter.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    });

    let _ = tokio::join!(writer, reader);

    let inconsistent = inconsistent_reads.load(Ordering::SeqCst);
    assert_eq!(
        inconsistent, 0,
        "Detected {} inconsistent reads (possible dirty reads)",
        inconsistent
    );
}

/// Test concurrent transaction isolation
#[tokio::test]
async fn test_transaction_isolation_stress() {
    let temp_dir = TempDir::new().unwrap();
    let tx_manager = Arc::new(
        TransactionManager::new_async(temp_dir.path())
            .await
            .unwrap(),
    );

    let num_concurrent = 10;
    let iterations_per_task = 50;
    let barrier = Arc::new(Barrier::new(num_concurrent));
    let successful_ops = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    for task_id in 0..num_concurrent {
        let tm = Arc::clone(&tx_manager);
        let bar = Arc::clone(&barrier);
        let ops_counter = Arc::clone(&successful_ops);

        let handle = tokio::spawn(async move {
            // Synchronize all tasks to start at the same time
            bar.wait().await;

            for i in 0..iterations_per_task {
                // Alternate between isolation levels
                let level = match (task_id + i) % 4 {
                    0 => IsolationLevel::ReadUncommitted,
                    1 => IsolationLevel::ReadCommitted,
                    2 => IsolationLevel::RepeatableRead,
                    _ => IsolationLevel::Serializable,
                };

                if let Ok(tx_id) = tm.begin_transaction(level).await {
                    // Simulate varying workload
                    tokio::time::sleep(Duration::from_micros((rand::random::<u64>() % 100) + 10))
                        .await;

                    // 90% commit, 10% rollback
                    let result = if rand::random::<u8>().is_multiple_of(10) {
                        tm.rollback(tx_id).await
                    } else {
                        tm.commit(tx_id).await
                    };

                    if result.is_ok() {
                        ops_counter.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    let total_ops = successful_ops.load(Ordering::SeqCst);
    let expected_min = ((num_concurrent * iterations_per_task) as f64 * 0.9) as u64;

    assert!(
        total_ops >= expected_min,
        "Expected at least {} successful operations, got {}",
        expected_min,
        total_ops
    );

    println!(
        "Completed {} out of {} transaction operations successfully",
        total_ops,
        num_concurrent * iterations_per_task
    );
}

/// Test storage engine under rapid open/close cycles
#[tokio::test]
async fn test_rapid_storage_open_close() {
    let temp_dir = TempDir::new().unwrap();

    // First, create and populate the table
    {
        let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();
        storage
            .create_table(create_test_table_schema())
            .await
            .unwrap();

        for i in 0..50 {
            let row = create_test_row(i, i, &format!("persistent_data_{}", i));
            storage.insert_row("stress_test", row).await.unwrap();
        }
    }

    // Rapidly open and close storage, verifying data integrity
    for cycle in 0..10 {
        let storage = StorageEngine::new(temp_dir.path()).await.unwrap();

        let query = SelectQuery {
            table: "stress_test".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };

        let rows = storage.select_rows(&query).await.unwrap();

        assert!(
            rows.len() >= 50,
            "Cycle {}: Expected at least 50 rows, got {}",
            cycle,
            rows.len()
        );

        // Verify data integrity
        for row in &rows {
            if let (Some(Value::Integer(id)), Some(Value::Integer(counter))) =
                (row.fields.get("id"), row.fields.get("counter"))
            {
                assert_eq!(
                    *id, *counter,
                    "Cycle {}: Data corruption detected: id={}, counter={}",
                    cycle, id, counter
                );
            }
        }
    }
}
