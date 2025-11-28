# Google Cloud Storage Integration Guide

This guide covers the complete integration of Google Cloud Storage (GCS) with NeuroQuantumDB's backup system.

## Overview

The GCS backend provides enterprise-grade cloud storage for NeuroQuantumDB backups with the following features:

- **Multi-region redundancy** for high availability
- **Automatic scaling** without capacity planning
- **Encryption** at rest and in transit
- **Lifecycle management** for cost optimization
- **Integration** with Google Cloud ecosystem

## Prerequisites

### 1. Google Cloud Project Setup

```bash
# Create a new project (or use existing)
gcloud projects create neuroquantum-db --name="NeuroQuantumDB"

# Set as default project
gcloud config set project neuroquantum-db

# Enable required APIs
gcloud services enable storage.googleapis.com
gcloud services enable cloudresourcemanager.googleapis.com
```

### 2. Create GCS Bucket

```bash
# Create bucket with appropriate settings
gsutil mb -p neuroquantum-db -c STANDARD -l us-central1 gs://neuroquantum-backups

# Enable versioning (recommended)
gsutil versioning set on gs://neuroquantum-backups

# Set lifecycle rules (optional - for cost optimization)
gsutil lifecycle set lifecycle.json gs://neuroquantum-backups
```

Example `lifecycle.json`:
```json
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
    }
  ]
}
```

### 3. Authentication Setup

#### Option A: Service Account (Recommended for Production)

```bash
# Create service account
gcloud iam service-accounts create neuroquantum-backup \
  --description="NeuroQuantumDB Backup Service" \
  --display-name="NeuroQuantumDB Backup"

# Grant necessary permissions
gcloud projects add-iam-policy-binding neuroquantum-db \
  --member="serviceAccount:neuroquantum-backup@neuroquantum-db.iam.gserviceaccount.com" \
  --role="roles/storage.objectAdmin"

# Create and download key
gcloud iam service-accounts keys create ~/neuroquantum-backup-key.json \
  --iam-account=neuroquantum-backup@neuroquantum-db.iam.gserviceaccount.com

# Set environment variable
export GOOGLE_APPLICATION_CREDENTIALS=~/neuroquantum-backup-key.json
```

#### Option B: Application Default Credentials (Development)

```bash
# Authenticate with your user account
gcloud auth application-default login

# Grant your account storage permissions (development only)
gcloud projects add-iam-policy-binding neuroquantum-db \
  --member="user:$(gcloud config get-value account)" \
  --role="roles/storage.objectAdmin"
```

## Configuration

### Basic Configuration

```toml
# config/prod.toml
[backup]
storage_type = "gcs"
enable_compression = true
enable_encryption = true

[backup.gcs]
bucket = "neuroquantum-backups"
project_id = "neuroquantum-db"
use_default_credentials = false
credentials_path = "/etc/neuroquantum/gcs-key.json"
```

### Rust Code Configuration

```rust
use neuroquantum_core::storage::backup::{GCSConfig, GCSBackend, BackupManager};
use std::path::PathBuf;

let gcs_config = GCSConfig {
    bucket: "neuroquantum-backups".to_string(),
    project_id: "neuroquantum-db".to_string(),
    credentials_path: Some(PathBuf::from("/etc/neuroquantum/gcs-key.json")),
    use_default_credentials: false,
};

let backend = GCSBackend::new(gcs_config).await?;
```

## Usage Examples

### Basic Backup Operations

```rust
use neuroquantum_core::storage::backup::{BackupStorageBackend, GCSBackend, GCSConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize GCS backend
    let config = GCSConfig {
        bucket: "neuroquantum-backups".to_string(),
        project_id: "neuroquantum-db".to_string(),
        credentials_path: None, // Use default credentials
        use_default_credentials: true,
    };
    
    let backend = GCSBackend::new(config).await?;
    
    // Write backup file
    let backup_data = b"Database backup content";
    let backup_path = PathBuf::from("backups/2025/11/28/full_backup.tar.gz");
    backend.write_file(&backup_path, backup_data).await?;
    
    // Read backup file
    let restored_data = backend.read_file(&backup_path).await?;
    assert_eq!(backup_data, &restored_data[..]);
    
    // List backups
    let backups = backend.list_directory(&PathBuf::from("backups/2025/11")).await?;
    for backup in backups {
        println!("Found backup: {}", backup.display());
    }
    
    Ok(())
}
```

### Integration with Backup Manager

```rust
use neuroquantum_core::storage::backup::{BackupManager, BackupConfig, BackupStorageType};

let backup_config = BackupConfig {
    storage_backend: BackupStorageType::GCS,
    gcs_config: Some(gcs_config),
    enable_compression: true,
    enable_encryption: true,
    max_backup_files: 30,
    backup_interval: Duration::from_secs(24 * 3600), // Daily
};

let backup_manager = BackupManager::new(pager, wal_manager, backup_config).await?;

// Create backup
let backup_metadata = backup_manager.backup().await?;
println!("Backup created: {}", backup_metadata.backup_id);

// List available backups
let backups = backup_manager.list_backups().await?;
for backup in backups {
    println!("Backup: {} ({})", backup.backup_id, backup.timestamp);
}

// Restore from backup
backup_manager.restore(&backup_metadata.backup_id).await?;
println!("Database restored successfully");
```

