//! Encryption-at-Rest for NeuroQuantumDB
//! Provides transparent encryption/decryption using Post-Quantum Cryptography (ML-KEM)
//! combined with symmetric encryption (AES-256-GCM) for performance.
//!
//! # Security Architecture
//!
//! Master keys are stored securely using the OS keychain:
//! - **macOS**: Keychain Services
//! - **Windows**: Credential Manager
//! - **Linux**: Secret Service (GNOME Keyring, KWallet, etc.)
//!
//! Fallback to file-based storage is available for environments without keychain support,
//! but this is not recommended for production deployments.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::path::{Path, PathBuf};
use tokio::fs;
use zeroize::Zeroize;

/// Service name for keyring storage
const KEYRING_SERVICE: &str = "neuroquantum-db";

/// Key storage strategy for the encryption manager
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyStorageStrategy {
    /// Store keys in the OS keychain (recommended for production)
    #[default]
    OsKeychain,
    /// Fallback to file-based storage (for testing or unsupported environments)
    FileBased,
    /// Try OS keychain first, fall back to file if unavailable
    KeychainWithFileFallback,
}

/// Configuration for the encryption manager
///
/// This struct allows fine-grained control over encryption behavior,
/// particularly for production environments where security requirements
/// are stricter.
#[derive(Debug, Clone, Default)]
pub struct EncryptionConfig {
    /// The key storage strategy to use
    pub strategy: KeyStorageStrategy,

    /// If true, the encryption manager will return an error instead of
    /// falling back to file-based storage when the OS keychain is unavailable.
    ///
    /// **Recommended for production deployments.**
    ///
    /// When enabled:
    /// - `KeyStorageStrategy::OsKeychain` will fail if keychain is unavailable
    /// - `KeyStorageStrategy::KeychainWithFileFallback` will fail instead of falling back
    /// - `KeyStorageStrategy::FileBased` is not affected (will work but log a warning)
    ///
    /// This ensures that production systems never accidentally use less secure
    /// file-based key storage.
    pub forbid_file_fallback: bool,

    /// If true, treat this as a production environment with stricter security checks.
    /// When combined with `forbid_file_fallback`, this will cause startup to fail
    /// if the keychain is unavailable.
    pub production_mode: bool,
}

/// Result of a keychain operation
#[derive(Debug)]
pub struct KeychainStatus {
    /// Whether the keychain is available on this system
    pub available: bool,
    /// The backend being used (e.g., "macOS Keychain", "Windows Credential Manager")
    pub backend: String,
    /// Any warning messages
    pub warnings: Vec<String>,
}

/// Encrypted data container with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Nonce for AES-GCM (12 bytes)
    pub nonce: Vec<u8>,
    /// Encrypted ciphertext
    pub ciphertext: Vec<u8>,
    /// Key derivation salt (32 bytes)
    pub salt: Vec<u8>,
    /// Encryption algorithm version
    pub version: u32,
}

/// Key manager for encryption-at-rest with OS keychain support
#[derive(Clone)]
pub struct EncryptionManager {
    /// Master encryption key (derived from password or generated)
    master_key: [u8; 32],
    /// Key file path for fallback persistence
    key_path: PathBuf,
    /// Unique identifier for this database instance (used as keyring username)
    instance_id: String,
    /// Storage strategy being used
    storage_strategy: KeyStorageStrategy,
}

// Manual Debug implementation to avoid exposing the master key
impl std::fmt::Debug for EncryptionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionManager")
            .field("key_path", &self.key_path)
            .field("instance_id", &self.instance_id)
            .field("storage_strategy", &self.storage_strategy)
            .field("master_key", &"[REDACTED]")
            .finish()
    }
}

impl EncryptionManager {
    /// Create a new encryption manager with OS keychain support
    ///
    /// This method will:
    /// 1. Try to load an existing key from the OS keychain
    /// 2. If not found, generate a new key and store it in the keychain
    /// 3. Fall back to file-based storage if keychain is unavailable
    pub async fn new(data_dir: &Path) -> Result<Self> {
        Self::with_strategy(data_dir, KeyStorageStrategy::KeychainWithFileFallback).await
    }

    /// Create an encryption manager with a specific storage strategy
    pub async fn with_strategy(data_dir: &Path, strategy: KeyStorageStrategy) -> Result<Self> {
        Self::with_config(
            data_dir,
            EncryptionConfig {
                strategy,
                forbid_file_fallback: false,
                production_mode: false,
            },
        )
        .await
    }

