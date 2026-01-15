//! Integration tests for HAVING clause execution
//!
//! This test suite verifies that the HAVING clause works correctly with:
//! - Comparison operators: `=`, `<`, `>`, `<=`, `>=`, `<>`
//! - Aggregate functions: `COUNT()`, `SUM()`, `AVG()`, `MIN()`, `MAX()`
//! - Logical operators: `AND`, `OR`, `NOT`

use std::collections::HashMap;
use std::sync::Arc;

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use tempfile::TempDir;

/// Helper function to set up test data
async fn setup_orders_table(storage_arc: Arc<tokio::sync::RwLock<StorageEngine>>) {
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
                name: "category".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "amount".to_string(),
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

    let mut storage_guard = storage_arc.write().await;
    storage_guard.create_table(schema).await.unwrap();

    // Insert test data:
    // Electronics: 3 orders (100, 200, 300) = total 600, avg 200
    // Books: 2 orders (50, 100) = total 150, avg 75
    // Clothing: 4 orders (25, 50, 75, 100) = total 250, avg 62.5
    // Food: 1 order (30) = total 30, avg 30
    let orders = [
        ("Electronics", 100.0),
        ("Electronics", 200.0),
        ("Electronics", 300.0),
        ("Books", 50.0),
        ("Books", 100.0),
        ("Clothing", 25.0),
        ("Clothing", 50.0),
        ("Clothing", 75.0),
        ("Clothing", 100.0),
        ("Food", 30.0),
    ];

    for (i, (category, amount)) in orders.iter().enumerate() {
        let mut row = neuroquantum_core::storage::Row {
            id: 0,
            fields: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        row.fields
            .insert("id".to_string(), Value::Integer((i + 1) as i64));
        row.fields
            .insert("category".to_string(), Value::Text((*category).to_string()));
        row.fields
            .insert("amount".to_string(), Value::Float(*amount));
        storage_guard.insert_row("orders", row).await.unwrap();
    }
}

/// Test HAVING with COUNT(*) > N
#[tokio::test]
async fn test_having_count_greater_than() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) > 1";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics (3), Books (2), Clothing (4) - excludes Food (1)
    assert_eq!(result.rows.len(), 3);
}

/// Test HAVING with COUNT(*) >= N
#[tokio::test]
async fn test_having_count_greater_or_equal() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) >= 3";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics (3), Clothing (4)
    assert_eq!(result.rows.len(), 2);
}

/// Test HAVING with COUNT(*) = N
#[tokio::test]
async fn test_having_count_equal() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) = 2";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Books (2)
    assert_eq!(result.rows.len(), 1);
}

/// Test HAVING with COUNT(*) < N
#[tokio::test]
async fn test_having_count_less_than() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) < 3";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Books (2), Food (1)
    assert_eq!(result.rows.len(), 2);
}

/// Test HAVING with SUM aggregate
#[tokio::test]
async fn test_having_sum() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, SUM(amount) FROM orders GROUP BY category HAVING SUM(amount) > 200";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics (600), Clothing (250)
    assert_eq!(result.rows.len(), 2);
}

/// Test HAVING with AVG aggregate
#[tokio::test]
async fn test_having_avg() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, AVG(amount) FROM orders GROUP BY category HAVING AVG(amount) > 50";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics (200), Books (75), Clothing (62.5)
    assert_eq!(result.rows.len(), 3);
}

/// Test HAVING with logical AND
#[tokio::test]
async fn test_having_logical_and() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) > 1 AND COUNT(*) < 4";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics (3), Books (2)
    assert_eq!(result.rows.len(), 2);
}

/// Test HAVING with logical OR
#[tokio::test]
async fn test_having_logical_or() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) = 1 OR COUNT(*) = 4";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Food (1), Clothing (4)
    assert_eq!(result.rows.len(), 2);
}

/// Test HAVING with not equal operator
#[tokio::test]
async fn test_having_not_equal() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) <> 1";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics (3), Books (2), Clothing (4)
    assert_eq!(result.rows.len(), 3);
}

/// Test HAVING with MIN aggregate
#[tokio::test]
async fn test_having_min() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql = "SELECT category, MIN(amount) FROM orders GROUP BY category HAVING MIN(amount) >= 25";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics: min=100, Books: min=50, Clothing: min=25, Food: min=30
    // All satisfy MIN(amount) >= 25
    assert_eq!(result.rows.len(), 4);
}

/// Test HAVING with MAX aggregate
#[tokio::test]
async fn test_having_max() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    let sql =
        "SELECT category, MAX(amount) FROM orders GROUP BY category HAVING MAX(amount) >= 200";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics: max=300
    // Only Electronics has max >= 200
    assert_eq!(result.rows.len(), 1);
}

/// Test HAVING with NOT operator
#[tokio::test]
async fn test_having_not() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    setup_orders_table(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc).unwrap();
    let parser = Parser::new();

    // NOT COUNT(*) < 3 is equivalent to COUNT(*) >= 3
    let sql = "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING NOT COUNT(*) < 3";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Electronics (3), Clothing (4)
    assert_eq!(result.rows.len(), 2);
}
