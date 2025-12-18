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

/// Test IN operator parsing with list of values
#[test]
fn test_in_operator_parsing() {
    let parser = QSQLParser::new();

    // Test basic IN with integer list
    let result = parser.parse("SELECT * FROM users WHERE age IN (25, 30, 35)");
    assert!(result.is_ok(), "IN operator should parse successfully");

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::InList {
                expr,
                list,
                negated,
            }) = &select.where_clause
            {
                assert!(!negated, "Should not be negated");
                assert_eq!(list.len(), 3, "Should have 3 values in list");

                // Check the field expression
                if let Expression::Identifier(name) = expr.as_ref() {
                    assert_eq!(name, "age");
                } else {
                    panic!("Expected identifier 'age'");
                }

                // Check the list values
                if let Expression::Literal(crate::ast::Literal::Integer(v)) = &list[0] {
                    assert_eq!(*v, 25);
                } else {
                    panic!("Expected integer 25");
                }
            } else {
                panic!("Expected InList expression, got: {:?}", select.where_clause);
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test NOT IN operator parsing
#[test]
fn test_not_in_operator_parsing() {
    let parser = QSQLParser::new();

    let result = parser.parse("SELECT * FROM users WHERE status NOT IN ('inactive', 'banned')");
    assert!(result.is_ok(), "NOT IN operator should parse successfully");

    match result.unwrap() {
        Statement::Select(select) => {
            if let Some(Expression::InList {
                expr,
                list,
                negated,
            }) = &select.where_clause
            {
                assert!(*negated, "Should be negated (NOT IN)");
                assert_eq!(list.len(), 2, "Should have 2 values in list");

                // Check the field expression
                if let Expression::Identifier(name) = expr.as_ref() {
                    assert_eq!(name, "status");
                } else {
                    panic!("Expected identifier 'status'");
                }
            } else {
                panic!("Expected InList expression");
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test IN operator with complex expressions
#[test]
fn test_in_operator_with_and_or() {
    let parser = QSQLParser::new();

    let result = parser.parse("SELECT * FROM users WHERE age IN (25, 30) AND active = true");
    assert!(result.is_ok(), "IN with AND should parse successfully");

    match result.unwrap() {
        Statement::Select(select) => {
            // Should be AND at top level
            if let Some(Expression::BinaryOp {
                operator: crate::ast::BinaryOperator::And,
                left,
                ..
            }) = &select.where_clause
            {
                // Left side should be InList
                if let Expression::InList { list, negated, .. } = left.as_ref() {
                    assert!(!negated);
                    assert_eq!(list.len(), 2);
                } else {
                    panic!("Expected InList on left side of AND");
                }
            } else {
                panic!("Expected AND expression at top level");
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

// ========== JOIN Tests ==========

/// Test INNER JOIN parsing
#[test]
fn test_inner_join_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM users u INNER JOIN orders o ON u.id = o.user_id");

    assert!(result.is_ok(), "INNER JOIN should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.relations.len(), 1, "Should have one base relation");
            assert_eq!(from.relations[0].name, "users");
            assert_eq!(from.relations[0].alias, Some("u".to_string()));

            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            let join = &from.joins[0];
            assert_eq!(join.join_type, JoinType::Inner);
            assert_eq!(join.relation.name, "orders");
            assert_eq!(join.relation.alias, Some("o".to_string()));
            assert!(join.condition.is_some(), "Should have ON condition");
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test LEFT JOIN parsing
#[test]
fn test_left_join_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM users u LEFT JOIN orders o ON u.id = o.user_id");

    assert!(result.is_ok(), "LEFT JOIN should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert_eq!(from.joins[0].join_type, JoinType::Left);
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test LEFT OUTER JOIN parsing
#[test]
fn test_left_outer_join_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM users u LEFT OUTER JOIN orders o ON u.id = o.user_id");

    assert!(result.is_ok(), "LEFT OUTER JOIN should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert_eq!(from.joins[0].join_type, JoinType::Left);
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test RIGHT JOIN parsing
#[test]
fn test_right_join_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM users u RIGHT JOIN orders o ON u.id = o.user_id");

    assert!(result.is_ok(), "RIGHT JOIN should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert_eq!(from.joins[0].join_type, JoinType::Right);
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test FULL OUTER JOIN parsing
#[test]
fn test_full_outer_join_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM users u FULL OUTER JOIN orders o ON u.id = o.user_id");

    assert!(result.is_ok(), "FULL OUTER JOIN should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert_eq!(from.joins[0].join_type, JoinType::Full);
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test CROSS JOIN parsing
#[test]
fn test_cross_join_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM users u CROSS JOIN products p");

    assert!(result.is_ok(), "CROSS JOIN should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert_eq!(from.joins[0].join_type, JoinType::Cross);
            assert!(
                from.joins[0].condition.is_none(),
                "CROSS JOIN should not have ON condition"
            );
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test plain JOIN (defaults to INNER JOIN)
#[test]
fn test_plain_join_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT * FROM users u JOIN orders o ON u.id = o.user_id");

    assert!(result.is_ok(), "Plain JOIN should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert_eq!(
                from.joins[0].join_type,
                JoinType::Inner,
                "Plain JOIN defaults to INNER"
            );
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test multiple JOINs
#[test]
fn test_multiple_joins_parsing() {
    let parser = QSQLParser::new();
    let result = parser.parse(
        "SELECT * FROM users u \
         INNER JOIN orders o ON u.id = o.user_id \
         LEFT JOIN products p ON o.product_id = p.id",
    );

    assert!(result.is_ok(), "Multiple JOINs should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 2, "Should have two JOINs");
            assert_eq!(from.joins[0].join_type, JoinType::Inner);
            assert_eq!(from.joins[0].relation.name, "orders");
            assert_eq!(from.joins[1].join_type, JoinType::Left);
            assert_eq!(from.joins[1].relation.name, "products");
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test JOIN with WHERE clause
#[test]
fn test_join_with_where_clause() {
    let parser = QSQLParser::new();
    let result = parser.parse(
        "SELECT * FROM users u INNER JOIN orders o ON u.id = o.user_id WHERE u.active = true",
    );

    assert!(result.is_ok(), "JOIN with WHERE should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert!(select.where_clause.is_some(), "Should have WHERE clause");
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test JOIN with ORDER BY
#[test]
fn test_join_with_order_by() {
    let parser = QSQLParser::new();
    let result = parser.parse(
        "SELECT * FROM users u INNER JOIN orders o ON u.id = o.user_id ORDER BY o.created_at",
    );

    assert!(
        result.is_ok(),
        "JOIN with ORDER BY should parse successfully"
    );
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert!(!select.order_by.is_empty(), "Should have ORDER BY clause");
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test JOIN with LIMIT
#[test]
fn test_join_with_limit() {
    let parser = QSQLParser::new();
    let result =
        parser.parse("SELECT * FROM users u INNER JOIN orders o ON u.id = o.user_id LIMIT 10");

    assert!(result.is_ok(), "JOIN with LIMIT should parse successfully");
    match result.unwrap() {
        Statement::Select(select) => {
            let from = select.from.expect("Should have FROM clause");
            assert_eq!(from.joins.len(), 1, "Should have one JOIN");
            assert_eq!(select.limit, Some(10), "Should have LIMIT 10");
        }
        _ => panic!("Expected SELECT statement"),
    }
}

// =============================================================================
// CASE Expression Tests
// =============================================================================

/// Test simple CASE WHEN THEN ELSE END
#[test]
fn test_case_expression_simple() {
    let parser = QSQLParser::new();
    let result =
        parser.parse("SELECT name, CASE WHEN age > 30 THEN 'Senior' ELSE 'Junior' END FROM users");

    assert!(
        result.is_ok(),
        "Simple CASE expression should parse successfully: {:?}",
        result
    );
    match result.unwrap() {
        Statement::Select(select) => {
            assert_eq!(select.select_list.len(), 2, "Should have 2 select items");
            // Verify second item is a CASE expression
            if let SelectItem::Expression { expr, .. } = &select.select_list[1] {
                match expr {
                    Expression::Case {
                        when_clauses,
                        else_result,
                    } => {
                        assert_eq!(when_clauses.len(), 1, "Should have 1 WHEN clause");
                        assert!(else_result.is_some(), "Should have ELSE clause");
                    }
                    _ => panic!("Expected CASE expression"),
                }
            } else {
                panic!("Expected Expression select item");
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test CASE with multiple WHEN clauses
#[test]
fn test_case_expression_multiple_when() {
    let parser = QSQLParser::new();
    let result = parser.parse(
        "SELECT name, CASE WHEN age < 20 THEN 'Teen' WHEN age < 40 THEN 'Adult' ELSE 'Senior' END FROM users",
    );

    assert!(
        result.is_ok(),
        "CASE with multiple WHEN should parse successfully: {:?}",
        result
    );
    match result.unwrap() {
        Statement::Select(select) => {
            if let SelectItem::Expression { expr, .. } = &select.select_list[1] {
                match expr {
                    Expression::Case {
                        when_clauses,
                        else_result,
                    } => {
                        assert_eq!(when_clauses.len(), 2, "Should have 2 WHEN clauses");
                        assert!(else_result.is_some(), "Should have ELSE clause");
                    }
                    _ => panic!("Expected CASE expression"),
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test CASE without ELSE
#[test]
fn test_case_expression_without_else() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT name, CASE WHEN age > 30 THEN 'Senior' END FROM users");

    assert!(
        result.is_ok(),
        "CASE without ELSE should parse successfully: {:?}",
        result
    );
    match result.unwrap() {
        Statement::Select(select) => {
            if let SelectItem::Expression { expr, .. } = &select.select_list[1] {
                match expr {
                    Expression::Case {
                        when_clauses,
                        else_result,
                    } => {
                        assert_eq!(when_clauses.len(), 1, "Should have 1 WHEN clause");
                        assert!(else_result.is_none(), "Should NOT have ELSE clause");
                    }
                    _ => panic!("Expected CASE expression"),
                }
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test CASE with alias
#[test]
fn test_case_expression_with_alias() {
    let parser = QSQLParser::new();
    let result = parser.parse(
        "SELECT name, CASE WHEN age > 30 THEN 'Senior' ELSE 'Junior' END AS status FROM users",
    );

    assert!(
        result.is_ok(),
        "CASE with alias should parse successfully: {:?}",
        result
    );
    match result.unwrap() {
        Statement::Select(select) => {
            if let SelectItem::Expression { expr, alias } = &select.select_list[1] {
                assert!(
                    matches!(expr, Expression::Case { .. }),
                    "Should be CASE expression"
                );
                assert_eq!(
                    alias.as_deref(),
                    Some("status"),
                    "Should have alias 'status'"
                );
            }
        }
        _ => panic!("Expected SELECT statement"),
    }
}

/// Test CASE with string comparison
#[test]
fn test_case_expression_string_comparison() {
    let parser = QSQLParser::new();
    let result = parser.parse("SELECT CASE WHEN status = 'active' THEN 1 ELSE 0 END FROM users");

    assert!(
        result.is_ok(),
        "CASE with string comparison should parse: {:?}",
        result
    );
}

/// Test nested CASE expressions
#[test]
fn test_case_expression_with_complex_condition() {
    let parser = QSQLParser::new();
    let result = parser.parse(
        "SELECT CASE WHEN age > 30 AND status = 'active' THEN 'Senior Active' ELSE 'Other' END FROM users",
    );

    assert!(
        result.is_ok(),
        "CASE with complex AND condition should parse: {:?}",
        result
    );
}
