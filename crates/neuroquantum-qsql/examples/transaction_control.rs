//! Example: Using Transaction Control in NeuroQuantumDB
//!
//! This example demonstrates how to use BEGIN, COMMIT, and ROLLBACK
//! statements with the NeuroQuantumDB storage engine and transaction manager.
//!
//! # Features Demonstrated
//! - Starting transactions with BEGIN or START TRANSACTION
//! - Committing transactions
//! - Rolling back transactions
//! - Using different isolation levels
//! - Working with savepoints

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_core::transaction::TransactionManager;
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt::init();

    println!("🚀 NeuroQuantumDB Transaction Control Example\n");

    // Setup: Create temporary directories for storage and WAL
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    let wal_path = temp_dir.path().join("wal");

    tokio::fs::create_dir_all(&storage_path).await?;
    tokio::fs::create_dir_all(&wal_path).await?;

    // Initialize storage engine
    println!("📦 Initializing storage engine...");
    let storage = StorageEngine::new(&storage_path).await?;
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Initialize transaction manager
    println!("🔐 Initializing transaction manager with WAL...");
    let tx_manager = TransactionManager::new_async(&wal_path).await?;
    let tx_manager_arc = Arc::new(tx_manager);

    // Create query executor with storage and transaction support
    println!("⚙️  Setting up query executor...\n");
    let config = ExecutorConfig::default();
    let mut executor = QueryExecutor::with_storage(config, storage_arc.clone())?;
    executor.set_transaction_manager(tx_manager_arc.clone());

    // Create a test table
    println!("📋 Creating accounts table...");
    let schema = TableSchema {
        name: "accounts".to_string(),
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
                name: "balance".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: Some(Value::Integer(0)),
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
        storage_guard.create_table(schema).await?;
    }

    let parser = Parser::new();

    // Example 1: Simple Transaction with COMMIT
    println!("\n📝 Example 1: Simple Transaction with COMMIT");
    println!("─────────────────────────────────────────────");

    let begin_stmt = parser.parse("BEGIN")?;
    executor.execute_statement(&begin_stmt).await?;
    println!("✅ BEGIN transaction");

    let insert_stmt =
        parser.parse("INSERT INTO accounts (id, name, balance) VALUES (1, 'Alice', 1000)")?;
    executor.execute_statement(&insert_stmt).await?;
    println!("✅ Inserted Alice with balance 1000");

    let commit_stmt = parser.parse("COMMIT")?;
    executor.execute_statement(&commit_stmt).await?;
    println!("✅ COMMIT transaction");

    // Verify data was persisted
    let select_stmt = parser.parse("SELECT * FROM accounts WHERE id = 1")?;
    let result = executor.execute_statement(&select_stmt).await?;
    println!("📊 Query result: {} rows", result.rows.len());

    // Example 2: Transaction with ROLLBACK
    println!("\n📝 Example 2: Transaction with ROLLBACK");
    println!("──────────────────────────────────────────");

    let begin_stmt = parser.parse("BEGIN")?;
    executor.execute_statement(&begin_stmt).await?;
    println!("✅ BEGIN transaction");

    let insert_stmt =
        parser.parse("INSERT INTO accounts (id, name, balance) VALUES (2, 'Bob', 500)")?;
    executor.execute_statement(&insert_stmt).await?;
    println!("✅ Inserted Bob with balance 500");

    let rollback_stmt = parser.parse("ROLLBACK")?;
    executor.execute_statement(&rollback_stmt).await?;
    println!("✅ ROLLBACK transaction");

    // Verify data was NOT persisted
    let select_stmt = parser.parse("SELECT * FROM accounts WHERE id = 2")?;
    let result = executor.execute_statement(&select_stmt).await?;
    println!("📊 Query result: {} rows (should be 0)", result.rows.len());

    // Example 3: Transaction with Isolation Level
    println!("\n📝 Example 3: Transaction with Isolation Level");
    println!("─────────────────────────────────────────────────");

    let begin_stmt = parser.parse("BEGIN ISOLATION LEVEL SERIALIZABLE")?;
    executor.execute_statement(&begin_stmt).await?;
    println!("✅ BEGIN transaction with SERIALIZABLE isolation");

    let insert_stmt =
        parser.parse("INSERT INTO accounts (id, name, balance) VALUES (3, 'Charlie', 750)")?;
    executor.execute_statement(&insert_stmt).await?;
    println!("✅ Inserted Charlie with balance 750");

    let commit_stmt = parser.parse("COMMIT")?;
    executor.execute_statement(&commit_stmt).await?;
    println!("✅ COMMIT transaction");

    // Example 4: Bank Transfer (Atomic Operations)
    println!("\n📝 Example 4: Bank Transfer (Atomic Operations)");
    println!("──────────────────────────────────────────────────");

    // First, add Bob's account
    let begin_stmt = parser.parse("BEGIN")?;
    executor.execute_statement(&begin_stmt).await?;
    let insert_stmt =
        parser.parse("INSERT INTO accounts (id, name, balance) VALUES (2, 'Bob', 500)")?;
    executor.execute_statement(&insert_stmt).await?;
    let commit_stmt = parser.parse("COMMIT")?;
    executor.execute_statement(&commit_stmt).await?;

    // Now perform the transfer
    let begin_stmt = parser.parse("BEGIN")?;
    executor.execute_statement(&begin_stmt).await?;
    println!("✅ BEGIN transaction");

    let debit_stmt = parser.parse("UPDATE accounts SET balance = balance - 100 WHERE id = 1")?;
    executor.execute_statement(&debit_stmt).await?;
    println!("✅ Debited 100 from Alice");

    let credit_stmt = parser.parse("UPDATE accounts SET balance = balance + 100 WHERE id = 2")?;
    executor.execute_statement(&credit_stmt).await?;
    println!("✅ Credited 100 to Bob");

    let commit_stmt = parser.parse("COMMIT")?;
    executor.execute_statement(&commit_stmt).await?;
    println!("✅ COMMIT transaction - Transfer complete!");

    // Example 5: Savepoints
    println!("\n📝 Example 5: Using Savepoints");
    println!("──────────────────────────────");

    let begin_stmt = parser.parse("BEGIN")?;
    executor.execute_statement(&begin_stmt).await?;
    println!("✅ BEGIN transaction");

    let insert_stmt =
        parser.parse("INSERT INTO accounts (id, name, balance) VALUES (4, 'David', 300)")?;
    executor.execute_statement(&insert_stmt).await?;
    println!("✅ Inserted David with balance 300");

    let savepoint_stmt = parser.parse("SAVEPOINT sp1")?;
    executor.execute_statement(&savepoint_stmt).await?;
    println!("✅ Created savepoint 'sp1'");

    let insert_stmt =
        parser.parse("INSERT INTO accounts (id, name, balance) VALUES (5, 'Eve', 400)")?;
    executor.execute_statement(&insert_stmt).await?;
    println!("✅ Inserted Eve with balance 400");

    // Rollback to savepoint (keeps David, removes Eve)
    let rollback_to_stmt = parser.parse("ROLLBACK TO SAVEPOINT sp1")?;
    executor.execute_statement(&rollback_to_stmt).await?;
    println!("✅ Rolled back to savepoint 'sp1' (Eve insert undone)");

    let commit_stmt = parser.parse("COMMIT")?;
    executor.execute_statement(&commit_stmt).await?;
    println!("✅ COMMIT transaction");

    // Final summary
    println!("\n📊 Final Summary");
    println!("────────────────");
    let select_all = parser.parse("SELECT * FROM accounts")?;
    let result = executor.execute_statement(&select_all).await?;
    println!("Total accounts: {}", result.rows.len());
    for (i, row) in result.rows.iter().enumerate() {
        println!("  {}. {}", i + 1, format_account_row(row));
    }

    println!("\n✅ Transaction control example completed successfully!");

    Ok(())
}

fn format_account_row(
    row: &std::collections::HashMap<String, neuroquantum_qsql::query_plan::QueryValue>,
) -> String {
    let id = match row.get("id") {
        Some(neuroquantum_qsql::query_plan::QueryValue::Integer(i)) => i.to_string(),
        _ => "?".to_string(),
    };
    let name = match row.get("name") {
        Some(neuroquantum_qsql::query_plan::QueryValue::String(s)) => s.clone(),
        _ => "Unknown".to_string(),
    };
    let balance = match row.get("balance") {
        Some(neuroquantum_qsql::query_plan::QueryValue::Integer(b)) => b.to_string(),
        _ => "?".to_string(),
    };
    format!("ID: {}, Name: {}, Balance: {}", id, name, balance)
}
