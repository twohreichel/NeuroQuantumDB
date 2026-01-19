//! Integration tests for transaction control with storage engine
//!
//! Tests that verify BEGIN, COMMIT, ROLLBACK work end-to-end with
//! the storage engine and transaction manager.

use std::sync::Arc;

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_core::transaction::TransactionManager;
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use tempfile::TempDir;

/// Helper function to create a test environment with transaction support
async fn setup_test_environment() -> (
    TempDir,
    Arc<tokio::sync::RwLock<StorageEngine>>,
    Arc<TransactionManager>,
    QueryExecutor,
) {
    // Create temporary directories for storage and WAL
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().join("storage");
    let wal_path = temp_dir.path().join("wal");

    tokio::fs::create_dir_all(&storage_path).await.unwrap();
    tokio::fs::create_dir_all(&wal_path).await.unwrap();

    // Initialize storage engine
    let storage = StorageEngine::new(&storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Initialize transaction manager
    let tx_manager = TransactionManager::new_async(&wal_path).await.unwrap();
    let tx_manager_arc = Arc::new(tx_manager);

    // Create query executor with storage and transaction support
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    executor.set_transaction_manager(tx_manager_arc.clone());

    (temp_dir, storage_arc, tx_manager_arc, executor)
}

/// Helper function to create a test table
async fn create_test_table(storage_arc: &Arc<tokio::sync::RwLock<StorageEngine>>) {
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
                name: "balance".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: Some(Value::Integer(0)),
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: std::collections::HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
        foreign_keys: Vec::new(),
    };

    let mut storage_guard = storage_arc.write().await;
    storage_guard.create_table(schema).await.unwrap();
}

#[tokio::test]
async fn test_begin_commit_basic() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // BEGIN transaction
    let begin_stmt = parser.parse("BEGIN").unwrap();
    let result = executor.execute_statement(&begin_stmt).await.unwrap();
    assert_eq!(result.rows_affected, 0);

    // INSERT data
    let insert_stmt = parser
        .parse("INSERT INTO users (id, name, balance) VALUES (1, 'Alice', 100)")
        .unwrap();
    let result = executor.execute_statement(&insert_stmt).await.unwrap();
    assert_eq!(result.rows_affected, 1);

    // COMMIT transaction
    let commit_stmt = parser.parse("COMMIT").unwrap();
    let result = executor.execute_statement(&commit_stmt).await.unwrap();
    assert_eq!(result.rows_affected, 0);

    // Verify data was persisted
    let select_stmt = parser.parse("SELECT * FROM users WHERE id = 1").unwrap();
    let result = executor.execute_statement(&select_stmt).await.unwrap();
    assert_eq!(result.rows.len(), 1);

    println!("✅ BEGIN/COMMIT basic test: SUCCESS");
}

#[tokio::test]
async fn test_begin_rollback_basic() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // BEGIN transaction
    let begin_stmt = parser.parse("BEGIN").unwrap();
    executor.execute_statement(&begin_stmt).await.unwrap();

    // INSERT data
    let insert_stmt = parser
        .parse("INSERT INTO users (id, name, balance) VALUES (1, 'Bob', 200)")
        .unwrap();
    let result = executor.execute_statement(&insert_stmt).await.unwrap();
    assert_eq!(result.rows_affected, 1);

    // ROLLBACK transaction
    let rollback_stmt = parser.parse("ROLLBACK").unwrap();
    let result = executor.execute_statement(&rollback_stmt).await.unwrap();
    assert_eq!(result.rows_affected, 0);

    // Verify data was NOT persisted
    let select_stmt = parser.parse("SELECT * FROM users WHERE id = 1").unwrap();
    let result = executor.execute_statement(&select_stmt).await.unwrap();
    assert_eq!(result.rows.len(), 0);

    println!("✅ BEGIN/ROLLBACK basic test: SUCCESS");
}

#[tokio::test]
async fn test_start_transaction_commit() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // START TRANSACTION
    let start_stmt = parser.parse("START TRANSACTION").unwrap();
    executor.execute_statement(&start_stmt).await.unwrap();

    // INSERT data
    let insert_stmt = parser
        .parse("INSERT INTO users (id, name, balance) VALUES (2, 'Charlie', 300)")
        .unwrap();
    executor.execute_statement(&insert_stmt).await.unwrap();

    // COMMIT
    let commit_stmt = parser.parse("COMMIT").unwrap();
    executor.execute_statement(&commit_stmt).await.unwrap();

    // Verify data was persisted
    let select_stmt = parser.parse("SELECT * FROM users WHERE id = 2").unwrap();
    let result = executor.execute_statement(&select_stmt).await.unwrap();
    assert_eq!(result.rows.len(), 1);

    println!("✅ START TRANSACTION/COMMIT test: SUCCESS");
}

#[tokio::test]
async fn test_transaction_isolation_levels() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // Test different isolation levels (should not error)
    let isolation_levels = [
        "BEGIN ISOLATION LEVEL READ UNCOMMITTED",
        "BEGIN ISOLATION LEVEL READ COMMITTED",
        "BEGIN ISOLATION LEVEL REPEATABLE READ",
        "BEGIN ISOLATION LEVEL SERIALIZABLE",
    ];

    for (i, sql) in isolation_levels.iter().enumerate() {
        let begin_stmt = parser.parse(sql).unwrap();
        let result = executor.execute_statement(&begin_stmt).await;
        assert!(result.is_ok(), "Failed to parse isolation level: {sql}");

        // Insert some data
        let insert_sql = format!(
            "INSERT INTO users (id, name, balance) VALUES ({}, 'User{}', 100)",
            i + 10,
            i + 10
        );
        let insert_stmt = parser.parse(&insert_sql).unwrap();
        executor.execute_statement(&insert_stmt).await.unwrap();

        // Commit
        let commit_stmt = parser.parse("COMMIT").unwrap();
        executor.execute_statement(&commit_stmt).await.unwrap();
    }

    println!("✅ Transaction isolation levels test: SUCCESS");
}

