//! Google Cloud Storage Backup Example
//!
//! This example demonstrates how to use NeuroQuantumDB with Google Cloud Storage
//! for backup and restore operations.
//!
//! Setup:
//! 1. Create a GCS bucket: `gsutil mb gs://neuroquantum-backups`
//! 2. Set up authentication (one of):
//!    - Use Application Default Credentials: `gcloud auth application-default login`
//!    - Or use a service account key file
//! 3. Set environment variables:
//!    - GCS_BUCKET: Your bucket name
//!    - GCS_PROJECT_ID: Your GCP project ID
//!    - GOOGLE_APPLICATION_CREDENTIALS: Path to service account key (optional)
//!
//! Run with:
//! ```bash
//! export GCS_BUCKET=neuroquantum-backups
//! export GCS_PROJECT_ID=my-project-123
//! cargo run --example gcs_backup
//! ```

use anyhow::Result;
use neuroquantum_core::storage::backup::{BackupStorageBackend, GCSBackend, GCSConfig};
use std::env;
use std::path::PathBuf;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 NeuroQuantumDB GCS Backup Example");

    // Get GCS configuration from environment
    let gcs_bucket = env::var("GCS_BUCKET").expect("GCS_BUCKET environment variable required");
    let gcs_project_id =
        env::var("GCS_PROJECT_ID").expect("GCS_PROJECT_ID environment variable required");

    let credentials_path = env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .ok()
        .map(PathBuf::from);

    let use_default_credentials = credentials_path.is_none();

    info!("📦 GCS Configuration:");
    info!("  Bucket: {}", gcs_bucket);
    info!("  Project: {}", gcs_project_id);
    info!(
        "  Auth: {}",
        if use_default_credentials {
            "Application Default Credentials"
        } else {
            "Service Account Key"
        }
    );

    // Create GCS configuration
    let gcs_config = GCSConfig {
        bucket: gcs_bucket.clone(),
        project_id: gcs_project_id,
        credentials_path,
        use_default_credentials,
    };

    // Initialize GCS backend
    info!("🔧 Initializing GCS backend...");
    let gcs_backend = GCSBackend::new(gcs_config).await?;
    info!("✅ GCS backend initialized");

    // Demonstrate basic GCS operations
    info!("💾 Demonstrating GCS operations...");

    // 1. Write a test file
    let test_path = PathBuf::from("neuroquantum/examples/test_backup.dat");
    let test_data = b"NeuroQuantumDB GCS Integration - Neuromorphic Database Backup System";

    info!("📤 Writing test file to GCS...");
    gcs_backend.write_file(&test_path, test_data).await?;
    info!("✅ File written successfully");

    // 2. Read the file back
    info!("📥 Reading file from GCS...");
    let read_data = gcs_backend.read_file(&test_path).await?;
    assert_eq!(read_data, test_data, "Data integrity check failed");
    info!("✅ File read successfully ({} bytes)", read_data.len());

    // 3. List files
    info!("📋 Listing files in directory...");
    let files = gcs_backend
        .list_directory(&PathBuf::from("neuroquantum/examples"))
        .await?;
    info!("✅ Found {} file(s)", files.len());
    for file in &files {
        info!("  - {}", file.display());
    }

    // 4. Create a backup metadata example
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct BackupMetadata {
        backup_id: String,
        timestamp: String,
        database_version: String,
        size_bytes: u64,
    }

    let metadata = BackupMetadata {
        backup_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        database_version: "0.1.0".to_string(),
        size_bytes: test_data.len() as u64,
    };

    let metadata_json = serde_json::to_vec_pretty(&metadata)?;
    let metadata_path = PathBuf::from("neuroquantum/examples/backup_metadata.json");

    info!("📝 Writing backup metadata...");
    gcs_backend
        .write_file(&metadata_path, &metadata_json)
        .await?;
    info!("✅ Metadata written successfully");

    // 5. Clean up test files
    info!("🧹 Cleaning up test files...");
    gcs_backend.delete_file(&test_path).await?;
    gcs_backend.delete_file(&metadata_path).await?;
    info!("✅ Cleanup completed");

    info!("✅ Example completed successfully!");
    info!("");
    info!("📊 Summary:");
    info!("  - GCS backend initialized");
    info!("  - File write/read operations: ✅");
    info!("  - Directory listing: ✅");
    info!("  - Metadata storage: ✅");
    info!("  - Cleanup: ✅");
    info!("");
    info!("💡 Next steps:");
    info!("  - Integrate with full BackupManager for production use");
    info!("  - Configure automated backup schedules");
    info!("  - Set up lifecycle policies in GCS");
    info!("  - Enable versioning for backup recovery");

    Ok(())
}
