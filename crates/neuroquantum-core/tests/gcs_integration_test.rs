//! GCS Integration Tests
//!
//! These tests require actual Google Cloud Storage credentials and a test bucket.
//! They are ignored by default and can be run with:
//! ```
//! cargo test --test gcs_integration_test -- --ignored
//! ```
//!
//! Setup required:
//! 1. Create a GCS bucket for testing
//! 2. Set environment variables:
//!    - GCS_TEST_BUCKET: Your test bucket name
//!    - GCS_TEST_PROJECT_ID: Your GCP project ID
//!    - GOOGLE_APPLICATION_CREDENTIALS: Path to service account key (or use ADC)

use anyhow::Result;
use neuroquantum_core::storage::backup::{BackupStorageBackend, GCSBackend, GCSConfig};
use std::env;
use std::path::PathBuf;

/// Helper to get GCS test configuration from environment
fn get_gcs_test_config() -> Option<GCSConfig> {
    let bucket = env::var("GCS_TEST_BUCKET").ok()?;
    let project_id = env::var("GCS_TEST_PROJECT_ID").ok()?;

    // Check if we should use a specific credentials file or default credentials
    let credentials_path = env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .ok()
        .map(PathBuf::from);

    let use_default_credentials = credentials_path.is_none();

    Some(GCSConfig {
        bucket,
        project_id,
        credentials_path,
        use_default_credentials,
    })
}

#[tokio::test]
#[ignore] // Requires actual GCS setup
async fn test_gcs_backend_initialization() -> Result<()> {
    let config = get_gcs_test_config()
        .expect("GCS test config not available. Set GCS_TEST_BUCKET and GCS_TEST_PROJECT_ID");

    let _backend = GCSBackend::new(config).await?;

    println!("✅ GCS backend initialized successfully");
    Ok(())
}

