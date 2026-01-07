use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use utoipa::ToSchema;
use validator::Validate;

/// API-specific error types for NeuroQuantumDB REST interface
#[derive(Error, Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ApiError {
    #[error("Authentication failed: {0}")]
    Unauthorized(String),

    #[error("Access forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error: {message}")]
    InternalServerError { message: String },

    #[error("Invalid query: {details}")]
    InvalidQuery { details: String },

    #[error("Quantum operation failed: {operation} - {reason}")]
    QuantumOperationFailed { operation: String, reason: String },

    #[error("DNA compression error: {reason}")]
    CompressionError { reason: String },

    #[error("Rate limit exceeded: {limit} requests per {window}")]
    RateLimitExceeded { limit: u32, window: String },

    #[error("Validation error: {field} - {message}")]
    ValidationError { field: String, message: String },

    #[error("Quantum-resistant encryption error: {details}")]
    EncryptionError { details: String },

    #[error("Neural network training error: {details}")]
    NeuralNetworkError { details: String },

    #[error("Table operation error: {operation} - {details}")]
    TableError { operation: String, details: String },

    #[error("Connection pool error: {details}")]
    ConnectionPoolError { details: String },

    #[error("Circuit breaker open: {service}")]
    CircuitBreakerOpen { service: String },

    #[error("Service unavailable: {service} - {reason}")]
    ServiceUnavailable { service: String, reason: String },

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Standard API response wrapper
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub metadata: ResponseMetadata,
}

/// Response metadata for tracking and debugging
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResponseMetadata {
    pub response_time_ms: f64,
    pub timestamp: String,
    pub request_id: String,
    pub message: String,
    pub version: String,
}

impl ResponseMetadata {
    pub fn new(duration: std::time::Duration, message: &str) -> Self {
        Self {
            response_time_ms: duration.as_secs_f64() * 1000.0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id: uuid::Uuid::new_v4().to_string(),
            message: message.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, metadata: ResponseMetadata) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata,
        }
    }

    pub fn error(error: ApiError, metadata: ResponseMetadata) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            metadata,
        }
    }
}

// Data Transfer Objects for API operations

/// Table schema definition
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, Validate)]
pub struct TableSchema {
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub indexes: Option<Vec<IndexDefinition>>,
    pub constraints: Option<Vec<ConstraintDefinition>>,
    pub neuromorphic_config: Option<NeuromorphicConfig>,
    pub quantum_config: Option<QuantumConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, Validate)]
