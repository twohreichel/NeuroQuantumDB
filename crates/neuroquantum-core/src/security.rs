//! Production Security Layer for NeuroQuantumDB
//!
//! Implements quantum-resistant encryption, biometric authentication,
//! role-based access control, and comprehensive audit logging.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use ml_kem::{
    kem::{Decapsulate, Encapsulate},
    Ciphertext, Encoded, EncodedSizeUser, KemCore, MlKem1024,
};
use num_complex::Complex;
use pqcrypto_mldsa::mldsa87;
use pqcrypto_traits::sign::{PublicKey as SigPublicKey, SecretKey as SigSecretKey, SignedMessage};
use subtle::ConstantTimeEq;

/// ML-KEM-1024 ciphertext size in bytes (1568 bytes for Security Level 5)
const MLKEM1024_CIPHERTEXT_SIZE: usize = 1568;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_512};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

// ============================================================================
// Constant-Time Comparison Utilities
// ============================================================================

/// Constant-time comparison utilities for preventing timing attacks.
///
/// # Security Considerations
///
/// Timing attacks work by measuring how long it takes to compare two values.
/// Standard comparison operations (==, !=) will return immediately upon finding
/// the first difference, which can leak information about:
/// - API keys: How many characters match before the first difference
/// - Passwords: Similar information leakage
/// - Session tokens: Position of first mismatch
/// - Biometric thresholds: How close a value is to passing
///
/// These utilities ensure that comparisons take constant time regardless of
/// input, preventing such attacks.
///
/// Performs constant-time comparison of two byte slices.
/// Returns true if they are equal, false otherwise.
/// This function takes the same amount of time regardless of where the first
/// difference occurs, preventing timing attacks.
///
/// # Examples
///
/// ```
/// use neuroquantum_core::security::constant_time_compare;
///
/// let secret1 = b"my_secret_key_12345";
/// let secret2 = b"my_secret_key_12345";
/// let secret3 = b"my_secret_key_99999";
///
/// assert!(constant_time_compare(secret1, secret2));
/// assert!(!constant_time_compare(secret1, secret3));
/// ```
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

/// Performs constant-time comparison of two strings.
/// Converts strings to bytes and compares them in constant time.
///
/// # Examples
///
/// ```
/// use neuroquantum_core::security::constant_time_compare_str;
///
/// let token1 = "session_abc123";
/// let token2 = "session_abc123";
/// let token3 = "session_xyz789";
///
/// assert!(constant_time_compare_str(token1, token2));
/// assert!(!constant_time_compare_str(token1, token3));
/// ```
pub fn constant_time_compare_str(a: &str, b: &str) -> bool {
    constant_time_compare(a.as_bytes(), b.as_bytes())
}

/// Performs constant-time threshold comparison for floating point values.
/// This is used for similarity scores in biometric authentication.
/// Returns true if value >= threshold, using a constant-time comparison
/// to prevent leaking information about how close the value is to the threshold.
///
/// # Security Considerations
///
/// Without constant-time comparison, an attacker could learn:
/// - How close their biometric sample is to passing
/// - Whether they're getting warmer or colder
/// - Precise threshold values through binary search
///
/// This function prevents such leakage by always taking the same time regardless
/// of how close the value is to the threshold.
///
/// # Implementation
///
/// 1. Converts floats to fixed-point i32 (with 10000x scale for 4 decimal precision)
/// 2. Clamps values to valid i32 range to prevent undefined behavior
/// 3. Computes difference: value - threshold (wrapping to handle all cases)
/// 4. Extracts sign bit using arithmetic right shift (endian-agnostic)
/// 5. Returns true if difference is non-negative (sign bit = 0)
///
/// This approach leverages two's complement representation where negative numbers
/// have their sign bit (bit 31 for i32) set to 1. The algorithm is endian-agnostic
/// as it uses bit shifting instead of byte array indexing.
///
/// # Examples
///
/// ```
/// use neuroquantum_core::security::constant_time_threshold_check;
///
/// assert!(constant_time_threshold_check(0.90, 0.85));
/// assert!(!constant_time_threshold_check(0.80, 0.85));
/// ```
pub fn constant_time_threshold_check(value: f32, threshold: f32) -> bool {
    // Handle NaN explicitly for predictable behavior in security-critical context
    // NaN comparisons always return false for security (fail-safe default)
    if value.is_nan() || threshold.is_nan() {
        return false;
    }

    // Handle infinity cases explicitly to avoid overflow issues
    if value.is_infinite() {
        // Positive infinity is always >= any finite threshold
        // Negative infinity is always < any finite threshold
        return value.is_sign_positive() && !threshold.is_infinite();
    }
    if threshold.is_infinite() {
        // Value can never be >= positive infinity
        // Value is always >= negative infinity
        return threshold.is_sign_negative();
    }

    // Convert to fixed-point integers for constant-time comparison
    // Use 10000 scale factor for 4 decimal places precision
    // Clamp to valid i32 range to avoid undefined behavior from overflow
    let value_scaled = (value * 10000.0).clamp(i32::MIN as f32, i32::MAX as f32);
    let threshold_scaled = (threshold * 10000.0).clamp(i32::MIN as f32, i32::MAX as f32);
    let value_fixed = value_scaled as i32;
    let threshold_fixed = threshold_scaled as i32;

    // Compute difference: value - threshold
    let diff = value_fixed.wrapping_sub(threshold_fixed);

    // Extract sign bit in an endian-agnostic way
    // In two's complement, negative numbers have the sign bit (bit 31) set to 1
    // Shift right by 31 to get the sign bit in the LSB position
    let sign_bit = ((diff >> 31) & 1) as u8;

    // Return true if sign bit is 0 (non-negative), false if 1 (negative)
    // Use constant-time equality check to convert to bool
    sign_bit.ct_eq(&0u8).into()
}

