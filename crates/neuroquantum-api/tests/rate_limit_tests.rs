//! Tests for rate limiting service
//!
//! These tests validate memory-based rate limiting, rate limit resets,
//! and key isolation.

use neuroquantum_api::rate_limit::{RateLimitConfig, RateLimitService};

#[tokio::test]
async fn test_memory_rate_limiting() {
    let config = RateLimitConfig {
        requests_per_window: 5,
        window_size_seconds: 60,
        burst_allowance: Some(2),
        redis_url: None,
        fallback_to_memory: true,
    };

    let service = RateLimitService::new(config).await.unwrap();

    // Test normal operation
    for i in 0..5 {
        let result = service.check_rate_limit("test_user").await.unwrap();
        assert!(result.allowed, "Request {} should be allowed", i + 1);
    }

    // Test rate limit exceeded
    let result = service.check_rate_limit("test_user").await.unwrap();
    assert!(!result.allowed, "Request should be rate limited");
}

#[tokio::test]
async fn test_rate_limit_reset() {
    let config = RateLimitConfig {
        requests_per_window: 2,
        window_size_seconds: 60,
        burst_allowance: None,
        redis_url: None,
        fallback_to_memory: true,
    };

    let service = RateLimitService::new(config).await.unwrap();

    // Consume all tokens
    service.check_rate_limit("test_reset").await.unwrap();
    service.check_rate_limit("test_reset").await.unwrap();

    let result = service.check_rate_limit("test_reset").await.unwrap();
    assert!(!result.allowed);

    // Reset and try again
    service.reset_rate_limit("test_reset").await.unwrap();
    let result = service.check_rate_limit("test_reset").await.unwrap();
    assert!(result.allowed);
}

#[tokio::test]
async fn test_different_keys() {
    let config = RateLimitConfig::default();
    let service = RateLimitService::new(config).await.unwrap();

    // Different keys should have independent limits
    let result1 = service.check_rate_limit("user1").await.unwrap();
    let result2 = service.check_rate_limit("user2").await.unwrap();

    assert!(result1.allowed);
    assert!(result2.allowed);
    assert_eq!(result1.remaining, result2.remaining);
}
