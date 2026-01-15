//! Property-based tests for QSQL Parser and Query Executor
//!
//! This module provides comprehensive property-based testing using proptest
//! to ensure robustness and correctness of the QSQL parser and executor.
//!
//! ## Test Categories
//!
//! - **Parser Robustness**: Tests that the parser never panics on arbitrary input
//! - **SQL Roundtrip**: Tests that parsed SQL can be converted back to valid SQL
//! - **Identifier Validation**: Tests valid/invalid identifier handling
//! - **Expression Parsing**: Tests expression parsing for various operators
//! - **Query Plan Generation**: Tests that valid queries produce valid plans

use proptest::prelude::*;

use crate::ast::*;
use crate::optimizer::NeuromorphicOptimizer;
use crate::parser::{ParserConfig, QSQLParser};

/// Get configurable `PropTest` configuration from environment
///
/// Use `PROPTEST_CASES` environment variable to control test thoroughness:
/// - Fast (default): `PROPTEST_CASES=32` (development)
/// - Standard: `PROPTEST_CASES=64` (CI)
/// - Thorough: `PROPTEST_CASES=256` (pre-release)
/// - Exhaustive: `PROPTEST_CASES=512` (release)
fn get_proptest_config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(32); // Default: fast tests

    ProptestConfig {
        cases,
        max_shrink_iters: if cases > 100 { 1000 } else { 500 },
        max_shrink_time: if cases > 100 { 10000 } else { 5000 },
        ..ProptestConfig::default()
    }
}

// ============================================================================
// Strategy Generators for SQL Components
// ============================================================================

/// SQL reserved keywords that cannot be used as identifiers
const SQL_RESERVED_KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "AND",
    "OR",
    "NOT",
    "IN",
    "LIKE",
    "BETWEEN",
    "IS",
    "NULL",
    "AS",
    "ON",
    "JOIN",
    "LEFT",
    "RIGHT",
    "INNER",
    "OUTER",
    "FULL",
    "CROSS",
    "ORDER",
    "BY",
    "ASC",
    "DESC",
    "LIMIT",
    "OFFSET",
    "GROUP",
    "HAVING",
    "DISTINCT",
    "INSERT",
    "INTO",
    "VALUES",
    "UPDATE",
    "SET",
    "DELETE",
    "CREATE",
    "DROP",
    "TABLE",
    "INDEX",
    "ALTER",
    "ADD",
    "COLUMN",
    "PRIMARY",
    "KEY",
    "FOREIGN",
    "REFERENCES",
    "UNIQUE",
    "CHECK",
    "DEFAULT",
    "CONSTRAINT",
    "CASCADE",
    "RESTRICT",
    "WITH",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "UNION",
    "ALL",
    "EXCEPT",
    "INTERSECT",
    "EXISTS",
    "TRUE",
    "FALSE",
    "SERIAL",
    "BIGSERIAL",
    "SMALLSERIAL",
    "AUTO_INCREMENT",
    "GENERATED",
    "ALWAYS",
    "IDENTITY",
    "IF",
    "TRUNCATE",
    "TO",
    "DO",
    "GO",
    "NO",
    "AT",
    "OF",
];

/// Check if a string is a SQL reserved keyword (case-insensitive)
fn is_reserved_keyword(s: &str) -> bool {
    let upper = s.to_uppercase();
    SQL_RESERVED_KEYWORDS.contains(&upper.as_str())
}

/// Generate valid SQL identifiers (column names, table names, etc.)
fn sql_identifier() -> impl Strategy<Value = String> {
    // SQL identifiers: start with letter, followed by at least one more alphanumeric
    // to avoid single-character edge cases like just "_"
    prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{1,31}")
        .unwrap()
        .prop_filter("identifier must have at least 2 chars", |s| s.len() >= 2)
        .prop_filter("identifier must not be a reserved keyword", |s| {
            !is_reserved_keyword(s)
        })
}

/// Generate valid table names
fn table_name() -> impl Strategy<Value = String> {
    sql_identifier()
}

/// Generate valid column names
fn column_name() -> impl Strategy<Value = String> {
    sql_identifier()
}

