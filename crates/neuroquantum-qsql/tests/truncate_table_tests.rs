//! Integration tests for TRUNCATE TABLE functionality
//!
//! This test suite verifies TRUNCATE TABLE operations including:
//! - Basic TRUNCATE TABLE
//! - TRUNCATE TABLE CASCADE
//! - TRUNCATE TABLE RESTRICT
//! - TRUNCATE TABLE RESTART IDENTITY
//! - TRUNCATE TABLE CONTINUE IDENTITY

use std::sync::Arc;

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
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
    for i in 1..=5 {
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
async fn test_truncate_table_basic() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Verify table has data
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(
        result.rows.len(),
        5,
        "Table should have 5 rows before TRUNCATE"
    );

    // Execute TRUNCATE TABLE
    let truncate_sql = "TRUNCATE TABLE users";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_ok(), "TRUNCATE TABLE should succeed");

    // Verify table is empty
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(result.rows.len(), 0, "Table should be empty after TRUNCATE");
}

#[tokio::test]
async fn test_truncate_table_without_table_keyword() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE without TABLE keyword (should still work)
    let truncate_sql = "TRUNCATE users";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "TRUNCATE without TABLE keyword should succeed"
    );

    // Verify table is empty
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(result.rows.len(), 0, "Table should be empty after TRUNCATE");
}

#[tokio::test]
async fn test_truncate_table_cascade() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE TABLE with CASCADE
    let truncate_sql = "TRUNCATE TABLE users CASCADE";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_ok(), "TRUNCATE TABLE CASCADE should succeed");

    // Verify table is empty
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(
        result.rows.len(),
        0,
        "Table should be empty after TRUNCATE CASCADE"
    );
}

#[tokio::test]
async fn test_truncate_table_restrict() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE TABLE with RESTRICT
    let truncate_sql = "TRUNCATE TABLE users RESTRICT";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "TRUNCATE TABLE RESTRICT should succeed (no FK constraints)"
    );

    // Verify table is empty
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(
        result.rows.len(),
        0,
        "Table should be empty after TRUNCATE RESTRICT"
    );
}

#[tokio::test]
async fn test_truncate_table_restart_identity() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE TABLE with RESTART IDENTITY
    let truncate_sql = "TRUNCATE TABLE users RESTART IDENTITY";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "TRUNCATE TABLE RESTART IDENTITY should succeed"
    );

    // Verify table is empty
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(
        result.rows.len(),
        0,
        "Table should be empty after TRUNCATE RESTART IDENTITY"
    );
}

#[tokio::test]
async fn test_truncate_table_continue_identity() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE TABLE with CONTINUE IDENTITY
    let truncate_sql = "TRUNCATE TABLE users CONTINUE IDENTITY";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "TRUNCATE TABLE CONTINUE IDENTITY should succeed"
    );

    // Verify table is empty
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(
        result.rows.len(),
        0,
        "Table should be empty after TRUNCATE CONTINUE IDENTITY"
    );
}

#[tokio::test]
async fn test_truncate_table_restart_identity_cascade() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE TABLE with RESTART IDENTITY CASCADE
    let truncate_sql = "TRUNCATE TABLE users RESTART IDENTITY CASCADE";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "TRUNCATE TABLE RESTART IDENTITY CASCADE should succeed"
    );

    // Verify table is empty
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(result.rows.len(), 0, "Table should be empty");
}

#[tokio::test]
async fn test_truncate_table_continue_identity_restrict() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE TABLE with CONTINUE IDENTITY RESTRICT
    let truncate_sql = "TRUNCATE TABLE users CONTINUE IDENTITY RESTRICT";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_ok(),
        "TRUNCATE TABLE CONTINUE IDENTITY RESTRICT should succeed"
    );

    // Verify table is empty
    let select_sql = "SELECT * FROM users";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(result.rows.len(), 0, "Table should be empty");
}

#[tokio::test]
async fn test_truncate_is_faster_than_delete() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE and measure time
    let truncate_sql = "TRUNCATE TABLE users";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_ok(), "TRUNCATE should succeed");

    // Note: In a more complete test, we would compare this to DELETE FROM users
    // and verify that TRUNCATE is faster for large tables
    // For now, we just verify that TRUNCATE works
}

#[tokio::test]
async fn test_truncate_table_preserves_structure() {
    let (_temp_dir, storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute TRUNCATE TABLE
    let truncate_sql = "TRUNCATE TABLE users";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_ok(), "TRUNCATE TABLE should succeed");

    // Verify we can still insert data (table structure preserved)
    let insert_sql = "INSERT INTO users (id, name, age) VALUES (100, 'New User', 30)";
    let statement = parser.parse(insert_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(result.is_ok(), "INSERT after TRUNCATE should succeed");

    // Verify the new row exists
    let select_sql = "SELECT * FROM users WHERE id = 100";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(result.rows.len(), 1, "Should have 1 row after insert");

    // Verify table schema still exists
    let storage_guard = storage_arc.read().await;
    let schema = storage_guard.get_table_schema("users");
    assert!(
        schema.is_some(),
        "Table schema should still exist after TRUNCATE"
    );
    let schema = schema.unwrap();
    assert_eq!(schema.columns.len(), 3, "Table should still have 3 columns");
}

#[tokio::test]
async fn test_truncate_nonexistent_table() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Try to truncate a non-existent table
    let truncate_sql = "TRUNCATE TABLE nonexistent_table";
    let statement = parser.parse(truncate_sql).unwrap();
    let result = executor.execute_statement(&statement).await;
    assert!(
        result.is_err(),
        "TRUNCATE on non-existent table should fail"
    );
}

