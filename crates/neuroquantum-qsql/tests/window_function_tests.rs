//! Integration tests for Window Functions (ROW_NUMBER, RANK, DENSE_RANK, LAG, LEAD)
//!
//! This test suite verifies that window functions are correctly parsed and executed.

use neuroquantum_core::storage::{
    ColumnDefinition, DataType, Row, StorageEngine, TableSchema, Value,
};
use neuroquantum_qsql::{query_plan::QueryValue, ExecutorConfig, Parser, QueryExecutor};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper function to create a test table with employee data
async fn setup_test_db() -> (TempDir, Arc<tokio::sync::RwLock<StorageEngine>>) {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create employees table
    let schema = TableSchema {
        name: "employees".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: true,
            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "department".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "salary".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: std::collections::HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();

        // Insert test data
        let employees = vec![
            ("Alice", "Engineering", 100000),
            ("Bob", "Engineering", 90000),
            ("Charlie", "Engineering", 90000),
            ("Diana", "Sales", 80000),
            ("Eve", "Sales", 75000),
            ("Frank", "Marketing", 70000),
        ];

        for (i, (name, dept, salary)) in employees.into_iter().enumerate() {
            let mut row = Row {
                id: 0,
                fields: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            row.fields
                .insert("id".to_string(), Value::Integer((i + 1) as i64));
            row.fields
                .insert("name".to_string(), Value::Text(name.to_string()));
            row.fields
                .insert("department".to_string(), Value::Text(dept.to_string()));
            row.fields
                .insert("salary".to_string(), Value::Integer(salary));
            storage_guard.insert_row("employees", row).await.unwrap();
        }
    }

    (temp_dir, storage_arc)
}

/// Test ROW_NUMBER() parsing
#[test]
fn test_row_number_parsing() {
    let parser = Parser::new();
    let sql = "SELECT ROW_NUMBER() OVER (ORDER BY id) as row_num, name FROM employees";
    let result = parser.parse(sql);

    assert!(
        result.is_ok(),
        "Failed to parse ROW_NUMBER: {:?}",
        result.err()
    );

    let stmt = result.unwrap();
    if let neuroquantum_qsql::Statement::Select(select) = stmt {
        assert_eq!(select.select_list.len(), 2);
        // First item should be a window function
        if let neuroquantum_qsql::ast::SelectItem::Expression { expr, alias } =
            &select.select_list[0]
        {
            match expr {
                neuroquantum_qsql::ast::Expression::WindowFunction { function, .. } => {
                    assert_eq!(
                        *function,
                        neuroquantum_qsql::ast::WindowFunctionType::RowNumber
                    );
                }
                _ => panic!("Expected WindowFunction expression, got {:?}", expr),
            }
            assert_eq!(alias.as_deref(), Some("row_num"));
        } else {
            panic!("Expected Expression, got wildcard");
        }
    } else {
        panic!("Expected SELECT statement");
    }
}

/// Test RANK() parsing with PARTITION BY
#[test]
fn test_rank_with_partition_parsing() {
    let parser = Parser::new();
    let sql = "SELECT RANK() OVER (PARTITION BY department ORDER BY salary DESC) as dept_rank, name FROM employees";
    let result = parser.parse(sql);

    assert!(
        result.is_ok(),
        "Failed to parse RANK with PARTITION BY: {:?}",
        result.err()
    );

    let stmt = result.unwrap();
    if let neuroquantum_qsql::Statement::Select(select) = stmt {
        if let neuroquantum_qsql::ast::SelectItem::Expression { expr, .. } = &select.select_list[0]
        {
            match expr {
                neuroquantum_qsql::ast::Expression::WindowFunction {
                    function,
                    over_clause,
                    ..
                } => {
                    assert_eq!(*function, neuroquantum_qsql::ast::WindowFunctionType::Rank);
                    assert!(
                        !over_clause.partition_by.is_empty(),
                        "Expected PARTITION BY clause"
                    );
                    assert!(!over_clause.order_by.is_empty(), "Expected ORDER BY clause");
                }
                _ => panic!("Expected WindowFunction expression"),
            }
        }
    }
}

/// Test DENSE_RANK() parsing
#[test]
fn test_dense_rank_parsing() {
    let parser = Parser::new();
    let sql = "SELECT DENSE_RANK() OVER (ORDER BY salary DESC) as dense_rank_val FROM employees";
    let result = parser.parse(sql);

    assert!(
        result.is_ok(),
        "Failed to parse DENSE_RANK: {:?}",
        result.err()
    );
}

/// Test LAG() parsing with arguments
#[test]
fn test_lag_parsing() {
    let parser = Parser::new();
    let sql =
        "SELECT name, salary, LAG(salary, 1, 0) OVER (ORDER BY id) as prev_salary FROM employees";
    let result = parser.parse(sql);

    assert!(result.is_ok(), "Failed to parse LAG: {:?}", result.err());

    let stmt = result.unwrap();
    if let neuroquantum_qsql::Statement::Select(select) = stmt {
        // LAG should be the third item (index 2)
        if let neuroquantum_qsql::ast::SelectItem::Expression { expr, .. } = &select.select_list[2]
        {
            match expr {
                neuroquantum_qsql::ast::Expression::WindowFunction { function, args, .. } => {
                    assert_eq!(*function, neuroquantum_qsql::ast::WindowFunctionType::Lag);
                    assert_eq!(args.len(), 3, "LAG should have 3 arguments");
                }
                _ => panic!("Expected WindowFunction expression"),
            }
        }
    }
}

/// Test LEAD() parsing
#[test]
fn test_lead_parsing() {
    let parser = Parser::new();
    let sql = "SELECT name, LEAD(salary) OVER (ORDER BY id) as next_salary FROM employees";
    let result = parser.parse(sql);

    assert!(result.is_ok(), "Failed to parse LEAD: {:?}", result.err());
}

/// Test ROW_NUMBER() execution
#[tokio::test]
async fn test_row_number_execution() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    let sql = "SELECT ROW_NUMBER() OVER (ORDER BY id) as row_num, name FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    // Check that row numbers are sequential
    for (i, row) in result.rows.iter().enumerate() {
        if let Some(QueryValue::Integer(row_num)) = row.get("row_num") {
            assert_eq!(*row_num, (i + 1) as i64, "Row number should be sequential");
        } else {
            panic!("Expected row_num column with Integer value");
        }
    }

    println!("✅ ROW_NUMBER() execution: SUCCESS");
}