#[tokio::test]
async fn test_nested_transaction_error() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // BEGIN first transaction
    let begin_stmt = parser.parse("BEGIN").unwrap();
    executor.execute_statement(&begin_stmt).await.unwrap();

    // Try to BEGIN again - should fail
    let begin_stmt2 = parser.parse("BEGIN").unwrap();
    let result = executor.execute_statement(&begin_stmt2).await;
    assert!(result.is_err(), "Nested transaction should fail");

    // Clean up
    let rollback_stmt = parser.parse("ROLLBACK").unwrap();
    executor.execute_statement(&rollback_stmt).await.unwrap();

    println!("✅ Nested transaction error test: SUCCESS");
}

#[tokio::test]
async fn test_commit_without_transaction_error() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // Try to COMMIT without BEGIN - should fail
    let commit_stmt = parser.parse("COMMIT").unwrap();
    let result = executor.execute_statement(&commit_stmt).await;
    assert!(result.is_err(), "COMMIT without BEGIN should fail");

    println!("✅ COMMIT without transaction error test: SUCCESS");
}

#[tokio::test]
async fn test_rollback_without_transaction_error() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // Try to ROLLBACK without BEGIN - should fail
    let rollback_stmt = parser.parse("ROLLBACK").unwrap();
    let result = executor.execute_statement(&rollback_stmt).await;
    assert!(result.is_err(), "ROLLBACK without BEGIN should fail");

    println!("✅ ROLLBACK without transaction error test: SUCCESS");
}

#[tokio::test]
async fn test_savepoint_basic() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // BEGIN transaction
    let begin_stmt = parser.parse("BEGIN").unwrap();
    executor.execute_statement(&begin_stmt).await.unwrap();

    // INSERT first row
    let insert1 = parser
        .parse("INSERT INTO users (id, name, balance) VALUES (1, 'Alice', 100)")
        .unwrap();
    executor.execute_statement(&insert1).await.unwrap();

    // Create savepoint
    let savepoint_stmt = parser.parse("SAVEPOINT sp1").unwrap();
    let result = executor.execute_statement(&savepoint_stmt).await;
    if let Err(ref e) = result {
        eprintln!("SAVEPOINT error: {e:?}");
    }
    assert!(result.is_ok(), "SAVEPOINT should succeed");

    // INSERT second row
    let insert2 = parser
        .parse("INSERT INTO users (id, name, balance) VALUES (2, 'Bob', 200)")
        .unwrap();
    executor.execute_statement(&insert2).await.unwrap();

    // COMMIT
    let commit_stmt = parser.parse("COMMIT").unwrap();
    executor.execute_statement(&commit_stmt).await.unwrap();

    println!("✅ SAVEPOINT basic test: SUCCESS");
}

#[tokio::test]
async fn test_savepoint_without_transaction_error() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // Try to create SAVEPOINT without BEGIN - should fail
    let savepoint_stmt = parser.parse("SAVEPOINT sp1").unwrap();
    let result = executor.execute_statement(&savepoint_stmt).await;
    assert!(result.is_err(), "SAVEPOINT without BEGIN should fail");

    println!("✅ SAVEPOINT without transaction error test: SUCCESS");
}

#[tokio::test]
async fn test_rollback_to_savepoint() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // BEGIN transaction
    let begin_stmt = parser.parse("BEGIN").unwrap();
    executor.execute_statement(&begin_stmt).await.unwrap();

    // Create savepoint
    let savepoint_stmt = parser.parse("SAVEPOINT sp1").unwrap();
    executor.execute_statement(&savepoint_stmt).await.unwrap();

    // Try ROLLBACK TO SAVEPOINT - should succeed (even if not fully implemented)
    let rollback_to_stmt = parser.parse("ROLLBACK TO SAVEPOINT sp1").unwrap();
    let result = executor.execute_statement(&rollback_to_stmt).await;
    assert!(result.is_ok(), "ROLLBACK TO SAVEPOINT should succeed");

    // Clean up
    let rollback_stmt = parser.parse("ROLLBACK").unwrap();
    executor.execute_statement(&rollback_stmt).await.unwrap();

    println!("✅ ROLLBACK TO SAVEPOINT test: SUCCESS");
}

#[tokio::test]
async fn test_release_savepoint() {
    let (_temp_dir, storage_arc, _tx_manager, mut executor) = setup_test_environment().await;
    create_test_table(&storage_arc).await;

    let parser = Parser::new();

    // BEGIN transaction
    let begin_stmt = parser.parse("BEGIN").unwrap();
    executor.execute_statement(&begin_stmt).await.unwrap();

    // Create savepoint
    let savepoint_stmt = parser.parse("SAVEPOINT sp1").unwrap();
    executor.execute_statement(&savepoint_stmt).await.unwrap();

    // RELEASE SAVEPOINT - should succeed (even if not fully implemented)
    let release_stmt = parser.parse("RELEASE SAVEPOINT sp1").unwrap();
    let result = executor.execute_statement(&release_stmt).await;
    assert!(result.is_ok(), "RELEASE SAVEPOINT should succeed");

    // Clean up
    let commit_stmt = parser.parse("COMMIT").unwrap();
    executor.execute_statement(&commit_stmt).await.unwrap();

    println!("✅ RELEASE SAVEPOINT test: SUCCESS");
}
