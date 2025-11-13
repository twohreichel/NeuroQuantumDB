//! Simple test to isolate the serialization error

use neuroquantum_core::storage::{
    ColumnDefinition, DataType, Row, StorageEngine, TableSchema, Value,
};
use std::collections::HashMap;
use tempfile::TempDir;

#[tokio::test]
async fn test_simple_insert() {
    let temp_dir = TempDir::new().unwrap();
    let data_path = temp_dir.path();

    println!("Creating storage engine...");
    let mut storage = StorageEngine::new(data_path)
        .await
        .expect("Failed to create storage engine");

    println!("Creating table schema...");
    let schema = TableSchema {
        name: "simple_test".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: DataType::Integer,
            nullable: false,
            default_value: None,
        }],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
    };

    println!("Creating table...");
    match storage.create_table(schema.clone()).await {
        Ok(_) => println!("✅ Table created"),
        Err(e) => {
            println!("❌ Failed to create table: {}", e);
            panic!("Table creation failed");
        }
    }

    println!("Creating row...");
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Integer(1));

    let row = Row {
        id: 0,
        fields,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    println!("Inserting row...");
    match storage.insert_row("simple_test", row).await {
        Ok(id) => println!("✅ Row inserted with ID: {}", id),
        Err(e) => {
            println!("❌ Failed to insert row: {}", e);
            println!("Error details: {:?}", e);
            panic!("Insert failed: {}", e);
        }
    }

    println!("🎉 Test passed!");
}