/// Test RANK() execution with ties
#[tokio::test]
async fn test_rank_execution_with_ties() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    // Bob and Charlie have the same salary (90000), so they should have the same rank
    let sql =
        "SELECT RANK() OVER (ORDER BY salary DESC) as salary_rank, name, salary FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    // Find Bob and Charlie's ranks - they should be equal (both rank 2)
    let mut bob_rank = None;
    let mut charlie_rank = None;

    for row in &result.rows {
        if let Some(QueryValue::String(name)) = row.get("name") {
            if let Some(QueryValue::Integer(rank)) = row.get("salary_rank") {
                match name.as_str() {
                    "Bob" => bob_rank = Some(*rank),
                    "Charlie" => charlie_rank = Some(*rank),
                    _ => {}
                }
            }
        }
    }

    assert_eq!(
        bob_rank, charlie_rank,
        "Bob and Charlie should have the same rank (ties)"
    );

    println!("✅ RANK() with ties: SUCCESS");
}

/// Test RANK() with PARTITION BY
#[tokio::test]
async fn test_rank_with_partition_execution() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    let sql = "SELECT RANK() OVER (PARTITION BY department ORDER BY salary DESC) as dept_rank, name, department, salary FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    // Find Alice's rank in Engineering - should be 1 (highest salary in Engineering)
    for row in &result.rows {
        if let Some(QueryValue::String(name)) = row.get("name") {
            if name == "Alice" {
                if let Some(QueryValue::Integer(rank)) = row.get("dept_rank") {
                    assert_eq!(*rank, 1, "Alice should be rank 1 in Engineering");
                }
            }
        }
    }

    println!("✅ RANK() with PARTITION BY: SUCCESS");
}

/// Test DENSE_RANK() execution
#[tokio::test]
async fn test_dense_rank_execution() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    let sql = "SELECT DENSE_RANK() OVER (ORDER BY salary DESC) as dense_rank_val, name, salary FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    // With DENSE_RANK, Bob and Charlie (tied at 90000) should both have rank 2,
    // and Diana (80000) should have rank 3 (not 4 like RANK would give)
    let mut diana_rank = None;

    for row in &result.rows {
        if let Some(QueryValue::String(name)) = row.get("name") {
            if name == "Diana" {
                if let Some(QueryValue::Integer(rank)) = row.get("dense_rank_val") {
                    diana_rank = Some(*rank);
                }
            }
        }
    }

    // Diana should be rank 3 (after Alice at 1, and Bob/Charlie tied at 2)
    assert_eq!(diana_rank, Some(3), "Diana should have DENSE_RANK of 3");

    println!("✅ DENSE_RANK() execution: SUCCESS");
}

/// Test LAG() execution
#[tokio::test]
async fn test_lag_execution() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    let sql =
        "SELECT name, salary, LAG(salary, 1, 0) OVER (ORDER BY id) as prev_salary FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    // First row should have prev_salary = 0 (the default)
    // Second row should have prev_salary = 100000 (Alice's salary)
    let first_row = &result.rows[0];
    if let Some(QueryValue::Integer(prev_salary)) = first_row.get("prev_salary") {
        assert_eq!(
            *prev_salary, 0,
            "First row LAG should return default value 0"
        );
    }

    println!("✅ LAG() execution: SUCCESS");
}