## Performance Optimization

### Storage Classes

Choose appropriate storage class based on access patterns:

```bash
# Standard (hot data, frequent access)
gsutil defstorageclass set STANDARD gs://neuroquantum-backups

# Nearline (warm data, monthly access)
gsutil defstorageclass set NEARLINE gs://neuroquantum-backups

# Coldline (cold data, quarterly access)
gsutil defstorageclass set COLDLINE gs://neuroquantum-backups

# Archive (archival data, yearly access)
gsutil defstorageclass set ARCHIVE gs://neuroquantum-backups
```

### Regional vs Multi-Regional

```bash
# Regional bucket (lower latency, lower cost)
gsutil mb -l us-central1 gs://neuroquantum-backups-regional

# Multi-regional bucket (higher availability, higher cost)
gsutil mb -l us gs://neuroquantum-backups-multiregion
```

### Parallel Uploads

For large datasets, use parallel uploading:

```rust
use tokio::task::JoinSet;

async fn parallel_backup(
    backend: &GCSBackend,
    files: Vec<(PathBuf, Vec<u8>)>
) -> Result<()> {
    let mut join_set = JoinSet::new();
    
    for (path, data) in files {
        let backend = backend.clone(); // Implement Clone for GCSBackend
        join_set.spawn(async move {
            backend.write_file(&path, &data).await
        });
    }
    
    while let Some(result) = join_set.join_next().await {
        result??; // Handle both join and operation errors
    }
    
    Ok(())
}
```

## Security Best Practices

### 1. IAM Permissions

Use principle of least privilege:

```bash
# Minimal permissions for backup operations
gcloud projects add-iam-policy-binding neuroquantum-db \
  --member="serviceAccount:neuroquantum-backup@neuroquantum-db.iam.gserviceaccount.com" \
  --role="roles/storage.objectAdmin"

# For read-only restore operations
gcloud projects add-iam-policy-binding neuroquantum-db \
  --member="serviceAccount:neuroquantum-restore@neuroquantum-db.iam.gserviceaccount.com" \
  --role="roles/storage.objectViewer"
```

### 2. Bucket Policies

```bash
# Disable public access
gsutil iam ch -d allUsers gs://neuroquantum-backups
gsutil iam ch -d allAuthenticatedUsers gs://neuroquantum-backups

# Enable uniform bucket-level access
gsutil uniformbucketlevelaccess set on gs://neuroquantum-backups
```

### 3. Encryption

Enable customer-managed encryption keys (CMEK):

```bash
# Create KMS key
gcloud kms keyrings create neuroquantum-keyring --location=global

gcloud kms keys create neuroquantum-backup-key \
  --keyring=neuroquantum-keyring \
  --location=global \
  --purpose=encryption

# Set default encryption on bucket
gsutil kms encryption \
  -k projects/neuroquantum-db/locations/global/keyRings/neuroquantum-keyring/cryptoKeys/neuroquantum-backup-key \
  gs://neuroquantum-backups
```

## Monitoring and Alerting

### 1. Cloud Monitoring

```bash
# Enable monitoring API
gcloud services enable monitoring.googleapis.com

# Create notification channel (example: email)
gcloud alpha monitoring channels create \
  --display-name="NeuroQuantumDB Alerts" \
  --type=email \
  --channel-labels=email_address=admin@neuroquantum.com
```

### 2. Storage Metrics

Monitor key metrics:
- `storage.googleapis.com/storage/object_count`
- `storage.googleapis.com/storage/total_bytes`
- `storage.googleapis.com/api/request_count`

### 3. Custom Metrics in Application

```rust
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static::lazy_static! {
    static ref GCS_OPERATIONS_TOTAL: Counter = register_counter!(
        "gcs_operations_total",
        "Total number of GCS operations"
    ).unwrap();
    
    static ref GCS_OPERATION_DURATION: Histogram = register_histogram!(
        "gcs_operation_duration_seconds",
        "Duration of GCS operations"
    ).unwrap();
}

impl GCSBackend {
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        let _timer = GCS_OPERATION_DURATION.start_timer();
        GCS_OPERATIONS_TOTAL.inc();
        
        // ... implementation ...
        
        Ok(())
    }
}
```

## Cost Optimization

### 1. Lifecycle Management

