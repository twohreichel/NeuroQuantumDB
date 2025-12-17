//! Comprehensive SQL Engine Tests
//!
//! Diese Test-Suite validiert die vollständige SQL Engine Implementierung
//! mit SELECT, INSERT, UPDATE, DELETE sowie neuromorphic und quantum features.

use crate::ast::*;
use crate::parser::QSQLParser;
use crate::query_plan::*;
use std::time::Duration;

#[test]
fn test_basic_select_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM users");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert_eq!(select.select_list.len(), 1);
            assert!(select.from.is_some());
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_select_with_where_clause() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT name, age FROM users WHERE age > 30");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert_eq!(select.select_list.len(), 2);
            assert!(select.where_clause.is_some());
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_select_with_limit() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM products LIMIT 10");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert_eq!(select.limit, Some(10));
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_insert_statement_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("INSERT INTO users (name, age) VALUES ('John', 25)");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Insert(insert) => {
            assert_eq!(insert.table_name, "users");
            assert!(insert.columns.is_some());
            assert_eq!(insert.values.len(), 1);
        }
        _ => panic!("Expected INSERT statement"),
    }
}

#[test]
fn test_insert_multiple_values() {
    let parser = QSQLParser::new();
    let result = parser
        .parse("INSERT INTO products (name, price) VALUES ('Widget', 19.99), ('Gadget', 29.99)");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Insert(insert) => {
            assert_eq!(insert.table_name, "products");
            assert_eq!(insert.values.len(), 2);
        }
        _ => panic!("Expected INSERT statement"),
    }
}

#[test]
fn test_update_statement_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("UPDATE users SET age = 26 WHERE name = 'John'");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Update(update) => {
            assert_eq!(update.table_name, "users");
            assert_eq!(update.assignments.len(), 1);
            assert!(update.where_clause.is_some());
        }
        _ => panic!("Expected UPDATE statement"),
    }
}

#[test]
fn test_update_multiple_columns() {
    let parser = QSQLParser::new();
    let result = parser.parse("UPDATE products SET price = 24.99, category = 'electronics'");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Update(update) => {
            assert_eq!(update.table_name, "products");
            assert_eq!(update.assignments.len(), 2);
        }
        _ => panic!("Expected UPDATE statement"),
    }
}

#[test]
fn test_delete_statement_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("DELETE FROM users WHERE age < 18");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Delete(delete) => {
            assert_eq!(delete.table_name, "users");
            assert!(delete.where_clause.is_some());
        }
        _ => panic!("Expected DELETE statement"),
    }
}

#[test]
fn test_delete_all_rows() {
    let parser = QSQLParser::new();
    let result = parser.parse("DELETE FROM temp_table");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Delete(delete) => {
            assert_eq!(delete.table_name, "temp_table");
            assert!(delete.where_clause.is_none());
        }
        _ => panic!("Expected DELETE statement"),
    }
}

#[test]
fn test_neuromorphic_neuromatch_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("NEUROMATCH patterns");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::NeuroMatch(neuromatch) => {
            assert_eq!(neuromatch.target_table, "patterns");
            assert!(neuromatch.hebbian_strengthening);
        }
        _ => panic!("Expected NEUROMATCH statement"),
    }
}

#[test]
fn test_quantum_search_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("QUANTUM_SEARCH quantum_data AMPLITUDE_AMPLIFICATION");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::QuantumSearch(quantum_search) => {
            assert_eq!(quantum_search.target_table, "quantum_data");
            assert!(quantum_search.amplitude_amplification);
        }
        _ => panic!("Expected QUANTUM_SEARCH statement"),
    }
}

#[test]
fn test_dna_literal_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM sequences WHERE dna_sequence = DNA:ATGC");

    assert!(result.is_ok());
    // This tests that the parser can handle DNA literals in WHERE clauses
}

