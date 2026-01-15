//! Buffer Pool Auto-Tuning Example
//!
//! This example demonstrates how to use the automatic buffer pool sizing
//! feature to optimize memory usage based on available system RAM.
//!
//! Run with: `cargo run --example buffer_pool_auto_tuning`

use neuroquantum_core::storage::buffer::BufferPoolConfig;

fn main() {
    println!("🎯 NeuroQuantumDB Buffer Pool Auto-Tuning Demo\n");
    println!("{}", "=".repeat(60));

    // Example 1: Auto-tuned configuration (recommended for most systems)
    println!("\n1️⃣  Auto-Tuned Configuration (50% RAM):");
    println!("{}", "─".repeat(60));

    let auto_config = BufferPoolConfig::auto_tuned();
    print_config("Auto-Tuned", &auto_config);

    // Example 2: Conservative allocation for shared systems
    println!("\n2️⃣  Conservative Configuration (30% RAM):");
    println!("{}", "─".repeat(60));

    let conservative_config = BufferPoolConfig::with_ram_percentage(0.3);
    print_config("Conservative", &conservative_config);

    // Example 3: Aggressive allocation for dedicated database servers
    println!("\n3️⃣  Aggressive Configuration (80% RAM):");
    println!("{}", "─".repeat(60));

    let aggressive_config = BufferPoolConfig::with_ram_percentage(0.8);
    print_config("Aggressive", &aggressive_config);

    // Example 4: Default (fixed size) for comparison
    println!("\n4️⃣  Default Configuration (Fixed 1000 frames):");
    println!("{}", "─".repeat(60));

    let default_config = BufferPoolConfig::default();
    print_config("Default", &default_config);

    // System information
    println!("\n📊 System Information:");
    println!("{}", "─".repeat(60));
    print_system_info();

    println!("\n{}", "=".repeat(60));
    println!("✅ Demo complete!");
    println!("\n💡 Recommendation:");
    println!("   - Use `BufferPoolConfig::auto_tuned()` for most deployments");
    println!("   - Use `with_ram_percentage(0.8)` for dedicated DB servers");
    println!("   - Use `with_ram_percentage(0.3)` for shared/constrained systems");
}

fn print_config(name: &str, config: &BufferPoolConfig) {
    let frames = config.pool_size;
    let mb = (frames * 4) / 1024; // 4KB pages
    let gb = mb as f64 / 1024.0;

    println!("  Configuration: {name}");
    println!("  Pool Size:     {frames} frames");
    println!("  Memory Usage:  {mb} MB ({gb:.2} GB)");
    println!("  Eviction:      {:?}", config.eviction_policy);
    println!("  Flush Enabled: {}", config.enable_background_flush);
    println!("  Flush Interval: {:?}", config.flush_interval);
    println!("  Max Dirty:     {} pages", config.max_dirty_pages);
}

fn print_system_info() {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_memory();

    // sysinfo 0.30+ returns bytes
    let total_bytes = sys.total_memory();
    let available_bytes = sys.available_memory();
    let used_bytes = sys.used_memory();

    let total_gb = total_bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    let available_gb = available_bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    let used_gb = used_bytes as f64 / 1024.0 / 1024.0 / 1024.0;

    let usage_percent = (used_bytes as f64 / total_bytes as f64) * 100.0;

    println!("  Total RAM:     {total_gb:.2} GB");
    println!("  Available RAM: {available_gb:.2} GB");
    println!("  Used RAM:      {used_gb:.2} GB ({usage_percent:.1}%)");
    println!("  Free RAM:      {:.2} GB", total_gb - used_gb);

    println!("\n  Calculated Buffer Pool Sizes:");
    println!("    30% of RAM:  {:.2} GB", total_gb * 0.3);
    println!("    50% of RAM:  {:.2} GB", total_gb * 0.5);
    println!("    80% of RAM:  {:.2} GB", total_gb * 0.8);
}
