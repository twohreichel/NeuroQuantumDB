//! Abstract Syntax Tree definitions for QSQL language
//!
//! This module defines the AST nodes for parsing QSQL queries with neuromorphic
//! and quantum-inspired extensions.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Learning algorithms for pattern learning
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningAlgorithm {
    HebbianLearning,
    STDP, // Spike-Timing Dependent Plasticity
    BackPropagation,
    ReinforcementLearning,
    UnsupervisedClustering,
}

/// Learning rules for weight adaptation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningRule {
    Hebbian,
    AntiHebbian,
    OjasRule,
    BCM, // Bienenstock-Cooper-Munro
    STDP,
}

/// Neuromorphic extension statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NeuroExtension {
    NeuroMatch {
        field: String,
        pattern: String,
        synaptic_weight: f32,
        plasticity_threshold: Option<f32>,
    },
    QuantumJoin {
        left_table: String,
        right_table: String,
        entanglement_condition: String,
        superposition_fields: Vec<String>,
    },
    LearnPattern {
        pattern_name: String,
        training_data: String,
        learning_algorithm: LearningAlgorithm,
    },
    AdaptWeights {
        rule: LearningRule,
        learning_rate: f32,
    },
}

/// Root AST node representing a complete QSQL query
/// Note: `SelectStatement` is intentionally not boxed to avoid indirection overhead
/// for the most common query type. The size trade-off is acceptable for performance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    CreateTable(CreateTableStatement),
    DropTable(DropTableStatement),
    AlterTable(AlterTableStatement),
    CreateIndex(CreateIndexStatement),
    DropIndex(DropIndexStatement),
    TruncateTable(TruncateTableStatement),
    CompressTable(CompressTableStatement),
    // Neuromorphic extensions
    NeuroMatch(NeuroMatchStatement),
    SynapticOptimize(SynapticOptimizeStatement),
    LearnPattern(LearnPatternStatement),
    AdaptWeights(AdaptWeightsStatement),
    // Quantum extensions
    QuantumSearch(QuantumSearchStatement),
    SuperpositionQuery(SuperpositionQueryStatement),
    QuantumJoin(QuantumJoinStatement),
    // Query Analysis
    Explain(ExplainStatement),
    Analyze(AnalyzeStatement),
    // Transaction Control
    BeginTransaction(BeginTransactionStatement),
    Commit(CommitStatement),
    Rollback(RollbackStatement),
    Savepoint(SavepointStatement),
    RollbackToSavepoint(RollbackToSavepointStatement),
    ReleaseSavepoint(ReleaseSavepointStatement),
    // Prepared Statements
    Prepare(PrepareStatement),
    Execute(ExecuteStatement),
    Deallocate(DeallocateStatement),
}

/// Common Table Expression (CTE) for WITH clauses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonTableExpression {
    pub name: String,
    pub query: Box<SelectStatement>,
    pub columns: Option<Vec<String>>, // Optional column names
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
    /// NEUROMATCH clause for neuromorphic pattern matching within SELECT
    /// Syntax: SELECT ... FROM table NEUROMATCH('pattern') [WHERE ...]
    pub neuromatch_clause: Option<NeuroMatchClause>,
    // Quantum extensions
    pub quantum_parallel: bool,
    pub grover_iterations: Option<u32>,
    // WITH clause for CTEs
    pub with_clause: Option<WithClause>,
    /// UNION clause for compound queries (UNION / UNION ALL)
    /// Used in recursive CTEs: `anchor_query` UNION ALL `recursive_query`
    pub union_clause: Option<UnionClause>,
}

/// UNION clause for compound queries
/// Combines two SELECT statements with UNION or UNION ALL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnionClause {
    /// The type of set operation (UNION or UNION ALL)
    pub union_type: UnionType,
    /// The SELECT statement to combine with
    pub select: Box<SelectStatement>,
}

/// Type of UNION operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnionType {
    /// UNION - removes duplicate rows
    Union,
    /// UNION ALL - keeps all rows including duplicates
    UnionAll,
}

/// NEUROMATCH clause for use within SELECT statements
/// Enables brain-inspired pattern matching directly in queries
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuroMatchClause {
    /// The pattern to match against (can be a string literal or expression)
    pub pattern: Expression,
    /// Optional field to match against (if not specified, matches all fields)
    pub field: Option<String>,
    /// Synaptic weight threshold for matching (default: 0.5)
    pub synaptic_weight: f32,
    /// Whether to apply Hebbian learning to strengthen matched patterns
    pub hebbian_learning: bool,
}