pub struct ColumnDefinition {
    #[validate(length(min = 1, max = 64))]
    pub name: String,
    pub data_type: DataType,
    pub nullable: Option<bool>,
    pub default_value: Option<serde_json::Value>,
    pub constraints: Option<Vec<String>>,
    /// Whether this column auto-increments
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_increment: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum DataType {
    Integer,
    Float,
    Text,
    Boolean,
    DateTime,
    Binary,
    Json,
    DnaSequence,
    NeuralVector,
    QuantumState,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct IndexDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
    pub unique: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum IndexType {
    BTree,
    Hash,
    NeuralSimilarity,
    QuantumEntanglement,
    DnaKmer,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ConstraintDefinition {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub columns: Vec<String>,
    pub reference_table: Option<String>,
    pub reference_columns: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum ConstraintType {
    PrimaryKey,
    ForeignKey,
    Unique,
    Check,
    NotNull,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct NeuromorphicConfig {
    pub learning_rate: Option<f32>,
    pub plasticity_enabled: Option<bool>,
    pub adaptation_threshold: Option<f32>,
    pub memory_consolidation: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QuantumConfig {
    pub entanglement_enabled: Option<bool>,
    pub coherence_time_ms: Option<u32>,
    pub superposition_states: Option<u8>,
    pub measurement_basis: Option<String>,
}

// CRUD Operation DTOs

/// Generic SQL query request
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct SqlQueryRequest {
    #[validate(length(min = 1))]
    pub query: String,
}

/// Generic SQL query response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SqlQueryResponse {
    pub success: bool,
    pub rows_affected: Option<usize>,
    pub rows: Option<Vec<HashMap<String, serde_json::Value>>>,
    pub columns: Option<Vec<String>>,
    pub error: Option<String>,
    pub execution_time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateTableRequest {
    #[validate(nested)]
    pub schema: TableSchema,
    pub if_not_exists: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTableResponse {
    pub table_name: String,
    pub created_at: String,
    pub schema: TableSchema,
    pub table_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct InsertDataRequest {
    #[validate(length(min = 1, max = 64))]
    pub table_name: String,
    pub records: Vec<HashMap<String, serde_json::Value>>,
    pub on_conflict: Option<ConflictResolution>,
    pub batch_size: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum ConflictResolution {
    Ignore,
    Replace,
    Update,
    Abort,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InsertDataResponse {
    pub inserted_count: usize,
    pub failed_count: usize,
    pub inserted_ids: Vec<String>,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct QueryDataRequest {
    #[validate(length(min = 1, max = 64))]
    pub table_name: String,
    pub filters: Option<HashMap<String, FilterValue>>,
    pub sort: Option<Vec<SortField>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub columns: Option<Vec<String>>,
    pub neural_similarity: Option<NeuralSimilarityQuery>,
    pub quantum_search: Option<QuantumSearchQuery>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct FilterValue {
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum FilterOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    In,
    NotIn,
    Like,
    NotLike,
    IsNull,
    IsNotNull,
    Contains,
    StartsWith,
    EndsWith,
    NeuralSimilar,
    QuantumEntangled,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SortField {
    pub column: String,
    pub direction: SortDirection,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct NeuralSimilarityQuery {
    pub reference_vector: Vec<f32>,
    pub similarity_threshold: f32,
    pub max_results: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QuantumSearchQuery {
    pub quantum_state: Vec<f32>,
    pub entanglement_strength: f32,
    pub coherence_time: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryDataResponse {
    pub records: Vec<HashMap<String, serde_json::Value>>,
    pub total_count: usize,
    pub returned_count: usize,
    pub has_more: bool,
    pub query_stats: QueryStats,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryStats {
    pub execution_time_ms: f64,
    pub rows_scanned: usize,
    pub indexes_used: Vec<String>,
    pub neural_operations: Option<u32>,
    pub quantum_operations: Option<u32>,
    pub cache_hit_rate: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateDataRequest {
    #[validate(length(min = 1, max = 64))]
    pub table_name: String,
    pub filters: HashMap<String, FilterValue>,
    pub updates: HashMap<String, serde_json::Value>,
    pub optimistic_lock_version: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateDataResponse {
    pub updated_count: usize,
    pub matched_count: usize,
    pub new_version: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct DeleteDataRequest {
    #[validate(length(min = 1, max = 64))]
    pub table_name: String,
    pub filters: HashMap<String, FilterValue>,
    pub cascade: Option<bool>,
    pub soft_delete: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteDataResponse {
    pub deleted_count: usize,
    pub cascaded_deletes: Option<HashMap<String, usize>>,
}

// Advanced Feature DTOs

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct TrainNeuralNetworkRequest {
    pub network_name: String,
    pub training_data: Vec<TrainingExample>,
    pub config: NeuralNetworkConfig,
    pub validation_split: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TrainingExample {
    pub input: Vec<f32>,
    pub target: Vec<f32>,
    pub weight: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct NeuralNetworkConfig {
    pub layers: Vec<LayerConfig>,
    pub learning_rate: f32,
    pub epochs: u32,
    pub batch_size: u32,
    pub optimizer: OptimizerType,
    pub loss_function: LossFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct LayerConfig {
    pub layer_type: LayerType,
    pub size: u32,
    pub activation: ActivationFunction,
    pub dropout: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum LayerType {
    Dense,
    Convolutional,
    Recurrent,
    Attention,
    Neuromorphic,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum ActivationFunction {
    ReLU,
    Sigmoid,
    Tanh,
    Softmax,
    Swish,
    SpikingNeuron,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum OptimizerType {
    SGD,
    Adam,
    AdaGrad,
    RMSprop,
    NeuromorphicSTDP,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum LossFunction {
    MeanSquaredError,
    CrossEntropy,
    BinaryCrossEntropy,
    Huber,
    SpikeTimingLoss,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrainNeuralNetworkResponse {
    pub network_id: String,
    pub training_status: TrainingStatus,
    pub initial_loss: Option<f32>,
    pub training_started_at: String,
    pub estimated_completion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum TrainingStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct QuantumSearchRequest {
    #[validate(length(min = 1, max = 64))]
    pub table_name: String,
    pub query_vector: Vec<f32>,
    pub similarity_threshold: f32,
    pub max_results: Option<u32>,
    pub entanglement_boost: Option<f32>,
    /// Enable real quantum TFIM computation
    #[serde(default)]
    pub use_tfim: bool,
    /// TFIM configuration (optional)
    pub tfim_config: Option<TFIMRequestConfig>,
    /// Enable QUBO optimization
    #[serde(default)]
    pub use_qubo: bool,
    /// QUBO configuration (optional)
    pub qubo_config: Option<QUBORequestConfig>,
    /// Enable Quantum Parallel Tempering
    #[serde(default)]
    pub use_parallel_tempering: bool,
    /// Parallel Tempering configuration (optional)
    pub parallel_tempering_config: Option<ParallelTemperingRequestConfig>,
    /// Enable real quantum Grover's search algorithm
    #[serde(default)]
    pub use_grover: bool,
    /// Grover search configuration (optional)
    pub grover_config: Option<GroverRequestConfig>,
    /// Pattern to search for (used with Grover search for byte pattern matching)
    pub search_pattern: Option<String>,
}

/// TFIM configuration for quantum search
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TFIMRequestConfig {
    /// Solution method: "trotter", "vqe", or "qaoa"
    #[serde(default = "default_tfim_method")]
    pub method: String,
    /// Number of measurement shots
    #[serde(default = "default_num_shots")]
    pub num_shots: u32,
    /// Trotter steps (for Trotter-Suzuki method)
    #[serde(default = "default_trotter_steps")]
    pub trotter_steps: u32,
    /// Evolution time
    #[serde(default = "default_evolution_time")]
    pub evolution_time: f64,
    /// Transverse field strength
    #[serde(default = "default_transverse_field")]
    pub transverse_field: f64,
    /// Number of QAOA layers (for QAOA method)
    #[serde(default = "default_qaoa_layers")]
    pub qaoa_layers: u32,
    /// VQE ansatz depth (for VQE method)
    #[serde(default = "default_vqe_depth")]
    pub vqe_depth: u32,
}

fn default_tfim_method() -> String {
    "trotter".to_string()
}
fn default_num_shots() -> u32 {
    1000
}
fn default_trotter_steps() -> u32 {
    10
}
fn default_evolution_time() -> f64 {
    1.0
}
fn default_transverse_field() -> f64 {
    0.5
}
fn default_qaoa_layers() -> u32 {
    2
}
fn default_vqe_depth() -> u32 {
    3
}

impl Default for TFIMRequestConfig {
    fn default() -> Self {
        Self {
            method: default_tfim_method(),
            num_shots: default_num_shots(),
            trotter_steps: default_trotter_steps(),
            evolution_time: default_evolution_time(),
            transverse_field: default_transverse_field(),
            qaoa_layers: default_qaoa_layers(),
            vqe_depth: default_vqe_depth(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuantumSearchResponse {
    pub results: Vec<QuantumSearchResult>,
    pub quantum_stats: QuantumStats,
    /// TFIM-specific results (when use_tfim=true)
    pub tfim_results: Option<TFIMResults>,
    /// QUBO optimization results (when use_qubo=true)
    pub qubo_results: Option<QUBOResults>,
    /// Parallel Tempering results (when use_parallel_tempering=true)
    pub parallel_tempering_results: Option<ParallelTemperingResults>,
    /// Grover's search results (when use_grover=true)
    pub grover_results: Option<GroverResults>,
}

/// TFIM computation results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TFIMResults {
    /// Ground state energy
    pub energy: f64,
    /// Energy variance
    pub energy_variance: f64,
    /// Magnetization per qubit
    pub magnetization: Vec<f64>,
    /// Order parameter
    pub order_parameter: f64,
    /// Correlation matrix (flattened)
    pub correlations: Vec<f64>,
    /// Number of qubits used
    pub num_qubits: usize,
    /// Solution method used
    pub method_used: String,
    /// Whether quantum backend was used (vs classical fallback)
    pub used_quantum: bool,
    /// Ground state fidelity (if available)
    pub fidelity: Option<f64>,
}

/// QUBO request configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QUBORequestConfig {
    /// Backend: "vqe", "qaoa", "sqa" (simulated quantum annealing), or "classical"
    #[serde(default = "default_qubo_backend")]
    pub backend: String,
    /// Number of measurement shots
    #[serde(default = "default_num_shots")]
    pub num_shots: u32,
    /// QAOA circuit depth
    #[serde(default = "default_qaoa_depth")]
    pub qaoa_depth: u32,
    /// Maximum optimization iterations
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Convergence threshold
    #[serde(default = "default_convergence_threshold")]
    pub convergence_threshold: f64,
}

fn default_qubo_backend() -> String {
    "qaoa".to_string()
}
fn default_qaoa_depth() -> u32 {
    3
}
fn default_max_iterations() -> u32 {
    500
}
fn default_convergence_threshold() -> f64 {
    1e-6
}

impl Default for QUBORequestConfig {
    fn default() -> Self {
        Self {
            backend: default_qubo_backend(),
            num_shots: default_num_shots(),
            qaoa_depth: default_qaoa_depth(),
            max_iterations: default_max_iterations(),
            convergence_threshold: default_convergence_threshold(),
        }
    }
}

/// QUBO optimization results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QUBOResults {
    /// Binary variable assignments (0 or 1)
    pub variables: Vec<u8>,
    /// QUBO objective value (energy)
    pub energy: f64,
    /// Solution quality (0.0 to 1.0)
    pub quality: f64,
    /// Backend used for solving
    pub backend_used: String,
    /// Number of quantum circuit evaluations
    pub quantum_evaluations: u32,
    /// Optimization iterations
    pub iterations: u32,
    /// Whether convergence was achieved
    pub converged: bool,
    /// Computation time in milliseconds
    pub computation_time_ms: f64,
    /// Best state probability (if measurement-based)
    pub best_state_probability: Option<f64>,
}

/// Parallel Tempering request configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParallelTemperingRequestConfig {
    /// Backend: "pimc" (Path Integral Monte Carlo), "qmc", "annealing", or "hybrid"
    #[serde(default = "default_pt_backend")]
    pub backend: String,
    /// Number of temperature replicas
    #[serde(default = "default_num_replicas")]
    pub num_replicas: u32,
    /// Minimum temperature
    #[serde(default = "default_min_temperature")]
    pub min_temperature: f64,
    /// Maximum temperature
    #[serde(default = "default_max_temperature")]
    pub max_temperature: f64,
    /// Number of Trotter slices for PIMC
    #[serde(default = "default_pt_trotter_slices")]
    pub trotter_slices: u32,
    /// Number of exchange rounds
    #[serde(default = "default_num_exchanges")]
    pub num_exchanges: u32,
    /// Transverse field strength
    #[serde(default = "default_pt_transverse_field")]
    pub transverse_field: f64,
}

fn default_pt_backend() -> String {
    "pimc".to_string()
}
fn default_num_replicas() -> u32 {
    8
}
fn default_min_temperature() -> f64 {
    0.1
}
fn default_max_temperature() -> f64 {
    10.0
}
fn default_pt_trotter_slices() -> u32 {
    20
}
fn default_num_exchanges() -> u32 {
    100
}
fn default_pt_transverse_field() -> f64 {
    1.0
}

impl Default for ParallelTemperingRequestConfig {
    fn default() -> Self {
        Self {
            backend: default_pt_backend(),
            num_replicas: default_num_replicas(),
            min_temperature: default_min_temperature(),
            max_temperature: default_max_temperature(),
            trotter_slices: default_pt_trotter_slices(),
            num_exchanges: default_num_exchanges(),
            transverse_field: default_pt_transverse_field(),
        }
    }
}

/// Parallel Tempering results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParallelTemperingResults {
    /// Best spin configuration found
    pub best_configuration: Vec<i8>,
    /// Energy of the best configuration
    pub best_energy: f64,
    /// Replica ID that found the best solution
    pub best_replica_id: u32,
    /// Total exchange attempts
    pub total_exchanges: u32,
    /// Exchange acceptance rate
    pub acceptance_rate: f64,
    /// Estimated ground state energy
    pub ground_state_energy_estimate: f64,
    /// Quantum fidelity with expected thermal state
    pub thermal_state_fidelity: f64,
    /// Backend used
    pub backend_used: String,
    /// Computation time in milliseconds
    pub computation_time_ms: f64,
}

/// Grover's search request configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GroverRequestConfig {
    /// Backend: "simulator", "superconducting", "trapped_ion", "neutral_atom", or "classical"
    #[serde(default = "default_grover_backend")]
    pub backend: String,
    /// Number of measurement shots
    #[serde(default = "default_grover_shots")]
    pub num_shots: u32,
    /// Maximum Grover iterations (0 for auto-optimal)
    #[serde(default)]
    pub max_iterations: u32,
    /// Enable error mitigation
    #[serde(default = "default_error_mitigation")]
    pub error_mitigation: bool,
    /// Minimum success probability threshold
    #[serde(default = "default_success_threshold")]
    pub success_threshold: f64,
}

fn default_grover_backend() -> String {
    "simulator".to_string()
}
fn default_grover_shots() -> u32 {
    1024
}
fn default_error_mitigation() -> bool {
    true
}
fn default_success_threshold() -> f64 {
    0.5
}

impl Default for GroverRequestConfig {
    fn default() -> Self {
        Self {
            backend: default_grover_backend(),
            num_shots: default_grover_shots(),
            max_iterations: 0, // Auto-calculate optimal
            error_mitigation: default_error_mitigation(),
            success_threshold: default_success_threshold(),
        }
    }
}

/// Grover's search results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GroverResults {
    /// Found indices in the search space
    pub found_indices: Vec<usize>,
    /// Measurement probabilities for each found index
    pub probabilities: Vec<f64>,
    /// Number of Grover iterations performed
    pub iterations: usize,
    /// Optimal number of iterations (theoretical π/4 * √N)
    pub optimal_iterations: usize,
    /// Number of qubits used
    pub num_qubits: usize,
    /// Circuit depth
    pub circuit_depth: usize,
    /// Backend used for computation
    pub backend_used: String,
    /// Theoretical quantum speedup (√N)
    pub quantum_speedup: f64,
    /// Computation time in milliseconds
    pub computation_time_ms: f64,
    /// Best measurement probability (from sampling)
    pub best_probability: Option<f64>,
    /// Number of marked states (targets)
    pub num_marked_states: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuantumSearchResult {
    pub record: HashMap<String, serde_json::Value>,
    pub similarity_score: f32,
    pub quantum_probability: f32,
    pub entanglement_strength: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuantumStats {
    pub coherence_time_used_ms: f32,
    pub superposition_states: u32,
    pub measurement_collapses: u32,
    pub entanglement_operations: u32,
    /// Circuit depth (for TFIM)
    pub circuit_depth: Option<u32>,
    /// Number of quantum gates (for TFIM)
    pub num_gates: Option<u32>,
    /// Trotter steps used (for TFIM)
    pub trotter_steps: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct CompressDnaRequest {
    pub sequences: Vec<String>,
    pub algorithm: CompressionAlgorithm,
    pub compression_level: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum CompressionAlgorithm {
    KmerBased,
    NeuralNetwork,
    QuantumInspired,
    Hybrid,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompressDnaResponse {
    pub compressed_sequences: Vec<CompressedSequence>,
    pub compression_stats: CompressionStats,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompressedSequence {
    pub original_length: usize,
    pub compressed_data: String,
    pub compression_ratio: f32,
    pub checksum: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompressionStats {
    pub total_input_size: usize,
    pub total_compressed_size: usize,
    pub average_compression_ratio: f32,
    pub compression_time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct DecompressDnaRequest {
    pub compressed_data: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DecompressDnaResponse {
    pub decompressed_sequences: Vec<DecompressedSequence>,
    pub decompression_stats: DecompressionStats,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DecompressedSequence {
    pub decompressed_data: String,
    pub original_checksum: String,
    pub checksum_valid: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DecompressionStats {
    pub total_compressed_size: usize,
    pub total_decompressed_size: usize,
    pub decompression_time_ms: f64,
}

// Performance and Monitoring DTOs

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PerformanceStats {
    pub system_metrics: SystemMetrics,
    pub database_metrics: DatabaseMetrics,
    pub neural_metrics: NeuralMetrics,
    pub quantum_metrics: QuantumMetrics,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SystemMetrics {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub disk_usage_mb: u64,
    pub network_io_mb: f64,
    pub power_consumption_watts: Option<f32>,
    pub temperature_celsius: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DatabaseMetrics {
    pub active_connections: u32,
    pub queries_per_second: f32,
    pub average_query_time_ms: f32,
    pub cache_hit_ratio: f32,
    pub total_tables: u32,
    pub total_records: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NeuralMetrics {
    pub active_networks: u32,
    pub training_jobs: u32,
    pub inference_operations_per_second: f32,
    pub average_accuracy: f32,
    pub synaptic_updates_per_second: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuantumMetrics {
    pub coherence_time_ms: f32,
    pub entanglement_operations_per_second: f32,
    pub quantum_state_fidelity: f32,
    pub measurement_error_rate: f32,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let metadata = ResponseMetadata {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            response_time_ms: 0.0,
            message: self.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let response = ApiResponse::<()>::error(self.clone(), metadata);

        match self {
            ApiError::Unauthorized(_) => HttpResponse::Unauthorized().json(response),
            ApiError::Forbidden(_) => HttpResponse::Forbidden().json(response),
            ApiError::BadRequest(_) | ApiError::ValidationError { .. } => {
                HttpResponse::BadRequest().json(response)
            }
            ApiError::NotFound(_) => HttpResponse::NotFound().json(response),
            ApiError::Conflict(_) => HttpResponse::Conflict().json(response),
            ApiError::RateLimitExceeded { .. } => HttpResponse::TooManyRequests().json(response),
            ApiError::ServiceUnavailable { .. } | ApiError::CircuitBreakerOpen { .. } => {
                HttpResponse::ServiceUnavailable().json(response)
            }
            ApiError::NotImplemented(_) => HttpResponse::NotImplemented().json(response),
            _ => HttpResponse::InternalServerError().json(response),
        }
    }
}

impl<T> From<ApiResponse<T>> for HttpResponse
where
    T: Serialize,
{
    fn from(response: ApiResponse<T>) -> Self {
        if response.success {
            HttpResponse::Ok().json(response)
        } else {
            let mut status = match &response.error {
                Some(ApiError::ValidationError { .. }) => HttpResponse::BadRequest(),
                Some(ApiError::Unauthorized(_)) => HttpResponse::Unauthorized(),
                Some(ApiError::Forbidden(_)) => HttpResponse::Forbidden(),
                Some(ApiError::BadRequest(_)) => HttpResponse::BadRequest(),
                Some(ApiError::NotFound(_)) => HttpResponse::NotFound(),
                Some(ApiError::Conflict(_)) => HttpResponse::Conflict(),
                Some(ApiError::RateLimitExceeded { .. }) => HttpResponse::TooManyRequests(),
                Some(ApiError::QuantumOperationFailed { .. }) => {
                    HttpResponse::InternalServerError()
                }
                Some(ApiError::InvalidQuery { .. }) => HttpResponse::BadRequest(),
                Some(ApiError::InternalServerError { .. }) => HttpResponse::InternalServerError(),
                Some(ApiError::CompressionError { .. }) => HttpResponse::InternalServerError(),
                Some(ApiError::EncryptionError { .. }) => HttpResponse::InternalServerError(),
                Some(ApiError::NeuralNetworkError { .. }) => HttpResponse::InternalServerError(),
                Some(ApiError::TableError { .. }) => HttpResponse::InternalServerError(),
                Some(ApiError::ConnectionPoolError { .. }) => HttpResponse::InternalServerError(),
                Some(ApiError::CircuitBreakerOpen { .. }) => HttpResponse::ServiceUnavailable(),
                Some(ApiError::ServiceUnavailable { .. }) => HttpResponse::ServiceUnavailable(),
                Some(ApiError::NotImplemented(_)) => HttpResponse::NotImplemented(),
                None => HttpResponse::InternalServerError(),
            };
            status.json(response)
        }
    }
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn error_response(self) -> HttpResponse {
        self.into()
    }
}

/// Authentication token structure
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub sub: String,       // Subject (user ID)
    pub exp: usize,        // Expiration time
    pub iat: usize,        // Issued at
    pub quantum_level: u8, // Quantum security level (0-255)
    pub permissions: Vec<String>,
}

/// Quantum-resistant authentication claims
#[derive(Debug, Serialize, Deserialize)]
pub struct QuantumAuthClaims {
    pub user_id: String,
    pub session_id: String,
    pub quantum_signature: String,
    pub kyber_public_key: String,
    pub dilithium_signature: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let metadata = ResponseMetadata {
            request_id: "test-123".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            response_time_ms: 500.0,
            message: "Success".to_string(),
            version: "1.0.0".to_string(),
        };
        let response = ApiResponse::success("test data", metadata);
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_error_response() {
        let error = ApiError::InvalidQuery {
            details: "Missing required field".to_string(),
        };

        let metadata = ResponseMetadata {
            request_id: "test-456".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            response_time_ms: 100.0,
            message: "Error".to_string(),
            version: "1.0.0".to_string(),
        };
        let response = ApiResponse::<()>::error(error, metadata);
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }
}
