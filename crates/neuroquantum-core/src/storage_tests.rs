//! Tests for the Storage Engine
//! Comprehensive test suite for persistent file-based storage

use std::collections::HashMap;
use tokio;
use tempfile::TempDir;

use crate::storage::*;
use crate::dna::DNACompressor;

#[tokio::test]
async fn test_storage_engine_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await;

    assert!(storage.is_ok(), "Storage engine should initialize successfully");

    let storage = storage.unwrap();
    // Verify directory structure was created
    assert!(temp_dir.path().join("tables").exists());
    assert!(temp_dir.path().join("indexes").exists());
    assert!(temp_dir.path().join("logs").exists());
    assert!(temp_dir.path().join("quantum").exists());
    assert!(temp_dir.path().join("metadata.json").exists());
}

#[tokio::test]
async fn test_create_table() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    let schema = create_test_schema("users");
    let result = storage.create_table(schema.clone()).await;

    assert!(result.is_ok(), "Table creation should succeed");

    // Verify table file was created
    assert!(temp_dir.path().join("tables").join("users.nqdb").exists());

    // Verify index file was created
    assert!(temp_dir.path().join("indexes").join("users_id.idx").exists());

    // Test duplicate table creation fails
    let duplicate_result = storage.create_table(schema).await;
    assert!(duplicate_result.is_err(), "Duplicate table creation should fail");
}

#[tokio::test]
async fn test_insert_and_select_rows() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    let schema = create_test_schema("test_table");
    storage.create_table(schema).await.unwrap();

    // Insert test rows
    let row1 = create_test_row(1, "Alice");
    let row2 = create_test_row(2, "Bob");

    let id1 = storage.insert_row("test_table", row1.clone()).await.unwrap();
    let id2 = storage.insert_row("test_table", row2.clone()).await.unwrap();

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);

    // Test SELECT * query
    let select_query = SelectQuery {
        table: "test_table".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let results = storage.select_rows(&select_query).await.unwrap();
    assert_eq!(results.len(), 2);

    // Verify data integrity
    assert_eq!(results[0].fields.get("name").unwrap(), &Value::Text("Alice".to_string()));
    assert_eq!(results[1].fields.get("name").unwrap(), &Value::Text("Bob".to_string()));
}

#[tokio::test]
async fn test_where_clause_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    let schema = create_test_schema("filtering_test");
    storage.create_table(schema).await.unwrap();

    // Insert test data
    for i in 1..=5 {
        let row = create_test_row(i, &format!("User{}", i));
        storage.insert_row("filtering_test", row).await.unwrap();
    }

    // Test WHERE clause with equality
    let where_clause = WhereClause {
        conditions: vec![Condition {
            field: "id".to_string(),
            operator: ComparisonOperator::Equal,
            value: Value::Integer(3),
        }],
    };

    let select_query = SelectQuery {
        table: "filtering_test".to_string(),
        columns: vec!["*".to_string()],
        where_clause: Some(where_clause),
        order_by: None,
        limit: None,
        offset: None,
    };

    let results = storage.select_rows(&select_query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].fields.get("id").unwrap(), &Value::Integer(3));
}

#[tokio::test]
async fn test_update_rows() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    let schema = create_test_schema("update_test");
    storage.create_table(schema).await.unwrap();

    // Insert test row
    let row = create_test_row(1, "Original Name");
    storage.insert_row("update_test", row).await.unwrap();

    // Update the row
    let mut set_values = HashMap::new();
    set_values.insert("name".to_string(), Value::Text("Updated Name".to_string()));

    let where_clause = WhereClause {
        conditions: vec![Condition {
            field: "id".to_string(),
            operator: ComparisonOperator::Equal,
            value: Value::Integer(1),
        }],
    };

    let update_query = UpdateQuery {
        table: "update_test".to_string(),
        set_values,
        where_clause: Some(where_clause),
    };

    let updated_count = storage.update_rows(&update_query).await.unwrap();
    assert_eq!(updated_count, 1);

    // Verify the update
    let select_query = SelectQuery {
        table: "update_test".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let results = storage.select_rows(&select_query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].fields.get("name").unwrap(), &Value::Text("Updated Name".to_string()));
}

#[tokio::test]
async fn test_delete_rows() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    let schema = create_test_schema("delete_test");
    storage.create_table(schema).await.unwrap();

    // Insert test rows
    for i in 1..=3 {
        let row = create_test_row(i, &format!("User{}", i));
        storage.insert_row("delete_test", row).await.unwrap();
    }

    // Delete specific row
    let where_clause = WhereClause {
        conditions: vec![Condition {
            field: "id".to_string(),
            operator: ComparisonOperator::Equal,
            value: Value::Integer(2),
        }],
    };

    let delete_query = DeleteQuery {
        table: "delete_test".to_string(),
        where_clause: Some(where_clause),
    };

    let deleted_count = storage.delete_rows(&delete_query).await.unwrap();
    assert_eq!(deleted_count, 1);

    // Verify deletion
    let select_query = SelectQuery {
        table: "delete_test".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let results = storage.select_rows(&select_query).await.unwrap();
    assert_eq!(results.len(), 2);

    // Verify correct rows remain
    let ids: Vec<i64> = results.iter()
        .map(|r| match r.fields.get("id").unwrap() {
            Value::Integer(i) => *i,
            _ => panic!("Expected integer ID"),
        })
        .collect();

    assert!(ids.contains(&1));
    assert!(ids.contains(&3));
    assert!(!ids.contains(&2));
}

