//! Foreign Key Constraint Integration Tests
//!
//! Tests for FOREIGN KEY constraints including:
//! - Parsing REFERENCES syntax with ON DELETE/ON UPDATE actions
//! - INSERT validation (referenced values must exist)
//! - DELETE with CASCADE, RESTRICT, SET NULL
//! - UPDATE with CASCADE, RESTRICT, SET NULL

use neuroquantum_qsql::ast::{ColumnConstraint, ReferentialAction, TableConstraint};
use neuroquantum_qsql::Parser;

// =============================================================================
// Parser Tests
// =============================================================================

#[test]
fn test_parse_column_level_references() {
    let parser = Parser::new();
    let sql = "CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER REFERENCES customers(id)
    )";

    let result = parser.parse(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    if let neuroquantum_qsql::ast::Statement::CreateTable(create) = result.unwrap() {
        assert_eq!(create.table_name, "orders");

        // Find the customer_id column
        let customer_id_col = create
            .columns
            .iter()
            .find(|c| c.name == "customer_id")
            .expect("customer_id column not found");

        // Check for ForeignKey constraint
        let has_fk = customer_id_col.constraints.iter().any(|c| {
            matches!(c, ColumnConstraint::ForeignKey { table, column, .. }
                if table == "customers" && column == "id")
        });
        assert!(has_fk, "ForeignKey constraint not found");
    } else {
        panic!("Expected CreateTable statement");
    }
}

#[test]
fn test_parse_column_level_references_with_on_delete_cascade() {
    let parser = Parser::new();
    let sql = "CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER REFERENCES customers(id) ON DELETE CASCADE
    )";

    let result = parser.parse(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    if let neuroquantum_qsql::ast::Statement::CreateTable(create) = result.unwrap() {
        let customer_id_col = create
            .columns
            .iter()
            .find(|c| c.name == "customer_id")
            .expect("customer_id column not found");

        let fk_constraint = customer_id_col
            .constraints
            .iter()
            .find_map(|c| {
                if let ColumnConstraint::ForeignKey {
                    on_delete,
                    on_update,
                    ..
                } = c
                {
                    Some((on_delete, on_update))
                } else {
                    None
                }
            })
            .expect("ForeignKey constraint not found");

        assert_eq!(*fk_constraint.0, ReferentialAction::Cascade);
        assert_eq!(*fk_constraint.1, ReferentialAction::Restrict); // Default
    } else {
        panic!("Expected CreateTable statement");
    }
}

#[test]
fn test_parse_column_level_references_with_on_update_set_null() {
    let parser = Parser::new();
    let sql = "CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER REFERENCES customers(id) ON UPDATE SET NULL
    )";

    let result = parser.parse(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    if let neuroquantum_qsql::ast::Statement::CreateTable(create) = result.unwrap() {
        let customer_id_col = create
            .columns
            .iter()
            .find(|c| c.name == "customer_id")
            .expect("customer_id column not found");

        let fk_constraint = customer_id_col
            .constraints
            .iter()
            .find_map(|c| {
                if let ColumnConstraint::ForeignKey {
                    on_delete,
                    on_update,
                    ..
                } = c
                {
                    Some((on_delete, on_update))
                } else {
                    None
                }
            })
            .expect("ForeignKey constraint not found");

        assert_eq!(*fk_constraint.0, ReferentialAction::Restrict); // Default
        assert_eq!(*fk_constraint.1, ReferentialAction::SetNull);
    } else {
        panic!("Expected CreateTable statement");
    }
}

#[test]
fn test_parse_column_level_references_with_both_actions() {
    let parser = Parser::new();
    let sql = "CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER REFERENCES customers(id) ON DELETE CASCADE ON UPDATE SET NULL
    )";

    let result = parser.parse(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    if let neuroquantum_qsql::ast::Statement::CreateTable(create) = result.unwrap() {
        let customer_id_col = create
            .columns
            .iter()
            .find(|c| c.name == "customer_id")
            .expect("customer_id column not found");

        let fk_constraint = customer_id_col
            .constraints
            .iter()
            .find_map(|c| {
                if let ColumnConstraint::ForeignKey {
                    on_delete,
                    on_update,
                    ..
                } = c
                {
                    Some((on_delete, on_update))
                } else {
                    None
                }
            })
            .expect("ForeignKey constraint not found");

        assert_eq!(*fk_constraint.0, ReferentialAction::Cascade);
        assert_eq!(*fk_constraint.1, ReferentialAction::SetNull);
    } else {
        panic!("Expected CreateTable statement");
    }
}

