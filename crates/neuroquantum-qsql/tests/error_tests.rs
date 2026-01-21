//! Tests for error handling functionality
//!
//! Extracted from src/error.rs inline tests

use neuroquantum_qsql::error::{
    ErrorContext, NeuromorphicError, ParseError, QSQLError, QuantumError, WithContext,
};

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
