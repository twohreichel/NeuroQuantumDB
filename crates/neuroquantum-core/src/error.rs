//! # Core Error Types
//!
//! Comprehensive error handling for the NeuroQuantumDB neuromorphic core
//! with detailed error classification and recovery strategies.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Result type alias for core operations
pub type CoreResult<T> = Result<T, CoreError>;

/// Comprehensive error types for neuromorphic core operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreError {
    /// Configuration validation errors
    InvalidConfig(String),

    /// Resource exhaustion errors (memory, connections, etc.)
    ResourceExhausted(String),

    /// Lock acquisition failures for concurrent access
    LockError(String),

    /// Entity not found errors
    NotFound(String),

    /// Invalid operation attempts
    InvalidOperation(String),

    /// Network topology errors
    NetworkError(String),

    /// Learning algorithm failures
    LearningError(String),

    /// Plasticity computation errors
    PlasticityError(String),

    /// Query processing errors
    QueryError(String),

    /// ARM64/NEON optimization errors
    NeonError(String),

    /// Memory allocation failures
    MemoryError(String),

    /// Serialization/deserialization errors
    SerializationError(String),

    /// I/O operation failures
    IoError(String),

    /// Timeout errors for operations
    TimeoutError(String),

    /// Validation errors for input data
    ValidationError(String),

    /// Internal system errors
    InternalError(String),
}

impl CoreError {
    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Create a new resource exhaustion error
    pub fn resource_exhausted<S: Into<String>>(msg: S) -> Self {
        Self::ResourceExhausted(msg.into())
    }

    /// Create a new lock error
    pub fn lock_error<S: Into<String>>(msg: S) -> Self {
        Self::LockError(msg.into())
    }

    /// Create a new not found error
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create a new invalid operation error
    pub fn invalid_operation<S: Into<String>>(msg: S) -> Self {
        Self::InvalidOperation(msg.into())
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            CoreError::InvalidConfig(_) => false,
            CoreError::ResourceExhausted(_) => true,
            CoreError::LockError(_) => true,
            CoreError::NotFound(_) => false,
            CoreError::InvalidOperation(_) => false,
            CoreError::NetworkError(_) => true,
            CoreError::LearningError(_) => true,
            CoreError::PlasticityError(_) => true,
            CoreError::QueryError(_) => true,
            CoreError::NeonError(_) => true,
            CoreError::MemoryError(_) => false,
            CoreError::SerializationError(_) => false,
            CoreError::IoError(_) => true,
            CoreError::TimeoutError(_) => true,
            CoreError::ValidationError(_) => false,
            CoreError::InternalError(_) => false,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            CoreError::InvalidConfig(_) => ErrorSeverity::High,
            CoreError::ResourceExhausted(_) => ErrorSeverity::Medium,
            CoreError::LockError(_) => ErrorSeverity::Low,
            CoreError::NotFound(_) => ErrorSeverity::Low,
            CoreError::InvalidOperation(_) => ErrorSeverity::Medium,
            CoreError::NetworkError(_) => ErrorSeverity::High,
            CoreError::LearningError(_) => ErrorSeverity::Medium,
            CoreError::PlasticityError(_) => ErrorSeverity::Medium,
            CoreError::QueryError(_) => ErrorSeverity::Medium,
            CoreError::NeonError(_) => ErrorSeverity::Low,
            CoreError::MemoryError(_) => ErrorSeverity::High,
            CoreError::SerializationError(_) => ErrorSeverity::Medium,
            CoreError::IoError(_) => ErrorSeverity::Medium,
            CoreError::TimeoutError(_) => ErrorSeverity::Low,
            CoreError::ValidationError(_) => ErrorSeverity::Medium,
            CoreError::InternalError(_) => ErrorSeverity::High,
        }
    }

    /// Get error category for monitoring
    pub fn category(&self) -> &'static str {
        match self {
            CoreError::InvalidConfig(_) => "configuration",
            CoreError::ResourceExhausted(_) => "resource",
            CoreError::LockError(_) => "concurrency",
            CoreError::NotFound(_) => "data",
            CoreError::InvalidOperation(_) => "operation",
            CoreError::NetworkError(_) => "network",
            CoreError::LearningError(_) => "learning",
            CoreError::PlasticityError(_) => "plasticity",
            CoreError::QueryError(_) => "query",
            CoreError::NeonError(_) => "optimization",
            CoreError::MemoryError(_) => "memory",
            CoreError::SerializationError(_) => "serialization",
            CoreError::IoError(_) => "io",
            CoreError::TimeoutError(_) => "timeout",
            CoreError::ValidationError(_) => "validation",
            CoreError::InternalError(_) => "internal",
        }
    }

    /// Log error with appropriate level
    pub fn log(&self) {
        match self.severity() {
            ErrorSeverity::Low => tracing::warn!("{}", self),
            ErrorSeverity::Medium => tracing::error!("{}", self),
            ErrorSeverity::High => {
                tracing::error!("{}", self);
                // Could trigger alerts for high severity errors
            }
        }
    }
}

/// Error severity levels for monitoring and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::InvalidConfig(msg) => write!(f, "Configuration error: {}", msg),
            CoreError::ResourceExhausted(msg) => write!(f, "Resource exhausted: {}", msg),
            CoreError::LockError(msg) => write!(f, "Lock error: {}", msg),
            CoreError::NotFound(msg) => write!(f, "Not found: {}", msg),
            CoreError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            CoreError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            CoreError::LearningError(msg) => write!(f, "Learning error: {}", msg),
            CoreError::PlasticityError(msg) => write!(f, "Plasticity error: {}", msg),
            CoreError::QueryError(msg) => write!(f, "Query error: {}", msg),
            CoreError::NeonError(msg) => write!(f, "NEON optimization error: {}", msg),
            CoreError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            CoreError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            CoreError::IoError(msg) => write!(f, "I/O error: {}", msg),
            CoreError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            CoreError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            CoreError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for CoreError {}