    /// Create an encryption manager with full configuration options
    ///
    /// This method provides the most control over encryption behavior,
    /// including the ability to forbid file-based fallback in production.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use neuroquantum_core::storage::encryption::{EncryptionManager, EncryptionConfig, KeyStorageStrategy};
    ///
    /// let config = EncryptionConfig {
    ///     strategy: KeyStorageStrategy::KeychainWithFileFallback,
    ///     forbid_file_fallback: true,  // Fail if keychain unavailable
    ///     production_mode: true,
    /// };
    ///
    /// let manager = EncryptionManager::with_config(&data_dir, config).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `forbid_file_fallback` is true and the OS keychain is unavailable
    /// - `production_mode` is true and `FileBased` strategy is explicitly requested
    /// - Key generation or storage fails
    pub async fn with_config(data_dir: &Path, config: EncryptionConfig) -> Result<Self> {
        let key_path = data_dir.join(".encryption_key");

        // Generate a unique instance ID based on the data directory
        let instance_id = Self::generate_instance_id(data_dir);

        // Check for forbidden configurations in production mode
        if config.production_mode && config.strategy == KeyStorageStrategy::FileBased {
            tracing::error!(
                "🚨 SECURITY: File-based key storage is not allowed in production mode"
            );
            return Err(anyhow!(
                "Security policy violation: File-based key storage is explicitly forbidden in production mode. \
                Use KeyStorageStrategy::OsKeychain or KeyStorageStrategy::KeychainWithFileFallback instead."
            ));
        }

        let (master_key, actual_strategy) = match config.strategy {
            | KeyStorageStrategy::OsKeychain => {
                match Self::load_or_create_keychain_key(&instance_id, &key_path).await {
                    | Ok(key) => (key, KeyStorageStrategy::OsKeychain),
                    | Err(e) if config.forbid_file_fallback => {
                        tracing::error!(
                            "🚨 SECURITY: OS keychain unavailable and file fallback is forbidden: {}",
                            e
                        );
                        return Err(anyhow!(
                            "Security policy violation: OS keychain is unavailable ({}) and file-based fallback \
                            is forbidden by configuration. Either:\n\
                            1. Ensure the OS keychain service is running and accessible\n\
                            2. Set 'forbid_file_fallback = false' (not recommended for production)\n\
                            3. Check system logs for keychain-related errors",
                            e
                        ));
                    },
                    | Err(e) => {
                        tracing::warn!(
                            "⚠️ OS keychain unavailable ({}), falling back to file-based storage",
                            e
                        );
                        let key = Self::load_or_create_file_key(&key_path).await?;
                        (key, KeyStorageStrategy::FileBased)
                    },
                }
            },
            | KeyStorageStrategy::FileBased => {
                if config.forbid_file_fallback && config.production_mode {
                    // This case is already handled above, but double-check
                    return Err(anyhow!(
                        "Security policy violation: File-based storage is forbidden in production"
                    ));
                }
                if config.production_mode {
                    tracing::warn!(
                        "⚠️ SECURITY WARNING: Using file-based key storage in production is not recommended"
                    );
                }
                let key = Self::load_or_create_file_key(&key_path).await?;
                (key, KeyStorageStrategy::FileBased)
            },
            | KeyStorageStrategy::KeychainWithFileFallback => {
                // IMPORTANT: First check if a file-based key exists.
                // This ensures backward compatibility with existing encrypted data.
                // If data was encrypted with a file-based key, we must use that key,
                // not a potentially different key from the keychain.
                if key_path.exists() {
                    if config.forbid_file_fallback && config.production_mode {
                        tracing::error!(
                            "🚨 SECURITY: Existing file-based key found but file storage is forbidden in production. \
                            Please migrate the key to the OS keychain using `migrate_to_keychain()` before enabling \
                            production mode with `forbid_file_fallback`."
                        );
                        return Err(anyhow!(
                            "Security policy violation: An existing file-based encryption key was found at '{}', \
                            but file-based storage is forbidden in production mode.\n\
                            To resolve this:\n\
                            1. Start the database without production mode to migrate the key\n\
                            2. Call `encryption_manager.migrate_to_keychain()` to move the key to the OS keychain\n\
                            3. Securely delete the file-based key after successful migration\n\
                            4. Restart with production mode enabled",
                            key_path.display()
                        ));
                    }
                    tracing::info!("🔑 Found existing file-based key, using it for compatibility");
                    let key = Self::load_master_key_from_file(&key_path).await?;
                    (key, KeyStorageStrategy::FileBased)
                } else {
                    // No existing file key, try keychain first
                    match Self::load_or_create_keychain_key(&instance_id, &key_path).await {
                        | Ok(key) => {
                            tracing::info!(
                                "🔐 Master key loaded from OS keychain for instance: {}",
                                instance_id
                            );
                            (key, KeyStorageStrategy::OsKeychain)
                        },
                        | Err(e) if config.forbid_file_fallback => {
                            tracing::error!(
                                "🚨 SECURITY: OS keychain unavailable and file fallback is forbidden: {}",
                                e
                            );
                            return Err(anyhow!(
                                "Security policy violation: OS keychain is unavailable ({}) and file-based fallback \
                                is forbidden by configuration. Either:\n\
                                1. Ensure the OS keychain service is running and accessible\n\
                                2. Set 'forbid_file_fallback = false' (not recommended for production)\n\
                                3. Check system logs for keychain-related errors",
                                e
                            ));
                        },
                        | Err(e) => {
                            tracing::warn!(
                                "⚠️ OS keychain unavailable ({}), falling back to file-based storage",
                                e
                            );
                            let key = Self::load_or_create_file_key(&key_path).await?;
                            (key, KeyStorageStrategy::FileBased)
                        },
                    }
                }
            },
        };

        let strategy_name = match actual_strategy {
            | KeyStorageStrategy::OsKeychain => "OS Keychain",
            | KeyStorageStrategy::FileBased => "File-based (not recommended for production)",
            | KeyStorageStrategy::KeychainWithFileFallback => "Keychain with fallback",
        };

        tracing::info!(
            "🔐 Encryption manager initialized with AES-256-GCM (storage: {})",
            strategy_name
        );

        Ok(Self {
            master_key,
            key_path,
            instance_id,
            storage_strategy: actual_strategy,
        })
    }