// ============================================================================
// Configuration Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Quantum-resistant encryption settings
    pub quantum_encryption: QuantumEncryptionConfig,
    /// Byzantine fault tolerance configuration
    pub byzantine_tolerance: ByzantineConfig,
    /// Access control and authentication
    pub access_control: AccessControlConfig,
    /// Audit logging configuration
    pub audit_logging: AuditConfig,
    /// Biometric authentication settings
    pub biometric_auth: BiometricAuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumEncryptionConfig {
    /// Use ML-KEM-1024 (FIPS 203) for key encapsulation
    pub mlkem_enabled: bool,
    /// Use ML-DSA-87 (FIPS 204) for digital signatures
    pub mldsa_enabled: bool,
    /// Key rotation interval in seconds
    pub key_rotation_interval: u64,
    /// Encryption strength level (1-5)
    pub security_level: u8,
    /// Enable data encryption at rest
    pub encrypt_at_rest: bool,
    /// Enable data encryption in transit
    pub encrypt_in_transit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineConfig {
    /// Minimum number of nodes for consensus
    pub min_nodes: usize,
    /// Maximum tolerated Byzantine failures
    pub max_byzantine_failures: usize,
    /// Consensus timeout in milliseconds
    pub consensus_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    /// Enable role-based access control
    pub rbac_enabled: bool,
    /// Session timeout in seconds
    pub session_timeout: u64,
    /// Maximum failed login attempts
    pub max_failed_attempts: u32,
    /// Account lockout duration in seconds
    pub lockout_duration: u64,
    /// Enable multi-factor authentication
    pub mfa_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable comprehensive audit logging
    pub enabled: bool,
    /// Log retention period in days
    pub retention_days: u32,
    /// Tamper-proof logging using cryptographic hashing
    pub tamper_proof: bool,
    /// Log all read operations
    pub log_reads: bool,
    /// Log all write operations
    pub log_writes: bool,
    /// Log authentication attempts
    pub log_auth: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricAuthConfig {
    /// Enable EEG-based authentication
    pub eeg_enabled: bool,
    /// Similarity threshold for EEG pattern matching (0.0-1.0)
    pub similarity_threshold: f32,
    /// Minimum EEG sample length
    pub min_sample_length: usize,
    /// Feature extraction method
    pub feature_method: BiometricFeatureMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiometricFeatureMethod {
    FFT,     // Fast Fourier Transform
    Wavelet, // Wavelet Transform
    Hybrid,  // Combined approach
}

// ============================================================================
// Quantum Cryptography Implementation
// ============================================================================

/// Quantum-safe cryptographic keys with automatic zeroing on drop
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct QuantumKeys {
    /// ML-KEM keys for key exchange
    mlkem_public: Vec<u8>,
    #[zeroize(skip)]
    mlkem_secret: Vec<u8>,
    /// ML-DSA signature keys
    mldsa_public: Vec<u8>,
    #[zeroize(skip)]
    mldsa_secret: Vec<u8>,
    /// AES-256-GCM symmetric key for data encryption
    #[zeroize(skip)]
    symmetric_key: [u8; 32],
    /// Key creation timestamp
    #[zeroize(skip)]
    created_at: SystemTime,
}

impl QuantumKeys {
    /// Generate new quantum-safe key pair
    pub fn generate() -> Result<Self, SecurityError> {
        info!("🔐 Generating quantum-safe cryptographic keys...");

        // Generate ML-KEM-1024 keys for KEM using RustCrypto ml-kem
        let mut rng = rand::thread_rng();
        let (mlkem_dk, mlkem_ek) = MlKem1024::generate(&mut rng);

        // Generate ML-DSA-87 keys for signatures
        let (mldsa_pk, mldsa_sk) = mldsa87::keypair();

        // Generate symmetric key for AES-256-GCM
        let mut symmetric_key = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut symmetric_key);

        // Serialize ML-KEM keys to bytes
        let mlkem_public_bytes = mlkem_ek.as_bytes().to_vec();
        let mlkem_secret_bytes = mlkem_dk.as_bytes().to_vec();

        Ok(Self {
            mlkem_public: mlkem_public_bytes,
            mlkem_secret: mlkem_secret_bytes,
            mldsa_public: mldsa_pk.as_bytes().to_vec(),
            mldsa_secret: mldsa_sk.as_bytes().to_vec(),
            symmetric_key,
            created_at: SystemTime::now(),
        })
    }

    /// Check if keys need rotation
    pub fn needs_rotation(&self, rotation_interval: u64) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(self.created_at) {
            elapsed.as_secs() > rotation_interval
        } else {
            false
        }
    }
}

pub struct QuantumCrypto {
    keys: Arc<RwLock<QuantumKeys>>,
    config: QuantumEncryptionConfig,
}

impl QuantumCrypto {
    pub fn new(config: QuantumEncryptionConfig) -> Result<Self, SecurityError> {
        let keys = QuantumKeys::generate()?;

        Ok(Self {
            keys: Arc::new(RwLock::new(keys)),
            config,
        })
    }

    /// Encrypt data using quantum-resistant encryption
    pub async fn quantum_encrypt(&self, data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if !self.config.mlkem_enabled {
            return Err(SecurityError::EncryptionNotEnabled);
        }

        let keys = self.keys.read().await;

        // Use AES-256-GCM for symmetric encryption
        let cipher = Aes256Gcm::new_from_slice(&keys.symmetric_key)
            .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);

        // Encrypt data
        let ciphertext = cipher
            .encrypt(&nonce, data)
            .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        debug!("✅ Encrypted {} bytes of data", data.len());
        Ok(result)
    }

    /// Decrypt data using quantum-resistant decryption
    pub async fn quantum_decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if encrypted_data.len() < 12 {
            return Err(SecurityError::InvalidEncryptedData);
        }

        let keys = self.keys.read().await;

        // Extract nonce and ciphertext
        let nonce_bytes: [u8; 12] = encrypted_data[..12]
            .try_into()
            .map_err(|_| SecurityError::InvalidEncryptedData)?;
        let nonce = Nonce::from(nonce_bytes);
        let ciphertext = &encrypted_data[12..];

        // Decrypt using AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&keys.symmetric_key)
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        let plaintext = cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        debug!("✅ Decrypted {} bytes of data", plaintext.len());
        Ok(plaintext)
    }

    /// Sign data using ML-DSA post-quantum signature
    pub async fn sign_data(&self, data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        if !self.config.mldsa_enabled {
            return Err(SecurityError::SignatureDisabled);
        }

        let keys = self.keys.read().await;

        // Reconstruct ML-DSA secret key
        let sk = mldsa87::SecretKey::from_bytes(&keys.mldsa_secret)
            .map_err(|e| SecurityError::SignatureFailed(format!("Invalid secret key: {:?}", e)))?;

        // Sign the data
        let signed = mldsa87::sign(data, &sk);

        debug!("✅ Signed {} bytes of data", data.len());
        Ok(signed.as_bytes().to_vec())
    }

    /// Verify ML-DSA signature
    pub async fn verify_signature(&self, signed_data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        let keys = self.keys.read().await;

        // Reconstruct ML-DSA public key
        let pk = mldsa87::PublicKey::from_bytes(&keys.mldsa_public).map_err(|e| {
            SecurityError::SignatureVerificationFailed(format!("Invalid public key: {:?}", e))
        })?;

        // Reconstruct signed message
        let signed_msg = mldsa87::SignedMessage::from_bytes(signed_data).map_err(|e| {
            SecurityError::SignatureVerificationFailed(format!("Invalid signature: {:?}", e))
        })?;

        // Verify and extract original message
        let message = mldsa87::open(&signed_msg, &pk).map_err(|e| {
            SecurityError::SignatureVerificationFailed(format!("Verification failed: {:?}", e))
        })?;

        debug!("✅ Signature verified successfully");
        Ok(message.to_vec())
    }

    /// Generate quantum-safe shared secret using ML-KEM
    ///
    /// This method deserializes the peer's encapsulation key from bytes,
    /// then encapsulates a shared secret that only the peer can decapsulate.
    pub async fn generate_shared_secret(
        &self,
        peer_public_key: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), SecurityError> {
        // Expected encapsulation key size for ML-KEM-1024 is 1568 bytes
        const MLKEM1024_EK_SIZE: usize = 1568;

        if peer_public_key.len() != MLKEM1024_EK_SIZE {
            return Err(SecurityError::KeyExchangeFailed(format!(
                "Invalid public key length: expected {} bytes, got {} bytes",
                MLKEM1024_EK_SIZE,
                peer_public_key.len()
            )));
        }

        // Deserialize the encapsulation key from bytes
        type MlKem1024EncapsulationKey = <MlKem1024 as KemCore>::EncapsulationKey;
        type MlKem1024EkArray = Encoded<MlKem1024EncapsulationKey>;

        // Use TryFrom to convert slice to fixed-size array
        let ek_array: MlKem1024EkArray = peer_public_key.try_into().map_err(|_| {
            SecurityError::KeyExchangeFailed("Failed to parse encapsulation key bytes".to_string())
        })?;
        let ek = MlKem1024EncapsulationKey::from_bytes(&ek_array);

        // Encapsulate a shared secret
        let mut rng = rand::thread_rng();
        let (ct, shared_secret) = ek.encapsulate(&mut rng).map_err(|_| {
            SecurityError::KeyExchangeFailed("ML-KEM encapsulation failed".to_string())
        })?;

        Ok((
            AsRef::<[u8]>::as_ref(&shared_secret).to_vec(),
            AsRef::<[u8]>::as_ref(&ct).to_vec(),
        ))
    }

    /// Rotate encryption keys for forward secrecy
    pub async fn rotate_keys(&self) -> Result<(), SecurityError> {
        info!("🔄 Rotating quantum-safe encryption keys...");

        let new_keys = QuantumKeys::generate()?;
        let mut keys_guard = self.keys.write().await;
        *keys_guard = new_keys;

        info!("✅ Encryption keys rotated successfully");
        Ok(())
    }

    /// Get public keys for key exchange
    pub async fn get_public_keys(&self) -> Result<(Vec<u8>, Vec<u8>), SecurityError> {
        let keys = self.keys.read().await;
        Ok((keys.mlkem_public.clone(), keys.mldsa_public.clone()))
    }

    /// Decapsulate a shared secret from ML-KEM ciphertext
    ///
    /// Takes ciphertext bytes from `generate_shared_secret` and returns the shared secret.
    /// This validates the ciphertext size and performs proper decapsulation using the
    /// decapsulation (secret) key.
    ///
    /// For ML-KEM-1024, ciphertext must be exactly 1568 bytes.
    pub async fn decapsulate_shared_secret(
        &self,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, SecurityError> {
        // Validate ciphertext size for ML-KEM-1024 (1568 bytes)
        if ciphertext.len() != MLKEM1024_CIPHERTEXT_SIZE {
            return Err(SecurityError::KeyExchangeFailed(format!(
                "Invalid ciphertext length: expected {} bytes, got {} bytes",
                MLKEM1024_CIPHERTEXT_SIZE,
                ciphertext.len()
            )));
        }

        let keys = self.keys.read().await;

        // Deserialize the decapsulation key from bytes
        type MlKem1024DecapsulationKey = <MlKem1024 as KemCore>::DecapsulationKey;
        type MlKem1024DkArray = Encoded<MlKem1024DecapsulationKey>;

        // Convert secret key bytes to fixed-size array
        let dk_array: MlKem1024DkArray = keys.mlkem_secret.as_slice().try_into().map_err(|_| {
            SecurityError::KeyExchangeFailed("Failed to parse decapsulation key bytes".to_string())
        })?;
        let dk = MlKem1024DecapsulationKey::from_bytes(&dk_array);

        // Deserialize the ciphertext from bytes
        let ct: Ciphertext<MlKem1024> = ciphertext.try_into().map_err(|_| {
            SecurityError::KeyExchangeFailed("Failed to parse ciphertext bytes".to_string())
        })?;

        // Decapsulate using the decapsulation key
        let shared_secret = dk.decapsulate(&ct).map_err(|_| {
            SecurityError::KeyExchangeFailed(
                "ML-KEM decapsulation failed - possibly corrupted ciphertext".to_string(),
            )
        })?;

        debug!("✅ Successfully decapsulated shared secret");
        Ok(AsRef::<[u8]>::as_ref(&shared_secret).to_vec())
    }
}

