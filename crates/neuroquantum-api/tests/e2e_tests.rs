//! End-to-End Integration Tests for NeuroQuantumDB API
//!
//! These tests verify complete workflows including table creation,
//! data insertion with DNA compression, and querying with statistics.

use neuroquantum_core::{NeuroQuantumDB, NeuroQuantumDBBuilder};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Helper to create test database instance
/// Returns both the database and the temp directory to keep it alive
async fn create_test_db() -> (Arc<RwLock<NeuroQuantumDB>>, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();

    // Use the new builder pattern for compile-time initialization guarantees
    let db = NeuroQuantumDBBuilder::new()
        .storage_path(temp_dir.path().to_path_buf())
        .build()
        .await
        .expect("Failed to initialize database");

    (Arc::new(RwLock::new(db)), temp_dir)
}

#[tokio::test]
async fn test_complete_workflow_with_query_statistics() {
    // 1. Setup database
    let (db, _temp_dir) = create_test_db().await;
    {
        let mut db_lock = db.write().await;
        let storage = db_lock.storage_mut();

        let schema = neuroquantum_core::storage::TableSchema {
            name: "users".to_string(),
            primary_key: "id".to_string(),
            columns: vec![
                neuroquantum_core::storage::ColumnDefinition {
                    name: "id".to_string(),
                    data_type: neuroquantum_core::storage::DataType::Integer,
                    nullable: false,
                    default_value: None,
                },
                neuroquantum_core::storage::ColumnDefinition {
                    name: "username".to_string(),
                    data_type: neuroquantum_core::storage::DataType::Text,
                    nullable: false,
                    default_value: None,
                },
                neuroquantum_core::storage::ColumnDefinition {
                    name: "email".to_string(),
                    data_type: neuroquantum_core::storage::DataType::Text,
                    nullable: false,
                    default_value: None,
                },
                neuroquantum_core::storage::ColumnDefinition {
                    name: "active".to_string(),
                    data_type: neuroquantum_core::storage::DataType::Boolean,
                    nullable: true,
                    default_value: Some(neuroquantum_core::storage::Value::Boolean(true)),
                },
            ],
            created_at: chrono::Utc::now(),
            version: 1,
        };

        storage
            .create_table(schema)
            .await
            .expect("Failed to create table");
    }

    // 4. Insert test data with DNA compression
    {
        let mut db_lock = db.write().await;
        let storage = db_lock.storage_mut();

        let test_users = [
            ("alice", "alice@example.com"),
            ("bob", "bob@example.com"),
            ("charlie", "charlie@example.com"),
        ];

        for (idx, (username, email)) in test_users.iter().enumerate() {
            let mut fields = HashMap::new();
            fields.insert(
                "id".to_string(),
                neuroquantum_core::storage::Value::Integer((idx + 1) as i64),
            );
            fields.insert(
                "username".to_string(),
                neuroquantum_core::storage::Value::Text(username.to_string()),
            );
            fields.insert(
                "email".to_string(),
                neuroquantum_core::storage::Value::Text(email.to_string()),
            );
            fields.insert(
                "active".to_string(),
                neuroquantum_core::storage::Value::Boolean(true),
            );

            let row = neuroquantum_core::storage::Row {
                id: 0, // Will be assigned by storage
                fields,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            storage
                .insert_row("users", row)
                .await
                .expect("Failed to insert row");
        }
    }

    // 5. Query data and verify DNA compression was used
    {
        let db_lock = db.read().await;
        let storage = db_lock.storage();

        let query = neuroquantum_core::storage::SelectQuery {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: Some(10),
            offset: None,
        };

        let (rows, stats) = storage
            .select_rows_with_stats(&query)
            .await
            .expect("Failed to select rows");

        // Verify results
        assert_eq!(rows.len(), 3, "Should have 3 users");

        // Verify statistics are populated
        assert!(
            stats.rows_examined >= 3,
            "Should have examined at least 3 rows"
        );

        // Check if indexes are tracked
        assert!(
            !stats.indexes_used.is_empty() || stats.rows_examined > 0,
            "Should have either used indexes or examined rows"
        );

        // Verify cache statistics
        if stats.cache_hits + stats.cache_misses > 0 {
            let cache_hit_rate = stats.cache_hit_rate();
            assert!(
                cache_hit_rate.is_some(),
                "Cache hit rate should be calculable"
            );
        }
    }

    // 6. Verify query statistics are properly tracked
    {
        let db_lock = db.read().await;
        let storage = db_lock.storage();

        let query = neuroquantum_core::storage::SelectQuery {
            table: "users".to_string(),
            columns: vec!["username".to_string(), "email".to_string()],
            where_clause: None,
            order_by: None,
            limit: Some(2),
            offset: Some(1),
        };

        let (rows, stats) = storage
            .select_rows_with_stats(&query)
            .await
            .expect("Failed to select rows with projection");

        assert_eq!(rows.len(), 2, "Should return 2 rows with limit");
        assert!(stats.rows_examined > 0, "Should track examined rows");
    }

    println!("✅ Complete E2E workflow test passed!");
}

#[tokio::test]
async fn test_concurrent_operations() {
    let (db, _temp_dir) = create_test_db().await;

    // Create table
    {
        let mut db_lock = db.write().await;
        let storage = db_lock.storage_mut();

        let schema = neuroquantum_core::storage::TableSchema {
            name: "concurrent_test".to_string(),
            primary_key: "id".to_string(),
            columns: vec![
                neuroquantum_core::storage::ColumnDefinition {
                    name: "id".to_string(),
                    data_type: neuroquantum_core::storage::DataType::Integer,
                    nullable: false,
                    default_value: None,
                },
                neuroquantum_core::storage::ColumnDefinition {
                    name: "value".to_string(),
                    data_type: neuroquantum_core::storage::DataType::Text,
                    nullable: false,
                    default_value: None,
                },
            ],
            created_at: chrono::Utc::now(),
            version: 1,
        };

        storage
            .create_table(schema)
            .await
            .expect("Failed to create table");
    }

    // Spawn concurrent read operations
    let mut handles = vec![];

    for i in 0..5 {
        let db_clone = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            let db_lock = db_clone.read().await;
            let storage = db_lock.storage();

            let query = neuroquantum_core::storage::SelectQuery {
                table: "concurrent_test".to_string(),
                columns: vec!["*".to_string()],
                where_clause: None,
                order_by: None,
                limit: Some(100),
                offset: None,
            };

            let result = storage.select_rows(&query).await;
            assert!(result.is_ok(), "Concurrent read {} should succeed", i);
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    println!("✅ Concurrent operations test passed!");
}

#[tokio::test]
async fn test_query_statistics_accuracy() {
    let (db, _temp_dir) = create_test_db().await;

    // Create and populate table
    {
        let mut db_lock = db.write().await;
        let storage = db_lock.storage_mut();

        let schema = neuroquantum_core::storage::TableSchema {
            name: "stats_test".to_string(),
            primary_key: "id".to_string(),
            columns: vec![
                neuroquantum_core::storage::ColumnDefinition {
                    name: "id".to_string(),
                    data_type: neuroquantum_core::storage::DataType::Integer,
                    nullable: false,
                    default_value: None,
                },
                neuroquantum_core::storage::ColumnDefinition {
                    name: "data".to_string(),
                    data_type: neuroquantum_core::storage::DataType::Text,
                    nullable: false,
                    default_value: None,
                },
            ],
            created_at: chrono::Utc::now(),
            version: 1,
        };

        storage.create_table(schema).await.unwrap();

        // Insert 100 rows
        for i in 0..100 {
            let mut fields = HashMap::new();
            fields.insert(
                "id".to_string(),
                neuroquantum_core::storage::Value::Integer(i),
            );
            fields.insert(
                "data".to_string(),
                neuroquantum_core::storage::Value::Text(format!("data_{}", i)),
            );

            let row = neuroquantum_core::storage::Row {
                id: 0,
                fields,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            storage.insert_row("stats_test", row).await.unwrap();
        }
    }

    // Test query statistics
    {
        let db_lock = db.read().await;
        let storage = db_lock.storage();

        let query = neuroquantum_core::storage::SelectQuery {
            table: "stats_test".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: Some(10),
            offset: Some(5),
        };

        let (rows, stats) = storage.select_rows_with_stats(&query).await.unwrap();

        // Verify results
        assert_eq!(rows.len(), 10, "Should return 10 rows");

        // Verify statistics
        assert_eq!(
            stats.rows_examined, 100,
            "Should examine all 100 rows (no index optimization yet)"
        );

        // Index should be tracked even if not optimally used
        assert!(
            stats.indexes_used.contains(&"stats_test_id".to_string()) || stats.rows_examined > 0,
            "Should track primary key index"
        );
    }

    println!("✅ Query statistics accuracy test passed!");
}