/// Top-level error type for NeuroQuantumDB operations
/// Combines all possible error types from different subsystems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NeuroQuantumError {
    /// Core neuromorphic computing errors
    CoreError(CoreError),

    /// Security-related errors
    SecurityError(String),

    /// DNA compression/decompression errors
    CompressionError(String),

    /// Quantum algorithm errors
    QuantumError(String),

    /// QSQL parsing/execution errors
    QueryError(String),

    /// Network/distributed system errors
    NetworkError(String),

    /// Configuration errors
    ConfigError(String),
}

impl fmt::Display for NeuroQuantumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeuroQuantumError::CoreError(e) => write!(f, "Core error: {}", e),
            NeuroQuantumError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            NeuroQuantumError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            NeuroQuantumError::QuantumError(msg) => write!(f, "Quantum error: {}", msg),
            NeuroQuantumError::QueryError(msg) => write!(f, "Query error: {}", msg),
            NeuroQuantumError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            NeuroQuantumError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for NeuroQuantumError {}

impl From<CoreError> for NeuroQuantumError {
    fn from(error: CoreError) -> Self {
        NeuroQuantumError::CoreError(error)
    }
}

impl From<crate::security::SecurityError> for NeuroQuantumError {
    fn from(error: crate::security::SecurityError) -> Self {
        NeuroQuantumError::SecurityError(error.to_string())
    }
}

impl From<crate::dna::CompressionError> for NeuroQuantumError {
    fn from(error: crate::dna::CompressionError) -> Self {
        NeuroQuantumError::CompressionError(error.to_string())
    }
}

/// Result type for high-level NeuroQuantumDB operations
pub type NeuroQuantumResult<T> = Result<T, NeuroQuantumError>;

/// Error recovery strategies for different error types
pub struct ErrorRecovery;

impl ErrorRecovery {
    /// Attempt to recover from an error
    pub fn attempt_recovery(error: &CoreError) -> Option<RecoveryAction> {
        match error {
            CoreError::ResourceExhausted(_) => Some(RecoveryAction::WaitAndRetry),
            CoreError::LockError(_) => Some(RecoveryAction::RetryWithBackoff),
            CoreError::NetworkError(_) => Some(RecoveryAction::Reconnect),
            CoreError::LearningError(_) => Some(RecoveryAction::ResetLearning),
            CoreError::PlasticityError(_) => Some(RecoveryAction::ResetPlasticity),
            CoreError::QueryError(_) => Some(RecoveryAction::SimplifyQuery),
            CoreError::NeonError(_) => Some(RecoveryAction::FallbackToScalar),
            CoreError::IoError(_) => Some(RecoveryAction::RetryWithBackoff),
            CoreError::TimeoutError(_) => Some(RecoveryAction::IncreaseTimeout),
            _ => None,
        }
    }
}

/// Recovery actions that can be taken for errors
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RecoveryAction {
    WaitAndRetry,
    RetryWithBackoff,
    Reconnect,
    ResetLearning,
    ResetPlasticity,
    SimplifyQuery,
    FallbackToScalar,
    IncreaseTimeout,
    RestartComponent,
}

/// Error context for debugging and monitoring
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error: CoreError,
    pub timestamp_secs: u64, // Store as seconds since epoch instead of Instant
    pub component: String,
    pub operation: String,
    pub recovery_attempted: bool,
    pub recovery_action: Option<RecoveryAction>,
}

impl ErrorContext {
    pub fn new(error: CoreError, component: String, operation: String) -> Self {
        let timestamp_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            error,
            timestamp_secs,
            component,
            operation,
            recovery_attempted: false,
            recovery_action: None,
        }
    }

    pub fn with_recovery(mut self, action: RecoveryAction) -> Self {
        self.recovery_attempted = true;
        self.recovery_action = Some(action);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = CoreError::config("Invalid learning rate");
        assert!(matches!(error, CoreError::InvalidConfig(_)));
        assert_eq!(error.category(), "configuration");
        assert_eq!(error.severity(), ErrorSeverity::High);
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_error_recovery() {
        let lock_error = CoreError::lock_error("Failed to acquire lock");
        let recovery = ErrorRecovery::attempt_recovery(&lock_error);
        assert_eq!(recovery, Some(RecoveryAction::RetryWithBackoff));

        let config_error = CoreError::config("Invalid config");
        let no_recovery = ErrorRecovery::attempt_recovery(&config_error);
        assert_eq!(no_recovery, None);
    }

    #[test]
    fn test_error_severity() {
        assert_eq!(CoreError::config("test").severity(), ErrorSeverity::High);
        assert_eq!(CoreError::lock_error("test").severity(), ErrorSeverity::Low);
        assert_eq!(
            CoreError::MemoryError("test".to_string()).severity(),
            ErrorSeverity::High
        );
    }

    #[test]
    fn test_error_context() {
        let error = CoreError::QueryError("Invalid query".to_string());
        let context = ErrorContext::new(
            error,
            "query_processor".to_string(),
            "process_query".to_string(),
        );

        assert_eq!(context.component, "query_processor");
        assert_eq!(context.operation, "process_query");
        assert!(!context.recovery_attempted);
    }

    #[test]
    fn test_error_display() {
        let error = CoreError::not_found("Node 123");
        assert_eq!(error.to_string(), "Not found: Node 123");
    }
}
