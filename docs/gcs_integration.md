# Google Cloud Storage Integration

This document provides comprehensive guidance for integrating NeuroQuantumDB with Google Cloud Storage (GCS) for backup and archival operations.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Authentication Setup](#authentication-setup)
- [Configuration](#configuration)
- [Usage Examples](#usage-examples)
- [Performance Optimization](#performance-optimization)
- [Security Best Practices](#security-best-practices)
- [Troubleshooting](#troubleshooting)
- [Cost Management](#cost-management)
- [Migration from S3](#migration-from-s3)

## Overview

The GCS backend provides seamless integration with Google Cloud Storage, offering:

- **Scalable Storage**: Virtually unlimited storage capacity
- **High Durability**: 99.999999999% (11 9's) durability
- **Global Availability**: Multi-region and regional storage options
- **Cost Efficiency**: Multiple storage classes for different access patterns
- **Security**: Encryption at rest and in transit by default
- **Integration**: Native integration with Google Cloud ecosystem

### Neuromorphic Architecture Mapping

The GCS integration follows neuromorphic principles:

- **GCS Backend** → **Hippocampus** (long-term memory consolidation)
- **Backup Operations** → **Sleep-dependent memory consolidation**
- **Restore Operations** → **Context-dependent memory retrieval**
- **Object Lifecycle** → **Synaptic homeostasis and pruning**
- **Multi-region Redundancy** → **Neural pathway redundancy**

## Prerequisites

### Software Requirements

- Rust 1.70+
- NeuroQuantumDB core library
- Internet connectivity
- Google Cloud project with billing enabled

### GCS Setup

1. **Create a GCS Bucket**:
   ```bash
   # Using gcloud CLI
   gsutil mb gs://neuroquantum-backups
   
   # Set bucket permissions
   gsutil iam ch serviceAccount:backup-service@myproject.iam.gserviceaccount.com:objectAdmin gs://neuroquantum-backups
   ```

2. **Configure Bucket Lifecycle** (optional):
   ```json
   {
     "rule": [
       {
         "action": {"type": "SetStorageClass", "storageClass": "NEARLINE"},
         "condition": {"age": 30, "matchesStorageClass": ["STANDARD"]}
       },
       {
         "action": {"type": "SetStorageClass", "storageClass": "COLDLINE"},
         "condition": {"age": 90, "matchesStorageClass": ["NEARLINE"]}
       },
       {
         "action": {"type": "Delete"},
         "condition": {"age": 2555} // 7 years retention
       }
     ]
   }
   ```

## Authentication Setup

### Method 1: Service Account (Recommended for Production)

1. **Create Service Account**:
   ```bash
   gcloud iam service-accounts create neuroquantum-backup \
     --display-name="NeuroQuantumDB Backup Service" \
     --description="Service account for NeuroQuantumDB GCS backups"
   ```

2. **Grant Permissions**:
   ```bash
   gcloud projects add-iam-policy-binding PROJECT_ID \
     --member="serviceAccount:neuroquantum-backup@PROJECT_ID.iam.gserviceaccount.com" \
     --role="roles/storage.objectAdmin"
   ```

3. **Generate Key File**:
   ```bash
   gcloud iam service-accounts keys create /path/to/service-account.json \
     --iam-account=neuroquantum-backup@PROJECT_ID.iam.gserviceaccount.com
   ```

4. **Set Environment Variable**:
   ```bash
   export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
   ```

### Method 2: Application Default Credentials (Development)

```bash
# Authenticate with your user account
gcloud auth application-default login

# Verify authentication
gcloud auth application-default print-access-token
```

### Method 3: Compute Engine Service Account

For applications running on Google Compute Engine, the default service account can be used without additional setup.

## Configuration

### Basic Configuration

```rust
use neuroquantum_core::storage::backup::GCSConfig;
use std::path::PathBuf;

// Service Account authentication
let gcs_config = GCSConfig {
    bucket: "neuroquantum-backups".to_string(),
    project_id: "my-project-123".to_string(),
    credentials_path: Some(PathBuf::from("/path/to/service-account.json")),
    use_default_credentials: false,
};

// Application Default Credentials
let gcs_config_adc = GCSConfig {
    bucket: "neuroquantum-backups".to_string(),
    project_id: "my-project-123".to_string(),
    credentials_path: None,
    use_default_credentials: true,
};
```

### Production Configuration

```toml
# config/prod.toml
[backup.gcs]
bucket = "neuroquantum-prod-backups"
project_id = "neuroquantum-prod"
use_default_credentials = false
credentials_path = "/etc/neuroquantum/gcs-service-account.json"

[backup]
storage_backend = "gcs"
enable_compression = true
enable_encryption = true
compression_algorithm = "dna"
retention_days = 2555  # 7 years
```

### Development Configuration

```toml
# config/dev.toml
[backup.gcs]
bucket = "neuroquantum-dev-backups"
project_id = "neuroquantum-dev"
use_default_credentials = true

[backup]
storage_backend = "gcs"
enable_compression = false
enable_encryption = false
retention_days = 30
```

## Usage Examples

### Basic Backup Operations

```rust
use neuroquantum_core::storage::backup::storage_backend::{BackupStorageBackend, GCSBackend};
use neuroquantum_core::storage::backup::GCSConfig;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GCS backend
    let config = GCSConfig {
        bucket: "neuroquantum-backups".to_string(),
        project_id: "my-project".to_string(),
        credentials_path: None,
        use_default_credentials: true,
    };
    
    let backend = GCSBackend::new(config).await?;
    
    // Create backup
    let backup_data = b"Sample backup data...";
    let backup_path = PathBuf::from("backups/2025-11-28/full_backup.db");
    
    backend.write_file(&backup_path, backup_data).await?;
    println!("✅ Backup uploaded successfully");
    
    // Verify backup
    let restored_data = backend.read_file(&backup_path).await?;
    assert_eq!(backup_data, &restored_data[..]);
    println!("✅ Backup verified");
    
    Ok(())
}
```

### Advanced Backup Workflow

```rust
use neuroquantum_core::storage::backup::{BackupManager, BackupConfig, BackupStorageType};
use neuroquantum_core::storage::backup::GCSConfig;

async fn create_advanced_backup() -> Result<(), Box<dyn std::error::Error>> {
    // Configure backup with GCS
    let gcs_config = GCSConfig {
        bucket: "neuroquantum-prod-backups".to_string(),
        project_id: "neuroquantum-prod".to_string(),
        credentials_path: Some(PathBuf::from("/etc/gcs/service-account.json")),
        use_default_credentials: false,
    };
    
    let backup_config = BackupConfig {
        storage_backend: BackupStorageType::GCS,
        gcs_config: Some(gcs_config),
        enable_compression: true,
        enable_encryption: true,
        compression_algorithm: "dna".to_string(),
        backup_prefix: "production".to_string(),
        ..Default::default()
    };
    
    // Initialize backup manager
    let backup_manager = BackupManager::new(pager, wal_manager, backup_config).await?;
    
    // Create full backup
    let backup_metadata = backup_manager.backup().await?;
    println!("✅ Full backup completed: {}", backup_metadata.backup_id);
    
    // Create incremental backup
    let incremental_metadata = backup_manager.incremental_backup().await?;
    println!("✅ Incremental backup completed: {}", incremental_metadata.backup_id);
    
    Ok(())
}
```

### Batch Operations

```rust
async fn batch_backup_operations() -> Result<(), Box<dyn std::error::Error>> {
    let backend = /* ... initialize backend ... */;
    
    // Batch upload multiple files
    let files = vec![
        ("data/table1.dat", b"table1 data"),
        ("data/table2.dat", b"table2 data"),
        ("logs/transactions.wal", b"transaction log"),
    ];
    
    let mut handles = vec![];
    
    for (path, data) in files {
        let backend = backend.clone();
        let file_path = PathBuf::from(format!("backup/2025-11-28/{}", path));
        let file_data = data.to_vec();
        
        let handle = tokio::spawn(async move {
            backend.write_file(&file_path, &file_data).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all uploads to complete
    for handle in handles {
        handle.await??;
    }
    
    println!("✅ Batch upload completed");
    Ok(())
}
```

## Performance Optimization

### Upload Performance

1. **Concurrent Uploads**: Use parallel uploads for multiple files
2. **Chunked Uploads**: For files > 32MB, consider resumable uploads
3. **Compression**: Enable DNA compression to reduce transfer size
4. **Regional Buckets**: Use buckets in the same region as your compute

```rust
// Optimized upload configuration
let config = GCSConfig {
    bucket: "neuroquantum-us-central1".to_string(), // Regional bucket
    project_id: "neuroquantum-prod".to_string(),
    // ... other config
};
```

### Storage Classes

Configure appropriate storage classes based on access patterns:

```rust
// For frequently accessed backups (recent)
// Use STANDARD storage class (default)

// For infrequently accessed backups (30+ days)
// Configure bucket lifecycle to transition to NEARLINE

// For archival backups (90+ days)
// Configure bucket lifecycle to transition to COLDLINE

// For long-term retention (365+ days)
// Configure bucket lifecycle to transition to ARCHIVE
```

### Performance Benchmarks

Expected performance characteristics:

| Operation | Standard | Nearline | Coldline | Archive |
|-----------|----------|----------|----------|---------|
| Upload | 100-200 MB/s | 100-200 MB/s | 100-200 MB/s | 100-200 MB/s |
| Download | 200-400 MB/s | 200-400 MB/s | Retrieval delay | Retrieval delay |
| First Byte Latency | 5-20ms | 5-20ms | ~1 second | ~5 seconds |
| Availability | 99.95% | 99.9% | 99.9% | 99.9% |

## Security Best Practices

### Access Control

1. **Principle of Least Privilege**:
   ```bash
   # Grant minimal required permissions
   gcloud projects add-iam-policy-binding PROJECT_ID \
     --member="serviceAccount:backup@PROJECT.iam.gserviceaccount.com" \
     --role="roles/storage.objectCreator"  # Write-only for backups
   
   gcloud projects add-iam-policy-binding PROJECT_ID \
     --member="serviceAccount:restore@PROJECT.iam.gserviceaccount.com" \
     --role="roles/storage.objectViewer"   # Read-only for restores
   ```

2. **Bucket-Level Permissions**:
   ```bash
   # Restrict access to specific bucket
   gsutil iam ch serviceAccount:backup@PROJECT.iam.gserviceaccount.com:objectAdmin gs://neuroquantum-backups
   ```

### Encryption

1. **Encryption in Transit**: Automatically enabled (TLS 1.3)

2. **Encryption at Rest**: 
   ```bash
   # Google-managed keys (default)
   gsutil kms encryption gs://neuroquantum-backups
   
   # Customer-managed keys (CMEK)
   gsutil kms encryption -k projects/PROJECT/locations/LOCATION/keyRings/RING/cryptoKeys/KEY gs://neuroquantum-backups
   ```

3. **Client-Side Encryption** (additional layer):
   ```rust
   let backup_config = BackupConfig {
       enable_encryption: true,
       encryption_key: "your-256-bit-encryption-key".to_string(),
       // ... other config
   };
   ```

### Network Security

1. **VPC Service Controls**:
   ```json
   {
     "resources": ["projects/PROJECT_ID"],
     "restrictedServices": ["storage.googleapis.com"],
     "vpcAccessibleServices": {
       "enableRestriction": true,
       "allowedServices": ["storage.googleapis.com"]
     }
   }
   ```

2. **Private Google Access**: Enable for Compute Engine instances without external IPs

## Troubleshooting

### Common Issues

1. **Authentication Errors**:
   ```
   Error: Failed to create GCS client: authentication failed
   
   Solutions:
   - Verify GOOGLE_APPLICATION_CREDENTIALS path
   - Check service account key file permissions
   - Ensure service account has required roles
   - Try: gcloud auth application-default login
   ```

2. **Permission Denied**:
   ```
   Error: Failed to upload to GCS: 403 Forbidden
   
   Solutions:
   - Check IAM permissions for the service account
   - Verify bucket exists and is accessible
   - Ensure project billing is enabled
   ```

3. **Network Connectivity**:
   ```
   Error: Failed to upload to GCS: connection timeout
   
   Solutions:
   - Check firewall rules for HTTPS (443) outbound
   - Verify DNS resolution for storage.googleapis.com
   - Test with: curl -I https://storage.googleapis.com
   ```

### Debug Mode

Enable detailed logging:

```rust
// Set log level to debug
std::env::set_var("RUST_LOG", "neuroquantum_core::storage::backup=debug");
tracing_subscriber::fmt::init();

// The GCS backend will now log detailed operation information
```

### Health Checks

```rust
async fn gcs_health_check() -> Result<bool, Box<dyn std::error::Error>> {
    let backend = GCSBackend::new(config).await?;
    
    // Test write/read/delete cycle
    let test_path = PathBuf::from("health_check/test.dat");
    let test_data = b"health check";
    
    // Write
    backend.write_file(&test_path, test_data).await?;
    
    // Read
    let read_data = backend.read_file(&test_path).await?;
    let success = read_data == test_data;
    
    // Cleanup
    backend.delete_file(&test_path).await?;
    
    Ok(success)
}
```

## Cost Management

### Storage Costs (as of 2025)

| Storage Class | Price/GB/month | Retrieval Fee |
|---------------|----------------|---------------|
| Standard | $0.020 | Free |
| Nearline | $0.010 | $0.010/GB |
| Coldline | $0.004 | $0.020/GB |
| Archive | $0.0012 | $0.050/GB |

### Cost Optimization Strategies

1. **Lifecycle Policies**: Automatically transition to cheaper storage classes
2. **Compression**: Use DNA compression to reduce storage volume
3. **Retention Policies**: Automatically delete old backups
4. **Regional Storage**: Use regional buckets when possible

```json
// Cost-optimized lifecycle policy
{
  "rule": [
    {
      "action": {"type": "SetStorageClass", "storageClass": "NEARLINE"},
      "condition": {"age": 30}
    },
    {
      "action": {"type": "SetStorageClass", "storageClass": "COLDLINE"},
      "condition": {"age": 90}
    },
    {
      "action": {"type": "SetStorageClass", "storageClass": "ARCHIVE"},
      "condition": {"age": 365}
    },
    {
      "action": {"type": "Delete"},
      "condition": {"age": 2555}
    }
  ]
}
```

### Cost Monitoring

```rust
// Estimate backup costs
fn estimate_monthly_cost(backup_size_gb: f64, access_frequency: &str) -> f64 {
    match access_frequency {
        "daily" => backup_size_gb * 0.020,      // Standard
        "weekly" => backup_size_gb * 0.010,     // Nearline
        "monthly" => backup_size_gb * 0.004,    // Coldline
        "yearly" => backup_size_gb * 0.0012,    // Archive
        _ => backup_size_gb * 0.020,
    }
}
```

## Migration from S3

### Configuration Migration

```rust
// From S3 config
let s3_config = S3Config {
    bucket: "neuroquantum-s3-backups".to_string(),
    region: "us-east-1".to_string(),
    access_key_id: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
    secret_access_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
};

// To GCS config
let gcs_config = GCSConfig {
    bucket: "neuroquantum-gcs-backups".to_string(),
    project_id: "neuroquantum-prod".to_string(),
    credentials_path: Some(PathBuf::from("/path/to/service-account.json")),
    use_default_credentials: false,
};
```

### Data Migration

```rust
async fn migrate_s3_to_gcs() -> Result<(), Box<dyn std::error::Error>> {
    let s3_backend = S3Backend::new(s3_config).await?;
    let gcs_backend = GCSBackend::new(gcs_config).await?;
    
    // List all objects in S3
    let objects = s3_backend.list_directory(&PathBuf::from("")).await?;
    
    // Migrate each object
    for object_path in objects {
        println!("Migrating: {}", object_path.display());
        
        // Download from S3
        let data = s3_backend.read_file(&object_path).await?;
        
        // Upload to GCS
        gcs_backend.write_file(&object_path, &data).await?;
        
        println!("✅ Migrated: {}", object_path.display());
    }
    
    Ok(())
}
```

### Feature Comparison

| Feature | S3 | GCS |
|---------|----|----|
| Storage Classes | 6 | 4 |
| Encryption | SSE-S3, SSE-KMS, SSE-C | Google-managed, CMEK |
| Lifecycle | Full support | Full support |
| Versioning | Yes | Yes |
| Cross-Region Replication | Yes | Yes (Turbo Replication) |
| CDN Integration | CloudFront | Cloud CDN |

## Integration Examples

### Complete Backup Workflow

```rust
use neuroquantum_core::storage::backup::*;

async fn production_backup_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize GCS backend
    let gcs_config = GCSConfig {
        bucket: "neuroquantum-prod-backups".to_string(),
        project_id: "neuroquantum-prod".to_string(),
        credentials_path: Some(PathBuf::from("/etc/gcs-key.json")),
        use_default_credentials: false,
    };
    
    let backup_config = BackupConfig {
        storage_backend: BackupStorageType::GCS,
        gcs_config: Some(gcs_config),
        enable_compression: true,
        enable_encryption: true,
        compression_algorithm: "dna".to_string(),
        ..Default::default()
    };
    
    // 2. Create backup manager
    let backup_manager = BackupManager::new(pager, wal_manager, backup_config).await?;
    
    // 3. Perform backup
    let metadata = backup_manager.backup().await?;
    
    // 4. Store metadata in GCS
    let metadata_json = serde_json::to_vec_pretty(&metadata)?;
    let metadata_path = PathBuf::from(format!("metadata/{}.json", metadata.backup_id));
    
    let backend = GCSBackend::new(gcs_config).await?;
    backend.write_file(&metadata_path, &metadata_json).await?;
    
    // 5. Verify backup integrity
    let verification_result = backup_manager.verify_backup(&metadata.backup_id).await?;
    assert!(verification_result.is_valid);
    
    println!("✅ Production backup completed successfully");
    println!("   Backup ID: {}", metadata.backup_id);
    println!("   Size: {} MB", metadata.total_size / 1_048_576);
    println!("   Duration: {:?}", metadata.duration);
    println!("   Compression Ratio: {:.2}:1", metadata.compression_ratio);
    
    Ok(())
}
```

## API Reference

### GCSConfig

```rust
pub struct GCSConfig {
    /// GCS bucket name
    pub bucket: String,
    
    /// Google Cloud project ID
    pub project_id: String,
    
    /// Path to service account key file (optional)
    pub credentials_path: Option<PathBuf>,
    
    /// Use application default credentials
    pub use_default_credentials: bool,
}
```

### GCSBackend Methods

```rust
impl GCSBackend {
    /// Create new GCS backend instance
    pub async fn new(config: GCSConfig) -> Result<Self>;
}

#[async_trait]
impl BackupStorageBackend for GCSBackend {
    /// Write file to GCS
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;
    
    /// Read file from GCS  
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    
    /// Delete file from GCS
    async fn delete_file(&self, path: &Path) -> Result<()>;
    
    /// Create logical directory
    async fn create_directory(&self, path: &Path) -> Result<()>;
    
    /// Check if directory exists
    async fn directory_exists(&self, path: &Path) -> Result<bool>;
    
    /// List directory contents
    async fn list_directory(&self, path: &Path) -> Result<Vec<PathBuf>>;
}
```

---

## Conclusion

The GCS integration provides enterprise-grade backup capabilities with neuromorphic design principles. It offers scalability, durability, and cost-effectiveness for long-term data retention while maintaining the innovative DNA compression and quantum-inspired features of NeuroQuantumDB.

For additional support or advanced configuration questions, please refer to the [Google Cloud Storage documentation](https://cloud.google.com/storage/docs) or the NeuroQuantumDB community forums.
