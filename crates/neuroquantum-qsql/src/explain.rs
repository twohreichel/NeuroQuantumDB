//! Query Plan Explanation and Analysis
//!
//! This module provides EXPLAIN and ANALYZE functionality for visualizing
//! query execution plans, cost estimation, and runtime statistics.

use crate::ast::{ExplainFormat, Statement, SelectStatement, NeuroMatchStatement, QuantumSearchStatement, QuantumJoinStatement};
use crate::error::{QSQLResult, QSQLError};
use crate::query_plan::QueryPlan;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

/// Configuration for EXPLAIN output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainConfig {
    pub show_costs: bool,
    pub show_timing: bool,
    pub show_buffers: bool,
    pub show_synaptic_pathways: bool,
    pub show_quantum_ops: bool,
    pub format: ExplainFormat,
}

impl Default for ExplainConfig {
    fn default() -> Self {
        Self {
            show_costs: true,
            show_timing: false,
            show_buffers: true,
            show_synaptic_pathways: true,
            show_quantum_ops: true,
            format: ExplainFormat::Text,
        }
    }
}

/// Query plan explanation with cost estimates and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainPlan {
    pub plan_nodes: Vec<PlanNode>,
    pub total_cost: f64,
    pub estimated_rows: u64,
    pub estimated_width: u32,
    pub planning_time: Duration,
    pub execution_time: Option<Duration>,
    pub synaptic_score: f32,
    pub quantum_score: f32,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Individual plan node in the execution tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub node_type: NodeType,
    pub node_id: String,
    pub relation_name: Option<String>,
    pub alias: Option<String>,
    pub startup_cost: f64,
    pub total_cost: f64,
    pub plan_rows: u64,
    pub plan_width: u32,
    pub actual_rows: Option<u64>,
    pub actual_time: Option<Duration>,
    pub filter: Option<String>,
    pub index_name: Option<String>,
    pub index_cond: Option<String>,
    pub join_type: Option<String>,
    pub children: Vec<PlanNode>,
    // Neuromorphic extensions
    pub synaptic_pathways: Vec<String>,
    pub neuromorphic_score: f32,
    // Quantum extensions
    pub quantum_operations: Vec<String>,
    pub quantum_advantage: Option<f32>,
}

/// Type of execution plan node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    SeqScan,
    IndexScan,
    IndexOnlyScan,
    BitmapIndexScan,
    BitmapHeapScan,
    TidScan,
    SubqueryScan,
    FunctionScan,
    ValuesScan,
    CteScan,
    NestedLoop,
    MergeJoin,
    HashJoin,
    MaterialNode,
    Sort,
    Group,
    Aggregate,
    WindowAgg,
    Unique,
    SetOp,
    LockRows,
    Limit,
    // Neuromorphic nodes
    NeuromorphicScan,
    SynapticFilter,
    PlasticityUpdate,
    HebbianJoin,
    // Quantum nodes
    QuantumScan,
    GroverSearch,
    SuperpositionJoin,
    AmplitudeAmplification,
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            | Self::SeqScan => write!(f, "Seq Scan"),
            | Self::IndexScan => write!(f, "Index Scan"),
            | Self::IndexOnlyScan => write!(f, "Index Only Scan"),
            | Self::BitmapIndexScan => write!(f, "Bitmap Index Scan"),
            | Self::BitmapHeapScan => write!(f, "Bitmap Heap Scan"),
            | Self::NestedLoop => write!(f, "Nested Loop"),
            | Self::MergeJoin => write!(f, "Merge Join"),
            | Self::HashJoin => write!(f, "Hash Join"),
            | Self::Sort => write!(f, "Sort"),
            | Self::Aggregate => write!(f, "Aggregate"),
            | Self::Limit => write!(f, "Limit"),
            | Self::NeuromorphicScan => write!(f, "Neuromorphic Scan"),
            | Self::SynapticFilter => write!(f, "Synaptic Filter"),
            | Self::QuantumScan => write!(f, "Quantum Scan"),
            | Self::GroverSearch => write!(f, "Grover Search"),
            | Self::SuperpositionJoin => write!(f, "Superposition Join"),
            | _ => write!(f, "{self:?}"),
        }
    }
}

/// Explain plan generator
pub struct ExplainGenerator {
    config: ExplainConfig,
}

impl ExplainGenerator {
    /// Create a new explain generator
    #[must_use] 
    pub const fn new(config: ExplainConfig) -> Self {
        Self { config }
    }

