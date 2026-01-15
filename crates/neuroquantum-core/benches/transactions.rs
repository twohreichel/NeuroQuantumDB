//! Transaction Performance Benchmarks
//!
//! This module benchmarks transaction operations including concurrent transactions,
//! savepoint overhead, and various isolation levels.
//!
//! Run with: cargo bench --features benchmarks --bench transactions

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use neuroquantum_core::transaction::{
    IsolationLevel, LockManager, LockType, Transaction, TransactionManager, LSN,
};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Benchmark single transaction creation and commit
fn bench_transaction_lifecycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_lifecycle");
    group.measurement_time(Duration::from_secs(10));

    let rt = Runtime::new().unwrap();

    // Benchmark transaction creation
    group.bench_function("create_transaction", |b| {
        b.iter(|| {
            let tx = Transaction::new(IsolationLevel::ReadCommitted, 30);
            black_box(tx)
        });
    });

    // Benchmark with different isolation levels
    let isolation_levels = [
        ("read_uncommitted", IsolationLevel::ReadUncommitted),
        ("read_committed", IsolationLevel::ReadCommitted),
        ("repeatable_read", IsolationLevel::RepeatableRead),
        ("serializable", IsolationLevel::Serializable),
    ];

    for (name, isolation) in &isolation_levels {
        group.bench_with_input(
            BenchmarkId::new("isolation", *name),
            isolation,
            |b, &isolation| {
                b.iter(|| {
                    let tx = Transaction::new(isolation, 30);
                    black_box(tx)
                });
            },
        );
    }

    // Benchmark full transaction lifecycle with TransactionManager
    group.bench_function("full_lifecycle", |b| {
        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let manager = TransactionManager::new_async(temp_dir.path())
                    .await
                    .unwrap();

                let tx_id = manager
                    .begin_transaction(IsolationLevel::ReadCommitted)
                    .await
                    .unwrap();
                manager.commit(tx_id).await.unwrap();

                black_box(tx_id)
            })
        });
    });

    group.finish();
}

