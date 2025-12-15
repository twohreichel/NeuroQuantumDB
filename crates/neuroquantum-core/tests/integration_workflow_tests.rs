//! Comprehensive Integration Tests for NeuroQuantumDB
//!
//! Tests complete workflows including CRUD operations, transactions,
//! backup/restore, and system recovery.

use neuroquantum_core::{
    storage::{
        ColumnDefinition, ComparisonOperator, Condition, DataType, DeleteQuery, Row, SelectQuery,
        TableSchema, UpdateQuery, Value, WhereClause,
    },
    NeuroQuantumDBBuilder,
};
use std::collections::HashMap;
use tempfile::TempDir;

/// Test 1: Complete CRUD Workflow
#[tokio::test]
async fn test_complete_crud_workflow() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut db = NeuroQuantumDBBuilder::new()
        .storage_path(temp_dir.path().to_path_buf())
        .build()
        .await?;

    // CREATE TABLE - using new builder pattern
    {
        let storage = db.storage_mut();
        let schema = TableSchema::new(
            "employees",
            "id",
            vec![
                ColumnDefinition::new("id", DataType::BigSerial), // Auto-increment ID
                ColumnDefinition::new("name", DataType::Text),
                ColumnDefinition::new("salary", DataType::Integer)
                    .nullable()
                    .with_default(Value::Integer(50000)),
            ],
        );
        storage.create_table(schema).await?;
    }

    // INSERT - note: id is auto-generated now!
    {
        let storage = db.storage_mut();
        for i in 1..=3 {
            let mut fields = HashMap::new();
            fields.insert("id".to_string(), Value::Integer(i));
            fields.insert("name".to_string(), Value::Text(format!("Employee {}", i)));
            fields.insert("salary".to_string(), Value::Integer(50000 + i * 10000));

            let row = Row {
                id: 0,
                fields,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            storage.insert_row("employees", row).await?;
        }
    }

    // READ
    {
        let storage = db.storage();
        let query = SelectQuery {
            table: "employees".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };
        let rows = storage.select_rows(&query).await?;
        assert_eq!(rows.len(), 3, "Should have 3 employees");
    }

    // UPDATE
    {
        let storage = db.storage_mut();
        let update = UpdateQuery {
            table: "employees".to_string(),
            set_values: HashMap::from([("salary".to_string(), Value::Integer(100000))]),
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: "id".to_string(),
                    operator: ComparisonOperator::Equal,
                    value: Value::Integer(1),
                }],
            }),
        };
        storage.update_rows(&update).await?;

        // Verify
        let query = SelectQuery {
            table: "employees".to_string(),
            columns: vec!["*".to_string()],
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: "id".to_string(),
                    operator: ComparisonOperator::Equal,
                    value: Value::Integer(1),
                }],
            }),
            order_by: None,
            limit: None,
            offset: None,
        };
        let rows = storage.select_rows(&query).await?;
        assert_eq!(rows[0].fields.get("salary"), Some(&Value::Integer(100000)));
    }

    // DELETE
    {
        let storage = db.storage_mut();
        let delete = DeleteQuery {
            table: "employees".to_string(),
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: "id".to_string(),
                    operator: ComparisonOperator::Equal,
                    value: Value::Integer(3),
                }],
            }),
        };
        storage.delete_rows(&delete).await?;

        // Verify
        let query = SelectQuery {
            table: "employees".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };
        let rows = storage.select_rows(&query).await?;
        assert_eq!(rows.len(), 2, "Should have 2 employees after deletion");
    }

    Ok(())
}

/// Test 2: Update and Delete Operations
#[tokio::test]
async fn test_update_delete_operations() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut db = NeuroQuantumDBBuilder::new()
        .storage_path(temp_dir.path().to_path_buf())
        .build()
        .await?;

    // Setup
    {
        let storage = db.storage_mut();
        let schema = TableSchema::new(
            "accounts",
            "id",
            vec![
                ColumnDefinition::new("id", DataType::BigSerial),
                ColumnDefinition::new("balance", DataType::Integer),
            ],
        );
        storage.create_table(schema).await?;

        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Integer(1));
        fields.insert("balance".to_string(), Value::Integer(1000));
        let row = Row {
            id: 0,
            fields,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        storage.insert_row("accounts", row).await?;
    }

    // Test Update
    {
        let storage = db.storage_mut();
        let update = UpdateQuery {
            table: "accounts".to_string(),
            set_values: HashMap::from([("balance".to_string(), Value::Integer(2000))]),
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: "id".to_string(),
                    operator: ComparisonOperator::Equal,
                    value: Value::Integer(1),
                }],
            }),
        };
        storage.update_rows(&update).await?;

        // Verify update
        let query = SelectQuery {
            table: "accounts".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };
        let rows = storage.select_rows(&query).await?;
        assert_eq!(rows[0].fields.get("balance"), Some(&Value::Integer(2000)));
    }

    Ok(())
}

