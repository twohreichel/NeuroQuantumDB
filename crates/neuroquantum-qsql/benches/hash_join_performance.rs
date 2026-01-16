//! Performance benchmarks for Hash Join optimization
//!
//! This benchmark suite demonstrates the O(n+m) performance of hash join
//! compared to O(n*m) nested loop join for large datasets.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use tempfile::TempDir;

/// Setup test tables with specified number of rows
async fn setup_test_data(
    num_customers: usize,
    num_orders: usize,
) -> (TempDir, Arc<tokio::sync::RwLock<StorageEngine>>) {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path();

    let storage = StorageEngine::new(storage_path).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    // Create customers table
    let customers_schema = TableSchema {
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
        ],
        primary_key: "id".to_string(),
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };

    // Create orders table
    let orders_schema = TableSchema {
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
    };

    {
        let mut storage_guard = storage_arc.write().await;
        storage_guard.create_table(customers_schema).await.unwrap();
        storage_guard.create_table(orders_schema).await.unwrap();
    }

    let parser = Parser::new();

    // Insert customers
    for i in 0..num_customers {
        let sql = format!(
            "INSERT INTO customers (id, name) VALUES ({}, 'Customer {}')",
            i, i
        );
        let mut executor =
            QueryExecutor::with_storage(ExecutorConfig::default(), storage_arc.clone()).unwrap();
        let statement = parser.parse(&sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }

    // Insert orders - distribute across customers
    for i in 0..num_orders {
        let customer_id = i % num_customers.max(1);
        let sql = format!(
            "INSERT INTO orders (id, customer_id, amount) VALUES ({}, {}, {}.0)",
            i,
            customer_id,
            (i as f64) * 10.5
        );
        let mut executor =
            QueryExecutor::with_storage(ExecutorConfig::default(), storage_arc.clone()).unwrap();
        let statement = parser.parse(&sql).unwrap();
        executor.execute_statement(&statement).await.unwrap();
    }

    (temp_dir, storage_arc)
}

/// Benchmark hash join vs nested loop join for different dataset sizes
fn bench_join_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("join_algorithms");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10);

    // Test with different dataset sizes
    let test_sizes = vec![
        (100, 100),   // Small: 10,000 comparisons
        (500, 500),   // Medium: 250,000 comparisons
        (1000, 1000), // Large: 1,000,000 comparisons
        (2000, 500),  // Asymmetric: 1,000,000 comparisons
    ];

    for (num_customers, num_orders) in test_sizes {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // Setup test data once
        let (_temp_dir, storage_arc) = runtime.block_on(setup_test_data(num_customers, num_orders));

        // Benchmark nested loop join (high threshold to force nested loop)
        group.bench_with_input(
            BenchmarkId::new("nested_loop", format!("{}x{}", num_customers, num_orders)),
            &(num_customers, num_orders),
            |b, _| {
                b.to_async(&runtime).iter(|| {
                    let storage = storage_arc.clone();
                    async move {
                        let config = ExecutorConfig {
                            hash_join_threshold: usize::MAX, // Force nested loop
                            ..Default::default()
                        };
                        let mut executor = QueryExecutor::with_storage(config, storage).unwrap();
                        let parser = Parser::new();

                        let sql = "SELECT c.name, SUM(o.amount) as total \
                                   FROM orders o \
                                   INNER JOIN customers c ON o.customer_id = c.id \
                                   GROUP BY c.name";
                        let statement = parser.parse(sql).unwrap();
                        black_box(executor.execute_statement(&statement).await.unwrap())
                    }
                });
            },
        );

        // Benchmark hash join (low threshold to force hash join)
        group.bench_with_input(
            BenchmarkId::new("hash_join", format!("{}x{}", num_customers, num_orders)),
            &(num_customers, num_orders),
            |b, _| {
                b.to_async(&runtime).iter(|| {
                    let storage = storage_arc.clone();
                    async move {
                        let config = ExecutorConfig {
                            hash_join_threshold: 0, // Force hash join
                            ..Default::default()
                        };
                        let mut executor = QueryExecutor::with_storage(config, storage).unwrap();
                        let parser = Parser::new();

                        let sql = "SELECT c.name, SUM(o.amount) as total \
                                   FROM orders o \
                                   INNER JOIN customers c ON o.customer_id = c.id \
                                   GROUP BY c.name";
                        let statement = parser.parse(sql).unwrap();
                        black_box(executor.execute_statement(&statement).await.unwrap())
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark the impact of dataset size on hash join performance
fn bench_hash_join_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_join_scalability");
    group.measurement_time(Duration::from_secs(20));

    let test_sizes = vec![
        (100, 1000),
        (1000, 10000),
        // Add more sizes as needed
    ];

    for (num_customers, num_orders) in test_sizes {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (_temp_dir, storage_arc) = runtime.block_on(setup_test_data(num_customers, num_orders));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", num_customers, num_orders)),
            &(num_customers, num_orders),
            |b, _| {
                b.to_async(&runtime).iter(|| {
                    let storage = storage_arc.clone();
                    async move {
                        let config = ExecutorConfig {
                            hash_join_threshold: 0, // Always use hash join
                            ..Default::default()
                        };
                        let mut executor = QueryExecutor::with_storage(config, storage).unwrap();
                        let parser = Parser::new();

                        let sql =
                            "SELECT c.name, COUNT(o.id) as order_count, SUM(o.amount) as total \
                                   FROM customers c \
                                   LEFT JOIN orders o ON c.id = o.customer_id \
                                   GROUP BY c.name";
                        let statement = parser.parse(sql).unwrap();
                        black_box(executor.execute_statement(&statement).await.unwrap())
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_join_algorithms, bench_hash_join_scalability);
criterion_main!(benches);