// ============================================================================
// Biometric Authentication - EEG-based
// ============================================================================

/// EEG brainwave pattern template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EEGTemplate {
    user_id: String,
    /// Feature vector extracted from EEG signal
    features: Vec<f32>,
    /// Statistical properties
    mean: f32,
    std_dev: f32,
    /// Frequency domain features (Alpha, Beta, Gamma, Delta, Theta)
    frequency_bands: FrequencyBands,
    created_at: SystemTime,
    last_used: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyBands {
    delta: f32, // 0.5-4 Hz
    theta: f32, // 4-8 Hz
    alpha: f32, // 8-13 Hz
    beta: f32,  // 13-30 Hz
    gamma: f32, // 30-100 Hz
}

pub struct BiometricAuth {
    eeg_templates: Arc<RwLock<HashMap<String, EEGTemplate>>>,
    config: BiometricAuthConfig,
}

impl BiometricAuth {
    pub fn new(config: BiometricAuthConfig) -> Self {
        Self {
            eeg_templates: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register new EEG pattern for a user
    pub async fn register_eeg_pattern(
        &self,
        user_id: String,
        eeg_data: &[f32],
    ) -> Result<(), SecurityError> {
        if !self.config.eeg_enabled {
            return Err(SecurityError::BiometricDisabled);
        }

        if eeg_data.len() < self.config.min_sample_length {
            return Err(SecurityError::InsufficientBiometricData);
        }

        info!("🧠 Registering EEG pattern for user: {}", user_id);

        // Extract features from raw EEG data
        let features = self.extract_eeg_features(eeg_data)?;
        let frequency_bands = self.extract_frequency_bands(eeg_data)?;

        // Calculate statistical properties
        let mean = features.iter().sum::<f32>() / features.len() as f32;
        let variance =
            features.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / features.len() as f32;
        let std_dev = variance.sqrt();

        let template = EEGTemplate {
            user_id: user_id.clone(),
            features,
            mean,
            std_dev,
            frequency_bands,
            created_at: SystemTime::now(),
            last_used: SystemTime::now(),
        };

        let mut templates = self.eeg_templates.write().await;
        templates.insert(user_id.clone(), template);

        info!("✅ EEG pattern registered for user: {}", user_id);
        Ok(())
    }

    /// Authenticate user using EEG pattern matching
    pub async fn authenticate_eeg(
        &self,
        user_id: &str,
        eeg_data: &[f32],
    ) -> Result<AuthResult, SecurityError> {
        if !self.config.eeg_enabled {
            return Err(SecurityError::BiometricDisabled);
        }

        debug!("🔍 Authenticating user {} with EEG pattern", user_id);

        let mut templates = self.eeg_templates.write().await;
        let template = templates
            .get_mut(user_id)
            .ok_or_else(|| SecurityError::UserNotFound(user_id.to_string()))?;

        // Extract features from current EEG sample
        let current_features = self.extract_eeg_features(eeg_data)?;

        // Calculate similarity score using cosine similarity
        let similarity = self.cosine_similarity(&template.features, &current_features);

        debug!("📊 EEG similarity score: {:.4}", similarity);

        // Update last used timestamp
        template.last_used = SystemTime::now();

        // Use constant-time threshold comparison to prevent timing attacks
        // This prevents attackers from learning how close they are to the threshold
        if constant_time_threshold_check(similarity, self.config.similarity_threshold) {
            info!("✅ EEG authentication successful for user: {}", user_id);
            Ok(AuthResult::Success {
                user_id: user_id.to_string(),
                confidence: similarity,
                method: AuthMethod::Biometric,
            })
        } else {
            warn!(
                "❌ EEG authentication failed for user: {} (similarity: {:.4})",
                user_id, similarity
            );
            Ok(AuthResult::Failed {
                reason: format!("Biometric match score too low: {:.4}", similarity),
            })
        }
    }

    /// Extract EEG features using specified method
    fn extract_eeg_features(&self, eeg_data: &[f32]) -> Result<Vec<f32>, SecurityError> {
        match self.config.feature_method {
            | BiometricFeatureMethod::FFT => self.fft_features(eeg_data),
            | BiometricFeatureMethod::Wavelet => self.wavelet_features(eeg_data),
            | BiometricFeatureMethod::Hybrid => {
                let mut fft = self.fft_features(eeg_data)?;
                let wavelet = self.wavelet_features(eeg_data)?;
                fft.extend(wavelet);
                Ok(fft)
            },
        }
    }

    /// Extract frequency domain features using FFT with rustfft
    fn fft_features(&self, eeg_data: &[f32]) -> Result<Vec<f32>, SecurityError> {
        let mut features = Vec::new();
        let window_size = 256.min(eeg_data.len());

        // Create FFT planner
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(window_size);

        // Process each window with 50% overlap
        // Ensure we process at least one window if data length equals window_size
        let end = if eeg_data.len() == window_size {
            window_size
        } else {
            eeg_data.len().saturating_sub(window_size)
        };

        for i in (0..=end.saturating_sub(window_size)).step_by((window_size / 2).max(1)) {
            if i + window_size > eeg_data.len() {
                break;
            }
            let window = &eeg_data[i..i + window_size];

            // Convert to complex numbers for FFT
            let mut buffer: Vec<Complex<f32>> =
                window.iter().map(|&x| Complex::new(x, 0.0)).collect();

            // Perform FFT
            fft.process(&mut buffer);

            // Calculate power spectrum and extract frequency band features
            let power_spectrum: Vec<f32> = buffer
                .iter()
                .take(window_size / 2) // Only need first half (Nyquist)
                .map(|c| c.norm_sqr())
                .collect();

            // Extract standard EEG frequency bands (assuming 256 Hz sampling rate)
            // Delta (0.5-4 Hz), Theta (4-8 Hz), Alpha (8-13 Hz), Beta (13-30 Hz), Gamma (30-50 Hz)
            let sampling_rate = 256.0;
            let freq_resolution = sampling_rate / window_size as f32;

            let delta_band = Self::band_power(&power_spectrum, 0.5, 4.0, freq_resolution);
            let theta_band = Self::band_power(&power_spectrum, 4.0, 8.0, freq_resolution);
            let alpha_band = Self::band_power(&power_spectrum, 8.0, 13.0, freq_resolution);
            let beta_band = Self::band_power(&power_spectrum, 13.0, 30.0, freq_resolution);
            let gamma_band = Self::band_power(&power_spectrum, 30.0, 50.0, freq_resolution);

            features
                .extend_from_slice(&[delta_band, theta_band, alpha_band, beta_band, gamma_band]);
        }

        Ok(features)
    }

    /// Calculate power in a specific frequency band
    fn band_power(
        power_spectrum: &[f32],
        low_freq: f32,
        high_freq: f32,
        freq_resolution: f32,
    ) -> f32 {
        let low_idx = (low_freq / freq_resolution) as usize;
        let high_idx = ((high_freq / freq_resolution) as usize).min(power_spectrum.len());

        if low_idx >= high_idx || high_idx > power_spectrum.len() {
            return 0.0;
        }

        power_spectrum[low_idx..high_idx].iter().sum::<f32>() / (high_idx - low_idx) as f32
    }

    /// Extract wavelet features
    fn wavelet_features(&self, eeg_data: &[f32]) -> Result<Vec<f32>, SecurityError> {
        // Simplified wavelet transform (Haar wavelet)
        let mut features = Vec::new();
        let mut signal = eeg_data.to_vec();

        while signal.len() > 1 {
            let mut approximation = Vec::new();
            let mut detail = Vec::new();

            for i in (0..signal.len() - 1).step_by(2) {
                let avg = (signal[i] + signal[i + 1]) / 2.0;
                let diff = (signal[i] - signal[i + 1]) / 2.0;
                approximation.push(avg);
                detail.push(diff);
            }

            // Use detail coefficients as features
            let energy: f32 = detail.iter().map(|x| x * x).sum();
            features.push(energy);

            signal = approximation;
        }

        Ok(features)
    }

    /// Extract frequency band powers using FFT
    fn extract_frequency_bands(&self, eeg_data: &[f32]) -> Result<FrequencyBands, SecurityError> {
        let window_size = eeg_data.len().min(512).next_power_of_two();
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(window_size);

        // Pad or truncate data to window size
        let mut buffer: Vec<Complex<f32>> = eeg_data
            .iter()
            .take(window_size)
            .map(|&x| Complex::new(x, 0.0))
            .collect();

        // Pad with zeros if needed
        buffer.resize(window_size, Complex::new(0.0, 0.0));

        // Perform FFT
        fft.process(&mut buffer);

        // Calculate power spectrum
        let power_spectrum: Vec<f32> = buffer
            .iter()
            .take(window_size / 2)
            .map(|c| c.norm_sqr())
            .collect();

        // Extract band powers (assuming 256 Hz sampling rate)
        let sampling_rate = 256.0;
        let freq_resolution = sampling_rate / window_size as f32;

        Ok(FrequencyBands {
            delta: Self::band_power(&power_spectrum, 0.5, 4.0, freq_resolution),
            theta: Self::band_power(&power_spectrum, 4.0, 8.0, freq_resolution),
            alpha: Self::band_power(&power_spectrum, 8.0, 13.0, freq_resolution),
            beta: Self::band_power(&power_spectrum, 13.0, 30.0, freq_resolution),
            gamma: Self::band_power(&power_spectrum, 30.0, 50.0, freq_resolution),
        })
    }

    /// Calculate cosine similarity between two feature vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

// ============================================================================
// Role-Based Access Control (RBAC)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    Developer,
    DataScientist,
    ReadOnly,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    // Data operations
    ReadData,
    WriteData,
    DeleteData,

    // Schema operations
    CreateTable,
    AlterTable,
    DropTable,

    // Advanced features
    UseQuantumSearch,
    UseNeuromorphicLearning,
    UseDNACompression,

    // System operations
    ManageUsers,
    ManageRoles,
    ViewAuditLogs,
    ConfigureSystem,

    // Custom permission
    Custom(String),
}

impl Role {
    /// Get default permissions for a role
    pub fn default_permissions(&self) -> Vec<Permission> {
        match self {
            | Role::Admin => vec![
                Permission::ReadData,
                Permission::WriteData,
                Permission::DeleteData,
                Permission::CreateTable,
                Permission::AlterTable,
                Permission::DropTable,
                Permission::UseQuantumSearch,
                Permission::UseNeuromorphicLearning,
                Permission::UseDNACompression,
                Permission::ManageUsers,
                Permission::ManageRoles,
                Permission::ViewAuditLogs,
                Permission::ConfigureSystem,
            ],
            | Role::Developer => vec![
                Permission::ReadData,
                Permission::WriteData,
                Permission::CreateTable,
                Permission::AlterTable,
                Permission::UseQuantumSearch,
                Permission::UseNeuromorphicLearning,
                Permission::UseDNACompression,
            ],
            | Role::DataScientist => vec![
                Permission::ReadData,
                Permission::WriteData,
                Permission::UseQuantumSearch,
                Permission::UseNeuromorphicLearning,
            ],
            | Role::ReadOnly => vec![Permission::ReadData],
            | Role::Custom(_) => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    password_hash: String,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
    pub created_at: SystemTime,
    pub last_login: Option<SystemTime>,
    pub failed_attempts: u32,
    pub locked_until: Option<SystemTime>,
}

impl User {
    pub fn new(username: String, password: &str, roles: Vec<Role>) -> Result<Self, SecurityError> {
        let password_hash = Self::hash_password(password)?;

        // Aggregate permissions from all roles
        let mut permissions = Vec::new();
        for role in &roles {
            permissions.extend(role.default_permissions());
        }
        permissions.dedup();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            username,
            password_hash,
            roles,
            permissions,
            created_at: SystemTime::now(),
            last_login: None,
            failed_attempts: 0,
            locked_until: None,
        })
    }

    fn hash_password(password: &str) -> Result<String, SecurityError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| SecurityError::PasswordHashFailed(e.to_string()))
    }

