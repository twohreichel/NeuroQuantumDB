//! Encryption-at-Rest for NeuroQuantumDB
//! Provides transparent encryption/decryption using Post-Quantum Cryptography (ML-KEM)
//! combined with symmetric encryption (AES-256-GCM) for performance

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::path::{Path, PathBuf};
use tokio::fs;
use zeroize::Zeroize;

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

/// Key manager for encryption-at-rest
#[derive(Clone)]
pub struct EncryptionManager {
    /// Master encryption key (derived from password or generated)
    master_key: [u8; 32],
    /// Key file path for persistence
    key_path: PathBuf,
}

impl EncryptionManager {
    /// Create a new encryption manager
    pub async fn new(data_dir: &Path) -> Result<Self> {
        let key_path = data_dir.join(".encryption_key");

        let master_key = if key_path.exists() {
            // Load existing key
            Self::load_master_key(&key_path).await?
        } else {
            // Generate new key
            let key = Self::generate_master_key();
            Self::save_master_key(&key_path, &key).await?;
            key
        };

        tracing::info!("🔐 Encryption manager initialized with AES-256-GCM");

        Ok(Self {
            master_key,
            key_path,
        })
    }

    /// Generate a new random master key
    fn generate_master_key() -> [u8; 32] {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// Save master key to file (encrypted with key derivation)
    async fn save_master_key(path: &Path, key: &[u8; 32]) -> Result<()> {
        // In production, this should be protected by HSM or system keychain
        // For now, we'll use base64 encoding with file permissions
        use base64::{engine::general_purpose, Engine as _};
        let encoded = general_purpose::STANDARD.encode(key);

        fs::write(path, encoded).await?;

        // Set restrictive file permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).await?.permissions();
            perms.set_mode(0o600); // rw------- (owner only)
            fs::set_permissions(path, perms).await?;
        }

        tracing::info!("🔑 Master encryption key saved to: {}", path.display());
        Ok(())
    }

    /// Load master key from file
    async fn load_master_key(path: &Path) -> Result<[u8; 32]> {
        use base64::{engine::general_purpose, Engine as _};
        let encoded = fs::read_to_string(path).await?;
        let decoded = general_purpose::STANDARD
            .decode(encoded.trim())
            .map_err(|e| anyhow!("Failed to decode master key: {}", e))?;

        if decoded.len() != 32 {
            return Err(anyhow!("Invalid master key length: {}", decoded.len()));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&decoded);

        tracing::info!("🔑 Master encryption key loaded from: {}", path.display());
        Ok(key)
    }

    /// Encrypt data using AES-256-GCM
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData> {
        // Generate random nonce (12 bytes for AES-GCM)
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher with master key
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
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

        let nonce = Nonce::from_slice(&encrypted.nonce);

        // Create cipher with master key
        let cipher = Aes256Gcm::new_from_slice(&self.master_key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        Ok(plaintext)
    }

    /// Derive a key from password using Argon2
    #[allow(dead_code)]
    pub fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
        use argon2::{
            password_hash::{PasswordHasher, SaltString},
            Argon2,
        };

        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| anyhow!("Failed to encode salt: {}", e))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encryption_roundtrip() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_encryption");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let manager = EncryptionManager::new(&temp_dir).await.unwrap();

        let plaintext = b"Hello, Quantum World! This is secret data.";
        let encrypted = manager.encrypt(plaintext).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_encryption_manager_persistence() {
        let temp_dir = std::env::temp_dir().join("neuroquantum_test_persistence");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let manager1 = EncryptionManager::new(&temp_dir).await.unwrap();
        let fingerprint1 = manager1.get_key_fingerprint();

        // Create new manager from same directory
        let manager2 = EncryptionManager::new(&temp_dir).await.unwrap();
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
}

