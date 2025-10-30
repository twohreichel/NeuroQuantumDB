//! EXPLAIN and ANALYZE Demo
//!
//! This example demonstrates the EXPLAIN and ANALYZE functionality for
//! query plan visualization and analysis.
//!
//! Run with: cargo run --example explain_analyze_demo

use neuroquantum_qsql::ast::*;
use neuroquantum_qsql::explain::*;
use neuroquantum_qsql::query_plan::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║      NeuroQuantumDB - EXPLAIN & ANALYZE Demo                 ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // Demo 1: EXPLAIN SELECT
    demo_explain_select().await?;

    // Demo 2: EXPLAIN NEUROMATCH
    demo_explain_neuromatch().await?;

    // Demo 3: EXPLAIN QUANTUM_SEARCH
    demo_explain_quantum_search().await?;

    // Demo 4: EXPLAIN QUANTUM_JOIN
    demo_explain_quantum_join().await?;

    // Demo 5: EXPLAIN with different formats
    demo_explain_formats().await?;

    // Demo 6: ANALYZE table
    demo_analyze_table().await?;

    println!("\n✅ All EXPLAIN & ANALYZE demos completed successfully!");
    Ok(())
}

async fn demo_explain_select() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Demo 1: EXPLAIN SELECT Query");
    println!("{}", "─".repeat(70));

    let select = SelectStatement {
        select_list: vec![],
        from: Some(FromClause {
            relations: vec![TableReference {
                name: "sensors".to_string(),
                alias: None,
                synaptic_weight: None,
                quantum_state: None,
            }],
            joins: vec![],
        }),
        where_clause: Some(Expression::BinaryOp {
            left: Box::new(Expression::Identifier("temperature".to_string())),
            operator: BinaryOperator::GreaterThan,
            right: Box::new(Expression::Literal(Literal::Integer(25))),
        }),
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: Some(100),
        offset: None,
        synaptic_weight: None,
        plasticity_threshold: None,
        quantum_parallel: false,
        grover_iterations: None,
    };

    let query_plan = QueryPlan {
        statement: Statement::Select(select),
        execution_strategy: ExecutionStrategy::Sequential,
        synaptic_pathways: vec![],
        quantum_optimizations: vec![],
        estimated_cost: 250.5,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(2),
            iterations_used: 1,
            convergence_achieved: true,
            synaptic_adaptations: 0,
            quantum_optimizations_applied: 0,
        },
    };

    let config = ExplainConfig::default();
    let generator = ExplainGenerator::new(config);
    let explain_plan = generator.generate_explain(&query_plan, false)?;

    let text = generator.format_text(&explain_plan);
    println!("{}", text);

    println!("✅ SELECT query explained\n");
    Ok(())
}

async fn demo_explain_neuromatch() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Demo 2: EXPLAIN NEUROMATCH Query");
    println!("{}", "─".repeat(70));

    let neuromatch = NeuroMatchStatement {
        target_table: "brain_patterns".to_string(),
        pattern_expression: Expression::NeuroPattern {
            pattern: "alpha_wave".to_string(),
            similarity_threshold: 0.85,
        },
        synaptic_weight: 0.92,
        learning_rate: Some(0.01),
        activation_threshold: Some(0.7),
        hebbian_strengthening: true,
    };

    let query_plan = QueryPlan {
        statement: Statement::NeuroMatch(neuromatch),
        execution_strategy: ExecutionStrategy::NeuromorphicOptimized,
        synaptic_pathways: vec![
            SynapticPathway {
                pathway_id: "cortex_pathway_1".to_string(),
                weight: 0.95,
                activation_threshold: 0.7,
                plasticity_enabled: true,
            },
            SynapticPathway {
                pathway_id: "hippocampus_pathway_2".to_string(),
                weight: 0.88,
                activation_threshold: 0.6,
                plasticity_enabled: true,
            },
        ],
        quantum_optimizations: vec![],
        estimated_cost: 175.3,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(5),
            iterations_used: 8,
            convergence_achieved: true,
            synaptic_adaptations: 2,
            quantum_optimizations_applied: 0,
        },
    };

    let config = ExplainConfig::default();
    let generator = ExplainGenerator::new(config);
    let explain_plan = generator.generate_explain(&query_plan, false)?;

    let text = generator.format_text(&explain_plan);
    println!("{}", text);

    println!("✅ NEUROMATCH query explained with synaptic pathways\n");
    Ok(())
}

async fn demo_explain_quantum_search() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚛️  Demo 3: EXPLAIN QUANTUM_SEARCH Query");
    println!("{}", "─".repeat(70));

    let quantum = QuantumSearchStatement {
        target_table: "large_dataset".to_string(),
        search_expression: Expression::BinaryOp {
            left: Box::new(Expression::Identifier("id".to_string())),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Literal(Literal::Integer(42))),
        },
        amplitude_amplification: true,
        oracle_function: Some("find_target_record".to_string()),
        max_iterations: Some(15),
    };

    let query_plan = QueryPlan {
        statement: Statement::QuantumSearch(quantum),
        execution_strategy: ExecutionStrategy::QuantumInspired,
        synaptic_pathways: vec![],
        quantum_optimizations: vec![
            QuantumOptimization {
                optimization_type: QuantumOptimizationType::GroverSearch,
                speedup_factor: 1.414,
                coherence_time: Duration::from_millis(150),
            },
            QuantumOptimization {
                optimization_type: QuantumOptimizationType::AmplitudeAmplification,
                speedup_factor: 2.0,
                coherence_time: Duration::from_millis(100),
            },
        ],
        estimated_cost: 89.7,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(3),
            iterations_used: 15,
            convergence_achieved: true,
            synaptic_adaptations: 0,
            quantum_optimizations_applied: 2,
        },
    };

    let config = ExplainConfig::default();
    let generator = ExplainGenerator::new(config);
    let explain_plan = generator.generate_explain(&query_plan, false)?;

    let text = generator.format_text(&explain_plan);
    println!("{}", text);

    println!("✅ QUANTUM_SEARCH explained with quantum optimizations\n");
    Ok(())
}