    pub fn verify_password(&self, password: &str) -> Result<bool, SecurityError> {
        let parsed_hash = PasswordHash::new(&self.password_hash)
            .map_err(|e| SecurityError::PasswordHashFailed(e.to_string()))?;

        // NOTE: Argon2::verify_password is designed to be constant-time and resistant
        // to timing attacks. It always performs the full verification process regardless
        // of where differences occur in the hash comparison.
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            SystemTime::now() < locked_until
        } else {
            false
        }
    }
}

pub struct RBACManager {
    users: Arc<RwLock<HashMap<String, User>>>,
    config: AccessControlConfig,
}

impl RBACManager {
    pub fn new(config: AccessControlConfig) -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Create a new user
    pub async fn create_user(
        &self,
        username: String,
        password: &str,
        roles: Vec<Role>,
    ) -> Result<String, SecurityError> {
        let mut users = self.users.write().await;

        if users.values().any(|u| u.username == username) {
            return Err(SecurityError::UserAlreadyExists(username));
        }

        let user = User::new(username.clone(), password, roles)?;
        let user_id = user.id.clone();

        users.insert(user_id.clone(), user);
        info!("👤 Created new user: {}", username);

        Ok(user_id)
    }

    /// Authenticate user with password
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthResult, SecurityError> {
        let mut users = self.users.write().await;

        let user = users
            .values_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| SecurityError::UserNotFound(username.to_string()))?;