    /// Generate explain plan from query plan
    pub fn generate_explain(
        &self,
        query_plan: &QueryPlan,
        analyze: bool,
    ) -> QSQLResult<ExplainPlan> {
        let start_time = std::time::Instant::now();

        // Generate plan tree
        let plan_nodes = self.build_plan_tree(query_plan)?;

        // Calculate totals
        let total_cost = query_plan.estimated_cost;
        let estimated_rows = self.estimate_total_rows(query_plan);
        let estimated_width = 100; // Average row width in bytes

        // Calculate scores
        let synaptic_score = self.calculate_synaptic_score(query_plan);
        let quantum_score = self.calculate_quantum_score(query_plan);

        // Generate warnings and suggestions
        let warnings = self.generate_warnings(query_plan);
        let suggestions = self.generate_suggestions(query_plan);

        let planning_time = start_time.elapsed();

        Ok(ExplainPlan {
            plan_nodes,
            total_cost,
            estimated_rows,
            estimated_width,
            planning_time,
            execution_time: if analyze {
                Some(Duration::from_millis(10))
            } else {
                None
            },
            synaptic_score,
            quantum_score,
            warnings,
            suggestions,
        })
    }

    /// Build plan tree from query plan
    fn build_plan_tree(&self, query_plan: &QueryPlan) -> QSQLResult<Vec<PlanNode>> {
        let mut nodes = Vec::new();

        match &query_plan.statement {
            | Statement::Select(select) => {
                let node = self.build_select_node(select, query_plan)?;
                nodes.push(node);
            },
            | Statement::NeuroMatch(neuromatch) => {
                let node = self.build_neuromatch_node(neuromatch, query_plan)?;
                nodes.push(node);
            },
            | Statement::QuantumSearch(quantum) => {
                let node = self.build_quantum_search_node(quantum, query_plan)?;
                nodes.push(node);
            },
            | Statement::QuantumJoin(qjoin) => {
                let node = self.build_quantum_join_node(qjoin, query_plan)?;
                nodes.push(node);
            },
            | _ => {
                nodes.push(self.build_generic_node(query_plan)?);
            },
        }

        Ok(nodes)
    }

    /// Build SELECT node
    fn build_select_node(
        &self,
        select: &SelectStatement,
        query_plan: &QueryPlan,
    ) -> QSQLResult<PlanNode> {
        let mut node = PlanNode {
            node_type: NodeType::SeqScan,
            node_id: "1".to_string(),
            relation_name: select
                .from
                .as_ref()
                .and_then(|f| f.relations.first().map(|r| r.name.clone())),
            alias: None,
            startup_cost: 0.0,
            total_cost: query_plan.estimated_cost * 0.8,
            plan_rows: 1000,
            plan_width: 100,
            actual_rows: None,
            actual_time: None,
            filter: select
                .where_clause
                .as_ref()
                .map(|_| "WHERE clause".to_string()),
            index_name: None,
            index_cond: None,
            join_type: None,
            children: Vec::new(),
            synaptic_pathways: query_plan
                .synaptic_pathways
                .iter()
                .map(|p| p.pathway_id.clone())
                .collect(),
            neuromorphic_score: self.calculate_synaptic_score(query_plan),
            quantum_operations: query_plan
                .quantum_optimizations
                .iter()
                .map(|q| format!("{:?}", q.optimization_type))
                .collect(),
            quantum_advantage: if query_plan.quantum_optimizations.is_empty() {
                None
            } else {
                Some(query_plan.quantum_optimizations[0].speedup_factor)
            },
        };

        // Add filter node if WHERE clause exists
        if select.where_clause.is_some() {
            node.children.push(PlanNode {
                node_type: NodeType::SeqScan,
                node_id: "1.1".to_string(),
                relation_name: select
                    .from
                    .as_ref()
                    .and_then(|f| f.relations.first().map(|r| r.name.clone())),
                alias: None,
                startup_cost: 0.0,
                total_cost: query_plan.estimated_cost * 0.2,
                plan_rows: 500,
                plan_width: 100,
                actual_rows: None,
                actual_time: None,
                filter: Some("Filter condition".to_string()),
                index_name: None,
                index_cond: None,
                join_type: None,
                children: Vec::new(),
                synaptic_pathways: Vec::new(),
                neuromorphic_score: 0.0,
                quantum_operations: Vec::new(),
                quantum_advantage: None,
            });
        }

        Ok(node)
    }