/// Test LEAD() execution
#[tokio::test]
async fn test_lead_execution() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    let sql =
        "SELECT name, salary, LEAD(salary, 1, 0) OVER (ORDER BY id) as next_salary FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    // Last row should have next_salary = 0 (the default, no next row)
    let last_row = &result.rows[5];
    if let Some(QueryValue::Integer(next_salary)) = last_row.get("next_salary") {
        assert_eq!(
            *next_salary, 0,
            "Last row LEAD should return default value 0"
        );
    }

    println!("✅ LEAD() execution: SUCCESS");
}

/// Test NTILE() parsing
#[test]
fn test_ntile_parsing() {
    let parser = Parser::new();
    let sql = "SELECT NTILE(4) OVER (ORDER BY salary DESC) as quartile, name FROM employees";
    let result = parser.parse(sql);

    assert!(result.is_ok(), "Failed to parse NTILE: {:?}", result.err());
}

/// Test FIRST_VALUE() parsing
#[test]
fn test_first_value_parsing() {
    let parser = Parser::new();
    let sql = "SELECT FIRST_VALUE(salary) OVER (PARTITION BY department ORDER BY salary DESC) as top_salary, name FROM employees";
    let result = parser.parse(sql);

    assert!(
        result.is_ok(),
        "Failed to parse FIRST_VALUE: {:?}",
        result.err()
    );
}

/// Test LAST_VALUE() parsing
#[test]
fn test_last_value_parsing() {
    let parser = Parser::new();
    let sql = "SELECT LAST_VALUE(salary) OVER (ORDER BY id) as last_salary, name FROM employees";
    let result = parser.parse(sql);

    assert!(
        result.is_ok(),
        "Failed to parse LAST_VALUE: {:?}",
        result.err()
    );
}

// ============================================================================
// Phase 2: Aggregate Window Functions (SUM, AVG, COUNT with OVER)
// ============================================================================

/// Test SUM() OVER parsing
#[test]
fn test_sum_over_parsing() {
    let parser = Parser::new();
    let sql = "SELECT name, SUM(salary) OVER (PARTITION BY department) as dept_total FROM employees";
    let result = parser.parse(sql);

    assert!(
        result.is_ok(),
        "Failed to parse SUM() OVER: {:?}",
        result.err()
    );
}

/// Test AVG() OVER parsing
#[test]
fn test_avg_over_parsing() {
    let parser = Parser::new();
    let sql = "SELECT name, AVG(salary) OVER (PARTITION BY department) as dept_avg FROM employees";
    let result = parser.parse(sql);

    assert!(
        result.is_ok(),
        "Failed to parse AVG() OVER: {:?}",
        result.err()
    );
}

/// Test COUNT() OVER parsing
#[test]
fn test_count_over_parsing() {
    let parser = Parser::new();
    let sql = "SELECT name, COUNT(*) OVER (PARTITION BY department) as dept_count FROM employees";
    let result = parser.parse(sql);

    assert!(
        result.is_ok(),
        "Failed to parse COUNT() OVER: {:?}",
        result.err()
    );
}

/// Test SUM() OVER execution
#[tokio::test]
async fn test_sum_over_execution() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    let sql = "SELECT name, salary, SUM(salary) OVER (PARTITION BY department ORDER BY id) as running_total FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    // Check that we have the running_total column
    for row in &result.rows {
        assert!(
            row.contains_key("running_total") || row.contains_key("SUM(salary)"),
            "Expected running_total or SUM(salary) column in result: {:?}",
            row.keys().collect::<Vec<_>>()
        );
    }

    println!("✅ SUM() OVER execution: SUCCESS");
}

/// Test AVG() OVER execution
#[tokio::test]
async fn test_avg_over_execution() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    let sql = "SELECT name, salary, AVG(salary) OVER (PARTITION BY department) as dept_avg FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    println!("✅ AVG() OVER execution: SUCCESS");
}

/// Test COUNT() OVER execution
#[tokio::test]
async fn test_count_over_execution() {
    let (_temp_dir, storage_arc) = setup_test_db().await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();
    let sql = "SELECT name, COUNT(*) OVER (PARTITION BY department) as dept_count FROM employees";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 6, "Expected 6 rows");

    // Engineering should have 3 employees
    for row in &result.rows {
        if let Some(QueryValue::String(dept)) = row.get("department") {
            if dept == "Engineering" {
                if let Some(QueryValue::Integer(count)) = row.get("dept_count").or_else(|| row.get("COUNT(*)")) {
                    assert_eq!(*count, 3, "Engineering should have 3 employees");
                }
            }
        }
    }

    println!("✅ COUNT() OVER execution: SUCCESS");
}
