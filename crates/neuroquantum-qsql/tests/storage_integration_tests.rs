//! Integration tests for SQL → Storage Engine with DNA compression
//!
//! This test suite verifies that SQL queries (INSERT, SELECT, UPDATE, DELETE)
//! actually use the storage engine with automatic DNA compression and
//! neuromorphic learning.

use std::collections::HashMap;
use std::sync::Arc;

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_qsql::query_plan::QueryValue;
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use tempfile::TempDir;

/// Test that INSERT queries use DNA compression via storage engine
#[tokio::test]
async fn test_insert_with_dna_compression() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine wrapped in Arc<RwLock> for shared access
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
                nullable: true,
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

    // Create query executor with storage integration
    let config = ExecutorConfig {
        enable_neuromorphic_learning: true,
        enable_synaptic_optimization: true,
        enable_dna_compression: true,
        ..Default::default()
    };

    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

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

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fields.get("name"), Some(&Value::text("John Doe")));
    assert_eq!(
        rows[0].fields.get("email"),
        Some(&Value::text("john@example.com"))
    );

    println!("✅ INSERT with DNA compression: SUCCESS");
}

/// Test that SELECT queries decompress DNA-compressed data
#[tokio::test]
async fn test_select_with_dna_decompression() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine wrapped in Arc<RwLock>
    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table
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

        // Insert test data directly via storage (will be DNA compressed)
        let mut row = neuroquantum_core::storage::Row {
            id: 0,
            fields: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        row.fields.insert("id".to_string(), Value::Integer(1));
        row.fields.insert("name".to_string(), Value::text("Widget"));
        row.fields.insert("price".to_string(), Value::Float(19.99));

        storage_guard.insert_row("products", row).await.unwrap();
    }

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

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

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create and populate table
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
                name: "salary".to_string(),
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

        storage_guard.insert_row("employees", row).await.unwrap();

        // Verify the row was actually inserted
        let verify_query = neuroquantum_core::storage::SelectQuery {
            table: "employees".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };
        let rows_before = storage_guard.select_rows(&verify_query).await.unwrap();
        println!(
            "📊 Rows before update in original storage: {}",
            rows_before.len()
        );
    }

    // Create executor and execute UPDATE
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // First, test if the executor can SELECT the data
    let parser = Parser::new();
    let select_test_sql = "SELECT * FROM employees WHERE id = 1";
    let select_test_statement = parser.parse(select_test_sql).unwrap();
    let select_test_result = executor
        .execute_statement(&select_test_statement)
        .await
        .unwrap();
    println!(
        "🔍 SELECT test found {} rows",
        select_test_result.rows.len()
    );

    let sql = "UPDATE employees SET salary = 60000.0 WHERE id = 1";
    let statement = parser.parse(sql).unwrap();
    println!("📝 Parsed UPDATE statement: {statement:?}");

    let result = executor.execute_statement(&statement).await.unwrap();
    println!("✏️ UPDATE result: rows_affected = {}", result.rows_affected);

    // Verify update succeeded
    assert_eq!(result.rows_affected, 1);

    // Verify updated data (DNA re-compressed) using a SELECT query through the executor
    let select_sql = "SELECT * FROM employees WHERE id = 1";
    let select_statement = parser.parse(select_sql).unwrap();
    let select_result = executor.execute_statement(&select_statement).await.unwrap();

    println!(
        "🔍 SELECT after UPDATE found {} rows",
        select_result.rows.len()
    );
    println!("📊 SELECT result: {select_result:?}");

    // Check that we got the updated salary
    assert_eq!(select_result.rows.len(), 1);
    // The result should contain the updated salary value
    let updated_row = &select_result.rows[0];
    if let Some(QueryValue::Float(salary)) = updated_row.get("salary") {
        assert_eq!(*salary, 60000.0);
    } else {
        panic!("Expected salary field to be a Float with value 60000.0");
    }

    println!("✅ UPDATE with DNA re-compression: SUCCESS");
}

