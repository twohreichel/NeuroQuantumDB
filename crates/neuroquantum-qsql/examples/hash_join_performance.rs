//! Hash Join Performance Example
//!
//! This example demonstrates the hash join optimization for large datasets.
//! It creates two tables, populates them with data, and shows how JOIN
//! operations benefit from automatic hash join selection.
//!
//! Run with: cargo run --example hash_join_performance

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to see hash join selection
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("=== Hash Join Performance Demonstration ===\n");

    // Create temporary storage
    let temp_dir = TempDir::new()?;
    let storage = StorageEngine::new(temp_dir.path()).await?;
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create customers table
    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard
            .create_table(TableSchema {
                name: "customers".to_string(),
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
                        name: "region".to_string(),
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
            })
            .await?;

        // Create orders table
        storage_guard
            .create_table(TableSchema {
                name: "orders".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "customer_id".to_string(),
                        data_type: DataType::Integer,
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
            })
            .await?;
    }

    let parser = Parser::new();
    let regions = vec!["North", "South", "East", "West"];

    // Scenario 1: Small dataset - will use nested loop join
    println!("--- Scenario 1: Small Dataset (10 customers, 20 orders) ---");
    println!("Expected: Nested loop join (product = 200 < threshold)\n");

    {
        let config = ExecutorConfig::default(); // threshold = 1000
        let mut executor = QueryExecutor::with_storage(config, storage_arc.clone())?;

        // Insert 10 customers
        for i in 1..=10 {
            let sql = format!(
                "INSERT INTO customers (id, name, region) VALUES ({}, 'Customer {}', '{}')",
                i,
                i,
                regions[i % regions.len()]
            );
            let statement = parser.parse(&sql)?;
            executor.execute_statement(&statement).await?;
        }

        // Insert 20 orders
        for i in 1..=20 {
            let customer_id = (i % 10) + 1;
            let sql = format!(
                "INSERT INTO orders (id, customer_id, amount) VALUES ({}, {}, {}.0)",
                i,
                customer_id,
                i as f64 * 10.5
            );
            let statement = parser.parse(&sql)?;
            executor.execute_statement(&statement).await?;
        }

        // Execute JOIN
        let start = Instant::now();
        let sql = "SELECT c.name, c.region, COUNT(o.id) as order_count, SUM(o.amount) as total \
                   FROM customers c \
                   LEFT JOIN orders o ON c.id = o.customer_id \
                   GROUP BY c.name, c.region";
        let statement = parser.parse(sql)?;
        let result = executor.execute_statement(&statement).await?;
        let duration = start.elapsed();

        println!("✓ Query executed in {:?}", duration);
        println!("✓ Result: {} rows\n", result.rows.len());
    }

    // Clear tables for next scenario
    {
        let mut executor =
            QueryExecutor::with_storage(ExecutorConfig::default(), storage_arc.clone())?;
        executor
            .execute_statement(&parser.parse("DELETE FROM orders")?)
            .await?;
        executor
            .execute_statement(&parser.parse("DELETE FROM customers")?)
            .await?;
    }

    // Scenario 2: Large dataset - will use hash join
    println!("--- Scenario 2: Large Dataset (100 customers, 5000 orders) ---");
    println!("Expected: Hash join (product = 500,000 > threshold)\n");

    let hash_join_duration = {
        let config = ExecutorConfig::default(); // threshold = 1000
        let mut executor = QueryExecutor::with_storage(config, storage_arc.clone())?;
        let parser = Parser::new();

        // Insert 100 customers
        println!("Inserting 100 customers...");
        for i in 1..=100 {
            let sql = format!(
                "INSERT INTO customers (id, name, region) VALUES ({}, 'Customer {}', '{}')",
                i,
                i,
                regions[i % regions.len()]
            );
            let statement = parser.parse(&sql)?;
            executor.execute_statement(&statement).await?;
        }

        // Insert 5000 orders
        println!("Inserting 5000 orders...");
        for i in 1..=5000 {
            let customer_id = (i % 100) + 1;
            let sql = format!(
                "INSERT INTO orders (id, customer_id, amount) VALUES ({}, {}, {}.0)",
                i,
                customer_id,
                i as f64 * 10.5
            );
            let statement = parser.parse(&sql)?;
            executor.execute_statement(&statement).await?;
        }

        println!("Data inserted successfully.\n");

        // Execute JOIN with hash join
        println!("Executing JOIN query (with hash join)...");
        let start = Instant::now();
        let sql = "SELECT c.name, c.region, COUNT(o.id) as order_count, SUM(o.amount) as total \
                   FROM customers c \
                   LEFT JOIN orders o ON c.id = o.customer_id \
                   GROUP BY c.name, c.region \
                   ORDER BY total DESC \
                   LIMIT 10";
        let statement = parser.parse(sql)?;
        let result = executor.execute_statement(&statement).await?;
        let hash_join_duration = start.elapsed();

        println!("✓ Hash join query executed in {:?}", hash_join_duration);
        println!("✓ Top 10 customers by total spend:\n");

        for (idx, row) in result.rows.iter().enumerate() {
            println!(
                "  {}. {} ({}) - {} orders, ${:.2} total",
                idx + 1,
                row.get("name")
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default(),
                row.get("region")
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default(),
                row.get("order_count")
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default(),
                row.get("total")
                    .and_then(|v| {
                        if let neuroquantum_qsql::query_plan::QueryValue::Float(f) = v {
                            Some(*f)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0)
            );
        }

        println!();
        
        hash_join_duration
    };

    // Scenario 3: Force nested loop join for comparison
    println!("--- Scenario 3: Same Dataset with Forced Nested Loop Join ---");
    println!("Expected: Nested loop join (threshold set to MAX)\n");

    {
        let config = ExecutorConfig {
            hash_join_threshold: usize::MAX, // Force nested loop
            ..Default::default()
        };
        let mut executor = QueryExecutor::with_storage(config, storage_arc.clone())?;

        println!("Executing same JOIN query (with nested loop join)...");
        let start = Instant::now();
        let sql = "SELECT c.name, c.region, COUNT(o.id) as order_count, SUM(o.amount) as total \
                   FROM customers c \
                   LEFT JOIN orders o ON c.id = o.customer_id \
                   GROUP BY c.name, c.region \
                   ORDER BY total DESC \
                   LIMIT 10";
        let statement = parser.parse(sql)?;
        let result = executor.execute_statement(&statement).await?;
        let nested_loop_duration = start.elapsed();

        println!(
            "✓ Nested loop join query executed in {:?}",
            nested_loop_duration
        );
        println!("✓ Result: {} rows\n", result.rows.len());

        // Calculate speedup
        let speedup = nested_loop_duration.as_secs_f64() / hash_join_duration.as_secs_f64();
        println!("=== Performance Summary ===");
        println!("Hash Join:        {:?}", hash_join_duration);
        println!("Nested Loop Join: {:?}", nested_loop_duration);
        println!("Speedup:          {:.2}x faster with hash join!", speedup);
    }

    println!("\n✓ Hash join demonstration completed successfully!");

    Ok(())
}