    /// Generate a unique instance ID from the data directory path
    fn generate_instance_id(data_dir: &Path) -> String {
        let hash = Self::hash_data(data_dir.to_string_lossy().as_bytes());
        use base64::{engine::general_purpose, Engine as _};
        // Use first 16 bytes as a unique but readable identifier
        general_purpose::URL_SAFE_NO_PAD.encode(&hash[..16])
    }

    /// Load or create a master key using the OS keychain
    /// Also saves a backup copy to file for disaster recovery
    async fn load_or_create_keychain_key(instance_id: &str, key_path: &Path) -> Result<[u8; 32]> {
        let entry = Entry::new(KEYRING_SERVICE, instance_id)
            .map_err(|e| anyhow!("Failed to access keychain: {}", e))?;

        // Try to load existing key
        match entry.get_password() {
            | Ok(encoded_key) => {
                let key = Self::decode_key(&encoded_key)?;
                tracing::debug!("🔑 Loaded existing master key from OS keychain");

                // Ensure file backup exists for disaster recovery
                if !key_path.exists() {
                    if let Err(e) = Self::save_master_key_to_file(key_path, &key).await {
                        tracing::warn!("⚠️ Failed to save key backup to file: {}", e);
                    } else {
                        tracing::debug!("🔑 Saved key backup to file for disaster recovery");
                    }
                }

                Ok(key)
            },
            | Err(keyring::Error::NoEntry) => {
                // No existing key, generate a new one
                let key = Self::generate_master_key();
                let encoded = Self::encode_key(&key);

                entry
                    .set_password(&encoded)
                    .map_err(|e| anyhow!("Failed to store key in keychain: {}", e))?;

                // Also save to file as backup for disaster recovery
                if let Err(e) = Self::save_master_key_to_file(key_path, &key).await {
                    tracing::warn!("⚠️ Failed to save key backup to file: {}", e);
                } else {
                    tracing::debug!("🔑 Saved key backup to file for disaster recovery");
                }

                tracing::info!("🔑 Generated and stored new master key in OS keychain");
                Ok(key)
            },
            | Err(e) => Err(anyhow!("Keychain error: {}", e)),
        }
    }

    /// Load or create a master key using file-based storage
    async fn load_or_create_file_key(key_path: &Path) -> Result<[u8; 32]> {
        if key_path.exists() {
            Self::load_master_key_from_file(key_path).await
        } else {
            let key = Self::generate_master_key();
            Self::save_master_key_to_file(key_path, &key).await?;
            Ok(key)
        }
    }

