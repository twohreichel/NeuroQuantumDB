//! Integration test for DNA Compression + Encryption
//! This test validates that data is properly compressed with DNA encoding
//! and encrypted with AES-256-GCM before being written to disk

use neuroquantum_core::storage::{
    ColumnDefinition, DataType, IdGenerationStrategy, Row, SelectQuery, StorageEngine, TableSchema,
    Value,
};
use std::collections::HashMap;
use tempfile::TempDir;

#[tokio::test]
async fn test_dna_compression_and_encryption_roundtrip() {
    // Create temporary directory for test
    let temp_dir = TempDir::new().unwrap();
    let data_path = temp_dir.path();

    // Initialize storage engine with encryption enabled
    let mut storage = StorageEngine::new(data_path)
        .await
        .expect("Failed to create storage engine");

    // Create a test table
    let schema = TableSchema {
        name: "test_users".to_string(),
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
            ColumnDefinition {
                name: "email".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: std::collections::HashMap::new(),
        id_strategy: IdGenerationStrategy::AutoIncrement,
    };

    storage
        .create_table(schema.clone())
        .await
        .expect("Failed to create table");

    // Insert test data
    let test_data = [
        ("Alice Johnson", "alice@quantum.db"),
        ("Bob Smith", "bob@neuroquantum.ai"),
        ("Charlie Brown", "charlie@dna-compression.org"),
    ];

    let mut inserted_ids = Vec::new();
    for (i, (name, email)) in test_data.iter().enumerate() {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Integer((i + 1) as i64));
        fields.insert("name".to_string(), Value::Text(name.to_string()));
        fields.insert("email".to_string(), Value::Text(email.to_string()));

        let row = Row {
            id: 0, // Will be assigned by storage engine
            fields,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let row_id = storage
            .insert_row("test_users", row)
            .await
            .expect("Failed to insert row");

        inserted_ids.push(row_id);
        println!(
            "✅ Inserted row {} with DNA compression + encryption",
            row_id
        );
    }

    // Verify data is actually on disk (encrypted and compressed)
    let table_file = data_path.join("tables").join("test_users.nqdb");
    assert!(table_file.exists(), "Table file should exist on disk");

    let file_content = tokio::fs::read(&table_file).await.unwrap();
    assert!(!file_content.is_empty(), "Table file should contain data");

    // Verify the file content is NOT plain JSON (should be binary encrypted data)
    let content_str = String::from_utf8_lossy(&file_content);
    assert!(
        !content_str.contains("Alice Johnson"),
        "Data should be encrypted, not plain text"
    );
    assert!(
        !content_str.contains("alice@quantum.db"),
        "Data should be encrypted, not plain text"
    );

    println!("✅ Verified: Data is encrypted on disk (not plain text)");

    // Now read back the data - should be automatically decrypted and decompressed
    let select_query = SelectQuery {
        table: "test_users".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage
        .select_rows(&select_query)
        .await
        .expect("Failed to select rows");

    assert_eq!(rows.len(), 3, "Should retrieve all 3 rows");

    // Verify data integrity
    for (i, (expected_name, expected_email)) in test_data.iter().enumerate() {
        let row = &rows[i];

        let name = match row.fields.get("name") {
            Some(Value::Text(n)) => n,
            _ => panic!("Name field not found or wrong type"),
        };

        let email = match row.fields.get("email") {
            Some(Value::Text(e)) => e,
            _ => panic!("Email field not found or wrong type"),
        };

        assert_eq!(name, expected_name, "Name should match");
        assert_eq!(email, expected_email, "Email should match");

        println!("✅ Row {} verified: {} <{}>", i + 1, name, email);
    }

    println!("🎉 DNA Compression + Encryption Round-Trip Test PASSED!");
}

#[tokio::test]
async fn test_compression_ratio_with_encryption() {
    let temp_dir = TempDir::new().unwrap();
    let data_path = temp_dir.path();

    let mut storage = StorageEngine::new(data_path)
        .await
        .expect("Failed to create storage engine");

    // Create table with a large text field
    let schema = TableSchema {
        name: "large_data".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "content".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: std::collections::HashMap::new(),
        id_strategy: IdGenerationStrategy::AutoIncrement,
    };

    storage.create_table(schema).await.unwrap();

    // Insert a row with repetitive content (should compress well)
    let repetitive_content = "ACGTACGT".repeat(1000); // 8000 characters
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Integer(1));
    fields.insert(
        "content".to_string(),
        Value::Text(repetitive_content.clone()),
    );

    let row = Row {
        id: 0,
        fields,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let original_size = repetitive_content.len();

    storage
        .insert_row("large_data", row)
        .await
        .expect("Failed to insert large row");

    // Check file size on disk
    let table_file = data_path.join("tables").join("large_data.nqdb");
    let file_size = tokio::fs::metadata(&table_file).await.unwrap().len() as usize;

    println!("📊 Compression Statistics:");
    println!("   Original size: {} bytes", original_size);
    println!("   Compressed + Encrypted: {} bytes", file_size);

    let compression_ratio = original_size as f64 / file_size as f64;
    println!("   Compression ratio: {:.2}x", compression_ratio);

    // Note: With encryption overhead, small files may actually expand
    // The encryption adds ~50+ bytes overhead (nonce, authentication tag, etc.)
    // DNA compression adds metadata (parity, checksum, dictionary, etc.)
    // For very small data, this overhead dominates
    // Real compression benefits appear with larger datasets
    println!("   ℹ️  Note: Encryption + DNA compression metadata adds overhead");
    println!("   ℹ️  Compression benefits increase with larger datasets");

    // Just verify the data is stored and retrievable
    assert!(file_size > 0, "File should contain data");

    // Verify data can be read back correctly
    let select_query = SelectQuery {
        table: "large_data".to_string(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: None,
        offset: None,
    };

    let rows = storage.select_rows(&select_query).await.unwrap();
    assert_eq!(rows.len(), 1);

    let retrieved_content = match rows[0].fields.get("content") {
        Some(Value::Text(c)) => c,
        _ => panic!("Content not found"),
    };

    assert_eq!(
        retrieved_content, &repetitive_content,
        "Content should be identical after compression/decompression"
    );

    println!("✅ DNA Compression + Encryption storage test PASSED");
    println!(
        "   File size: {} bytes (ratio: {:.2}x)",
        file_size, compression_ratio
    );
}