/// WITH clause containing one or more CTEs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithClause {
    pub recursive: bool,
    pub ctes: Vec<CommonTableExpression>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub plasticity_adaptation: Option<f32>,
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
    pub if_not_exists: bool,
    pub columns: Vec<ColumnDefinition>,
    pub constraints: Vec<TableConstraint>,
    // Neuromorphic extensions
    pub synaptic_indexing: bool,
    pub plasticity_config: Option<PlasticityConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropTableStatement {
    pub table_name: String,
    pub if_exists: bool,
    // Neuromorphic extensions
    pub preserve_synaptic_patterns: bool,
}

/// ALTER TABLE statement for modifying table structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterTableStatement {
    pub table_name: String,
    pub operation: AlterTableOperation,
    /// Execute operation concurrently without blocking reads/writes
    pub concurrently: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterTableOperation {
    AddColumn {
        column: ColumnDefinition,
    },
    DropColumn {
        column_name: String,
    },
    RenameColumn {
        old_name: String,
        new_name: String,
    },
    ModifyColumn {
        column_name: String,
        new_data_type: DataType,
        /// USING clause for type conversion
        using_expression: Option<String>,
    },
}

/// CREATE INDEX statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateIndexStatement {
    pub index_name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub if_not_exists: bool,
    /// Create index concurrently without blocking writes
    pub concurrently: bool,
}

/// DROP INDEX statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropIndexStatement {
    pub index_name: String,
    pub if_exists: bool,
    /// Drop index concurrently without blocking writes
    pub concurrently: bool,
}

/// CASCADE/RESTRICT behavior for TRUNCATE TABLE
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TruncateBehavior {
    /// Default behavior - error if foreign key constraints exist
    #[default]
    Restrict,
    /// CASCADE - also truncate all tables that have foreign key references
    Cascade,
}

/// TRUNCATE TABLE statement
/// Quickly removes all rows from a table without logging individual row deletions.
/// Syntax: TRUNCATE TABLE `table_name` [CASCADE | RESTRICT]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncateTableStatement {
    /// The name of the table to truncate
    pub table_name: String,
    /// CASCADE or RESTRICT behavior for foreign key constraints
    pub behavior: TruncateBehavior,
    /// RESTART IDENTITY - reset identity/serial columns to initial values
    pub restart_identity: bool,
}

/// Compression algorithm for COMPRESS TABLE statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// DNA-based compression using genetic encoding
    DNA,
}

/// COMPRESS TABLE statement
/// Applies compression algorithm to table data
/// Syntax: COMPRESS TABLE `table_name` USING `compression_algorithm`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressTableStatement {
    /// The name of the table to compress
    pub table_name: String,
    /// The compression algorithm to use
    pub algorithm: CompressionAlgorithm,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
