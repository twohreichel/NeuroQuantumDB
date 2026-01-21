//! Tests for JWT service and key rotation
//!
//! These tests validate JWT generation, validation, and key rotation functionality.

use std::sync::Arc;
use std::time::Duration;

use neuroquantum_api::jwt::{JwtConfig, JwtKeyRotation, JwtService};
use neuroquantum_api::permissions::{Permission, ADMIN, QUANTUM_AUTHENTICATED, READ};

#[tokio::test]
async fn test_jwt_generation_and_validation() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let service = JwtService::new(secret);

    let token = service
        .generate_token("test_user", Permission::read_write(), 128)
        .unwrap();

    let claims = service.validate_token(&token).await.unwrap();
    assert_eq!(claims.sub, "test_user");
    assert_eq!(claims.quantum_level, 128);
    assert!(claims.permissions.contains(&READ.to_string()));
}

#[tokio::test]
async fn test_quantum_token_generation() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let service = JwtService::new(secret);

    let token = service
        .generate_quantum_token("quantum_user", "session_123")
        .unwrap();
    let claims = service.validate_token(&token).await.unwrap();

    assert_eq!(claims.sub, "quantum_user");
    assert_eq!(claims.quantum_level, 255);
    assert!(claims
        .permissions
        .contains(&QUANTUM_AUTHENTICATED.to_string()));
}

#[tokio::test]
async fn test_invalid_token() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let service = JwtService::new(secret);

    let result = service.validate_token("invalid.token.here").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_jwt_key_rotation_creation() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let rotation_interval = Duration::from_secs(90 * 24 * 3600); // 90 days

    let rotation = JwtKeyRotation::new(secret, rotation_interval);

    // Should not need rotation immediately after creation
    assert!(!rotation.needs_rotation().await);

    // Current secret should match initial secret
    let current = rotation.current_secret().await;
    assert_eq!(current, secret);

    // Previous secret should be None
    assert!(rotation.previous_secret().await.is_none());
}

#[tokio::test]
async fn test_jwt_key_rotation_manual() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let rotation_interval = Duration::from_secs(1); // 1 second for testing

    let rotation = JwtKeyRotation::new(secret, rotation_interval);
    let initial_secret = rotation.current_secret().await;

    // Wait for rotation interval
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should need rotation now
    assert!(rotation.needs_rotation().await);

    // Perform rotation
    let rotated = rotation.rotate().await.unwrap();
    assert!(rotated);

    // Current secret should be different
    let new_secret = rotation.current_secret().await;
    assert_ne!(new_secret, initial_secret);

    // Previous secret should match initial
    let prev_secret = rotation.previous_secret().await;
    assert!(prev_secret.is_some());
    assert_eq!(prev_secret.unwrap(), initial_secret);
}

#[tokio::test]
async fn test_jwt_service_with_rotation() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let rotation_interval = Duration::from_secs(90 * 24 * 3600);

    let service = JwtService::with_rotation(secret, rotation_interval);

    // Verify rotation manager is set
    assert!(service.rotation_manager().is_some());

    // Generate a token
    let token = service
        .generate_token("rotation_user", Permission::to_owned(&[ADMIN]), 255)
        .unwrap();

    // Validate token
    let claims = service.validate_token(&token).await.unwrap();
    assert_eq!(claims.sub, "rotation_user");
}

#[tokio::test]
async fn test_jwt_validation_with_previous_key() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let rotation_interval = Duration::from_secs(1); // Short interval for testing
    let grace_period = Duration::from_secs(10); // 10 seconds grace

    let mut service = JwtService::new(secret);
    let rotation = JwtKeyRotation::with_grace_period(secret, rotation_interval, grace_period);
    service.set_key_rotation(Some(Arc::new(rotation)));

    // Generate token with initial key
    let token = service
        .generate_token("grace_user", vec!["test".to_string()], 128)
        .unwrap();

    // Token should validate
    assert!(service.validate_token(&token).await.is_ok());

    // Wait for rotation interval
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Rotate keys
    service.check_and_rotate().await.unwrap();

    // Token signed with old key should still validate (grace period)
    let claims = service.validate_token(&token).await.unwrap();
    assert_eq!(claims.sub, "grace_user");
}

#[tokio::test]
async fn test_jwt_config_with_rotation() {
    let config = JwtConfig {
        secret: "test-secret-key-minimum-32-chars!".to_string(),
        expiration_hours: 24,
        refresh_threshold_minutes: 60,
        quantum_enabled: false,
        algorithm: "HS256".to_string(),
        rotation_enabled: true,
        rotation_interval_days: 90,
        rotation_grace_period_hours: 24,
    };

    let service = config.into_service();

    // Verify rotation is enabled
    assert!(service.rotation_manager().is_some());
}

#[tokio::test]
async fn test_force_rotation() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let rotation_interval = Duration::from_secs(3600); // 1 hour

    let rotation = JwtKeyRotation::new(secret, rotation_interval);

    // Should not need rotation initially
    assert!(!rotation.needs_rotation().await);

    // Force rotation
    rotation.force_rotate().await.unwrap();

    // Secret should be different
    let new_secret = rotation.current_secret().await;
    assert_ne!(new_secret, secret);

    // Previous key should be None (force rotate doesn't keep it)
    assert!(rotation.previous_secret().await.is_none());
}

#[tokio::test]
async fn test_rotation_time_calculation() {
    let secret = b"test_secret_key_32_bytes_minimum!!";
    let rotation_interval = Duration::from_secs(3600); // 1 hour

    let rotation = JwtKeyRotation::new(secret, rotation_interval);

    let time_until = rotation.time_until_rotation().await.unwrap();

    // Should be close to 1 hour (allowing some tolerance)
    assert!(time_until.as_secs() >= 3595 && time_until.as_secs() <= 3600);
}