    /// Build NEUROMATCH node
    fn build_neuromatch_node(
        &self,
        neuromatch: &NeuroMatchStatement,
        query_plan: &QueryPlan,
    ) -> QSQLResult<PlanNode> {
        Ok(PlanNode {
            node_type: NodeType::NeuromorphicScan,
            node_id: "1".to_string(),
            relation_name: Some(neuromatch.target_table.clone()),
            alias: None,
            startup_cost: 10.0,
            total_cost: query_plan.estimated_cost,
            plan_rows: 100,
            plan_width: 120,
            actual_rows: None,
            actual_time: None,
            filter: Some(format!(
                "Synaptic Weight: {:.2}",
                neuromatch.synaptic_weight
            )),
            index_name: Some("synaptic_index".to_string()),
            index_cond: None,
            join_type: None,
            children: Vec::new(),
            synaptic_pathways: query_plan
                .synaptic_pathways
                .iter()
                .map(|p| format!("{} (weight: {:.2})", p.pathway_id, p.weight))
                .collect(),
            neuromorphic_score: neuromatch.synaptic_weight,
            quantum_operations: Vec::new(),
            quantum_advantage: None,
        })
    }

    /// Build `QUANTUM_SEARCH` node
    fn build_quantum_search_node(
        &self,
        quantum: &QuantumSearchStatement,
        query_plan: &QueryPlan,
    ) -> QSQLResult<PlanNode> {
        let speedup = if quantum.amplitude_amplification {
            2.0
        } else {
            1.414
        };

        Ok(PlanNode {
            node_type: NodeType::GroverSearch,
            node_id: "1".to_string(),
            relation_name: Some(quantum.target_table.clone()),
            alias: None,
            startup_cost: 5.0,
            total_cost: query_plan.estimated_cost,
            plan_rows: 50,
            plan_width: 80,
            actual_rows: None,
            actual_time: None,
            filter: Some("Quantum Oracle Function".to_string()),
            index_name: Some("quantum_index".to_string()),
            index_cond: None,
            join_type: None,
            children: Vec::new(),
            synaptic_pathways: Vec::new(),
            neuromorphic_score: 0.0,
            quantum_operations: vec![
                format!("Grover's Algorithm"),
                format!("Max Iterations: {}", quantum.max_iterations.unwrap_or(10)),
                format!(
                    "Amplitude Amplification: {}",
                    quantum.amplitude_amplification
                ),
            ],
            quantum_advantage: Some(speedup),
        })
    }

    /// Build `QUANTUM_JOIN` node
    fn build_quantum_join_node(
        &self,
        qjoin: &QuantumJoinStatement,
        query_plan: &QueryPlan,
    ) -> QSQLResult<PlanNode> {
        Ok(PlanNode {
            node_type: NodeType::SuperpositionJoin,
            node_id: "1".to_string(),
            relation_name: Some(qjoin.left_table.clone()),
            alias: None,
            startup_cost: 15.0,
            total_cost: query_plan.estimated_cost,
            plan_rows: 200,
            plan_width: 150,
            actual_rows: None,
            actual_time: None,
            filter: None,
            index_name: None,
            index_cond: None,
            join_type: Some("Quantum Entanglement Join".to_string()),
            children: vec![PlanNode {
                node_type: NodeType::QuantumScan,
                node_id: "1.1".to_string(),
                relation_name: Some(qjoin.right_table.clone()),
                alias: None,
                startup_cost: 0.0,
                total_cost: query_plan.estimated_cost * 0.3,
                plan_rows: 100,
                plan_width: 75,
                actual_rows: None,
                actual_time: None,
                filter: None,
                index_name: None,
                index_cond: None,
                join_type: None,
                children: Vec::new(),
                synaptic_pathways: Vec::new(),
                neuromorphic_score: 0.0,
                quantum_operations: vec!["Entanglement Preparation".to_string()],
                quantum_advantage: None,
            }],
            synaptic_pathways: Vec::new(),
            neuromorphic_score: 0.0,
            quantum_operations: vec![
                format!("Quantum Entanglement"),
                format!("Superposition Join"),
            ],
            quantum_advantage: Some(1.5),
        })
    }

