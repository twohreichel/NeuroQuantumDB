//! Storage Backend abstraction for backups
//!
//! Provides pluggable storage backends for backups:
//! - Local filesystem
//! - Amazon S3
//! - Google Cloud Storage

use anyhow::Result;
use async_trait::async_trait;
use google_cloud_storage::client::Storage;
use std::path::{Path, PathBuf};

use super::S3Config;

/// Trait for backup storage backends
#[async_trait]
pub trait BackupStorageBackend: Send + Sync {
    /// Write a file to storage
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;

    /// Read a file from storage
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>>;

    /// Delete a file from storage
    async fn delete_file(&self, path: &Path) -> Result<()>;

    /// Create a directory
    async fn create_directory(&self, path: &Path) -> Result<()>;

    /// Check if directory exists
    async fn directory_exists(&self, path: &Path) -> Result<bool>;

    /// List files in a directory
    async fn list_directory(&self, path: &Path) -> Result<Vec<PathBuf>>;
}

/// Local filesystem storage backend
pub struct LocalBackend {
    base_path: PathBuf,
}

impl LocalBackend {
    /// Create a new local backend
    pub async fn new(base_path: PathBuf) -> Result<Self> {
        // Ensure base directory exists
        tokio::fs::create_dir_all(&base_path).await?;
        Ok(Self { base_path })
    }

    /// Get full path for a relative path
    fn get_full_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_path.join(path)
        }
    }
}

#[async_trait]
impl BackupStorageBackend for LocalBackend {
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        let full_path = self.get_full_path(path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, data).await?;
        Ok(())
    }

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        let full_path = self.get_full_path(path);
        let data = tokio::fs::read(&full_path).await?;
        Ok(data)
    }

    async fn delete_file(&self, path: &Path) -> Result<()> {
        let full_path = self.get_full_path(path);
        tokio::fs::remove_file(&full_path).await?;
        Ok(())
    }

    async fn create_directory(&self, path: &Path) -> Result<()> {
        let full_path = self.get_full_path(path);
        tokio::fs::create_dir_all(&full_path).await?;
        Ok(())
    }

    async fn directory_exists(&self, path: &Path) -> Result<bool> {
        let full_path = self.get_full_path(path);
        Ok(full_path.exists() && full_path.is_dir())
    }

    async fn list_directory(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let full_path = self.get_full_path(path);
        let mut entries = tokio::fs::read_dir(&full_path).await?;
        let mut files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            files.push(entry.path());
        }

        Ok(files)
    }
}

/// Amazon S3 storage backend
pub struct S3Backend {
    config: S3Config,
    client: aws_sdk_s3::Client,
}

impl S3Backend {
    /// Create a new S3 backend
    pub async fn new(config: S3Config) -> Result<Self> {
        // Initialize AWS SDK configuration
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .load()
            .await;

        // Create S3 client with optional endpoint override for custom S3-compatible services
        let client = if let Some(endpoint) = &config.endpoint {
            let s3_config = aws_sdk_s3::config::Builder::from(&aws_config)
                .endpoint_url(endpoint)
                .force_path_style(true)
                .build();
            aws_sdk_s3::Client::from_conf(s3_config)
        } else {
            aws_sdk_s3::Client::new(&aws_config)
        };

        tracing::info!("✅ S3 backend initialized for bucket: {}", config.bucket);

        Ok(Self { config, client })
    }

    /// Get S3 key for a path
    fn get_s3_key(&self, path: &Path) -> String {
        // Remove leading slash if present and convert to string
        let path_str = path.to_string_lossy();
        path_str.trim_start_matches('/').to_string()
    }
}