async fn demo_explain_quantum_join() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Demo 4: EXPLAIN QUANTUM_JOIN Query");
    println!("{}", "─".repeat(70));

    let qjoin = QuantumJoinStatement {
        left_table: "users".to_string(),
        right_table: "orders".to_string(),
        on_condition: Some(Expression::BinaryOp {
            left: Box::new(Expression::Identifier("users.id".to_string())),
            operator: BinaryOperator::Equal,
            right: Box::new(Expression::Identifier("orders.user_id".to_string())),
        }),
        using_columns: vec![],
        quantum_state: Some("entangled".to_string()),
    };

    let query_plan = QueryPlan {
        statement: Statement::QuantumJoin(qjoin),
        execution_strategy: ExecutionStrategy::QuantumInspired,
        synaptic_pathways: vec![],
        quantum_optimizations: vec![QuantumOptimization {
            optimization_type: QuantumOptimizationType::QuantumJoin,
            speedup_factor: 1.5,
            coherence_time: Duration::from_millis(200),
        }],
        estimated_cost: 320.8,
        optimization_metadata: OptimizationMetadata {
            optimization_time: Duration::from_millis(4),
            iterations_used: 1,
            convergence_achieved: true,
            synaptic_adaptations: 0,
            quantum_optimizations_applied: 1,
        },
    };

    let config = ExplainConfig::default();
    let generator = ExplainGenerator::new(config);
    let explain_plan = generator.generate_explain(&query_plan, false)?;

    let text = generator.format_text(&explain_plan);
    println!("{}", text);

    println!("✅ QUANTUM_JOIN explained\n");
    Ok(())
}

async fn demo_explain_formats() -> Result<(), Box<dyn std::error::Error>> {
    println!("📄 Demo 5: EXPLAIN with Different Formats");
    println!("{}", "─".repeat(70));

    let select = SelectStatement {
        select_list: vec![],
        from: Some(FromClause {
            relations: vec![TableReference {
                name: "test_table".to_string(),
                alias: None,
                synaptic_weight: None,
                quantum_state: None,
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
        quantum_parallel: false,
        grover_iterations: None,
    };

    let query_plan = QueryPlan {
        statement: Statement::Select(select),
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

    // JSON Format
    println!("\n🔹 JSON Format:");
    let config_json = ExplainConfig {
        format: ExplainFormat::Json,
        ..Default::default()
    };
    let generator_json = ExplainGenerator::new(config_json);
    let explain_plan = generator_json.generate_explain(&query_plan, false)?;
    let json = generator_json.format_json(&explain_plan)?;
    println!("{}", &json[..200.min(json.len())]); // Print first 200 chars
    if json.len() > 200 {
        println!("... (truncated)");
    }

    // YAML Format
    println!("\n🔹 YAML Format:");
    let config_yaml = ExplainConfig {
        format: ExplainFormat::Yaml,
        ..Default::default()
    };
    let generator_yaml = ExplainGenerator::new(config_yaml);
    let explain_plan = generator_yaml.generate_explain(&query_plan, false)?;
    let yaml = generator_yaml.format_yaml(&explain_plan)?;
    println!("{}", &yaml[..200.min(yaml.len())]); // Print first 200 chars
    if yaml.len() > 200 {
        println!("... (truncated)");
    }

    println!("\n✅ Multiple formats demonstrated\n");
    Ok(())
}

async fn demo_analyze_table() -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 Demo 6: ANALYZE Table Statistics");
    println!("{}", "─".repeat(70));

    let stats = TableStatistics {
        table_name: "user_activity".to_string(),
        row_count: 1_000_000,
        page_count: 5000,
        avg_row_size: 256,
        null_frac: std::collections::HashMap::new(),
        distinct_values: std::collections::HashMap::new(),
        most_common_values: std::collections::HashMap::new(),
        histogram_bounds: std::collections::HashMap::new(),
        last_analyzed: std::time::SystemTime::now(),
        synaptic_density: 0.78,
        plasticity_index: 0.85,
    };

    println!("Table: {}", stats.table_name);
    println!("  Row Count:          {:>12}", stats.row_count);
    println!("  Page Count:         {:>12}", stats.page_count);
    println!("  Avg Row Size:       {:>12} bytes", stats.avg_row_size);
    println!(
        "  Synaptic Density:   {:>12.2}%",
        stats.synaptic_density * 100.0
    );
    println!(
        "  Plasticity Index:   {:>12.2}%",
        stats.plasticity_index * 100.0
    );

    println!("\n✅ Table statistics collected\n");
    Ok(())
}