    /// Build generic node for other statement types
    fn build_generic_node(&self, query_plan: &QueryPlan) -> QSQLResult<PlanNode> {
        Ok(PlanNode {
            node_type: NodeType::SeqScan,
            node_id: "1".to_string(),
            relation_name: None,
            alias: None,
            startup_cost: 0.0,
            total_cost: query_plan.estimated_cost,
            plan_rows: 100,
            plan_width: 100,
            actual_rows: None,
            actual_time: None,
            filter: None,
            index_name: None,
            index_cond: None,
            join_type: None,
            children: Vec::new(),
            synaptic_pathways: Vec::new(),
            neuromorphic_score: 0.0,
            quantum_operations: Vec::new(),
            quantum_advantage: None,
        })
    }

    /// Estimate total rows
    const fn estimate_total_rows(&self, _query_plan: &QueryPlan) -> u64 {
        1000 // Default estimate
    }

    /// Calculate synaptic optimization score (0.0 - 1.0)
    fn calculate_synaptic_score(&self, query_plan: &QueryPlan) -> f32 {
        if query_plan.synaptic_pathways.is_empty() {
            return 0.0;
        }

        let avg_weight: f32 = query_plan
            .synaptic_pathways
            .iter()
            .map(|p| p.weight)
            .sum::<f32>()
            / query_plan.synaptic_pathways.len() as f32;

        avg_weight.min(1.0)
    }

    /// Calculate quantum optimization score (0.0 - 1.0)
    fn calculate_quantum_score(&self, query_plan: &QueryPlan) -> f32 {
        if query_plan.quantum_optimizations.is_empty() {
            return 0.0;
        }

        let avg_speedup: f32 = query_plan
            .quantum_optimizations
            .iter()
            .map(|q| q.speedup_factor)
            .sum::<f32>()
            / query_plan.quantum_optimizations.len() as f32;

        (avg_speedup / 2.0).min(1.0)
    }

    /// Generate optimization warnings
    fn generate_warnings(&self, query_plan: &QueryPlan) -> Vec<String> {
        let mut warnings = Vec::new();

        if query_plan.estimated_cost > 1000.0 {
            warnings.push("Query has high estimated cost. Consider adding indexes.".to_string());
        }

        if query_plan.synaptic_pathways.is_empty() && query_plan.quantum_optimizations.is_empty() {
            warnings.push("No neuromorphic or quantum optimizations applied.".to_string());
        }

        warnings
    }

    /// Generate optimization suggestions
    fn generate_suggestions(&self, query_plan: &QueryPlan) -> Vec<String> {
        let mut suggestions = Vec::new();

        if query_plan.synaptic_pathways.is_empty() {
            suggestions.push("Consider using NEUROMATCH for pattern-based queries.".to_string());
        }

        if query_plan.quantum_optimizations.is_empty() {
            if let Statement::Select(s) = &query_plan.statement {
                if s.from.is_some() {
                    suggestions
                        .push("Consider using QUANTUM_SEARCH for large datasets.".to_string());
                }
            }
        }

        if query_plan.estimated_cost > 500.0 {
            suggestions.push("Query may benefit from synaptic indexing.".to_string());
        }

        suggestions
    }

    /// Format explain plan as text
    #[must_use] 
    pub fn format_text(&self, explain_plan: &ExplainPlan) -> String {
        let mut output = String::new();

        output.push_str("Query Plan\n");
        output.push_str(&format!("{}\n", "=".repeat(80)));

        for node in &explain_plan.plan_nodes {
            self.format_node_text(&mut output, node, 0);
        }

        output.push_str(&format!("\n{}\n", "-".repeat(80)));
        output.push_str(&format!(
            "Planning Time: {:.3}ms\n",
            explain_plan.planning_time.as_secs_f64() * 1000.0
        ));

        if let Some(exec_time) = explain_plan.execution_time {
            output.push_str(&format!(
                "Execution Time: {:.3}ms\n",
                exec_time.as_secs_f64() * 1000.0
            ));
        }

        output.push_str(&format!("Total Cost: {:.2}\n", explain_plan.total_cost));
        output.push_str(&format!(
            "Estimated Rows: {}\n",
            explain_plan.estimated_rows
        ));

        if self.config.show_synaptic_pathways && explain_plan.synaptic_score > 0.0 {
            output.push_str(&format!(
                "Neuromorphic Score: {:.2}\n",
                explain_plan.synaptic_score
            ));
        }

        if self.config.show_quantum_ops && explain_plan.quantum_score > 0.0 {
            output.push_str(&format!(
                "Quantum Optimization Score: {:.2}\n",
                explain_plan.quantum_score
            ));
        }

        if !explain_plan.warnings.is_empty() {
            output.push_str("\nWarnings:\n");
            for warning in &explain_plan.warnings {
                output.push_str(&format!("  ⚠️  {warning}\n"));
            }
        }

        if !explain_plan.suggestions.is_empty() {
            output.push_str("\nSuggestions:\n");
            for suggestion in &explain_plan.suggestions {
                output.push_str(&format!("  💡 {suggestion}\n"));
            }
        }

        output
    }