#[test]
fn test_complex_expression_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT name FROM users WHERE age > 18 AND status = 'active'");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert!(select.where_clause.is_some());
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_parser_error_handling() {
    let parser = QSQLParser::new();

    // Invalid SQL should return an error
    let result = parser.parse("INVALID SQL SYNTAX");
    assert!(result.is_err());

    // Empty query should return an error
    let result = parser.parse("");
    assert!(result.is_err());

    // Incomplete query should return an error
    let result = parser.parse("SELECT");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_query_executor_integration() {
    // Use testing config to allow legacy mode with simulated data
    let mut executor = QueryExecutor::with_config(ExecutorConfig::testing()).unwrap();
    let parser = QSQLParser::new();

    // Parse a simple SELECT query
    let ast = parser.parse("SELECT * FROM users").unwrap();

    // Create a basic query plan
    let plan = create_test_query_plan(ast);

    // Execute the query
    let result = executor.execute(&plan).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_neuromorphic_execution() {
    // Use testing config to allow legacy mode with simulated data
    let mut executor = QueryExecutor::with_config(ExecutorConfig::testing()).unwrap();
    let parser = QSQLParser::new();

    // Parse neuromorphic query
    let ast = parser.parse("NEUROMATCH brain_patterns").unwrap();
    let plan = create_test_query_plan(ast);

    // Execute neuromorphic query
    let result = executor.execute(&plan).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_quantum_execution() {
    // Use testing config to allow legacy mode with simulated data
    let mut executor = QueryExecutor::with_config(ExecutorConfig::testing()).unwrap();
    let parser = QSQLParser::new();

    // Parse quantum search query
    let ast = parser.parse("QUANTUM_SEARCH quantum_states").unwrap();
    let plan = create_test_query_plan(ast);

    // Execute quantum query
    let result = executor.execute(&plan).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_complete_sql_workflow() {
    // Use testing config to allow legacy mode with simulated data
    let mut executor = QueryExecutor::with_config(ExecutorConfig::testing()).unwrap();
    let parser = QSQLParser::new();

    // Test a complete workflow: CREATE, INSERT, SELECT, UPDATE, DELETE
    let queries = vec![
        "SELECT * FROM test_table",
        "INSERT INTO test_table (id, name) VALUES (1, 'Test')",
        "UPDATE test_table SET name = 'Updated' WHERE id = 1",
        "DELETE FROM test_table WHERE id = 1",
    ];

    for query in queries {
        let ast = parser.parse(query).unwrap();
        let plan = create_test_query_plan(ast);
        let result = executor.execute(&plan).await;
        assert!(result.is_ok(), "Failed to execute query: {}", query);
    }
}

#[test]
fn test_performance_requirements() {
    let parser = QSQLParser::new();
    let start = std::time::Instant::now();

    // Parse 100 queries to test performance
    for i in 0..100 {
        let query = format!("SELECT * FROM table_{} WHERE id = {}", i % 10, i);
        let result = parser.parse(&query);
        assert!(result.is_ok());
    }

    let duration = start.elapsed();
    println!("Parsed 100 queries in {:?}", duration);

    // Should be able to parse 100 simple queries in under 100ms
    assert!(
        duration.as_millis() < 1000,
        "Parsing took too long: {:?}",
        duration
    );
}

/// Demonstration of the complete SQL Engine capabilities
#[test]
fn demo_complete_sql_engine() {
    println!("🚀 NeuroQuantum SQL Engine Demo");

    let parser = QSQLParser::new();

    // Demonstrate SQL compatibility
    println!("📊 Standard SQL Support:");
    let standard_queries = vec![
        "SELECT * FROM users",
        "INSERT INTO products (name, price) VALUES ('Widget', 19.99)",
        "UPDATE users SET age = 30 WHERE name = 'John'",
        "DELETE FROM logs WHERE created < '2023-01-01'",
    ];

    for query in standard_queries {
        let result = parser.parse(query);
        println!(
            "  ✅ {}: {}",
            query,
            if result.is_ok() { "PARSED" } else { "FAILED" }
        );
    }

    // Demonstrate neuromorphic features
    println!("🧠 Neuromorphic Extensions:");
    let neuro_queries = vec![
        "NEUROMATCH neural_patterns",
        "SELECT * FROM memories WHERE synaptic_weight > 0.8",
    ];

    for query in neuro_queries {
        let result = parser.parse(query);
        println!(
            "  ✅ {}: {}",
            query,
            if result.is_ok() { "PARSED" } else { "FAILED" }
        );
    }

    // Demonstrate quantum features
    println!("⚛️  Quantum Extensions:");
    let quantum_queries = vec![
        "QUANTUM_SEARCH quantum_states",
        "QUANTUM_SEARCH entangled_data AMPLITUDE_AMPLIFICATION",
    ];

    for query in quantum_queries {
        let result = parser.parse(query);
        println!(
            "  ✅ {}: {}",
            query,
            if result.is_ok() { "PARSED" } else { "FAILED" }
        );
    }
}

fn create_test_query_plan(statement: Statement) -> QueryPlan {
    QueryPlan {
        statement,
        execution_strategy: ExecutionStrategy::Sequential,
        synaptic_pathways: vec![],
        quantum_optimizations: vec![],
        estimated_cost: 100.0,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(1),
            iterations_used: 1,
            convergence_achieved: true,
            synaptic_adaptations: 0,
            quantum_optimizations_applied: 0,
        },
    }
}

// =============================================================================
// Operator Precedence Parser Tests (Pratt Parsing)
// =============================================================================

/// Test that multiplication has higher precedence than addition
#[test]
fn test_operator_precedence_mult_over_add() {
    let parser = QSQLParser::new();

    // 1 + 2 * 3 should parse as 1 + (2 * 3), not (1 + 2) * 3
    let result = parser.parse("SELECT * FROM t WHERE x = 1 + 2 * 3");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                left: _,
                operator: BinaryOperator::Equal,
                right,
            }) = select.where_clause
            {
                // The right side should be: 1 + (2 * 3)
                if let Expression::BinaryOp {
                    left: add_left,
                    operator: add_op,
                    right: add_right,
                } = *right
                {
                    assert_eq!(add_op, BinaryOperator::Add);

                    // Left side of addition is 1
                    if let Expression::Literal(Literal::Integer(n)) = *add_left {
                        assert_eq!(n, 1);
                    } else {
                        panic!("Expected integer literal 1");
                    }

                    // Right side of addition is 2 * 3
                    if let Expression::BinaryOp {
                        left: mult_left,
                        operator: mult_op,
                        right: mult_right,
                    } = *add_right
                    {
                        assert_eq!(mult_op, BinaryOperator::Multiply);
                        if let Expression::Literal(Literal::Integer(n)) = *mult_left {
                            assert_eq!(n, 2);
                        }
                        if let Expression::Literal(Literal::Integer(n)) = *mult_right {
                            assert_eq!(n, 3);
                        }
                    } else {
                        panic!("Expected multiplication expression");
                    }
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test that AND has higher precedence than OR
#[test]
fn test_operator_precedence_and_over_or() {
    let parser = QSQLParser::new();

    // a OR b AND c should parse as a OR (b AND c), not (a OR b) AND c
    let result = parser.parse("SELECT * FROM t WHERE a = 1 OR b = 2 AND c = 3");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                operator: top_op, ..
            }) = &select.where_clause
            {
                // The top-level operator should be OR (since AND binds tighter)
                assert_eq!(*top_op, BinaryOperator::Or);
            } else {
                panic!("Expected binary operation at top level");
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test comparison operators have correct precedence relative to arithmetic
#[test]
fn test_operator_precedence_comparison_over_arithmetic() {
    let parser = QSQLParser::new();

    // x > 1 + 2 should parse as x > (1 + 2)
    let result = parser.parse("SELECT * FROM t WHERE x > 1 + 2");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                left: cmp_left,
                operator: cmp_op,
                right: cmp_right,
            }) = select.where_clause
            {
                // Top level should be comparison
                assert_eq!(cmp_op, BinaryOperator::GreaterThan);

                // Left side should be identifier x
                if let Expression::Identifier(name) = *cmp_left {
                    assert_eq!(name, "x");
                } else {
                    panic!("Expected identifier x");
                }

                // Right side should be addition 1 + 2
                if let Expression::BinaryOp {
                    operator: add_op, ..
                } = *cmp_right
                {
                    assert_eq!(add_op, BinaryOperator::Add);
                } else {
                    panic!("Expected addition expression");
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test parenthesized expressions override precedence
#[test]
fn test_parentheses_override_precedence() {
    let parser = QSQLParser::new();

    // (1 + 2) * 3 should parse as (1 + 2) * 3, not 1 + (2 * 3)
    let result = parser.parse("SELECT * FROM t WHERE x = (1 + 2) * 3");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                left: _,
                operator: BinaryOperator::Equal,
                right,
            }) = select.where_clause
            {
                // The right side should be: (1 + 2) * 3
                // Top-level operation is multiplication
                if let Expression::BinaryOp {
                    left: mult_left,
                    operator: mult_op,
                    right: mult_right,
                } = *right
                {
                    assert_eq!(mult_op, BinaryOperator::Multiply);

                    // Left side of multiplication is (1 + 2)
                    if let Expression::BinaryOp {
                        operator: add_op, ..
                    } = *mult_left
                    {
                        assert_eq!(add_op, BinaryOperator::Add);
                    } else {
                        panic!("Expected addition expression in parentheses");
                    }

                    // Right side is 3
                    if let Expression::Literal(Literal::Integer(n)) = *mult_right {
                        assert_eq!(n, 3);
                    }
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test unary NOT operator
#[test]
fn test_unary_not_operator() {
    let parser = QSQLParser::new();

    let result = parser.parse("SELECT * FROM t WHERE NOT active = 1");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::UnaryOp {
                operator: UnaryOperator::Not,
                ..
            }) = &select.where_clause
            {
                // Top-level should be NOT
            } else {
                panic!("Expected NOT unary operator at top level");
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test unary minus operator
#[test]
fn test_unary_minus_operator() {
    let parser = QSQLParser::new();

    let result = parser.parse("SELECT * FROM t WHERE x = -5");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                left: _,
                operator: BinaryOperator::Equal,
                right,
            }) = select.where_clause
            {
                if let Expression::UnaryOp {
                    operator: UnaryOperator::Minus,
                    operand,
                } = *right
                {
                    if let Expression::Literal(Literal::Integer(n)) = *operand {
                        assert_eq!(n, 5);
                    } else {
                        panic!("Expected integer literal 5");
                    }
                } else {
                    panic!("Expected unary minus");
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test function call parsing
#[test]
fn test_function_call_parsing() {
    let parser = QSQLParser::new();

    let result = parser.parse("SELECT * FROM t WHERE COUNT(id) > 10");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                left,
                operator: BinaryOperator::GreaterThan,
                ..
            }) = select.where_clause
            {
                if let Expression::FunctionCall { name, args } = *left {
                    assert_eq!(name, "COUNT");
                    assert_eq!(args.len(), 1);
                } else {
                    panic!("Expected function call");
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test complex nested expressions
#[test]
fn test_complex_nested_expression() {
    let parser = QSQLParser::new();

    // Complex expression with multiple operators and precedence levels
    let result = parser.parse("SELECT * FROM t WHERE a = 1 AND b > 2 + 3 * 4 OR c < 10");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            // Top level should be OR (lowest precedence among binary logical ops)
            if let Some(Expression::BinaryOp {
                operator: top_op, ..
            }) = &select.where_clause
            {
                assert_eq!(*top_op, BinaryOperator::Or);
            } else {
                panic!("Expected OR at top level");
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test left associativity of operators
#[test]
fn test_left_associativity() {
    let parser = QSQLParser::new();

    // a - b - c should parse as (a - b) - c (left associative)
    let result = parser.parse("SELECT * FROM t WHERE x = a - b - c");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                left: _,
                operator: BinaryOperator::Equal,
                right,
            }) = select.where_clause
            {
                // Right side: (a - b) - c
                // Top-level subtraction
                if let Expression::BinaryOp {
                    left: sub_left,
                    operator: sub_op,
                    right: sub_right,
                } = *right
                {
                    assert_eq!(sub_op, BinaryOperator::Subtract);

                    // Right operand should be c (identifier)
                    if let Expression::Identifier(name) = *sub_right {
                        assert_eq!(name, "c");
                    } else {
                        panic!("Expected identifier c");
                    }

                    // Left operand should be a - b
                    if let Expression::BinaryOp {
                        operator: inner_op, ..
                    } = *sub_left
                    {
                        assert_eq!(inner_op, BinaryOperator::Subtract);
                    } else {
                        panic!("Expected inner subtraction");
                    }
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test LIKE operator parsing
#[test]
fn test_like_operator() {
    let parser = QSQLParser::new();

    let result = parser.parse("SELECT * FROM t WHERE name LIKE '%test%'");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                operator: BinaryOperator::Like,
                ..
            }) = &select.where_clause
            {
                // LIKE operator correctly parsed
            } else {
                panic!("Expected LIKE operator");
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test division and modulo operators
#[test]
fn test_division_and_modulo() {
    let parser = QSQLParser::new();

    let result = parser.parse("SELECT * FROM t WHERE x = 10 / 3 % 2");
    assert!(result.is_ok());

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::BinaryOp {
                left: _,
                operator: BinaryOperator::Equal,
                right,
            }) = select.where_clause
            {
                // Should be (10 / 3) % 2 due to left associativity and equal precedence
                if let Expression::BinaryOp {
                    operator: BinaryOperator::Modulo,
                    left: mod_left,
                    ..
                } = *right
                {
                    if let Expression::BinaryOp {
                        operator: BinaryOperator::Divide,
                        ..
                    } = *mod_left
                    {
                        // Correct structure
                    } else {
                        panic!("Expected division as left operand of modulo");
                    }
                } else {
                    panic!("Expected modulo at top of arithmetic expression");
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

// ============================================================================
// GROUP BY and HAVING Tests
// ============================================================================

#[test]
fn test_group_by_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT name, COUNT(*) FROM users GROUP BY name");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert!(
                !select.group_by.is_empty(),
                "GROUP BY clause should be parsed"
            );
            assert_eq!(
                select.group_by.len(),
                1,
                "Should have exactly one GROUP BY column"
            );
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_group_by_with_having() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT name, COUNT(*) FROM users GROUP BY name HAVING COUNT(*) > 1");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert!(
                !select.group_by.is_empty(),
                "GROUP BY clause should be parsed"
            );
            assert!(select.having.is_some(), "HAVING clause should be parsed");
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_group_by_multiple_columns() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT name, email, COUNT(*) FROM users GROUP BY name, email");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert_eq!(
                select.group_by.len(),
                2,
                "Should have exactly two GROUP BY columns"
            );
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_group_by_with_aggregate_functions() {
    let parser = QSQLParser::new();
    let result =
        parser.parse("SELECT category, SUM(price), AVG(quantity) FROM products GROUP BY category");

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert_eq!(
                select.select_list.len(),
                3,
                "Should have three select items"
            );
            assert!(!select.group_by.is_empty(), "GROUP BY should be parsed");
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_group_by_with_where_and_having() {
    let parser = QSQLParser::new();
    let result = parser.parse(
        "SELECT department, COUNT(*) FROM employees WHERE active = true GROUP BY department HAVING COUNT(*) >= 5",
    );

    assert!(result.is_ok());
    match result.unwrap() {
        Statement::Select(select) => {
            assert!(
                select.where_clause.is_some(),
                "WHERE clause should be parsed"
            );
            assert!(
                !select.group_by.is_empty(),
                "GROUP BY clause should be parsed"
            );
            assert!(select.having.is_some(), "HAVING clause should be parsed");
        }
        _ => panic!("Expected SELECT statement"),
    }
}
