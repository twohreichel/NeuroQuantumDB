//! Google Cloud Storage Backend Example
//!
//! This example demonstrates how to use the NeuroQuantumDB GCS backend
//! for backup and restore operations.
//!
//! ## Prerequisites
//!
//! 1. **GCS Bucket:** Create a bucket in Google Cloud Console
//! 2. **Authentication:** Set up service account or default credentials
//! 3. **Environment Variables:**
//!    - `GOOGLE_APPLICATION_CREDENTIALS` (path to service account JSON)
//!    - Or use Application Default Credentials (ADC)
//!
//! ## Usage
//!
//! ```bash
//! # With service account key
//! export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
//! cargo run --example gcs_backup
//!
//! # With Application Default Credentials (on GCP)
//! gcloud auth application-default login
//! cargo run --example gcs_backup
//! ```

use anyhow::Result;
use neuroquantum_core::storage::backup::{BackupStorageBackend, GCSBackend, GCSConfig};
use std::path::PathBuf;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    println!("☁️  NeuroQuantumDB - Google Cloud Storage Backend Demo");
    println!("═══════════════════════════════════════════════════════");
    println!();

    // Demo configuration
    let config = GCSConfig {
        bucket: "neuroquantum-demo".to_string(), // Change to your bucket
        project_id: "neuroquantum-project".to_string(), // Change to your project
        credentials_path: std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .ok()
            .map(PathBuf::from),
        use_default_credentials: true, // Use ADC if no explicit credentials
    };

    println!("🔧 Configuration:");
    println!("   Bucket: {}", config.bucket);
    println!("   Project: {}", config.project_id);
    println!("   Credentials: {:?}", config.credentials_path);
    println!("   Use Default: {}", config.use_default_credentials);
    println!();

    // Initialize GCS backend
    info!("Initializing GCS backend...");
    let backend = match GCSBackend::new(config).await {
        Ok(backend) => backend,
        Err(e) => {
            eprintln!("❌ Failed to initialize GCS backend: {}", e);
            eprintln!("💡 Make sure you have:");
            eprintln!("   1. Created a GCS bucket");
            eprintln!("   2. Set up authentication (service account or ADC)");
            eprintln!("   3. Granted Storage Admin permissions");
            return Err(e);
        }
    };

    println!("✅ GCS backend initialized successfully!");
    println!();

    // Demo 1: Basic file operations
    println!("📁 Demo 1: Basic File Operations");
    println!("─────────────────────────────────");

    let test_file = PathBuf::from("demo/basic_test.txt");
    let test_data =
        b"Hello from NeuroQuantumDB GCS Backend!\nThis is a test file for demonstration.";

    // Write file
    info!("Writing test file...");
    backend.write_file(&test_file, test_data).await?;
    println!("✅ File written: {} bytes", test_data.len());

    // Read file back
    info!("Reading test file...");
    let read_data = backend.read_file(&test_file).await?;
    println!("✅ File read: {} bytes", read_data.len());

    // Verify data integrity
    assert_eq!(read_data, test_data);
    println!("✅ Data integrity verified!");

    // List files
    let files = backend.list_directory(&PathBuf::from("demo")).await?;
    println!("✅ Directory listing: {} files found", files.len());
    for file in &files {
        println!("   📄 {}", file.display());
    }

    println!();

    // Demo 2: Structured data (JSON)
    println!("📋 Demo 2: Structured Data Storage (JSON)");
    println!("──────────────────────────────────────────");

    let metadata = serde_json::json!({
        "backup_id": "demo_backup_001",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "database": {
            "name": "neuroquantum_demo",
            "version": "1.0.0",
            "tables": 5,
            "size_mb": 42.7
        },
        "compression": {
            "algorithm": "dna_quaternary",
            "ratio": 3.2,
            "original_size": 134217728,
            "compressed_size": 41943040
        },
        "neuromorphic_features": {
            "synaptic_weights": 10000,
            "learning_rate": 0.001,
            "activation_function": "sigmoid"
        }
    });

    let json_data = serde_json::to_vec_pretty(&metadata)?;
    let json_file = PathBuf::from("demo/backup_metadata.json");

    info!("Storing JSON metadata...");
    backend.write_file(&json_file, &json_data).await?;
    println!("✅ JSON metadata stored: {} bytes", json_data.len());

    // Read and parse JSON back
    let read_json_data = backend.read_file(&json_file).await?;
    let parsed_metadata: serde_json::Value = serde_json::from_slice(&read_json_data)?;
    println!("✅ JSON metadata parsed successfully");
    println!("   📊 Backup ID: {}", parsed_metadata["backup_id"]);
    println!(
        "   📊 Compression Ratio: {}",
        parsed_metadata["compression"]["ratio"]
    );

    println!();

    // Demo 3: Binary data (simulated database page)
    println!("💾 Demo 3: Binary Data Storage (Database Page)");
    println!("───────────────────────────────────────────────────");

    // Create simulated 4KB database page
    let page_size = 4096;
    let mut page_data = vec![0u8; page_size];

    // Fill with pattern (simulating B+ tree node)
    for (i, byte) in page_data.iter_mut().enumerate() {
        *byte = match i % 16 {
            0..=3 => 0xDE,        // Magic number
            4..=7 => 0xAD,        // Page type
            8..=11 => 0xBE,       // Checksum
            12..=15 => 0xEF,      // Flags
            _ => (i % 256) as u8, // Data
        };
    }

    let page_file = PathBuf::from("demo/database_page_001.dat");

    info!("Storing database page...");
    backend.write_file(&page_file, &page_data).await?;
    println!("✅ Database page stored: {} bytes (4KB)", page_data.len());

    // Verify page integrity
    let read_page_data = backend.read_file(&page_file).await?;
    assert_eq!(read_page_data.len(), page_size);
    assert_eq!(read_page_data, page_data);
    println!(
        "✅ Page integrity verified: Magic={:#04X}, Type={:#04X}",
        read_page_data[0], read_page_data[4]
    );

    println!();

    // Demo 4: Directory operations
    println!("📁 Demo 4: Directory Operations");
    println!("───────────────────────────────");

    // Create nested directory structure
    let nested_file1 = PathBuf::from("demo/backups/2025/11/28/full_backup.tar.gz");
    let nested_file2 = PathBuf::from("demo/backups/2025/11/28/incremental_001.tar.gz");
    let nested_file3 = PathBuf::from("demo/logs/application.log");

    let files_to_create = vec![
        (&nested_file1, b"Simulated full backup data" as &[u8]),
        (&nested_file2, b"Simulated incremental backup data"),
        (
            &nested_file3,
            b"2025-11-28 14:30:00 INFO: Backup completed successfully",
        ),
    ];

    info!("Creating nested directory structure...");
    for (file_path, data) in &files_to_create {
        backend.write_file(file_path, data).await?;
        println!("✅ Created: {}", file_path.display());
    }

    // List directory contents
    let backup_files = backend
        .list_directory(&PathBuf::from("demo/backups"))
        .await?;
    println!("📋 Backup directory: {} files", backup_files.len());

    let log_files = backend.list_directory(&PathBuf::from("demo/logs")).await?;
    println!("📋 Logs directory: {} files", log_files.len());

    // Check if directories exist
    let backup_exists = backend
        .directory_exists(&PathBuf::from("demo/backups"))
        .await?;
    let nonexistent_exists = backend
        .directory_exists(&PathBuf::from("demo/nonexistent"))
        .await?;

    println!("✅ Backup directory exists: {}", backup_exists);
    println!("✅ Nonexistent directory exists: {}", nonexistent_exists);

    println!();

    // Demo 5: Cleanup demonstration
    println!("🗑️  Demo 5: Cleanup Operations");
    println!("─────────────────────────────");

    // List all demo files before cleanup
    let all_demo_files = backend.list_directory(&PathBuf::from("demo")).await?;
    println!(
        "📋 Total demo files before cleanup: {}",
        all_demo_files.len()
    );

    // Delete specific files
    let files_to_delete = vec![
        test_file,
        json_file,
        page_file,
        nested_file1,
        nested_file2,
        nested_file3,
    ];

    info!("Cleaning up demo files...");
    for file_path in files_to_delete {
        match backend.delete_file(&file_path).await {
            Ok(()) => println!("✅ Deleted: {}", file_path.display()),
            Err(e) => println!("⚠️  Could not delete {}: {}", file_path.display(), e),
        }
    }

    println!();

    // Demo 6: Performance characteristics
    println!("⚡ Demo 6: Performance Characteristics");
    println!("─────────────────────────────────────");

    // Test with different file sizes
    let test_sizes = vec![
        (1024, "1KB"),
        (10 * 1024, "10KB"),
        (100 * 1024, "100KB"),
        (1024 * 1024, "1MB"),
    ];

    for (size, label) in test_sizes {
        let test_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let test_file = PathBuf::from(format!("perf_test_{}.dat", label));

        // Measure write performance
        let start = std::time::Instant::now();
        backend.write_file(&test_file, &test_data).await?;
        let write_duration = start.elapsed();

        // Measure read performance
        let start = std::time::Instant::now();
        let _read_data = backend.read_file(&test_file).await?;
        let read_duration = start.elapsed();

        // Calculate throughput
        let write_throughput = (size as f64 / 1024.0 / 1024.0) / write_duration.as_secs_f64();
        let read_throughput = (size as f64 / 1024.0 / 1024.0) / read_duration.as_secs_f64();

        println!(
            "📊 {} - Write: {:.2} MB/s, Read: {:.2} MB/s",
            label, write_throughput, read_throughput
        );

        // Cleanup
        backend.delete_file(&test_file).await.ok();
    }

    println!();

    // Summary
    println!("📊 Demo Summary");
    println!("───────────────");
    println!("✅ All GCS operations completed successfully!");
    println!("🧬 NeuroQuantumDB GCS backend is fully functional");
    println!();
    println!("🔬 Neuromorphic Mapping:");
    println!("   • GCS Storage → Long-term Memory (Hippocampus)");
    println!("   • Bucket Organization → Cortical Layers");
    println!("   • Object Prefixes → Dendritic Branching");
    println!("   • Redundancy → Neural Pathway Backup");
    println!();
    println!("☁️  Cloud-Native Features:");
    println!("   • Multi-region redundancy");
    println!("   • Automatic scaling");
    println!("   • Encryption at rest and in transit");
    println!("   • Lifecycle management");

    Ok(())
}
