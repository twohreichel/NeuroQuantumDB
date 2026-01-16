//! B+ Tree Performance Benchmarks
//!
//! Run with: cargo bench --features benchmarks --bench `btree_benchmark`

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use neuroquantum_core::storage::btree::BTree;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn btree_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_insert_sequential");

    for size in &[100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let rt = Runtime::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                    for i in 0..size {
                        let key = format!("key{i:010}").into_bytes();
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

    for size in &[100, 1_000, 10_000] {
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

    for size in &[1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let rt = Runtime::new().unwrap();

            // Setup: create tree with data
            let (temp_dir, btree) = rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                for i in 0..size {
                    let key = format!("key{i:010}").into_bytes();
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
    for scan_size in &[10, 100, 1_000, 10_000] {
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
                        let key = format!("key{i:010}").into_bytes();
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

    for size in &[100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let rt = Runtime::new().unwrap();

            b.iter(|| {
                rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                    // Insert keys
                    for i in 0..size {
                        let key = format!("key{i:010}").into_bytes();
                        btree.insert(key, i as u64).await.unwrap();
                    }

                    // Delete keys
                    for i in 0..size {
                        let key = format!("key{i:010}").into_bytes();
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
                    let key = format!("key{i:010}").into_bytes();
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

fn btree_compression_leaf_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_compression_leaf");

    use neuroquantum_core::storage::btree::{CompressedLeafNode, LeafNode};

    // Benchmark with different key patterns
    let key_patterns = vec![
        (
            "uuid",
            (0..100)
                .map(|i| format!("550e8400-e29b-41d4-a716-{i:012}").into_bytes())
                .collect::<Vec<_>>(),
        ),
        (
            "timestamp",
            (0..100)
                .map(|i| format!("2024-01-01T12:00:{i:02}.000Z").into_bytes())
                .collect::<Vec<_>>(),
        ),
        (
            "url",
            (0..100)
                .map(|i| format!("https://example.com/api/v1/users/{i}").into_bytes())
                .collect::<Vec<_>>(),
        ),
        (
            "path",
            (0..100)
                .map(|i| format!("/home/user/documents/project/file_{i}.txt").into_bytes())
                .collect::<Vec<_>>(),
        ),
    ];

    for (pattern_name, keys) in key_patterns {
        // Benchmark uncompressed insert
        group.bench_function(format!("{pattern_name}_uncompressed_insert"), |b| {
            b.iter(|| {
                let mut leaf = LeafNode::new(128);
                for (i, key) in keys.iter().enumerate() {
                    leaf.insert(key.clone(), i as u64).unwrap();
                }
                black_box(leaf)
            });
        });

        // Benchmark compressed insert
        group.bench_function(format!("{pattern_name}_compressed_insert"), |b| {
            b.iter(|| {
                let mut leaf = CompressedLeafNode::new(128);
                for (i, key) in keys.iter().enumerate() {
                    leaf.insert(key.clone(), i as u64).unwrap();
                }
                black_box(leaf)
            });
        });

        // Benchmark uncompressed search
        let mut uncompressed = LeafNode::new(128);
        for (i, key) in keys.iter().enumerate() {
            uncompressed.insert(key.clone(), i as u64).unwrap();
        }

        group.bench_function(format!("{pattern_name}_uncompressed_search"), |b| {
            let mut counter = 0;
            b.iter(|| {
                let key = &keys[counter % keys.len()];
                counter += 1;
                black_box(uncompressed.search(key))
            });
        });

        // Benchmark compressed search
        let mut compressed = CompressedLeafNode::new(128);
        for (i, key) in keys.iter().enumerate() {
            compressed.insert(key.clone(), i as u64).unwrap();
        }

        group.bench_function(format!("{pattern_name}_compressed_search"), |b| {
            let mut counter = 0;
            b.iter(|| {
                let key = &keys[counter % keys.len()];
                counter += 1;
                black_box(compressed.search(key))
            });
        });
    }

    group.finish();
}

fn btree_compression_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("btree_compression_memory");

    use neuroquantum_core::storage::btree::{BTreeNode, CompressedLeafNode, LeafNode};

    // Test memory usage with UUIDs (very common in databases)
    let keys: Vec<Vec<u8>> = (0..1000)
        .map(|i| format!("550e8400-e29b-41d4-a716-{i:012}").into_bytes())
        .collect();

    group.bench_function("memory_uncompressed", |b| {
        b.iter(|| {
            let mut leaf = LeafNode::new(128);
            for (i, key) in keys.iter().take(100).enumerate() {
                leaf.insert(key.clone(), i as u64).unwrap();
            }
            let node = BTreeNode::Leaf(leaf);
            black_box(node.memory_usage())
        });
    });

    group.bench_function("memory_compressed", |b| {
        b.iter(|| {
            let mut leaf = CompressedLeafNode::new(128);
            for (i, key) in keys.iter().take(100).enumerate() {
                leaf.insert(key.clone(), i as u64).unwrap();
            }
            let node = BTreeNode::CompressedLeaf(leaf);
            black_box(node.memory_usage())
        });
    });

    // Benchmark conversion
    let mut leaf = LeafNode::new(128);
    for (i, key) in keys.iter().take(100).enumerate() {
        leaf.insert(key.clone(), i as u64).unwrap();
    }

    group.bench_function("compress_conversion", |b| {
        let leaf = leaf.clone();
        b.iter(|| black_box(CompressedLeafNode::from_leaf_node(&leaf)));
    });

    let compressed = CompressedLeafNode::from_leaf_node(&leaf);

    group.bench_function("decompress_conversion", |b| {
        let compressed = compressed.clone();
        b.iter(|| black_box(compressed.to_leaf_node()));
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
    btree_mixed_workload,
    btree_compression_leaf_nodes,
    btree_compression_memory_usage
);

criterion_main!(benches);