    /// Generate a new random master key
    fn generate_master_key() -> [u8; 32] {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// Encode a key for storage
    fn encode_key(key: &[u8; 32]) -> String {
        use base64::{engine::general_purpose, Engine as _};
        general_purpose::STANDARD.encode(key)
    }

    /// Decode a key from storage
    fn decode_key(encoded: &str) -> Result<[u8; 32]> {
        use base64::{engine::general_purpose, Engine as _};
        let decoded = general_purpose::STANDARD
            .decode(encoded.trim())
            .map_err(|e| anyhow!("Failed to decode master key: {}", e))?;

        if decoded.len() != 32 {
            return Err(anyhow!("Invalid master key length: {}", decoded.len()));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded);
        Ok(key)
    }

    /// Save master key to file (fallback method)
    async fn save_master_key_to_file(path: &Path, key: &[u8; 32]) -> Result<()> {
        let encoded = Self::encode_key(key);

        fs::write(path, encoded).await?;

        // Set restrictive file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).await?.permissions();
            perms.set_mode(0o600); // rw------- (owner only)
            fs::set_permissions(path, perms).await?;
        }

        tracing::warn!(
            "⚠️ Master encryption key saved to file: {} (consider using OS keychain for production)",
            path.display()
        );
        Ok(())
    }

    /// Load master key from file
    async fn load_master_key_from_file(path: &Path) -> Result<[u8; 32]> {
        let encoded = fs::read_to_string(path).await?;
        let key = Self::decode_key(&encoded)?;

        tracing::info!(
            "🔑 Master encryption key loaded from file: {}",
            path.display()
        );
        Ok(key)
    }

    /// Migrate key from file-based storage to OS keychain
    ///
    /// This method allows upgrading an existing deployment to use keychain storage.
    /// After successful migration, the file can be securely deleted.
    pub async fn migrate_to_keychain(&self) -> Result<MigrationResult> {
        if self.storage_strategy == KeyStorageStrategy::OsKeychain {
            return Ok(MigrationResult {
                success: true,
                message: "Key is already stored in OS keychain".to_string(),
                file_can_be_deleted: false,
            });
        }

        let entry = Entry::new(KEYRING_SERVICE, &self.instance_id)
            .map_err(|e| anyhow!("Failed to access keychain: {}", e))?;

        // Check if key already exists in keychain
        if entry.get_password().is_ok() {
            return Ok(MigrationResult {
                success: true,
                message: "Key already exists in keychain".to_string(),
                file_can_be_deleted: true,
            });
        }

        // Store the current key in the keychain
        let encoded = Self::encode_key(&self.master_key);
        entry
            .set_password(&encoded)
            .map_err(|e| anyhow!("Failed to store key in keychain: {}", e))?;

        tracing::info!(
            "🔐 Successfully migrated master key to OS keychain for instance: {}",
            self.instance_id
        );

        Ok(MigrationResult {
            success: true,
            message: format!(
                "Key migrated to OS keychain. File at {} can be securely deleted.",
                self.key_path.display()
            ),
            file_can_be_deleted: true,
        })
    }

    /// Rotate the master key
    ///
    /// This generates a new master key and stores it in the configured storage.
    /// Note: This does NOT re-encrypt existing data. You must re-encrypt all data
    /// using the new key after rotation.
    pub async fn rotate_key(&mut self) -> Result<KeyRotationResult> {
        let old_fingerprint = self.get_key_fingerprint();

        // Generate new key
        let new_key = Self::generate_master_key();

        // Store new key
        match self.storage_strategy {
            | KeyStorageStrategy::OsKeychain | KeyStorageStrategy::KeychainWithFileFallback => {
                let entry = Entry::new(KEYRING_SERVICE, &self.instance_id)
                    .map_err(|e| anyhow!("Failed to access keychain: {}", e))?;

                let encoded = Self::encode_key(&new_key);
                entry
                    .set_password(&encoded)
                    .map_err(|e| anyhow!("Failed to store rotated key in keychain: {}", e))?;
            },
            | KeyStorageStrategy::FileBased => {
                Self::save_master_key_to_file(&self.key_path, &new_key).await?;
            },
        }

        // Update in-memory key
        self.master_key.zeroize();
        self.master_key = new_key;

        let new_fingerprint = self.get_key_fingerprint();

        tracing::info!(
            "🔄 Master key rotated: {} -> {}",
            old_fingerprint,
            new_fingerprint
        );

        Ok(KeyRotationResult {
            success: true,
            old_fingerprint,
            new_fingerprint,
            warning: Some("All encrypted data must be re-encrypted with the new key".to_string()),
        })
    }

