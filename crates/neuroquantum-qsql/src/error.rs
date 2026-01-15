//! Error types for QSQL language implementation
//!
//! This module defines comprehensive error handling for parsing, optimization,
//! and execution of QSQL queries with neuromorphic and quantum extensions.

use std::fmt;
use thiserror::Error;

/// Main error type for QSQL operations
#[derive(Error, Debug)]
pub enum QSQLError {
    #[error("Parse error: {message} at position {position}")]
    ParseError { message: String, position: usize },

    #[error("Semantic error: {message}")]
    SemanticError { message: String },

    #[error("Optimization error: {message}")]
    OptimizationError { message: String },

    #[error("Execution error: {message}")]
    ExecutionError { message: String },

    #[error("Neuromorphic error: {message}")]
    NeuromorphicError { message: String },

    #[error("Quantum error: {message}")]
    QuantumError { message: String },

    #[error("Natural language processing error: {message}")]
    NLPError { message: String },

    #[error("Type error: {message}")]
    TypeError { message: String },

    #[error("Runtime error: {message}")]
    RuntimeError { message: String },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Memory error: {message}")]
    MemoryError { message: String },

    #[error("IO error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },

    #[error("Serialization error: {source}")]
    SerializationError {
        #[from]
        source: serde_json::Error,
    },

    #[error("Prepared statement error: {message}")]
    PreparedStatementError { message: String },
}

// Remove Clone trait for error types that contain non-cloneable fields
impl Clone for QSQLError {
    fn clone(&self) -> Self {
        match self {
            | Self::ParseError { message, position } => Self::ParseError {
                message: message.clone(),
                position: *position,
            },
            | Self::SemanticError { message } => Self::SemanticError {
                message: message.clone(),
            },
            | Self::OptimizationError { message } => Self::OptimizationError {
                message: message.clone(),
            },
            | Self::ExecutionError { message } => Self::ExecutionError {
                message: message.clone(),
            },
            | Self::NeuromorphicError { message } => Self::NeuromorphicError {
                message: message.clone(),
            },
            | Self::QuantumError { message } => Self::QuantumError {
                message: message.clone(),
            },
            | Self::NLPError { message } => Self::NLPError {
                message: message.clone(),
            },
            | Self::TypeError { message } => Self::TypeError {
                message: message.clone(),
            },
            | Self::RuntimeError { message } => Self::RuntimeError {
                message: message.clone(),
            },
            | Self::ConfigError { message } => Self::ConfigError {
                message: message.clone(),
            },
            | Self::MemoryError { message } => Self::MemoryError {
                message: message.clone(),
            },
            | Self::IOError { source } => Self::IOError {
                source: std::io::Error::new(source.kind(), format!("{source}")),
            },
            | Self::SerializationError { source } => Self::SerializationError {
                source: serde_json::Error::io(std::io::Error::other(format!("{source}"))),
            },
            | Self::PreparedStatementError { message } => Self::PreparedStatementError {
                message: message.clone(),
            },
        }
    }
}

/// Specialized error for parsing operations
#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Unexpected token '{token}' at position {position}")]
    UnexpectedToken { token: String, position: usize },

    #[error("Expected '{expected}' but found '{found}' at position {position}")]
    ExpectedToken {
        expected: String,
        found: String,
        position: usize,
    },

    #[error("Invalid syntax: {message} at position {position}")]
    InvalidSyntax { message: String, position: usize },

    #[error("Incomplete query at position {position}")]
    IncompleteQuery { position: usize },

    #[error("Invalid neuromorphic syntax: {message}")]
    InvalidNeuromorphicSyntax { message: String },

    #[error("Invalid quantum syntax: {message}")]
    InvalidQuantumSyntax { message: String },

    #[error("Unsupported feature: {feature}")]
    UnsupportedFeature { feature: String },

    #[error("Invalid identifier: '{identifier}'")]
    InvalidIdentifier { identifier: String },

    #[error("Invalid literal: '{literal}'")]
    InvalidLiteral { literal: String },

    #[error("Nested query depth exceeded (max: {max_depth})")]
    NestedQueryDepthExceeded { max_depth: usize },
}

/// Specialized error for neuromorphic operations
#[derive(Error, Debug, Clone)]
pub enum NeuromorphicError {
    #[error("Invalid synaptic weight: {weight} (must be between 0.0 and 1.0)")]
    InvalidSynapticWeight { weight: f32 },