// Parser unit tests
mod parser_tests {
    use neuroquantum_qsql::ast::{Statement, TruncateBehavior};
    use neuroquantum_qsql::Parser;

    #[test]
    fn test_parse_truncate_table_basic() {
        let parser = Parser::new();
        let result = parser.parse("TRUNCATE TABLE users");
        assert!(result.is_ok(), "Should parse TRUNCATE TABLE");

        if let Ok(Statement::TruncateTable(stmt)) = result {
            assert_eq!(stmt.table_name, "users");
            assert_eq!(stmt.behavior, TruncateBehavior::Restrict);
            assert!(!stmt.restart_identity);
        } else {
            panic!("Expected TruncateTable statement");
        }
    }

    #[test]
    fn test_parse_truncate_without_table_keyword() {
        let parser = Parser::new();
        let result = parser.parse("TRUNCATE users");
        assert!(
            result.is_ok(),
            "Should parse TRUNCATE without TABLE keyword"
        );

        if let Ok(Statement::TruncateTable(stmt)) = result {
            assert_eq!(stmt.table_name, "users");
        } else {
            panic!("Expected TruncateTable statement");
        }
    }

    #[test]
    fn test_parse_truncate_cascade() {
        let parser = Parser::new();
        let result = parser.parse("TRUNCATE TABLE users CASCADE");
        assert!(result.is_ok(), "Should parse TRUNCATE TABLE CASCADE");

        if let Ok(Statement::TruncateTable(stmt)) = result {
            assert_eq!(stmt.table_name, "users");
            assert_eq!(stmt.behavior, TruncateBehavior::Cascade);
        } else {
            panic!("Expected TruncateTable statement");
        }
    }

    #[test]
    fn test_parse_truncate_restrict() {
        let parser = Parser::new();
        let result = parser.parse("TRUNCATE TABLE users RESTRICT");
        assert!(result.is_ok(), "Should parse TRUNCATE TABLE RESTRICT");

        if let Ok(Statement::TruncateTable(stmt)) = result {
            assert_eq!(stmt.table_name, "users");
            assert_eq!(stmt.behavior, TruncateBehavior::Restrict);
        } else {
            panic!("Expected TruncateTable statement");
        }
    }

    #[test]
    fn test_parse_truncate_restart_identity() {
        let parser = Parser::new();
        let result = parser.parse("TRUNCATE TABLE users RESTART IDENTITY");
        assert!(
            result.is_ok(),
            "Should parse TRUNCATE TABLE RESTART IDENTITY"
        );

        if let Ok(Statement::TruncateTable(stmt)) = result {
            assert_eq!(stmt.table_name, "users");
            assert!(stmt.restart_identity);
        } else {
            panic!("Expected TruncateTable statement");
        }
    }

    #[test]
    fn test_parse_truncate_continue_identity() {
        let parser = Parser::new();
        let result = parser.parse("TRUNCATE TABLE users CONTINUE IDENTITY");
        assert!(
            result.is_ok(),
            "Should parse TRUNCATE TABLE CONTINUE IDENTITY"
        );

        if let Ok(Statement::TruncateTable(stmt)) = result {
            assert_eq!(stmt.table_name, "users");
            assert!(!stmt.restart_identity);
        } else {
            panic!("Expected TruncateTable statement");
        }
    }

    #[test]
    fn test_parse_truncate_restart_identity_cascade() {
        let parser = Parser::new();
        let result = parser.parse("TRUNCATE TABLE users RESTART IDENTITY CASCADE");
        assert!(
            result.is_ok(),
            "Should parse TRUNCATE TABLE RESTART IDENTITY CASCADE"
        );

        if let Ok(Statement::TruncateTable(stmt)) = result {
            assert_eq!(stmt.table_name, "users");
            assert!(stmt.restart_identity);
            assert_eq!(stmt.behavior, TruncateBehavior::Cascade);
        } else {
            panic!("Expected TruncateTable statement");
        }
    }

    #[test]
    fn test_parse_truncate_case_insensitive() {
        let parser = Parser::new();

        // Test lowercase
        let result = parser.parse("truncate table users");
        assert!(result.is_ok(), "Should parse lowercase TRUNCATE TABLE");

        // Test mixed case
        let result = parser.parse("Truncate Table Users");
        assert!(result.is_ok(), "Should parse mixed case TRUNCATE TABLE");
    }
}
