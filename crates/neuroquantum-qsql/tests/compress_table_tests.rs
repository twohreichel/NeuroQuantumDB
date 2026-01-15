//! Integration tests for COMPRESS TABLE functionality
//!
//! This test suite verifies COMPRESS TABLE operations including:
//! - Basic COMPRESS TABLE USING DNA
//! - Compression statistics reporting
//! - Error handling for non-existent tables

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema};
use neuroquantum_qsql::query_plan::QueryValue;
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use std::sync::Arc;
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
            ColumnDefinition {
                name: "severity".to_string(),
                data_type: DataType::Text,
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
    for i in 1..=10 {
        let sql = format!(
            "INSERT INTO logs (id, message, severity) VALUES ({i}, 'Log message {i}', 'INFO')"
        );
        let statement = parser.parse(&sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }

    (temp_dir, storage_arc, executor)
}

#[tokio::test]
async fn test_compress_table_basic() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Verify table has data
    let select_sql = "SELECT * FROM logs";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(
        result.rows.len(),
        10,
        "Table should have 10 rows before compression"
    );

    // Execute COMPRESS TABLE
    let compress_sql = "COMPRESS TABLE logs USING DNA";
    let statement = parser.parse(compress_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify compression result structure
    assert_eq!(
        result.columns.len(),
        7,
        "Compression result should have 7 columns"
    );
    assert_eq!(result.rows.len(), 1, "Should return one result row");

    // Verify column names
    let column_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        column_names,
        vec![
            "table_name",
            "algorithm",
            "original_size_bytes",
            "compressed_size_bytes",
            "compression_ratio_percent",
            "space_saved_bytes",
            "rows_compressed"
        ]
    );

    // Verify result values
    let row = &result.rows[0];

    // Check table_name
    assert_eq!(
        row.get("table_name"),
        Some(&QueryValue::String("logs".to_string())),
        "Table name should be 'logs'"
    );

    // Check algorithm
    assert_eq!(
        row.get("algorithm"),
        Some(&QueryValue::String("DNA".to_string())),
        "Algorithm should be 'DNA'"
    );

    // Verify rows_compressed
    assert_eq!(
        row.get("rows_compressed"),
        Some(&QueryValue::Integer(10)),
        "Should report 10 rows compressed"
    );

    // Verify rows_affected
    assert_eq!(result.rows_affected, 10, "Should affect 10 rows");

    // Verify optimization flag
    assert!(
        result.optimization_applied,
        "Compression should be marked as optimization applied"
    );

    // Verify data is still accessible after compression
    let select_sql = "SELECT * FROM logs";
    let statement = parser.parse(select_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();
    assert_eq!(
        result.rows.len(),
        10,
        "Table should still have 10 rows after compression"
    );
}

#[tokio::test]
async fn test_compress_table_statistics() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Execute COMPRESS TABLE
    let compress_sql = "COMPRESS TABLE logs USING DNA";
    let statement = parser.parse(compress_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify compression statistics are reasonable
    let row = &result.rows[0];

    // Extract statistics
    let original_size = if let Some(QueryValue::Integer(size)) = row.get("original_size_bytes") {
        *size
    } else {
        panic!("Expected original_size_bytes to be an integer");
    };

    let compressed_size = if let Some(QueryValue::Integer(size)) = row.get("compressed_size_bytes")
    {
        *size
    } else {
        panic!("Expected compressed_size_bytes to be an integer");
    };

    let compression_ratio =
        if let Some(QueryValue::Float(ratio)) = row.get("compression_ratio_percent") {
            *ratio
        } else {
            panic!("Expected compression_ratio_percent to be a float");
        };

    let space_saved = if let Some(QueryValue::Integer(saved)) = row.get("space_saved_bytes") {
        *saved
    } else {
        panic!("Expected space_saved_bytes to be an integer");
    };

    // Verify compression statistics make sense
    assert!(
        original_size > 0,
        "Original size should be positive: {original_size}"
    );
    assert!(
        compressed_size > 0,
        "Compressed size should be positive: {compressed_size}"
    );
    assert!(
        compressed_size < original_size,
        "Compressed size ({compressed_size}) should be less than original size ({original_size})"
    );
    assert!(
        compression_ratio > 0.0 && compression_ratio <= 100.0,
        "Compression ratio should be between 0 and 100: {compression_ratio}"
    );
    assert!(
        space_saved > 0,
        "Space saved should be positive: {space_saved}"
    );
    assert_eq!(
        space_saved,
        original_size - compressed_size,
        "Space saved should equal original_size - compressed_size"
    );
}

#[tokio::test]
async fn test_compress_table_nonexistent() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Try to compress a non-existent table
    let compress_sql = "COMPRESS TABLE nonexistent_table USING DNA";
    let statement = parser.parse(compress_sql).unwrap();
    let result = executor.execute_statement(&statement).await;

    // Should return an error
    assert!(
        result.is_err(),
        "Compressing non-existent table should return an error"
    );

    let error_message = format!("{:?}", result.unwrap_err());
    assert!(
        error_message.contains("nonexistent_table")
            || error_message.contains("does not exist")
            || error_message.contains("cannot be accessed"),
        "Error message should mention the table: {error_message}"
    );
}

#[tokio::test]
async fn test_compress_table_empty_table() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    // Initialize storage engine
    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create empty test table
    let schema = TableSchema {
        name: "empty_logs".to_string(),
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
        auto_increment_columns: std::collections::HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();
    }

    let config = ExecutorConfig {
        enable_neuromorphic_learning: true,
        enable_synaptic_optimization: true,
        enable_dna_compression: true,
        ..Default::default()
    };
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    let parser = Parser::new();

    // Execute COMPRESS TABLE on empty table
    let compress_sql = "COMPRESS TABLE empty_logs USING DNA";
    let statement = parser.parse(compress_sql).unwrap();
    let result = executor.execute_statement(&statement).await.unwrap();

    // Verify result
    assert_eq!(result.rows.len(), 1, "Should return one result row");

    let row = &result.rows[0];
    assert_eq!(
        row.get("rows_compressed"),
        Some(&QueryValue::Integer(0)),
        "Should report 0 rows compressed"
    );

    assert_eq!(result.rows_affected, 0, "Should affect 0 rows");
}

#[tokio::test]
async fn test_compress_table_case_insensitive() {
    let (_temp_dir, _storage_arc, mut executor) = setup_test_env().await;
    let parser = Parser::new();

    // Test with different case variations
    let test_cases = [
        "COMPRESS TABLE logs USING DNA",
        "compress table logs using dna",
        "CoMpReSs TaBlE logs UsInG dNa",
    ];

    for (i, sql) in test_cases.iter().enumerate() {
        let statement = parser.parse(sql).unwrap();
        let result = executor.execute_statement(&statement).await.unwrap();

        assert_eq!(
            result.rows.len(),
            1,
            "Test case {i}: Should return one result row"
        );

        let row = &result.rows[0];
        assert_eq!(
            row.get("algorithm"),
            Some(&QueryValue::String("DNA".to_string())),
            "Test case {i}: Algorithm should be DNA"
        );
    }
}
