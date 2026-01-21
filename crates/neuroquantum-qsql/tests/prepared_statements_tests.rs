//! Tests for prepared statements functionality
//!
//! Extracted from src/prepared_statements.rs inline tests

use std::collections::HashMap;

use neuroquantum_qsql::ast::{
    BinaryOperator, ExecuteStatement, Expression, FromClause, Literal, ParameterRef,
    PrepareStatement, SelectItem, SelectStatement, Statement, TableReference,
};
use neuroquantum_qsql::prepared_statements::PreparedStatementManager;

#[test]
fn test_prepare_statement() {
    let mut manager = PreparedStatementManager::new();

    // Create a simple SELECT statement with a parameter
    let select = SelectStatement {
        select_list: vec![SelectItem::Wildcard],
        from: Some(FromClause {
            relations: vec![TableReference {
                name: "users".to_string(),
                alias: None,
                synaptic_weight: None,
                quantum_state: None,
                subquery: None,
            }],
            joins: vec![],
        }),
        where_clause: Some(Expression::BinaryOp {
            left: Box::new(Expression::Identifier("id".to_string())),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Parameter(ParameterRef::Positional(1))),
        }),
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
        offset: None,
        synaptic_weight: None,
        plasticity_threshold: None,
        neuromatch_clause: None,
        quantum_parallel: false,
        grover_iterations: None,
        with_clause: None,
        union_clause: None,
    };

    let prepare_stmt = PrepareStatement {
        name: "get_user".to_string(),
        statement: Box::new(Statement::Select(select)),
        parameter_types: None,
    };

    // Prepare the statement
    assert!(manager.prepare(&prepare_stmt).is_ok());
    assert!(manager.exists("get_user"));

    // Check parameter count
    let prepared = manager.get("get_user").unwrap();
    assert_eq!(prepared.parameter_count, 1);
}

#[test]
fn test_execute_with_positional_parameters() {
    let mut manager = PreparedStatementManager::new();

    // Create a SELECT with positional parameter
    let select = SelectStatement {
        select_list: vec![SelectItem::Wildcard],
        from: Some(FromClause {
            relations: vec![TableReference {
                name: "users".to_string(),
                alias: None,
                synaptic_weight: None,
                quantum_state: None,
                subquery: None,
            }],
            joins: vec![],
        }),
        where_clause: Some(Expression::BinaryOp {
            left: Box::new(Expression::Identifier("id".to_string())),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Parameter(ParameterRef::Positional(1))),
        }),
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
        offset: None,
        synaptic_weight: None,
        plasticity_threshold: None,
        neuromatch_clause: None,
        quantum_parallel: false,
        grover_iterations: None,
        with_clause: None,
        union_clause: None,
    };

    let prepare_stmt = PrepareStatement {
        name: "get_user".to_string(),
        statement: Box::new(Statement::Select(select)),
        parameter_types: None,
    };

    manager.prepare(&prepare_stmt).unwrap();

    // Execute with a parameter value
    let execute_stmt = ExecuteStatement {
        name: "get_user".to_string(),
        parameters: vec![Expression::Literal(Literal::Integer(42))],
        named_parameters: HashMap::new(),
    };

    let bound = manager.bind_parameters("get_user", &execute_stmt).unwrap();

    // Verify the parameter was substituted
    if let Statement::Select(select) = bound {
        if let Some(Expression::BinaryOp { right, .. }) = select.where_clause {
            assert_eq!(*right, Expression::Literal(Literal::Integer(42)));
        } else {
            panic!("Expected BinaryOp in where clause");
        }
    } else {
        panic!("Expected Select statement");
    }
}