Automatically transition objects to cheaper storage classes:

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
      "action": {"type": "SetStorageClass", "storageClass": "ARCHIVE"},
      "condition": {"age": 365, "matchesStorageClass": ["COLDLINE"]}
    },
    {
      "action": {"type": "Delete"},
      "condition": {"age": 2555} // 7 years retention
    }
  ]
}
```

### 2. Compression

Enable compression in NeuroQuantumDB:

```rust
let backup_config = BackupConfig {
    enable_compression: true, // Use DNA compression
    compression_level: 9,     // Maximum compression
    // ...
};
```

### 3. Deduplication

Implement client-side deduplication:

```rust
use std::collections::HashMap;
use sha3::{Digest, Sha3_256};

struct DeduplicatingGCSBackend {
    inner: GCSBackend,
    hash_cache: HashMap<String, PathBuf>,
}

impl DeduplicatingGCSBackend {
    async fn write_file_deduplicated(&mut self, path: &Path, data: &[u8]) -> Result<()> {
        let hash = format!("{:x}", Sha3_256::digest(data));
        
        if let Some(existing_path) = self.hash_cache.get(&hash) {
            // Create symlink-like reference instead of duplicate storage
            self.create_reference(path, existing_path).await?;
        } else {
            self.inner.write_file(path, data).await?;
            self.hash_cache.insert(hash, path.to_path_buf());
        }
        
        Ok(())
    }
}
```

## Troubleshooting

### Common Issues

1. **Authentication Errors**
   ```
   Error: Failed to create GCS client: No credentials found
   ```
   - Verify `GOOGLE_APPLICATION_CREDENTIALS` environment variable
   - Check service account key file exists and is readable
   - Ensure service account has necessary permissions

2. **Permission Denied**
   ```
   Error: Failed to upload to GCS: Forbidden (403)
   ```
   - Check IAM permissions for service account
   - Verify bucket exists and is accessible
   - Check bucket-level IAM policies

3. **Network Issues**
   ```
   Error: Failed to upload to GCS: Connection timeout
   ```
   - Check firewall rules for HTTPS (port 443)
   - Verify internet connectivity
   - Consider using private Google access for VPC environments

### Debugging

Enable detailed logging:

```rust
use tracing::{Level, info, debug};
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .with_target(true)
    .init();

// Logs will show detailed GCS operation information
```

### Health Checks

Implement health checks for GCS connectivity:

```rust
impl GCSBackend {
    pub async fn health_check(&self) -> Result<()> {
        // Simple connectivity test
        let test_data = b"health_check";
        let test_path = PathBuf::from(".health_check");
        
        // Write, read, delete test file
        self.write_file(&test_path, test_data).await?;
        let read_data = self.read_file(&test_path).await?;
        assert_eq!(read_data, test_data);
        self.delete_file(&test_path).await?;
        
        Ok(())
    }
}
```

## Migration Guide

### From Local Storage

```rust
async fn migrate_from_local(
    local_backend: &LocalBackend,
    gcs_backend: &GCSBackend,
    local_path: &Path
) -> Result<()> {
    let files = local_backend.list_directory(local_path).await?;
    
    for file_path in files {
        let data = local_backend.read_file(&file_path).await?;
        gcs_backend.write_file(&file_path, &data).await?;
        println!("Migrated: {}", file_path.display());
    }
    
    Ok(())
}
```

### From S3

```rust
async fn migrate_from_s3(
    s3_backend: &S3Backend,
    gcs_backend: &GCSBackend,
) -> Result<()> {
    let files = s3_backend.list_directory(&PathBuf::new()).await?;
    
    for file_path in files {
        let data = s3_backend.read_file(&file_path).await?;
        gcs_backend.write_file(&file_path, &data).await?;
        println!("Migrated from S3: {}", file_path.display());
    }
    
    Ok(())
}
```

## Integration Testing

Run integration tests with real GCS:

```bash
# Set up test environment
export GCS_TEST_BUCKET=neuroquantum-test-$(date +%s)
export GCS_TEST_PROJECT_ID=neuroquantum-db

# Create temporary test bucket
gsutil mb gs://$GCS_TEST_BUCKET

# Run integration tests
cargo test --test gcs_integration_test -- --ignored

# Cleanup test bucket
gsutil rm -r gs://$GCS_TEST_BUCKET
```

## References

- [Google Cloud Storage Documentation](https://cloud.google.com/storage/docs)
- [cloud-storage Rust Crate](https://docs.rs/cloud-storage/)
- [Google Cloud IAM Best Practices](https://cloud.google.com/iam/docs/using-iam-securely)
- [GCS Performance Guidelines](https://cloud.google.com/storage/docs/request-rate)

---

**Neuromorphic Integration Notes:**

The GCS backend represents the **long-term memory system** of NeuroQuantumDB, analogous to the brain's hippocampus and cortical storage areas:

- **Hierarchical Organization:** Bucket/folder structure mirrors cortical layering
- **Redundancy:** Multi-region replication resembles neural pathway redundancy  
- **Adaptive Access:** Storage classes adapt like synaptic strength based on usage
- **Consolidation:** Lifecycle policies mirror memory consolidation processes

This biological inspiration ensures both robust data preservation and cost-effective storage management.
