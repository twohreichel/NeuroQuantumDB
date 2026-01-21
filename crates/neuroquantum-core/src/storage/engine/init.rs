//! Initialization and setup for `StorageEngine`

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;

use anyhow::{anyhow, Result};
use lru::LruCache;
use tokio::fs;
use tracing::{debug, info};

use super::StorageEngine;
use crate::dna::QuantumDNACompressor;
use crate::storage::encryption::EncryptionManager;
use crate::storage::stats::{DatabaseMetadata, QueryExecutionStats};
use crate::transaction::TransactionManager;

impl StorageEngine {
    /// Create a placeholder storage engine for two-phase initialization
    ///
    /// This is used for synchronous construction of StorageEngine,
    /// which is then properly initialized with async `new()` method.
    ///
    /// **Important:** This should NOT be used directly for production.
    /// Always follow with proper async initialization via `new()`.
    ///
    /// # Example
    /// ```no_run
    /// use neuroquantum_core::storage::StorageEngine;
    /// use std::path::Path;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let data_dir = Path::new("./data");
    /// // Don't use new_placeholder directly - use new() instead
    /// let storage = StorageEngine::new(data_dir).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[doc(hidden)]
    #[must_use]
    pub fn new_placeholder(data_dir: &std::path::Path) -> Self {
        let metadata = DatabaseMetadata {
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now(),
            last_backup: None,
            tables: HashMap::new(),
            next_row_id: 1,
            next_lsn: 1,
        };

        Self {
            data_dir: data_dir.to_path_buf(),
            indexes: HashMap::new(),
            transaction_log: Vec::new(),
            compressed_blocks: HashMap::new(),
            metadata,
            dna_compressor: QuantumDNACompressor::new(),
            next_row_id: 1,
            next_lsn: 1,
            // SAFETY: 10000 is a non-zero constant
            #[allow(clippy::expect_used)]
            row_cache: LruCache::new(NonZeroUsize::new(10000).expect("10000 is non-zero")),
            transaction_manager: TransactionManager::new(),
            encryption_manager: None,
            last_query_stats: QueryExecutionStats::default(),
        }
    }

    /// Create new storage engine instance
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Directory creation fails
    /// - Transaction manager initialization fails
    /// - Encryption manager initialization fails
    /// - Loading existing data fails
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();

        info!("🗄️ Initializing StorageEngine at: {}", data_dir.display());

        // Create directory structure
        Self::create_directory_structure(&data_dir).await?;

        // Initialize DNA compressor
        let dna_compressor = QuantumDNACompressor::new();

        // Load existing metadata or create new
        let metadata = Self::load_or_create_metadata(&data_dir).await?;

        // Initialize transaction manager with real log manager
        let log_dir = data_dir.join("logs");
        let transaction_manager = crate::transaction::TransactionManager::new_async(&log_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize transaction manager: {e}"))?;

        // Initialize encryption manager for data-at-rest encryption
        let encryption_manager = EncryptionManager::new(&data_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize encryption manager: {e}"))?;

        info!(
            "🔐 Encryption-at-rest enabled with key fingerprint: {}",
            encryption_manager.get_key_fingerprint()
        );

        let mut engine = Self {
            data_dir: data_dir.clone(),
            indexes: HashMap::new(),
            transaction_log: Vec::new(),
            compressed_blocks: HashMap::new(),
            metadata,
            dna_compressor,
            next_row_id: 1,
            next_lsn: 1,
            // SAFETY: 10000 is a non-zero constant
            #[allow(clippy::expect_used)]
            row_cache: LruCache::new(NonZeroUsize::new(10000).expect("10000 is non-zero")),
            transaction_manager,
            encryption_manager: Some(encryption_manager),
            last_query_stats: QueryExecutionStats::default(),
        };

        // Load existing data
        engine.load_from_disk().await?;

        Ok(engine)
    }

    /// Create the required directory structure
    pub(crate) async fn create_directory_structure(data_dir: &Path) -> Result<()> {
        let dirs = [
            data_dir,
            &data_dir.join("tables"),
            &data_dir.join("indexes"),
            &data_dir.join("logs"),
            &data_dir.join("quantum"),
        ];

        for dir in &dirs {
            if dir.exists() {
                debug!("📁 Directory already exists: {}", dir.display());
            } else {
                fs::create_dir_all(dir).await.map_err(|e| {
                    anyhow!(
                        "Failed to create directory '{}': {} (Error code: {})",
                        dir.display(),
                        e,
                        e.raw_os_error().unwrap_or(-1)
                    )
                })?;
                info!("📁 Created directory: {}", dir.display());
            }
        }

        Ok(())
    }

    /// Load existing metadata or create new
    pub(crate) async fn load_or_create_metadata(data_dir: &Path) -> Result<DatabaseMetadata> {
        let metadata_path = data_dir.join("metadata.json");

        if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path).await?;
            let metadata: DatabaseMetadata = serde_json::from_str(&content)?;
            info!("📋 Loaded existing metadata");
            Ok(metadata)
        } else {
            let metadata = DatabaseMetadata {
                version: "1.0.0".to_string(),
                created_at: chrono::Utc::now(),
                last_backup: None,
                tables: HashMap::new(),
                next_row_id: 1,
                next_lsn: 1,
            };

            // Save metadata
            let content = serde_json::to_string_pretty(&metadata)?;
            fs::write(&metadata_path, content).await?;

            info!("📋 Created new metadata");
            Ok(metadata)
        }
    }

    /// Initialize transaction manager properly after construction
    ///
    /// # Errors
    ///
    /// Returns an error if transaction manager initialization fails.
    pub async fn init_transaction_manager(&mut self) -> Result<()> {
        let log_dir = self.data_dir.join("logs");
        self.transaction_manager = crate::transaction::TransactionManager::new_async(&log_dir)
            .await
            .map_err(|e| anyhow!("Failed to initialize transaction manager: {e}"))?;

        info!("✅ Transaction manager initialized with ACID support");
        Ok(())
    }
}
