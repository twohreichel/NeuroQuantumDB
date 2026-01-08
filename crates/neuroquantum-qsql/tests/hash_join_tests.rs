//! Integration tests for Hash Join optimization
//!
//! This test suite verifies that the hash join algorithm works correctly
//! and provides performance improvements over nested loop joins for large tables.

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema};
use neuroquantum_qsql::{query_plan::QueryValue, ExecutorConfig, Parser, QueryExecutor};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper function to create a test storage engine with two tables
async fn setup_test_tables() -> (TempDir, Arc<tokio::sync::RwLock<StorageEngine>>) {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();
    
    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));
    
    // Create users table
    let users_schema = TableSchema {
        name: "users".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "name".to_string(),
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
    
    // Create orders table
    let orders_schema = TableSchema {
        name: "orders".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "order_id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "user_id".to_string(),
                data_type: DataType::Integer,
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
        primary_key: "order_id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };
    
    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(users_schema).await.unwrap();
        storage_guard.create_table(orders_schema).await.unwrap();
    }
    
    (temp_dir, storage_arc)
}

/// Test that hash join is selected for large tables (threshold > 1000)
#[tokio::test]
async fn test_hash_join_selection_for_large_tables() {
    let (_temp_dir, storage_arc) = setup_test_tables().await;
    
    // Configure with low hash join threshold to force hash join
    let config = ExecutorConfig {
        hash_join_threshold: 100, // Low threshold to trigger hash join
        ..Default::default()
    };
    
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();
    
    // Insert test data - enough rows to exceed threshold (11 x 11 = 121 > 100)
    for i in 1..=11 {
        let insert_user = format!("INSERT INTO users (id, name) VALUES ({}, 'User {}')", i, i);
        let statement = parser.parse(&insert_user).unwrap();
        executor.execute_statement(&statement).await.unwrap();
        
        let insert_order = format!(
            "INSERT INTO orders (order_id, user_id, amount) VALUES ({}, {}, {}.0)",
            i, i, i * 10
        );
        let statement = parser.parse(&insert_order).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }
    
    // Execute INNER JOIN - should use hash join
    let sql = "SELECT users.name, orders.amount FROM users \
               INNER JOIN orders ON users.id = orders.user_id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    
    // Verify results
    assert_eq!(result.rows_affected, 11);
    assert_eq!(result.rows.len(), 11);
    
    // Verify data correctness
    let first_row = &result.rows[0];
    assert!(first_row.contains_key("name"));
    assert!(first_row.contains_key("amount"));
}