    #[error("Invalid learning rate: {rate} (must be between 0.0 and 1.0)")]
    InvalidLearningRate { rate: f32 },

    #[error("Synaptic network not found: {network_id}")]
    SynapticNetworkNotFound { network_id: String },

    #[error("Plasticity threshold exceeded: {current} > {threshold}")]
    PlasticityThresholdExceeded { current: f32, threshold: f32 },

    #[error("Hebbian learning convergence failed after {iterations} iterations")]
    HebbianConvergenceFailed { iterations: u32 },

    #[error("Neural pathway optimization failed: {reason}")]
    PathwayOptimizationFailed { reason: String },

    #[error("Spike timing dependent plasticity error: {message}")]
    STDPError { message: String },

    #[error("Homeostatic scaling failed: {reason}")]
    HomeostaticScalingFailed { reason: String },
}

/// Specialized error for quantum operations
#[derive(Error, Debug, Clone)]
pub enum QuantumError {
    #[error("Quantum coherence lost: {reason}")]
    CoherenceLost { reason: String },

    #[error("Grover's algorithm failed: {reason}")]
    GroversFailed { reason: String },

    #[error("Amplitude amplification error: {message}")]
    AmplitudeAmplificationError { message: String },

    #[error("Quantum entanglement broken: {entities:?}")]
    EntanglementBroken { entities: Vec<String> },

    #[error("Superposition collapse unexpected: {state}")]
    UnexpectedCollapse { state: String },

    #[error("Oracle function not found: {oracle_name}")]
    OracleNotFound { oracle_name: String },

    #[error("Quantum measurement error: {message}")]
    MeasurementError { message: String },

    #[error("Quantum annealing convergence failed after {iterations} iterations")]
    AnnealingConvergenceFailed { iterations: u32 },

    #[error("Invalid quantum state: {state}")]
    InvalidQuantumState { state: String },
}

/// Specialized error for natural language processing
#[derive(Error, Debug, Clone)]
pub enum NLPError {
    #[error("Failed to parse natural language query: {query}")]
    ParseFailed { query: String },

    #[error("Ambiguous query interpretation: {interpretations:?}")]
    AmbiguousInterpretation { interpretations: Vec<String> },

    #[error("Unsupported natural language construct: {construct}")]
    UnsupportedConstruct { construct: String },

    #[error("Language model not available")]
    ModelNotAvailable,

    #[error("Translation failed: {from} -> {to}")]
    TranslationFailed { from: String, to: String },

    #[error("Intent recognition failed: {text}")]
    IntentRecognitionFailed { text: String },

    #[error("Entity extraction failed: {text}")]
    EntityExtractionFailed { text: String },
}

/// Specialized error for query optimization
#[derive(Error, Debug, Clone)]
pub enum OptimizationError {
    #[error("Cost estimation failed: {reason}")]
    CostEstimationFailed { reason: String },

    #[error("Index selection failed: {table}")]
    IndexSelectionFailed { table: String },

    #[error("Join order optimization failed: {reason}")]
    JoinOrderFailed { reason: String },

    #[error("Predicate pushdown failed: {reason}")]
    PredicatePushdownFailed { reason: String },

    #[error("Statistics not available for table: {table}")]
    StatisticsUnavailable { table: String },

    #[error("Optimization timeout after {seconds} seconds")]
    OptimizationTimeout { seconds: u64 },

    #[error("Recursive optimization detected")]
    RecursiveOptimization,

    #[error("Invalid optimization hint: {hint}")]
    InvalidOptimizationHint { hint: String },
}

/// Result type alias for QSQL operations
pub type QSQLResult<T> = Result<T, QSQLError>;

/// Error context for better error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub query: String,
    pub position: Option<usize>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub operation: String,
}

impl ErrorContext {
    #[must_use]
    pub const fn new(query: String, operation: String) -> Self {
        Self {
            query,
            position: None,
            line: None,
            column: None,
            operation,
        }
    }