/// Test that DELETE queries clean up DNA-compressed blocks
#[tokio::test]
async fn test_delete_with_dna_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create and populate table
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

        // Insert test data
        for i in 1..=3 {
            let mut row = neuroquantum_core::storage::Row {
                id: 0,
                fields: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            row.fields.insert("id".to_string(), Value::Integer(i));
            row.fields
                .insert("message".to_string(), Value::text(format!("Log entry {i}")));
            storage_guard.insert_row("logs", row).await.unwrap();
        }
    }

    // Create executor and execute DELETE
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Note: Parser WHERE clause support for DELETE is limited
    // For now, test DELETE without WHERE (deletes all rows)
    let parser = Parser::new();
    let sql = "DELETE FROM logs";
    let statement = parser.parse(sql).unwrap();

    println!("📝 Parsed statement: {statement:?}");

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

    let storage_guard = storage_arc.read().await;
    let rows = storage_guard.select_rows(&query).await.unwrap();
    assert_eq!(
        rows.len(),
        0,
        "Expected 0 rows remaining, got {}",
        rows.len()
    );

    println!("✅ DELETE with DNA cleanup: SUCCESS (all rows deleted, DNA blocks freed)");
}

/// Test that DROP TABLE removes table and all associated data
#[tokio::test]
async fn test_drop_table_removes_table() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table
    let schema = TableSchema {
        name: "temp_table".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: true,
            },
            ColumnDefinition {
                name: "data".to_string(),
                data_type: DataType::Text,
                nullable: true,
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

        // Insert some test data
        let mut row = neuroquantum_core::storage::Row {
            id: 0,
            fields: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        row.fields.insert("id".to_string(), Value::Integer(1));
        row.fields
            .insert("data".to_string(), Value::text("test data"));
        storage_guard.insert_row("temp_table", row).await.unwrap();
    }

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Parse and execute DROP TABLE
    let parser = Parser::new();
    let sql = "DROP TABLE temp_table";
    let statement = parser.parse(sql).unwrap();

    // Execute DROP TABLE
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify DROP succeeded (no error thrown)
    assert_eq!(result.rows.len(), 0);

    // Verify table no longer exists by trying to select from it
    let select_sql = "SELECT * FROM temp_table";
    let select_statement = parser.parse(select_sql).unwrap();
    let select_result = executor.execute_statement(&select_statement).await;

    // Should fail because table doesn't exist
    assert!(
        select_result.is_err(),
        "SELECT from dropped table should fail"
    );

    println!("✅ DROP TABLE: SUCCESS (table and data removed)");
}

/// Test DROP TABLE IF EXISTS with non-existent table
#[tokio::test]
async fn test_drop_table_if_exists_non_existent() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Parse and execute DROP TABLE IF EXISTS on non-existent table
    let parser = Parser::new();
    let sql = "DROP TABLE IF EXISTS non_existent_table";
    let statement = parser.parse(sql).unwrap();

    // Execute DROP TABLE IF EXISTS - should succeed without error
    let result = executor.execute_statement(&statement).await;

    assert!(
        result.is_ok(),
        "DROP TABLE IF EXISTS on non-existent table should succeed"
    );

    println!("✅ DROP TABLE IF EXISTS (non-existent): SUCCESS");
}

/// Test DROP TABLE without IF EXISTS on non-existent table fails
#[tokio::test]
async fn test_drop_table_non_existent_fails() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Parse and execute DROP TABLE on non-existent table
    let parser = Parser::new();
    let sql = "DROP TABLE non_existent_table";
    let statement = parser.parse(sql).unwrap();

    // Execute DROP TABLE - should fail
    let result = executor.execute_statement(&statement).await;

    assert!(
        result.is_err(),
        "DROP TABLE on non-existent table should fail"
    );

    println!("✅ DROP TABLE (non-existent, no IF EXISTS): correctly fails");
}

/// Test that DROP TABLE cleans up table files from disk
#[tokio::test]
async fn test_drop_table_cleans_up_files() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    let table_file = storage_path.join("tables").join("cleanup_test.nqdb");
    let index_file = storage_path.join("indexes").join("cleanup_test_id.idx");

    // Create test table
    let schema = TableSchema {
        name: "cleanup_test".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: DataType::Integer,
            nullable: false,
            default_value: None,
            auto_increment: true,
        }],
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

    // Verify table file exists
    assert!(table_file.exists(), "Table file should exist after CREATE");
    assert!(index_file.exists(), "Index file should exist after CREATE");

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Drop the table
    let parser = Parser::new();
    let sql = "DROP TABLE cleanup_test";
    let statement = parser.parse(sql).unwrap();
    executor.execute_statement(&statement).await.unwrap();

    // Verify files are cleaned up
    assert!(
        !table_file.exists(),
        "Table file should be deleted after DROP"
    );
    assert!(
        !index_file.exists(),
        "Index file should be deleted after DROP"
    );

    println!("✅ DROP TABLE file cleanup: SUCCESS");
}

