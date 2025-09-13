use crate::error::{ApiError, ApiResponse, ResponseMetadata};
use neuroquantum_core::{NeuroQuantumDB, QueryRequest};
use neuroquantum_qsql::parser::QSQLParser;
use actix_web::{web, HttpRequest, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{error, info};
use utoipa::{IntoParams, ToSchema};

/// Request structure for quantum-enhanced search operations
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct QuantumSearchRequest {
    /// The search query string
    pub query: String,
    /// Quantum enhancement level (0-255)
    pub quantum_level: Option<u8>,
    /// Enable Grover's algorithm optimization
    pub use_grovers: Option<bool>,
    /// Maximum number of results
    pub limit: Option<u32>,
    /// Result offset for pagination
    pub offset: Option<u32>,
}

/// Response structure for quantum search results
#[derive(Debug, Serialize, ToSchema)]
pub struct QuantumSearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: u64,
    pub quantum_speedup: f32,
    pub compression_savings: f32,
    pub neuromorphic_optimizations: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResult {
    pub id: String,
    pub data: serde_json::Value,
    pub relevance_score: f32,
    pub synaptic_strength: f32,
}

/// QSQL query execution request
#[derive(Debug, Deserialize, ToSchema)]
pub struct QSQLRequest {
    pub query: String,
    pub parameters: Option<serde_json::Value>,
    pub optimize: Option<bool>,
    pub explain: Option<bool>,
}

/// QSQL query execution response
#[derive(Debug, Serialize, ToSchema)]
pub struct QSQLResponse {
    pub results: serde_json::Value,
    pub execution_plan: Option<String>,
    pub performance_metrics: QueryMetrics,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QueryMetrics {
    pub execution_time_us: u64,
    pub memory_usage_mb: f32,
    pub power_consumption_mw: f32,
    pub quantum_operations: u32,
    pub synaptic_adaptations: u32,
}

/// Quantum-optimized search endpoint
#[utoipa::path(
    get,
    path = "/api/v1/quantum-search",
    params(QuantumSearchRequest),
    responses(
        (status = 200, description = "Quantum search completed successfully", body = QuantumSearchResponse),
        (status = 400, description = "Invalid search parameters"),
        (status = 401, description = "Authentication required"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Quantum Operations"
)]
#[tracing::instrument(skip(db, _req))]
pub async fn quantum_search(
    query: web::Query<QuantumSearchRequest>,
    db: web::Data<NeuroQuantumDB>,
    _req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let start_time = Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        query = %query.query,
        "Processing quantum search request"
    );

    // Validate quantum level (u8 is always <= 255, but we keep validation for API consistency)
    let quantum_level = query.quantum_level.unwrap_or(128);
    
    // Execute quantum-enhanced search
    match execute_quantum_search(&db, &query, quantum_level).await {
        Ok(response) => {
            let metadata = create_metadata(request_id.clone(), start_time, true);
            info!(
                request_id = %request_id,
                results_count = response.results.len(),
                quantum_speedup = response.quantum_speedup,
                "Quantum search completed successfully"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response, metadata)))
        }
        Err(e) => {
            let metadata = create_metadata(request_id.clone(), start_time, false);
            error!(
                request_id = %request_id,
                error = %e,
                "Quantum search failed"
            );

            Ok(ApiResponse::<()>::error(
                ApiError::QuantumOperationFailed {
                    operation: "quantum_search".to_string(),
                },
                metadata,
            ).error_response())
        }
    }
}

/// QSQL query execution endpoint
#[utoipa::path(
    post,
    path = "/api/v1/qsql/execute",
    request_body = QSQLRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = QSQLResponse),
        (status = 400, description = "Invalid QSQL syntax"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Query execution failed")
    ),
    tag = "QSQL Operations"
)]
#[tracing::instrument(skip(db, request))]
pub async fn execute_qsql(
    request: web::Json<QSQLRequest>,
    db: web::Data<NeuroQuantumDB>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let start_time = Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        query = %request.query,
        "Processing QSQL execution request"
    );

    // Parse and validate QSQL query
    let parser = QSQLParser::new();
    let query_plan = match parser.parse_query(&request.query) {
        Ok(plan) => plan,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(ApiError::InvalidQuery {
                details: format!("QSQL parsing error: {}", e),
            }));
        }
    };

    // Execute query with neuromorphic optimization
    match execute_qsql_query(&db, query_plan, request.optimize.unwrap_or(true)).await {
        Ok(response) => {
            let metadata = create_metadata(request_id.clone(), start_time, true);
            info!(
                request_id = %request_id,
                execution_time_us = response.performance_metrics.execution_time_us,
                "QSQL query executed successfully"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response, metadata)))
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "QSQL query execution failed"
            );

            Ok(HttpResponse::InternalServerError().json(ApiError::InvalidQuery {
                details: format!("Query execution error: {}", e),
            }))
        }
    }
}

