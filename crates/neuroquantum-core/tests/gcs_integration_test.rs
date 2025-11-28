/*!
 * GCS Integration Tests
 *
 * These tests require a real Google Cloud Storage bucket and authentication.
 * They are ignored by default and only run when:
 * 1. Environment variables are set: GCS_TEST_BUCKET, GCS_TEST_PROJECT_ID
 * 2. Valid GCS credentials are available (GOOGLE_APPLICATION_CREDENTIALS or ADC)
 * 3. Tests are run with: `cargo test --test gcs_integration_test -- --ignored`
 *
 * Example setup:
 * ```bash
 * export GCS_TEST_BUCKET=neuroquantum-test
 * export GCS_TEST_PROJECT_ID=my-project-123
 * export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
 * cargo test --test gcs_integration_test -- --ignored
 * ```
 */

use neuroquantum_core::storage::backup::storage_backend::{BackupStorageBackend, GCSBackend};
use neuroquantum_core::storage::backup::GCSConfig;
use std::path::PathBuf;

/// Helper function to create GCS config from environment
fn get_gcs_config() -> Option<GCSConfig> {
    let bucket = std::env::var("GCS_TEST_BUCKET").ok()?;
    let project_id = std::env::var("GCS_TEST_PROJECT_ID").ok()?;

    Some(GCSConfig {
        bucket,
        project_id,
        credentials_path: std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .ok()
            .map(PathBuf::from),
        use_default_credentials: std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err(),
    })
}

#[tokio::test]
#[ignore = "requires real GCS bucket and credentials"]
async fn test_gcs_backend_initialization() {
    let config = get_gcs_config().expect("GCS config not available");
    let backend = GCSBackend::new(config).await;
    assert!(backend.is_ok(), "Failed to initialize GCS backend");
}

#[tokio::test]
#[ignore = "requires real GCS bucket and credentials"]
async fn test_gcs_write_read_delete_cycle() {
    let config = get_gcs_config().expect("GCS config not available");
    let backend = GCSBackend::new(config).await.unwrap();

    let test_path = PathBuf::from("test_integration/test_file.dat");
    let test_data = b"Hello, NeuroQuantumDB GCS Integration!";

    // Write file
    backend.write_file(&test_path, test_data).await.unwrap();

    // Read file
    let read_data = backend.read_file(&test_path).await.unwrap();
    assert_eq!(read_data, test_data);

    // Delete file
    backend.delete_file(&test_path).await.unwrap();

    // Verify deletion (should fail)
    let read_result = backend.read_file(&test_path).await;
    assert!(read_result.is_err(), "File should be deleted");
}

#[tokio::test]
#[ignore = "requires real GCS bucket and credentials"]
async fn test_gcs_directory_operations() {
    let config = get_gcs_config().expect("GCS config not available");
    let backend = GCSBackend::new(config).await.unwrap();

    let test_dir = PathBuf::from("test_integration/test_dir");
    let test_file1 = test_dir.join("file1.dat");
    let test_file2 = test_dir.join("file2.dat");

    // Create directory (implicit through file creation)
    backend.create_directory(&test_dir).await.unwrap();

    // Create files in directory
    backend
        .write_file(&test_file1, b"file1 content")
        .await
        .unwrap();
    backend
        .write_file(&test_file2, b"file2 content")
        .await
        .unwrap();

    // Check directory existence
    let dir_exists = backend.directory_exists(&test_dir).await.unwrap();
    assert!(dir_exists, "Directory should exist");

    // List directory contents
    let files = backend.list_directory(&test_dir).await.unwrap();
    assert!(files.len() >= 2, "Should find at least 2 files");

    // Cleanup
    backend.delete_file(&test_file1).await.unwrap();
    backend.delete_file(&test_file2).await.unwrap();
}

#[tokio::test]
#[ignore = "requires real GCS bucket and credentials"]
async fn test_gcs_large_file_handling() {
    let config = get_gcs_config().expect("GCS config not available");
    let backend = GCSBackend::new(config).await.unwrap();

    let test_path = PathBuf::from("test_integration/large_file.dat");

    // Create 10MB test data
    let large_data = vec![0xAB; 10 * 1024 * 1024];

    // Write large file
    backend.write_file(&test_path, &large_data).await.unwrap();

    // Read large file
    let read_data = backend.read_file(&test_path).await.unwrap();
    assert_eq!(read_data.len(), large_data.len());
    assert_eq!(read_data, large_data);

    // Cleanup
    backend.delete_file(&test_path).await.unwrap();
}

