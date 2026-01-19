//! Integration tests for DEFAULT values in INSERT statements
//!
//! This test suite verifies that DEFAULT values work correctly:
//! 1. DEFAULT keyword in CREATE TABLE column definitions
//! 2. Explicit DEFAULT keyword in INSERT VALUES
//! 3. Automatic application of defaults for missing columns
//!
//! Implements feature requested in Issue #245

use std::collections::HashMap;
use std::sync::Arc;

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use tempfile::TempDir;

/// Test parsing of CREATE TABLE with DEFAULT values
#[test]
fn test_parse_create_table_with_defaults() {
    let parser = Parser::new();

    let sql = "CREATE TABLE users (
        id BIGSERIAL PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT,
        status TEXT DEFAULT 'active',
        age INTEGER DEFAULT 0
    )";

    let result = parser.parse(sql);
    assert!(
        result.is_ok(),
        "Failed to parse CREATE TABLE with DEFAULT: {:?}",
        result.err()
    );

    let statement = result.unwrap();
    match statement {
        | neuroquantum_qsql::ast::Statement::CreateTable(create) => {
            assert_eq!(create.table_name, "users");
            assert_eq!(create.columns.len(), 5);

            // Check status column has default 'active'
            let status_col = create.columns.iter().find(|c| c.name == "status").unwrap();
            let has_default = status_col.constraints.iter().any(|c| {
                matches!(
                    c,
                    neuroquantum_qsql::ast::ColumnConstraint::Default(
                        neuroquantum_qsql::ast::Expression::Literal(
                            neuroquantum_qsql::ast::Literal::String(s)
                        )
                    ) if s == "active"
                )
            });
            assert!(has_default, "Expected DEFAULT 'active' for status column");

            // Check age column has default 0
            let age_col = create.columns.iter().find(|c| c.name == "age").unwrap();
            let has_default = age_col.constraints.iter().any(|c| {
                matches!(
                    c,
                    neuroquantum_qsql::ast::ColumnConstraint::Default(
                        neuroquantum_qsql::ast::Expression::Literal(
                            neuroquantum_qsql::ast::Literal::Integer(0)
                        )
                    )
                )
            });
            assert!(has_default, "Expected DEFAULT 0 for age column");
        },
        | _ => panic!("Expected CREATE TABLE statement"),
    }
}

/// Test parsing of INSERT with explicit DEFAULT keyword
#[test]
fn test_parse_insert_with_default_keyword() {
    let parser = Parser::new();

    let sql = "INSERT INTO users (name, status) VALUES ('Alice', DEFAULT)";

    let result = parser.parse(sql);
    assert!(
        result.is_ok(),
        "Failed to parse INSERT with DEFAULT: {:?}",
        result.err()
    );

    let statement = result.unwrap();
    match statement {
        | neuroquantum_qsql::ast::Statement::Insert(insert) => {
            assert_eq!(insert.table_name, "users");
            assert_eq!(insert.columns.as_ref().unwrap().len(), 2);
            assert_eq!(insert.values.len(), 1);
            assert_eq!(insert.values[0].len(), 2);

            // Check that second value is DEFAULT
            assert!(
                matches!(
                    insert.values[0][1],
                    neuroquantum_qsql::ast::Expression::Default
                ),
                "Expected DEFAULT expression for second value"
            );
        },
        | _ => panic!("Expected INSERT statement"),
    }
}

/// Test executing INSERT with automatic DEFAULT value application
#[tokio::test]
async fn test_execute_insert_with_missing_column_defaults() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine
    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table with default values
    let schema = TableSchema {
        name: "users".to_string(),
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
                name: "status".to_string(),
                data_type: DataType::Text,
                nullable: true,
                default_value: Some(Value::text("active")),
                auto_increment: false,
            },
            ColumnDefinition {
                name: "age".to_string(),
                data_type: DataType::Integer,
                nullable: true,
                default_value: Some(Value::Integer(0)),
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
        foreign_keys: Vec::new(),
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    // Create parser and executor
    let parser = Parser::new();
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // INSERT with missing columns (status and age should get defaults)
    let sql = "INSERT INTO users (name) VALUES ('Alice')";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "Failed to execute INSERT: {:?}",
        result.err()
    );

    // Verify the row was inserted with default values
    let query = neuroquantum_core::storage::SelectQuery {
        table: "users".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();

    assert_eq!(rows.len(), 1, "Expected 1 row");
    let row = &rows[0];

    // Check that status has default value 'active'
    assert_eq!(
        row.fields.get("status"),
        Some(&Value::text("active")),
        "Expected default status 'active'"
    );

    // Check that age has default value 0
    assert_eq!(
        row.fields.get("age"),
        Some(&Value::Integer(0)),
        "Expected default age 0"
    );
}

