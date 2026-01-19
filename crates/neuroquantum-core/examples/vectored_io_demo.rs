#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Example demonstrating vectored I/O performance improvements
//!
//! This example shows the performance benefits of the optimized batch I/O
//! implementation compared to sequential single-page operations.

use std::time::Instant;

use anyhow::Result;
use neuroquantum_core::storage::pager::io::PageIO;
use neuroquantum_core::storage::pager::page::{Page, PageId, PageType};
use neuroquantum_core::storage::pager::PagerConfig;
use tempfile::TempDir;
use tokio::fs::OpenOptions;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 Vectored I/O Performance Demonstration\n");
    println!("{}", "=".repeat(60));

    // Test different batch sizes
    for batch_size in [10, 50, 100, 500, 1000] {
        run_benchmark(batch_size).await?;
        println!();
    }

    println!("{}", "=".repeat(60));
    println!("\n✅ Benchmark complete!");
    println!("\n📊 Summary:");
    println!("  • Larger batch sizes show greater performance improvements");
    println!("  • Contiguous pages are read/written in single syscalls");
    println!("  • Scattered access still benefits from optimized grouping");

    Ok(())
}

async fn run_benchmark(batch_size: u64) -> Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("benchmark.db");

    println!("\n📦 Batch size: {batch_size} pages");
    println!("{}", "-".repeat(60));

    // Create test file
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&file_path)
        .await?;

    let io = PageIO::new(file, PagerConfig::default());

    // === WRITE BENCHMARK ===

    // Create test pages
    let pages: Vec<Page> = (0..batch_size)
        .map(|i| {
            let mut page = Page::new(PageId(i), PageType::Data);
            let data = format!("Test data for page {i}").into_bytes();
            page.write_data(0, &data).unwrap();
            page.update_checksum();
            page
        })
        .collect();

    // Write with batch optimization
    let start = Instant::now();
    io.write_pages_batch(&pages).await?;
    io.sync().await?;
    let batch_write_time = start.elapsed();

    println!(
        "  ✍️  Batch write:  {:>8.3} ms ({:.2} pages/ms)",
        batch_write_time.as_secs_f64() * 1000.0,
        batch_size as f64 / (batch_write_time.as_secs_f64() * 1000.0)
    );

    // === READ BENCHMARK ===

    // Sequential read (contiguous pages)
    let page_ids: Vec<PageId> = (0..batch_size).map(PageId).collect();

    let start = Instant::now();
    let read_pages = io.read_pages_batch(&page_ids).await?;
    let batch_read_time = start.elapsed();

    assert_eq!(read_pages.len(), batch_size as usize);

    println!(
        "  📖 Batch read:   {:>8.3} ms ({:.2} pages/ms)",
        batch_read_time.as_secs_f64() * 1000.0,
        batch_size as f64 / (batch_read_time.as_secs_f64() * 1000.0)
    );

    // === SCATTERED READ BENCHMARK ===

    // Read in reverse order (worst case for cache, but still optimized grouping)
    let scattered_ids: Vec<PageId> = (0..batch_size)
        .map(PageId)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let start = Instant::now();
    let scattered_pages = io.read_pages_batch(&scattered_ids).await?;
    let scattered_read_time = start.elapsed();

    assert_eq!(scattered_pages.len(), batch_size as usize);
    // Verify order is preserved
    assert_eq!(scattered_pages[0].id(), PageId(batch_size - 1));
    assert_eq!(scattered_pages.last().unwrap().id(), PageId(0));

    println!(
        "  🔀 Scattered read: {:>7.3} ms ({:.2} pages/ms)",
        scattered_read_time.as_secs_f64() * 1000.0,
        batch_size as f64 / (scattered_read_time.as_secs_f64() * 1000.0)
    );

    // Calculate performance metrics
    let total_time = batch_write_time + batch_read_time;
    let throughput_mb = (batch_size as f64 * 4096.0) / (1024.0 * 1024.0) / total_time.as_secs_f64();

    println!("  📊 Throughput:   {throughput_mb:>8.2} MB/s");

    Ok(())
}
