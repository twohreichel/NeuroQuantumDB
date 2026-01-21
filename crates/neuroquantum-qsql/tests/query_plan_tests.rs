//! Tests for query plan parsing functionality
//!
//! Note: Tests that require ExecutorConfig::testing() (which uses allow_legacy_mode)
//! must stay inline in src/query_plan.rs because they rely on #[cfg(test)] only features.
//! This file contains only the parser-related tests that don't require the executor.

use neuroquantum_qsql::ast::{Expression, SelectItem, Statement};
use neuroquantum_qsql::parser::QSQLParser;

#[test]
fn test_extract_year_parsing() {
    let parser = QSQLParser::new();
    let sql = "SELECT EXTRACT(YEAR FROM '2025-12-23')";
    let result = parser.parse_query(sql);

    if let Err(e) = &result {
        eprintln!("Parse error: {e:?}");
    }

    assert!(result.is_ok(), "Failed to parse EXTRACT(YEAR FROM date)");

    let stmt = result.unwrap();
    if let Statement::Select(select) = stmt {
        assert_eq!(select.select_list.len(), 1);
        if let SelectItem::Expression { expr, .. } = &select.select_list[0] {
            match expr {
                Expression::Extract { field, .. } => {
                    assert_eq!(field, "YEAR");
                }
                _ => panic!("Expected Extract expression"),
            }
        }
    } else {
        panic!("Expected SELECT statement");
    }
}

#[test]
fn test_extract_all_fields() {
    let parser = QSQLParser::new();
    let fields = vec![
        "YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND", "DOW", "DOY", "WEEK", "QUARTER",
        "EPOCH",
    ];

    for field in fields {
        let sql = format!("SELECT EXTRACT({field} FROM '2025-12-23 14:30:45')");
        let result = parser.parse_query(&sql);
        assert!(result.is_ok(), "Failed to parse EXTRACT({field} FROM date)");
    }
}

#[test]
fn test_extract_in_where_clause() {
    let parser = QSQLParser::new();
    let sql = "SELECT * FROM events WHERE EXTRACT(YEAR FROM created_at) = 2025";
    let result = parser.parse_query(sql);
    assert!(result.is_ok(), "Failed to parse EXTRACT in WHERE clause");
}

#[test]
fn test_extract_missing_from_keyword() {
    let parser = QSQLParser::new();
    let sql = "SELECT EXTRACT(YEAR '2025-12-23')";
    let result = parser.parse_query(sql);
    assert!(result.is_err(), "Should fail without FROM keyword");
}
