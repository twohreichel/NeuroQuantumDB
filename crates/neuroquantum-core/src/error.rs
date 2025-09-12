//! Error types for the NeuroQuantumDB core

use thiserror::Error;
use crate::synaptic::NodeId;

/// Result type alias for core operations
pub type CoreResult<T> = Result<T, CoreError>;

/// Core error types for neuromorphic operations
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Node {0} not found")]
    NodeNotFound(NodeId),

    #[error("Connection already exists between nodes {source} and {target}")]
    ConnectionAlreadyExists { source: NodeId, target: NodeId },

    #[error("Connection not found between nodes {source} and {target}")]
    ConnectionNotFound { source: NodeId, target: NodeId },

    #[error("Network capacity exceeded (max nodes reached)")]
    NetworkCapacityExceeded,

    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,

    #[error("Invalid node state for node {0}")]
    InvalidNodeState(NodeId),

    #[error("Invalid connection weight {weight} between nodes {source} and {target} (must be -1.0 to 1.0)")]
    InvalidConnectionWeight {
        source: NodeId,
        target: NodeId,
        weight: f32,
    },

    #[error("Dangling connection from node {source} to non-existent node {target}")]
    DanglingConnection { source: NodeId, target: NodeId },

    #[error("Learning engine error: {message}")]
    LearningError { message: String },

    #[error("Plasticity matrix error: {message}")]
    PlasticityError { message: String },

    #[error("Query processing error: {message}")]
    QueryError { message: String },

    #[error("ARM64/NEON optimization error: {message}")]
    OptimizationError { message: String },

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl CoreError {
    /// Create a learning error
    pub fn learning_error(message: impl Into<String>) -> Self {
        Self::LearningError { message: message.into() }
    }

    /// Create a plasticity error
    pub fn plasticity_error(message: impl Into<String>) -> Self {
        Self::PlasticityError { message: message.into() }
    }

    /// Create a query error
    pub fn query_error(message: impl Into<String>) -> Self {
        Self::QueryError { message: message.into() }
    }

    /// Create an optimization error
    pub fn optimization_error(message: impl Into<String>) -> Self {
        Self::OptimizationError { message: message.into() }
    }

    /// Create a configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError { message: message.into() }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError { message: message.into() }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            CoreError::NodeNotFound(_) => true,
            CoreError::ConnectionNotFound { .. } => true,
            CoreError::ConnectionAlreadyExists { .. } => true,
            CoreError::NetworkCapacityExceeded => false,
            CoreError::MemoryLimitExceeded => false,
            CoreError::InvalidNodeState(_) => false,
            CoreError::InvalidConnectionWeight { .. } => false,
            CoreError::DanglingConnection { .. } => false,
            CoreError::LearningError { .. } => true,
            CoreError::PlasticityError { .. } => true,
            CoreError::QueryError { .. } => true,
            CoreError::OptimizationError { .. } => true,
            CoreError::SerializationError(_) => true,
            CoreError::IoError(_) => true,
            CoreError::ConfigError { .. } => false,
            CoreError::InternalError { .. } => false,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            CoreError::NodeNotFound(_) => ErrorSeverity::Low,
            CoreError::ConnectionNotFound { .. } => ErrorSeverity::Low,
            CoreError::ConnectionAlreadyExists { .. } => ErrorSeverity::Low,
            CoreError::NetworkCapacityExceeded => ErrorSeverity::High,
            CoreError::MemoryLimitExceeded => ErrorSeverity::Critical,
            CoreError::InvalidNodeState(_) => ErrorSeverity::High,
            CoreError::InvalidConnectionWeight { .. } => ErrorSeverity::Medium,
            CoreError::DanglingConnection { .. } => ErrorSeverity::High,
            CoreError::LearningError { .. } => ErrorSeverity::Medium,
            CoreError::PlasticityError { .. } => ErrorSeverity::Medium,
            CoreError::QueryError { .. } => ErrorSeverity::Medium,
            CoreError::OptimizationError { .. } => ErrorSeverity::Low,
            CoreError::SerializationError(_) => ErrorSeverity::Medium,
            CoreError::IoError(_) => ErrorSeverity::Medium,
            CoreError::ConfigError { .. } => ErrorSeverity::High,
            CoreError::InternalError { .. } => ErrorSeverity::Critical,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "LOW"),
            ErrorSeverity::Medium => write!(f, "MEDIUM"),
            ErrorSeverity::High => write!(f, "HIGH"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        assert!(CoreError::NodeNotFound(1).is_recoverable());
        assert!(!CoreError::MemoryLimitExceeded.is_recoverable());
        assert!(!CoreError::NetworkCapacityExceeded.is_recoverable());
    }

    #[test]
    fn test_error_severity() {
        assert_eq!(CoreError::NodeNotFound(1).severity(), ErrorSeverity::Low);
        assert_eq!(CoreError::MemoryLimitExceeded.severity(), ErrorSeverity::Critical);
        assert_eq!(CoreError::InvalidNodeState(1).severity(), ErrorSeverity::High);
    }

    #[test]
    fn test_error_creation_helpers() {
        let learning_err = CoreError::learning_error("test learning error");
        match learning_err {
            CoreError::LearningError { message } => assert_eq!(message, "test learning error"),
            _ => panic!("Expected LearningError"),
        }

        let config_err = CoreError::config_error("test config error");
        match config_err {
            CoreError::ConfigError { message } => assert_eq!(message, "test config error"),
            _ => panic!("Expected ConfigError"),
        }
    }
}