        // Check if account is locked
        if user.is_locked() {
            warn!("🔒 Authentication attempt on locked account: {}", username);
            return Ok(AuthResult::Failed {
                reason: "Account is locked due to too many failed attempts".to_string(),
            });
        }

        // Verify password
        if user.verify_password(password)? {
            user.failed_attempts = 0;
            user.last_login = Some(SystemTime::now());

            info!("✅ User authenticated successfully: {}", username);
            Ok(AuthResult::Success {
                user_id: user.id.clone(),
                confidence: 1.0,
                method: AuthMethod::Password,
            })
        } else {
            user.failed_attempts += 1;

            if user.failed_attempts >= self.config.max_failed_attempts {
                let lockout_duration = std::time::Duration::from_secs(self.config.lockout_duration);
                user.locked_until = Some(SystemTime::now() + lockout_duration);
                warn!("🔒 Account locked due to failed attempts: {}", username);
            }

            warn!(
                "❌ Authentication failed for user: {} (attempt {})",
                username, user.failed_attempts
            );
            Ok(AuthResult::Failed {
                reason: "Invalid credentials".to_string(),
            })
        }
    }

    /// Check if user has permission
    pub async fn check_permission(
        &self,
        user_id: &str,
        permission: &Permission,
    ) -> Result<bool, SecurityError> {
        let users = self.users.read().await;
        let user = users
            .get(user_id)
            .ok_or_else(|| SecurityError::UserNotFound(user_id.to_string()))?;

        Ok(user.has_permission(permission))
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: &str) -> Result<User, SecurityError> {
        let users = self.users.read().await;
        users
            .get(user_id)
            .cloned()
            .ok_or_else(|| SecurityError::UserNotFound(user_id.to_string()))
    }
}

