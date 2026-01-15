//! Error types for `NeuroQuantumDB`
//!
//! This module defines comprehensive error handling for the `NeuroQuantumDB` system,
//! including DNA compression errors, storage errors, and system-level errors.

use thiserror::Error;

/// Main error type for `NeuroQuantumDB` operations
#[derive(Debug, Error)]
pub enum NeuroQuantumError {
    #[error("Core system error: {0}")]
    CoreError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Query processing error: {0}")]
    QueryError(String),

    #[error("Learning engine error: {0}")]
    LearningError(String),

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Operation timeout: {0}")]
    Timeout(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Lock error: {0}")]
    LockError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Deadlock detected: {0}")]
    DeadlockDetected(String),

    #[error("Isolation violation: {0}")]
    IsolationViolation(String),

    #[error("Concurrent modification detected: {0}")]
    ConcurrentModification(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

// Type aliases for backward compatibility with existing modules
pub type CoreError = NeuroQuantumError;
pub type CoreResult<T> = Result<T, NeuroQuantumError>;

impl NeuroQuantumError {
    /// Create an invalid operation error
    #[must_use]
    pub fn invalid_operation(msg: &str) -> Self {
        Self::InvalidOperation(msg.to_string())
    }
}

// Convert DNA errors to NeuroQuantumError
impl From<crate::dna::DNAError> for NeuroQuantumError {
    fn from(error: crate::dna::DNAError) -> Self {
        Self::CompressionError(error.to_string())
    }
}
