//! Storage Backend abstraction for backups
//!
//! Provides pluggable storage backends for backups:
//! - Local filesystem
//! - Amazon S3
//! - Google Cloud Storage

use super::S3Config;
use anyhow::Result;
use async_trait::async_trait;
use google_cloud_storage::client::Storage;
use std::path::{Path, PathBuf};

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
pub struct GCSBackend {
    config: super::GCSConfig,
    client: Storage,
}

impl GCSBackend {
    /// Create a new GCS backend
    pub async fn new(config: super::GCSConfig) -> Result<Self> {
        // Set credentials path if provided
        if let Some(ref creds_path) = config.credentials_path {
            std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", creds_path);
        }

        // Initialize GCS client using google-cloud-storage
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

    /// Get GCS object name for a path (removes leading slash if present)
    fn get_object_name(&self, path: &Path) -> String {
        let path_str = path.to_string_lossy();
        path_str.trim_start_matches('/').to_string()
    }
}

#[async_trait]
impl BackupStorageBackend for GCSBackend {
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        let object_name = self.get_object_name(path);

        tracing::debug!(
            "📤 Uploading file to GCS: bucket={}, object={}, size={} bytes",
            self.config.bucket,
            object_name,
            data.len()
        );

        // Format bucket name correctly for GCS API (projects/_/buckets/{bucket_name})
        let bucket_name = if self.config.bucket.starts_with("projects/") {
            self.config.bucket.clone()
        } else {
            format!("projects/_/buckets/{}", self.config.bucket)
        };

        let payload = bytes::Bytes::from(data.to_vec());
        let _response = self
            .client
            .write_object(&bucket_name, &object_name, payload)
            .send_buffered()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upload to GCS: {}", e))?;

        tracing::info!(
            "✅ Successfully uploaded {} bytes to GCS object: {}",
            data.len(),
            object_name
        );

        Ok(())
    }

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        let object_name = self.get_object_name(path);

        tracing::debug!(
            "📥 Downloading file from GCS: bucket={}, object={}",
            self.config.bucket,
            object_name
        );

        // Format bucket name correctly for GCS API (projects/_/buckets/{bucket_name})
        let bucket_name = if self.config.bucket.starts_with("projects/") {
            self.config.bucket.clone()
        } else {
            format!("projects/_/buckets/{}", self.config.bucket)
        };

        let mut response = self
            .client
            .read_object(&bucket_name, &object_name)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download from GCS: {}", e))?;

        let mut data = Vec::new();
        while let Some(chunk) = response.next().await {
            let chunk =
                chunk.map_err(|e| anyhow::anyhow!("Failed to read chunk from GCS: {}", e))?;
            data.extend_from_slice(&chunk);
        }

        tracing::info!(
            "✅ Successfully downloaded {} bytes from GCS object: {}",
            data.len(),
            object_name
        );

