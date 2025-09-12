use anyhow::Result;
use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;
use thiserror::Error;

/// API-specific error types for NeuroQuantumDB REST interface
#[derive(Error, Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ApiError {
    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    #[error("Authorization denied: {resource}")]
    AuthorizationDenied { resource: String },

    #[error("Invalid query: {details}")]
    InvalidQuery { details: String },

    #[error("Quantum operation failed: {operation}")]
    QuantumOperationFailed { operation: String },

    #[error("DNA compression error: {reason}")]
    CompressionError { reason: String },

    #[error("Rate limit exceeded: {limit} requests per {window}")]
    RateLimitExceeded { limit: u32, window: String },

    #[error("Resource not found: {resource_type}")]
    ResourceNotFound { resource_type: String },

    #[error("Internal server error: {context}")]
    InternalError { context: String },

    #[error("Validation error: {field} - {message}")]
    ValidationError { field: String, message: String },

    #[error("Quantum-resistant encryption error: {details}")]
    EncryptionError { details: String },
}

/// Standard API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub metadata: ResponseMetadata,
}

/// Response metadata for observability
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub request_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub processing_time_us: u64,
    pub quantum_enhancement: bool,
    pub compression_ratio: Option<f32>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, metadata: ResponseMetadata) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata,
        }
    }

    pub fn error(error: ApiError, metadata: ResponseMetadata) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            metadata,
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let metadata = ResponseMetadata {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            processing_time_us: 0,
            quantum_enhancement: false,
            compression_ratio: None,
        };

        let response = ApiResponse::<()>::error(self.clone(), metadata);

        match self {
            ApiError::AuthenticationFailed { .. } => HttpResponse::Unauthorized().json(response),
            ApiError::AuthorizationDenied { .. } => HttpResponse::Forbidden().json(response),
            ApiError::InvalidQuery { .. } => HttpResponse::BadRequest().json(response),
            ApiError::ValidationError { .. } => HttpResponse::BadRequest().json(response),
            ApiError::ResourceNotFound { .. } => HttpResponse::NotFound().json(response),
            ApiError::RateLimitExceeded { .. } => HttpResponse::TooManyRequests().json(response),
            _ => HttpResponse::InternalServerError().json(response),
        }
    }
}

/// Authentication token structure
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub sub: String,        // Subject (user ID)
    pub exp: usize,         // Expiration time
    pub iat: usize,         // Issued at
    pub quantum_level: u8,  // Quantum security level (0-255)
    pub permissions: Vec<String>,
}

/// Quantum-resistant authentication claims
#[derive(Debug, Serialize, Deserialize)]
pub struct QuantumAuthClaims {
    pub user_id: String,
    pub session_id: String,
    pub quantum_signature: String,
    pub kyber_public_key: String,
    pub dilithium_signature: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let metadata = ResponseMetadata {
            request_id: "test-123".to_string(),
            timestamp: chrono::Utc::now(),
            processing_time_us: 500,
            quantum_enhancement: true,
            compression_ratio: Some(1000.0),
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
            timestamp: chrono::Utc::now(),
            processing_time_us: 100,
            quantum_enhancement: false,
            compression_ratio: None,
        };

        let response = ApiResponse::<()>::error(error, metadata);
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }
}