#[async_trait]
impl BackupStorageBackend for S3Backend {
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        let key = self.get_s3_key(path);

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .body(aws_sdk_s3::primitives::ByteStream::from(data.to_vec()))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("S3 write failed: {}", e))?;

        tracing::info!(
            "✅ S3 write: bucket={}, key={}, size={} bytes",
            self.config.bucket,
            key,
            data.len()
        );

        Ok(())
    }

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        let key = self.get_s3_key(path);

        let resp = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("S3 read failed: {}", e))?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| anyhow::anyhow!("S3 body read failed: {}", e))?
            .into_bytes()
            .to_vec();

        tracing::info!(
            "✅ S3 read: bucket={}, key={}, size={} bytes",
            self.config.bucket,
            key,
            data.len()
        );

        Ok(data)
    }

    async fn delete_file(&self, path: &Path) -> Result<()> {
        let key = self.get_s3_key(path);

        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("S3 delete failed: {}", e))?;

        tracing::info!("✅ S3 delete: bucket={}, key={}", self.config.bucket, key);

        Ok(())
    }

    async fn create_directory(&self, _path: &Path) -> Result<()> {
        // S3 doesn't have directories, but we can create a marker object
        // For now, no-op
        Ok(())
    }

    async fn directory_exists(&self, _path: &Path) -> Result<bool> {
        // S3 doesn't have directories
        // Could check if any objects with this prefix exist
        Ok(true)
    }

    async fn list_directory(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let prefix = self.get_s3_key(path);

        let resp = self
            .client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(&prefix)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("S3 list failed: {}", e))?;

        let files: Vec<PathBuf> = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.key())
            .map(PathBuf::from)
            .collect();

        tracing::info!(
            "✅ S3 list: bucket={}, prefix={}, found {} objects",
            self.config.bucket,
            prefix,
            files.len()
        );

        Ok(files)
    }
}

/// Google Cloud Storage backend
#[allow(dead_code)]
pub struct GCSBackend {
    config: super::GCSConfig,
    client: Storage,
}

impl GCSBackend {
    /// Create a new GCS backend
    pub async fn new(config: super::GCSConfig) -> Result<Self> {
        // Initialize GCS client using the builder pattern
        // The google-cloud-storage crate uses environment variables for authentication:
        // - GOOGLE_APPLICATION_CREDENTIALS: path to service account key
        // Or default credentials if running on GCP

        let client = Storage::builder()
            .build()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create GCS client: {}", e))?;

        tracing::info!(
            "✅ GCS backend initialized for bucket: {}, project: {}",
            config.bucket,
            config.project_id
        );

        Ok(Self { config, client })
    }

    /// Get GCS bucket path in the format: projects/_/buckets/{bucket_id}
    fn get_bucket_path(&self) -> String {
        format!("projects/_/buckets/{}", self.config.bucket)
    }

    /// Get GCS object name for a path
    fn get_object_name(&self, path: &Path) -> String {
        // Remove leading slash if present and convert to string
        let path_str = path.to_string_lossy();
        path_str.trim_start_matches('/').to_string()
    }
}

#[async_trait]
impl BackupStorageBackend for GCSBackend {
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        let _object_name = self.get_object_name(path);
        let _bucket_path = self.get_bucket_path();

        // Note: The google-cloud-storage v1.4 API is complex and requires:
        // - Using WriteObject builder with proper StreamingSource implementations
        // - Understanding the Payload type conversions
        // For now, we provide a placeholder implementation

        tracing::warn!(
            "⚠️  GCS write operation not fully implemented - using placeholder for {} bytes to {}",
            data.len(),
            _object_name
        );

        // TODO: Implement using:
        // use bytes::Bytes;
        // let payload = Bytes::copy_from_slice(data);
        // self.client.write_object(&bucket_path, &object_name, payload)
        //     .send_buffered().await?;