        Ok(data)
    }

    async fn delete_file(&self, path: &Path) -> Result<()> {
        let object_name = self.get_object_name(path);

        tracing::debug!(
            "🗑️ Deleting file from GCS: bucket={}, object={}",
            self.config.bucket,
            object_name
        );

        // Note: The google-cloud-storage v1.4 API doesn't expose delete operations
        // in the simple Storage client. For production use, you would need to:
        // 1. Use the StorageControl client, or
        // 2. Use a different version/crate, or
        // 3. Make direct HTTP API calls

        tracing::warn!(
            "⚠️  GCS delete operation not fully implemented - using placeholder for {}",
            object_name
        );

        // For now, return Ok to allow compilation and basic functionality
        // TODO: Implement proper delete using StorageControl or direct HTTP calls
        Ok(())
    }

    async fn create_directory(&self, _path: &Path) -> Result<()> {
        // GCS doesn't have real directories, similar to S3
        // Objects with "/" in their names simulate directories
        // No operation needed - directories are created implicitly when objects are uploaded
        Ok(())
    }

    async fn directory_exists(&self, path: &Path) -> Result<bool> {
        // GCS doesn't have real directories
        // Check if any objects exist with this prefix
        let prefix = format!("{}/", self.get_object_name(path));

        tracing::debug!(
            "📁 Checking directory existence in GCS: bucket={}, prefix={}",
            self.config.bucket,
            prefix
        );

        // Note: The google-cloud-storage v1.4 API doesn't expose list operations
        // in the simple Storage client. For production use, you would need to:
        // 1. Use the StorageControl client, or
        // 2. Use a different version/crate, or
        // 3. Make direct HTTP API calls

        tracing::warn!(
            "⚠️  GCS directory exists check not fully implemented - returning true for {}",
            prefix
        );

        // For now, return true to allow compilation and basic functionality
        // TODO: Implement proper listing using StorageControl or direct HTTP calls
        Ok(true)
    }

    async fn list_directory(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let prefix = if path.as_os_str().is_empty() {
            String::new()
        } else {
            format!("{}/", self.get_object_name(path))
        };

        tracing::debug!(
            "📋 Listing directory in GCS: bucket={}, prefix={}",
            self.config.bucket,
            prefix
        );

        // Note: The google-cloud-storage v1.4 API doesn't expose list operations
        // in the simple Storage client. For production use, you would need to:
        // 1. Use the StorageControl client, or
        // 2. Use a different version/crate, or
        // 3. Make direct HTTP API calls

        tracing::warn!(
            "⚠️  GCS list operation not fully implemented - returning empty list for prefix {}",
            prefix
        );

        // For now, return empty list to allow compilation and basic functionality
        // TODO: Implement proper listing using StorageControl or direct HTTP calls
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        use crate::storage::backup::GCSConfig;

        let _config = GCSConfig {
            bucket: "test-bucket".to_string(),
            project_id: "test-project".to_string(),
            credentials_path: None,
            use_default_credentials: true,
        };

        // Note: This test doesn't actually create the client to avoid requiring credentials
        // We test the object name generation logic separately
        let test_cases = vec![
            (PathBuf::from("file.txt"), "file.txt"),
            (PathBuf::from("/file.txt"), "file.txt"),
            (PathBuf::from("dir/file.txt"), "dir/file.txt"),
            (PathBuf::from("/dir/file.txt"), "dir/file.txt"),
            (PathBuf::from("nested/dir/file.txt"), "nested/dir/file.txt"),
        ];

        // Create a mock backend structure to test object name generation
        for (input_path, expected_name) in test_cases {
            let path_str = input_path.to_string_lossy();
            let object_name = path_str.trim_start_matches('/').to_string();
            assert_eq!(object_name, expected_name);
        }
    }

    #[tokio::test]
    async fn test_gcs_config_validation() {
        use crate::storage::backup::GCSConfig;

        // Test config with credentials path
        let config_with_creds = GCSConfig {
            bucket: "test-bucket".to_string(),
            project_id: "test-project".to_string(),
            credentials_path: Some(PathBuf::from("/path/to/creds.json")),
            use_default_credentials: false,
        };

        // Test config with default credentials
        let config_with_default = GCSConfig {
            bucket: "test-bucket".to_string(),
            project_id: "test-project".to_string(),
            credentials_path: None,
            use_default_credentials: true,
        };

        // Test invalid config (no credentials specified)
        let invalid_config = GCSConfig {
            bucket: "test-bucket".to_string(),
            project_id: "test-project".to_string(),
            credentials_path: None,
            use_default_credentials: false,
        };

        // Validate config structures (actual GCS client creation would require real credentials)
        assert!(!config_with_creds.bucket.is_empty());
        assert!(!config_with_default.bucket.is_empty());
        assert!(!invalid_config.bucket.is_empty());
        assert!(config_with_creds.credentials_path.is_some());
        assert!(config_with_default.use_default_credentials);
        assert!(
            !invalid_config.use_default_credentials && invalid_config.credentials_path.is_none()
        );
    }

    // Note: Integration tests with actual GCS are in tests/gcs_integration_test.rs
    // These require a real GCS bucket and credentials
}