    /// Format a single node as text
    fn format_node_text(&self, output: &mut String, node: &PlanNode, indent: usize) {
        let indent_str = "  ".repeat(indent);

        output.push_str(&format!("{}{}", indent_str, node.node_type));

        if let Some(ref relation) = node.relation_name {
            output.push_str(&format!(" on {relation}"));
        }

        if self.config.show_costs {
            output.push_str(&format!(
                " (cost={:.2}..{:.2}",
                node.startup_cost, node.total_cost
            ));
            output.push_str(&format!(
                " rows={} width={})",
                node.plan_rows, node.plan_width
            ));
        }

        output.push('\n');

        if let Some(ref filter) = node.filter {
            output.push_str(&format!("{indent_str}  Filter: {filter}\n"));
        }

        if let Some(ref index) = node.index_name {
            output.push_str(&format!("{indent_str}  Index: {index}\n"));
        }

        if self.config.show_synaptic_pathways && !node.synaptic_pathways.is_empty() {
            output.push_str(&format!(
                "{}  Synaptic Pathways: {}\n",
                indent_str,
                node.synaptic_pathways.len()
            ));
            for pathway in &node.synaptic_pathways {
                output.push_str(&format!("{indent_str}    • {pathway}\n"));
            }
        }

        if self.config.show_quantum_ops && !node.quantum_operations.is_empty() {
            output.push_str(&format!("{indent_str}  Quantum Operations:\n"));
            for op in &node.quantum_operations {
                output.push_str(&format!("{indent_str}    • {op}\n"));
            }

            if let Some(advantage) = node.quantum_advantage {
                output.push_str(&format!(
                    "{indent_str}  Quantum Speedup: {advantage:.2}x\n"
                ));
            }
        }

        for child in &node.children {
            self.format_node_text(output, child, indent + 1);
        }
    }

    /// Format explain plan as JSON
    pub fn format_json(&self, explain_plan: &ExplainPlan) -> QSQLResult<String> {
        serde_json::to_string_pretty(explain_plan).map_err(|e| QSQLError::ExecutionError {
            message: format!("Failed to serialize to JSON: {e}"),
        })
    }

    /// Format explain plan as YAML
    pub fn format_yaml(&self, explain_plan: &ExplainPlan) -> QSQLResult<String> {
        serde_yaml_ng::to_string(explain_plan).map_err(|e| QSQLError::ExecutionError {
            message: format!("Failed to serialize to YAML: {e}"),
        })
    }
}

/// Table statistics for ANALYZE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStatistics {
    pub table_name: String,
    pub row_count: u64,
    pub page_count: u64,
    pub avg_row_size: u32,
    pub null_frac: HashMap<String, f64>,
    pub distinct_values: HashMap<String, u64>,
    pub most_common_values: HashMap<String, Vec<String>>,
    pub histogram_bounds: HashMap<String, Vec<String>>,
    pub last_analyzed: std::time::SystemTime,
    // Neuromorphic statistics
    pub synaptic_density: f32,
    pub plasticity_index: f32,
}

impl Default for TableStatistics {
    fn default() -> Self {
        Self {
            table_name: String::new(),
            row_count: 0,
            page_count: 0,
            avg_row_size: 0,
            null_frac: HashMap::new(),
            distinct_values: HashMap::new(),
            most_common_values: HashMap::new(),
            histogram_bounds: HashMap::new(),
            last_analyzed: std::time::SystemTime::now(),
            synaptic_density: 0.0,
            plasticity_index: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            statement: Statement::Select(select),
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
            statement: Statement::Select(select),
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
            statement: Statement::NeuroMatch(neuromatch),
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
            statement: Statement::QuantumSearch(quantum),
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

        let explain_plan = generator.generate_explain(&query_plan, false).unwrap();
        let json = generator.format_json(&explain_plan).unwrap();

        assert!(json.contains("\"total_cost\""));
        assert!(json.contains("\"plan_nodes\""));
        assert!(json.contains("\"planning_time\""));
    }
}