/// Benchmark concurrent transaction operations
fn bench_concurrent_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_transactions");
    group.measurement_time(Duration::from_secs(15));

    for concurrency in &[10, 50, 100] {
        group.throughput(Throughput::Elements(*concurrency as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &concurrency| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let manager = Arc::new(
                            TransactionManager::new_async(temp_dir.path())
                                .await
                                .unwrap(),
                        );

                        let handles: Vec<_> = (0..concurrency)
                            .map(|_| {
                                let mgr = Arc::clone(&manager);
                                tokio::spawn(async move {
                                    let tx_id = mgr
                                        .begin_transaction(IsolationLevel::ReadCommitted)
                                        .await
                                        .unwrap();
                                    mgr.commit(tx_id).await.unwrap();
                                    tx_id
                                })
                            })
                            .collect();

                        let results: Vec<_> = futures::future::join_all(handles)
                            .await
                            .into_iter()
                            .collect();
                        black_box(results)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent transactions with mixed read/write operations
fn bench_concurrent_mixed_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_mixed_operations");
    group.measurement_time(Duration::from_secs(15));

    for concurrency in &[10, 25, 50] {
        group.throughput(Throughput::Elements(*concurrency as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &concurrency| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let manager = Arc::new(
                            TransactionManager::new_async(temp_dir.path())
                                .await
                                .unwrap(),
                        );

                        let handles: Vec<_> = (0..concurrency)
                            .map(|i| {
                                let mgr = Arc::clone(&manager);
                                tokio::spawn(async move {
                                    let isolation = if i % 2 == 0 {
                                        IsolationLevel::ReadCommitted
                                    } else {
                                        IsolationLevel::RepeatableRead
                                    };

                                    let tx_id = mgr.begin_transaction(isolation).await.unwrap();

                                    // Simulate some work
                                    tokio::time::sleep(Duration::from_micros(10)).await;

                                    // 80% commit, 20% abort
                                    if i % 5 != 0 {
                                        mgr.commit(tx_id).await.unwrap();
                                    } else {
                                        mgr.rollback(tx_id).await.unwrap();
                                    }
                                    tx_id
                                })
                            })
                            .collect();

                        let results: Vec<_> = futures::future::join_all(handles)
                            .await
                            .into_iter()
                            .collect();
                        black_box(results)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark savepoint creation and rollback overhead
fn bench_savepoint_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("savepoint_overhead");
    group.measurement_time(Duration::from_secs(10));

    let rt = Runtime::new().unwrap();

    // Benchmark savepoint creation
    for savepoint_count in &[1, 5, 10, 20] {
        group.throughput(Throughput::Elements(*savepoint_count as u64));

        group.bench_with_input(
            BenchmarkId::new("create", savepoint_count),
            savepoint_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let manager = TransactionManager::new_async(temp_dir.path())
                            .await
                            .unwrap();

                        let tx_id = manager
                            .begin_transaction(IsolationLevel::ReadCommitted)
                            .await
                            .unwrap();

                        let mut savepoint_lsns: Vec<LSN> = Vec::new();
                        for i in 0..count {
                            let name = format!("sp_{i}");
                            let lsn = manager.create_savepoint(tx_id, name).await.unwrap();
                            savepoint_lsns.push(lsn);
                        }

                        manager.commit(tx_id).await.unwrap();
                        black_box(savepoint_lsns)
                    })
                });
            },
        );
    }

    // Benchmark savepoint rollback
    for savepoint_count in &[1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("rollback", savepoint_count),
            savepoint_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let manager = TransactionManager::new_async(temp_dir.path())
                            .await
                            .unwrap();

                        let tx_id = manager
                            .begin_transaction(IsolationLevel::ReadCommitted)
                            .await
                            .unwrap();

                        // Create savepoints and store their LSNs
                        let mut savepoint_lsns: Vec<LSN> = Vec::new();
                        for i in 0..count {
                            let lsn = manager
                                .create_savepoint(tx_id, format!("sp_{i}"))
                                .await
                                .unwrap();
                            savepoint_lsns.push(lsn);
                        }

                        // Rollback to middle savepoint using LSN
                        let middle_idx = count / 2;
                        if middle_idx < savepoint_lsns.len() {
                            let rollback_lsn = savepoint_lsns[middle_idx];
                            manager
                                .rollback_to_savepoint(tx_id, rollback_lsn)
                                .await
                                .unwrap();
                        }

                        manager.commit(tx_id).await.unwrap();
                        black_box(tx_id)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark lock manager performance
fn bench_lock_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("lock_manager");
    group.measurement_time(Duration::from_secs(10));

    let rt = Runtime::new().unwrap();

    // Benchmark single lock acquisition
    group.bench_function("acquire_single_lock", |b| {
        let mut counter = 0u64;

        b.iter(|| {
            let tx_id = Uuid::new_v4();
            let resource = format!("resource_{counter}");
            counter += 1;

            rt.block_on(async {
                let lm = LockManager::new();
                lm.acquire_lock(tx_id, resource, LockType::Shared)
                    .await
                    .unwrap();
                lm.release_locks(&tx_id).await.unwrap();
                black_box(tx_id)
            })
        });
    });

    // Benchmark multiple locks per transaction
    for lock_count in &[5, 10, 20] {
        group.throughput(Throughput::Elements(*lock_count as u64));

        group.bench_with_input(
            BenchmarkId::new("acquire_multiple", lock_count),
            lock_count,
            |b, &count| {
                let mut counter = 0u64;

                b.iter(|| {
                    let tx_id = Uuid::new_v4();
                    let resources: Vec<String> = (0..count)
                        .map(|i| format!("resource_{counter}_{i}"))
                        .collect();
                    counter += 1;

                    rt.block_on(async {
                        let lm = LockManager::new();
                        for resource in resources {
                            lm.acquire_lock(tx_id, resource, LockType::Shared)
                                .await
                                .unwrap();
                        }
                        lm.release_locks(&tx_id).await.unwrap();
                        black_box(tx_id)
                    })
                });
            },
        );
    }

    // Benchmark shared vs exclusive locks
    group.bench_function("shared_lock", |b| {
        let mut counter = 0u64;

        b.iter(|| {
            let tx_id = Uuid::new_v4();
            let resource = format!("shared_{counter}");
            counter += 1;

            rt.block_on(async {
                let lm = LockManager::new();
                lm.acquire_lock(tx_id, resource, LockType::Shared)
                    .await
                    .unwrap();
                lm.release_locks(&tx_id).await.unwrap();
                black_box(tx_id)
            })
        });
    });

    group.bench_function("exclusive_lock", |b| {
        let mut counter = 0u64;

        b.iter(|| {
            let tx_id = Uuid::new_v4();
            let resource = format!("exclusive_{counter}");
            counter += 1;

            rt.block_on(async {
                let lm = LockManager::new();
                lm.acquire_lock(tx_id, resource, LockType::Exclusive)
                    .await
                    .unwrap();
                lm.release_locks(&tx_id).await.unwrap();
                black_box(tx_id)
            })
        });
    });

    group.finish();
}

/// Benchmark lock contention scenarios
fn bench_lock_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("lock_contention");
    group.measurement_time(Duration::from_secs(10));

    let rt = Runtime::new().unwrap();

    // Benchmark sequential lock acquisition on same resource
    for tx_count in &[5, 10, 20] {
        group.throughput(Throughput::Elements(*tx_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(tx_count),
            tx_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let lock_manager = LockManager::new();

                        // Each transaction acquires and releases same resource sequentially
                        for i in 0..count {
                            let tx_id = Uuid::new_v4();
                            let resource = format!("shared_resource_{i}");

                            lock_manager
                                .acquire_lock(tx_id, resource, LockType::Shared)
                                .await
                                .unwrap();
                            lock_manager.release_locks(&tx_id).await.unwrap();
                        }

                        black_box(count)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark transaction throughput under high load
fn bench_transaction_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_throughput");
    group.measurement_time(Duration::from_secs(20));

    let rt = Runtime::new().unwrap();

    for transaction_count in &[100, 500, 1000] {
        group.throughput(Throughput::Elements(*transaction_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(transaction_count),
            transaction_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let temp_dir = TempDir::new().unwrap();
                        let manager = Arc::new(
                            TransactionManager::new_async(temp_dir.path())
                                .await
                                .unwrap(),
                        );

                        // Run transactions with 4 concurrent workers
                        let worker_count = 4;
                        let per_worker = count / worker_count;

                        let handles: Vec<_> = (0..worker_count)
                            .map(|_| {
                                let mgr = Arc::clone(&manager);
                                tokio::spawn(async move {
                                    let mut completed = 0;
                                    for _ in 0..per_worker {
                                        let tx_id = mgr
                                            .begin_transaction(IsolationLevel::ReadCommitted)
                                            .await
                                            .unwrap();
                                        mgr.commit(tx_id).await.unwrap();
                                        completed += 1;
                                    }
                                    completed
                                })
                            })
                            .collect();

                        let results: Vec<_> = futures::future::join_all(handles)
                            .await
                            .into_iter()
                            .collect();
                        black_box(results)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark transaction creation with different timeouts
fn bench_transaction_timeout_settings(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_timeout_settings");
    group.measurement_time(Duration::from_secs(10));

    for timeout in &[10, 30, 60, 120] {
        group.bench_with_input(
            BenchmarkId::from_parameter(timeout),
            timeout,
            |b, &timeout| {
                b.iter(|| {
                    let tx = Transaction::new(IsolationLevel::ReadCommitted, timeout);
                    black_box(tx)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_transaction_lifecycle,
    bench_concurrent_transactions,
    bench_concurrent_mixed_operations,
    bench_savepoint_overhead,
    bench_lock_manager,
    bench_lock_contention,
    bench_transaction_throughput,
    bench_transaction_timeout_settings,
);

criterion_main!(benches);