/// Can be either a regular table or a derived table (subquery)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableReference {
    /// Table name (for regular tables). Empty string for derived tables.
    pub name: String,
    /// Alias for the table or derived table. Required for derived tables.
    pub alias: Option<String>,
    pub synaptic_weight: Option<f32>,
    pub quantum_state: Option<QuantumState>,
    /// Subquery for derived tables (when this is Some, this is a derived table)
    /// Syntax: (SELECT ...) AS alias
    pub subquery: Option<Box<SelectStatement>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    // Subqueries
    Subquery(Box<Statement>),

    // Neuromorphic expressions
    NeuroPattern {
        pattern: String,
        similarity_threshold: f32,
    },
    SynapticActivation {
        source: Box<Expression>,
        weight: f32,
    },
    SynapticMatch {
        pattern: Box<Expression>,
        weight: f32,
        threshold: Option<f32>,
    },
    PlasticityFunction {
        input: Box<Expression>,
        learning_rate: f32,
    },

    // Quantum expressions
    QuantumSuperposition {
        states: Vec<Expression>,
    },
    QuantumMeasurement {
        target: Box<Expression>,
        basis: String,
    },
    AmplitudeAmplification {
        target: Box<Expression>,
        oracle: String,
    },

    // IN list expression for WHERE column IN (value1, value2, ...)
    InList {
        expr: Box<Expression>,
        list: Vec<Expression>,
        negated: bool, // true for NOT IN
    },

    // IN subquery expression for WHERE column IN (SELECT ...)
    InSubquery {
        expr: Box<Expression>,
        subquery: Box<SelectStatement>,
        negated: bool, // true for NOT IN
    },

    // EXISTS subquery expression for WHERE EXISTS (SELECT ...)
    Exists {
        subquery: Box<SelectStatement>,
        negated: bool, // true for NOT EXISTS
    },

    // Scalar subquery expression - returns a single value
    // Used in SELECT list: SELECT name, (SELECT AVG(age) FROM users) AS avg_age FROM users
    // Used in WHERE clause: WHERE age > (SELECT AVG(age) FROM users)
    ScalarSubquery {
        subquery: Box<SelectStatement>,
    },

    // CASE expression for conditional logic
    // CASE WHEN condition1 THEN result1 WHEN condition2 THEN result2 ... ELSE else_result END
    Case {
        /// List of (condition, result) pairs for WHEN clauses
        when_clauses: Vec<(Box<Expression>, Box<Expression>)>,
        /// Optional ELSE result (if None, returns NULL when no condition matches)
        else_result: Option<Box<Expression>>,
    },

    // EXTRACT expression for date/time parts
    // EXTRACT(field FROM source)
    Extract {
        /// The field to extract (e.g., YEAR, MONTH, DAY, etc.)
        field: String,
        /// The source expression (date/time value)
        source: Box<Expression>,
    },

    // IS NULL / IS NOT NULL expression
    IsNull {
        expr: Box<Expression>,
        negated: bool, // true for IS NOT NULL
    },

    // Window function expression
    // e.g., ROW_NUMBER() OVER (PARTITION BY col1 ORDER BY col2)
    WindowFunction {
        /// The window function (`ROW_NUMBER`, RANK, `DENSE_RANK`, LAG, LEAD, etc.)
        function: WindowFunctionType,
        /// Arguments for the function (e.g., column name for LAG/LEAD)
        args: Vec<Expression>,
        /// The OVER clause specification
        over_clause: WindowSpec,
    },

    // Parameter placeholder for prepared statements
    // Supports positional ($1, $2) and named (:name) parameters
    Parameter(ParameterRef),

    // DEFAULT keyword for INSERT statements
    // Used when inserting a row with a column that should use its default value
    // Syntax: INSERT INTO table (col1, col2) VALUES ('value', DEFAULT)
    Default,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    DNA(String),
    QuantumBit(bool, f64), // state, amplitude
}

/// Window function types for SQL window functions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowFunctionType {
    /// `ROW_NUMBER()` - assigns sequential row numbers
    RowNumber,
    /// `RANK()` - assigns rank with gaps for ties
    Rank,
    /// `DENSE_RANK()` - assigns rank without gaps for ties
    DenseRank,
    /// LAG(column, offset, default) - accesses previous row's value
    Lag,
    /// LEAD(column, offset, default) - accesses next row's value
    Lead,
    /// NTILE(n) - distributes rows into n buckets
    Ntile,
    /// `FIRST_VALUE(column)` - returns first value in the window
    FirstValue,
    /// `LAST_VALUE(column)` - returns last value in the window
    LastValue,
    /// `NTH_VALUE(column`, n) - returns nth value in the window
    NthValue,
    // --- Phase 2: Aggregate Window Functions ---
    /// SUM(column) OVER () - running/partition sum
    Sum,
    /// AVG(column) OVER () - running/partition average
    Avg,
    /// COUNT(*|column) OVER () - running/partition count
    Count,
    /// MIN(column) OVER () - running/partition minimum
    Min,
    /// MAX(column) OVER () - running/partition maximum
    Max,
}

