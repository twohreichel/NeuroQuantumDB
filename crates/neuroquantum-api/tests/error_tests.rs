//! Tests for API error handling and response types
//!
//! These tests validate API response creation and error conversion.

use neuroquantum_api::error::{ApiError, ApiResponse, ResponseMetadata};

#[test]
fn test_api_response_success() {
    let metadata = ResponseMetadata {
        request_id: "test-123".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        response_time_ms: 500.0,
        message: "Success".to_string(),
        version: "1.0.0".to_string(),
    };
    let response = ApiResponse::success("test data", metadata);
    assert!(response.success);
    assert!(response.data.is_some());
    assert!(response.error.is_none());
}

#[test]
fn test_api_error_response() {
    let error = ApiError::InvalidQuery {
        details: "Missing required field".to_string(),
    };

    let metadata = ResponseMetadata {
        request_id: "test-456".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        response_time_ms: 100.0,
        message: "Error".to_string(),
        version: "1.0.0".to_string(),
    };
    let response = ApiResponse::<()>::error(error, metadata);
    assert!(!response.success);
    assert!(response.data.is_none());
    assert!(response.error.is_some());
}
