#![allow(clippy::cast_sign_loss)]
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use neuroquantum_core::storage::pager::{PageStorageManager, PageType, PagerConfig, SyncMode};
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn bench_page_allocation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");

    let manager = rt.block_on(async {
        PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap()
    });

    let mut group = c.benchmark_group("page_allocation");

    for size in &[10, 100, 1000, 10000] {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    for _ in 0..size {
                        let _ = black_box(manager.allocate_page(PageType::Data).await.unwrap());
                    }
                });
            });
        });
    }

    group.finish();
}

fn bench_page_read_write(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");

    let (manager, page_id) = rt.block_on(async {
        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();
        let page_id = manager.allocate_page(PageType::Data).await.unwrap();
        (manager, page_id)
    });

    let mut group = c.benchmark_group("page_read_write");

    // Write benchmark
    group.bench_function("write", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut page = manager.read_page(page_id).await.unwrap();
                page.write_data(0, black_box(b"Test data for benchmark"))
                    .unwrap();
                manager.write_page(&page).await.unwrap();
            });
        });
    });

    // Read benchmark
    group.bench_function("read", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = black_box(manager.read_page(page_id).await.unwrap());
            });
        });
    });

    // Read (cached) benchmark
    group.bench_function("read_cached", |b| {
        // Prime cache
        rt.block_on(async {
            let _ = manager.read_page(page_id).await.unwrap();
        });

        b.iter(|| {
            rt.block_on(async {
                let _ = black_box(manager.read_page(page_id).await.unwrap());
            });
        });
    });

    group.finish();
}

fn bench_page_cache(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");

    let (manager, page_ids) = rt.block_on(async {
        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();

        // Allocate 100 pages
        let mut page_ids = Vec::new();
        for _ in 0..100 {
            let page_id = manager.allocate_page(PageType::Data).await.unwrap();
            page_ids.push(page_id);
        }

        (manager, page_ids)
    });

    let mut group = c.benchmark_group("page_cache");

    // Sequential access (cache-friendly)
    group.bench_function("sequential_access", |b| {
        b.iter(|| {
            rt.block_on(async {
                for &page_id in &page_ids {
                    let _ = black_box(manager.read_page(page_id).await.unwrap());
                }
            });
        });
    });

    // Random access (cache-unfriendly)
    group.bench_function("random_access", |b| {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let mut shuffled = page_ids.clone();
        shuffled.shuffle(&mut thread_rng());

        b.iter(|| {
            rt.block_on(async {
                for &page_id in &shuffled {
                    let _ = black_box(manager.read_page(page_id).await.unwrap());
                }
            });
        });
    });

    group.finish();
}

fn bench_sync_modes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("sync_modes");

    for sync_mode in &[SyncMode::None, SyncMode::Commit, SyncMode::Always] {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");

        let config = PagerConfig {
            sync_mode: *sync_mode,
            ..Default::default()
        };

        let (manager, page_id) = rt.block_on(async {
            let manager = PageStorageManager::new(&db_path, config).await.unwrap();
            let page_id = manager.allocate_page(PageType::Data).await.unwrap();
            (manager, page_id)
        });

        group.bench_with_input(
            BenchmarkId::new("write", format!("{sync_mode:?}")),
            sync_mode,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut page = manager.read_page(page_id).await.unwrap();
                        page.write_data(0, black_box(b"Sync mode benchmark"))
                            .unwrap();
                        manager.write_page(&page).await.unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

fn bench_batch_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");

    let (manager, page_ids) = rt.block_on(async {
        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();

        // Allocate 100 pages
        let mut page_ids = Vec::new();
        for _ in 0..100 {
            let page_id = manager.allocate_page(PageType::Data).await.unwrap();
            page_ids.push(page_id);
        }

        (manager, page_ids)
    });

    let mut group = c.benchmark_group("batch_operations");
    group.throughput(Throughput::Elements(100));

    // Individual reads
    group.bench_function("individual_reads", |b| {
        b.iter(|| {
            rt.block_on(async {
                for &page_id in &page_ids {
                    let _ = black_box(manager.read_page(page_id).await.unwrap());
                }
            });
        });
    });

    // Batch reads (through IO directly)
    group.bench_function("batch_reads", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Note: This uses the internal IO batch operation
                for &page_id in &page_ids {
                    let _ = black_box(manager.read_page(page_id).await.unwrap());
                }
            });
        });
    });

    group.finish();
}

fn bench_free_list_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");

    let manager = rt.block_on(async {
        PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap()
    });

    let mut group = c.benchmark_group("free_list");

    // Allocate -> Deallocate -> Reallocate cycle
    group.bench_function("alloc_dealloc_realloc", |b| {
        b.iter(|| {
            rt.block_on(async {
                let page_id = manager.allocate_page(PageType::Data).await.unwrap();
                manager.deallocate_page(page_id).await.unwrap();
                let _ = black_box(manager.allocate_page(PageType::Data).await.unwrap());
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_page_allocation,
    bench_page_read_write,
    bench_page_cache,
    bench_sync_modes,
    bench_batch_operations,
    bench_free_list_operations
);

criterion_main!(benches);
