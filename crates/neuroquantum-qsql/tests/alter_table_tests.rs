//! Integration tests for ALTER TABLE functionality
//!
//! This test suite verifies ALTER TABLE operations including:
//! - ADD COLUMN
//! - DROP COLUMN
//! - RENAME COLUMN
//! - MODIFY COLUMN

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create a test setup with storage and executor
async fn setup_test_env() -> (
    TempDir,
    Arc<tokio::sync::RwLock<StorageEngine>>,
    QueryExecutor,
) {
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
                name: "age".to_string(),
                data_type: DataType::Integer,
                nullable: true,
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
    }

    // Insert some test data
    let parser = Parser::new();
    let config = ExecutorConfig {
        enable_neuromorphic_learning: true,
        enable_synaptic_optimization: true,
        enable_dna_compression: true,
        ..Default::default()
    };
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Insert test rows
    for i in 1..=3 {
        let sql = format!(
            "INSERT INTO users (id, name, age) VALUES ({}, 'User {}', {})",
            i,
            i,
            20 + i
        );
        let statement = parser.parse(&sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }

    (temp_dir, storage_arc, executor)
}

#[tokio::test]
async fn test_alter_table_add_column() {
    let (_temp_dir, storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Add a new column with default value
    let sql = "ALTER TABLE users ADD COLUMN email TEXT DEFAULT 'unknown@example.com'";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "ALTER TABLE ADD COLUMN failed: {result:?}"
    );

    // Verify the column was added to schema by checking storage directly
    {
        let storage_guard = storage_arc.read().await;

        // Use select_rows to get data
        let query = neuroquantum_core::storage::SelectQuery {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: Some(1),
            offset: None,
        };
        let rows = storage_guard.select_rows(&query).await.unwrap();
        assert_eq!(rows.len(), 1, "Should have 1 row");

        // Check if email field exists
        let row = &rows[0];
        assert!(
            row.fields.contains_key("email"),
            "Row should have email column. Fields: {:?}",
            row.fields.keys().collect::<Vec<_>>()
        );

        // Check the default value
        if let Some(email_val) = row.fields.get("email") {
            match email_val {
                | neuroquantum_core::storage::Value::Text(s) => {
                    assert_eq!(s, "unknown@example.com", "Email should have default value");
                },
                | _ => panic!("Email should be Text type"),
            }
        }
    }
}

#[tokio::test]
async fn test_alter_table_add_column_nullable() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Add a nullable column without default
    let sql = "ALTER TABLE users ADD COLUMN phone TEXT";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "ALTER TABLE ADD COLUMN failed: {result:?}"
    );

    // Verify existing rows have NULL for the new column
    let sql = "SELECT id, name, phone FROM users WHERE id = 1";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 1, "Should have 1 row");
}

#[tokio::test]
async fn test_alter_table_drop_column() {
    let (_temp_dir, storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Drop a column
    let sql = "ALTER TABLE users DROP COLUMN age";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "ALTER TABLE DROP COLUMN failed: {result:?}"
    );

    // Verify the column was removed from schema
    {
        let storage_guard = storage_arc.read().await;
        let table_count = storage_guard.get_table_count();
        assert_eq!(table_count, 1, "Should still have 1 table");
    }

    // Verify existing rows no longer have the column
    let sql = "SELECT id, name FROM users WHERE id = 1";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 1, "Should have 1 row");
    let row = &result.rows[0];
    assert!(!row.contains_key("age"), "Row should not have age column");
}

#[tokio::test]
async fn test_alter_table_drop_primary_key_fails() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Try to drop primary key column - should fail
    let sql = "ALTER TABLE users DROP COLUMN id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_err(),
        "Should not be able to drop primary key column"
    );
}