#[test]
fn test_execute_with_named_parameters() {
    let mut manager = PreparedStatementManager::new();

    // Create a SELECT with named parameter
    let select = SelectStatement {
        select_list: vec![SelectItem::Wildcard],
        from: Some(FromClause {
            relations: vec![TableReference {
                name: "users".to_string(),
                alias: None,
                synaptic_weight: None,
                quantum_state: None,
                subquery: None,
            }],
            joins: vec![],
        }),
        where_clause: Some(Expression::BinaryOp {
            left: Box::new(Expression::Identifier("name".to_string())),
            operator: BinaryOperator::Like,
            right: Box::new(Expression::Parameter(ParameterRef::Named(
                "pattern".to_string(),
            ))),
        }),
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
        offset: None,
        synaptic_weight: None,
        plasticity_threshold: None,
        neuromatch_clause: None,
        quantum_parallel: false,
        grover_iterations: None,
        with_clause: None,
        union_clause: None,
    };

    let prepare_stmt = PrepareStatement {
        name: "find_users".to_string(),
        statement: Box::new(Statement::Select(select)),
        parameter_types: None,
    };

    manager.prepare(&prepare_stmt).unwrap();

    // Execute with a named parameter value
    let mut named_params = HashMap::new();
    named_params.insert(
        "pattern".to_string(),
        Expression::Literal(Literal::String("John%".to_string())),
    );

    let execute_stmt = ExecuteStatement {
        name: "find_users".to_string(),
        parameters: vec![],
        named_parameters: named_params,
    };

    let bound = manager
        .bind_parameters("find_users", &execute_stmt)
        .unwrap();

    // Verify the parameter was substituted
    if let Statement::Select(select) = bound {
        if let Some(Expression::BinaryOp { right, .. }) = select.where_clause {
            assert_eq!(
                *right,
                Expression::Literal(Literal::String("John%".to_string()))
            );
        } else {
            panic!("Expected BinaryOp in where clause");
        }
    } else {
        panic!("Expected Select statement");
    }
}

#[test]
fn test_deallocate() {
    let mut manager = PreparedStatementManager::new();

    let select = SelectStatement {
        select_list: vec![SelectItem::Wildcard],
        from: None,
        where_clause: None,
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
        offset: None,
        synaptic_weight: None,
        plasticity_threshold: None,
        neuromatch_clause: None,
        quantum_parallel: false,
        grover_iterations: None,
        with_clause: None,
        union_clause: None,
    };

    let prepare_stmt = PrepareStatement {
        name: "test_stmt".to_string(),
        statement: Box::new(Statement::Select(select)),
        parameter_types: None,
    };

    manager.prepare(&prepare_stmt).unwrap();
    assert!(manager.exists("test_stmt"));

    manager.deallocate("test_stmt").unwrap();
    assert!(!manager.exists("test_stmt"));
}

#[test]
fn test_deallocate_all() {
    let mut manager = PreparedStatementManager::new();

    for i in 0..5 {
        let select = SelectStatement {
            select_list: vec![SelectItem::Wildcard],
            from: None,
            where_clause: None,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            synaptic_weight: None,
            plasticity_threshold: None,
            neuromatch_clause: None,
            quantum_parallel: false,
            grover_iterations: None,
            with_clause: None,
            union_clause: None,
        };

        let prepare_stmt = PrepareStatement {
            name: format!("stmt_{i}"),
            statement: Box::new(Statement::Select(select)),
            parameter_types: None,
        };

        manager.prepare(&prepare_stmt).unwrap();
    }

    assert_eq!(manager.list_statements().len(), 5);

    manager.deallocate_all();
    assert_eq!(manager.list_statements().len(), 0);
}

#[test]
fn test_duplicate_prepare_fails() {
    let mut manager = PreparedStatementManager::new();

    let select = SelectStatement {
        select_list: vec![SelectItem::Wildcard],
        from: None,
        where_clause: None,
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
        offset: None,
        synaptic_weight: None,
        plasticity_threshold: None,
        neuromatch_clause: None,
        quantum_parallel: false,
        grover_iterations: None,
        with_clause: None,
        union_clause: None,
    };

    let prepare_stmt = PrepareStatement {
        name: "test_stmt".to_string(),
        statement: Box::new(Statement::Select(select.clone())),
        parameter_types: None,
    };

    manager.prepare(&prepare_stmt).unwrap();

    // Second prepare with same name should fail
    let prepare_stmt2 = PrepareStatement {
        name: "test_stmt".to_string(),
        statement: Box::new(Statement::Select(select)),
        parameter_types: None,
    };

    assert!(manager.prepare(&prepare_stmt2).is_err());
}
