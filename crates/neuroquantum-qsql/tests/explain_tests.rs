//! Integration tests for EXPLAIN functionality
//!
//! Tests for query plan explanation and analysis features including:
//! - Text format output
//! - JSON format output
//! - NEUROMATCH explain plans
//! - QUANTUM_SEARCH explain plans

use std::sync::Arc;
use std::time::Duration;

use neuroquantum_qsql::ast::{
    ExplainFormat, Expression, FromClause, Literal, NeuroMatchStatement, QuantumSearchStatement,
    SelectStatement, Statement, TableReference,
};
use neuroquantum_qsql::explain::{ExplainConfig, ExplainGenerator, NodeType};
use neuroquantum_qsql::query_plan::{
    ExecutionStrategy, OptimizationMetadata, QueryPlan, QuantumOptimization,
    QuantumOptimizationType, SynapticPathway,
};

#[test]
fn test_explain_generator() {
    let config = ExplainConfig::default();
    let generator = ExplainGenerator::new(config);

    let select = SelectStatement {
        select_list: vec![],
        from: Some(FromClause {
            relations: vec![TableReference {
                name: "users".to_string(),
                alias: None,
                synaptic_weight: None,
                quantum_state: None,
                subquery: None,
            }],
            joins: vec![],
        }),
        where_clause: None,
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
    };

    let query_plan = QueryPlan {
        statement: Arc::new(Statement::Select(select)),
        execution_strategy: ExecutionStrategy::Sequential,
        synaptic_pathways: vec![],
        quantum_optimizations: vec![],
        estimated_cost: 100.0,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(1),
            iterations_used: 1,
            convergence_achieved: true,
            synaptic_adaptations: 0,
            quantum_optimizations_applied: 0,
        },
    };

    let explain_plan = generator.generate_explain(&query_plan, false).unwrap();

    assert_eq!(explain_plan.total_cost, 100.0);
    assert!(!explain_plan.plan_nodes.is_empty());
    assert!(explain_plan.planning_time.as_millis() < 100);
}

#[test]
fn test_explain_text_format() {
    let config = ExplainConfig::default();
    let generator = ExplainGenerator::new(config);

    let select = SelectStatement {
        select_list: vec![],
        from: Some(FromClause {
            relations: vec![TableReference {
                name: "sensors".to_string(),
                alias: None,
                synaptic_weight: None,
                quantum_state: None,
                subquery: None,
            }],
            joins: vec![],
        }),
        where_clause: Some(Expression::Literal(Literal::Boolean(true))),
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
    };

    let query_plan = QueryPlan {
        statement: Arc::new(Statement::Select(select)),
        execution_strategy: ExecutionStrategy::Sequential,
        synaptic_pathways: vec![],
        quantum_optimizations: vec![],
        estimated_cost: 250.0,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(2),
            iterations_used: 1,
            convergence_achieved: true,
            synaptic_adaptations: 0,
            quantum_optimizations_applied: 0,
        },
    };

    let explain_plan = generator.generate_explain(&query_plan, false).unwrap();
    let text = generator.format_text(&explain_plan);

    assert!(text.contains("Query Plan"));
    assert!(text.contains("Seq Scan"));
    assert!(text.contains("sensors"));
    assert!(text.contains("cost="));
}

#[test]
fn test_explain_neuromatch() {
    let config = ExplainConfig::default();
    let generator = ExplainGenerator::new(config);

    let neuromatch = NeuroMatchStatement {
        target_table: "patterns".to_string(),
        pattern_expression: Expression::Literal(Literal::String("test".to_string())),
        synaptic_weight: 0.85,
        learning_rate: Some(0.01),
        activation_threshold: Some(0.5),
        hebbian_strengthening: true,
    };

    let query_plan = QueryPlan {
        statement: Arc::new(Statement::NeuroMatch(neuromatch)),
        execution_strategy: ExecutionStrategy::NeuromorphicOptimized,
        synaptic_pathways: vec![SynapticPathway {
            pathway_id: "pathway_1".to_string(),
            weight: 0.9,
            activation_threshold: 0.5,
            plasticity_enabled: true,
        }],
        quantum_optimizations: vec![],
        estimated_cost: 150.0,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(3),
            iterations_used: 5,
            convergence_achieved: true,
            synaptic_adaptations: 1,
            quantum_optimizations_applied: 0,
        },
    };

    let explain_plan = generator.generate_explain(&query_plan, false).unwrap();

    assert!(explain_plan.synaptic_score > 0.0);
    assert_eq!(
        explain_plan.plan_nodes[0].node_type,
        NodeType::NeuromorphicScan
    );
    assert!(!explain_plan.plan_nodes[0].synaptic_pathways.is_empty());
}

#[test]
fn test_explain_quantum_search() {
    let config = ExplainConfig::default();
    let generator = ExplainGenerator::new(config);

    let quantum = QuantumSearchStatement {
        target_table: "large_dataset".to_string(),
        search_expression: Expression::Literal(Literal::Integer(42)),
        amplitude_amplification: true,
        oracle_function: Some("find_target".to_string()),
        max_iterations: Some(20),
    };

    let query_plan = QueryPlan {
        statement: Arc::new(Statement::QuantumSearch(quantum)),
        execution_strategy: ExecutionStrategy::QuantumInspired,
        synaptic_pathways: vec![],
        quantum_optimizations: vec![QuantumOptimization {
            optimization_type: QuantumOptimizationType::GroverSearch,
            speedup_factor: 1.414,
            coherence_time: Duration::from_millis(100),
        }],
        estimated_cost: 80.0,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(2),
            iterations_used: 20,
            convergence_achieved: true,
            synaptic_adaptations: 0,
            quantum_optimizations_applied: 1,
        },
    };

    let explain_plan = generator.generate_explain(&query_plan, false).unwrap();

    assert!(explain_plan.quantum_score > 0.0);
    assert_eq!(explain_plan.plan_nodes[0].node_type, NodeType::GroverSearch);
    assert!(!explain_plan.plan_nodes[0].quantum_operations.is_empty());
    assert!(explain_plan.plan_nodes[0].quantum_advantage.is_some());
}

#[test]
fn test_explain_json_format() {
    let config = ExplainConfig {
        format: ExplainFormat::Json,
        ..Default::default()
    };
    let generator = ExplainGenerator::new(config);

    let select = SelectStatement {
        select_list: vec![],
        from: None,
        where_clause: None,
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
    };

    let query_plan = QueryPlan {
        statement: Arc::new(Statement::Select(select)),
        execution_strategy: ExecutionStrategy::Sequential,
        synaptic_pathways: vec![],
        quantum_optimizations: vec![],
        estimated_cost: 50.0,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(1),
            iterations_used: 1,
            convergence_achieved: true,
            synaptic_adaptations: 0,
            quantum_optimizations_applied: 0,
        },
    };

    let explain_plan = generator.generate_explain(&query_plan, false).unwrap();
    let json = generator.format_json(&explain_plan).unwrap();

    assert!(json.contains("\"total_cost\""));
    assert!(json.contains("\"plan_nodes\""));
    assert!(json.contains("\"planning_time\""));
}