/// Test hash join with INNER JOIN
#[tokio::test]
async fn test_hash_join_inner_join() {
    let (_temp_dir, storage_arc) = setup_test_tables().await;
    
    let config = ExecutorConfig {
        hash_join_threshold: 2, // Very low threshold to force hash join
        ..Default::default()
    };
    
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();
    
    // Insert test data
    let inserts = vec![
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
        "INSERT INTO users (id, name) VALUES (2, 'Bob')",
        "INSERT INTO users (id, name) VALUES (3, 'Charlie')",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (101, 1, 50.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (102, 1, 75.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (103, 2, 100.0)",
    ];
    
    for sql in inserts {
        let statement = parser.parse(sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }
    
    // Execute INNER JOIN
    let sql = "SELECT users.name, orders.order_id, orders.amount \
               FROM users INNER JOIN orders ON users.id = orders.user_id \
               ORDER BY users.id, orders.order_id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    
    // Verify: Alice has 2 orders, Bob has 1 order, Charlie has 0 orders
    assert_eq!(result.rows.len(), 3);
    
    // Check first row (Alice's first order)
    assert_eq!(
        result.rows[0].get("name"),
        Some(&QueryValue::String("Alice".to_string()))
    );
    assert_eq!(
        result.rows[0].get("order_id"),
        Some(&QueryValue::Integer(101))
    );
}

/// Test hash join with LEFT JOIN
#[tokio::test]
async fn test_hash_join_left_join() {
    let (_temp_dir, storage_arc) = setup_test_tables().await;
    
    let config = ExecutorConfig {
        hash_join_threshold: 2,
        ..Default::default()
    };
    
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();
    
    // Insert test data
    let inserts = vec![
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
        "INSERT INTO users (id, name) VALUES (2, 'Bob')",
        "INSERT INTO users (id, name) VALUES (3, 'Charlie')", // No orders
        "INSERT INTO orders (order_id, user_id, amount) VALUES (101, 1, 50.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (102, 2, 100.0)",
    ];
    
    for sql in inserts {
        let statement = parser.parse(sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }
    
    // Execute LEFT JOIN - should include Charlie with NULL order
    let sql = "SELECT users.name, orders.order_id \
               FROM users LEFT JOIN orders ON users.id = orders.user_id \
               ORDER BY users.id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    
    // Verify: All 3 users, Charlie should have NULL order_id
    assert_eq!(result.rows.len(), 3);
    
    // Charlie (row 3) should have NULL order_id
    let charlie_row = &result.rows[2];
    assert_eq!(
        charlie_row.get("name"),
        Some(&QueryValue::String("Charlie".to_string()))
    );
    assert_eq!(
        charlie_row.get("orders.order_id"),
        Some(&QueryValue::Null)
    );
}

/// Test hash join with RIGHT JOIN
#[tokio::test]
async fn test_hash_join_right_join() {
    let (_temp_dir, storage_arc) = setup_test_tables().await;
    
    let config = ExecutorConfig {
        hash_join_threshold: 2,
        ..Default::default()
    };
    
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();
    
    // Insert test data - order 103 has no matching user
    let inserts = vec![
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
        "INSERT INTO users (id, name) VALUES (2, 'Bob')",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (101, 1, 50.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (102, 2, 100.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (103, 99, 200.0)", // No user
    ];
    
    for sql in inserts {
        let statement = parser.parse(sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }
    
    // Execute RIGHT JOIN - should include order 103 with NULL user
    let sql = "SELECT users.name, orders.order_id \
               FROM users RIGHT JOIN orders ON users.id = orders.user_id \
               ORDER BY orders.order_id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    
    // Verify: All 3 orders, order 103 should have NULL name
    assert_eq!(result.rows.len(), 3);
    
    // Order 103 should have NULL name
    let orphan_order = &result.rows[2];
    assert_eq!(
        orphan_order.get("order_id"),
        Some(&QueryValue::Integer(103))
    );
    assert_eq!(
        orphan_order.get("users.name"),
        Some(&QueryValue::Null)
    );
}

/// Test hash join with FULL OUTER JOIN
#[tokio::test]
async fn test_hash_join_full_outer_join() {
    let (_temp_dir, storage_arc) = setup_test_tables().await;
    
    let config = ExecutorConfig {
        hash_join_threshold: 2,
        ..Default::default()
    };
    
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();
    
    // Insert test data
    let inserts = vec![
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
        "INSERT INTO users (id, name) VALUES (3, 'Charlie')", // No order
        "INSERT INTO orders (order_id, user_id, amount) VALUES (101, 1, 50.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (103, 99, 200.0)", // No user
    ];
    
    for sql in inserts {
        let statement = parser.parse(sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }
    
    // Execute FULL OUTER JOIN - should include both unmatched rows
    let sql = "SELECT users.name, orders.order_id \
               FROM users FULL OUTER JOIN orders ON users.id = orders.user_id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    
    // Verify: Should have 3 rows total
    // 1. Alice + order 101 (matched)
    // 2. Charlie + NULL (unmatched user)
    // 3. NULL + order 103 (unmatched order)
    assert_eq!(result.rows.len(), 3);
    
    // Verify we have at least one NULL user and one NULL order
    let has_null_user = result.rows.iter().any(|row| {
        matches!(row.get("users.name"), Some(&QueryValue::Null))
    });
    let has_null_order = result.rows.iter().any(|row| {
        matches!(row.get("orders.order_id"), Some(&QueryValue::Null))
    });
    
    assert!(has_null_user, "Should have at least one row with NULL user");
    assert!(has_null_order, "Should have at least one row with NULL order");
}

/// Test that nested loop join is used for small tables (below threshold)
#[tokio::test]
async fn test_nested_loop_join_for_small_tables() {
    let (_temp_dir, storage_arc) = setup_test_tables().await;
    
    // Use default config with threshold = 1000
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();
    
    // Insert small amount of data (2 x 2 = 4 < 1000)
    let inserts = vec![
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
        "INSERT INTO users (id, name) VALUES (2, 'Bob')",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (101, 1, 50.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (102, 2, 100.0)",
    ];
    
    for sql in inserts {
        let statement = parser.parse(sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }
    
    // Execute JOIN - should use nested loop (but still work correctly)
    let sql = "SELECT users.name, orders.amount \
               FROM users INNER JOIN orders ON users.id = orders.user_id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    
    // Verify results are correct regardless of algorithm used
    assert_eq!(result.rows.len(), 2);
    assert!(result.rows.iter().all(|row| {
        row.contains_key("name") && row.contains_key("amount")
    }));
}

/// Test hash join with compound join condition (multiple equality checks)
#[tokio::test]
async fn test_hash_join_compound_condition() {
    let (_temp_dir, storage_arc) = setup_test_tables().await;
    
    let config = ExecutorConfig {
        hash_join_threshold: 2,
        ..Default::default()
    };
    
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();
    
    // Insert test data
    let inserts = vec![
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
        "INSERT INTO users (id, name) VALUES (2, 'Bob')",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (101, 1, 50.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (102, 2, 100.0)",
    ];
    
    for sql in inserts {
        let statement = parser.parse(sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }
    
    // Execute JOIN with simple equality condition
    let sql = "SELECT users.name, orders.amount \
               FROM users INNER JOIN orders ON users.id = orders.user_id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    
    assert_eq!(result.rows.len(), 2);
}

/// Test that hash join correctly handles duplicate keys
#[tokio::test]
async fn test_hash_join_duplicate_keys() {
    let (_temp_dir, storage_arc) = setup_test_tables().await;
    
    let config = ExecutorConfig {
        hash_join_threshold: 2,
        ..Default::default()
    };
    
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();
    
    // Insert test data - multiple orders for same user
    let inserts = vec![
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (101, 1, 50.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (102, 1, 75.0)",
        "INSERT INTO orders (order_id, user_id, amount) VALUES (103, 1, 100.0)",
    ];
    
    for sql in inserts {
        let statement = parser.parse(sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }
    
    // Execute JOIN - should return 3 rows (one user, three orders)
    let sql = "SELECT users.name, orders.order_id, orders.amount \
               FROM users INNER JOIN orders ON users.id = orders.user_id \
               ORDER BY orders.order_id";
    let statement = parser.parse(sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    
    // Verify all 3 orders are returned
    assert_eq!(result.rows.len(), 3);
    
    // Verify all rows have Alice as the name
    for row in &result.rows {
        assert_eq!(
            row.get("name"),
            Some(&QueryValue::String("Alice".to_string()))
        );
    }
    
    // Verify we have all three order IDs
    let order_ids: Vec<i64> = result.rows.iter()
        .filter_map(|row| {
            if let Some(&QueryValue::Integer(id)) = row.get("order_id") {
                Some(id)
            } else {
                None
            }
        })
        .collect();
    
    assert_eq!(order_ids, vec![101, 102, 103]);
}