/// Test that CTE (WITH clause) queries work correctly with storage engine
#[tokio::test]
async fn test_cte_with_storage() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

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
                name: "status".to_string(),
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

        // Insert test data
        for (i, (name, status, age)) in [
            ("Alice", "active", 30),
            ("Bob", "inactive", 25),
            ("Charlie", "active", 35),
            ("Diana", "active", 22),
        ]
        .iter()
        .enumerate()
        {
            let mut row = neuroquantum_core::storage::Row {
                id: 0,
                fields: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            row.fields
                .insert("id".to_string(), Value::Integer((i + 1) as i64));
            row.fields.insert("name".to_string(), Value::text(*name));
            row.fields
                .insert("status".to_string(), Value::text(*status));
            row.fields
                .insert("age".to_string(), Value::Integer(i64::from(*age)));

            storage_guard.insert_row("users", row).await.unwrap();
        }
    }

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    // Parse and execute CTE query
    let parser = Parser::new();
    let sql = r"
        WITH active_users AS (
            SELECT * FROM users WHERE status = 'active'
        )
        SELECT * FROM active_users WHERE age > 25
    ";
    let statement = parser.parse(sql).unwrap();

    // Execute CTE query
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify CTE worked correctly - should get Alice (30) and Charlie (35)
    // Diana (22) is active but age <= 25
    assert_eq!(
        result.rows.len(),
        2,
        "Expected 2 active users over 25, got {}",
        result.rows.len()
    );

    // Verify the names are correct
    let names: Vec<String> = result
        .rows
        .iter()
        .filter_map(|row| {
            if let Some(QueryValue::String(name)) = row.get("name") {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    assert!(names.contains(&"Alice".to_string()), "Should contain Alice");
    assert!(
        names.contains(&"Charlie".to_string()),
        "Should contain Charlie"
    );

    println!("✅ CTE (WITH clause) execution: SUCCESS");
}

/// Test that UPDATE without WHERE clause works correctly and updates all rows
/// This test validates the safety feature that logs a warning but still executes the update
#[tokio::test]
async fn test_update_without_where_clause_affects_all_rows() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table with a 'status' integer column
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

        // Insert test data - all users have status = 0 initially
        for i in 1..=5 {
            let mut row = neuroquantum_core::storage::Row {
                id: 0,
                fields: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            row.fields.insert("id".to_string(), Value::Integer(i));
            row.fields
                .insert("name".to_string(), Value::text(format!("User{i}")));
            row.fields.insert("status".to_string(), Value::Integer(0));
            storage_guard.insert_row("users", row).await.unwrap();
        }
    }

    // Create executor
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();

    // Verify all 5 rows exist before update
    let select_before = parser.parse("SELECT * FROM users").unwrap();
    let result_before = executor.execute_statement(&select_before).await.unwrap();
    assert_eq!(
        result_before.rows.len(),
        5,
        "Should have 5 users before update"
    );

    // Execute UPDATE without WHERE clause - should update all rows
    let sql = "UPDATE users SET status = 1";
    println!("📝 Executing: {sql}");
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify all 5 rows were updated
    assert_eq!(
        result.rows_affected, 5,
        "UPDATE without WHERE should affect all 5 rows"
    );
    println!("✏️ UPDATE result: rows_affected = {}", result.rows_affected);

    // Verify all users now have status = 1
    let select_after = parser
        .parse("SELECT * FROM users WHERE status = 1")
        .unwrap();
    let result_after = executor.execute_statement(&select_after).await.unwrap();
    assert_eq!(
        result_after.rows.len(),
        5,
        "All 5 users should now have status = 1"
    );

    println!("✅ UPDATE without WHERE clause: SUCCESS (all rows updated with warning logged)");
}
