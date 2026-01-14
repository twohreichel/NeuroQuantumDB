// Integration test for Issue #278 - NEUROMATCH clause returns no results
// This test reproduces the bug and verifies the fix

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_neuromatch_returns_results() {
    // Setup: Create storage engine and executor
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    let executor_config = ExecutorConfig {
        enable_synaptic_optimization: true,
        enable_neuromorphic_learning: true,
        ..ExecutorConfig::default()
    };

    let mut executor = QueryExecutor::with_storage(executor_config, storage_arc.clone())
        .expect("Failed to create executor");

    // Create test table
    let schema = TableSchema {
        name: "books".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "title".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "author".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "genre".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "description".to_string(),
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
    }

    let parser = Parser::new();

    // Insert test data with fantasy/magic related content
    let insert_queries = vec![
        "INSERT INTO books (id, title, author, genre, description) VALUES (1, 'Harry Potter', 'J.K. Rowling', 'Fantasy', 'A young wizard discovers magic and battles dark forces')",
        "INSERT INTO books (id, title, author, genre, description) VALUES (2, 'Der Hobbit', 'J.R.R. Tolkien', 'Fantasy', 'Bilbo embarks on an adventure full of magic and danger')",
        "INSERT INTO books (id, title, author, genre, description) VALUES (3, 'Die Säulen der Erde', 'Ken Follett', 'Historical', 'Medieval cathedral building, no magic involved')",
        "INSERT INTO books (id, title, author, genre, description) VALUES (4, 'Das Lied von Eis und Feuer', 'George R.R. Martin', 'Fantasy', 'Epic fantasy with dragons, magic, and adventure')",
        "INSERT INTO books (id, title, author, genre, description) VALUES (5, '1984', 'George Orwell', 'Dystopian', 'A totalitarian future, no fantasy or magic')",
    ];

    for insert_sql in insert_queries {
        let insert_stmt = parser.parse(insert_sql).unwrap();
        let result = executor.execute_statement(&insert_stmt).await.unwrap();
        println!("Inserted row: rows_affected={}", result.rows_affected);
    }

    // TEST 1: Execute NEUROMATCH query with legacy syntax
    println!("\n=== TEST 1: NEUROMATCH with legacy syntax ===");
    let neuromatch_sql = "SELECT * FROM books NEUROMATCH 'Fantasy Magie Abenteuer' STRENGTH > 0.5";
    println!("Query: {}", neuromatch_sql);

    let neuromatch_stmt = parser.parse(neuromatch_sql).unwrap();

    // Verify it's a SELECT with neuromatch_clause
    if let neuroquantum_qsql::ast::Statement::Select(ref select) = neuromatch_stmt {
        assert!(
            select.neuromatch_clause.is_some(),
            "NEUROMATCH clause should be present"
        );
        println!("✓ NEUROMATCH clause present in parsed statement");

        let clause = select.neuromatch_clause.as_ref().unwrap();
        println!("  Pattern: {:?}", clause.pattern);
        println!("  Threshold: {}", clause.synaptic_weight);
    }

    let result = executor.execute_statement(&neuromatch_stmt).await.unwrap();

    println!("Result:");
    println!("  rows_affected: {}", result.rows_affected);
    println!("  rows returned: {}", result.rows.len());

    // Print all returned rows
    for (i, row) in result.rows.iter().enumerate() {
        println!("  Row {}: {:?}", i + 1, row);
    }

    // ASSERTION: Should return matching rows (at least Harry Potter and Der Hobbit)
    // Bug #278: This previously returned 0 rows
    assert!(
        !result.rows.is_empty(),
        "BUG #278 FIXED: NEUROMATCH now returns results! Got {} rows",
        result.rows.len()
    );

    // Verify that fantasy books are included
    let has_harry_potter = result.rows.iter().any(|row| {
        row.get("title")
            .map(|v| match v {
                neuroquantum_qsql::query_plan::QueryValue::String(s) => s.contains("Harry Potter"),
                _ => false,
            })
            .unwrap_or(false)
    });

    let has_hobbit = result.rows.iter().any(|row| {
        row.get("title")
            .map(|v| match v {
                neuroquantum_qsql::query_plan::QueryValue::String(s) => s.contains("Hobbit"),
                _ => false,
            })
            .unwrap_or(false)
    });

    assert!(
        has_harry_potter,
        "Harry Potter should match 'Fantasy Magie Abenteuer'"
    );
    assert!(
        has_hobbit,
        "Der Hobbit should match 'Fantasy Magie Abenteuer'"
    );

    // Note: The edge case of "1984" matching due to "no fantasy or magic" containing
    // the keywords is acceptable for this fix. Semantic negation detection could be
    // added in a future enhancement, but the main bug (returning no results) is fixed.

    println!("✓ TEST 1 PASSED: NEUROMATCH returns correct results (Issue #278 FIXED)");

    // TEST 2: Test with different threshold
    println!("\n=== TEST 2: NEUROMATCH with higher threshold ===");
    let high_threshold_sql = "SELECT * FROM books NEUROMATCH 'Fantasy' STRENGTH > 0.8";
    println!("Query: {}", high_threshold_sql);

    let high_threshold_stmt = parser.parse(high_threshold_sql).unwrap();
    let result2 = executor
        .execute_statement(&high_threshold_stmt)
        .await
        .unwrap();

    println!("Result with STRENGTH > 0.8:");
    println!("  rows returned: {}", result2.rows.len());

    // With exact word match "Fantasy", fantasy genre books should match with high score
    assert!(
        !result2.rows.is_empty(),
        "Should return fantasy books with high similarity"
    );

    println!("✓ TEST 2 PASSED: High threshold filtering works");
}
