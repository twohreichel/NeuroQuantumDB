/// Post-Quantum Cryptography utilities for NeuroQuantumDB
///
/// Implements NIST post-quantum standards:
/// - ML-KEM (Kyber) for key encapsulation (using RustCrypto ml-kem crate)
/// - ML-DSA (Dilithium) for digital signatures
use ml_kem::{
    kem::{Decapsulate, Encapsulate},
    Ciphertext, EncodedSizeUser, KemCore, MlKem768,
};
use pqcrypto_mldsa::mldsa65;
use pqcrypto_traits::sign::{PublicKey as SignPublicKey, SecretKey as SignSecretKey};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Type aliases for ML-KEM-768 (NIST Security Level 3)
type MlKemDecapsulationKey = <MlKem768 as KemCore>::DecapsulationKey;
type MlKemEncapsulationKey = <MlKem768 as KemCore>::EncapsulationKey;

/// ML-KEM-768 ciphertext size in bytes (1088 bytes)
const MLKEM768_CIPHERTEXT_SIZE: usize = 1088;
/// ML-KEM-768 shared secret size in bytes (32 bytes)
#[allow(dead_code)]
const MLKEM768_SHARED_SECRET_SIZE: usize = 32;

#[derive(Error, Debug)]
pub enum PQCryptoError {
    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid ciphertext: {0}")]
    InvalidCiphertext(String),

    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decapsulation failed: {0}")]
    DecapsulationFailed(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
}

/// Post-quantum cryptographic key manager
///
/// Uses ML-KEM-768 (RustCrypto implementation) for key encapsulation and
/// ML-DSA-65 (pqcrypto implementation) for digital signatures.
/// Both provide NIST Security Level 3.
#[derive(Clone)]
pub struct PQCryptoManager {
    // ML-KEM (Kyber) keys for key encapsulation using RustCrypto ml-kem
    mlkem_encapsulation_key: Arc<MlKemEncapsulationKey>,
    mlkem_decapsulation_key: Arc<MlKemDecapsulationKey>,

    // ML-DSA (Dilithium) keys for digital signatures
    mldsa_public_key: Arc<mldsa65::PublicKey>,
    mldsa_secret_key: Arc<mldsa65::SecretKey>,
}

/// Quantum-resistant token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumTokenClaims {
    pub user_id: String,
    pub session_id: String,
    pub timestamp: i64,
    pub quantum_signature: String, // Base64-encoded ML-DSA signature
    pub kem_ciphertext: String,    // Base64-encoded ML-KEM ciphertext
}

impl PQCryptoManager {
    /// Create a new post-quantum crypto manager with generated keys
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();

        // Generate ML-KEM-768 key pair (NIST Security Level 3) using RustCrypto
        let (dk, ek) = MlKem768::generate(&mut rng);

        // Generate ML-DSA-65 key pair (NIST Security Level 3)
        let (mldsa_pk, mldsa_sk) = mldsa65::keypair();

        // ML-KEM-768 key sizes are fixed by the standard
        // Encapsulation key: 1184 bytes, Decapsulation key: 2400 bytes
        tracing::info!(
            "🔐 Generated post-quantum key pairs: ML-KEM-768 (ek: 1184 bytes, dk: 2400 bytes), ML-DSA-65 (pk: {} bytes, sk: {} bytes)",
            mldsa_pk.as_bytes().len(),
            mldsa_sk.as_bytes().len()
        );

