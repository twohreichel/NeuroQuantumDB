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

    #[error("Quantum operation failed: {operation}")]
    QuantumOperationFailed { operation: String },

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
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuantumSearchResponse {
    pub results: Vec<QuantumSearchResult>,
    pub quantum_stats: QuantumStats,
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
