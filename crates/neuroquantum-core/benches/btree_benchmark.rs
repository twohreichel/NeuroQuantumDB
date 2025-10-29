//! B+ Tree Performance Benchmarks
//!
//! Run with: cargo bench --features benchmarks --bench btree_benchmark

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use neuroquantum_core::storage::btree::BTree;
use std::hint::black_box;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn btree_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_insert_sequential");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let rt = Runtime::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                    for i in 0..size {
                        let key = format!("key{:010}", i).into_bytes();
                        btree.insert(key, i as u64).await.unwrap();
                    }

                    black_box(btree)
                })
            });
        });
    }

    group.finish();
}

fn btree_insert_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_insert_random");

    for size in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let rt = Runtime::new().unwrap();

            // Pre-generate random keys
            let keys: Vec<Vec<u8>> = (0..size)
                .map(|i| format!("key{:010}", (i * 7919) % size).into_bytes())
                .collect();

            b.iter(|| {
                rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                    for (i, key) in keys.iter().enumerate() {
                        btree.insert(key.clone(), i as u64).await.unwrap();
                    }

                    black_box(btree)
                })
            });
        });
    }

    group.finish();
}

fn btree_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_search");
    group.measurement_time(Duration::from_secs(10));

    for size in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let rt = Runtime::new().unwrap();

            // Setup: create tree with data
            let (temp_dir, btree) = rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                for i in 0..size {
                    let key = format!("key{:010}", i).into_bytes();
                    btree.insert(key, i as u64).await.unwrap();
                }

                (temp_dir, btree)
            });

            // Benchmark: search for keys
            let mut counter = 0;
            b.iter(|| {
                let key = format!("key{:010}", counter % size).into_bytes();
                counter += 1;
                rt.block_on(async { black_box(btree.search(&key).await.unwrap()) })
            });

            drop(temp_dir);
        });
    }

    group.finish();
}

fn btree_range_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_range_scan");

    let tree_size = 100_000;
    for scan_size in [10, 100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*scan_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(scan_size),
            scan_size,
            |b, &scan_size| {
                let rt = Runtime::new().unwrap();

                // Setup: create tree with data
                let (temp_dir, btree) = rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                    for i in 0..tree_size {
                        let key = format!("key{:010}", i).into_bytes();
                        btree.insert(key, i as u64).await.unwrap();
                    }

                    (temp_dir, btree)
                });

                // Benchmark: range scan
                b.iter(|| {
                    rt.block_on(async {
                        let start = format!("key{:010}", 10_000).into_bytes();
                        let end = format!("key{:010}", 10_000 + scan_size).into_bytes();
                        black_box(btree.range_scan(&start, &end).await.unwrap())
                    })
                });

                drop(temp_dir);
            },
        );
    }

    group.finish();
}

fn btree_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_delete");

    for size in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let rt = Runtime::new().unwrap();

            b.iter(|| {
                rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                    // Insert keys
                    for i in 0..size {
                        let key = format!("key{:010}", i).into_bytes();
                        btree.insert(key, i as u64).await.unwrap();
                    }

                    // Delete keys
                    for i in 0..size {
                        let key = format!("key{:010}", i).into_bytes();
                        btree.delete(&key).await.unwrap();
                    }

                    black_box(btree)
                })
            });
        });
    }

    group.finish();
}

fn btree_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_mixed_workload");
    group.measurement_time(Duration::from_secs(15));

    // Simulate real-world workload: 50% reads, 30% inserts, 20% deletes
    group.bench_function("realistic_workload", |b| {
        let rt = Runtime::new().unwrap();

        b.iter(|| {
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                // Initial data
                for i in 0..1000 {
                    let key = format!("key{:010}", i).into_bytes();
                    btree.insert(key, i as u64).await.unwrap();
                }

                // Mixed operations
                for i in 0..1000 {
                    let op = i % 10;

                    if op < 5 {
                        // 50% reads
                        let key = format!("key{:010}", i % 1000).into_bytes();
                        black_box(btree.search(&key).await.unwrap());
                    } else if op < 8 {
                        // 30% inserts
                        let key = format!("key{:010}", 1000 + i).into_bytes();
                        btree.insert(key, i as u64).await.unwrap();
                    } else {
                        // 20% deletes
                        let key = format!("key{:010}", i % 500).into_bytes();
                        black_box(btree.delete(&key).await.unwrap());
                    }
                }

                black_box(btree)
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    btree_insert_sequential,
    btree_insert_random,
    btree_search,
    btree_range_scan,
    btree_delete,
    btree_mixed_workload
);

criterion_main!(benches);