/// Window specification for OVER clause
/// e.g., OVER (PARTITION BY col1 ORDER BY col2 DESC)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WindowSpec {
    /// PARTITION BY columns - divides rows into groups
    pub partition_by: Vec<Expression>,
    /// ORDER BY columns within each partition
    pub order_by: Vec<OrderByItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    // String operations
    Like,
    NotLike,

    // Set operations
    In,
    NotIn,

    // Neuromorphic operators
    SynapticSimilarity,
    PlasticityUpdate,
    HebbianStrengthening,

    // Quantum operators
    QuantumEntanglement,
    SuperpositionCollapse,
    AmplitudeInterference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Minus,
    Plus,
    // Neuromorphic
    ActivationFunction,
    SynapticNormalization,
    // Quantum
    QuantumMeasure,
    PhaseFactor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectItem {
    Wildcard,
    Expression {
        expr: Expression,
        alias: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByItem {
    pub expression: Expression,
    pub ascending: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    // Standard SQL types
    Integer,
    BigInt,
    SmallInt,
    Real,
    Double,
    Decimal(u8, u8),      // precision, scale
    VarChar(Option<u32>), // Variable length with optional max length
    Varchar(u32),
    Char(u32),
    Text,
    Boolean,
    Date,
    Time,
    Timestamp,
    Blob,

    // Auto-increment types (PostgreSQL-style)
    /// SERIAL - auto-incrementing 32-bit integer (1 to 2,147,483,647)
    Serial,
    /// BIGSERIAL - auto-incrementing 64-bit integer (1 to 9,223,372,036,854,775,807)
    BigSerial,
    /// SMALLSERIAL - auto-incrementing 16-bit integer (1 to 32,767)
    SmallSerial,

    // Neuromorphic types
    DNASequence,
    SynapticWeight,
    NeuralPattern,
    PlasticityMatrix,

    // Quantum types
    QuantumBit,
    QuantumRegister(u32), // number of qubits
    SuperpositionState,
    EntanglementPair,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnConstraint {
    NotNull,
    Unique,
    PrimaryKey,
    ForeignKey {
        table: String,
        column: String,
    },
    Check(Expression),
    Default(Expression),
    /// `AUTO_INCREMENT` constraint (MySQL-style)
    /// Automatically generates sequential unique values for the column
    AutoIncrement,
    /// GENERATED AS IDENTITY (SQL:2003 standard)
    /// Provides ALWAYS or BY DEFAULT identity generation
    Identity {
        /// If true: GENERATED ALWAYS AS IDENTITY (cannot override)
        /// If false: GENERATED BY DEFAULT AS IDENTITY (can override)
        always: bool,
        /// Optional start value (default: 1)
        start: Option<i64>,
        /// Optional increment value (default: 1)
        increment: Option<i64>,
    },
    // Neuromorphic constraints
    SynapticRange {
        min: f32,
        max: f32,
    },
    PlasticityThreshold(f32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableConstraint {
    PrimaryKey(Vec<String>),
    ForeignKey {
        columns: Vec<String>,
        referenced_table: String,
        referenced_columns: Vec<String>,
    },
    Unique(Vec<String>),
    Check(Expression),
    // Neuromorphic constraints
    SynapticCoherence {
        columns: Vec<String>,
        threshold: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynapticProperties {
    pub weight_range: (f32, f32),
    pub plasticity_enabled: bool,
    pub learning_rate: f32,
    pub decay_factor: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlasticityConfig {
    pub global_learning_rate: f32,
    pub hebbian_strengthening: bool,
    pub synaptic_pruning: bool,
    pub adaptation_threshold: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictResolution {
    Ignore,
    Replace,
    Update(Vec<Assignment>),
    // Neuromorphic conflict resolution
    SynapticAdaptation {
        learning_rate: f32,
        adaptation_strategy: String,
    },
}

/// LEARN PATTERN statement for ML integration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearnPatternStatement {
    pub target_table: String,
    pub pattern_expression: Option<Expression>,
    pub learning_rate: Option<f64>,
    pub epochs: Option<u64>,
    pub algorithm: Option<String>,
}

/// ADAPT `SYNAPTIC_WEIGHTS` statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdaptWeightsStatement {
    pub target_table: String,
    pub weight_expression: Option<Expression>,
    pub plasticity_threshold: Option<f64>,
    pub hebbian_strengthening: bool,
}

/// `QUANTUM_JOIN` statement for entangled table operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantumJoinStatement {
    pub left_table: String,
    pub right_table: String,
    pub on_condition: Option<Expression>,
    pub using_columns: Vec<String>,
    pub quantum_state: Option<String>,
}

/// EXPLAIN statement for query plan visualization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplainStatement {
    pub statement: Box<Statement>,
    pub analyze: bool,         // If true, execute and show actual statistics
    pub verbose: bool,         // Show detailed information
    pub format: ExplainFormat, // Output format
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplainFormat {
    Text,
    Json,
    Yaml,
    Xml,
}

/// ANALYZE statement for collecting table statistics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeStatement {
    pub table_name: String,
    pub columns: Option<Vec<String>>, // Specific columns to analyze, None = all
    pub sample_size: Option<u64>,     // Number of rows to sample
}

/// BEGIN or START TRANSACTION statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginTransactionStatement {
    /// Optional isolation level (if not specified, use default)
    pub isolation_level: Option<String>,
}

/// COMMIT statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitStatement {}

/// ROLLBACK statement (without savepoint)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackStatement {}

/// SAVEPOINT statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavepointStatement {
    pub name: String,
}

/// ROLLBACK TO SAVEPOINT statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackToSavepointStatement {
    pub name: String,
}

/// RELEASE SAVEPOINT statement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSavepointStatement {
    pub name: String,
}

/// PREPARE statement for creating prepared statements
/// Syntax: PREPARE name AS query
/// Example: PREPARE `get_user` AS SELECT * FROM users WHERE id = $1
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareStatement {
    /// Name of the prepared statement
    pub name: String,
    /// The SQL statement to prepare (with parameter placeholders)
    pub statement: Box<Statement>,
    /// Parameter definitions (optional, for type hints)
    /// Maps parameter name/index to expected data type
    pub parameter_types: Option<Vec<DataType>>,
}

/// EXECUTE statement for running prepared statements
/// Syntax: EXECUTE name [(param1, param2, ...)]
/// Example: EXECUTE `get_user(42)`
/// Named parameters: EXECUTE `find_users(age` := 25, pattern := 'John%')
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteStatement {
    /// Name of the prepared statement to execute
    pub name: String,
    /// Positional parameter values
    pub parameters: Vec<Expression>,
    /// Named parameter values (for :name style parameters)
    pub named_parameters: HashMap<String, Expression>,
}

/// DEALLOCATE statement for removing prepared statements
/// Syntax: DEALLOCATE name | DEALLOCATE ALL
/// Example: DEALLOCATE `get_user`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeallocateStatement {
    /// Name of the prepared statement to deallocate (None for ALL)
    pub name: Option<String>,
}