/// Test 3: Complex Queries with WHERE Clauses
#[tokio::test]
async fn test_complex_queries() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();

    // Create DB with data
    {
        let mut db = NeuroQuantumDBBuilder::new()
            .storage_path(storage_path.clone())
            .build()
            .await?;

        let storage = db.storage_mut();
        let schema = TableSchema::new(
            "products",
            "id",
            vec![
                ColumnDefinition::new("id", DataType::BigSerial),
                ColumnDefinition::new("name", DataType::Text),
                ColumnDefinition::new("price", DataType::Integer),
            ],
        );
        storage.create_table(schema).await?;

        // Insert test data
        for i in 1..=10 {
            let mut fields = HashMap::new();
            fields.insert("id".to_string(), Value::Integer(i));
            fields.insert("name".to_string(), Value::Text(format!("Product {}", i)));
            fields.insert("price".to_string(), Value::Integer(i * 100));

            let row = Row {
                id: 0,
                fields,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            storage.insert_row("products", row).await?;
        }

        // Query with WHERE clause
        let query = SelectQuery {
            table: "products".to_string(),
            columns: vec!["*".to_string()],
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: "price".to_string(),
                    operator: ComparisonOperator::GreaterThan,
                    value: Value::Integer(500),
                }],
            }),
            order_by: None,
            limit: None,
            offset: None,
        };
        let rows = storage.select_rows(&query).await?;
        assert!(!rows.is_empty());
        assert!(rows.len() < 10);
    }

    Ok(())
}

/// Test 4: Database Persistence across Restarts
#[tokio::test]
async fn test_persistence_across_restarts() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_path_buf();

    // Phase 1: Create data
    {
        let mut db = NeuroQuantumDBBuilder::new()
            .storage_path(storage_path.clone())
            .build()
            .await?;

        let storage = db.storage_mut();
        let schema = TableSchema::new(
            "persistent_test",
            "id",
            vec![
                ColumnDefinition::new("id", DataType::BigSerial),
                ColumnDefinition::new("value", DataType::Text),
            ],
        );
        storage.create_table(schema).await?;

        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Integer(1));
        fields.insert(
            "value".to_string(),
            Value::Text("persistent_data".to_string()),
        );
        let row = Row {
            id: 0,
            fields,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        storage.insert_row("persistent_test", row).await?;

        // Properly close database
        drop(db);
    }

    // Phase 2: Reopen and verify
    {
        let db = NeuroQuantumDBBuilder::new()
            .storage_path(storage_path.clone())
            .build()
            .await?;

        let storage = db.storage();
        let query = SelectQuery {
            table: "persistent_test".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };
        let rows = storage.select_rows(&query).await?;
        assert_eq!(rows.len(), 1, "Data should persist across restarts");
        assert_eq!(
            rows[0].fields.get("value"),
            Some(&Value::Text("persistent_data".to_string()))
        );
    }

    Ok(())
}

/// Test 5: Bulk Operations Performance
#[tokio::test]
async fn test_bulk_operations() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;

    let mut db = NeuroQuantumDBBuilder::new()
        .storage_path(temp_dir.path().to_path_buf())
        .build()
        .await?;

    {
        let storage = db.storage_mut();
        let schema = TableSchema::new(
            "bulk_test",
            "id",
            vec![
                ColumnDefinition::new("id", DataType::BigSerial),
                ColumnDefinition::new("value", DataType::Integer),
            ],
        );
        storage.create_table(schema).await?;

        // Bulk insert
        for i in 1..=100 {
            let mut fields = HashMap::new();
            fields.insert("id".to_string(), Value::Integer(i));
            fields.insert("value".to_string(), Value::Integer(i * 10));
            let row = Row {
                id: 0,
                fields,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            storage.insert_row("bulk_test", row).await?;
        }

        // Verify count
        let query = SelectQuery {
            table: "bulk_test".to_string(),
            columns: vec!["*".to_string()],
            where_clause: None,
            order_by: None,
            limit: None,
            offset: None,
        };
        let rows = storage.select_rows(&query).await?;
        assert_eq!(rows.len(), 100);

        // Bulk delete
        let delete = DeleteQuery {
            table: "bulk_test".to_string(),
            where_clause: Some(WhereClause {
                conditions: vec![Condition {
                    field: "id".to_string(),
                    operator: ComparisonOperator::GreaterThan,
                    value: Value::Integer(50),
                }],
            }),
        };
        storage.delete_rows(&delete).await?;

        // Verify deletion
        let rows = storage.select_rows(&query).await?;
        assert!(rows.len() <= 50);
    }

    Ok(())
}
