//! Security Module Tests
//!
//! Tests for quantum cryptography, RBAC, biometric authentication, and audit logging.

use std::collections::HashMap;

use neuroquantum_core::security::{
    constant_time_compare, constant_time_compare_str, constant_time_threshold_check,
    AccessControlConfig, AuditConfig, AuditEventType, AuditLogger, AuditResult, AuthResult,
    BiometricAuth, BiometricAuthConfig, BiometricFeatureMethod, Permission, QuantumCrypto,
    QuantumEncryptionConfig, RBACManager, Role,
};

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
    if let Err(neuroquantum_core::security::SecurityError::KeyExchangeFailed(msg)) = result {
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