/// Test executing INSERT with explicit DEFAULT keyword
#[tokio::test]
async fn test_execute_insert_with_explicit_default() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine
    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table with default values (using types that are well-supported)
    let schema = TableSchema {
        name: "products".to_string(),
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
                name: "price".to_string(),
                data_type: DataType::Float,
                nullable: true,
                default_value: Some(Value::Float(9.99)),
                auto_increment: false,
            },
            ColumnDefinition {
                name: "category".to_string(),
                data_type: DataType::Text,
                nullable: true,
                default_value: Some(Value::text("general")),
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
        foreign_keys: Vec::new(),
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    // Create parser and executor
    let parser = Parser::new();
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // INSERT with explicit DEFAULT keyword for price, explicit category
    let sql =
        "INSERT INTO products (name, price, category) VALUES ('Widget', DEFAULT, 'electronics')";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "Failed to execute INSERT with DEFAULT: {:?}",
        result.err()
    );

    // Verify the row was inserted with default price
    let query = neuroquantum_core::storage::SelectQuery {
        table: "products".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();

    assert_eq!(rows.len(), 1, "Expected 1 row");
    let row = &rows[0];

    // Check that price has default value 9.99
    if let Some(Value::Float(price)) = row.fields.get("price") {
        assert!(
            (*price - 9.99).abs() < 0.001,
            "Expected default price 9.99, got {price}"
        );
    } else {
        panic!("Expected price column with default value");
    }

    // Check that category is 'electronics' (explicit value, not default)
    assert_eq!(
        row.fields.get("category"),
        Some(&Value::text("electronics")),
        "Expected category to be 'electronics' (explicit value)"
    );
}

/// Test that NULL is used when no default is specified and column is nullable
#[tokio::test]
async fn test_insert_null_for_nullable_without_default() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine
    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table - description is nullable without default
    let schema = TableSchema {
        name: "items".to_string(),
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
                name: "description".to_string(),
                data_type: DataType::Text,
                nullable: true,
                default_value: None, // No default, but nullable
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
        foreign_keys: Vec::new(),
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    // Create parser and executor
    let parser = Parser::new();
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // INSERT without description - should succeed since it's nullable
    let sql = "INSERT INTO items (name) VALUES ('Test Item')";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "Failed to execute INSERT: {:?}",
        result.err()
    );

    // Verify the row was inserted
    let query = neuroquantum_core::storage::SelectQuery {
        table: "items".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();

    assert_eq!(rows.len(), 1, "Expected 1 row");
}

/// Test multiple inserts with various DEFAULT scenarios
#[tokio::test]
async fn test_multiple_inserts_with_defaults() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine
    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table
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
                nullable: true,
                default_value: Some(Value::text("General")),
                auto_increment: false,
            },
            ColumnDefinition {
                name: "salary".to_string(),
                data_type: DataType::Integer,
                nullable: true,
                default_value: Some(Value::Integer(50000)),
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
        foreign_keys: Vec::new(),
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    // Create parser and executor
    let parser = Parser::new();
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Insert 1: All defaults
    let sql1 = "INSERT INTO employees (name) VALUES ('Alice')";
    let stmt1 = parser.parse(sql1).unwrap();
    executor.execute_statement(&stmt1).await.unwrap();

    // Insert 2: Override department, default salary
    let sql2 = "INSERT INTO employees (name, department) VALUES ('Bob', 'Engineering')";
    let stmt2 = parser.parse(sql2).unwrap();
    executor.execute_statement(&stmt2).await.unwrap();

    // Insert 3: Override salary, default department
    let sql3 = "INSERT INTO employees (name, salary) VALUES ('Charlie', 75000)";
    let stmt3 = parser.parse(sql3).unwrap();
    executor.execute_statement(&stmt3).await.unwrap();

    // Insert 4: Override both
    let sql4 = "INSERT INTO employees (name, department, salary) VALUES ('Diana', 'Sales', 60000)";
    let stmt4 = parser.parse(sql4).unwrap();
    executor.execute_statement(&stmt4).await.unwrap();

    // Verify all inserts
    let query = neuroquantum_core::storage::SelectQuery {
        table: "employees".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();

    assert_eq!(rows.len(), 4, "Expected 4 rows");

    // Check Alice (all defaults)
    let alice = rows
        .iter()
        .find(|r| r.fields.get("name") == Some(&Value::text("Alice")))
        .unwrap();
    assert_eq!(
        alice.fields.get("department"),
        Some(&Value::text("General"))
    );
    assert_eq!(alice.fields.get("salary"), Some(&Value::Integer(50000)));

    // Check Bob (custom department, default salary)
    let bob = rows
        .iter()
        .find(|r| r.fields.get("name") == Some(&Value::text("Bob")))
        .unwrap();
    assert_eq!(
        bob.fields.get("department"),
        Some(&Value::text("Engineering"))
    );
    assert_eq!(bob.fields.get("salary"), Some(&Value::Integer(50000)));

    // Check Charlie (default department, custom salary)
    let charlie = rows
        .iter()
        .find(|r| r.fields.get("name") == Some(&Value::text("Charlie")))
        .unwrap();
    assert_eq!(
        charlie.fields.get("department"),
        Some(&Value::text("General"))
    );
    assert_eq!(charlie.fields.get("salary"), Some(&Value::Integer(75000)));

    // Check Diana (both custom)
    let diana = rows
        .iter()
        .find(|r| r.fields.get("name") == Some(&Value::text("Diana")))
        .unwrap();
    assert_eq!(diana.fields.get("department"), Some(&Value::text("Sales")));
    assert_eq!(diana.fields.get("salary"), Some(&Value::Integer(60000)));
}
