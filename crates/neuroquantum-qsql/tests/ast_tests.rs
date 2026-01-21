//! Tests for AST structures and creation
//!
//! Extracted from src/ast.rs inline tests

use neuroquantum_qsql::ast::{
    BinaryOperator, Expression, Literal, NeuroMatchStatement, QuantumSearchStatement,
};

#[test]
fn test_neuromorphic_ast_creation() {
    let neuromatch = NeuroMatchStatement {
        target_table: "users".to_string(),
        pattern_expression: Expression::BinaryOp {
            left: Box::new(Expression::Identifier("age".to_string())),
            operator: BinaryOperator::GreaterThan,
            right: Box::new(Expression::Literal(Literal::Integer(30))),
        },
        synaptic_weight: 0.8,
        learning_rate: Some(0.01),
        activation_threshold: Some(0.5),
        hebbian_strengthening: true,
    };

    assert_eq!(neuromatch.target_table, "users");
    assert_eq!(neuromatch.synaptic_weight, 0.8);
    assert!(neuromatch.hebbian_strengthening);
}

#[test]
fn test_quantum_ast_creation() {
    let quantum_search = QuantumSearchStatement {
        target_table: "products".to_string(),
        search_expression: Expression::BinaryOp {
            left: Box::new(Expression::Identifier("price".to_string())),
            operator: BinaryOperator::LessThan,
            right: Box::new(Expression::Literal(Literal::Float(100.0))),
        },
        amplitude_amplification: true,
        oracle_function: Some("price_oracle".to_string()),
        max_iterations: Some(10),
    };

    assert_eq!(quantum_search.target_table, "products");
    assert!(quantum_search.amplitude_amplification);
    assert_eq!(quantum_search.max_iterations, Some(10));
}

#[test]
fn test_synaptic_expression() {
    let synaptic_expr = Expression::SynapticMatch {
        pattern: Box::new(Expression::Identifier("user_behavior".to_string())),
        weight: 0.75,
        threshold: Some(0.6),
    };

    match synaptic_expr {
        | Expression::SynapticMatch { weight, .. } => assert_eq!(weight, 0.75),
        | _ => panic!("Expected SynapticMatch expression"),
    }
}

#[test]
fn test_quantum_superposition() {
    let superposition = Expression::QuantumSuperposition {
        states: vec![
            Expression::Literal(Literal::Boolean(true)),
            Expression::Literal(Literal::Boolean(false)),
        ],
    };

    match superposition {
        | Expression::QuantumSuperposition { states } => assert_eq!(states.len(), 2),
        | _ => panic!("Expected QuantumSuperposition expression"),
    }
}