    #[must_use]
    pub fn with_position(mut self, position: usize) -> Self {
        self.position = Some(position);

        // Calculate line and column from position
        let mut line = 1;
        let mut column = 1;

        for (i, ch) in self.query.chars().enumerate() {
            if i >= position {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        self.line = Some(line);
        self.column = Some(column);
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error in {}", self.operation)?;

        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(f, " at line {line}, column {column}")?;
        } else if let Some(position) = self.position {
            write!(f, " at position {position}")?;
        }

        if !self.query.is_empty() {
            write!(f, " in query: {}", self.query)?;
        }

        Ok(())
    }
}

/// Helper trait for adding context to errors
pub trait WithContext<T> {
    fn with_context(self, context: ErrorContext) -> QSQLResult<T>;
    fn with_operation(self, operation: &str) -> QSQLResult<T>;
}

impl<T, E> WithContext<T> for Result<T, E>
where
    E: Into<QSQLError>,
{
    fn with_context(self, context: ErrorContext) -> QSQLResult<T> {
        self.map_err(|e| {
            let mut error = e.into();
            // Add context information to error message
            match &mut error {
                | QSQLError::ParseError { message, position } => {
                    if let Some(ctx_pos) = context.position {
                        *position = ctx_pos;
                    }
                    *message = format!("{message} ({context})");
                },
                | QSQLError::SemanticError { message } => {
                    *message = format!("{message} ({context})");
                },
                | _ => {},
            }
            error
        })
    }

    fn with_operation(self, operation: &str) -> QSQLResult<T> {
        self.map_err(|e| {
            let mut error = e.into();
            match &mut error {
                | QSQLError::ParseError { message, .. } => {
                    *message = format!("{message} during {operation}");
                },
                | QSQLError::SemanticError { message } => {
                    *message = format!("{message} during {operation}");
                },
                | QSQLError::OptimizationError { message } => {
                    *message = format!("{message} during {operation}");
                },
                | QSQLError::ExecutionError { message } => {
                    *message = format!("{message} during {operation}");
                },
                | _ => {},
            }
            error
        })
    }
}

// Conversion implementations for easier error handling
impl From<ParseError> for QSQLError {
    fn from(err: ParseError) -> Self {
        match err {
            | ParseError::UnexpectedToken { token, position } => Self::ParseError {
                message: format!("Unexpected token '{token}'"),
                position,
            },
            | ParseError::ExpectedToken {
                expected,
                found,
                position,
            } => Self::ParseError {
                message: format!("Expected '{expected}' but found '{found}'"),
                position,
            },
            | _ => Self::ParseError {
                message: err.to_string(),
                position: 0,
            },
        }
    }
}

impl From<NeuromorphicError> for QSQLError {
    fn from(err: NeuromorphicError) -> Self {
        Self::NeuromorphicError {
            message: err.to_string(),
        }
    }
}

impl From<QuantumError> for QSQLError {
    fn from(err: QuantumError) -> Self {
        Self::QuantumError {
            message: err.to_string(),
        }
    }
}

impl From<NLPError> for QSQLError {
    fn from(err: NLPError) -> Self {
        Self::NLPError {
            message: err.to_string(),
        }
    }
}

impl From<OptimizationError> for QSQLError {
    fn from(err: OptimizationError) -> Self {
        Self::OptimizationError {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new("SELECT * FROM users".to_string(), "parsing".to_string())
            .with_position(7);

        assert_eq!(context.line, Some(1));
        assert_eq!(context.column, Some(8));
        assert_eq!(context.position, Some(7));
    }

    #[test]
    fn test_neuromorphic_error_conversion() {
        let neuro_error = NeuromorphicError::InvalidSynapticWeight { weight: 1.5 };
        let qsql_error: QSQLError = neuro_error.into();

        match qsql_error {
            | QSQLError::NeuromorphicError { message } => {
                assert!(message.contains("Invalid synaptic weight"));
            },
            | _ => panic!("Expected NeuromorphicError"),
        }
    }

    #[test]
    fn test_quantum_error_conversion() {
        let quantum_error = QuantumError::CoherenceLost {
            reason: "decoherence".to_string(),
        };
        let qsql_error: QSQLError = quantum_error.into();

        match qsql_error {
            | QSQLError::QuantumError { message } => {
                assert!(message.contains("Quantum coherence lost"));
            },
            | _ => panic!("Expected QuantumError"),
        }
    }

    #[test]
    fn test_with_context_trait() {
        let result: Result<(), ParseError> = Err(ParseError::InvalidSyntax {
            message: "test error".to_string(),
            position: 5,
        });

        let context = ErrorContext::new(
            "SELECT * FROM test".to_string(),
            "test operation".to_string(),
        );

        let with_context = result.with_context(context);
        assert!(with_context.is_err());
    }
}
