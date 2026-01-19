//! Example: Multi-Row INSERT (Bulk Insert) in `NeuroQuantumDB`
//!
//! This example demonstrates how to use multi-row INSERT statements to efficiently
//! insert multiple rows in a single query, reducing network roundtrips and improving
//! performance through batch operations.
//!
//! # Features Demonstrated
//! - Single-row vs multi-row INSERT syntax
//! - Batch insertion with auto-increment IDs
//! - Multi-row INSERT within transactions
//! - Performance benefits of bulk operations
//! - Error handling for batch inserts

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt::init();

    println!("🚀 NeuroQuantumDB Multi-Row INSERT Example\n");

    // Setup: Create temporary storage directory
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path();

    // Initialize storage engine
    println!("📦 Initializing storage engine...");
    let storage = StorageEngine::new(storage_path).await?;
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create query executor
    println!("⚙️  Setting up query executor...\n");
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone())?;
    let parser = Parser::new();

    // Create a test table for users
    println!("📝 Creating 'users' table with auto-increment ID...");
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
            ColumnDefinition {
                name: "email".to_string(),
                data_type: DataType::Text,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "age".to_string(),
                data_type: DataType::Integer,
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
        foreign_keys: Vec::new(),
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(schema).await?;
    }
    println!("✅ Table created successfully\n");

    // ========================================
    // Example 1: Single-row INSERT (baseline)
    // ========================================
    println!("📊 Example 1: Single-row INSERT");
    println!("─────────────────────────────────");

    let start = Instant::now();

    // Insert one row at a time
    let sql1 = "INSERT INTO users (name, email, age) VALUES ('Alice', 'alice@example.com', 25)";
    let stmt1 = parser.parse(sql1)?;
    let result1 = executor.execute_statement(&stmt1).await?;
    println!("Inserted {} row", result1.rows_affected);

    let sql2 = "INSERT INTO users (name, email, age) VALUES ('Bob', 'bob@example.com', 30)";
    let stmt2 = parser.parse(sql2)?;
    let result2 = executor.execute_statement(&stmt2).await?;
    println!("Inserted {} row", result2.rows_affected);

    let sql3 = "INSERT INTO users (name, email, age) VALUES ('Charlie', 'charlie@example.com', 35)";
    let stmt3 = parser.parse(sql3)?;
    let result3 = executor.execute_statement(&stmt3).await?;
    println!("Inserted {} row", result3.rows_affected);

    let single_duration = start.elapsed();
    println!("⏱️  Time taken for 3 single-row INSERTs: {single_duration:?}\n");

    // Clear the table for next example
    let truncate_sql = "DELETE FROM users WHERE id > 0";
    let truncate_stmt = parser.parse(truncate_sql)?;
    executor.execute_statement(&truncate_stmt).await?;

    // ========================================
    // Example 2: Multi-row INSERT (efficient)
    // ========================================
    println!("📊 Example 2: Multi-row INSERT (Bulk Insert)");
    println!("──────────────────────────────────────────────");

    let start = Instant::now();

    // Insert multiple rows in a single statement
    let multi_sql = r"
        INSERT INTO users (name, email, age) VALUES 
            ('Alice', 'alice@example.com', 25),
            ('Bob', 'bob@example.com', 30),
            ('Charlie', 'charlie@example.com', 35)
    ";

    let multi_stmt = parser.parse(multi_sql)?;
    let multi_result = executor.execute_statement(&multi_stmt).await?;

    let multi_duration = start.elapsed();

    println!(
        "✅ Inserted {} rows in a single statement",
        multi_result.rows_affected
    );
    println!("⏱️  Time taken: {multi_duration:?}");

    if multi_duration < single_duration {
        let speedup = single_duration.as_micros() as f64 / multi_duration.as_micros() as f64;
        println!("🚀 Multi-row INSERT was {speedup:.2}x faster!");
    }
    println!();

    // ========================================
    // Example 3: Large batch INSERT
    // ========================================
    println!("📊 Example 3: Large Batch INSERT (50 rows)");
    println!("───────────────────────────────────────────");

    // Build a large multi-row INSERT
    let mut values_parts = Vec::new();
    for i in 1..=50 {
        values_parts.push(format!(
            "('User{}', 'user{}@example.com', {})",
            i,
            i,
            20 + (i % 50)
        ));
    }
    let large_sql = format!(
        "INSERT INTO users (name, email, age) VALUES {}",
        values_parts.join(", ")
    );

    let start = Instant::now();
    let large_stmt = parser.parse(&large_sql)?;
    let large_result = executor.execute_statement(&large_stmt).await?;
    let large_duration = start.elapsed();

    println!("✅ Inserted {} rows", large_result.rows_affected);
    println!("⏱️  Time taken: {large_duration:?}");
    println!("📈 Average per row: {:?}\n", large_duration / 50);

    // ========================================
    // Example 4: Query the results
    // ========================================
    println!("📊 Example 4: Query inserted data");
    println!("──────────────────────────────────");

    let select_sql = "SELECT * FROM users WHERE age >= 30 LIMIT 5";
    let select_stmt = parser.parse(select_sql)?;
    let select_result = executor.execute_statement(&select_stmt).await?;

    println!("Found {} users with age >= 30:", select_result.rows.len());
    for row in select_result.rows.iter().take(5) {
        let name = row
            .get("name")
            .map(|v| format!("{v:?}"))
            .unwrap_or_default();
        let email = row
            .get("email")
            .map(|v| format!("{v:?}"))
            .unwrap_or_default();
        let age = row.get("age").map(|v| format!("{v:?}")).unwrap_or_default();
        println!("  - {name} ({email}) - Age: {age}");
    }
    println!();

    // ========================================
    // Example 5: Multi-row INSERT in transaction
    // ========================================
    println!("📊 Example 5: Multi-row INSERT with Transaction");
    println!("─────────────────────────────────────────────────");

    // Begin transaction
    let begin_sql = "BEGIN";
    let begin_stmt = parser.parse(begin_sql)?;
    executor.execute_statement(&begin_stmt).await?;
    println!("🔒 Transaction started");

    // Multi-row INSERT within transaction
    let tx_sql = r"
        INSERT INTO users (name, email, age) VALUES 
            ('David', 'david@example.com', 28),
            ('Eve', 'eve@example.com', 32),
            ('Frank', 'frank@example.com', 45)
    ";

    let tx_stmt = parser.parse(tx_sql)?;
    let tx_result = executor.execute_statement(&tx_stmt).await?;
    println!(
        "📝 Inserted {} rows (not yet committed)",
        tx_result.rows_affected
    );

    // Commit transaction
    let commit_sql = "COMMIT";
    let commit_stmt = parser.parse(commit_sql)?;
    executor.execute_statement(&commit_stmt).await?;
    println!("✅ Transaction committed - all rows persisted atomically\n");

    // ========================================
    // Summary
    // ========================================
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║           Multi-Row INSERT Benefits                 ║");
    println!("╠══════════════════════════════════════════════════════╣");
    println!("║ ✓ Reduced network roundtrips                        ║");
    println!("║ ✓ Batch WAL writes for better durability            ║");
    println!("║ ✓ Optimized B+ tree operations                      ║");
    println!("║ ✓ Atomic insertion (all-or-nothing)                 ║");
    println!("║ ✓ DNA compression across all inserted rows          ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    println!("🎉 Multi-row INSERT example completed successfully!");

    Ok(())
}
