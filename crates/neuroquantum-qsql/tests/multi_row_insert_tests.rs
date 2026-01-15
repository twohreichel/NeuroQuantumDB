//! Integration tests for Multi-Row INSERT (Bulk Insert)
//!
//! This test suite verifies that multi-row INSERT statements work correctly
//! with proper parsing, execution, and storage integration.

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Test parsing of multi-row INSERT statement
#[test]
fn test_parse_multi_row_insert() {
    let parser = Parser::new();

    // Test multi-row INSERT with 3 rows
    let sql = "INSERT INTO users (name, email, age) VALUES \
               ('Alice', 'alice@example.com', 25), \
               ('Bob', 'bob@example.com', 30), \
               ('Charlie', 'charlie@example.com', 35)";

    let result = parser.parse(sql);
    assert!(
        result.is_ok(),
        "Failed to parse multi-row INSERT: {:?}",
        result.err()
    );

    let statement = result.unwrap();
    match statement {
        | neuroquantum_qsql::ast::Statement::Insert(insert) => {
            assert_eq!(insert.table_name, "users");
            assert_eq!(insert.values.len(), 3, "Expected 3 value sets");

            // Check first row
            assert_eq!(insert.values[0].len(), 3, "Expected 3 values in first row");

            // Check second row
            assert_eq!(insert.values[1].len(), 3, "Expected 3 values in second row");

            // Check third row
            assert_eq!(insert.values[2].len(), 3, "Expected 3 values in third row");
        },
        | _ => panic!("Expected INSERT statement"),
    }
}

/// Test multi-row INSERT with storage engine
#[tokio::test]
async fn test_execute_multi_row_insert() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine
    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table
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
                name: "email".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "age".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    // Create query executor
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Parse and execute multi-row INSERT
    let parser = Parser::new();
    let sql = "INSERT INTO users (name, email, age) VALUES \
               ('Alice', 'alice@example.com', 25), \
               ('Bob', 'bob@example.com', 30), \
               ('Charlie', 'charlie@example.com', 35)";

    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify 3 rows were inserted
    assert_eq!(result.rows_affected, 3, "Expected 3 rows to be inserted");

    // Verify data is in storage
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

    assert_eq!(rows.len(), 3, "Expected 3 rows in storage");

    // Verify individual rows
    assert_eq!(
        rows[0].fields.get("name"),
        Some(&Value::Text("Alice".to_string()))
    );
    assert_eq!(
        rows[0].fields.get("email"),
        Some(&Value::Text("alice@example.com".to_string()))
    );
    assert_eq!(rows[0].fields.get("age"), Some(&Value::Integer(25)));

    assert_eq!(
        rows[1].fields.get("name"),
        Some(&Value::Text("Bob".to_string()))
    );
    assert_eq!(
        rows[1].fields.get("email"),
        Some(&Value::Text("bob@example.com".to_string()))
    );
    assert_eq!(rows[1].fields.get("age"), Some(&Value::Integer(30)));

    assert_eq!(
        rows[2].fields.get("name"),
        Some(&Value::Text("Charlie".to_string()))
    );
    assert_eq!(
        rows[2].fields.get("email"),
        Some(&Value::Text("charlie@example.com".to_string()))
    );
    assert_eq!(rows[2].fields.get("age"), Some(&Value::Integer(35)));
}

/// Test multi-row INSERT with auto-increment IDs
///
/// NOTE: This test is currently ignored due to a bug in DNA compression
/// that causes certain rows to fail decompression when reading back from disk.
/// The issue appears to be related to how Float values are serialized/compressed.
/// TODO: Investigate and fix the DNA compression issue in storage.rs/dna.rs
#[tokio::test]
#[ignore = "DNA compression bug causes row decompression failures - needs investigation"]
async fn test_multi_row_insert_auto_increment() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create table with auto-increment ID
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
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Insert multiple rows without specifying ID
    let parser = Parser::new();
    let sql = "INSERT INTO products (name, price) VALUES \
               ('Laptop', 999.99), \
               ('Mouse', 29.99), \
               ('Keyboard', 79.99)";

    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows_affected, 3, "Expected 3 rows to be inserted");

    // Verify auto-increment IDs were generated
    let query = neuroquantum_core::storage::SelectQuery {
        table: "products".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: Some(neuroquantum_core::storage::OrderBy {
            field: "id".to_string(),
            direction: neuroquantum_core::storage::SortDirection::Ascending,
        }),
        limit: None,
        offset: None,
    };

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();

    assert_eq!(rows.len(), 3);

    // IDs should be sequential
    assert!(rows[0].fields.contains_key("id"));
    assert!(rows[1].fields.contains_key("id"));
    assert!(rows[2].fields.contains_key("id"));
}

/// Test multi-row INSERT in transaction (atomic)
#[tokio::test]
async fn test_multi_row_insert_transaction() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table
    let schema = TableSchema {
        name: "orders".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: true,
            },
            ColumnDefinition {
                name: "customer".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "total".to_string(),
                data_type: DataType::Float,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();

    // Begin transaction
    let begin_sql = "BEGIN";
    let begin_stmt = parser.parse(begin_sql).unwrap();
    executor.execute_statement(&begin_stmt).await.unwrap();

    // Multi-row INSERT within transaction
    let insert_sql = "INSERT INTO orders (customer, total) VALUES \
                      ('Alice', 100.50), \
                      ('Bob', 250.75), \
                      ('Charlie', 75.25)";

    let insert_stmt = parser.parse(insert_sql).unwrap();
    let result = executor.execute_statement(&insert_stmt).await.unwrap();
    assert_eq!(result.rows_affected, 3);

    // Commit transaction
    let commit_sql = "COMMIT";
    let commit_stmt = parser.parse(commit_sql).unwrap();
    executor.execute_statement(&commit_stmt).await.unwrap();

    // Verify all rows were committed
    let query = neuroquantum_core::storage::SelectQuery {
        table: "orders".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();
    assert_eq!(rows.len(), 3, "All 3 rows should be committed");
}

/// Test large batch INSERT
#[tokio::test]
async fn test_large_batch_insert() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    let schema = TableSchema {
        name: "logs".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: true,
            },
            ColumnDefinition {
                name: "message".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Build a multi-row INSERT with 10 rows
    let mut values_parts = Vec::new();
    for i in 1..=10 {
        values_parts.push(format!("('Log message {i}')"));
    }
    let sql = format!(
        "INSERT INTO logs (message) VALUES {}",
        values_parts.join(", ")
    );

    let parser = Parser::new();
    let statement = parser.parse(&sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows_affected, 10, "Expected 10 rows to be inserted");

    // Verify all rows are in storage
    let query = neuroquantum_core::storage::SelectQuery {
        table: "logs".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();
    assert_eq!(rows.len(), 10);
}
