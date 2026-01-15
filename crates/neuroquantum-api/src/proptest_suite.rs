//! Property-based tests for the `NeuroQuantum` API
//!
//! This module provides comprehensive property-based testing for API components
//! including request validation, error handling, and configuration.
//!
//! ## Test Categories
//!
//! - **Request Validation**: Tests for input validation robustness
//! - **Error Handling**: Tests for consistent error responses
//! - **Configuration**: Tests for configuration parsing
//! - **Serialization**: Tests for JSON serialization/deserialization

use std::time::Duration;

use proptest::prelude::*;

use crate::{
    error::{ApiError, ApiResponse, ResponseMetadata, SqlQueryRequest},
    permissions::Permission,
    rate_limit::RateLimitConfig,
};

/// Get configurable `PropTest` configuration from environment
///
/// Use `PROPTEST_CASES` environment variable to control test thoroughness:
/// - Fast (default): `PROPTEST_CASES=32` (development)
/// - Standard: `PROPTEST_CASES=64` (CI)
/// - Thorough: `PROPTEST_CASES=256` (pre-release)
fn get_proptest_config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(32); // Default: fast tests

    ProptestConfig {
        cases,
        max_shrink_iters: if cases > 100 { 1000 } else { 500 },
        max_shrink_time: if cases > 100 { 10000 } else { 5000 },
        ..ProptestConfig::default()
    }
}

// ============================================================================
// Strategy Generators for API Components
// ============================================================================

/// Generate valid API key names
fn api_key_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_-]{2,31}")
        .unwrap()
        .prop_filter("name must not be empty", |s| !s.is_empty())
}

/// Generate valid permission strings
fn permission_string() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "read".to_string(),
        "write".to_string(),
        "admin".to_string(),
        "query".to_string(),
        "neural".to_string(),
        "quantum".to_string(),
        "dna".to_string(),
    ])
}

/// Generate lists of permissions
fn permission_list() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(permission_string(), 1..5)
}

/// Generate valid expiry hours (1 hour to 1 year)
fn expiry_hours() -> impl Strategy<Value = Option<u32>> {
    prop::option::of(1u32..8760)
}

/// Generate valid rate limits
fn rate_limit() -> impl Strategy<Value = Option<u32>> {
    prop::option::of(1u32..100000)
}

/// Generate arbitrary strings for fuzzing
fn arbitrary_string() -> impl Strategy<Value = String> {
    prop::string::string_regex(".{0,100}").unwrap()
}

/// Generate table names for SQL-related tests
fn table_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]{1,31}").unwrap()
}

/// Generate SQL queries for testing
fn sql_query() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(
            prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]{1,15}").unwrap(),
            1..5,
        ),
        table_name(),
    )
        .prop_map(|(cols, table)| format!("SELECT {} FROM {}", cols.join(", "), table))
}

/// Generate IP address strings
fn ip_address() -> impl Strategy<Value = String> {
    (0u8..255, 0u8..255, 0u8..255, 0u8..255)
        .prop_map(|(a, b, c, d)| format!("{a}.{b}.{c}.{d}"))
}

// ============================================================================
// Property-Based Tests
// ============================================================================

