//! Simple test to isolate the serialization error

use neuroquantum_core::storage::{
    ColumnDefinition, DataType, IdGenerationStrategy, Row, StorageEngine, TableSchema,
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
            data_type: DataType::BigSerial, // Auto-increment ID
            nullable: false,
            default_value: None,
            auto_increment: true,
        }],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: IdGenerationStrategy::AutoIncrement,
    };

    println!("Creating table...");
    match storage.create_table(schema.clone()).await {
        | Ok(_) => println!("✅ Table created"),
        | Err(e) => {
            println!("❌ Failed to create table: {}", e);
            panic!("Table creation failed");
        },
    }

    // Test: Insert without specifying ID - it should be auto-generated!
    println!("Creating row WITHOUT id (should be auto-generated)...");
    let fields = HashMap::new();
    // Note: We intentionally DON'T set the id field - it should be auto-generated

    let row = Row {
        id: 0, // Placeholder, will be assigned by storage engine
        fields,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    println!("Inserting row...");
    match storage.insert_row("simple_test", row).await {
        | Ok(id) => println!("✅ Row inserted with ID: {}", id),
        | Err(e) => {
            println!("❌ Failed to insert row: {}", e);
            println!("Error details: {:?}", e);
            panic!("Insert failed: {}", e);
        },
    }

    println!("🎉 Test passed!");
}