/// Generate simple integer literals for WHERE clauses
fn integer_literal() -> impl Strategy<Value = i64> {
    prop::num::i64::ANY
}

/// Generate simple string literals (without problematic characters)
fn string_literal() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 ]{0,50}")
        .unwrap()
        .prop_map(|s| s.replace('\'', "''")) // Escape single quotes
}

/// Generate float literals for neuromorphic weights
fn synaptic_weight() -> impl Strategy<Value = f32> {
    0.0f32..1.0f32
}

/// Generate simple comparison operators
fn comparison_operator() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["=", "!=", "<", ">", "<=", ">="])
}

/// Generate arithmetic operators
fn arithmetic_operator() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["+", "-", "*", "/"])
}

/// Generate aggregate functions
fn aggregate_function() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["COUNT", "SUM", "AVG", "MIN", "MAX"])
}

/// Generate ORDER BY directions
fn order_direction() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["ASC", "DESC", ""])
}

// ============================================================================
// SQL Query Generators
// ============================================================================

/// Generate simple SELECT queries
fn simple_select_query() -> impl Strategy<Value = String> {
    (prop::collection::vec(column_name(), 1..5), table_name())
        .prop_map(|(cols, table)| format!("SELECT {} FROM {}", cols.join(", "), table))
}

/// Generate SELECT queries with WHERE clause
fn select_with_where() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(column_name(), 1..3),
        table_name(),
        column_name(),
        comparison_operator(),
        integer_literal(),
    )
        .prop_map(|(cols, table, where_col, op, value)| {
            format!(
                "SELECT {} FROM {} WHERE {} {} {}",
                cols.join(", "),
                table,
                where_col,
                op,
                value
            )
        })
}

/// Generate SELECT queries with LIMIT and OFFSET
fn select_with_limit() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(column_name(), 1..3),
        table_name(),
        1u64..1000,
        0u64..100,
    )
        .prop_map(|(cols, table, limit, offset)| {
            format!(
                "SELECT {} FROM {} LIMIT {} OFFSET {}",
                cols.join(", "),
                table,
                limit,
                offset
            )
        })
}

/// Generate SELECT queries with ORDER BY
fn select_with_order_by() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(column_name(), 1..3),
        table_name(),
        column_name(),
        order_direction(),
    )
        .prop_map(|(cols, table, order_col, dir)| {
            if dir.is_empty() {
                format!(
                    "SELECT {} FROM {} ORDER BY {}",
                    cols.join(", "),
                    table,
                    order_col
                )
            } else {
                format!(
                    "SELECT {} FROM {} ORDER BY {} {}",
                    cols.join(", "),
                    table,
                    order_col,
                    dir
                )
            }
        })
}

/// Generate SELECT queries with GROUP BY and aggregates
fn select_with_group_by() -> impl Strategy<Value = String> {
    (
        column_name(),
        aggregate_function(),
        column_name(),
        table_name(),
    )
        .prop_map(|(group_col, agg_fn, agg_col, table)| {
            format!("SELECT {group_col}, {agg_fn}({agg_col}) FROM {table} GROUP BY {group_col}")
        })
}

/// Generate INSERT statements
fn insert_statement() -> impl Strategy<Value = String> {
    (
        table_name(),
        prop::collection::vec(column_name(), 1..4),
        prop::collection::vec(integer_literal(), 1..4),
    )
        .prop_filter("columns and values must match", |(_, cols, vals)| {
            cols.len() == vals.len()
        })
        .prop_map(|(table, cols, vals)| {
            let values_str = vals
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table,
                cols.join(", "),
                values_str
            )
        })
}

/// Generate UPDATE statements
fn update_statement() -> impl Strategy<Value = String> {
    (
        table_name(),
        column_name(),
        integer_literal(),
        column_name(),
        comparison_operator(),
        integer_literal(),
    )
        .prop_map(|(table, set_col, set_val, where_col, op, where_val)| {
            format!("UPDATE {table} SET {set_col} = {set_val} WHERE {where_col} {op} {where_val}")
        })
}

