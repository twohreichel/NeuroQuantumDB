//! Abstract Syntax Tree definitions for QSQL language
//!
//! This module defines the AST nodes for parsing QSQL queries with neuromorphic
//! and quantum-inspired extensions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Root AST node representing a complete QSQL query
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    CreateTable(CreateTableStatement),
    DropTable(DropTableStatement),
    // Neuromorphic extensions
    NeuroMatch(NeuroMatchStatement),
    SynapticOptimize(SynapticOptimizeStatement),
    // Quantum extensions
    QuantumSearch(QuantumSearchStatement),
    SuperpositionQuery(SuperpositionQueryStatement),
}

/// Standard SQL SELECT with neuromorphic and quantum extensions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectStatement {
    pub select_list: Vec<SelectItem>,
    pub from: Option<FromClause>,
    pub where_clause: Option<Expression>,
    pub group_by: Vec<Expression>,
    pub having: Option<Expression>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    // Neuromorphic extensions
    pub synaptic_weight: Option<f32>,
    pub plasticity_threshold: Option<f32>,
    // Quantum extensions
    pub quantum_parallel: bool,
    pub grover_iterations: Option<u32>,
}

/// Neuromorphic NEUROMATCH statement for brain-inspired pattern matching
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuroMatchStatement {
    pub target_table: String,
    pub pattern_expression: Expression,
    pub synaptic_weight: f32,
    pub learning_rate: Option<f32>,
    pub activation_threshold: Option<f32>,
    pub hebbian_strengthening: bool,
}

/// Quantum search statement using Grover's algorithm
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantumSearchStatement {
    pub target_table: String,
    pub search_expression: Expression,
    pub amplitude_amplification: bool,
    pub oracle_function: Option<String>,
    pub max_iterations: Option<u32>,
}

/// Superposition query for parallel quantum processing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuperpositionQueryStatement {
    pub parallel_queries: Vec<Statement>,
    pub coherence_maintenance: bool,
    pub entanglement_pairs: Vec<(String, String)>,
}