#[tokio::test]
async fn test_order_by_and_limit() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    let schema = create_test_schema("order_test");
    storage.create_table(schema).await.unwrap();

    // Insert test rows in random order
    let names = vec!["Charlie", "Alice", "Bob"];
    for (i, name) in names.iter().enumerate() {
        let row = create_test_row((i + 1) as i64, name);
        storage.insert_row("order_test", row).await.unwrap();
    }

    // Test ORDER BY with LIMIT
    let select_query = SelectQuery {
        table: "order_test".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: Some(OrderBy {
            field: "name".to_string(),
            direction: SortDirection::Ascending,
        }),
        limit: Some(2),
        offset: None,
    };

    let results = storage.select_rows(&select_query).await.unwrap();
    assert_eq!(results.len(), 2);

    // Verify ordering
    assert_eq!(results[0].fields.get("name").unwrap(), &Value::Text("Alice".to_string()));
    assert_eq!(results[1].fields.get("name").unwrap(), &Value::Text("Bob".to_string()));
}

#[tokio::test]
async fn test_persistence_across_restarts() {
    let temp_dir = TempDir::new().unwrap();

    // First session: create table and insert data
    {
        let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();
        let schema = create_test_schema("persistence_test");
        storage.create_table(schema).await.unwrap();

        let row = create_test_row(1, "Persistent Data");
        storage.insert_row("persistence_test", row).await.unwrap();

        // Explicitly flush to disk
        storage.flush_to_disk().await.unwrap();
    }

    // Second session: load existing data
    {
        let storage = StorageEngine::new(temp_dir.path()).await.unwrap();

        let select_query = SelectQuery {
            table: "persistence_test".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };

        let results = storage.select_rows(&select_query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].fields.get("name").unwrap(), &Value::Text("Persistent Data".to_string()));
    }
}

#[tokio::test]
async fn test_transaction_logging() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    let schema = create_test_schema("transaction_test");
    storage.create_table(schema).await.unwrap();

    // Insert data to generate transactions
    let row1 = create_test_row(1, "Transaction Test 1");
    let row2 = create_test_row(2, "Transaction Test 2");

    storage.insert_row("transaction_test", row1).await.unwrap();
    storage.insert_row("transaction_test", row2).await.unwrap();

    // Flush to ensure transaction log is written
    storage.flush_to_disk().await.unwrap();

    // Verify transaction log file exists
    assert!(temp_dir.path().join("logs").join("transaction.log").exists());
}

#[tokio::test]
async fn test_dna_compression_integration() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    let schema = create_test_schema("compression_test");
    storage.create_table(schema).await.unwrap();

    // Insert a row with larger data to test compression
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Integer(1));
    fields.insert("name".to_string(), Value::Text("A".repeat(1000))); // Large text data
    fields.insert("created_at".to_string(), Value::Timestamp(chrono::Utc::now()));

    let large_row = Row {
        id: 1,
        fields,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let result = storage.insert_row("compression_test", large_row).await;
    assert!(result.is_ok(), "Should successfully compress and store large data");

    // Verify we can retrieve the data correctly
    let select_query = SelectQuery {
        table: "compression_test".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let results = storage.select_rows(&select_query).await.unwrap();
    assert_eq!(results.len(), 1);

    // Verify data integrity after compression/decompression
    if let Value::Text(name) = results[0].fields.get("name").unwrap() {
        assert_eq!(name.len(), 1000);
        assert!(name.chars().all(|c| c == 'A'));
    } else {
        panic!("Expected text value");
    }

    // Flush and verify compressed blocks file exists
    storage.flush_to_disk().await.unwrap();
    assert!(temp_dir.path().join("quantum").join("compressed_blocks.qdata").exists());
}

#[tokio::test]
async fn test_schema_validation() {
    let temp_dir = TempDir::new().unwrap();
    let mut storage = StorageEngine::new(temp_dir.path()).await.unwrap();

    // Create test table
    let schema = create_test_schema("validation_test");
    storage.create_table(schema).await.unwrap();

    // Test inserting row with wrong data type
    let mut invalid_fields = HashMap::new();
    invalid_fields.insert("id".to_string(), Value::Text("not_an_integer".to_string())); // Should be integer
    invalid_fields.insert("name".to_string(), Value::Text("Valid Name".to_string()));
    invalid_fields.insert("created_at".to_string(), Value::Timestamp(chrono::Utc::now()));

    let invalid_row = Row {
        id: 1,
        fields: invalid_fields,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let result = storage.insert_row("validation_test", invalid_row).await;
    assert!(result.is_err(), "Should reject row with invalid data type");

    // Test inserting row with missing required field
    let mut incomplete_fields = HashMap::new();
    incomplete_fields.insert("id".to_string(), Value::Integer(2));
    // Missing required "name" field
    incomplete_fields.insert("created_at".to_string(), Value::Timestamp(chrono::Utc::now()));

    let incomplete_row = Row {
        id: 2,
        fields: incomplete_fields,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let result = storage.insert_row("validation_test", incomplete_row).await;
    assert!(result.is_err(), "Should reject row with missing required field");
}
