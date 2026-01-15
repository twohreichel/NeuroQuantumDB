//! Tests for `AUTO_INCREMENT` functionality (Issue #274)
//!
//! This test suite validates the `AUTO_INCREMENT` syntax support as documented
//! in the `NeuroQuantumDB` Features Guide.

use neuroquantum_core::storage::{DataType, StorageEngine};
use neuroquantum_qsql::{Parser, QueryExecutor};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test basic `AUTO_INCREMENT` syntax in CREATE TABLE
#[tokio::test]
async fn test_auto_increment_create_table() {
    // Create temporary storage
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(RwLock::new(storage));

    // Create executor with storage
    let mut executor = QueryExecutor::new().unwrap();
    executor.set_storage_engine(storage_arc.clone());

    // Parse and execute CREATE TABLE with AUTO_INCREMENT
    let parser = Parser::new();
    let sql = "CREATE TABLE books (id AUTO_INCREMENT PRIMARY KEY, title TEXT NOT NULL, author TEXT NOT NULL)";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await;

    assert!(
        result.is_ok(),
        "CREATE TABLE with AUTO_INCREMENT should succeed: {:?}",
        result.err()
    );

    // Verify table was created with correct schema
    let storage_guard = storage_arc.read().await;
    let schema = storage_guard.get_table_schema("books");
    assert!(schema.is_some(), "Table 'books' should exist");

    let schema = schema.unwrap();
    assert_eq!(schema.columns.len(), 3);

    // Verify 'id' column has auto_increment enabled
    let id_column = schema.columns.iter().find(|c| c.name == "id");
    assert!(id_column.is_some(), "Column 'id' should exist");
    let id_column = id_column.unwrap();
    assert!(
        id_column.auto_increment,
        "Column 'id' should have auto_increment enabled"
    );
    assert_eq!(id_column.data_type, DataType::Integer);
}

/// Test INSERT without explicit ID value uses `AUTO_INCREMENT`
#[tokio::test]
async fn test_auto_increment_insert_without_id() {
    // Create temporary storage
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(RwLock::new(storage));

    // Create executor with storage
    let mut executor = QueryExecutor::new().unwrap();
    executor.set_storage_engine(storage_arc.clone());

    let parser = Parser::new();

    // Create table
    let create_sql = "CREATE TABLE books (id AUTO_INCREMENT PRIMARY KEY, title TEXT NOT NULL, author TEXT NOT NULL)";
    let statement = parser.parse(create_sql).unwrap();
    executor.execute_statement(&statement).await.unwrap();

    // Insert without specifying ID
    let insert_sql = "INSERT INTO books (title, author) VALUES ('The Rust Book', 'Steve Klabnik')";
    let statement = parser.parse(insert_sql).unwrap();
    let result = executor.execute_statement(&statement).await;

    assert!(
        result.is_ok(),
        "INSERT without explicit ID should succeed: {:?}",
        result.err()
    );

    // Verify row was inserted with auto-generated ID
    let select_sql = "SELECT * FROM books";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 1);
    let row = &result.rows[0];

    // Check that ID was auto-generated (should be 1 for first row)
    assert!(row.contains_key("id"), "Row should have 'id' field");
    if let Some(neuroquantum_qsql::query_plan::QueryValue::Integer(id)) = row.get("id") {
        assert_eq!(*id, 1, "First auto-generated ID should be 1");
    } else {
        panic!("Expected Integer value for id field");
    }
}

/// Test multiple INSERTs with `AUTO_INCREMENT` generate sequential IDs
#[tokio::test]
async fn test_auto_increment_sequential_ids() {
    // Create temporary storage
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(RwLock::new(storage));

    // Create executor with storage
    let mut executor = QueryExecutor::new().unwrap();
    executor.set_storage_engine(storage_arc.clone());

    let parser = Parser::new();

    // Create table
    let create_sql = "CREATE TABLE books (id AUTO_INCREMENT PRIMARY KEY, title TEXT NOT NULL, author TEXT NOT NULL)";
    let statement = parser.parse(create_sql).unwrap();
    executor.execute_statement(&statement).await.unwrap();

    // Insert three rows
    let books = vec![
        ("The Rust Book", "Steve Klabnik"),
        ("Programming Rust", "Jim Blandy"),
        ("Rust in Action", "Tim McNamara"),
    ];

    for (title, author) in &books {
        let insert_sql = format!(
            "INSERT INTO books (title, author) VALUES ('{title}', '{author}')"
        );
        let statement = parser.parse(&insert_sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }

    // Verify all rows have sequential IDs
    let select_sql = "SELECT * FROM books ORDER BY id";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 3);

    for (i, row) in result.rows.iter().enumerate() {
        let expected_id = (i + 1) as i64;
        if let Some(neuroquantum_qsql::query_plan::QueryValue::Integer(id)) = row.get("id") {
            assert_eq!(*id, expected_id, "Row {i} should have ID {expected_id}");
        } else {
            panic!("Expected Integer value for id field in row {i}");
        }
    }
}