#[test]
fn test_parse_column_level_references_no_action() {
    let parser = Parser::new();
    let sql = "CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER REFERENCES customers(id) ON DELETE NO ACTION
    )";

    let result = parser.parse(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    if let neuroquantum_qsql::ast::Statement::CreateTable(create) = result.unwrap() {
        let customer_id_col = create
            .columns
            .iter()
            .find(|c| c.name == "customer_id")
            .expect("customer_id column not found");

        let fk_constraint = customer_id_col
            .constraints
            .iter()
            .find_map(|c| {
                if let ColumnConstraint::ForeignKey { on_delete, .. } = c {
                    Some(on_delete)
                } else {
                    None
                }
            })
            .expect("ForeignKey constraint not found");

        assert_eq!(*fk_constraint, ReferentialAction::NoAction);
    } else {
        panic!("Expected CreateTable statement");
    }
}

#[test]
fn test_parse_table_level_foreign_key() {
    let parser = Parser::new();
    let sql = "CREATE TABLE orders (
        id INTEGER PRIMARY KEY,
        customer_id INTEGER,
        FOREIGN KEY (customer_id) REFERENCES customers(id) ON DELETE CASCADE
    )";

    let result = parser.parse(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    if let neuroquantum_qsql::ast::Statement::CreateTable(create) = result.unwrap() {
        assert_eq!(create.table_name, "orders");

        // Check table-level constraints
        let fk_constraint = create.constraints.iter().find_map(|c| {
            if let TableConstraint::ForeignKey {
                columns,
                referenced_table,
                referenced_columns,
                on_delete,
                on_update,
                ..
            } = c
            {
                Some((
                    columns,
                    referenced_table,
                    referenced_columns,
                    on_delete,
                    on_update,
                ))
            } else {
                None
            }
        });

        assert!(fk_constraint.is_some(), "ForeignKey constraint not found");
        let (columns, ref_table, ref_columns, on_delete, on_update) = fk_constraint.unwrap();

        assert_eq!(columns, &vec!["customer_id".to_string()]);
        assert_eq!(ref_table, "customers");
        assert_eq!(ref_columns, &vec!["id".to_string()]);
        assert_eq!(*on_delete, ReferentialAction::Cascade);
        assert_eq!(*on_update, ReferentialAction::Restrict);
    } else {
        panic!("Expected CreateTable statement");
    }
}

#[test]
fn test_parse_composite_foreign_key() {
    let parser = Parser::new();
    let sql = "CREATE TABLE order_items (
        id INTEGER PRIMARY KEY,
        order_id INTEGER,
        product_id INTEGER,
        FOREIGN KEY (order_id, product_id) REFERENCES order_products(order_id, product_id) ON DELETE CASCADE ON UPDATE CASCADE
    )";

    let result = parser.parse(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    if let neuroquantum_qsql::ast::Statement::CreateTable(create) = result.unwrap() {
        let fk_constraint = create.constraints.iter().find_map(|c| {
            if let TableConstraint::ForeignKey {
                columns,
                referenced_table,
                referenced_columns,
                on_delete,
                on_update,
                ..
            } = c
            {
                Some((
                    columns,
                    referenced_table,
                    referenced_columns,
                    on_delete,
                    on_update,
                ))
            } else {
                None
            }
        });

        assert!(fk_constraint.is_some(), "ForeignKey constraint not found");
        let (columns, ref_table, ref_columns, on_delete, on_update) = fk_constraint.unwrap();

        assert_eq!(
            columns,
            &vec!["order_id".to_string(), "product_id".to_string()]
        );
        assert_eq!(ref_table, "order_products");
        assert_eq!(
            ref_columns,
            &vec!["order_id".to_string(), "product_id".to_string()]
        );
        assert_eq!(*on_delete, ReferentialAction::Cascade);
        assert_eq!(*on_update, ReferentialAction::Cascade);
    } else {
        panic!("Expected CreateTable statement");
    }
}

// =============================================================================
// ReferentialAction Display Tests
// =============================================================================

#[test]
fn test_referential_action_display() {
    assert_eq!(format!("{}", ReferentialAction::Restrict), "RESTRICT");
    assert_eq!(format!("{}", ReferentialAction::Cascade), "CASCADE");
    assert_eq!(format!("{}", ReferentialAction::SetNull), "SET NULL");
    assert_eq!(format!("{}", ReferentialAction::SetDefault), "SET DEFAULT");
    assert_eq!(format!("{}", ReferentialAction::NoAction), "NO ACTION");
}

// =============================================================================
// Integration Tests (with storage engine)
// =============================================================================

#[cfg(test)]
mod storage_integration_tests {
    use std::sync::Arc;

    use neuroquantum_core::storage::StorageEngine;
    use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    async fn setup_test_env() -> (TempDir, QueryExecutor, Parser) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage = StorageEngine::new(temp_dir.path())
            .await
            .expect("Failed to create storage");
        let storage_arc = Arc::new(RwLock::new(storage));

        let config = ExecutorConfig {
            enable_neuromorphic_learning: false,
            enable_synaptic_optimization: false,
            enable_dna_compression: false,
            ..Default::default()
        };
        let executor =
            QueryExecutor::with_storage(config, storage_arc).expect("Failed to create executor");
        let parser = Parser::new();