#[tokio::test]
#[ignore] // Requires actual GCS setup
async fn test_gcs_write_read_delete_cycle() -> Result<()> {
    let config = get_gcs_test_config().expect("GCS test config not available");

    let backend = GCSBackend::new(config).await?;

    // Test data
    let test_path = PathBuf::from("test/neuroquantum/test_file.dat");
    let test_data = b"NeuroQuantumDB GCS Integration Test - Neuromorphic Architecture";

    // Write
    backend.write_file(&test_path, test_data).await?;
    println!("✅ Successfully wrote {} bytes to GCS", test_data.len());

    // Read
    let read_data = backend.read_file(&test_path).await?;
    assert_eq!(read_data, test_data, "Read data should match written data");
    println!("✅ Successfully read {} bytes from GCS", read_data.len());

    // Delete
    backend.delete_file(&test_path).await?;
    println!("✅ Successfully deleted file from GCS");

    // Verify deletion (should fail to read)
    let read_result = backend.read_file(&test_path).await;
    assert!(read_result.is_err(), "Reading deleted file should fail");
    println!("✅ Verified file was deleted");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires actual GCS setup
async fn test_gcs_directory_operations() -> Result<()> {
    let config = get_gcs_test_config().expect("GCS test config not available");

    let backend = GCSBackend::new(config).await?;

    // Create test files in a "directory"
    let base_path = PathBuf::from("test/neuroquantum/backup_test");

    let file1 = base_path.join("file1.dat");
    let file2 = base_path.join("file2.dat");
    let file3 = base_path.join("subdir/file3.dat");

    backend.write_file(&file1, b"File 1 content").await?;
    backend.write_file(&file2, b"File 2 content").await?;
    backend.write_file(&file3, b"File 3 content").await?;
    println!("✅ Created 3 test files");

    // List directory
    let files = backend.list_directory(&base_path).await?;
    assert!(files.len() >= 3, "Should find at least 3 files");
    println!("✅ Listed {} objects in directory", files.len());

    // Cleanup
    backend.delete_file(&file1).await?;
    backend.delete_file(&file2).await?;
    backend.delete_file(&file3).await?;
    println!("✅ Cleaned up test files");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires actual GCS setup
async fn test_gcs_large_file_handling() -> Result<()> {
    let config = get_gcs_test_config().expect("GCS test config not available");

    let backend = GCSBackend::new(config).await?;

    // Create a 10MB test file
    let large_data: Vec<u8> = (0..10_000_000).map(|i| (i % 256) as u8).collect();

    let test_path = PathBuf::from("test/neuroquantum/large_file.dat");

    println!("📊 Writing {} MB file...", large_data.len() / 1_000_000);

    let start = std::time::Instant::now();
    backend.write_file(&test_path, &large_data).await?;
    let write_duration = start.elapsed();

    let write_mbps = (large_data.len() as f64 / 1_000_000.0) / write_duration.as_secs_f64();
    println!(
        "✅ Wrote large file in {:?} ({:.2} MB/s)",
        write_duration, write_mbps
    );

    // Read back
    let start = std::time::Instant::now();
    let read_data = backend.read_file(&test_path).await?;
    let read_duration = start.elapsed();

    let read_mbps = (read_data.len() as f64 / 1_000_000.0) / read_duration.as_secs_f64();
    println!(
        "✅ Read large file in {:?} ({:.2} MB/s)",
        read_duration, read_mbps
    );

    assert_eq!(read_data.len(), large_data.len(), "Size should match");
    assert_eq!(read_data, large_data, "Content should match");

    // Cleanup
    backend.delete_file(&test_path).await?;
    println!("✅ Cleaned up large file");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires actual GCS setup
async fn test_gcs_concurrent_operations() -> Result<()> {
    let config = get_gcs_test_config().expect("GCS test config not available");

    let backend = std::sync::Arc::new(GCSBackend::new(config).await?);

    let num_concurrent = 10;
    let mut handles = Vec::new();

    println!(
        "🚀 Starting {} concurrent write operations...",
        num_concurrent
    );

    for i in 0..num_concurrent {
        let backend_clone = backend.clone();
        let handle = tokio::spawn(async move {
            let path = PathBuf::from(format!("test/neuroquantum/concurrent/file_{}.dat", i));
            let data = format!("Concurrent write test data {}", i);
            backend_clone.write_file(&path, data.as_bytes()).await?;

            // Read back to verify
            let read_data = backend_clone.read_file(&path).await?;
            assert_eq!(read_data, data.as_bytes());

            // Cleanup
            backend_clone.delete_file(&path).await?;

            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await??;
    }

    println!(
        "✅ All {} concurrent operations completed successfully",
        num_concurrent
    );

    Ok(())
}

#[tokio::test]
#[ignore] // Requires actual GCS setup
async fn test_gcs_backup_metadata_storage() -> Result<()> {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct BackupMetadata {
        backup_id: String,
        timestamp: String,
        size_bytes: u64,
        file_count: u32,
    }

    let config = get_gcs_test_config().expect("GCS test config not available");

    let backend = GCSBackend::new(config).await?;

    let metadata = BackupMetadata {
        backup_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        size_bytes: 1_234_567_890,
        file_count: 42,
    };

    let metadata_json = serde_json::to_vec_pretty(&metadata)?;
    let metadata_path = PathBuf::from("test/neuroquantum/metadata/backup_metadata.json");

    // Write metadata
    backend.write_file(&metadata_path, &metadata_json).await?;
    println!("✅ Wrote backup metadata to GCS");

    // Read and deserialize
    let read_data = backend.read_file(&metadata_path).await?;
    let read_metadata: BackupMetadata = serde_json::from_slice(&read_data)?;

    assert_eq!(read_metadata, metadata, "Metadata should match");
    println!("✅ Successfully read and verified metadata");

    // Cleanup
    backend.delete_file(&metadata_path).await?;

    Ok(())
}

#[tokio::test]
#[ignore] // Requires actual GCS setup
async fn test_gcs_error_handling() -> Result<()> {
    let config = get_gcs_test_config().expect("GCS test config not available");

    let backend = GCSBackend::new(config).await?;

    // Try to read non-existent file
    let non_existent = PathBuf::from("test/neuroquantum/does_not_exist.dat");
    let result = backend.read_file(&non_existent).await;

    assert!(result.is_err(), "Reading non-existent file should fail");
    println!("✅ Error handling for non-existent file works correctly");

    // Try to delete non-existent file
    let delete_result = backend.delete_file(&non_existent).await;
    // GCS delete is idempotent, so this might succeed or fail depending on implementation
    println!("Delete non-existent result: {:?}", delete_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_gcs_config_serialization() -> Result<()> {
    use neuroquantum_core::storage::backup::GCSConfig;

    let config = GCSConfig {
        bucket: "neuroquantum-backups".to_string(),
        project_id: "neuroquantum-prod-12345".to_string(),
        credentials_path: Some(PathBuf::from("/etc/gcp/service-account.json")),
        use_default_credentials: false,
    };

    // Serialize
    let json = serde_json::to_string_pretty(&config)?;
    println!("Serialized config:\n{}", json);

    // Deserialize
    let deserialized: GCSConfig = serde_json::from_str(&json)?;

    assert_eq!(deserialized.bucket, config.bucket);
    assert_eq!(deserialized.project_id, config.project_id);
    assert_eq!(
        deserialized.use_default_credentials,
        config.use_default_credentials
    );

    println!("✅ Config serialization/deserialization works");

    Ok(())
}

/// Helper function to run all GCS tests
/// Use with: cargo test --test gcs_integration_test -- --ignored --test-threads=1
///
/// Note: This is a placeholder. Run individual tests separately:
/// cargo test --test gcs_integration_test test_gcs_ -- --ignored
#[tokio::test]
#[ignore]
async fn run_all_gcs_tests_placeholder() -> Result<()> {
    println!("🧪 GCS integration test suite");
    println!("Run individual tests with: cargo test --test gcs_integration_test -- --ignored");
    println!();
    println!("Available tests:");
    println!("  - test_gcs_backend_initialization");
    println!("  - test_gcs_write_read_delete_cycle");
    println!("  - test_gcs_directory_operations");
    println!("  - test_gcs_large_file_handling");
    println!("  - test_gcs_concurrent_operations");
    println!("  - test_gcs_backup_metadata_storage");
    println!("  - test_gcs_error_handling");

    Ok(())
}