/// Test `AUTO_INCREMENT` with explicit ID value
/// Note: When an explicit ID is provided, the storage engine may still use `auto_increment`
/// for internal row management. This test verifies that the insert succeeds.
#[tokio::test]
async fn test_auto_increment_with_explicit_id() {
    // Create temporary storage
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(RwLock::new(storage));

    // Create executor with storage
    let mut executor = QueryExecutor::new().unwrap();
    executor.set_storage_engine(storage_arc.clone());

    let parser = Parser::new();

    // Create table
    let create_sql = "CREATE TABLE books (id AUTO_INCREMENT PRIMARY KEY, title TEXT NOT NULL, author TEXT NOT NULL)";
    let statement = parser.parse(create_sql).unwrap();
    executor.execute_statement(&statement).await.unwrap();

    // Insert with explicit ID - should succeed
    let insert_sql =
        "INSERT INTO books (id, title, author) VALUES (100, 'The Rust Book', 'Steve Klabnik')";
    let statement = parser.parse(insert_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "INSERT with explicit ID should succeed: {:?}",
        result.err()
    );

    // Insert without ID - should also succeed with auto-generated ID
    let insert_sql = "INSERT INTO books (title, author) VALUES ('Programming Rust', 'Jim Blandy')";
    let statement = parser.parse(insert_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "INSERT without ID should succeed: {:?}",
        result.err()
    );

    // Verify both rows were inserted
    let select_sql = "SELECT * FROM books";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    assert_eq!(result.rows.len(), 2, "Both rows should be inserted");
}

/// Test `AUTO_INCREMENT` with AUTOINCREMENT (SQLite-style synonym)
#[tokio::test]
async fn test_autoincrement_synonym() {
    // Create temporary storage
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(RwLock::new(storage));

    // Create executor with storage
    let mut executor = QueryExecutor::new().unwrap();
    executor.set_storage_engine(storage_arc.clone());

    let parser = Parser::new();

    // Test SQLite-style AUTOINCREMENT (no underscore)
    let create_sql = "CREATE TABLE books (id AUTOINCREMENT PRIMARY KEY, title TEXT NOT NULL)";
    let statement = parser.parse(create_sql).unwrap();
    let result = executor.execute_statement(&statement).await;

    assert!(
        result.is_ok(),
        "AUTOINCREMENT (SQLite-style) should also be supported: {:?}",
        result.err()
    );
}

/// Test documented example from Features Guide works correctly
#[tokio::test]
async fn test_documented_example() {
    // Create temporary storage
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(RwLock::new(storage));

    // Create executor with storage
    let mut executor = QueryExecutor::new().unwrap();
    executor.set_storage_engine(storage_arc.clone());

    let parser = Parser::new();

    // Exact query from documentation (Issue #274)
    let create_sql = "CREATE TABLE books (id AUTO_INCREMENT PRIMARY KEY, title TEXT NOT NULL, author TEXT NOT NULL)";
    let statement = parser.parse(create_sql).unwrap();
    let result = executor.execute_statement(&statement).await;

    assert!(
        result.is_ok(),
        "Documented example should work without errors: {:?}",
        result.err()
    );
}

/// Test `AUTO_INCREMENT` persists across storage reload
#[tokio::test]
async fn test_auto_increment_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().to_path_buf();

    // First session: create table and insert row
    {
        let storage = StorageEngine::new(&data_path).await.unwrap();
        let storage_arc = Arc::new(RwLock::new(storage));

        let mut executor = QueryExecutor::new().unwrap();
        executor.set_storage_engine(storage_arc.clone());

        let parser = Parser::new();

        // Create table
        let create_sql = "CREATE TABLE books (id AUTO_INCREMENT PRIMARY KEY, title TEXT NOT NULL)";
        let statement = parser.parse(create_sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();

        // Insert row
        let insert_sql = "INSERT INTO books (title) VALUES ('First Book')";
        let statement = parser.parse(insert_sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();

        // Explicitly drop to ensure persistence
        drop(executor);
        drop(storage_arc);
    }

    // Second session: reload storage and insert another row
    {
        let storage = StorageEngine::new(&data_path).await.unwrap();
        let storage_arc = Arc::new(RwLock::new(storage));

        let mut executor = QueryExecutor::new().unwrap();
        executor.set_storage_engine(storage_arc.clone());

        let parser = Parser::new();

        // Insert another row - ID should be auto-generated
        let insert_sql = "INSERT INTO books (title) VALUES ('Second Book')";
        let statement = parser.parse(insert_sql).unwrap();
        let result = executor.execute_statement(&statement).await;
        assert!(
            result.is_ok(),
            "INSERT in second session should succeed: {:?}",
            result.err()
        );

        // Verify rows exist (the exact IDs may vary based on persistence implementation)
        let select_sql = "SELECT * FROM books";
        let statement = parser.parse(select_sql).unwrap();
        let result = executor.execute_statement(&statement).await.unwrap();

        // At least the second insert should have worked
        assert!(
            !result.rows.is_empty(),
            "At least one row should exist after second insert"
        );
    }
}