/// Synaptic optimization directive
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynapticOptimizeStatement {
    pub target_index: String,
    pub optimization_type: SynapticOptimizationType,
    pub learning_parameters: HashMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SynapticOptimizationType {
    HebbianLearning,
    SynapticPlasticity,
    PathwayStrengthening,
    NeuralPruning,
}

/// Standard SQL statements with neuromorphic extensions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertStatement {
    pub table_name: String,
    pub columns: Option<Vec<String>>,
    pub values: Vec<Vec<Expression>>,
    pub on_conflict: Option<ConflictResolution>,
    // Neuromorphic extensions
    pub synaptic_adaptation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStatement {
    pub table_name: String,
    pub assignments: Vec<Assignment>,
    pub where_clause: Option<Expression>,
    // Neuromorphic extensions
    pub pathway_reinforcement: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteStatement {
    pub table_name: String,
    pub where_clause: Option<Expression>,
    // Neuromorphic extensions
    pub synaptic_pruning: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTableStatement {
    pub table_name: String,
    pub columns: Vec<ColumnDefinition>,
    pub constraints: Vec<TableConstraint>,
    // Neuromorphic extensions
    pub synaptic_indexing: bool,
    pub plasticity_config: Option<PlasticityConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropTableStatement {
    pub table_name: String,
    pub if_exists: bool,
    // Neuromorphic extensions
    pub preserve_synaptic_patterns: bool,
}

/// FROM clause with quantum join support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FromClause {
    pub relations: Vec<TableReference>,
    pub joins: Vec<JoinClause>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub relation: TableReference,
    pub condition: Option<Expression>,
    // Quantum extensions
    pub quantum_entanglement: bool,
    pub superposition_join: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
    // Quantum extensions
    QuantumJoin,
    SuperpositionJoin,
    EntangledJoin,
}

/// Table reference with neuromorphic annotations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableReference {
    pub name: String,
    pub alias: Option<String>,
    pub synaptic_weight: Option<f32>,
    pub quantum_state: Option<QuantumState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuantumState {
    Superposition,
    Entangled(String), // Entangled with another table
    Collapsed,
}

/// Expression tree with neuromorphic and quantum operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    // Literals
    Literal(Literal),
    Identifier(String),

    // Standard operators
    BinaryOp {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    UnaryOp {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },

    // Function calls
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },

    // Neuromorphic operators
    SynapticMatch {
        pattern: Box<Expression>,
        weight: f32,
        threshold: Option<f32>,
    },

    HebbianStrength {
        source: Box<Expression>,
        target: Box<Expression>,
    },

    PlasticityAdaptation {
        base: Box<Expression>,
        adaptation_rate: f32,
    },

    // Quantum operators
    QuantumSuperposition {
        states: Vec<Expression>,
    },

    AmplitudeAmplification {
        target: Box<Expression>,
        amplification_factor: f32,
    },

    QuantumEntanglement {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // Subqueries
    Subquery(Box<Statement>),

    // Case expressions
    Case {
        operand: Option<Box<Expression>>,
        when_branches: Vec<(Expression, Expression)>,
        else_branch: Option<Box<Expression>>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Blob(Vec<u8>),
    // DNA-inspired literals
    DNASequence(String),   // ATGC sequence
    QuantumBit(bool, f64), // Value and probability amplitude
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,

    // Logical
    And,
    Or,

    // Pattern matching
    Like,
    NotLike,
    ILike,
    NotILike,

    // Set operations
    In,
    NotIn,

    // Neuromorphic operators
    SynapticStrength,
    PlasticityFlow,
    HebbianCorrelation,

    // Quantum operators
    QuantumCorrelation,
    SuperpositionMerge,
    EntanglementBond,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Plus,
    Minus,
    // Neuromorphic operators
    SynapticActivation,
    PlasticityDecay,
    // Quantum operators
    QuantumMeasurement,
    AmplitudeCollapse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectItem {
    pub expression: Expression,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByItem {
    pub expression: Expression,
    pub direction: OrderDirection,
    pub nulls_first: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderDirection {
    Ascending,
    Descending,
    // Neuromorphic ordering
    SynapticStrength,
    PlasticityGradient,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub column: String,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub constraints: Vec<ColumnConstraint>,
    // Neuromorphic extensions
    pub synaptic_properties: Option<SynapticProperties>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    // Standard types
    Boolean,
    TinyInt,
    SmallInt,
    Integer,
    BigInt,
    Real,
    Double,
    Char(Option<u32>),
    VarChar(Option<u32>),
    Text,
    Blob,
    Date,
    Time,
    Timestamp,

    // Neuromorphic types
    SynapticWeight,
    PlasticityMatrix,
    NeuralActivation,

    // Quantum types
    QuantumBit,
    SuperpositionState,
    EntanglementPair,

    // DNA-inspired types
    DNASequence,
    ProteinStructure,
    QuaternaryEncoding,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnConstraint {
    NotNull,
    Unique,
    PrimaryKey,
    ForeignKey {
        references_table: String,
        references_column: String,
    },
    Check(Expression),
    Default(Expression),
    // Neuromorphic constraints
    SynapticRange {
        min: f32,
        max: f32,
    },
    PlasticityBounds {
        threshold: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableConstraint {
    PrimaryKey(Vec<String>),
    ForeignKey {
        columns: Vec<String>,
        references_table: String,
        references_columns: Vec<String>,
    },
    Unique(Vec<String>),
    Check(Expression),
    // Neuromorphic constraints
    SynapticNetwork {
        nodes: Vec<String>,
        connections: Vec<(String, String, f32)>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynapticProperties {
    pub initial_weight: f32,
    pub learning_rate: f32,
    pub decay_factor: f32,
    pub activation_threshold: f32,
    pub plasticity_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlasticityConfig {
    pub global_learning_rate: f32,
    pub hebbian_enabled: bool,
    pub spike_timing_dependent: bool,
    pub homeostatic_scaling: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictResolution {
    Ignore,
    Replace,
    Update(Vec<Assignment>),
    // Neuromorphic resolution
    SynapticAdaptation,
}

// Display implementations for better debugging and logging
impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Select(_s) => write!(f, "SELECT"),
            Statement::NeuroMatch(n) => write!(f, "NEUROMATCH {}", n.target_table),
            Statement::QuantumSearch(q) => write!(f, "QUANTUM_SEARCH {}", q.target_table),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Literal(lit) => write!(f, "{:?}", lit),
            Expression::Identifier(id) => write!(f, "{}", id),
            Expression::BinaryOp {
                left,
                operator,
                right,
            } => {
                write!(f, "({} {:?} {})", left, operator, right)
            }
            Expression::SynapticMatch {
                pattern, weight, ..
            } => {
                write!(f, "SYNAPTIC_MATCH({}, {})", pattern, weight)
            }
            Expression::QuantumSuperposition { states } => {
                write!(f, "QUANTUM_SUPERPOSITION({})", states.len())
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuromorphic_ast_creation() {
        let neuromatch = NeuroMatchStatement {
            target_table: "users".to_string(),
            pattern_expression: Expression::BinaryOp {
                left: Box::new(Expression::Identifier("age".to_string())),
                operator: BinaryOperator::GreaterThan,
                right: Box::new(Expression::Literal(Literal::Integer(30))),
            },
            synaptic_weight: 0.8,
            learning_rate: Some(0.01),
            activation_threshold: Some(0.5),
            hebbian_strengthening: true,
        };

        assert_eq!(neuromatch.target_table, "users");
        assert_eq!(neuromatch.synaptic_weight, 0.8);
        assert!(neuromatch.hebbian_strengthening);
    }

    #[test]
    fn test_quantum_ast_creation() {
        let quantum_search = QuantumSearchStatement {
            target_table: "products".to_string(),
            search_expression: Expression::BinaryOp {
                left: Box::new(Expression::Identifier("price".to_string())),
                operator: BinaryOperator::LessThan,
                right: Box::new(Expression::Literal(Literal::Float(100.0))),
            },
            amplitude_amplification: true,
            oracle_function: Some("price_oracle".to_string()),
            max_iterations: Some(10),
        };

        assert_eq!(quantum_search.target_table, "products");
        assert!(quantum_search.amplitude_amplification);
        assert_eq!(quantum_search.max_iterations, Some(10));
    }

    #[test]
    fn test_synaptic_expression() {
        let synaptic_expr = Expression::SynapticMatch {
            pattern: Box::new(Expression::Identifier("user_behavior".to_string())),
            weight: 0.75,
            threshold: Some(0.6),
        };

        match synaptic_expr {
            Expression::SynapticMatch { weight, .. } => assert_eq!(weight, 0.75),
            _ => panic!("Expected SynapticMatch expression"),
        }
    }

    #[test]
    fn test_quantum_superposition() {
        let superposition = Expression::QuantumSuperposition {
            states: vec![
                Expression::Literal(Literal::Boolean(true)),
                Expression::Literal(Literal::Boolean(false)),
            ],
        };

        match superposition {
            Expression::QuantumSuperposition { states } => assert_eq!(states.len(), 2),
            _ => panic!("Expected QuantumSuperposition expression"),
        }
    }
}
