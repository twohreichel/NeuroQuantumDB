//! Tests for API configuration
//!
//! These tests validate configuration defaults, development/production modes,
//! validation, and URL generation.

use neuroquantum_api::config::{ApiConfig, TlsConfig};

#[test]
fn test_default_config() {
    let config = ApiConfig::default();
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert!(config.rate_limit.enabled);
    assert!(config.monitoring.metrics_enabled);
}

#[test]
fn test_development_config() {
    let config = ApiConfig::development();
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.logging.level, "debug");
    assert!(!config.jwt.quantum_enabled);
}

#[test]
fn test_production_config() {
    let config = ApiConfig::production();
    assert_eq!(config.server.port, 443);
    assert_eq!(config.logging.level, "info");
    assert!(config.jwt.quantum_enabled);
    assert!(config.security.quantum_encryption);
    assert!(config.server.tls.is_some());
}

#[test]
fn test_config_validation() {
    let mut config = ApiConfig::default();

    // Test invalid JWT secret
    config.jwt.secret = "short".to_string();
    assert!(config.validate().is_err());

    // Test valid config
    config.jwt.secret = "this-is-a-valid-32-character-secret!".to_string();
    assert!(config.validate().is_ok());
}

#[test]
fn test_bind_address() {
    let config = ApiConfig::default();
    assert_eq!(config.bind_address(), "127.0.0.1:8080");
}

#[test]
fn test_base_url() {
    let config = ApiConfig::default();
    assert_eq!(config.base_url(), "http://127.0.0.1:8080");

    let mut tls_config = ApiConfig::default();
    tls_config.server.tls = Some(TlsConfig {
        cert_file: "test.crt".to_string(),
        key_file: "test.key".to_string(),
        ca_file: None,
    });
    assert_eq!(tls_config.base_url(), "https://127.0.0.1:8080");
}