/// Parameter placeholder expression for prepared statements
/// Supports both positional ($1, $2) and named (:name) parameters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParameterRef {
    /// Positional parameter ($1, $2, etc.) - 1-indexed
    Positional(u32),
    /// Named parameter (:name)
    Named(String),
}

// Display implementations for better debugging and logging
impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            | Self::Select(_s) => write!(f, "SELECT"),
            | Self::CreateTable(ct) => write!(f, "CREATE TABLE {}", ct.table_name),
            | Self::DropTable(dt) => write!(f, "DROP TABLE {}", dt.table_name),
            | Self::AlterTable(at) => write!(f, "ALTER TABLE {}", at.table_name),
            | Self::CreateIndex(ci) => write!(f, "CREATE INDEX {}", ci.index_name),
            | Self::DropIndex(di) => write!(f, "DROP INDEX {}", di.index_name),
            | Self::TruncateTable(tt) => write!(f, "TRUNCATE TABLE {}", tt.table_name),
            | Self::NeuroMatch(n) => write!(f, "NEUROMATCH {}", n.target_table),
            | Self::QuantumSearch(q) => write!(f, "QUANTUM_SEARCH {}", q.target_table),
            | Self::Explain(e) => {
                write!(f, "EXPLAIN {}", if e.analyze { "ANALYZE" } else { "" })
            },
            | Self::Analyze(a) => write!(f, "ANALYZE {}", a.table_name),
            | Self::BeginTransaction(_) => write!(f, "BEGIN TRANSACTION"),
            | Self::Commit(_) => write!(f, "COMMIT"),
            | Self::Rollback(_) => write!(f, "ROLLBACK"),
            | Self::Savepoint(s) => write!(f, "SAVEPOINT {}", s.name),
            | Self::RollbackToSavepoint(s) => write!(f, "ROLLBACK TO SAVEPOINT {}", s.name),
            | Self::ReleaseSavepoint(s) => write!(f, "RELEASE SAVEPOINT {}", s.name),
            | Self::Prepare(p) => write!(f, "PREPARE {}", p.name),
            | Self::Execute(e) => write!(f, "EXECUTE {}", e.name),
            | Self::Deallocate(d) => match &d.name {
                | Some(name) => write!(f, "DEALLOCATE {name}"),
                | None => write!(f, "DEALLOCATE ALL"),
            },
            | _ => write!(f, "{self:?}"),
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            | Self::Literal(lit) => write!(f, "{lit:?}"),
            | Self::Identifier(id) => write!(f, "{id}"),
            | Self::BinaryOp {
                left,
                operator,
                right,
            } => {
                write!(f, "({left} {operator:?} {right})")
            },
            | Self::SynapticMatch {
                pattern, weight, ..
            } => {
                write!(f, "SYNAPTIC_MATCH({pattern}, {weight})")
            },
            | Self::QuantumSuperposition { states } => {
                write!(f, "QUANTUM_SUPERPOSITION({})", states.len())
            },
            | _ => write!(f, "{self:?}"),
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
            | Expression::SynapticMatch { weight, .. } => assert_eq!(weight, 0.75),
            | _ => panic!("Expected SynapticMatch expression"),
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
            | Expression::QuantumSuperposition { states } => assert_eq!(states.len(), 2),
            | _ => panic!("Expected QuantumSuperposition expression"),
        }
    }
}
