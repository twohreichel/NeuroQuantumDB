// Test to verify HAVING clause execution
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

    // Create test table
    let schema = TableSchema {
        name: "users".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: true,
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

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await.unwrap();

        // Insert test data - some names repeated
        for (i, name) in [
            "Alice", "Bob", "Alice", "Charlie", "Bob", "Bob", "David",
        ]
        .iter()
        .enumerate()
        {
            let mut row = neuroquantum_core::storage::Row {
                id: 0,
                fields: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            row.fields.insert("id".to_string(), Value::Integer((i + 1) as i64));
            row.fields.insert("name".to_string(), Value::Text(name.to_string()));
            storage_guard.insert_row("users", row).await.unwrap();
        }
    }

    // Create query executor with storage integration
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();
    let parser = Parser::new();

    // Test: GROUP BY name HAVING COUNT(*) > 1
    // Expected: Alice (2), Bob (3) - should exclude Charlie and David
    let sql = "SELECT name, COUNT(*) FROM users GROUP BY name HAVING COUNT(*) > 1";
    println!("Query: {}", sql);
    
    match parser.parse(sql) {
        Ok(statement) => {
            println!("Parsed: {:?}", statement);
            match executor.execute_statement(&statement).await {
                Ok(result) => {
                    println!("Success! Rows returned: {}", result.rows.len());
                    for row in &result.rows {
                        println!("  Row: {:?}", row);
                    }
                    // We expect 2 rows (Alice and Bob)
                    assert_eq!(result.rows.len(), 2, "Expected 2 rows (Alice with 2, Bob with 3)");
                }
                Err(e) => {
                    println!("ERROR during execution: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("ERROR during parsing: {:?}", e);
        }
    }

    println!("\n✅ HAVING clause test completed");
}
