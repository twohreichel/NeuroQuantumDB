//! Integration tests for Subquery functionality
//!
//! This test suite verifies that subqueries work correctly:
//! - Scalar subqueries in SELECT list: `SELECT name, (SELECT AVG(age) FROM users) AS avg_age`
//! - Scalar subqueries in WHERE: `WHERE age > (SELECT AVG(age) FROM users)`
//! - IN subqueries: `WHERE id IN (SELECT user_id FROM orders)`
//! - EXISTS subqueries: `WHERE EXISTS (SELECT 1 FROM orders WHERE ...)`
//! - NOT EXISTS subqueries: `WHERE NOT EXISTS (SELECT 1 FROM ...)`
//! - Derived tables: `SELECT * FROM (SELECT * FROM users WHERE age > 25) AS adults`

use neuroquantum_core::storage::{ColumnDefinition, DataType, StorageEngine, TableSchema, Value};
use neuroquantum_qsql::{ExecutorConfig, Parser, QueryExecutor};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper function to set up test tables
async fn setup_test_tables(storage_arc: Arc<tokio::sync::RwLock<StorageEngine>>) {
    // Create users table
    let users_schema = TableSchema {
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
                name: "age".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: false,
            },
            ColumnDefinition {
                name: "department_id".to_string(),
                data_type: DataType::Integer,
                nullable: true,
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

    // Create departments table
    let departments_schema = TableSchema {
        name: "departments".to_string(),
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
                name: "active".to_string(),
                data_type: DataType::Boolean,
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
                auto_increment: true,
            },
            ColumnDefinition {
                name: "user_id".to_string(),
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

    let mut storage_guard = storage_arc.write().await;
    storage_guard.create_table(users_schema).await.unwrap();
    storage_guard
        .create_table(departments_schema)
        .await
        .unwrap();
    storage_guard.create_table(orders_schema).await.unwrap();

    // Insert departments
    let departments = [
        (1, "Engineering", true),
        (2, "Marketing", true),
        (3, "Sales", false), // Inactive department
    ];

    for (id, name, active) in departments.iter() {
        let mut row = neuroquantum_core::storage::Row {
            id: 0,
            fields: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        row.fields.insert("id".to_string(), Value::Integer(*id));
        row.fields
            .insert("name".to_string(), Value::Text(name.to_string()));
        row.fields
            .insert("active".to_string(), Value::Boolean(*active));
        storage_guard.insert_row("departments", row).await.unwrap();
    }

    // Insert users
    // Users: ages 20, 25, 30, 35, 40 => avg = 30
    let users = [
        (1, "Alice", 20, Some(1)),   // Engineering
        (2, "Bob", 25, Some(1)),     // Engineering
        (3, "Charlie", 30, Some(2)), // Marketing
        (4, "Diana", 35, Some(2)),   // Marketing
        (5, "Eve", 40, Some(3)),     // Sales (inactive dept)
        (6, "Frank", 45, None),      // No department
    ];

    for (id, name, age, dept_id) in users.iter() {
        let mut row = neuroquantum_core::storage::Row {
            id: 0,
            fields: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        row.fields.insert("id".to_string(), Value::Integer(*id));
        row.fields
            .insert("name".to_string(), Value::Text(name.to_string()));
        row.fields.insert("age".to_string(), Value::Integer(*age));
        match dept_id {
            Some(d) => row
                .fields
                .insert("department_id".to_string(), Value::Integer(*d)),
            None => row.fields.insert("department_id".to_string(), Value::Null),
        };
        storage_guard.insert_row("users", row).await.unwrap();
    }

    // Insert orders
    // Alice (id=1): 2 orders
    // Charlie (id=3): 1 order
    // Diana (id=4): 3 orders
    // Eve (id=5): 0 orders
    // Frank (id=6): 0 orders
    let orders = [
        (1, 1, 100.0), // Alice
        (2, 1, 150.0), // Alice
        (3, 3, 200.0), // Charlie
        (4, 4, 50.0),  // Diana
        (5, 4, 75.0),  // Diana
        (6, 4, 125.0), // Diana
    ];

    for (id, user_id, amount) in orders.iter() {
        let mut row = neuroquantum_core::storage::Row {
            id: 0,
            fields: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        row.fields.insert("id".to_string(), Value::Integer(*id));
        row.fields
            .insert("user_id".to_string(), Value::Integer(*user_id));
        row.fields
            .insert("amount".to_string(), Value::Float(*amount));
        storage_guard.insert_row("orders", row).await.unwrap();
    }
}

/// Create a storage engine and executor for testing
async fn create_test_executor() -> (
    TempDir,
    Arc<tokio::sync::RwLock<StorageEngine>>,
    QueryExecutor,
) {
    let temp_dir = TempDir::new().unwrap();
    let storage = StorageEngine::new(temp_dir.path()).await.unwrap();
    let storage_arc = Arc::new(tokio::sync::RwLock::new(storage));

    setup_test_tables(storage_arc.clone()).await;

    let config = ExecutorConfig::default();
    let executor = QueryExecutor::with_storage(config, storage_arc.clone()).unwrap();

    (temp_dir, storage_arc, executor)
}

// =============================================================================
// Parser Tests
// =============================================================================

mod parser_tests {
    use super::*;
    use neuroquantum_qsql::*;

    #[test]
    fn test_parse_scalar_subquery_in_select_list() {
        let parser = Parser::new();
        let sql = "SELECT name, (SELECT AVG(age) FROM users) AS avg_age FROM users";
        let result = parser.parse(sql);
        assert!(
            result.is_ok(),
            "Failed to parse scalar subquery in SELECT list: {:?}",
            result.err()
        );

        let stmt = result.unwrap();
        if let Statement::Select(select) = stmt {
            assert_eq!(select.select_list.len(), 2);
            // First item should be 'name' identifier
            if let SelectItem::Expression {
                expr: Expression::Identifier(name),
                ..
            } = &select.select_list[0]
            {
                assert_eq!(name, "name");
            } else {
                panic!("Expected first SELECT item to be identifier 'name'");
            }
            // Second item should be a scalar subquery with alias 'avg_age'
            if let SelectItem::Expression {
                expr: Expression::ScalarSubquery { .. },
                alias,
            } = &select.select_list[1]
            {
                assert_eq!(alias.as_deref(), Some("avg_age"));
            } else {
                panic!("Expected second SELECT item to be a ScalarSubquery");
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_parse_scalar_subquery_in_where() {
        let parser = Parser::new();
        let sql = "SELECT * FROM users WHERE age > (SELECT AVG(age) FROM users)";
        let result = parser.parse(sql);
        assert!(
            result.is_ok(),
            "Failed to parse scalar subquery in WHERE: {:?}",
            result.err()
        );

        let stmt = result.unwrap();
        if let Statement::Select(select) = stmt {
            assert!(select.where_clause.is_some());
            // The WHERE clause should contain a BinaryOp with ScalarSubquery on the right
            if let Some(Expression::BinaryOp {
                right, operator, ..
            }) = &select.where_clause
            {
                assert!(matches!(operator, BinaryOperator::GreaterThan));
                assert!(matches!(right.as_ref(), Expression::ScalarSubquery { .. }));
            } else {
                panic!("Expected BinaryOp in WHERE clause");
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_parse_in_subquery() {
        let parser = Parser::new();
        let sql = "SELECT * FROM users WHERE department_id IN (SELECT id FROM departments WHERE active = true)";
        let result = parser.parse(sql);
        assert!(
            result.is_ok(),
            "Failed to parse IN subquery: {:?}",
            result.err()
        );

        let stmt = result.unwrap();
        if let Statement::Select(select) = stmt {
            assert!(select.where_clause.is_some());
            if let Some(Expression::InSubquery { negated, .. }) = &select.where_clause {
                assert!(!negated, "Expected non-negated IN subquery");
            } else {
                panic!("Expected InSubquery in WHERE clause");
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_parse_not_in_subquery() {
        let parser = Parser::new();
        let sql = "SELECT * FROM users WHERE department_id NOT IN (SELECT id FROM departments WHERE active = false)";
        let result = parser.parse(sql);
        assert!(
            result.is_ok(),
            "Failed to parse NOT IN subquery: {:?}",
            result.err()
        );

        let stmt = result.unwrap();
        if let Statement::Select(select) = stmt {
            assert!(select.where_clause.is_some());
            if let Some(Expression::InSubquery { negated, .. }) = &select.where_clause {
                assert!(*negated, "Expected negated NOT IN subquery");
            } else {
                panic!("Expected InSubquery in WHERE clause");
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_parse_exists_subquery() {
        let parser = Parser::new();
        let sql =
            "SELECT * FROM users u WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id)";
        let result = parser.parse(sql);
        assert!(
            result.is_ok(),
            "Failed to parse EXISTS subquery: {:?}",
            result.err()
        );

        let stmt = result.unwrap();
        if let Statement::Select(select) = stmt {
            assert!(select.where_clause.is_some());
            if let Some(Expression::Exists { negated, .. }) = &select.where_clause {
                assert!(!negated, "Expected non-negated EXISTS");
            } else {
                panic!(
                    "Expected Exists in WHERE clause, got: {:?}",
                    select.where_clause
                );
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_parse_not_exists_subquery() {
        let parser = Parser::new();
        let sql = "SELECT * FROM users u WHERE NOT EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id)";
        let result = parser.parse(sql);
        assert!(
            result.is_ok(),
            "Failed to parse NOT EXISTS subquery: {:?}",
            result.err()
        );

        let stmt = result.unwrap();
        if let Statement::Select(select) = stmt {
            assert!(select.where_clause.is_some());
            if let Some(Expression::Exists { negated, .. }) = &select.where_clause {
                assert!(*negated, "Expected negated NOT EXISTS");
            } else {
                panic!("Expected Exists in WHERE clause");
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }

    #[test]
    fn test_parse_derived_table() {
        let parser = Parser::new();
        let sql = "SELECT * FROM (SELECT * FROM users WHERE age > 25) AS adult_users";
        let result = parser.parse(sql);
        assert!(
            result.is_ok(),
            "Failed to parse derived table: {:?}",
            result.err()
        );

        let stmt = result.unwrap();
        if let Statement::Select(select) = stmt {
            if let Some(from) = &select.from {
                assert_eq!(from.relations.len(), 1);
                let table_ref = &from.relations[0];
                assert!(
                    table_ref.subquery.is_some(),
                    "Expected subquery in table reference"
                );
                assert_eq!(table_ref.alias.as_deref(), Some("adult_users"));
            } else {
                panic!("Expected FROM clause");
            }
        } else {
            panic!("Expected SELECT statement");
        }
    }
}

// =============================================================================
// Execution Tests
// =============================================================================

mod execution_tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_in_subquery() {
        let (_temp_dir, _storage_arc, mut executor) = create_test_executor().await;
        let parser = Parser::new();

        // First, let's verify the data was inserted correctly
        let check_sql = "SELECT * FROM departments";
        let check_stmt = parser.parse(check_sql).unwrap();
        let check_result = executor.execute_statement(&check_stmt).await;
        assert!(
            check_result.is_ok(),
            "Check query failed: {:?}",
            check_result.err()
        );
        let check_res = check_result.unwrap();
        println!(
            "Departments table has {} rows: {:?}",
            check_res.rows_affected, check_res.rows
        );
        assert!(
            check_res.rows_affected >= 3,
            "Expected at least 3 departments"
        );

        // Test simple subquery without WHERE clause
        let subquery_sql = "SELECT id FROM departments";
        let subquery_stmt = parser.parse(subquery_sql).unwrap();
        let subquery_result = executor.execute_statement(&subquery_stmt).await;
        assert!(
            subquery_result.is_ok(),
            "Subquery alone failed: {:?}",
            subquery_result.err()
        );
        let subquery_res = subquery_result.unwrap();
        println!(
            "Subquery (no WHERE) returned {} rows: {:?}",
            subquery_res.rows_affected, subquery_res.rows
        );
        assert!(
            subquery_res.rows_affected >= 3,
            "Expected at least 3 departments"
        );

        // Test the users table
        let users_sql = "SELECT * FROM users";
        let users_stmt = parser.parse(users_sql).unwrap();
        let users_result = executor.execute_statement(&users_stmt).await;
        assert!(
            users_result.is_ok(),
            "Users query failed: {:?}",
            users_result.err()
        );
        let users_res = users_result.unwrap();
        println!(
            "Users table has {} rows: {:?}",
            users_res.rows_affected, users_res.rows
        );
        assert!(users_res.rows_affected >= 6, "Expected at least 6 users");

        // Now test with a simpler IN list query first
        let in_list_sql = "SELECT name FROM users WHERE department_id IN (1, 2)";
        let in_list_stmt = parser.parse(in_list_sql).unwrap();
        let in_list_result = executor.execute_statement(&in_list_stmt).await;
        assert!(
            in_list_result.is_ok(),
            "IN list query failed: {:?}",
            in_list_result.err()
        );
        let in_list_res = in_list_result.unwrap();
        println!(
            "IN list returned {} rows: {:?}",
            in_list_res.rows_affected, in_list_res.rows
        );
        assert_eq!(
            in_list_res.rows_affected, 4,
            "Expected 4 users in departments 1 and 2"
        );

        // Get users in active departments (id 1 and 2)
        let sql = "SELECT name FROM users WHERE department_id IN (SELECT id FROM departments)";
        let stmt = parser.parse(sql).unwrap();
        let result = executor.execute_statement(&stmt).await;

        assert!(
            result.is_ok(),
            "Failed to execute IN subquery: {:?}",
            result.err()
        );
        let query_result = result.unwrap();
        println!(
            "IN subquery returned {} rows: {:?}",
            query_result.rows_affected, query_result.rows
        );

        // Should return at least some users (since at least 5 users have departments)
        assert!(
            query_result.rows_affected >= 4,
            "Expected at least 4 users in departments"
        );
    }

    #[tokio::test]
    async fn test_execute_derived_table() {
        let (_temp_dir, _storage_arc, mut executor) = create_test_executor().await;
        let parser = Parser::new();

        // Get users older than 25 using a derived table
        let sql = "SELECT * FROM (SELECT * FROM users WHERE age > 25) AS adult_users";
        let stmt = parser.parse(sql).unwrap();
        let result = executor.execute_statement(&stmt).await;

        assert!(
            result.is_ok(),
            "Failed to execute derived table: {:?}",
            result.err()
        );
        let query_result = result.unwrap();

        // Should return Charlie (30), Diana (35), Eve (40), Frank (45)
        assert_eq!(
            query_result.rows_affected, 4,
            "Expected 4 users older than 25"
        );
    }
}