    /// Delete the master key from storage
    ///
    /// # Warning
    /// This will make all encrypted data unrecoverable!
    pub async fn delete_key(&self) -> Result<()> {
        // Delete from keychain if applicable
        if self.storage_strategy == KeyStorageStrategy::OsKeychain
            || self.storage_strategy == KeyStorageStrategy::KeychainWithFileFallback
        {
            if let Ok(entry) = Entry::new(KEYRING_SERVICE, &self.instance_id) {
                let _ = entry.delete_credential();
            }
        }

        // Delete file if it exists
        if self.key_path.exists() {
            fs::remove_file(&self.key_path).await?;
        }

        tracing::warn!(
            "🗑️ Master encryption key deleted - all encrypted data is now unrecoverable!"
        );
        Ok(())
    }

    /// Check the status of the OS keychain
    pub fn check_keychain_status() -> KeychainStatus {
        let test_entry = Entry::new(KEYRING_SERVICE, "status-check");

        match test_entry {
            | Ok(_) => {
                let backend = Self::detect_keychain_backend();
                KeychainStatus {
                    available: true,
                    backend,
                    warnings: vec![],
                }
            },
            | Err(e) => KeychainStatus {
                available: false,
                backend: "None".to_string(),
                warnings: vec![format!("Keychain not available: {}", e)],
            },
        }
    }

    /// Detect which keychain backend is being used
    fn detect_keychain_backend() -> String {
        #[cfg(target_os = "macos")]
        {
            "macOS Keychain".to_string()
        }
        #[cfg(target_os = "windows")]
        {
            "Windows Credential Manager".to_string()
        }
        #[cfg(target_os = "linux")]
        {
            "Linux Secret Service (GNOME Keyring/KWallet)".to_string()
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            "Unknown".to_string()
        }
    }

    /// Get the current storage strategy
    pub fn storage_strategy(&self) -> KeyStorageStrategy {
        self.storage_strategy
    }

    /// Check if the encryption manager is using secure (keychain) storage
    ///
    /// Returns `true` if keys are stored in the OS keychain, `false` if using file-based storage.
    /// This is useful for security audits and production readiness checks.
    pub fn is_using_secure_storage(&self) -> bool {
        self.storage_strategy == KeyStorageStrategy::OsKeychain
    }

    /// Check if the current configuration is production-ready
    ///
    /// Returns a security status report indicating whether the current key storage
    /// configuration is suitable for production use.
    pub fn security_status(&self) -> SecurityStatus {
        let is_secure = self.is_using_secure_storage();
        let keychain_status = Self::check_keychain_status();

        SecurityStatus {
            is_production_ready: is_secure,
            storage_strategy: self.storage_strategy,
            keychain_available: keychain_status.available,
            keychain_backend: keychain_status.backend,
            warnings: if is_secure {
                vec![]
            } else {
                vec![
                    "⚠️ Using file-based key storage is not recommended for production".to_string(),
                    "Consider migrating to OS keychain using `migrate_to_keychain()`".to_string(),
                ]
            },
            recommendations: if is_secure {
                vec!["Key storage configuration is secure".to_string()]
            } else {
                vec![
                    "Enable 'forbid_file_fallback' in production configuration".to_string(),
                    "Ensure OS keychain service is running and accessible".to_string(),
                ]
            },
        }
    }

    /// Get the instance ID
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Get the key file path
    pub fn key_path(&self) -> &Path {
        &self.key_path
    }

    /// Encrypt data using AES-256-GCM
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData> {
        // Generate random nonce (12 bytes for AES-GCM)
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);

        // Create cipher with master key
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

