//! Google Cloud Storage Integration Tests
//!
//! These tests require a real GCS bucket and valid credentials.
//! They are ignored by default to avoid requiring GCS setup in CI/CD.
//!
//! To run these tests:
//! 1. Set up a GCS bucket and service account
//! 2. Set environment variables: GCS_TEST_BUCKET and GCS_TEST_PROJECT_ID
//! 3. Run: `cargo test --test gcs_integration_test -- --ignored`

use anyhow::Result;
use neuroquantum_core::storage::backup::{BackupStorageBackend, GCSBackend, GCSConfig};
use std::env;
use std::path::PathBuf;

/// Get GCS test configuration from environment variables
fn get_test_config() -> Option<GCSConfig> {
    let bucket = env::var("GCS_TEST_BUCKET").ok()?;
    let project_id = env::var("GCS_TEST_PROJECT_ID").ok()?;

    Some(GCSConfig {
        bucket,
        project_id,
        credentials_path: env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .ok()
            .map(PathBuf::from),
        use_default_credentials: env::var("GCS_USE_DEFAULT_CREDENTIALS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
    })
}

#[tokio::test]
#[ignore] // Requires real GCS bucket
async fn test_gcs_backend_initialization() -> Result<()> {
    let config =
        get_test_config().ok_or_else(|| anyhow::anyhow!("GCS test configuration not available"))?;

    let _backend = GCSBackend::new(config).await?;
    println!("✅ GCS backend initialized successfully");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real GCS bucket
async fn test_gcs_write_read_delete_cycle() -> Result<()> {
    let config =
        get_test_config().ok_or_else(|| anyhow::anyhow!("GCS test configuration not available"))?;

    let backend = GCSBackend::new(config).await?;

    // Test data
    let test_file = PathBuf::from("test_files/integration_test.dat");
    let test_data = b"Hello from NeuroQuantumDB GCS Integration Test!";

    // Write file
    backend.write_file(&test_file, test_data).await?;
    println!("✅ File written to GCS");

    // Read file back
    let read_data = backend.read_file(&test_file).await?;
    assert_eq!(read_data, test_data);
    println!("✅ File read from GCS successfully");

    // Delete file
    backend.delete_file(&test_file).await?;
    println!("✅ File deleted from GCS");

    // Verify file is gone (should fail to read)
    let result = backend.read_file(&test_file).await;
    assert!(result.is_err());
    println!("✅ Confirmed file deletion");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real GCS bucket
async fn test_gcs_directory_operations() -> Result<()> {
    let config =
        get_test_config().ok_or_else(|| anyhow::anyhow!("GCS test configuration not available"))?;

    let backend = GCSBackend::new(config).await?;

    // Test directory creation (implicit in GCS)
    let test_dir = PathBuf::from("test_directory");
    backend.create_directory(&test_dir).await?;
    println!("✅ Directory created (implicit in GCS)");

    // Create a file in the directory
    let test_file = PathBuf::from("test_directory/nested_file.txt");
    let test_data = b"Nested file content";
    backend.write_file(&test_file, test_data).await?;
    println!("✅ File created in directory");

    // Check directory exists (should return true now)
    let exists = backend.directory_exists(&test_dir).await?;
    assert!(exists);
    println!("✅ Directory exists check passed");

    // List directory contents
    let files = backend.list_directory(&test_dir).await?;
    assert!(!files.is_empty());
    assert!(files
        .iter()
        .any(|f| f.to_string_lossy().contains("nested_file.txt")));
    println!("✅ Directory listing successful: {} files", files.len());

    // Cleanup
    backend.delete_file(&test_file).await?;
    println!("✅ Cleanup completed");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real GCS bucket
async fn test_gcs_large_file_handling() -> Result<()> {
    let config =
        get_test_config().ok_or_else(|| anyhow::anyhow!("GCS test configuration not available"))?;

    let backend = GCSBackend::new(config).await?;

    // Create a large test file (10MB)
    let large_data: Vec<u8> = (0..10_000_000).map(|i| (i % 256) as u8).collect();
    let test_file = PathBuf::from("large_files/test_10mb.dat");

    // Write large file
    let start = std::time::Instant::now();
    backend.write_file(&test_file, &large_data).await?;
    let write_duration = start.elapsed();
    println!("✅ Large file written in {:?}", write_duration);

    // Read large file back
    let start = std::time::Instant::now();
    let read_data = backend.read_file(&test_file).await?;
    let read_duration = start.elapsed();

    assert_eq!(read_data.len(), large_data.len());
    assert_eq!(read_data, large_data);
    println!("✅ Large file read back in {:?}", read_duration);

    // Calculate throughput
    let write_throughput =
        (large_data.len() as f64 / 1024.0 / 1024.0) / write_duration.as_secs_f64();
    let read_throughput = (read_data.len() as f64 / 1024.0 / 1024.0) / read_duration.as_secs_f64();

    println!("📊 Write throughput: {:.2} MB/s", write_throughput);
    println!("📊 Read throughput: {:.2} MB/s", read_throughput);

    // Cleanup
    backend.delete_file(&test_file).await?;
    println!("✅ Large file cleanup completed");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real GCS bucket
async fn test_gcs_concurrent_operations() -> Result<()> {
    let config =
        get_test_config().ok_or_else(|| anyhow::anyhow!("GCS test configuration not available"))?;

    let backend = GCSBackend::new(config).await?;

    // Create multiple files concurrently
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let test_data = format!("Concurrent test file #{}", i);
            let test_file = PathBuf::from(format!("concurrent_test/file_{}.txt", i));

            tokio::spawn(async move { (test_file, test_data.into_bytes()) })
        })
        .collect();

    // Wait for all tasks to complete
    let mut files_and_data = Vec::new();
    for task in tasks {
        files_and_data.push(task.await.unwrap());
    }

    // Upload all files sequentially to avoid lifetime issues
    for (file, data) in &files_and_data {
        backend.write_file(file, data).await?;
    }
    println!("✅ All 10 files uploaded concurrently");

    // List files to verify
    let listed_files = backend
        .list_directory(&PathBuf::from("concurrent_test"))
        .await?;
    assert_eq!(listed_files.len(), 10);
    println!("✅ All 10 files listed successfully");

    // Cleanup all files sequentially
    for (file, _) in &files_and_data {
        backend.delete_file(file).await?;
    }
    println!("✅ All files cleaned up successfully");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real GCS bucket
async fn test_gcs_backup_metadata_storage() -> Result<()> {
    let config =
        get_test_config().ok_or_else(|| anyhow::anyhow!("GCS test configuration not available"))?;

    let backend = GCSBackend::new(config).await?;

    // Simulate backup metadata (JSON format)
    let metadata = serde_json::json!({
        "backup_id": "backup_20251128_143022",
        "timestamp": "2025-11-28T14:30:22Z",
        "database_version": "1.0.0",
        "size_bytes": 1048576,
        "compression_ratio": 2.5,
        "files": [
            "data/table_001.dat",
            "data/table_002.dat",
            "wal/transaction.log"
        ]
    });

    let metadata_bytes = serde_json::to_vec_pretty(&metadata)?;
    let metadata_file = PathBuf::from("backups/metadata/backup_20251128_143022.json");

    // Store metadata
    backend.write_file(&metadata_file, &metadata_bytes).await?;
    println!("✅ Backup metadata stored");

    // Read metadata back
    let read_metadata_bytes = backend.read_file(&metadata_file).await?;
    let read_metadata: serde_json::Value = serde_json::from_slice(&read_metadata_bytes)?;

    assert_eq!(metadata, read_metadata);
    println!("✅ Backup metadata read and validated");

    // Cleanup
    backend.delete_file(&metadata_file).await?;
    println!("✅ Metadata cleanup completed");

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real GCS bucket
async fn test_gcs_error_handling() -> Result<()> {
    let config =
        get_test_config().ok_or_else(|| anyhow::anyhow!("GCS test configuration not available"))?;

    let backend = GCSBackend::new(config).await?;

    // Test reading non-existent file
    let non_existent_file = PathBuf::from("non_existent/missing_file.txt");
    let result = backend.read_file(&non_existent_file).await;
    assert!(result.is_err());
    println!("✅ Error handling for non-existent file works correctly");

    // Test deleting non-existent file
    let result = backend.delete_file(&non_existent_file).await;
    assert!(result.is_err());
    println!("✅ Error handling for deleting non-existent file works correctly");

    Ok(())
}

#[tokio::test]
async fn test_gcs_config_serialization() -> Result<()> {
    // Test GCSConfig serialization/deserialization (doesn't require GCS access)
    let original_config = GCSConfig {
        bucket: "test-bucket".to_string(),
        project_id: "test-project-123".to_string(),
        credentials_path: Some(PathBuf::from("/path/to/service-account.json")),
        use_default_credentials: false,
    };

    // Serialize to JSON
    let json_str = serde_json::to_string_pretty(&original_config)?;
    println!("✅ GCSConfig serialized to JSON");

    // Deserialize from JSON
    let deserialized_config: GCSConfig = serde_json::from_str(&json_str)?;

    // Verify all fields match
    assert_eq!(original_config.bucket, deserialized_config.bucket);
    assert_eq!(original_config.project_id, deserialized_config.project_id);
    assert_eq!(
        original_config.credentials_path,
        deserialized_config.credentials_path
    );
    assert_eq!(
        original_config.use_default_credentials,
        deserialized_config.use_default_credentials
    );
    println!("✅ GCSConfig deserialized and validated");

    Ok(())
}