        (temp_dir, executor, parser)
    }

    #[tokio::test]
    async fn test_foreign_key_insert_violation() {
        let (_temp_dir, mut executor, parser) = setup_test_env().await;

        // Create parent table
        let stmt = parser
            .parse("CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to create customers table");

        // Create child table with foreign key
        let stmt = parser
            .parse(
                "CREATE TABLE orders (
                id INTEGER PRIMARY KEY,
                customer_id INTEGER REFERENCES customers(id)
            )",
            )
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to create orders table");

        // Insert a customer
        let stmt = parser
            .parse("INSERT INTO customers (id, name) VALUES (1, 'Alice')")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to insert customer");

        // Insert order with valid customer_id - should succeed
        let stmt = parser
            .parse("INSERT INTO orders (id, customer_id) VALUES (1, 1)")
            .unwrap();
        let result = executor.execute_statement(&stmt).await;
        assert!(result.is_ok(), "Valid FK insert should succeed");

        // Insert order with invalid customer_id - should fail
        let stmt = parser
            .parse("INSERT INTO orders (id, customer_id) VALUES (2, 999)")
            .unwrap();
        let result = executor.execute_statement(&stmt).await;
        assert!(result.is_err(), "Invalid FK insert should fail: {result:?}");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Foreign key violation")
                || err.to_string().contains("does not exist"),
            "Error message should mention foreign key violation: {err}"
        );
    }

    #[tokio::test]
    async fn test_foreign_key_delete_restrict() {
        let (_temp_dir, mut executor, parser) = setup_test_env().await;

        // Create parent table
        let stmt = parser
            .parse("CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to create customers table");

        // Create child table with RESTRICT (default)
        let stmt = parser
            .parse(
                "CREATE TABLE orders (
                id INTEGER PRIMARY KEY,
                customer_id INTEGER REFERENCES customers(id)
            )",
            )
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to create orders table");

        // Insert data
        let stmt = parser
            .parse("INSERT INTO customers (id, name) VALUES (1, 'Alice')")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to insert customer");

        let stmt = parser
            .parse("INSERT INTO orders (id, customer_id) VALUES (1, 1)")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to insert order");

        // Try to delete customer - should fail due to RESTRICT
        let stmt = parser.parse("DELETE FROM customers WHERE id = 1").unwrap();
        let result = executor.execute_statement(&stmt).await;
        assert!(
            result.is_err(),
            "Delete with RESTRICT should fail when referenced"
        );
    }

    #[tokio::test]
    async fn test_foreign_key_delete_cascade() {
        let (_temp_dir, mut executor, parser) = setup_test_env().await;

        // Create parent table
        let stmt = parser
            .parse("CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to create customers table");

        // Create child table with CASCADE
        let stmt = parser
            .parse(
                "CREATE TABLE orders (
                id INTEGER PRIMARY KEY,
                customer_id INTEGER REFERENCES customers(id) ON DELETE CASCADE
            )",
            )
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to create orders table");

        // Insert data
        let stmt = parser
            .parse("INSERT INTO customers (id, name) VALUES (1, 'Alice')")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to insert customer");

        let stmt = parser
            .parse("INSERT INTO orders (id, customer_id) VALUES (1, 1)")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to insert order");

        let stmt = parser
            .parse("INSERT INTO orders (id, customer_id) VALUES (2, 1)")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to insert order 2");

        // Delete customer - should cascade to orders
        let stmt = parser.parse("DELETE FROM customers WHERE id = 1").unwrap();
        let result = executor.execute_statement(&stmt).await;
        assert!(
            result.is_ok(),
            "Delete with CASCADE should succeed: {:?}",
            result.err()
        );

        // Verify orders were also deleted
        let stmt = parser.parse("SELECT * FROM orders").unwrap();
        let result = executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to select orders");
        assert_eq!(result.rows.len(), 0, "Orders should be deleted via cascade");
    }

    #[tokio::test]
    async fn test_foreign_key_null_value_allowed() {
        let (_temp_dir, mut executor, parser) = setup_test_env().await;

        // Create parent table
        let stmt = parser
            .parse("CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to create customers table");

        // Create child table
        let stmt = parser
            .parse(
                "CREATE TABLE orders (
                id INTEGER PRIMARY KEY,
                customer_id INTEGER REFERENCES customers(id)
            )",
            )
            .unwrap();
        executor
            .execute_statement(&stmt)
            .await
            .expect("Failed to create orders table");

        // Insert order with NULL customer_id - should be allowed
        let stmt = parser
            .parse("INSERT INTO orders (id, customer_id) VALUES (1, NULL)")
            .unwrap();
        let result = executor.execute_statement(&stmt).await;
        assert!(
            result.is_ok(),
            "NULL FK value should be allowed: {:?}",
            result.err()
        );
    }
}