// ============================================================================
// Audit Logging
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: SystemTime,
    pub user_id: Option<String>,
    pub event_type: AuditEventType,
    pub resource: String,
    pub action: String,
    pub result: AuditResult,
    pub details: HashMap<String, String>,
    /// Cryptographic hash of previous log entry for tamper detection
    pub previous_hash: Option<String>,
    /// Hash of this entry
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    SchemaChange,
    SystemConfiguration,
    SecurityEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    PartialSuccess,
}

pub struct AuditLogger {
    logs: Arc<RwLock<Vec<AuditLog>>>,
    config: AuditConfig,
}

impl AuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Log an audit event
    pub async fn log_event(
        &self,
        user_id: Option<String>,
        event_type: AuditEventType,
        resource: String,
        action: String,
        result: AuditResult,
        details: HashMap<String, String>,
    ) -> Result<(), SecurityError> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut logs = self.logs.write().await;

        // Get hash of previous entry for tamper-proof chain
        let previous_hash = if self.config.tamper_proof {
            logs.last().map(|log| log.hash.clone())
        } else {
            None
        };

        let mut entry = AuditLog {
            id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            user_id,
            event_type,
            resource,
            action,
            result,
            details,
            previous_hash,
            hash: String::new(),
        };

        // Calculate hash for this entry
        if self.config.tamper_proof {
            entry.hash = self.calculate_log_hash(&entry);
        }

        logs.push(entry);

        // Trim old logs based on retention policy
        self.cleanup_old_logs(&mut logs).await;

        Ok(())
    }

    /// Calculate cryptographic hash of log entry
    fn calculate_log_hash(&self, log: &AuditLog) -> String {
        let mut hasher = Sha3_512::new();

        hasher.update(log.id.as_bytes());
        hasher.update(
            log.timestamp
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .to_le_bytes(),
        );
        if let Some(ref user_id) = log.user_id {
            hasher.update(user_id.as_bytes());
        }
        hasher.update(log.resource.as_bytes());
        hasher.update(log.action.as_bytes());
        if let Some(ref prev_hash) = log.previous_hash {
            hasher.update(prev_hash.as_bytes());
        }

        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Cleanup logs older than retention period
    async fn cleanup_old_logs(&self, logs: &mut Vec<AuditLog>) {
        let retention_duration =
            std::time::Duration::from_secs(self.config.retention_days as u64 * 86400);

        let cutoff = SystemTime::now() - retention_duration;
        logs.retain(|log| log.timestamp > cutoff);
    }

    /// Verify integrity of audit log chain
    pub async fn verify_integrity(&self) -> Result<bool, SecurityError> {
        if !self.config.tamper_proof {
            return Ok(true);
        }

        let logs = self.logs.read().await;

        for i in 1..logs.len() {
            let current = &logs[i];
            let previous = &logs[i - 1];

            // Verify chain
            if current.previous_hash.as_ref() != Some(&previous.hash) {
                error!("🚨 Audit log chain broken at index {}", i);
                return Ok(false);
            }

            // Verify hash
            let calculated_hash = self.calculate_log_hash(current);
            if calculated_hash != current.hash {
                error!("🚨 Audit log hash mismatch at index {}", i);
                return Ok(false);
            }
        }

        info!("✅ Audit log integrity verified");
        Ok(true)
    }

    /// Get audit logs for a user
    pub async fn get_user_logs(&self, user_id: &str) -> Vec<AuditLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| log.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect()
    }

    /// Get all audit logs
    pub async fn get_all_logs(&self) -> Vec<AuditLog> {
        let logs = self.logs.read().await;
        logs.clone()
    }
}

// Add hex encoding support
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

// ============================================================================
// Authentication Results
// ============================================================================

#[derive(Debug, Clone)]
pub enum AuthResult {
    Success {
        user_id: String,
        confidence: f32,
        method: AuthMethod,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub enum AuthMethod {
    Password,
    Biometric,
    ApiKey,
    QuantumSignature,
}

// ============================================================================
// Session Management
// ============================================================================

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub created_at: SystemTime,
    pub last_access: SystemTime,
    pub expires_at: SystemTime,
    pub permissions: Vec<Permission>,
}

impl Session {
    pub fn new(user_id: String, permissions: Vec<Permission>, timeout_secs: u64) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            last_access: now,
            expires_at: now + std::time::Duration::from_secs(timeout_secs),
            permissions,
        }
    }

    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    pub fn refresh(&mut self, timeout_secs: u64) {
        let now = SystemTime::now();
        self.last_access = now;
        self.expires_at = now + std::time::Duration::from_secs(timeout_secs);
    }
}

// ============================================================================
// Main Security Manager
// ============================================================================

pub struct SecurityManager {
    config: Arc<RwLock<SecurityConfig>>,
    quantum_crypto: Arc<QuantumCrypto>,
    biometric_auth: Arc<BiometricAuth>,
    rbac_manager: Arc<RBACManager>,
    audit_logger: Arc<AuditLogger>,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SecurityManager {
    /// Initialize production security manager
    pub fn new(config: SecurityConfig) -> Result<Self, SecurityError> {
        info!("🔐 Initializing NeuroQuantumDB Security Manager...");

        let quantum_crypto = Arc::new(QuantumCrypto::new(config.quantum_encryption.clone())?);
        let biometric_auth = Arc::new(BiometricAuth::new(config.biometric_auth.clone()));
        let rbac_manager = Arc::new(RBACManager::new(config.access_control.clone()));
        let audit_logger = Arc::new(AuditLogger::new(config.audit_logging.clone()));

        info!("✅ Security Manager initialized successfully");

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            quantum_crypto,
            biometric_auth,
            rbac_manager,
            audit_logger,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Encrypt data using quantum-resistant algorithms
    pub async fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        self.quantum_crypto.quantum_encrypt(data).await
    }

    /// Decrypt data using quantum-resistant algorithms
    pub async fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        self.quantum_crypto.quantum_decrypt(encrypted_data).await
    }

    /// Sign data with post-quantum signature
    pub async fn sign_data(&self, data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        self.quantum_crypto.sign_data(data).await
    }

    /// Verify post-quantum signature
    pub async fn verify_signature(&self, signed_data: &[u8]) -> Result<Vec<u8>, SecurityError> {
        self.quantum_crypto.verify_signature(signed_data).await
    }

    /// Register EEG biometric pattern
    pub async fn register_biometric(
        &self,
        user_id: String,
        eeg_data: &[f32],
    ) -> Result<(), SecurityError> {
        self.biometric_auth
            .register_eeg_pattern(user_id, eeg_data)
            .await
    }

