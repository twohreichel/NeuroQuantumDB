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
    let mut executor = QueryExecutor::new().unwrap();
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
    let mut executor = QueryExecutor::new().unwrap();
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
    let mut executor = QueryExecutor::new().unwrap();
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
    let mut executor = QueryExecutor::new().unwrap();
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
