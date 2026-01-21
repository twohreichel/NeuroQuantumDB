//! Integration tests for Index Advisor functionality
//!
//! Tests for automatic index recommendations based on query pattern analysis including:
//! - Query tracking and statistics collection
//! - Full table scan detection
//! - Index recommendation generation
//! - Existing index handling
//! - Recommendation priority calculation

use neuroquantum_qsql::ast::*;
use neuroquantum_qsql::index_advisor::{IndexAdvisor, IndexAdvisorConfig, RecommendationPriority};

fn create_test_select(table: &str, where_column: Option<&str>) -> Statement {
    let where_clause = where_column.map(|col| Expression::BinaryOp {
        left: Box::new(Expression::Identifier(col.to_string())),
        operator: BinaryOperator::Equal,
        right: Box::new(Expression::Literal(Literal::Integer(1))),
    });

    Statement::Select(SelectStatement {
        select_list: vec![SelectItem::Wildcard],
        from: Some(FromClause {
            relations: vec![TableReference {
                name: table.to_string(),
                alias: None,
                synaptic_weight: None,
                quantum_state: None,
                subquery: None,
            }],
            joins: vec![],
        }),
        where_clause,
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
        offset: None,
        synaptic_weight: None,
        plasticity_threshold: None,
        neuromatch_clause: None,
        quantum_parallel: false,
        grover_iterations: None,
        with_clause: None,
        union_clause: None,
    })
}

#[test]
fn test_index_advisor_creation() {
    let advisor = IndexAdvisor::new();
    let stats = advisor.get_statistics();
    assert_eq!(stats.total_queries_analyzed, 0);
    assert_eq!(stats.tables_tracked, 0);
}

#[test]
fn test_track_simple_select() {
    let advisor = IndexAdvisor::new();
    let query = create_test_select("users", Some("id"));

    advisor.track_query(&query);

    let stats = advisor.get_statistics();
    assert_eq!(stats.total_queries_analyzed, 1);
    assert_eq!(stats.tables_tracked, 1);

    let table_stats = advisor.get_table_stats("users").unwrap();
    // query_count is incremented per column access, not per query
    assert!(table_stats.query_count >= 1);
    assert!(table_stats.columns.contains_key("id"));
}

#[test]
fn test_track_full_table_scan() {
    let advisor = IndexAdvisor::new();
    let query = create_test_select("users", None); // No WHERE clause

    advisor.track_query(&query);

    let stats = advisor.get_statistics();
    assert_eq!(stats.full_scan_queries, 1);
}

#[test]
fn test_generate_recommendations() {
    let config = IndexAdvisorConfig {
        min_query_threshold: 2, // Lower threshold for testing
        ..Default::default()
    };

    let advisor = IndexAdvisor::with_config(config);

    // Track multiple queries on the same column
    for _ in 0..5 {
        let query = create_test_select("users", Some("email"));
        advisor.track_query(&query);
    }

    let recommendations = advisor.get_recommendations();

    // Should recommend an index on email column
    assert!(!recommendations.is_empty());
    assert!(recommendations
        .iter()
        .any(|r| r.table_name == "users" && r.columns.contains(&"email".to_string())));
}

#[test]
fn test_existing_index_not_recommended() {
    let config = IndexAdvisorConfig {
        min_query_threshold: 2,
        ..Default::default()
    };

    let advisor = IndexAdvisor::with_config(config);

    // Register existing index
    advisor.register_existing_index("users", &["email".to_string()]);

    // Track queries
    for _ in 0..5 {
        let query = create_test_select("users", Some("email"));
        advisor.track_query(&query);
    }

    let recommendations = advisor.get_recommendations();

    // Should NOT recommend index on email since it already exists
    assert!(!recommendations
        .iter()
        .any(|r| r.table_name == "users" && r.columns.len() == 1 && r.columns[0] == "email"));
}

#[test]
fn test_clear_statistics() {
    let advisor = IndexAdvisor::new();

    for _ in 0..3 {
        let query = create_test_select("users", Some("id"));
        advisor.track_query(&query);
    }

    assert!(advisor.get_statistics().total_queries_analyzed > 0);

    advisor.clear_statistics();

    let stats = advisor.get_statistics();
    assert_eq!(stats.total_queries_analyzed, 0);
    assert_eq!(stats.tables_tracked, 0);
}

#[test]
fn test_recommendation_priority() {
    let config = IndexAdvisorConfig {
        min_query_threshold: 1,
        ..Default::default()
    };

    let advisor = IndexAdvisor::with_config(config);

    // Track many queries to create high-priority recommendation
    for _ in 0..100 {
        let query = create_test_select("orders", Some("customer_id"));
        advisor.track_query(&query);
    }

    let recommendations = advisor.get_recommendations();

    assert!(!recommendations.is_empty());
    // First recommendation should have high priority due to frequency
    assert!(matches!(
        recommendations[0].priority,
        RecommendationPriority::Critical | RecommendationPriority::High
    ));
}

#[test]
fn test_create_statement_format() {
    let config = IndexAdvisorConfig {
        min_query_threshold: 1,
        ..Default::default()
    };

    let advisor = IndexAdvisor::with_config(config);

    for _ in 0..5 {
        let query = create_test_select("products", Some("category"));
        advisor.track_query(&query);
    }

    let recommendations = advisor.get_recommendations();

    if let Some(rec) = recommendations.first() {
        assert!(rec.create_statement.starts_with("CREATE INDEX"));
        assert!(rec.create_statement.contains("products"));
        assert!(rec.create_statement.contains("category"));
    }
}

#[test]
fn test_tracking_disabled() {
    let config = IndexAdvisorConfig {
        enable_tracking: false,
        ..Default::default()
    };

    let advisor = IndexAdvisor::with_config(config);

    for _ in 0..10 {
        let query = create_test_select("users", Some("id"));
        advisor.track_query(&query);
    }

    let stats = advisor.get_statistics();
    // Tracking is disabled, so nothing should be recorded
    assert_eq!(stats.tables_tracked, 0);
}