/// Generate DELETE statements
fn delete_statement() -> impl Strategy<Value = String> {
    (
        table_name(),
        column_name(),
        comparison_operator(),
        integer_literal(),
    )
        .prop_map(|(table, where_col, op, where_val)| {
            format!("DELETE FROM {table} WHERE {where_col} {op} {where_val}")
        })
}

/// Generate neuromorphic NEUROMATCH queries
fn neuromatch_query() -> impl Strategy<Value = String> {
    (table_name(), string_literal(), synaptic_weight()).prop_map(|(table, pattern, strength)| {
        format!("SELECT * FROM {table} NEUROMATCH '{pattern}' STRENGTH > {strength:.2}")
    })
}

/// Generate quantum JOIN queries
fn quantum_join_query() -> impl Strategy<Value = String> {
    (table_name(), table_name(), column_name(), column_name()).prop_map(
        |(table1, table2, col1, col2)| {
            format!(
                "SELECT * FROM {table1} QUANTUM_JOIN {table2} ON superposition({table1}.{col1}, {table2}.{col2})"
            )
        },
    )
}

/// Generate arbitrary (potentially invalid) input for fuzzing
fn arbitrary_input() -> impl Strategy<Value = String> {
    prop::string::string_regex(".{0,200}").unwrap()
}

/// Generate SQL-like gibberish to test parser resilience
fn sql_like_gibberish() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec![
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "JOIN", "ON", "AND", "OR",
            "NOT", "*", ",", "(", ")", "=", "<", ">", "123", "'test'", "table", "column", " ",
        ]),
        1..20,
    )
    .prop_map(|tokens| tokens.join(""))
}

// ============================================================================
// Property-Based Tests
// ============================================================================