/// Health check endpoint with system metrics
#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "System healthy", body = SystemHealth),
        (status = 503, description = "System unhealthy")
    ),
    tag = "System"
)]
pub async fn health_check(
    db: web::Data<NeuroQuantumDB>,
) -> ActixResult<HttpResponse> {
    let health = SystemHealth {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime_seconds(),
        memory_usage_mb: get_memory_usage_mb(),
        power_consumption_mw: get_power_consumption_mw(),
        active_connections: db.get_active_connections(),
        quantum_operations_per_second: db.get_quantum_ops_rate(),
        neuromorphic_adaptations: db.get_synaptic_adaptations(),
        compression_ratio: db.get_avg_compression_ratio(),
    };

    Ok(HttpResponse::Ok().json(health))
}

/// Prometheus metrics endpoint for monitoring
#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    responses(
        (status = 200, description = "Prometheus metrics", content_type = "text/plain"),
        (status = 401, description = "Authentication required")
    ),
    tag = "System"
)]
pub async fn prometheus_metrics(
    db: web::Data<NeuroQuantumDB>,
) -> ActixResult<HttpResponse> {
    let metrics = collect_prometheus_metrics(&db).await;

    Ok(HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(metrics))
}

/// Database schema introspection endpoint
#[utoipa::path(
    get,
    path = "/api/v1/schema",
    responses(
        (status = 200, description = "Database schema information", body = SchemaInfo),
        (status = 401, description = "Authentication required")
    ),
    tag = "System"
)]
pub async fn schema_info(
    db: web::Data<NeuroQuantumDB>,
) -> ActixResult<HttpResponse> {
    let start_time = Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        "Processing schema introspection request"
    );

    // Retrieve schema information
    match db.get_schema_info().await {
        Ok(schema_info) => {
            let metadata = create_metadata(request_id.clone(), start_time, true);
            info!(
                request_id = %request_id,
                "Schema introspection completed successfully"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(schema_info, metadata)))
        }
        Err(e) => {
            let metadata = create_metadata(request_id.clone(), start_time, false);
            error!(
                request_id = %request_id,
                error = %e,
                "Schema introspection failed"
            );

            Ok(ApiResponse::<()>::error(
                ApiError::InternalError {
                    context: format!("Schema introspection failed: {}", e),
                },
                metadata,
            ).error_response())
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SystemHealth {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub memory_usage_mb: f32,
    pub power_consumption_mw: f32,
    pub active_connections: u32,
    pub quantum_operations_per_second: f32,
    pub neuromorphic_adaptations: u64,
    pub compression_ratio: f32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SchemaInfo {
    pub tables: Vec<TableInfo>,
    pub synaptic_networks: Vec<SynapticNetworkInfo>,
    pub quantum_indexes: Vec<QuantumIndexInfo>,
    pub compression_stats: CompressionStats,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub row_count: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub synaptic_indexed: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SynapticNetworkInfo {
    pub name: String,
    pub node_count: u32,
    pub connection_count: u64,
    pub average_strength: f32,
    pub learning_rate: f32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QuantumIndexInfo {
    pub name: String,
    pub quantum_level: u8,
    pub grovers_optimized: bool,
    pub search_speedup: f32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompressionStats {
    pub total_size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub compression_ratio: f32,
    pub dna_encoded_blocks: u64,
}

// Helper functions

async fn execute_quantum_search(
    db: &NeuroQuantumDB,
    request: &QuantumSearchRequest,
    quantum_level: u8,
) -> anyhow::Result<QuantumSearchResponse> {
    let query_req = QueryRequest {
        query: request.query.clone(),
        quantum_level,
        use_grovers: request.use_grovers.unwrap_or(true),
        limit: request.limit.unwrap_or(100),
        offset: request.offset.unwrap_or(0),
    };

    let result = db.quantum_search(query_req).await?;

    Ok(QuantumSearchResponse {
        results: result.results.into_iter().map(|r| SearchResult {
            id: r.id,
            data: r.data,
            relevance_score: r.relevance_score,
            synaptic_strength: r.synaptic_strength,
        }).collect(),
        total_count: result.total_count,
        quantum_speedup: result.quantum_speedup,
        compression_savings: result.compression_savings,
        neuromorphic_optimizations: result.neuromorphic_optimizations,
    })
}

async fn execute_qsql_query(
    db: &NeuroQuantumDB,
    statement: neuroquantum_qsql::ast::Statement,
    optimize: bool,
) -> anyhow::Result<QSQLResponse> {
    // Convert Statement to QueryPlan using the optimizer
    use neuroquantum_qsql::optimizer::NeuromorphicOptimizer;
    let mut optimizer = NeuromorphicOptimizer::new()?;
    let query_plan = optimizer.optimize(statement)?;

    let result = db.execute_qsql(query_plan, optimize).await?;

    Ok(QSQLResponse {
        results: result.data,
        execution_plan: result.execution_plan,
        performance_metrics: QueryMetrics {
            execution_time_us: result.execution_time_us,
            memory_usage_mb: result.memory_usage_mb,
            power_consumption_mw: result.power_consumption_mw,
            quantum_operations: result.quantum_operations,
            synaptic_adaptations: result.synaptic_adaptations,
        },
    })
}

fn create_metadata(request_id: String, start_time: Instant, quantum_enhancement: bool) -> ResponseMetadata {
    ResponseMetadata {
        request_id,
        timestamp: chrono::Utc::now(),
        processing_time_us: start_time.elapsed().as_micros() as u64,
        quantum_enhancement,
        compression_ratio: None, // Will be set by compression layer
    }
}

fn get_uptime_seconds() -> u64 {
    // Implementation would track actual uptime
    0
}

fn get_memory_usage_mb() -> f32 {
    // Implementation would get actual memory usage
    0.0
}

fn get_power_consumption_mw() -> f32 {
    // Implementation would measure actual power consumption
    0.0
}

async fn collect_prometheus_metrics(db: &NeuroQuantumDB) -> String {
    let mut metrics = String::new();

    // System metrics
    metrics.push_str("# HELP neuroquantum_uptime_seconds Uptime in seconds\n");
    metrics.push_str("# TYPE neuroquantum_uptime_seconds counter\n");
    metrics.push_str(&format!("neuroquantum_uptime_seconds {}\n", get_uptime_seconds()));

    // Memory metrics
    metrics.push_str("# HELP neuroquantum_memory_usage_bytes Memory usage in bytes\n");
    metrics.push_str("# TYPE neuroquantum_memory_usage_bytes gauge\n");
    metrics.push_str(&format!("neuroquantum_memory_usage_bytes {}\n", get_memory_usage_mb() * 1024.0 * 1024.0));

    // Power consumption metrics
    metrics.push_str("# HELP neuroquantum_power_consumption_milliwatts Power consumption in milliwatts\n");
    metrics.push_str("# TYPE neuroquantum_power_consumption_milliwatts gauge\n");
    metrics.push_str(&format!("neuroquantum_power_consumption_milliwatts {}\n", get_power_consumption_mw()));

    // Database metrics
    metrics.push_str("# HELP neuroquantum_active_connections Active database connections\n");
    metrics.push_str("# TYPE neuroquantum_active_connections gauge\n");
    metrics.push_str(&format!("neuroquantum_active_connections {}\n", db.get_active_connections()));

    // Quantum metrics
    metrics.push_str("# HELP neuroquantum_quantum_operations_per_second Quantum operations per second\n");
    metrics.push_str("# TYPE neuroquantum_quantum_operations_per_second gauge\n");
    metrics.push_str(&format!("neuroquantum_quantum_operations_per_second {}\n", db.get_quantum_ops_rate()));

    // Neuromorphic metrics
    metrics.push_str("# HELP neuroquantum_synaptic_adaptations_total Total synaptic adaptations\n");
    metrics.push_str("# TYPE neuroquantum_synaptic_adaptations_total counter\n");
    metrics.push_str(&format!("neuroquantum_synaptic_adaptations_total {}\n", db.get_synaptic_adaptations()));

    // Compression metrics
    metrics.push_str("# HELP neuroquantum_compression_ratio Current compression ratio\n");
    metrics.push_str("# TYPE neuroquantum_compression_ratio gauge\n");
    metrics.push_str(&format!("neuroquantum_compression_ratio {}\n", db.get_avg_compression_ratio()));

    metrics
}