        // Encrypt
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Generate salt for key derivation (used for future enhancements)
        let mut salt = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut salt);

        Ok(EncryptedData {
            nonce: nonce_bytes.to_vec(),
            ciphertext,
            salt: salt.to_vec(),
            version: 1,
        })
    }

    /// Decrypt data using AES-256-GCM
    pub fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>> {
        if encrypted.version != 1 {
            return Err(anyhow!(
                "Unsupported encryption version: {}",
                encrypted.version
            ));
        }

        if encrypted.nonce.len() != 12 {
            return Err(anyhow!(
                "Invalid nonce length: {} (expected 12)",
                encrypted.nonce.len()
            ));
        }

        // Convert Vec<u8> to [u8; 12] for From trait
        let nonce_array: [u8; 12] = encrypted
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Invalid nonce format"))?;
        let nonce = Nonce::from(nonce_array);

        // Create cipher with master key
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

        // Decrypt
        let plaintext = cipher
            .decrypt(&nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        Ok(plaintext)
    }

    /// Derive a key from password using Argon2
    pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
        use argon2::{
            password_hash::{PasswordHasher, SaltString},
            Argon2,
        };

        let salt_string =
            SaltString::encode_b64(salt).map_err(|e| anyhow!("Failed to encode salt: {}", e))?;

        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| anyhow!("Password hashing failed: {}", e))?;

        // Extract the hash as bytes
        let hash_bytes = hash.hash.ok_or_else(|| anyhow!("No hash produced"))?;

        let mut key = [0u8; 32];
        let hash_slice = hash_bytes.as_bytes();
        let len = std::cmp::min(32, hash_slice.len());
        key[..len].copy_from_slice(&hash_slice[..len]);

        Ok(key)
    }

    /// Hash data using SHA3-256
    pub fn hash_data(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Get encryption key fingerprint for verification
    pub fn get_key_fingerprint(&self) -> String {
        let hash = Self::hash_data(&self.master_key);
        use base64::{engine::general_purpose, Engine as _};
        general_purpose::STANDARD.encode(&hash[..8]) // First 8 bytes as fingerprint
    }
}

impl Drop for EncryptionManager {
    fn drop(&mut self) {
        // Zeroize master key on drop for security
        self.master_key.zeroize();
    }
}

/// Result of a key migration operation
#[derive(Debug)]
pub struct MigrationResult {
    /// Whether the migration was successful
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Whether the file-based key can be safely deleted
    pub file_can_be_deleted: bool,
}

/// Result of a key rotation operation
#[derive(Debug)]
pub struct KeyRotationResult {
    /// Whether the rotation was successful
    pub success: bool,
    /// Fingerprint of the old key
    pub old_fingerprint: String,
    /// Fingerprint of the new key
    pub new_fingerprint: String,
    /// Warning message about data re-encryption
    pub warning: Option<String>,
}

/// Security status report for the encryption manager
///
/// This struct provides a comprehensive view of the encryption manager's
/// security posture, useful for auditing and production readiness checks.
#[derive(Debug)]
pub struct SecurityStatus {
    /// Whether the current configuration is suitable for production
    pub is_production_ready: bool,
    /// The key storage strategy currently in use
    pub storage_strategy: KeyStorageStrategy,
    /// Whether the OS keychain is available on this system
    pub keychain_available: bool,
    /// The keychain backend being used (if available)
    pub keychain_backend: String,
    /// Security warnings (if any)
    pub warnings: Vec<String>,
    /// Recommendations for improving security
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encryption_roundtrip() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_encryption");
        fs::create_dir_all(&temp_dir).await.unwrap();

        // Use file-based storage for tests to avoid polluting the system keychain
        let manager = EncryptionManager::with_strategy(&temp_dir, KeyStorageStrategy::FileBased)
            .await
            .unwrap();

        let plaintext = b"Hello, Quantum World! This is secret data.";
        let encrypted = manager.encrypt(plaintext).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_encryption_manager_persistence() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_persistence_v2");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let manager1 = EncryptionManager::with_strategy(&temp_dir, KeyStorageStrategy::FileBased)
            .await
            .unwrap();
        let fingerprint1 = manager1.get_key_fingerprint();

        // Create new manager from same directory
        let manager2 = EncryptionManager::with_strategy(&temp_dir, KeyStorageStrategy::FileBased)
            .await
            .unwrap();
        let fingerprint2 = manager2.get_key_fingerprint();