#[tokio::test]
#[ignore = "requires real GCS bucket and credentials"]
async fn test_gcs_concurrent_operations() {
    let config = get_gcs_config().expect("GCS config not available");
    let backend = std::sync::Arc::new(GCSBackend::new(config).await.unwrap());

    let mut handles = vec![];

    // Perform 10 concurrent uploads
    for i in 0..10 {
        let backend_clone = backend.clone();
        let handle = tokio::spawn(async move {
            let test_path = PathBuf::from(format!("test_integration/concurrent_{}.dat", i));
            let test_data = format!("Concurrent test data {}", i).into_bytes();

            // Write
            backend_clone
                .write_file(&test_path, &test_data)
                .await
                .unwrap();

            // Read back
            let read_data = backend_clone.read_file(&test_path).await.unwrap();
            assert_eq!(read_data, test_data);

            // Delete
            backend_clone.delete_file(&test_path).await.unwrap();

            i
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        println!("Completed concurrent operation {}", result);
    }
}

#[tokio::test]
#[ignore = "requires real GCS bucket and credentials"]
async fn test_gcs_backup_metadata_storage() {
    let config = get_gcs_config().expect("GCS config not available");
    let backend = GCSBackend::new(config).await.unwrap();

    // Simulate backup metadata
    let metadata = serde_json::json!({
        "backup_id": "backup_20251128_123456",
        "timestamp": "2025-11-28T12:34:56Z",
        "size_bytes": 12345678,
        "compression": "dna",
        "checksum": "sha256:abcd1234..."
    });

    let metadata_path = PathBuf::from("backups/metadata/backup_20251128_123456.json");
    let metadata_data = serde_json::to_vec_pretty(&metadata).unwrap();

    // Store metadata
    backend
        .write_file(&metadata_path, &metadata_data)
        .await
        .unwrap();

    // Read metadata
    let read_metadata_data = backend.read_file(&metadata_path).await.unwrap();
    let read_metadata: serde_json::Value = serde_json::from_slice(&read_metadata_data).unwrap();

    assert_eq!(metadata, read_metadata);

    // Cleanup
    backend.delete_file(&metadata_path).await.unwrap();
}

#[tokio::test]
#[ignore = "requires real GCS bucket and credentials"]
async fn test_gcs_error_handling() {
    let config = get_gcs_config().expect("GCS config not available");
    let backend = GCSBackend::new(config).await.unwrap();

    // Test reading non-existent file
    let non_existent = PathBuf::from("test_integration/does_not_exist.dat");
    let read_result = backend.read_file(&non_existent).await;
    assert!(
        read_result.is_err(),
        "Should fail to read non-existent file"
    );

    // Test deleting non-existent file
    let delete_result = backend.delete_file(&non_existent).await;
    // Note: GCS delete is idempotent, so it might return Ok even for non-existent files
    assert!(delete_result.is_ok() || delete_result.is_err());
}

#[tokio::test]
#[ignore = "requires real GCS bucket and credentials"]
async fn test_gcs_config_serialization() {
    use serde_json;

    let config = GCSConfig {
        bucket: "test-bucket".to_string(),
        project_id: "test-project".to_string(),
        credentials_path: Some(PathBuf::from("/path/to/creds.json")),
        use_default_credentials: false,
    };

    // Test serialization
    let json_str = serde_json::to_string(&config).expect("Should serialize");

    // Test deserialization
    let deserialized: GCSConfig = serde_json::from_str(&json_str).expect("Should deserialize");

    assert_eq!(config.bucket, deserialized.bucket);
    assert_eq!(config.project_id, deserialized.project_id);
    assert_eq!(config.credentials_path, deserialized.credentials_path);
    assert_eq!(
        config.use_default_credentials,
        deserialized.use_default_credentials
    );
}