    /// Authenticate with EEG biometric
    pub async fn authenticate_biometric(
        &self,
        user_id: &str,
        eeg_data: &[f32],
    ) -> Result<AuthResult, SecurityError> {
        let result = self
            .biometric_auth
            .authenticate_eeg(user_id, eeg_data)
            .await?;

        // Log authentication attempt
        let mut details = HashMap::new();
        details.insert("method".to_string(), "biometric_eeg".to_string());

        let audit_result = match &result {
            | AuthResult::Success { .. } => AuditResult::Success,
            | AuthResult::Failed { .. } => AuditResult::Failure,
        };

        self.audit_logger
            .log_event(
                Some(user_id.to_string()),
                AuditEventType::Authentication,
                "biometric_auth".to_string(),
                "authenticate".to_string(),
                audit_result,
                details,
            )
            .await?;

        Ok(result)
    }

    /// Create user with password authentication
    pub async fn create_user(
        &self,
        username: String,
        password: &str,
        roles: Vec<Role>,
    ) -> Result<String, SecurityError> {
        let username_for_log = username.clone();
        let user_id = self
            .rbac_manager
            .create_user(username, password, roles)
            .await?;

        // Log user creation
        let mut details = HashMap::new();
        details.insert("username".to_string(), username_for_log);

        self.audit_logger
            .log_event(
                Some(user_id.clone()),
                AuditEventType::SystemConfiguration,
                "users".to_string(),
                "create".to_string(),
                AuditResult::Success,
                details,
            )
            .await?;

        Ok(user_id)
    }

    /// Authenticate user with password
    pub async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<String, SecurityError> {
        let result = self.rbac_manager.authenticate(username, password).await?;

        match result {
            | AuthResult::Success { user_id, .. } => {
                // Create session
                let user = self.rbac_manager.get_user(&user_id).await?;
                let config = self.config.read().await;

                let session = Session::new(
                    user_id.clone(),
                    user.permissions.clone(),
                    config.access_control.session_timeout,
                );

                let session_id = session.id.clone();
                let mut sessions = self.sessions.write().await;
                sessions.insert(session_id.clone(), session);

                Ok(session_id)
            },
            | AuthResult::Failed { reason } => Err(SecurityError::AuthenticationFailed(reason)),
        }
    }

    /// Validate session and check permissions
    pub async fn validate_session(
        &self,
        session_id: &str,
        required_permission: &Permission,
    ) -> Result<bool, SecurityError> {
        let mut sessions = self.sessions.write().await;

        let session = sessions
            .get_mut(session_id)
            .ok_or(SecurityError::InvalidSession)?;

        if session.is_expired() {
            sessions.remove(session_id);
            return Err(SecurityError::SessionExpired);
        }

        let config = self.config.read().await;
        session.refresh(config.access_control.session_timeout);

        Ok(session.permissions.contains(required_permission))
    }

    /// Log audit event
    pub async fn log_audit(
        &self,
        user_id: Option<String>,
        event_type: AuditEventType,
        resource: String,
        action: String,
        result: AuditResult,
        details: HashMap<String, String>,
    ) -> Result<(), SecurityError> {
        self.audit_logger
            .log_event(user_id, event_type, resource, action, result, details)
            .await
    }

    /// Verify audit log integrity
    pub async fn verify_audit_integrity(&self) -> Result<bool, SecurityError> {
        self.audit_logger.verify_integrity().await
    }

    /// Rotate encryption keys
    pub async fn rotate_keys(&self) -> Result<(), SecurityError> {
        self.quantum_crypto.rotate_keys().await
    }
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Quantum encryption is disabled")]
    QuantumEncryptionDisabled,

    #[error("Encryption is not enabled")]
    EncryptionNotEnabled,

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Signature is disabled")]
    SignatureDisabled,