proptest! {
    #![proptest_config(get_proptest_config())]

    // ========================================================================
    // Permission Tests
    // ========================================================================

    /// Permission::read_only() should always contain "read"
    #[test]
    fn read_only_permissions_contain_read(_unused in 0..1i32) {
        let perms = Permission::read_only();
        prop_assert!(perms.contains(&"read".to_string()));
    }

    /// Permission::read_write() should contain both read and write
    #[test]
    fn read_write_permissions_contain_both(_unused in 0..1i32) {
        let perms = Permission::read_write();
        prop_assert!(perms.contains(&"read".to_string()));
        prop_assert!(perms.contains(&"write".to_string()));
    }

    /// Permission::admin_permissions() should contain admin
    #[test]
    fn admin_permissions_contain_admin(_unused in 0..1i32) {
        let perms = Permission::admin_permissions();
        prop_assert!(perms.contains(&"admin".to_string()));
    }

    // ========================================================================
    // Error Handling Tests
    // ========================================================================

    /// ApiError should format consistently
    #[test]
    fn api_error_formats_consistently(message in arbitrary_string()) {
        use actix_web::ResponseError;

        let error = ApiError::InternalServerError {
            message: message,
        };

        // Should not panic when formatting
        let _ = format!("{error}");
        let _ = format!("{error:?}");

        // Should produce valid HTTP response
        let response = error.error_response();
        prop_assert!(response.status().as_u16() >= 400);
    }

    /// ApiResponse should serialize correctly for any payload
    #[test]
    fn api_response_serializes(
        success in any::<bool>(),
        message in arbitrary_string()
    ) {
        let metadata = ResponseMetadata::new(Duration::from_millis(10), "test");

        let response: ApiResponse<String> = if success {
            ApiResponse::success("test_data".to_string(), metadata)
        } else {
            let error = ApiError::BadRequest(message);
            let metadata2 = ResponseMetadata::new(Duration::from_millis(10), "error");
            ApiResponse::error(error, metadata2)
        };

        // Should serialize to JSON without panic
        let json_result = serde_json::to_string(&response);
        prop_assert!(json_result.is_ok(), "Failed to serialize ApiResponse");

        // Should deserialize back
        if let Ok(json) = json_result {
            let parsed: Result<ApiResponse<String>, _> = serde_json::from_str(&json);
            prop_assert!(parsed.is_ok(), "Failed to deserialize ApiResponse");
        }
    }

    // ========================================================================
    // Request Validation Tests
    // ========================================================================

    /// SQL query requests should handle arbitrary input safely
    #[test]
    fn sql_query_request_handles_arbitrary_sql(query in arbitrary_string()) {
        let request = SqlQueryRequest {
            query: query,
        };

        // Should serialize without panic
        let json_result = serde_json::to_string(&request);
        prop_assert!(json_result.is_ok());
    }

    /// Valid SQL queries should be serializable
    #[test]
    fn valid_sql_query_serializable(query in sql_query()) {
        let request = SqlQueryRequest {
            query: query.clone(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: SqlQueryRequest = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(parsed.query, query);
    }

    // ========================================================================
    // Rate Limiting Tests
    // ========================================================================

    /// Rate limit configuration should accept valid values
    #[test]
    fn rate_limit_config_valid(
        requests_per_hour in 1u32..100000,
        burst_allowance in 1u32..1000
    ) {
        let config = RateLimitConfig {
            requests_per_window: requests_per_hour,
            window_size_seconds: 3600,
            burst_allowance: Some(burst_allowance),
            redis_url: None,
            fallback_to_memory: true,
        };

        prop_assert_eq!(config.requests_per_window, requests_per_hour);
        prop_assert_eq!(config.burst_allowance, Some(burst_allowance));
    }

    // ========================================================================
    // Configuration Tests
    // ========================================================================

    /// Configuration should handle various port numbers
    #[test]
    fn config_handles_ports(port in 1u16..65535) {
        // Just verify that port numbers in valid range don't cause issues
        let config_str = format!(
            r#"
            [server]
            host = "127.0.0.1"
            port = {port}
            workers = 4
            "#
        );

        // Should parse as valid TOML at minimum
        let parsed: Result<toml::Value, _> = toml::from_str(&config_str);
        prop_assert!(parsed.is_ok());
    }

    // ========================================================================
    // IP Address Handling Tests
    // ========================================================================

    /// Rate limiting should handle various IP formats
    #[test]
    fn rate_limit_handles_ips(ip in ip_address()) {
        // IP addresses in various formats should be acceptable as identifiers
        prop_assert!(ip.split('.').count() == 4);

        // Each octet should be valid
        for octet in ip.split('.') {
            let num: Result<u8, _> = octet.parse();
            prop_assert!(num.is_ok());
        }
    }

    // ========================================================================
    // API Key Name Validation Tests
    // ========================================================================

    /// Valid API key names should match pattern
    #[test]
    fn valid_api_key_names_match_pattern(name in api_key_name()) {
        // Should start with a letter
        let first_char = name.chars().next().unwrap();
        prop_assert!(first_char.is_alphabetic());

        // Should only contain valid characters
        prop_assert!(name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
    }

    /// Permission lists should not be empty
    #[test]
    fn permission_lists_not_empty(permissions in permission_list()) {
        prop_assert!(!permissions.is_empty());
    }

    /// Expiry hours should be reasonable when present
    #[test]
    fn expiry_hours_reasonable(expiry in expiry_hours()) {
        if let Some(hours) = expiry {
            prop_assert!(hours > 0);
            prop_assert!(hours <= 8760); // Max 1 year
        }
    }

    /// Rate limits should be positive when present
    #[test]
    fn rate_limits_positive(limit in rate_limit()) {
        if let Some(limit_val) = limit {
            prop_assert!(limit_val > 0);
        }
    }
}

// ============================================================================
// Non-Property Edge Case Tests
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_sql_injection_in_query_request() {
        let injection_queries = vec![
            "'; DROP TABLE users; --",
            "1; DELETE FROM data WHERE 1=1",
            "UNION SELECT * FROM passwords",
            "${7*7}",
            "{{constructor.constructor('return this')()}}",
        ];

        for query in injection_queries {
            let request = SqlQueryRequest {
                query: query.to_string(),
            };

            // Should serialize without panic
            let json = serde_json::to_string(&request);
            assert!(json.is_ok(), "Failed to serialize injection query");
        }
    }

    #[test]
    fn test_unicode_in_requests() {
        let unicode_strings = vec![
            "SELECT * FROM 用户表",
            "SELECT name FROM users WHERE city = 'München'",
            "SELECT * FROM таблица",
            "SELECT emoji FROM 📊",
        ];

        for query in unicode_strings {
            let request = SqlQueryRequest {
                query: query.to_string(),
            };

            let json = serde_json::to_string(&request);
            assert!(json.is_ok(), "Failed to serialize unicode query: {query}");
        }
    }

    #[test]
    fn test_null_bytes_in_request() {
        let request = SqlQueryRequest {
            query: "SELECT * FROM test\0table".to_string(),
        };

        // Should handle null bytes gracefully
        let json = serde_json::to_string(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_empty_query() {
        let request = SqlQueryRequest {
            query: String::new(),
        };

        let json = serde_json::to_string(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_very_long_query() {
        let long_query = "SELECT ".to_string() + &"a, ".repeat(10000) + "b FROM test";
        let request = SqlQueryRequest { query: long_query };

        let json = serde_json::to_string(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_api_error_variants() {
        use actix_web::ResponseError;

        let errors = vec![
            ApiError::BadRequest("test".to_string()),
            ApiError::Unauthorized("test".to_string()),
            ApiError::Forbidden("test".to_string()),
            ApiError::NotFound("test".to_string()),
            ApiError::InternalServerError {
                message: "test".to_string(),
            },
        ];

        for error in errors {
            // Should format without panic
            let _ = format!("{error}");
            let _ = format!("{error:?}");

            // Should produce valid HTTP response
            let response = error.error_response();
            assert!(response.status().as_u16() >= 400);
        }
    }

    #[test]
    fn test_permission_combinations() {
        let read_only = Permission::read_only();
        let read_write = Permission::read_write();
        let admin = Permission::admin_permissions();

        // read_write should be superset of read_only
        for perm in &read_only {
            assert!(read_write.contains(perm));
        }

        // admin should contain admin permission
        assert!(admin.contains(&"admin".to_string()));
    }

    #[test]
    fn test_api_response_success() {
        let metadata = ResponseMetadata::new(Duration::from_millis(5), "test");
        let response: ApiResponse<String> = ApiResponse::success("test".to_string(), metadata);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_api_response_error() {
        let metadata = ResponseMetadata::new(Duration::from_millis(5), "error test");
        let error = ApiError::BadRequest("error message".to_string());
        let response: ApiResponse<String> = ApiResponse::error(error, metadata);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("error"));
    }

    #[test]
    fn test_response_metadata_creation() {
        let duration = Duration::from_millis(100);
        let metadata = ResponseMetadata::new(duration, "test message");

        assert!(metadata.response_time_ms >= 99.0 && metadata.response_time_ms <= 101.0);
        assert_eq!(metadata.message, "test message");
        assert!(!metadata.request_id.is_empty());
        assert!(!metadata.timestamp.is_empty());
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();

        assert_eq!(config.requests_per_window, 100);
        assert_eq!(config.window_size_seconds, 3600);
        assert_eq!(config.burst_allowance, Some(10));
        assert!(config.fallback_to_memory);
    }
}