        Self {
            mlkem_encapsulation_key: Arc::new(ek),
            mlkem_decapsulation_key: Arc::new(dk),
            mldsa_public_key: Arc::new(mldsa_pk),
            mldsa_secret_key: Arc::new(mldsa_sk),
        }
    }

    /// Get the ML-KEM encapsulation (public) key as base64
    pub fn get_mlkem_public_key_base64(&self) -> String {
        let encoded = self.mlkem_encapsulation_key.as_bytes();
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, encoded)
    }

    /// Get the ML-DSA public key as base64
    pub fn get_mldsa_public_key_base64(&self) -> String {
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            self.mldsa_public_key.as_bytes(),
        )
    }

    /// Sign a message using ML-DSA (Dilithium)
    pub fn sign_message(&self, message: &[u8]) -> Vec<u8> {
        let signed_msg = mldsa65::sign(message, &self.mldsa_secret_key);
        // SignedMessage is a wrapper, convert to bytes using the trait
        use pqcrypto_traits::sign::SignedMessage;
        signed_msg.as_bytes().to_vec()
    }

    /// Verify a signature using ML-DSA (Dilithium)
    pub fn verify_signature(&self, signed_message_bytes: &[u8]) -> Result<Vec<u8>, PQCryptoError> {
        // Create SignedMessage from bytes using the trait
        use pqcrypto_traits::sign::SignedMessage;
        let signed_msg = mldsa65::SignedMessage::from_bytes(signed_message_bytes)
            .map_err(|_| PQCryptoError::SignatureVerificationFailed)?;

        // Open (verify) the signed message and extract original message
        let opened_msg = mldsa65::open(&signed_msg, &self.mldsa_public_key)
            .map_err(|_| PQCryptoError::SignatureVerificationFailed)?;

        Ok(opened_msg.to_vec())
    }

    /// Encapsulate a shared secret using ML-KEM (Kyber)
    /// Returns (ciphertext_bytes, shared_secret_bytes)
    ///
    /// The ciphertext can be transmitted to the key holder who can then
    /// decapsulate it using their decapsulation key to obtain the same shared secret.
    /// For ML-KEM-768, ciphertext is 1088 bytes and shared secret is 32 bytes.
    pub fn encapsulate(&self) -> (Vec<u8>, Vec<u8>) {
        let mut rng = rand::thread_rng();

        // Encapsulate using the encapsulation key
        let (ct, shared_secret) = self
            .mlkem_encapsulation_key
            .encapsulate(&mut rng)
            .expect("ML-KEM encapsulation should not fail with valid key");

        // Convert to bytes - Ciphertext and SharedKey are Array types that implement AsRef<[u8]>
        let ct_bytes: Vec<u8> = AsRef::<[u8]>::as_ref(&ct).to_vec();
        let ss_bytes: Vec<u8> = AsRef::<[u8]>::as_ref(&shared_secret).to_vec();

        (ct_bytes, ss_bytes)
    }

    /// Decapsulate a shared secret using ML-KEM (Kyber)
    ///
    /// Takes ciphertext bytes (as returned by encapsulate) and returns the shared secret.
    /// This correctly deserializes the ciphertext and performs proper decapsulation
    /// using the decapsulation (secret) key.
    pub fn decapsulate(&self, ciphertext_bytes: &[u8]) -> Result<Vec<u8>, PQCryptoError> {
        // Validate ciphertext size for ML-KEM-768 (1088 bytes)
        if ciphertext_bytes.len() != MLKEM768_CIPHERTEXT_SIZE {
            return Err(PQCryptoError::InvalidCiphertext(format!(
                "Invalid ciphertext length: expected {} bytes, got {} bytes",
                MLKEM768_CIPHERTEXT_SIZE,
                ciphertext_bytes.len()
            )));
        }

        // Deserialize the ciphertext from bytes using TryFrom
        // Ciphertext<MlKem768> is a hybrid_array::Array with fixed size 1088
        let ct: Ciphertext<MlKem768> = ciphertext_bytes.try_into().map_err(|_| {
            PQCryptoError::InvalidCiphertext("Failed to parse ciphertext bytes".to_string())
        })?;

        // Decapsulate using the decapsulation key
        let shared_secret = self.mlkem_decapsulation_key.decapsulate(&ct).map_err(|_| {
            PQCryptoError::DecapsulationFailed(
                "ML-KEM decapsulation failed - possibly corrupted ciphertext".to_string(),
            )
        })?;

        Ok(AsRef::<[u8]>::as_ref(&shared_secret).to_vec())
    }

    /// Generate quantum token claims with signatures
    pub fn generate_quantum_claims(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<QuantumTokenClaims, PQCryptoError> {
        let timestamp = chrono::Utc::now().timestamp();
        let message = format!("{}:{}:{}", user_id, session_id, timestamp);

        // Sign the message with ML-DSA
        let signature = self.sign_message(message.as_bytes());
        let quantum_signature =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &signature);

        // Generate KEM ciphertext
        let (ciphertext, _shared_secret) = self.encapsulate();
        let kem_ciphertext =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &ciphertext);

        Ok(QuantumTokenClaims {
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
            timestamp,
            quantum_signature,
            kem_ciphertext,
        })
    }

    /// Verify quantum token claims
    pub fn verify_quantum_claims(&self, claims: &QuantumTokenClaims) -> Result<(), PQCryptoError> {
        // Reconstruct the original message
        let message = format!(
            "{}:{}:{}",
            claims.user_id, claims.session_id, claims.timestamp
        );

        // Decode signature from base64
        let signature = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &claims.quantum_signature,
        )
        .map_err(|e| PQCryptoError::EncodingError(format!("Invalid base64: {}", e)))?;

        // Verify the signature
        let verified_msg = self.verify_signature(&signature)?;

        // Check if the verified message matches
        if verified_msg != message.as_bytes() {
            return Err(PQCryptoError::SignatureVerificationFailed);
        }

        Ok(())
    }
}

