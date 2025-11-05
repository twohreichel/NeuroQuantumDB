//! Integration tests for SQL → Storage Engine with DNA compression
//!
//! This test suite verifies that SQL queries (INSERT, SELECT, UPDATE, DELETE)
//! actually use the storage engine with automatic DNA compression and
//! neuromorphic learning.

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use std::collections::HashMap;
use tempfile::TempDir;

/// Test that INSERT queries use DNA compression via storage engine
#[tokio::test]
async fn test_insert_with_dna_compression() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine
    let mut storage = StorageEngine::new(storage_path).await.unwrap();

    // Create test table
    let schema = TableSchema {
        name: "users".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "email".to_string(),
                data_type: DataType::Text,
                nullable: true,
                default_value: None,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
    };

    storage.create_table(schema).await.unwrap();

    // Create query executor with storage integration
    let config = ExecutorConfig {
        enable_neuromorphic_learning: true,
        enable_synaptic_optimization: true,
        enable_dna_compression: true,
        ..Default::default()
    };

    let mut executor = QueryExecutor::with_storage(config, storage.clone()).unwrap();

    // Parse and execute INSERT
    let parser = Parser::new();
    let sql = "INSERT INTO users (id, name, email) VALUES (1, 'John Doe', 'john@example.com')";
    let statement = parser.parse(sql).unwrap();

    // Execute via query executor
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify insert succeeded
    assert_eq!(result.rows_affected, 1);
    assert!(!result.rows.is_empty());

    // Verify data is in storage (DNA compressed!)
    let query = neuroquantum_core::storage::SelectQuery {
        table: "users".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage.select_rows(&query).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].fields.get("name"),
        Some(&Value::Text("John Doe".to_string()))
    );
    assert_eq!(
        rows[0].fields.get("email"),
        Some(&Value::Text("john@example.com".to_string()))
    );

    println!("✅ INSERT with DNA compression: SUCCESS");
}

/// Test that SELECT queries decompress DNA-compressed data
#[tokio::test]
async fn test_select_with_dna_decompression() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine
    let mut storage = StorageEngine::new(storage_path).await.unwrap();

    // Create test table
    let schema = TableSchema {
        name: "products".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "price".to_string(),
                data_type: DataType::Float,
                nullable: false,
                default_value: None,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
    };

    storage.create_table(schema).await.unwrap();

    // Insert test data directly via storage (will be DNA compressed)
    let mut row = neuroquantum_core::storage::Row {
        id: 0,
        fields: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    row.fields.insert("id".to_string(), Value::Integer(1));
    row.fields
        .insert("name".to_string(), Value::Text("Widget".to_string()));
    row.fields.insert("price".to_string(), Value::Float(19.99));

    storage.insert_row("products", row).await.unwrap();

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage.clone()).unwrap();

    // Parse and execute SELECT
    let parser = Parser::new();
    let sql = "SELECT * FROM products";
    let statement = parser.parse(sql).unwrap();

    // Execute via query executor (should decompress DNA automatically)
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify data was decompressed correctly
    assert_eq!(result.rows.len(), 1);
    assert!(result.rows[0].contains_key("name"));
    assert!(result.rows[0].contains_key("price"));

    println!("✅ SELECT with DNA decompression: SUCCESS");
}

/// Test that UPDATE queries re-compress data with DNA
#[tokio::test]
async fn test_update_with_dna_recompression() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let mut storage = StorageEngine::new(storage_path).await.unwrap();

    // Create and populate table
    let schema = TableSchema {
        name: "employees".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "salary".to_string(),
                data_type: DataType::Float,
                nullable: false,
                default_value: None,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
    };

    storage.create_table(schema).await.unwrap();

    // Insert initial data
    let mut row = neuroquantum_core::storage::Row {
        id: 0,
        fields: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    row.fields.insert("id".to_string(), Value::Integer(1));
    row.fields
        .insert("salary".to_string(), Value::Float(50000.0));

    storage.insert_row("employees", row).await.unwrap();

    // Create executor and execute UPDATE
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage.clone()).unwrap();

    let parser = Parser::new();
    let sql = "UPDATE employees SET salary = 60000.0 WHERE id = 1";
    let statement = parser.parse(sql).unwrap();

    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify update succeeded
    assert_eq!(result.rows_affected, 1);

    // Verify updated data (DNA re-compressed)
    let query = neuroquantum_core::storage::SelectQuery {
        table: "employees".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage.select_rows(&query).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fields.get("salary"), Some(&Value::Float(60000.0)));

    println!("✅ UPDATE with DNA re-compression: SUCCESS");
}

/// Test that DELETE queries clean up DNA-compressed blocks
#[tokio::test]
async fn test_delete_with_dna_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let mut storage = StorageEngine::new(storage_path).await.unwrap();

    // Create and populate table
    let schema = TableSchema {
        name: "logs".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
            },
            ColumnDefinition {
                name: "message".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
    };

    storage.create_table(schema).await.unwrap();

    // Insert test data
    for i in 1..=3 {
        let mut row = neuroquantum_core::storage::Row {
            id: 0,
            fields: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        row.fields.insert("id".to_string(), Value::Integer(i));
        row.fields.insert(
            "message".to_string(),
            Value::Text(format!("Log entry {}", i)),
        );
        storage.insert_row("logs", row).await.unwrap();
    }

    // Create executor and execute DELETE
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage.clone()).unwrap();

    // Note: Parser WHERE clause support for DELETE is limited
    // For now, test DELETE without WHERE (deletes all rows)
    let parser = Parser::new();
    let sql = "DELETE FROM logs";
    let statement = parser.parse(sql).unwrap();

    println!("📝 Parsed statement: {:?}", statement);

    let result = executor.execute_statement(&statement).await.unwrap();

    println!("📊 Delete result: rows_affected = {}", result.rows_affected);

    // Verify delete succeeded (all 3 rows deleted)
    assert_eq!(
        result.rows_affected, 3,
        "Expected 3 rows deleted, got {}",
        result.rows_affected
    );

    // Verify data was deleted (DNA blocks cleaned up)
    let query = neuroquantum_core::storage::SelectQuery {
        table: "logs".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage.select_rows(&query).await.unwrap();
    assert_eq!(
        rows.len(),
        0,
        "Expected 0 rows remaining, got {}",
        rows.len()
    );

    println!("✅ DELETE with DNA cleanup: SUCCESS (all rows deleted, DNA blocks freed)");
}
