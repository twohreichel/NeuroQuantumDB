//! Tests for middleware functions
//!
//! These tests validate public endpoint detection and circuit breaker behavior.

use std::time::Duration;

use neuroquantum_api::error::ApiError;
use neuroquantum_api::middleware::{is_public_endpoint, CircuitBreaker};

#[test]
fn test_is_public_endpoint() {
    assert!(is_public_endpoint("/health"));
    assert!(is_public_endpoint("/metrics"));
    assert!(is_public_endpoint("/api/v1/auth/login"));
    assert!(is_public_endpoint("/api-docs/"));
    assert!(!is_public_endpoint("/api/v1/tables"));
    assert!(!is_public_endpoint("/api/v1/neural/train"));
}

#[test]
fn test_circuit_breaker_states() {
    let cb = CircuitBreaker::new(3, 2, Duration::from_secs(30));

    // Test successful calls
    let result: Result<&str, ApiError> = cb.call_service("test", || Ok("success"));
    assert!(result.is_ok());

    // Test failure threshold
    for _ in 0..3 {
        let _: Result<&str, ApiError> = cb.call_service("test", || {
            Err(ApiError::ServiceUnavailable {
                service: "test".to_string(),
                reason: "test failure".to_string(),
            })
        });
    }

    // Circuit should be open now
    let result: Result<&str, ApiError> = cb.call_service("test", || Ok("should fail"));
    assert!(matches!(result, Err(ApiError::CircuitBreakerOpen { .. })));
}