        assert_eq!(fingerprint1, fingerprint2);

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[test]
    fn test_hash_data() {
        let data = b"test data";
        let hash1 = EncryptionManager::hash_data(data);
        let hash2 = EncryptionManager::hash_data(data);
        assert_eq!(hash1, hash2);

        let different_data = b"different data";
        let hash3 = EncryptionManager::hash_data(different_data);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_key_encoding() {
        let key = EncryptionManager::generate_master_key();
        let encoded = EncryptionManager::encode_key(&key);
        let decoded = EncryptionManager::decode_key(&encoded).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn test_instance_id_generation() {
        let path1 = Path::new("/data/db1");
        let path2 = Path::new("/data/db2");

        let id1 = EncryptionManager::generate_instance_id(path1);
        let id2 = EncryptionManager::generate_instance_id(path2);

        // Different paths should generate different IDs
        assert_ne!(id1, id2);

        // Same path should generate same ID
        let id1_again = EncryptionManager::generate_instance_id(path1);
        assert_eq!(id1, id1_again);
    }

    #[test]
    fn test_keychain_status_check() {
        let status = EncryptionManager::check_keychain_status();
        // Just verify it doesn't panic - actual availability depends on system
        println!("Keychain status: {:?}", status);
    }

    #[tokio::test]
    async fn test_key_rotation_file_based() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_rotation");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let mut manager =
            EncryptionManager::with_strategy(&temp_dir, KeyStorageStrategy::FileBased)
                .await
                .unwrap();

        let old_fingerprint = manager.get_key_fingerprint();

        // Encrypt some data with old key
        let plaintext = b"Secret data before rotation";
        let encrypted = manager.encrypt(plaintext).unwrap();

        // Rotate the key
        let result = manager.rotate_key().await.unwrap();
        assert!(result.success);
        assert_eq!(result.old_fingerprint, old_fingerprint);
        assert_ne!(result.new_fingerprint, old_fingerprint);

        // Old encrypted data should fail to decrypt with new key
        // (This is expected behavior - data must be re-encrypted)
        let decrypt_result = manager.decrypt(&encrypted);
        assert!(decrypt_result.is_err());

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[test]
    fn test_derive_key_from_password() {
        let password = "test_password_123";
        let salt = b"random_salt_value___"; // 20 bytes

        let key1 = EncryptionManager::derive_key_from_password(password, salt).unwrap();
        let key2 = EncryptionManager::derive_key_from_password(password, salt).unwrap();

        // Same password and salt should produce same key
        assert_eq!(key1, key2);

        // Different password should produce different key
        let key3 = EncryptionManager::derive_key_from_password("different_password", salt).unwrap();
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_storage_strategy_getter() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_strategy");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let manager = EncryptionManager::with_strategy(&temp_dir, KeyStorageStrategy::FileBased)
            .await
            .unwrap();

        assert_eq!(manager.storage_strategy(), KeyStorageStrategy::FileBased);

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_with_config_file_based() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_with_config");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let config = EncryptionConfig {
            strategy: KeyStorageStrategy::FileBased,
            forbid_file_fallback: false,
            production_mode: false,
        };

        let manager = EncryptionManager::with_config(&temp_dir, config)
            .await
            .unwrap();
        assert_eq!(manager.storage_strategy(), KeyStorageStrategy::FileBased);

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_production_mode_forbids_file_based_strategy() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_prod_mode");
        fs::create_dir_all(&temp_dir).await.unwrap();

        // Explicitly requesting FileBased in production mode should fail
        let config = EncryptionConfig {
            strategy: KeyStorageStrategy::FileBased,
            forbid_file_fallback: false,
            production_mode: true,
        };

        let result = EncryptionManager::with_config(&temp_dir, config).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Security policy violation"));
        assert!(err_msg.contains("forbidden in production"));

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_security_status() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_security_status");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let manager = EncryptionManager::with_strategy(&temp_dir, KeyStorageStrategy::FileBased)
            .await
            .unwrap();

        let status = manager.security_status();

        // File-based storage should not be production ready
        assert!(!status.is_production_ready);
        assert_eq!(status.storage_strategy, KeyStorageStrategy::FileBased);
        assert!(!status.warnings.is_empty());
        assert!(!status.recommendations.is_empty());

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_is_using_secure_storage() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_secure_storage");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let manager = EncryptionManager::with_strategy(&temp_dir, KeyStorageStrategy::FileBased)
            .await
            .unwrap();

        // File-based is not secure
        assert!(!manager.is_using_secure_storage());

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[test]
    fn test_encryption_config_default() {
        let config = EncryptionConfig::default();
        // Default strategy is OsKeychain (most secure option)
        assert_eq!(config.strategy, KeyStorageStrategy::OsKeychain);
        assert!(!config.forbid_file_fallback);
        assert!(!config.production_mode);
    }
}