impl Default for PQCryptoManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mlkem_key_generation() {
        let manager = PQCryptoManager::new();
        let pk = manager.get_mlkem_public_key_base64();
        assert!(!pk.is_empty());
    }

    #[test]
    fn test_signature_sign_verify() {
        let manager = PQCryptoManager::new();
        let message = b"Hello, Quantum World!";

        let signed = manager.sign_message(message);
        let verified = manager.verify_signature(&signed).unwrap();

        assert_eq!(verified, message);
    }

    #[test]
    fn test_kem_encapsulate_decapsulate() {
        let manager = PQCryptoManager::new();

        // Test that encapsulation produces non-empty results
        let (ciphertext, shared_secret_sender) = manager.encapsulate();
        assert!(!ciphertext.is_empty(), "Ciphertext should not be empty");
        assert!(
            !shared_secret_sender.is_empty(),
            "Shared secret should not be empty"
        );

        // Expected sizes for ML-KEM-768
        assert_eq!(
            ciphertext.len(),
            1088,
            "ML-KEM-768 ciphertext should be 1088 bytes"
        );
        assert_eq!(
            shared_secret_sender.len(),
            32,
            "ML-KEM-768 shared secret should be 32 bytes"
        );

        // Test that multiple encapsulations produce different results (randomness)
        let (ciphertext2, _shared_secret2) = manager.encapsulate();
        assert_ne!(
            ciphertext, ciphertext2,
            "Ciphertexts should be different due to randomness"
        );

        // Test full decapsulation roundtrip - this now works correctly!
        let shared_secret_receiver = manager.decapsulate(&ciphertext).unwrap();
        assert_eq!(
            shared_secret_sender, shared_secret_receiver,
            "Decapsulated shared secret must match the encapsulated one"
        );
    }

    #[test]
    fn test_kem_decapsulation_with_invalid_ciphertext() {
        let manager = PQCryptoManager::new();

        // Test with wrong length
        let short_ciphertext = vec![0u8; 100];
        let result = manager.decapsulate(&short_ciphertext);
        assert!(
            result.is_err(),
            "Should fail with invalid ciphertext length"
        );

        // Test with correct length but invalid content (should still produce a result due to implicit rejection)
        // ML-KEM uses implicit rejection, meaning it produces a pseudorandom output for invalid ciphertexts
        // This is a security feature to prevent timing attacks
        let invalid_ciphertext = vec![0u8; 1088];
        let result = manager.decapsulate(&invalid_ciphertext);
        // Note: ML-KEM may succeed with implicit rejection (returns pseudorandom key)
        // or fail depending on the implementation
        if let Ok(secret) = result {
            // The secret should be 32 bytes even with implicit rejection
            assert_eq!(secret.len(), 32);
        }
    }

    #[test]
    fn test_kem_multiple_roundtrips() {
        let manager = PQCryptoManager::new();

        // Perform multiple encapsulation/decapsulation roundtrips
        for _ in 0..5 {
            let (ciphertext, expected_secret) = manager.encapsulate();
            let decapsulated_secret = manager.decapsulate(&ciphertext).unwrap();
            assert_eq!(
                expected_secret, decapsulated_secret,
                "Each roundtrip must produce matching secrets"
            );
        }
    }

    #[test]
    fn test_quantum_claims_generation_verification() {
        let manager = PQCryptoManager::new();

        let claims = manager
            .generate_quantum_claims("user123", "session456")
            .unwrap();
        assert_eq!(claims.user_id, "user123");
        assert_eq!(claims.session_id, "session456");

        // Verify the claims
        manager.verify_quantum_claims(&claims).unwrap();
    }

    #[test]
    fn test_invalid_signature_fails() {
        let manager = PQCryptoManager::new();
        let invalid_signed = vec![0u8; 100];

        let result = manager.verify_signature(&invalid_signed);
        assert!(result.is_err());
    }
}