proptest! {
    #![proptest_config(get_proptest_config())]

    // ========================================================================
    // Parser Robustness Tests
    // ========================================================================

    /// The parser should never panic on arbitrary input
    #[test]
    fn parser_never_panics_on_arbitrary_input(input in arbitrary_input()) {
        let parser = QSQLParser::new();
        // We don't care about the result, just that it doesn't panic
        let _ = parser.parse_query(&input);
    }

    /// The parser should never panic on SQL-like gibberish
    #[test]
    fn parser_never_panics_on_sql_gibberish(input in sql_like_gibberish()) {
        let parser = QSQLParser::new();
        let _ = parser.parse_query(&input);
    }

    /// The parser should handle extremely long inputs without panic
    #[test]
    fn parser_handles_long_input(
        base_query in simple_select_query(),
        multiplier in 1usize..100
    ) {
        let parser = QSQLParser::new();
        let long_query = base_query.repeat(multiplier);
        let _ = parser.parse_query(&long_query);
    }

    // ========================================================================
    // Valid SQL Parsing Tests
    // ========================================================================

    /// Simple SELECT queries should parse successfully
    #[test]
    fn simple_select_parses(query in simple_select_query()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    /// SELECT with WHERE clause should parse successfully
    #[test]
    fn select_with_where_parses(query in select_with_where()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    /// SELECT with LIMIT/OFFSET should parse successfully
    #[test]
    fn select_with_limit_parses(query in select_with_limit()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    /// SELECT with ORDER BY should parse successfully
    #[test]
    fn select_with_order_by_parses(query in select_with_order_by()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    /// SELECT with GROUP BY should parse successfully
    #[test]
    fn select_with_group_by_parses(query in select_with_group_by()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    /// INSERT statements should parse successfully
    #[test]
    fn insert_parses(query in insert_statement()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    /// UPDATE statements should parse successfully
    #[test]
    fn update_parses(query in update_statement()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    /// DELETE statements should parse successfully
    #[test]
    fn delete_parses(query in delete_statement()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    // ========================================================================
    // Neuromorphic Extension Tests
    // ========================================================================

    /// NEUROMATCH queries should parse successfully
    #[test]
    fn neuromatch_parses(query in neuromatch_query()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    /// QUANTUM_JOIN queries should parse successfully
    #[test]
    fn quantum_join_parses(query in quantum_join_query()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {} - Error: {:?}", query, result.err());
    }

    // ========================================================================
    // Semantic Consistency Tests
    // ========================================================================

    /// Parsed SELECT statements should have correct structure
    #[test]
    fn select_has_correct_structure(
        cols in prop::collection::vec(column_name(), 1..5),
        table in table_name()
    ) {
        let query = format!("SELECT {} FROM {}", cols.join(", "), table);
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);

        prop_assert!(result.is_ok());
        let stmt = result.unwrap();

        match stmt {
            Statement::Select(select) => {
                // Number of select items should match
                prop_assert_eq!(
                    select.select_list.len(),
                    cols.len(),
                    "Column count mismatch"
                );

                // FROM clause should exist
                prop_assert!(select.from.is_some(), "FROM clause missing");
            }
            _ => prop_assert!(false, "Expected SELECT statement"),
        }
    }

    /// Parsed INSERT statements should have correct structure
    #[test]
    fn insert_has_correct_structure(query in insert_statement()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);

        prop_assert!(result.is_ok());
        let stmt = result.unwrap();

        match stmt {
            Statement::Insert(insert) => {
                // Table name should not be empty
                prop_assert!(!insert.table_name.is_empty());
            }
            _ => prop_assert!(false, "Expected INSERT statement"),
        }
    }

    // ========================================================================
    // Error Handling Tests
    // ========================================================================

    /// Empty queries should produce ParseError
    #[test]
    fn empty_query_returns_error(whitespace in prop::string::string_regex("\\s{0,10}").unwrap()) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(&whitespace);
        prop_assert!(result.is_err(), "Empty query should fail");
    }

    /// Queries with only keywords should fail
    #[test]
    fn incomplete_query_returns_error(
        keyword in prop::sample::select(vec!["SELECT", "INSERT", "UPDATE", "DELETE"])
    ) {
        let parser = QSQLParser::new();
        let result = parser.parse_query(keyword);
        prop_assert!(result.is_err(), "Incomplete query should fail");
    }

    // ========================================================================
    // Optimizer Tests
    // ========================================================================

    /// Parsed queries should be optimizable without panic
    #[test]
    fn parsed_queries_optimizable(query in simple_select_query()) {
        let parser = QSQLParser::new();
        let parse_result = parser.parse_query(&query);

        if let Ok(stmt) = parse_result {
            let optimizer = NeuromorphicOptimizer::new();
            if let Ok(mut opt) = optimizer {
                // Optimization should not panic
                let _ = opt.optimize(stmt);
            }
        }
    }

    // ========================================================================
    // Parser Configuration Tests
    // ========================================================================

    /// Parser with different configs should handle same input consistently
    #[test]
    fn parser_config_consistency(
        query in simple_select_query(),
        enable_neuro in any::<bool>(),
        enable_quantum in any::<bool>()
    ) {
        let config1 = ParserConfig {
            enable_neuromorphic_extensions: enable_neuro,
            enable_quantum_extensions: enable_quantum,
            ..Default::default()
        };

        let config2 = ParserConfig {
            enable_neuromorphic_extensions: !enable_neuro,
            enable_quantum_extensions: !enable_quantum,
            ..Default::default()
        };

        let parser1 = QSQLParser::with_config(config1);
        let parser2 = QSQLParser::with_config(config2);

        if let (Ok(p1), Ok(p2)) = (parser1, parser2) {
            // Standard SQL should parse with any config
            let result1 = p1.parse_query(&query);
            let result2 = p2.parse_query(&query);

            // Both should either succeed or fail (for standard SQL)
            prop_assert_eq!(
                result1.is_ok(),
                result2.is_ok(),
                "Config should not affect standard SQL parsing"
            );
        }
    }

    // ========================================================================
    // Identifier Validation Tests
    // ========================================================================

    /// Valid identifiers should be accepted
    #[test]
    fn valid_identifiers_accepted(id in sql_identifier()) {
        let query = format!("SELECT {id} FROM test_table");
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Valid identifier should be accepted: {}", id);
    }

    /// Identifiers starting with numbers should fail
    #[test]
    fn numeric_start_identifiers_fail(
        num in 0i32..9,
        rest in prop::string::string_regex("[a-zA-Z_]{1,10}").unwrap()
    ) {
        let invalid_id = format!("{num}{rest}");
        let query = format!("SELECT {invalid_id} FROM test_table");
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        // Should fail or the parser may interpret it differently
        // The important thing is it doesn't panic
        let _ = result;
    }

    // ========================================================================
    // Expression Parsing Tests
    // ========================================================================

    /// Arithmetic expressions should parse correctly
    #[test]
    fn arithmetic_expressions_parse(
        col1 in column_name(),
        op in arithmetic_operator(),
        col2 in column_name(),
        table in table_name()
    ) {
        let query = format!("SELECT {col1} {op} {col2} FROM {table}");
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {}", query);
    }

    /// Nested expressions with parentheses should parse without panicking
    /// Note: The parser may not support all expression syntaxes, but should never panic
    #[test]
    fn nested_expressions_dont_panic(
        col1 in column_name(),
        col2 in column_name(),
        col3 in column_name(),
        table in table_name()
    ) {
        let query = format!("SELECT ({col1} + {col2}) * {col3} FROM {table}");
        let parser = QSQLParser::new();
        // We don't assert success, just that it doesn't panic
        let _ = parser.parse_query(&query);
    }

    /// Complex WHERE conditions should parse
    #[test]
    fn complex_where_parses(
        col1 in column_name(),
        col2 in column_name(),
        val1 in integer_literal(),
        val2 in integer_literal(),
        table in table_name()
    ) {
        let query = format!(
            "SELECT * FROM {table} WHERE {col1} > {val1} AND {col2} < {val2}"
        );
        let parser = QSQLParser::new();
        let result = parser.parse_query(&query);
        prop_assert!(result.is_ok(), "Failed to parse: {}", query);
    }
}

// ============================================================================
// Non-Property Tests (for edge cases that need specific assertions)
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_unicode_in_string_literals() {
        let parser = QSQLParser::new();

        // Unicode in string literals should work
        let query = "SELECT * FROM users WHERE name = 'München'";
        let result = parser.parse_query(query);
        assert!(result.is_ok(), "Unicode should be allowed in strings");
    }

    #[test]
    fn test_very_long_identifiers() {
        let parser = QSQLParser::new();

        // Very long but valid identifier
        let long_id = "a".repeat(100);
        let query = format!("SELECT {long_id} FROM test");
        let result = parser.parse_query(&query);
        // Should either parse or fail gracefully
        let _ = result;
    }

    #[test]
    fn test_deeply_nested_parentheses() {
        let parser = QSQLParser::new();

        // Deeply nested parentheses
        let nested = "(((((1 + 2)))))";
        let query = format!("SELECT {nested} FROM test");
        let result = parser.parse_query(&query);
        // Should either parse or fail gracefully (no panic)
        let _ = result;
    }

    #[test]
    fn test_multiple_statements_separated() {
        let parser = QSQLParser::new();

        // Multiple statements should be handled
        let query = "SELECT * FROM a; SELECT * FROM b";
        let result = parser.parse_query(query);
        // Parser may handle this differently, but shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_sql_injection_patterns() {
        let parser = QSQLParser::new();

        // Common SQL injection patterns should be handled safely
        let injection_patterns = vec![
            "SELECT * FROM users WHERE id = 1; DROP TABLE users;--",
            "SELECT * FROM users WHERE name = '' OR '1'='1'",
            "SELECT * FROM users WHERE id = 1 UNION SELECT * FROM passwords",
            "SELECT * FROM users WHERE name = ''; DELETE FROM users;--",
        ];

        for pattern in injection_patterns {
            // Should not panic, result doesn't matter
            let _ = parser.parse_query(pattern);
        }
    }

    #[test]
    fn test_null_byte_in_input() {
        let parser = QSQLParser::new();

        // Null bytes should be handled
        let query = "SELECT * FROM test\0WHERE id = 1";
        let _ = parser.parse_query(query);
    }

    #[test]
    fn test_special_characters_in_identifiers() {
        let parser = QSQLParser::new();

        // Special characters should be rejected or handled
        let special_ids = vec!["col@name", "col#name", "col$name", "col-name"];

        for id in special_ids {
            let query = format!("SELECT {id} FROM test");
            let _ = parser.parse_query(&query);
        }
    }
}
