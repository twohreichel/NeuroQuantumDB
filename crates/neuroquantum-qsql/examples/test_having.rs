//! Integration test for HAVING clause execution
//!
//! This test verifies that HAVING clause works correctly with:
//! - Comparison operators: `=`, `<`, `>`, `<=`, `>=`, `<>`
//! - Aggregate functions: `COUNT()`, `SUM()`, `AVG()`
//! - Logical operators: `AND`, `OR`

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create test table with numeric data
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

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();

        // Insert test data
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
                .insert("category".to_string(), Value::Text(category.to_string()));
            row.fields
                .insert("amount".to_string(), Value::Float(*amount));
            storage_guard.insert_row("orders", row).await.unwrap();
        }
    }

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();

    println!("=== HAVING Clause Integration Tests ===\n");

    // Test 1: COUNT(*) > N
    run_test(
        &parser,
        &mut executor,
        "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) > 1",
        "COUNT(*) > 1",
        3, // Electronics (3), Books (2), Clothing (4)
    )
    .await;

    // Test 2: COUNT(*) >= N
    run_test(
        &parser,
        &mut executor,
        "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) >= 3",
        "COUNT(*) >= 3",
        2, // Electronics (3), Clothing (4)
    )
    .await;

    // Test 3: COUNT(*) = N
    run_test(
        &parser,
        &mut executor,
        "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) = 2",
        "COUNT(*) = 2",
        1, // Books (2)
    )
    .await;

    // Test 4: COUNT(*) < N
    run_test(
        &parser,
        &mut executor,
        "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) < 3",
        "COUNT(*) < 3",
        2, // Books (2), Food (1)
    )
    .await;

    // Test 5: SUM with HAVING
    run_test(
        &parser,
        &mut executor,
        "SELECT category, SUM(amount) FROM orders GROUP BY category HAVING SUM(amount) > 200",
        "SUM(amount) > 200",
        2, // Electronics (600), Clothing (250)
    )
    .await;

    // Test 6: AVG with HAVING
    run_test(
        &parser,
        &mut executor,
        "SELECT category, AVG(amount) FROM orders GROUP BY category HAVING AVG(amount) > 50",
        "AVG(amount) > 50",
        3, // Electronics (200), Books (75), Clothing (62.5)
    )
    .await;

    // Test 7: Logical AND in HAVING
    run_test(&parser, &mut executor,
        "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) > 1 AND COUNT(*) < 4",
        "COUNT(*) > 1 AND COUNT(*) < 4",
        2,  // Electronics (3), Books (2)
    ).await;

    // Test 8: Logical OR in HAVING
    run_test(&parser, &mut executor,
        "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) = 1 OR COUNT(*) = 4",
        "COUNT(*) = 1 OR COUNT(*) = 4",
        2,  // Food (1), Clothing (4)
    ).await;

    // Test 9: NOT equal
    run_test(
        &parser,
        &mut executor,
        "SELECT category, COUNT(*) FROM orders GROUP BY category HAVING COUNT(*) <> 1",
        "COUNT(*) <> 1",
        3, // Electronics (3), Books (2), Clothing (4)
    )
    .await;

    println!("\n✅ All HAVING clause tests passed!");
}

async fn run_test(
    parser: &Parser,
    executor: &mut QueryExecutor,
    sql: &str,
    description: &str,
    expected_rows: usize,
) {
    print!("Test: HAVING {} ... ", description);

    match parser.parse(sql) {
        Ok(statement) => match executor.execute_statement(&statement).await {
            Ok(result) => {
                if result.rows.len() == expected_rows {
                    println!("✓ (got {} rows)", result.rows.len());
                } else {
                    println!(
                        "✗ FAILED (expected {} rows, got {})",
                        expected_rows,
                        result.rows.len()
                    );
                    for row in &result.rows {
                        println!("    Row: {:?}", row);
                    }
                    std::process::exit(1);
                }
            }
            Err(e) => {
                println!("✗ EXECUTION ERROR: {:?}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            println!("✗ PARSE ERROR: {:?}", e);
            std::process::exit(1);
        }
    }
}