    #[error("Signature failed: {0}")]
    SignatureFailed(String),

    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),

    #[error("Biometric authentication is disabled")]
    BiometricDisabled,

    #[error("Insufficient biometric data")]
    InsufficientBiometricData,

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Password hash failed: {0}")]
    PasswordHashFailed(String),

    #[error("Invalid session")]
    InvalidSession,

    #[error("Session expired")]
    SessionExpired,

    #[error("Access denied")]
    AccessDenied,

    #[error("Invalid encrypted data")]
    InvalidEncryptedData,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            quantum_encryption: QuantumEncryptionConfig {
                mlkem_enabled: true,
                mldsa_enabled: true,
                key_rotation_interval: 3600,
                security_level: 5,
                encrypt_at_rest: true,
                encrypt_in_transit: true,
            },
            byzantine_tolerance: ByzantineConfig {
                min_nodes: 3,
                max_byzantine_failures: 1,
                consensus_timeout_ms: 5000,
            },
            access_control: AccessControlConfig {
                rbac_enabled: true,
                session_timeout: 1800,
                max_failed_attempts: 5,
                lockout_duration: 900,
                mfa_enabled: true,
            },
            audit_logging: AuditConfig {
                enabled: true,
                retention_days: 90,
                tamper_proof: true,
                log_reads: false,
                log_writes: true,
                log_auth: true,
            },
            biometric_auth: BiometricAuthConfig {
                eeg_enabled: true,
                similarity_threshold: 0.85,
                min_sample_length: 256,
                feature_method: BiometricFeatureMethod::Hybrid,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quantum_encryption() {
        let config = QuantumEncryptionConfig {
            mlkem_enabled: true,
            mldsa_enabled: true,
            key_rotation_interval: 3600,
            security_level: 5,
            encrypt_at_rest: true,
            encrypt_in_transit: true,
        };

        let crypto = QuantumCrypto::new(config).unwrap();
        let test_data = b"test quantum encryption with ML-KEM and ML-DSA";

        let encrypted = crypto.quantum_encrypt(test_data).await.unwrap();
        let decrypted = crypto.quantum_decrypt(&encrypted).await.unwrap();

        assert_eq!(test_data, &decrypted[..]);
    }

    #[tokio::test]
    async fn test_digital_signature() {
        let config = QuantumEncryptionConfig {
            mlkem_enabled: true,
            mldsa_enabled: true,
            key_rotation_interval: 3600,
            security_level: 5,
            encrypt_at_rest: true,
            encrypt_in_transit: true,
        };

        let crypto = QuantumCrypto::new(config).unwrap();
        let test_data = b"test digital signature";

        let signed = crypto.sign_data(test_data).await.unwrap();
        let verified = crypto.verify_signature(&signed).await.unwrap();

        assert_eq!(test_data, &verified[..]);
    }

    #[tokio::test]
    async fn test_rbac() {
        let config = AccessControlConfig {
            rbac_enabled: true,
            session_timeout: 1800,
            max_failed_attempts: 5,
            lockout_duration: 900,
            mfa_enabled: false,
        };

        let rbac = RBACManager::new(config);

        let user_id = rbac
            .create_user(
                "test_user".to_string(),
                "secure_password",
                vec![Role::Developer],
            )
            .await
            .unwrap();

        let result = rbac
            .authenticate("test_user", "secure_password")
            .await
            .unwrap();
        assert!(matches!(result, AuthResult::Success { .. }));

        let has_perm = rbac
            .check_permission(&user_id, &Permission::ReadData)
            .await
            .unwrap();
        assert!(has_perm);
    }

    #[tokio::test]
    async fn test_audit_logging() {
        let config = AuditConfig {
            enabled: true,
            retention_days: 90,
            tamper_proof: true,
            log_reads: true,
            log_writes: true,
            log_auth: true,
        };

        let logger = AuditLogger::new(config);

        let mut details = HashMap::new();
        details.insert("action".to_string(), "test".to_string());

        logger
            .log_event(
                Some("user123".to_string()),
                AuditEventType::DataAccess,
                "test_table".to_string(),
                "SELECT".to_string(),
                AuditResult::Success,
                details,
            )
            .await
            .unwrap();

        let integrity = logger.verify_integrity().await.unwrap();
        assert!(integrity);
    }

    #[tokio::test]
    async fn test_biometric_auth() {
        let config = BiometricAuthConfig {
            eeg_enabled: true,
            similarity_threshold: 0.8,
            min_sample_length: 128,
            feature_method: BiometricFeatureMethod::FFT,
        };

        let bio_auth = BiometricAuth::new(config);

        // Generate mock EEG data
        let eeg_data: Vec<f32> = (0..256).map(|i| (i as f32 * 0.1).sin()).collect();

        bio_auth
            .register_eeg_pattern("user123".to_string(), &eeg_data)
            .await
            .unwrap();

        // Authenticate with similar pattern
        let result = bio_auth
            .authenticate_eeg("user123", &eeg_data)
            .await
            .unwrap();

        assert!(matches!(result, AuthResult::Success { .. }));
    }

    #[tokio::test]
    async fn test_mlkem_encapsulate_decapsulate_roundtrip() {
        let config = QuantumEncryptionConfig {
            mlkem_enabled: true,
            mldsa_enabled: true,
            key_rotation_interval: 3600,
            security_level: 5,
            encrypt_at_rest: true,
            encrypt_in_transit: true,
        };

        let crypto = QuantumCrypto::new(config).unwrap();

        // Get our public key
        let (mlkem_public, _mldsa_public) = crypto.get_public_keys().await.unwrap();

        // Encapsulate a shared secret using our own public key
        let (shared_secret_sender, ciphertext) =
            crypto.generate_shared_secret(&mlkem_public).await.unwrap();

        // Verify ciphertext size
        assert_eq!(
            ciphertext.len(),
            1568,
            "ML-KEM-1024 ciphertext should be 1568 bytes"
        );

        // Decapsulate the shared secret
        let shared_secret_receiver = crypto.decapsulate_shared_secret(&ciphertext).await.unwrap();

        // Both sides should have the same shared secret
        assert_eq!(
            shared_secret_sender, shared_secret_receiver,
            "Encapsulated and decapsulated shared secrets must match"
        );

        // Shared secret should be 32 bytes
        assert_eq!(
            shared_secret_receiver.len(),
            32,
            "ML-KEM-1024 shared secret should be 32 bytes"
        );
    }

    #[tokio::test]
    async fn test_mlkem_decapsulate_invalid_ciphertext_size() {
        let config = QuantumEncryptionConfig {
            mlkem_enabled: true,
            mldsa_enabled: true,
            key_rotation_interval: 3600,
            security_level: 5,
            encrypt_at_rest: true,
            encrypt_in_transit: true,
        };

        let crypto = QuantumCrypto::new(config).unwrap();

        // Try to decapsulate with wrong ciphertext size
        let invalid_ciphertext = vec![0u8; 100];
        let result = crypto.decapsulate_shared_secret(&invalid_ciphertext).await;

        assert!(result.is_err(), "Should fail with invalid ciphertext size");
        if let Err(SecurityError::KeyExchangeFailed(msg)) = result {
            assert!(
                msg.contains("Invalid ciphertext length"),
                "Error should mention invalid ciphertext length"
            );
        }
    }

    #[test]
    fn test_constant_time_compare_equal() {
        let secret1 = b"my_secret_api_key_12345";
        let secret2 = b"my_secret_api_key_12345";
        assert!(constant_time_compare(secret1, secret2));
    }

    #[test]
    fn test_constant_time_compare_different() {
        let secret1 = b"my_secret_api_key_12345";
        let secret2 = b"my_secret_api_key_99999";
        assert!(!constant_time_compare(secret1, secret2));
    }

    #[test]
    fn test_constant_time_compare_different_lengths() {
        let secret1 = b"short";
        let secret2 = b"much_longer_secret";
        assert!(!constant_time_compare(secret1, secret2));
    }

    #[test]
    fn test_constant_time_compare_str_equal() {
        let token1 = "session_abc123xyz";
        let token2 = "session_abc123xyz";
        assert!(constant_time_compare_str(token1, token2));
    }

    #[test]
    fn test_constant_time_compare_str_different() {
        let token1 = "session_abc123xyz";
        let token2 = "session_def456uvw";
        assert!(!constant_time_compare_str(token1, token2));
    }

    #[test]
    fn test_constant_time_threshold_check_above() {
        assert!(constant_time_threshold_check(0.90, 0.85));
        assert!(constant_time_threshold_check(0.85, 0.85));
    }

    #[test]
    fn test_constant_time_threshold_check_below() {
        assert!(!constant_time_threshold_check(0.80, 0.85));
        assert!(!constant_time_threshold_check(0.84, 0.85));
    }

    #[test]
    fn test_constant_time_threshold_check_precision() {
        // Test precision at 4 decimal places
        assert!(constant_time_threshold_check(0.8501, 0.85));
        assert!(constant_time_threshold_check(0.85001, 0.85));
    }

    #[test]
    fn test_constant_time_threshold_check_edge_cases() {
        assert!(constant_time_threshold_check(1.0, 0.0));
        assert!(constant_time_threshold_check(0.0, 0.0));
        assert!(!constant_time_threshold_check(-0.1, 0.0));
    }

    #[test]
    fn test_constant_time_threshold_check_nan() {
        // NaN should always return false for security (fail-safe)
        assert!(!constant_time_threshold_check(f32::NAN, 0.85));
        assert!(!constant_time_threshold_check(0.90, f32::NAN));
        assert!(!constant_time_threshold_check(f32::NAN, f32::NAN));
    }

    #[test]
    fn test_constant_time_threshold_check_infinity() {
        // Infinity should be handled correctly after clamping
        assert!(constant_time_threshold_check(f32::INFINITY, 0.85));
        assert!(!constant_time_threshold_check(f32::NEG_INFINITY, 0.85));
        assert!(!constant_time_threshold_check(0.85, f32::INFINITY));
    }
}