#[tokio::test]
async fn test_alter_table_rename_column() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Rename a column
    let sql = "ALTER TABLE users RENAME COLUMN name TO full_name";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "ALTER TABLE RENAME COLUMN failed: {result:?}"
    );

    // Verify the column was renamed
    let sql = "SELECT id, full_name FROM users WHERE id = 1";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 1, "Should have 1 row");
    let row = &result.rows[0];
    assert!(
        row.contains_key("full_name"),
        "Row should have full_name column"
    );
    assert!(!row.contains_key("name"), "Row should not have name column");
}

#[tokio::test]
async fn test_alter_table_rename_to_existing_column_fails() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Try to rename to an existing column name - should fail
    let sql = "ALTER TABLE users RENAME COLUMN name TO age";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_err(),
        "Should not be able to rename to existing column name"
    );
}

#[tokio::test]
async fn test_alter_table_modify_column_int_to_text() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Modify column type from INTEGER to TEXT
    let sql = "ALTER TABLE users MODIFY COLUMN age TEXT";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "ALTER TABLE MODIFY COLUMN failed: {result:?}"
    );

    // Verify the data was converted
    let sql = "SELECT id, age FROM users WHERE id = 1";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 1, "Should have 1 row");
}

#[tokio::test]
async fn test_alter_table_modify_column_text_to_int() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // First, add a text column with integer values
    let sql = "ALTER TABLE users ADD COLUMN score TEXT DEFAULT '100'";
    let statement = parser.parse(sql).unwrap();
    executor.execute_statement(&statement).await.unwrap();

    // Now modify it to INTEGER
    let sql = "ALTER TABLE users MODIFY COLUMN score INTEGER";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "ALTER TABLE MODIFY COLUMN failed: {result:?}"
    );
}

#[tokio::test]
async fn test_alter_table_multiple_operations() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Perform multiple ALTER TABLE operations
    let operations = vec![
        "ALTER TABLE users ADD COLUMN email TEXT DEFAULT 'test@example.com'",
        "ALTER TABLE users ADD COLUMN phone TEXT",
        "ALTER TABLE users RENAME COLUMN name TO full_name",
        "ALTER TABLE users DROP COLUMN age",
        "ALTER TABLE users MODIFY COLUMN email TEXT",
    ];

    for sql in operations {
        let statement = parser.parse(sql).unwrap();
        let result = executor.execute_statement(&statement).await;
        assert!(result.is_ok(), "ALTER TABLE operation failed for: {sql}");
    }

    // Verify final schema by selecting
    let sql = "SELECT id, full_name, email, phone FROM users WHERE id = 1";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 1, "Should have 1 row");
    let row = &result.rows[0];
    assert!(row.contains_key("full_name"), "Should have full_name");
    assert!(row.contains_key("email"), "Should have email");
    assert!(row.contains_key("phone"), "Should have phone");
    assert!(!row.contains_key("name"), "Should not have name");
    assert!(!row.contains_key("age"), "Should not have age");
}

#[tokio::test]
async fn test_alter_table_nonexistent_table() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Try to alter a table that doesn't exist
    let sql = "ALTER TABLE nonexistent ADD COLUMN col1 TEXT";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_err(), "Should fail for nonexistent table");
}

#[tokio::test]
async fn test_alter_table_nonexistent_column() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Try to drop a column that doesn't exist
    let sql = "ALTER TABLE users DROP COLUMN nonexistent";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_err(), "Should fail for nonexistent column");

    // Try to rename a column that doesn't exist
    let sql = "ALTER TABLE users RENAME COLUMN nonexistent TO new_name";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_err(), "Should fail for nonexistent column");

    // Try to modify a column that doesn't exist
    let sql = "ALTER TABLE users MODIFY COLUMN nonexistent TEXT";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_err(), "Should fail for nonexistent column");
}

#[tokio::test]
async fn test_alter_table_add_duplicate_column() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Try to add a column that already exists
    let sql = "ALTER TABLE users ADD COLUMN name TEXT";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_err(), "Should fail for duplicate column");
}