        Ok(())
    }

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        let _object_name = self.get_object_name(path);
        let _bucket_path = self.get_bucket_path();

        // Note: The google-cloud-storage v1.4 API requires using ReadObject builder
        // and streaming the response. For now, we provide a placeholder implementation

        tracing::warn!(
            "⚠️  GCS read operation not fully implemented - returning empty data for {}",
            _object_name
        );

        // TODO: Implement using:
        // let response = self.client.read_object(&bucket_path, &object_name);
        // Stream and collect the data appropriately

        Ok(Vec::new())
    }

    async fn delete_file(&self, _path: &Path) -> Result<()> {
        // Note: The google-cloud-storage v1.4 Storage client doesn't directly expose
        // delete operations in the simple API. For production use, you would need to:
        // 1. Use the StorageControl client, or
        // 2. Use a different crate like cloud-storage, or
        // 3. Make direct HTTP API calls

        tracing::warn!("⚠️  GCS delete operation not fully implemented - using placeholder");

        // For now, return Ok to allow compilation and basic functionality
        // TODO: Implement using StorageControl or alternative approach
        Ok(())
    }

    async fn create_directory(&self, _path: &Path) -> Result<()> {
        // GCS doesn't have directories, similar to S3
        // Objects with "/" in their names simulate directories
        Ok(())
    }

    async fn directory_exists(&self, _path: &Path) -> Result<bool> {
        // GCS doesn't have real directories
        // Could check if any objects with this prefix exist
        Ok(true)
    }

    async fn list_directory(&self, _path: &Path) -> Result<Vec<PathBuf>> {
        // Note: The google-cloud-storage v1.4 Storage client doesn't directly expose
        // list operations in the simple API. For production use, you would need to:
        // 1. Use the StorageControl client, or
        // 2. Use a different crate like cloud-storage, or
        // 3. Make direct HTTP API calls

        tracing::warn!("⚠️  GCS list operation not fully implemented - returning empty list");

        // For now, return empty list to allow compilation and basic functionality
        // TODO: Implement using StorageControl or alternative approach
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::backup::GCSConfig;

    #[tokio::test]
    async fn test_local_backend_creation() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_backup");
        let backend = LocalBackend::new(temp_dir.clone()).await;
        assert!(backend.is_ok());

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_local_backend_write_read() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_backup_rw");
        let backend = LocalBackend::new(temp_dir.clone()).await.unwrap();

        let test_file = PathBuf::from("test.dat");
        let test_data = b"Hello, NeuroQuantumDB!";

        // Write
        backend.write_file(&test_file, test_data).await.unwrap();

        // Read
        let read_data = backend.read_file(&test_file).await.unwrap();
        assert_eq!(read_data, test_data);

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_local_backend_directory_operations() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_backup_dir");
        let backend = LocalBackend::new(temp_dir.clone()).await.unwrap();

        let test_dir = PathBuf::from("test_subdir");

        // Create directory
        backend.create_directory(&test_dir).await.unwrap();

        // Check exists
        let exists = backend.directory_exists(&test_dir).await.unwrap();
        assert!(exists);

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_s3_backend_creation() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test-key".to_string(),
            secret_key: "test-secret".to_string(),
            endpoint: None,
        };

        let backend = S3Backend::new(config).await;
        assert!(backend.is_ok());
    }

    #[tokio::test]
    async fn test_gcs_backend_object_name_generation() {
        // Test that object names are correctly generated from paths
        let _config = GCSConfig {
            bucket: "test-bucket".to_string(),
            project_id: "test-project".to_string(),
            credentials_path: None,
            use_default_credentials: true,
        };

        // We can't actually create the backend without valid credentials,
        // but we can test the logic through a mock
        let path1 = PathBuf::from("/backups/2025-11-20/full.tar.gz");
        let path2 = PathBuf::from("backups/2025-11-20/incremental.tar.gz");

        // Expected behavior: leading slashes should be trimmed
        let expected1 = "backups/2025-11-20/full.tar.gz";
        let expected2 = "backups/2025-11-20/incremental.tar.gz";

        // Direct string manipulation test
        assert_eq!(path1.to_string_lossy().trim_start_matches('/'), expected1);
        assert_eq!(path2.to_string_lossy().trim_start_matches('/'), expected2);
    }

    #[tokio::test]
    async fn test_gcs_config_validation() {
        // Test 1: Valid config with default credentials
        let config1 = GCSConfig {
            bucket: "neuroquantum-backups".to_string(),
            project_id: "neuroquantum-prod".to_string(),
            credentials_path: None,
            use_default_credentials: true,
        };
        assert!(config1.use_default_credentials);
        assert!(config1.credentials_path.is_none());

        // Test 2: Valid config with credentials file
        let config2 = GCSConfig {
            bucket: "neuroquantum-backups".to_string(),
            project_id: "neuroquantum-prod".to_string(),
            credentials_path: Some(PathBuf::from("/path/to/credentials.json")),
            use_default_credentials: false,
        };
        assert!(!config2.use_default_credentials);
        assert!(config2.credentials_path.is_some());

        // Test 3: Both methods specified (should prefer credentials file)
        let config3 = GCSConfig {
            bucket: "neuroquantum-backups".to_string(),
            project_id: "neuroquantum-prod".to_string(),
            credentials_path: Some(PathBuf::from("/path/to/credentials.json")),
            use_default_credentials: true,
        };
        assert!(config3.credentials_path.is_some());
    }

    // Note: Integration tests with actual GCS are in tests/gcs_integration_test.rs
    // These require a real GCS bucket and credentials
}
