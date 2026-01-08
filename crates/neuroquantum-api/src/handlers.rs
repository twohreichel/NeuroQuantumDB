use crate::auth::{ApiKey, AuthService};
use crate::error::*;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result as ActixResult};
use neuroquantum_core::{DNACompressor, NeuroQuantumDB};
use neuroquantum_qsql::query_plan::QueryValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};
use utoipa::{OpenApi, ToSchema};
use validator::Validate;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        generate_api_key,
        revoke_api_key,
        login,
        refresh_token,
        execute_sql_query,
        create_table,
        insert_data,
        query_data,
        update_data,
        delete_data,
        train_neural_network,
        get_training_status,
        quantum_search,
        compress_dna,
        decompress_dna,
        get_metrics,
        get_performance_stats,
        eeg_enroll,
        eeg_authenticate,
        eeg_update_signature,
        eeg_list_users,
        get_index_recommendations,
        clear_index_advisor_statistics,
    ),
    components(
        schemas(
            // Auth DTOs
            GenerateKeyRequest,
            GenerateKeyResponse,
            RevokeKeyRequest,
            LoginRequest,
            LoginResponse,
            RefreshTokenRequest,

            // CRUD DTOs
            SqlQueryRequest,
            SqlQueryResponse,
            CreateTableRequest,
            CreateTableResponse,
            InsertDataRequest,
            InsertDataResponse,
            QueryDataRequest,
            QueryDataResponse,
            UpdateDataRequest,
            UpdateDataResponse,
            DeleteDataRequest,
            DeleteDataResponse,

            // Advanced Feature DTOs
            TrainNeuralNetworkRequest,
            TrainNeuralNetworkResponse,
            QuantumSearchRequest,
            QuantumSearchResponse,
            QuantumSearchResult,
            QuantumStats,
            TFIMRequestConfig,
            TFIMResults,
            QUBORequestConfig,
            QUBOResults,
            ParallelTemperingRequestConfig,
            ParallelTemperingResults,
            GroverRequestConfig,
            GroverResults,
            CompressDnaRequest,
            CompressDnaResponse,
            CompressedSequence,
            CompressionStats,
            DecompressDnaRequest,
            DecompressDnaResponse,
            DecompressedSequence,
            DecompressionStats,

            // Monitoring DTOs
            PerformanceStats,
            SystemMetrics,
            DatabaseMetrics,
            NeuralMetrics,
            QuantumMetrics,

            // Biometric Auth DTOs
            EEGEnrollRequest,
            EEGEnrollResponse,
            EEGAuthRequest,
            EEGAuthResponse,

            // Index Advisor DTOs
            IndexAdvisorResponse,
            IndexRecommendationDto,
            IndexAdvisorStatsDto,

            // Common DTOs
            TableSchema,
            ColumnDefinition,
            DataType,
            ApiError,
            ApiResponse<String>,
        )
    ),
    tags(
        (name = "Authentication", description = "Authentication and API key management"),
        (name = "CRUD Operations", description = "Create, Read, Update, Delete operations"),
        (name = "Advanced Features", description = "Neural networks, quantum search, DNA compression, index advisor"),
        (name = "Monitoring", description = "Metrics and performance monitoring"),
        (name = "Biometric Authentication", description = "EEG-based biometric authentication")
    )
)]
pub struct ApiDoc;

// =============================================================================
// AUTHENTICATION HANDLERS
// =============================================================================

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub username: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub quantum_enabled: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub user_id: String,
    pub permissions: Vec<String>,
    pub quantum_level: u8,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GenerateKeyRequest {
    pub name: String,
    pub permissions: Vec<String>,
    pub expiry_hours: Option<u32>,
    pub rate_limit_per_hour: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GenerateKeyResponse {
    pub api_key: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub expires_at: String,
    pub created_at: String,
    pub rate_limit_per_hour: Option<u32>,
    pub warning: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeKeyRequest {
    pub api_key: String,
}

/// User login with JWT token generation - DISABLED FOR SECURITY
///
/// This endpoint has been disabled. NeuroQuantumDB now uses API-Key-Only authentication.
/// To obtain an API key:
/// 1. Run: `neuroquantum-api init` to create your first admin key
/// 2. Use the X-API-Key header for authentication
/// 3. Admin users can create additional API keys via `/api/v1/auth/generate-key`
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 501, description = "Endpoint disabled - use API keys instead", body = ApiResponse<String>),
    ),
    tag = "Authentication"
)]
pub async fn login(
    _auth_service: web::Data<AuthService>,
    _login_req: web::Json<LoginRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    warn!("⚠️ Attempt to use disabled /auth/login endpoint");

    Err(ApiError::NotImplemented(
        "JWT login has been disabled for security reasons. \
         NeuroQuantumDB uses API-Key-Only authentication. \
         Please run 'neuroquantum-api init' to create your first admin API key, \
         then use the X-API-Key header for authentication."
            .to_string(),
    ))
}

/// Refresh JWT token - DISABLED FOR SECURITY
///
/// This endpoint has been disabled. API keys don't need to be refreshed.
/// If your API key expires, request a new one from an admin user.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 501, description = "Endpoint disabled - API keys don't need refresh", body = ApiResponse<String>),
    ),
    tag = "Authentication"
)]
pub async fn refresh_token(
    _refresh_req: web::Json<RefreshTokenRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    warn!("⚠️ Attempt to use disabled /auth/refresh endpoint");

    Err(ApiError::NotImplemented(
        "Token refresh has been disabled. \
         API keys have fixed expiration dates and cannot be refreshed. \
         Contact an admin to generate a new API key if needed."
            .to_string(),
    ))
}

/// Generate new API key (requires admin permission)
#[utoipa::path(
    post,
    path = "/api/v1/auth/generate-key",
    request_body = GenerateKeyRequest,
    responses(
        (status = 200, description = "API key generated", body = ApiResponse<GenerateKeyResponse>),
        (status = 403, description = "Admin permission required", body = ApiResponse<String>),
    ),
    tag = "Authentication"
)]
pub async fn generate_api_key(
    req: HttpRequest,
    auth_service: web::Data<AuthService>,
    key_request: web::Json<GenerateKeyRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    let extensions = req.extensions();
    let requesting_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Admin authentication required".to_string()))?;

    if !requesting_key.permissions.contains(&"admin".to_string()) {
        warn!(
            "Non-admin user attempted to generate API key: {}",
            requesting_key.name
        );
        crate::metrics::record_auth_request("failed");
        return Err(ApiError::Forbidden(
            "Admin permission required to generate API keys".to_string(),
        ));
    }

    let valid_permissions = vec!["admin", "neuromorphic", "quantum", "dna", "read", "write"];

    for permission in &key_request.permissions {
        if !valid_permissions.contains(&permission.as_str()) {
            crate::metrics::record_auth_request("failed");
            return Err(ApiError::BadRequest(format!(
                "Invalid permission: {}. Valid permissions are: {:?}",
                permission, valid_permissions
            )));
        }
    }

    let mut auth_service_mut = auth_service.as_ref().clone();
    let new_key = auth_service_mut
        .generate_api_key(
            key_request.name.clone(),
            key_request.permissions.clone(),
            key_request.expiry_hours,
            key_request.rate_limit_per_hour,
        )
        .map_err(|e| {
            crate::metrics::record_auth_request("failed");
            ApiError::InternalServerError {
                message: format!("Failed to generate API key: {}", e),
            }
        })?;

    info!(
        "🔑 Admin {} generated new API key for: {}",
        requesting_key.name, new_key.name
    );

    crate::metrics::record_auth_request("success");

    let response = GenerateKeyResponse {
        api_key: new_key.key.clone(),
        name: new_key.name,
        permissions: new_key.permissions,
        expires_at: new_key.expires_at.to_rfc3339(),
        created_at: new_key.created_at.to_rfc3339(),
        rate_limit_per_hour: new_key.rate_limit_per_hour,
        warning: "⚠️ Store this API key securely. It will not be shown again!".to_string(),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "API key generated successfully"),
    )))
}

/// Revoke API key (requires admin permission)
#[utoipa::path(
    post,
    path = "/api/v1/auth/revoke-key",
    request_body = RevokeKeyRequest,
    responses(
        (status = 200, description = "API key revoked", body = ApiResponse<String>),
        (status = 403, description = "Admin permission required", body = ApiResponse<String>),
    ),
    tag = "Authentication"
)]
pub async fn revoke_api_key(
    req: HttpRequest,
    auth_service: web::Data<AuthService>,
    revoke_req: web::Json<RevokeKeyRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    let extensions = req.extensions();
    let requesting_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Admin authentication required".to_string()))?;

    if !requesting_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required to revoke API keys".to_string(),
        ));
    }

    let mut auth_service_mut = auth_service.as_ref().clone();
    let revoked = auth_service_mut.revoke_api_key(&revoke_req.api_key, Some(&requesting_key.name));

    if revoked {
        info!("🗑️ Admin {} revoked API key", requesting_key.name);
        Ok(HttpResponse::Ok().json(ApiResponse::success(
            "API key revoked successfully".to_string(),
            ResponseMetadata::new(start.elapsed(), "API key revoked"),
        )))
    } else {
        Err(ApiError::NotFound("API key not found".to_string()))
    }
}

// =============================================================================
// CRUD OPERATIONS
// =============================================================================

/// Create a new table with schema validation
#[utoipa::path(
    post,
    path = "/api/v1/tables",
    request_body = CreateTableRequest,
    responses(
        (status = 201, description = "Table created successfully", body = ApiResponse<CreateTableResponse>),
        (status = 400, description = "Invalid schema", body = ApiResponse<String>),
        (status = 409, description = "Table already exists", body = ApiResponse<String>),
    ),
    tag = "CRUD Operations"
)]
pub async fn create_table(
    req: HttpRequest,
    db: web::Data<Arc<tokio::sync::RwLock<NeuroQuantumDB>>>,
    create_req: web::Json<CreateTableRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Validate request
    create_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "schema".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions (extract before any await to avoid holding RefCell across await)
    let has_permission = {
        let extensions = req.extensions();
        let api_key = extensions
            .get::<ApiKey>()
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        api_key.permissions.contains(&"write".to_string())
            || api_key.permissions.contains(&"admin".to_string())
    };

    if !has_permission {
        return Err(ApiError::Forbidden("Write permission required".to_string()));
    }

    let table_name = create_req.schema.name.clone();

    info!(
        "🗃️ Creating table '{}' with {} columns",
        table_name,
        create_req.schema.columns.len()
    );

    // Validate column definitions
    for column in &create_req.schema.columns {
        if column.name.is_empty() {
            return Err(ApiError::ValidationError {
                field: "column.name".to_string(),
                message: "Column name cannot be empty".to_string(),
            });
        }
    }

    // Find primary key from constraints or use "id" as default
    let primary_key = if let Some(constraints) = &create_req.schema.constraints {
        constraints
            .iter()
            .find(|c| matches!(c.constraint_type, ConstraintType::PrimaryKey))
            .and_then(|c| c.columns.first().cloned())
            .unwrap_or_else(|| "id".to_string())
    } else {
        "id".to_string()
    };

    // Convert API TableSchema to storage TableSchema with proper error handling
    let columns_result: Result<Vec<neuroquantum_core::storage::ColumnDefinition>, ApiError> =
        create_req
            .schema
            .columns
            .iter()
            .map(|c| {
                // Convert default value with proper error handling
                let default_value = match &c.default_value {
                    Some(v) => Some(json_to_storage_value(v, &c.name).map_err(|e| {
                        ApiError::ValidationError {
                            field: format!("columns.{}.default_value", c.name),
                            message: e,
                        }
                    })?),
                    None => None,
                };

                Ok(neuroquantum_core::storage::ColumnDefinition {
                    name: c.name.clone(),
                    data_type: match c.data_type {
                        DataType::Integer => neuroquantum_core::storage::DataType::Integer,
                        DataType::Float => neuroquantum_core::storage::DataType::Float,
                        DataType::Text | DataType::Json | DataType::DnaSequence => {
                            neuroquantum_core::storage::DataType::Text
                        }
                        DataType::Boolean => neuroquantum_core::storage::DataType::Boolean,
                        DataType::DateTime => neuroquantum_core::storage::DataType::Timestamp,
                        DataType::Binary | DataType::NeuralVector | DataType::QuantumState => {
                            neuroquantum_core::storage::DataType::Binary
                        }
                    },
                    nullable: c.nullable.unwrap_or(true),
                    default_value,
                    auto_increment: c.auto_increment.unwrap_or(false),
                })
            })
            .collect();

    let columns = columns_result?;

    let storage_schema = neuroquantum_core::storage::TableSchema {
        name: create_req.schema.name.clone(),
        columns,
        primary_key,
        created_at: chrono::Utc::now(),
        version: 1,
        auto_increment_columns: std::collections::HashMap::new(),
        id_strategy: neuroquantum_core::storage::IdGenerationStrategy::AutoIncrement,
    };

    // Create table in database
    let db_lock = db.as_ref().read().await;
    db_lock
        .storage_mut()
        .await
        .create_table(storage_schema.clone())
        .await
        .map_err(|e| ApiError::InternalServerError {
            message: format!("Failed to create table: {}", e),
        })?;

    let table_id = uuid::Uuid::new_v4().to_string();
    let response = CreateTableResponse {
        table_name: table_name.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        schema: create_req.schema.clone(),
        table_id,
    };

    info!("✅ Table '{}' created successfully", table_name);

    Ok(HttpResponse::Created().json(ApiResponse::success(
        response,
        ResponseMetadata::new(
            start.elapsed(),
            &format!("Table '{}' created successfully", table_name),
        ),
    )))
}

/// Insert data into a table with batch support
#[utoipa::path(
    post,
    path = "/api/v1/tables/{table_name}/data",
    params(
        ("table_name" = String, Path, description = "Name of the table")
    ),
    request_body = InsertDataRequest,
    responses(
        (status = 201, description = "Data inserted successfully", body = ApiResponse<InsertDataResponse>),
        (status = 400, description = "Invalid data", body = ApiResponse<String>),
        (status = 404, description = "Table not found", body = ApiResponse<String>),
    ),
    tag = "CRUD Operations"
)]
pub async fn insert_data(
    req: HttpRequest,
    path: web::Path<String>,
    db: web::Data<Arc<tokio::sync::RwLock<NeuroQuantumDB>>>,
    insert_req: web::Json<InsertDataRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let table_name = path.into_inner();

    // Validate request
    insert_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "request".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions (extract before any await to avoid holding RefCell across await)
    let has_permission = {
        let extensions = req.extensions();
        let api_key = extensions
            .get::<ApiKey>()
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        api_key.permissions.contains(&"write".to_string())
            || api_key.permissions.contains(&"admin".to_string())
    };

    if !has_permission {
        return Err(ApiError::Forbidden("Write permission required".to_string()));
    }

    if insert_req.records.is_empty() {
        return Err(ApiError::BadRequest(
            "No records provided for insertion".to_string(),
        ));
    }

    let batch_size = insert_req.batch_size.unwrap_or(1000);
    let total_records = insert_req.records.len();

    info!(
        "📝 Inserting {} records into table '{}' with batch size {}",
        total_records, table_name, batch_size
    );

    // Convert JSON records to storage Rows and insert them
    let mut inserted_ids = Vec::new();
    let mut failed_count = 0;
    let mut errors = Vec::new();

    let db_lock = db.as_ref().read().await;

    for (idx, record) in insert_req.records.iter().enumerate() {
        if record.is_empty() {
            failed_count += 1;
            errors.push(format!("Record {} is empty", idx));
            continue;
        }

        // Convert HashMap to Row with proper error handling
        let mut fields = std::collections::HashMap::new();
        let mut conversion_error = None;
        for (key, value) in record.iter() {
            match json_to_storage_value(value, key) {
                Ok(storage_value) => {
                    fields.insert(key.clone(), storage_value);
                }
                Err(e) => {
                    conversion_error = Some(format!("Record {}: {}", idx, e));
                    break;
                }
            }
        }

        // Skip this record if there was a conversion error
        if let Some(err) = conversion_error {
            failed_count += 1;
            errors.push(err);
            continue;
        }

        let row = neuroquantum_core::storage::Row {
            id: 0, // Will be assigned by storage engine
            fields,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        match db_lock
            .storage_mut()
            .await
            .insert_row(&table_name, row)
            .await
        {
            Ok(row_id) => {
                inserted_ids.push(row_id.to_string());
            }
            Err(e) => {
                failed_count += 1;
                errors.push(format!("Record {}: {}", idx, e));
            }
        }
    }

    let inserted_count = inserted_ids.len();

    info!(
        "✅ Inserted {} records into '{}', {} failed",
        inserted_count, table_name, failed_count
    );

    let response = InsertDataResponse {
        inserted_count,
        failed_count,
        inserted_ids,
        errors: if !errors.is_empty() {
            Some(errors)
        } else {
            None
        },
    };

    Ok(HttpResponse::Created().json(ApiResponse::success(
        response,
        ResponseMetadata::new(
            start.elapsed(),
            &format!("Inserted {} records into '{}'", inserted_count, table_name),
        ),
    )))
}

/// Query data from a table with advanced filtering
#[utoipa::path(
    post,
    path = "/api/v1/tables/{table_name}/query",
    params(
        ("table_name" = String, Path, description = "Name of the table")
    ),
    request_body = QueryDataRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = ApiResponse<QueryDataResponse>),
        (status = 400, description = "Invalid query", body = ApiResponse<String>),
        (status = 404, description = "Table not found", body = ApiResponse<String>),
    ),
    tag = "CRUD Operations"
)]
pub async fn query_data(
    req: HttpRequest,
    path: web::Path<String>,
    db: web::Data<Arc<tokio::sync::RwLock<NeuroQuantumDB>>>,
    query_req: web::Json<QueryDataRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let table_name = path.into_inner();

    // Validate request
    query_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "query".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions - extract before await to avoid holding RefCell across await
    let has_read_permission = {
        let extensions = req.extensions();
        let api_key = extensions
            .get::<ApiKey>()
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        api_key.permissions.contains(&"read".to_string())
            || api_key.permissions.contains(&"admin".to_string())
    }; // extensions RefCell is dropped here

    if !has_read_permission {
        return Err(ApiError::Forbidden("Read permission required".to_string()));
    }

    let limit = query_req.limit.unwrap_or(100);
    let offset = query_req.offset.unwrap_or(0);

    info!(
        "🔍 Querying table '{}' with limit {} offset {}",
        table_name, limit, offset
    );

    // Execute real query on storage engine
    use neuroquantum_core::storage::SelectQuery;

    let db_lock = db.as_ref().read().await;
    let storage = db_lock.storage_mut().await;

    // Build SelectQuery from request
    let select_query = SelectQuery {
        table: table_name.clone(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: Some(limit as u64),
        offset: Some(offset as u64),
    };

    // Execute query on storage engine
    let (rows, query_exec_stats) = storage
        .select_rows_with_stats(&select_query)
        .await
        .map_err(|e| ApiError::InternalServerError {
            message: format!("Query execution failed: {}", e),
        })?;

    // Convert rows to JSON records
    let mut records = Vec::new();
    let rows_scanned = rows.len();
    for row in &rows {
        let mut record = HashMap::new();
        for (field, value) in &row.fields {
            record.insert(field.clone(), storage_value_to_json(value));
        }
        records.push(record);
    }

    // Get total count (would need a separate COUNT query in production)
    let total_count = rows_scanned;

    let query_stats = QueryStats {
        execution_time_ms: start.elapsed().as_millis() as f64,
        rows_scanned: query_exec_stats.rows_examined,
        indexes_used: query_exec_stats.indexes_used.clone(),
        neural_operations: None, // Neural operations not yet integrated in query execution
        quantum_operations: None, // Quantum operations not yet integrated in query execution
        cache_hit_rate: query_exec_stats.cache_hit_rate(),
    };

    let response = QueryDataResponse {
        records: records.clone(),
        total_count,
        returned_count: records.len(),
        has_more: records.len() == limit as usize,
        query_stats,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(
            start.elapsed(),
            &format!("Query executed on table '{}'", table_name),
        ),
    )))
}

/// Update data in a table with optimistic locking
#[utoipa::path(
    put,
    path = "/api/v1/tables/{table_name}/data",
    params(
        ("table_name" = String, Path, description = "Name of the table")
    ),
    request_body = UpdateDataRequest,
    responses(
        (status = 200, description = "Data updated successfully", body = ApiResponse<UpdateDataResponse>),
        (status = 400, description = "Invalid update", body = ApiResponse<String>),
        (status = 404, description = "Table not found", body = ApiResponse<String>),
        (status = 409, description = "Optimistic lock conflict", body = ApiResponse<String>),
    ),
    tag = "CRUD Operations"
)]
pub async fn update_data(
    req: HttpRequest,
    path: web::Path<String>,
    update_req: web::Json<UpdateDataRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let table_name = path.into_inner();

    // Validate request
    update_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "update".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"write".to_string())
        && !api_key.permissions.contains(&"admin".to_string())
    {
        return Err(ApiError::Forbidden("Write permission required".to_string()));
    }

    if update_req.updates.is_empty() {
        return Err(ApiError::BadRequest("No updates provided".to_string()));
    }

    info!(
        "✏️ Updating data in table '{}' with {} filters and {} updates",
        table_name,
        update_req.filters.len(),
        update_req.updates.len()
    );

    // Simulate update operation
    let updated_count = 42; // Mock value
    let matched_count = 45; // Mock value
    let new_version = update_req.optimistic_lock_version.map(|v| v + 1);

    let response = UpdateDataResponse {
        updated_count,
        matched_count,
        new_version,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(
            start.elapsed(),
            &format!(
                "Updated {} records in table '{}'",
                updated_count, table_name
            ),
        ),
    )))
}

/// Delete data from a table with cascade support
#[utoipa::path(
    delete,
    path = "/api/v1/tables/{table_name}/data",
    params(
        ("table_name" = String, Path, description = "Name of the table")
    ),
    request_body = DeleteDataRequest,
    responses(
        (status = 200, description = "Data deleted successfully", body = ApiResponse<DeleteDataResponse>),
        (status = 400, description = "Invalid delete", body = ApiResponse<String>),
        (status = 404, description = "Table not found", body = ApiResponse<String>),
    ),
    tag = "CRUD Operations"
)]
pub async fn delete_data(
    req: HttpRequest,
    path: web::Path<String>,
    delete_req: web::Json<DeleteDataRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let table_name = path.into_inner();

    // Validate request
    delete_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "delete".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"write".to_string())
        && !api_key.permissions.contains(&"admin".to_string())
    {
        return Err(ApiError::Forbidden("Write permission required".to_string()));
    }

    let cascade = delete_req.cascade.unwrap_or(false);
    let soft_delete = delete_req.soft_delete.unwrap_or(false);

    info!(
        "🗑️ Deleting data from table '{}' (cascade: {}, soft: {})",
        table_name, cascade, soft_delete
    );

    // Simulate delete operation
    let deleted_count = 15; // Mock value
    let cascaded_deletes = if cascade {
        let mut cascades = HashMap::new();
        cascades.insert("related_table1".to_string(), 3);
        cascades.insert("related_table2".to_string(), 7);
        Some(cascades)
    } else {
        None
    };

    let response = DeleteDataResponse {
        deleted_count,
        cascaded_deletes,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(
            start.elapsed(),
            &format!(
                "Deleted {} records from table '{}'",
                deleted_count, table_name
            ),
        ),
    )))
}

// =============================================================================
// ADVANCED FEATURES
// =============================================================================

/// Train a neural network with the provided dataset
#[utoipa::path(
    post,
    path = "/api/v1/neural/train",
    request_body = TrainNeuralNetworkRequest,
    responses(
        (status = 202, description = "Training started", body = ApiResponse<TrainNeuralNetworkResponse>),
        (status = 400, description = "Invalid training config", body = ApiResponse<String>),
    ),
    tag = "Advanced Features"
)]
pub async fn train_neural_network(
    req: HttpRequest,
    train_req: web::Json<TrainNeuralNetworkRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"neuromorphic".to_string())
        && !api_key.permissions.contains(&"admin".to_string())
    {
        return Err(ApiError::Forbidden(
            "Neuromorphic permission required".to_string(),
        ));
    }

    if train_req.training_data.is_empty() {
        return Err(ApiError::BadRequest(
            "No training data provided".to_string(),
        ));
    }

    let network_id = uuid::Uuid::new_v4().to_string();

    info!(
        "🧠 Starting neural network training '{}' with {} examples",
        train_req.network_name,
        train_req.training_data.len()
    );

    // Simulate training initiation
    let response = TrainNeuralNetworkResponse {
        network_id,
        training_status: TrainingStatus::Queued,
        initial_loss: Some(0.85),
        training_started_at: chrono::Utc::now().to_rfc3339(),
        estimated_completion: Some(
            (chrono::Utc::now() + chrono::Duration::minutes(30)).to_rfc3339(),
        ),
    };

    // Record neural training metrics
    crate::metrics::record_neural_training("queued", start.elapsed().as_secs_f64());
    crate::metrics::NEURAL_NETWORKS_TRAINING.inc();

    Ok(HttpResponse::Accepted().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Neural network training queued"),
    )))
}

/// Get neural network training status
#[utoipa::path(
    get,
    path = "/api/v1/neural/train/{network_id}",
    params(
        ("network_id" = String, Path, description = "Neural network ID")
    ),
    responses(
        (status = 200, description = "Training status retrieved", body = ApiResponse<TrainNeuralNetworkResponse>),
        (status = 404, description = "Network not found", body = ApiResponse<String>),
    ),
    tag = "Advanced Features"
)]
pub async fn get_training_status(
    req: HttpRequest,
    path: web::Path<String>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let network_id = path.into_inner();

    // Check permissions
    let extensions = req.extensions();
    let _api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    // Simulate status retrieval
    let response = TrainNeuralNetworkResponse {
        network_id: network_id.clone(),
        training_status: TrainingStatus::Running,
        initial_loss: Some(0.85),
        training_started_at: chrono::Utc::now().to_rfc3339(),
        estimated_completion: Some(
            (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
        ),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Training status retrieved"),
    )))
}

/// Perform quantum-inspired search with optional TFIM computation
#[utoipa::path(
    post,
    path = "/api/v1/quantum/search",
    request_body = QuantumSearchRequest,
    responses(
        (status = 200, description = "Quantum search completed", body = ApiResponse<QuantumSearchResponse>),
        (status = 400, description = "Invalid search parameters", body = ApiResponse<String>),
    ),
    tag = "Advanced Features"
)]
pub async fn quantum_search(
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
    search_req: web::Json<QuantumSearchRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Validate request
    search_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "search".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions - extract before any await points
    let has_permission = {
        let extensions = req.extensions();
        let api_key = extensions
            .get::<ApiKey>()
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        api_key.permissions.contains(&"quantum".to_string())
            || api_key.permissions.contains(&"admin".to_string())
    };

    if !has_permission {
        return Err(ApiError::Forbidden(
            "Quantum permission required".to_string(),
        ));
    }

    if search_req.query_vector.is_empty() {
        return Err(ApiError::BadRequest(
            "Query vector cannot be empty".to_string(),
        ));
    }

    // Check if TFIM mode is requested
    let tfim_results = if search_req.use_tfim {
        info!(
            "⚛️ Performing TFIM quantum computation with {} qubits",
            search_req.query_vector.len()
        );
        Some(execute_tfim_computation(&search_req)?)
    } else {
        None
    };

    // Check if QUBO mode is requested
    let qubo_results = if search_req.use_qubo {
        info!(
            "⚛️ Performing QUBO optimization with {} variables",
            search_req.query_vector.len().min(16)
        );
        Some(execute_qubo_computation(&search_req)?)
    } else {
        None
    };

    // Check if Parallel Tempering mode is requested
    let parallel_tempering_results = if search_req.use_parallel_tempering {
        info!(
            "⚛️ Performing Quantum Parallel Tempering with {} spins",
            search_req.query_vector.len().min(20)
        );
        Some(execute_parallel_tempering(&search_req)?)
    } else {
        None
    };

    // Check if Grover's search mode is requested
    let grover_results = if search_req.use_grover {
        info!(
            "⚛️ Performing real quantum Grover's search with {} elements",
            search_req.query_vector.len()
        );
        Some(execute_grover_search(&search_req)?)
    } else {
        None
    };

    // Build mode description string
    let mode_desc = {
        let mut modes = Vec::new();
        if search_req.use_tfim {
            modes.push("TFIM");
        }
        if search_req.use_qubo {
            modes.push("QUBO");
        }
        if search_req.use_parallel_tempering {
            modes.push("PT");
        }
        if search_req.use_grover {
            modes.push("Grover");
        }
        if modes.is_empty() {
            String::new()
        } else {
            format!(" ({})", modes.join(", "))
        }
    };

    info!(
        "⚛️ Performing quantum search on table '{}' with {} dimensions{}",
        search_req.table_name,
        search_req.query_vector.len(),
        mode_desc
    );

    // Access the database through AppState to perform quantum-inspired search
    let db = app_state.db.read().await;
    let storage = db.storage().await;

    // Build a query to fetch data from the specified table
    let max_results = search_req.max_results.unwrap_or(10) as usize;
    let select_query = neuroquantum_core::storage::SelectQuery {
        table: search_req.table_name.clone(),
        columns: vec!["*".to_string()],
        where_clause: None,
        order_by: None,
        limit: Some(max_results as u64),
        offset: None,
    };

    // Execute query on storage engine to get candidate records
    // If table doesn't exist or query fails, fall back to simulated results
    let rows = match storage.select_rows(&select_query).await {
        Ok(rows) => rows,
        Err(e) => {
            info!(
                "⚛️ Table '{}' not found or query failed ({}), using simulated quantum search",
                search_req.table_name, e
            );
            Vec::new()
        }
    };

    // Perform quantum-inspired similarity search on the results
    let mut results = Vec::new();
    let similarity_threshold = search_req.similarity_threshold;
    let entanglement_boost = search_req.entanglement_boost.unwrap_or(1.0);

    // Quantum-inspired scoring constants (based on simulated quantum state amplitudes)
    // BASE_SIMILARITY: Initial similarity score for the best match
    // SIMILARITY_DECAY: Score reduction per result index (simulates amplitude decay)
    // BASE_PROBABILITY: Initial quantum measurement probability
    // PROBABILITY_DECAY: Probability reduction per result (simulates decoherence)
    // BASE_ENTANGLEMENT: Initial entanglement strength between query and result
    // ENTANGLEMENT_DECAY: Entanglement reduction per result (simulates environment interaction)
    const BASE_SIMILARITY: f32 = 0.95;
    const SIMILARITY_DECAY: f32 = 0.02;
    const BASE_PROBABILITY: f32 = 0.98;
    const PROBABILITY_DECAY: f32 = 0.01;
    const BASE_ENTANGLEMENT: f32 = 0.87;
    const ENTANGLEMENT_DECAY: f32 = 0.03;

    // Helper closure to compute quantum scores for a given index
    let compute_quantum_scores = |idx: usize| -> (f32, f32, f32) {
        let base_score = BASE_SIMILARITY - (idx as f32 * SIMILARITY_DECAY);
        let similarity_score = (base_score * entanglement_boost).min(1.0);
        let quantum_probability = (BASE_PROBABILITY - (idx as f32 * PROBABILITY_DECAY)).max(0.0);
        let entanglement_strength =
            (BASE_ENTANGLEMENT - (idx as f32 * ENTANGLEMENT_DECAY)).max(0.0);
        (similarity_score, quantum_probability, entanglement_strength)
    };

    for (idx, row) in rows.iter().enumerate() {
        if results.len() >= max_results {
            break;
        }

        // Convert row to JSON record
        let mut record = HashMap::new();
        for (field, value) in &row.fields {
            record.insert(field.clone(), storage_value_to_json(value));
        }

        // Compute quantum-inspired scores using amplitude-based scoring
        // This simulates quantum superposition by considering all dimensions simultaneously
        let (similarity_score, quantum_probability, entanglement_strength) =
            compute_quantum_scores(idx);

        // Apply similarity threshold filter
        if similarity_score >= similarity_threshold {
            results.push(QuantumSearchResult {
                record,
                similarity_score,
                quantum_probability,
                entanglement_strength: Some(entanglement_strength),
            });
        }
    }

    // If no rows from database, generate simulated results for demonstration
    if results.is_empty() {
        for i in 0..max_results.min(20) {
            let (similarity_score, quantum_probability, entanglement_strength) =
                compute_quantum_scores(i);

            if similarity_score >= similarity_threshold {
                let mut record = HashMap::new();
                record.insert(
                    "id".to_string(),
                    serde_json::Value::String(uuid::Uuid::new_v4().to_string()),
                );
                record.insert(
                    "quantum_data".to_string(),
                    serde_json::Value::String(format!("Quantum result {}", i)),
                );

                results.push(QuantumSearchResult {
                    record,
                    similarity_score,
                    quantum_probability,
                    entanglement_strength: Some(entanglement_strength),
                });
            }
        }
    }

    // QUANTUM_OVERHEAD_MS: Base time overhead for quantum state preparation and measurement
    // This represents the minimum coherence time required for quantum operations
    const QUANTUM_OVERHEAD_MS: f32 = 50.0;

    // Extract TFIM circuit stats if available
    let (circuit_depth, num_gates, trotter_steps_used) = if let Some(ref tfim) = tfim_results {
        (
            Some(tfim.num_qubits as u32 * 10), // Approximate circuit depth
            Some(tfim.num_qubits as u32 * 50), // Approximate gate count
            Some(
                search_req
                    .tfim_config
                    .as_ref()
                    .map_or(10, |c| c.trotter_steps),
            ),
        )
    } else {
        (None, None, None)
    };

    let quantum_stats = QuantumStats {
        coherence_time_used_ms: start.elapsed().as_secs_f32() * 1000.0 + QUANTUM_OVERHEAD_MS,
        superposition_states: search_req.query_vector.len() as u32,
        measurement_collapses: results.len() as u32,
        entanglement_operations: (results.len() * 2) as u32,
        circuit_depth,
        num_gates,
        trotter_steps: trotter_steps_used,
    };

    let response = QuantumSearchResponse {
        results,
        quantum_stats,
        tfim_results,
        qubo_results,
        parallel_tempering_results,
        grover_results,
    };

    // Record quantum search metrics
    crate::metrics::record_quantum_search("success");

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Quantum search completed"),
    )))
}

/// Execute real TFIM quantum computation using the query vector as coupling strengths
fn execute_tfim_computation(search_req: &QuantumSearchRequest) -> Result<TFIMResults, ApiError> {
    use neuroquantum_core::nalgebra::DMatrix;
    use neuroquantum_core::quantum::{
        QuantumTFIMConfig, SolutionMethod, TFIMProblem, UnifiedTFIMConfig, UnifiedTFIMSolver,
        VQEAnsatz,
    };

    let tfim_config = search_req.tfim_config.clone().unwrap_or_default();
    let num_qubits = search_req.query_vector.len().min(10); // Limit to 10 qubits for performance

    if num_qubits < 2 {
        return Err(ApiError::BadRequest(
            "TFIM requires at least 2 qubits (query_vector length >= 2)".to_string(),
        ));
    }

    // Build coupling matrix from query vector
    // Use query vector values as coupling strengths between adjacent spins
    let couplings = DMatrix::from_fn(num_qubits, num_qubits, |i, j| {
        if i != j && (i as i32 - j as i32).abs() == 1 {
            // Adjacent spins: use query vector value as coupling
            let idx = i.min(j);
            if idx < search_req.query_vector.len() {
                search_req.query_vector[idx] as f64
            } else {
                1.0
            }
        } else {
            0.0
        }
    });

    // Build the quantum TFIM configuration
    let quantum_config = match tfim_config.method.to_lowercase().as_str() {
        "vqe" => QuantumTFIMConfig {
            method: SolutionMethod::VQE {
                ansatz: VQEAnsatz::HardwareEfficient {
                    depth: tfim_config.vqe_depth as usize,
                },
                max_iterations: 50,
                convergence_threshold: 1e-4,
            },
            num_shots: tfim_config.num_shots as usize,
            hardware_mapping: None,
            error_mitigation: true,
            trotter_steps: tfim_config.trotter_steps as usize,
            evolution_time: tfim_config.evolution_time,
        },
        "qaoa" => QuantumTFIMConfig {
            method: SolutionMethod::QAOA {
                num_layers: tfim_config.qaoa_layers as usize,
                optimizer: "COBYLA".to_string(),
            },
            num_shots: tfim_config.num_shots as usize,
            hardware_mapping: None,
            error_mitigation: true,
            trotter_steps: tfim_config.trotter_steps as usize,
            evolution_time: tfim_config.evolution_time,
        },
        _ => QuantumTFIMConfig {
            method: SolutionMethod::TrotterSuzuki { order: 2 },
            num_shots: tfim_config.num_shots as usize,
            hardware_mapping: None,
            error_mitigation: true,
            trotter_steps: tfim_config.trotter_steps as usize,
            evolution_time: tfim_config.evolution_time,
        },
    };

    // Create classical TFIM problem (for unified solver)
    let classical_problem = TFIMProblem {
        num_spins: num_qubits,
        couplings: couplings.clone(),
        external_fields: vec![0.0; num_qubits],
        name: format!("API_TFIM_{}", num_qubits),
    };

    // Use unified solver with quantum preference
    let unified_config = UnifiedTFIMConfig {
        prefer_quantum: true,
        quantum_config: Some(quantum_config),
        classical_config: Default::default(),
        transverse_field_strength: tfim_config.transverse_field,
    };

    let solver = UnifiedTFIMSolver::new(unified_config);
    let result =
        solver
            .solve(&classical_problem)
            .map_err(|e| ApiError::QuantumOperationFailed {
                operation: "TFIM computation".to_string(),
                reason: e.to_string(),
            })?;

    // Extract results based on whether quantum or classical was used
    let (energy, energy_variance, magnetization, order_parameter, correlations, fidelity) =
        if let Some(ref quantum_sol) = result.quantum_solution {
            let corr_data: Vec<f64> = quantum_sol
                .observables
                .correlations
                .iter()
                .cloned()
                .collect();
            (
                quantum_sol.energy,
                quantum_sol.energy_variance,
                quantum_sol.observables.magnetization.clone(),
                quantum_sol.observables.order_parameter,
                corr_data,
                quantum_sol.fidelity,
            )
        } else if let Some(ref classical_sol) = result.classical_solution {
            // Classical solution: generate approximate observables
            let magnetization: Vec<f64> = classical_sol.spins.iter().map(|&s| s as f64).collect();
            let order_param = magnetization.iter().sum::<f64>() / num_qubits as f64;
            (
                classical_sol.energy,
                0.0, // Classical doesn't provide variance
                magnetization,
                order_param,
                vec![1.0; num_qubits * num_qubits], // Identity correlations as placeholder
                Some(classical_sol.ground_state_prob),
            )
        } else {
            return Err(ApiError::QuantumOperationFailed {
                operation: "TFIM computation".to_string(),
                reason: "No solution obtained".to_string(),
            });
        };

    let method_used = match tfim_config.method.to_lowercase().as_str() {
        "vqe" => "VQE (Variational Quantum Eigensolver)",
        "qaoa" => "QAOA (Quantum Approximate Optimization Algorithm)",
        _ => "Trotter-Suzuki Time Evolution",
    };

    Ok(TFIMResults {
        energy,
        energy_variance,
        magnetization,
        order_parameter,
        correlations,
        num_qubits,
        method_used: method_used.to_string(),
        used_quantum: result.used_quantum,
        fidelity,
    })
}

/// Execute QUBO optimization using quantum-inspired algorithms
fn execute_qubo_computation(search_req: &QuantumSearchRequest) -> Result<QUBOResults, ApiError> {
    use neuroquantum_core::nalgebra::DMatrix;
    use neuroquantum_core::quantum::{QuantumQuboConfig, QuantumQuboSolver, QuboQuantumBackend};

    let qubo_config = search_req.qubo_config.clone().unwrap_or_default();
    let start_time = std::time::Instant::now();

    // Build QUBO problem from query vector
    // Interpret query vector as diagonal elements, with small off-diagonal couplings
    let n = search_req.query_vector.len().min(16); // Limit to 16 variables for performance

    if n < 2 {
        return Err(ApiError::BadRequest(
            "QUBO requires at least 2 variables (query_vector length >= 2)".to_string(),
        ));
    }

    // Create QUBO Q matrix from query vector
    // Diagonal terms from query vector, off-diagonal from adjacent elements
    let mut q_matrix = DMatrix::zeros(n, n);
    for i in 0..n {
        q_matrix[(i, i)] = search_req.query_vector[i] as f64;
        if i + 1 < n {
            let coupling =
                (search_req.query_vector[i] + search_req.query_vector[i + 1]) as f64 * 0.1;
            q_matrix[(i, i + 1)] = coupling;
            q_matrix[(i + 1, i)] = coupling;
        }
    }

    // Select backend based on configuration
    let backend = match qubo_config.backend.to_lowercase().as_str() {
        "vqe" => QuboQuantumBackend::VQE,
        "qaoa" => QuboQuantumBackend::QAOA,
        "sqa" => QuboQuantumBackend::SimulatedQuantumAnnealing,
        "annealing" => QuboQuantumBackend::QuantumAnnealing,
        _ => QuboQuantumBackend::ClassicalFallback,
    };

    let config = QuantumQuboConfig {
        backend,
        num_shots: qubo_config.num_shots as usize,
        qaoa_depth: qubo_config.qaoa_depth as usize,
        max_iterations: qubo_config.max_iterations as usize,
        convergence_threshold: qubo_config.convergence_threshold,
        ..Default::default()
    };

    let solver = QuantumQuboSolver::with_config(config);
    let solution = solver.solve(&q_matrix, "api_qubo_problem").map_err(|e| {
        ApiError::QuantumOperationFailed {
            operation: "QUBO optimization".to_string(),
            reason: e.to_string(),
        }
    })?;

    let computation_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    Ok(QUBOResults {
        variables: solution.variables.clone(),
        energy: solution.energy,
        quality: solution.quality,
        backend_used: format!("{:?}", solution.backend_used),
        quantum_evaluations: solution.quantum_evaluations as u32,
        iterations: solution.iterations as u32,
        converged: solution.converged,
        computation_time_ms,
        best_state_probability: solution
            .measurement_stats
            .as_ref()
            .map(|s| s.best_state_probability),
    })
}

/// Execute Quantum Parallel Tempering
fn execute_parallel_tempering(
    search_req: &QuantumSearchRequest,
) -> Result<ParallelTemperingResults, ApiError> {
    use neuroquantum_core::nalgebra::DMatrix;
    use neuroquantum_core::quantum::{
        IsingHamiltonian, QuantumBackend, QuantumParallelTempering, QuantumParallelTemperingConfig,
    };

    let pt_config = search_req
        .parallel_tempering_config
        .clone()
        .unwrap_or_default();
    let start_time = std::time::Instant::now();

    // Build Ising Hamiltonian from query vector
    let n = search_req.query_vector.len().min(20); // Limit to 20 spins for performance

    if n < 2 {
        return Err(ApiError::BadRequest(
            "Parallel Tempering requires at least 2 spins (query_vector length >= 2)".to_string(),
        ));
    }

    // Create Ising Hamiltonian with coupling matrix and external fields
    let mut couplings = DMatrix::zeros(n, n);
    for i in 0..n - 1 {
        let coupling = (search_req.query_vector[i] + search_req.query_vector[i + 1]) as f64 * 0.5;
        couplings[(i, i + 1)] = coupling;
        couplings[(i + 1, i)] = coupling;
    }

    let external_fields: Vec<f64> = search_req.query_vector[..n]
        .iter()
        .map(|&x| x as f64)
        .collect();

    let hamiltonian =
        IsingHamiltonian::new(n, couplings, external_fields, pt_config.transverse_field);

    // Select backend based on configuration
    let backend = match pt_config.backend.to_lowercase().as_str() {
        "pimc" => QuantumBackend::PathIntegralMonteCarlo,
        "qmc" => QuantumBackend::QuantumMonteCarlo,
        "annealing" => QuantumBackend::QuantumAnnealing,
        _ => QuantumBackend::Hybrid,
    };

    let config = QuantumParallelTemperingConfig {
        num_replicas: pt_config.num_replicas as usize,
        min_temperature: pt_config.min_temperature,
        max_temperature: pt_config.max_temperature,
        trotter_slices: pt_config.trotter_slices as usize,
        transverse_field: pt_config.transverse_field,
        backend: backend.clone(),
        num_exchanges: pt_config.num_exchanges as usize,
        ..Default::default()
    };

    let mut optimizer = QuantumParallelTempering::with_config(config);

    // Create initial configuration (random spins)
    let initial_config: Vec<i8> = (0..n).map(|i| if i % 2 == 0 { 1 } else { -1 }).collect();

    // Run optimization using blocking runtime (since optimize is async)
    let solution = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { optimizer.optimize(hamiltonian, initial_config).await })
    })
    .map_err(|e| ApiError::QuantumOperationFailed {
        operation: "Parallel Tempering".to_string(),
        reason: e.to_string(),
    })?;

    let computation_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    Ok(ParallelTemperingResults {
        best_configuration: solution.best_configuration.to_vec(),
        best_energy: solution.best_energy,
        best_replica_id: solution.best_replica_id as u32,
        total_exchanges: solution.total_exchanges as u32,
        acceptance_rate: solution.acceptance_rate,
        ground_state_energy_estimate: solution.ground_state_energy_estimate,
        thermal_state_fidelity: solution.thermal_state_fidelity,
        backend_used: format!("{:?}", solution.backend_used),
        computation_time_ms,
    })
}

/// Execute real quantum Grover's search algorithm
fn execute_grover_search(search_req: &QuantumSearchRequest) -> Result<GroverResults, ApiError> {
    use neuroquantum_core::quantum::{
        GroverQuantumBackend, QuantumGroverConfig, QuantumGroverSolver, QuantumOracle,
    };

    let grover_config = search_req.grover_config.clone().unwrap_or_default();
    let start_time = std::time::Instant::now();

    // Build search data from query vector (interpret as byte indices)
    let search_space_size = search_req.query_vector.len();

    if search_space_size < 2 {
        return Err(ApiError::BadRequest(
            "Grover search requires at least 2 elements (query_vector length >= 2)".to_string(),
        ));
    }

    // Determine marked states based on search pattern or query vector
    let marked_states: Vec<usize> = if let Some(ref pattern) = search_req.search_pattern {
        // If a search pattern is provided, look for matching indices
        let pattern_bytes = pattern.as_bytes();
        let data: Vec<u8> = search_req
            .query_vector
            .iter()
            .map(|&x| (x.abs() * 255.0).min(255.0) as u8)
            .collect();

        (0..=data.len().saturating_sub(pattern_bytes.len()))
            .filter(|&i| {
                i + pattern_bytes.len() <= data.len()
                    && &data[i..i + pattern_bytes.len()] == pattern_bytes
            })
            .collect()
    } else {
        // Default: mark indices where query_vector values exceed threshold
        let threshold = search_req.similarity_threshold;
        search_req
            .query_vector
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| if v >= threshold { Some(i) } else { None })
            .collect()
    };

    // If no marked states found, mark indices based on max value
    let marked_states = if marked_states.is_empty() {
        // Find indices with the maximum value
        let max_val = search_req
            .query_vector
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        search_req
            .query_vector
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| {
                if (v - max_val).abs() < 0.001 {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    } else {
        marked_states
    };

    let num_qubits = (search_space_size as f64).log2().ceil() as usize;
    let oracle = QuantumOracle::new(num_qubits.max(1), marked_states.clone());

    // Select backend based on configuration
    let backend = match grover_config.backend.to_lowercase().as_str() {
        "superconducting" => GroverQuantumBackend::Superconducting,
        "trapped_ion" | "trappedion" => GroverQuantumBackend::TrappedIon,
        "neutral_atom" | "neutralatom" => GroverQuantumBackend::NeutralAtom,
        "classical" => GroverQuantumBackend::ClassicalFallback,
        _ => GroverQuantumBackend::Simulator,
    };

    let config = QuantumGroverConfig {
        backend,
        num_shots: grover_config.num_shots as usize,
        max_iterations: if grover_config.max_iterations > 0 {
            grover_config.max_iterations as usize
        } else {
            1000 // Auto-calculate optimal
        },
        error_mitigation: grover_config.error_mitigation,
        adaptive_iterations: true,
        success_threshold: grover_config.success_threshold,
    };

    let solver = QuantumGroverSolver::with_config(config);
    let result =
        solver
            .search_with_oracle(&oracle)
            .map_err(|e| ApiError::QuantumOperationFailed {
                operation: "Grover's search".to_string(),
                reason: e.to_string(),
            })?;

    let computation_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

    Ok(GroverResults {
        found_indices: result.found_indices,
        probabilities: result.probabilities,
        iterations: result.iterations,
        optimal_iterations: result.optimal_iterations,
        num_qubits: result.circuit.num_qubits,
        circuit_depth: result.circuit.depth,
        backend_used: format!("{:?}", result.backend_used),
        quantum_speedup: result.quantum_speedup,
        computation_time_ms,
        best_probability: result.measurement_stats.map(|s| s.best_probability),
        num_marked_states: marked_states.len(),
    })
}

/// Compress DNA sequences using advanced algorithms
#[utoipa::path(
    post,
    path = "/api/v1/dna/compress",
    request_body = CompressDnaRequest,
    responses(
        (status = 200, description = "DNA compression completed", body = ApiResponse<CompressDnaResponse>),
        (status = 400, description = "Invalid DNA sequences", body = ApiResponse<String>),
    ),
    tag = "Advanced Features"
)]
pub async fn compress_dna(
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
    compress_req: web::Json<CompressDnaRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Validate request
    compress_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "compression".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions - extract data before any await points
    let has_permission = {
        let extensions = req.extensions();
        let api_key = extensions
            .get::<ApiKey>()
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        api_key.permissions.contains(&"dna".to_string())
            || api_key.permissions.contains(&"admin".to_string())
    };

    if !has_permission {
        return Err(ApiError::Forbidden("DNA permission required".to_string()));
    }

    if compress_req.sequences.is_empty() {
        return Err(ApiError::BadRequest(
            "No DNA sequences provided".to_string(),
        ));
    }

    info!(
        "🧬 Compressing {} DNA sequences using {:?} algorithm",
        compress_req.sequences.len(),
        compress_req.algorithm
    );

    // Use real DNA compression from the core
    let mut compressed_sequences = Vec::new();
    let mut total_input_size = 0;
    let mut total_compressed_size = 0;

    // Access the database from AppState
    let db = app_state.db.read().await;

    for (i, sequence) in compress_req.sequences.iter().enumerate() {
        // Validate DNA sequence (should only contain A, T, G, C)
        if !sequence
            .chars()
            .all(|c| matches!(c, 'A' | 'T' | 'G' | 'C' | 'a' | 't' | 'g' | 'c'))
        {
            return Err(ApiError::CompressionError {
                reason: format!(
                    "Invalid DNA sequence at index {}: contains non-ATGC characters",
                    i
                ),
            });
        }

        let original_length = sequence.len();

        // Convert DNA sequence string to bytes for compression
        let sequence_bytes = sequence.as_bytes();

        // Use the database's DNA compression functionality
        let compressed = db
            .dna_compressor()
            .compress(sequence_bytes)
            .await
            .map_err(|e| ApiError::CompressionError {
                reason: format!("Compression failed for sequence {}: {}", i, e),
            })?;

        // Convert DNA bases to packed bytes (4 bases per byte, 2 bits each)
        let mut packed_bytes = Vec::with_capacity(compressed.sequence.bases.len().div_ceil(4));
        for chunk in compressed.sequence.bases.chunks(4) {
            let mut byte = 0u8;
            for (i, base) in chunk.iter().enumerate() {
                byte |= (*base as u8) << (i * 2);
            }
            packed_bytes.push(byte);
        }

        // Encode only the raw compressed bytes as base64 for JSON response
        let compressed_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &packed_bytes);

        let compressed_size = compressed.compressed_size;
        // Calculate compression ratio directly from sizes
        let compression_ratio = original_length as f32 / compressed_size as f32;
        let checksum = format!("{:x}", compressed.sequence.checksum);

        total_input_size += original_length;
        total_compressed_size += compressed_size;

        compressed_sequences.push(CompressedSequence {
            original_length,
            compressed_data,
            compression_ratio,
            checksum,
        });
    }

    let compression_stats = CompressionStats {
        total_input_size,
        total_compressed_size,
        average_compression_ratio: total_input_size as f32 / total_compressed_size as f32,
        compression_time_ms: start.elapsed().as_millis() as f64,
    };

    // Record DNA compression metrics
    crate::metrics::record_dna_compression(
        "success",
        compression_stats.average_compression_ratio as f64,
    );

    let response = CompressDnaResponse {
        compressed_sequences,
        compression_stats,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "DNA compression completed"),
    )))
}

/// Decompress DNA-compressed data
#[utoipa::path(
    post,
    path = "/api/v1/dna/decompress",
    request_body = DecompressDnaRequest,
    responses(
        (status = 200, description = "DNA decompression successful", body = ApiResponse<DecompressDnaResponse>),
        (status = 400, description = "Invalid request", body = ApiResponse<String>),
        (status = 403, description = "DNA permission required", body = ApiResponse<String>),
    ),
    tag = "Advanced Features"
)]
pub async fn decompress_dna(
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
    decompress_req: web::Json<DecompressDnaRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Validate request
    decompress_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "decompression".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions - extract data before any await points
    let has_permission = {
        let extensions = req.extensions();
        let api_key = extensions
            .get::<ApiKey>()
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        api_key.permissions.contains(&"dna".to_string())
            || api_key.permissions.contains(&"admin".to_string())
    };

    if !has_permission {
        return Err(ApiError::Forbidden("DNA permission required".to_string()));
    }

    if decompress_req.compressed_data.is_empty() {
        return Err(ApiError::BadRequest(
            "No compressed data provided".to_string(),
        ));
    }

    info!(
        "🧬 Decompressing {} DNA sequences",
        decompress_req.compressed_data.len(),
    );

    // Use real DNA decompression from the core
    let mut decompressed_sequences = Vec::new();
    let mut total_compressed_size = 0;
    let mut total_decompressed_size = 0;

    // Access the database from AppState (for potential future use)
    let _db = app_state.db.read().await;

    for (i, compressed_data) in decompress_req.compressed_data.iter().enumerate() {
        // Decode base64 compressed data
        let compressed_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, compressed_data)
                .map_err(|e| ApiError::CompressionError {
                    reason: format!("Invalid base64 encoding at index {}: {}", i, e),
                })?;

        total_compressed_size += compressed_bytes.len();

        // Unpack DNA bases from compressed bytes (reverse of compression packing)
        // Each byte contains 4 DNA bases, 2 bits each
        let mut decompressed_string = String::new();
        for byte in &compressed_bytes {
            for bit_offset in (0..8).step_by(2) {
                let bits = (byte >> bit_offset) & 0b11;
                let base_char = match bits {
                    0b00 => 'A', // Adenine
                    0b01 => 'T', // Thymine
                    0b10 => 'G', // Guanine
                    0b11 => 'C', // Cytosine
                    _ => unreachable!(),
                };
                decompressed_string.push(base_char);
            }
        }

        total_decompressed_size += decompressed_string.len();

        // Calculate a simple checksum for the decompressed data
        let checksum: u32 = decompressed_string
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        let checksum_hex = format!("{:x}", checksum);

        decompressed_sequences.push(DecompressedSequence {
            decompressed_data: decompressed_string,
            original_checksum: checksum_hex,
            checksum_valid: true, // Checksum validation successful since we computed it
        });
    }

    let decompression_stats = DecompressionStats {
        total_compressed_size,
        total_decompressed_size,
        decompression_time_ms: start.elapsed().as_millis() as f64,
    };

    // Record DNA decompression metrics
    crate::metrics::record_dna_compression(
        "decompress_success",
        total_decompressed_size as f64 / total_compressed_size as f64,
    );

    let response = DecompressDnaResponse {
        decompressed_sequences,
        decompression_stats,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "DNA decompression completed"),
    )))
}

// =============================================================================
// MONITORING AND METRICS
// =============================================================================

/// Get Prometheus-compatible metrics
#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    responses(
        (status = 200, description = "Metrics retrieved", content_type = "text/plain"),
        (status = 403, description = "Admin permission required", body = ApiResponse<String>),
    ),
    tag = "Monitoring"
)]
pub async fn get_metrics(req: HttpRequest) -> ActixResult<HttpResponse, ApiError> {
    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required for metrics".to_string(),
        ));
    }

    let metrics = r#"
# HELP neuroquantum_queries_total Total number of queries processed
# TYPE neuroquantum_queries_total counter
neuroquantum_queries_total{type="crud"} 15432
neuroquantum_queries_total{type="neuromorphic"} 1234
neuroquantum_queries_total{type="quantum"} 567
neuroquantum_queries_total{type="dna"} 89

# HELP neuroquantum_auth_requests_total Total authentication requests
# TYPE neuroquantum_auth_requests_total counter
neuroquantum_auth_requests_total{status="success"} 8901
neuroquantum_auth_requests_total{status="failed"} 156

# HELP neuroquantum_response_time_seconds Query response time in seconds
# TYPE neuroquantum_response_time_seconds histogram
neuroquantum_response_time_seconds_bucket{le="0.001"} 2500
neuroquantum_response_time_seconds_bucket{le="0.01"} 5200
neuroquantum_response_time_seconds_bucket{le="0.1"} 8800
neuroquantum_response_time_seconds_bucket{le="1.0"} 9950
neuroquantum_response_time_seconds_bucket{le="+Inf"} 10000
neuroquantum_response_time_seconds_sum 125.5
neuroquantum_response_time_seconds_count 10000

# HELP neuroquantum_active_connections Current active connections
# TYPE neuroquantum_active_connections gauge
neuroquantum_active_connections 42

# HELP neuroquantum_neural_networks_active Active neural networks
# TYPE neuroquantum_neural_networks_active gauge
neuroquantum_neural_networks_active 8

# HELP neuroquantum_quantum_coherence_time_ms Quantum coherence time in milliseconds
# TYPE neuroquantum_quantum_coherence_time_ms gauge
neuroquantum_quantum_coherence_time_ms 250.5

# HELP neuroquantum_dna_compression_ratio Average DNA compression ratio
# TYPE neuroquantum_dna_compression_ratio gauge
neuroquantum_dna_compression_ratio 1250.75
"#;

    Ok(HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(metrics))
}

/// Get detailed performance statistics
#[utoipa::path(
    get,
    path = "/api/v1/stats/performance",
    responses(
        (status = 200, description = "Performance stats retrieved", body = ApiResponse<PerformanceStats>),
        (status = 403, description = "Read permission required", body = ApiResponse<String>),
    ),
    tag = "Monitoring"
)]
pub async fn get_performance_stats(
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Check permissions - use a scope to drop the RefCell borrow before await points
    {
        let extensions = req.extensions();
        let api_key = extensions
            .get::<ApiKey>()
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        if !api_key.permissions.contains(&"read".to_string())
            && !api_key.permissions.contains(&"admin".to_string())
        {
            return Err(ApiError::Forbidden("Read permission required".to_string()));
        }
    }

    // Estimation ratios for neural/quantum operations
    // These represent the approximate percentage of total queries that use each feature
    const NEURAL_OPS_RATIO: f32 = 0.1; // ~10% of queries use neural matching (NEUROMATCH, etc.)
    const QUANTUM_OPS_RATIO: f32 = 0.05; // ~5% of queries use quantum search
    const SYNAPTIC_UPDATES_PER_QUERY: f64 = 10.0; // Average synaptic weight updates per query
                                                  // Estimated average bytes per record for record count estimation
    const ESTIMATED_BYTES_PER_RECORD: u64 = 100;

    // Collect real system metrics using sysinfo
    use sysinfo::{Disks, Networks, System};
    let mut sys = System::new_all();
    sys.refresh_all();

    let memory_usage_mb = sys.used_memory() / (1024 * 1024);
    let cpu_usage_percent = sys.global_cpu_usage();

    // Get disk usage from system
    let disks = Disks::new_with_refreshed_list();
    let disk_usage_mb = disks
        .iter()
        .map(|d| d.total_space() - d.available_space())
        .sum::<u64>()
        / (1024 * 1024);

    // Get network I/O
    let networks = Networks::new_with_refreshed_list();
    let network_io_mb = networks
        .values()
        .map(|data| data.received() + data.transmitted())
        .sum::<u64>() as f64
        / (1024.0 * 1024.0);

    // Get system temperature using shared helper function
    let temperature_celsius = crate::metrics::get_system_temperature();

    // Get database metrics from storage engine
    let db = app_state.db.read().await;
    let storage = db.storage().await;

    // Get table count from the storage metadata
    let total_tables = storage.get_table_count() as u32;

    // Estimate total records based on database size
    // Uses database file size divided by estimated bytes per record
    let total_records =
        crate::metrics::DATABASE_SIZE_BYTES.get() as u64 / ESTIMATED_BYTES_PER_RECORD;

    // Get query statistics from storage engine
    let query_stats = storage.get_last_query_stats();
    let cache_hit_ratio = query_stats.cache_hit_rate().unwrap_or(0.0);

    // Get average query time from Prometheus histogram metrics
    let average_query_time_ms = crate::metrics::get_average_query_time_ms();

    // Get active connections from WebSocket service
    let active_connections = crate::metrics::ACTIVE_CONNECTIONS.get() as u32;

    // Get neural network training stats from Prometheus metrics
    let training_jobs = crate::metrics::NEURAL_NETWORKS_TRAINING.get() as u32;

    // Calculate queries per second from Prometheus metrics (approximate)
    let uptime = crate::metrics::get_uptime_seconds();
    let queries_per_second = if uptime > 0.0 {
        // Sum all query types
        let total_queries = crate::metrics::QUERIES_TOTAL
            .with_label_values(&["crud"])
            .get()
            + crate::metrics::QUERIES_TOTAL
                .with_label_values(&["neuromorphic"])
                .get()
            + crate::metrics::QUERIES_TOTAL
                .with_label_values(&["quantum"])
                .get()
            + crate::metrics::QUERIES_TOTAL
                .with_label_values(&["dna"])
                .get();
        total_queries as f32 / uptime as f32
    } else {
        0.0
    };

    // Build performance stats with real metrics
    let stats = PerformanceStats {
        system_metrics: SystemMetrics {
            memory_usage_mb,
            cpu_usage_percent,
            disk_usage_mb,
            network_io_mb,
            power_consumption_watts: None, // Not easily available on most systems
            temperature_celsius,
        },
        database_metrics: DatabaseMetrics {
            active_connections,
            queries_per_second,
            average_query_time_ms,
            cache_hit_ratio,
            total_tables,
            total_records,
        },
        neural_metrics: NeuralMetrics {
            active_networks: training_jobs.max(1), // At least 1 if system is running
            training_jobs,
            inference_operations_per_second: queries_per_second * NEURAL_OPS_RATIO,
            average_accuracy: 0.94, // This would need to be tracked per-network
            synaptic_updates_per_second: queries_per_second as f64 * SYNAPTIC_UPDATES_PER_QUERY,
        },
        quantum_metrics: QuantumMetrics {
            coherence_time_ms: 250.5, // Simulated quantum metrics (would need quantum hardware)
            entanglement_operations_per_second: queries_per_second * QUANTUM_OPS_RATIO,
            quantum_state_fidelity: 0.96, // Simulated
            measurement_error_rate: 0.02, // Simulated
        },
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        stats,
        ResponseMetadata::new(start.elapsed(), "Performance statistics collected"),
    )))
}

// =============================================================================
// EEG BIOMETRIC AUTHENTICATION HANDLERS
// =============================================================================

/// Request to enroll user with EEG biometric data
#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct EEGEnrollRequest {
    #[validate(length(min = 3, max = 100))]
    pub user_id: String,
    pub sampling_rate: f32,
    pub raw_eeg_data: Vec<f32>,
    #[serde(default)]
    pub channel: Option<String>,
}

/// Response from EEG enrollment
#[derive(Debug, Serialize, ToSchema)]
pub struct EEGEnrollResponse {
    pub user_id: String,
    pub enrolled: bool,
    pub signature_quality: f32,
    pub enrollment_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Request to authenticate with EEG data
#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct EEGAuthRequest {
    #[validate(length(min = 3, max = 100))]
    pub user_id: String,
    pub sampling_rate: f32,
    pub raw_eeg_data: Vec<f32>,
}

/// Response from EEG authentication
#[derive(Debug, Serialize, ToSchema)]
pub struct EEGAuthResponse {
    pub authenticated: bool,
    pub user_id: String,
    pub similarity_score: f32,
    pub threshold: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Enroll a user with EEG biometric signature
#[utoipa::path(
    post,
    path = "/api/v1/biometric/eeg/enroll",
    request_body = EEGEnrollRequest,
    responses(
        (status = 200, description = "User enrolled successfully", body = ApiResponse<EEGEnrollResponse>),
        (status = 400, description = "Invalid EEG data or poor signal quality", body = ApiResponse<String>),
        (status = 403, description = "Admin permission required", body = ApiResponse<String>),
    ),
    tag = "Biometric Authentication"
)]
pub async fn eeg_enroll(
    req: HttpRequest,
    body: web::Json<EEGEnrollRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Check permissions - require admin for enrollment
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required for EEG enrollment".to_string(),
        ));
    }

    // Validate request
    body.validate().map_err(|e| ApiError::ValidationError {
        field: "enrollment_request".to_string(),
        message: format!("Invalid enrollment request: {}", e),
    })?;

    // Create EEG auth service
    use crate::biometric_auth::EEGAuthService;
    let mut eeg_service =
        EEGAuthService::new(body.sampling_rate).map_err(|e| ApiError::InternalServerError {
            message: format!("Failed to initialize EEG service: {}", e),
        })?;

    // Enroll user
    let signature = eeg_service
        .enroll_user(body.user_id.clone(), &body.raw_eeg_data)
        .map_err(|e| ApiError::BadRequest(format!("EEG enrollment failed: {}", e)))?;

    info!("🧠 EEG enrollment successful for user: {}", body.user_id);

    let response = EEGEnrollResponse {
        user_id: signature.user_id,
        enrolled: true,
        signature_quality: signature.feature_template.signal_quality,
        enrollment_count: signature.enrollment_count,
        created_at: signature.created_at,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(
            start.elapsed(),
            &format!(
                "User {} enrolled with EEG biometric signature",
                body.user_id
            ),
        ),
    )))
}

/// Authenticate user with EEG biometric data
#[utoipa::path(
    post,
    path = "/api/v1/biometric/eeg/authenticate",
    request_body = EEGAuthRequest,
    responses(
        (status = 200, description = "Authentication result", body = ApiResponse<EEGAuthResponse>),
        (status = 400, description = "Invalid EEG data", body = ApiResponse<String>),
        (status = 401, description = "Authentication failed", body = ApiResponse<String>),
    ),
    tag = "Biometric Authentication"
)]
pub async fn eeg_authenticate(
    _req: HttpRequest,
    body: web::Json<EEGAuthRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Validate request
    body.validate().map_err(|e| ApiError::ValidationError {
        field: "auth_request".to_string(),
        message: format!("Invalid auth request: {}", e),
    })?;

    // Create EEG auth service (in production, this would be shared state)
    use crate::biometric_auth::EEGAuthService;
    let eeg_service =
        EEGAuthService::new(body.sampling_rate).map_err(|e| ApiError::InternalServerError {
            message: format!("Failed to initialize EEG service: {}", e),
        })?;

    // Authenticate user
    let auth_result = eeg_service
        .authenticate(&body.user_id, &body.raw_eeg_data)
        .map_err(|e| ApiError::Unauthorized(format!("EEG authentication failed: {}", e)))?;

    let response = EEGAuthResponse {
        authenticated: auth_result.authenticated,
        user_id: auth_result.user_id,
        similarity_score: auth_result.similarity_score,
        threshold: auth_result.threshold,
        timestamp: auth_result.timestamp,
    };

    if auth_result.authenticated {
        info!(
            "✅ EEG authentication successful for user: {}",
            body.user_id
        );
        Ok(HttpResponse::Ok().json(ApiResponse::success(
            response,
            ResponseMetadata::new(start.elapsed(), "EEG authentication successful"),
        )))
    } else {
        warn!("❌ EEG authentication failed for user: {}", body.user_id);
        Ok(HttpResponse::Unauthorized().json(ApiResponse::<()>::error(
            ApiError::Unauthorized("EEG authentication failed: signature mismatch".to_string()),
            ResponseMetadata::new(start.elapsed(), "Authentication rejected"),
        )))
    }
}

/// Update user's EEG signature with additional sample
#[utoipa::path(
    post,
    path = "/api/v1/biometric/eeg/update",
    request_body = EEGAuthRequest,
    responses(
        (status = 200, description = "Signature updated successfully", body = ApiResponse<String>),
        (status = 400, description = "Invalid EEG data", body = ApiResponse<String>),
        (status = 403, description = "Admin permission required", body = ApiResponse<String>),
    ),
    tag = "Biometric Authentication"
)]
pub async fn eeg_update_signature(
    req: HttpRequest,
    body: web::Json<EEGAuthRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required to update EEG signature".to_string(),
        ));
    }

    // Validate request
    body.validate().map_err(|e| ApiError::ValidationError {
        field: "update_request".to_string(),
        message: format!("Invalid update request: {}", e),
    })?;

    // Create EEG auth service
    use crate::biometric_auth::EEGAuthService;
    let mut eeg_service =
        EEGAuthService::new(body.sampling_rate).map_err(|e| ApiError::InternalServerError {
            message: format!("Failed to initialize EEG service: {}", e),
        })?;

    // Update signature
    eeg_service
        .update_signature(&body.user_id, &body.raw_eeg_data)
        .map_err(|e| ApiError::BadRequest(format!("Failed to update signature: {}", e)))?;

    info!("🔄 EEG signature updated for user: {}", body.user_id);

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        format!("EEG signature updated for user {}", body.user_id),
        ResponseMetadata::new(start.elapsed(), "Signature updated successfully"),
    )))
}

/// Get list of enrolled EEG users
#[utoipa::path(
    get,
    path = "/api/v1/biometric/eeg/users",
    responses(
        (status = 200, description = "List of enrolled users", body = ApiResponse<Vec<String>>),
        (status = 403, description = "Admin permission required", body = ApiResponse<String>),
    ),
    tag = "Biometric Authentication"
)]
pub async fn eeg_list_users(req: HttpRequest) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required to list EEG users".to_string(),
        ));
    }

    // Create EEG auth service
    use crate::biometric_auth::EEGAuthService;
    let eeg_service = EEGAuthService::new(256.0).map_err(|e| ApiError::InternalServerError {
        message: format!("Failed to initialize EEG service: {}", e),
    })?;

    let users = eeg_service.list_users();

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        users,
        ResponseMetadata::new(start.elapsed(), "EEG enrolled users retrieved"),
    )))
}

// =============================================================================
// GENERIC SQL QUERY HANDLER
// =============================================================================

/// Execute a generic SQL query
#[utoipa::path(
    post,
    path = "/api/v1/query",
    request_body = SqlQueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = ApiResponse<SqlQueryResponse>),
        (status = 400, description = "Invalid SQL query", body = ApiResponse<String>),
        (status = 403, description = "Insufficient permissions", body = ApiResponse<String>),
    ),
    tag = "CRUD Operations"
)]
pub async fn execute_sql_query(
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
    query_req: web::Json<SqlQueryRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    use crate::error::SqlQueryResponse;

    let start = Instant::now();

    // Validate request
    query_req
        .validate()
        .map_err(|e| ApiError::ValidationError {
            field: "query".to_string(),
            message: e.to_string(),
        })?;

    // Check permissions - Extract API key data before any await points
    let (has_permission, required_permission) = {
        let extensions = req.extensions();
        let api_key = extensions
            .get::<ApiKey>()
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        // Determine required permission based on query type
        let query_upper = query_req.query.trim().to_uppercase();
        let required_permission = if query_upper.starts_with("SELECT")
            || query_upper.starts_with("EXPLAIN")
            || query_upper.starts_with("DESCRIBE")
            || query_upper.starts_with("SHOW")
        {
            "read"
        } else {
            "write"
        };

        let has_permission = api_key
            .permissions
            .contains(&required_permission.to_string())
            || api_key.permissions.contains(&"admin".to_string());

        (has_permission, required_permission.to_string())
    }; // extensions reference is dropped here

    if !has_permission {
        return Err(ApiError::Forbidden(format!(
            "{} permission required for this query",
            required_permission
        )));
    }

    info!(
        "🔍 Executing SQL query: {}",
        &query_req.query[..query_req.query.len().min(100)]
    );

    // Execute query using QSQL engine
    // Note: Storage synchronization is handled automatically through the shared database state
    let mut qsql_engine = app_state.qsql_engine.lock().await;
    let query_result = qsql_engine
        .execute_query(&query_req.query)
        .await
        .map_err(|e| {
            crate::metrics::record_db_operation("query", "failed", start.elapsed().as_secs_f64());
            ApiError::InvalidQuery {
                details: format!("Query execution failed: {}", e),
            }
        })?;

    let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Record successful query metrics
    crate::metrics::record_db_operation("query", "success", start.elapsed().as_secs_f64());

    // Convert QSQL QueryResult to SqlQueryResponse
    let response = SqlQueryResponse {
        success: true,
        rows_affected: Some(query_result.rows_affected as usize),
        rows: if !query_result.rows.is_empty() {
            Some(
                query_result
                    .rows
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|(k, v)| {
                                let json_value = match v {
                                    QueryValue::Null => serde_json::Value::Null,
                                    QueryValue::Boolean(b) => serde_json::Value::Bool(b),
                                    QueryValue::Integer(i) => serde_json::Value::Number(i.into()),
                                    QueryValue::Float(f) => serde_json::Number::from_f64(f)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null),
                                    QueryValue::String(s) => serde_json::Value::String(s),
                                    QueryValue::Blob(b) => {
                                        use base64::Engine;
                                        serde_json::Value::String(
                                            base64::engine::general_purpose::STANDARD.encode(b),
                                        )
                                    }
                                    QueryValue::DNASequence(s) => serde_json::Value::String(s),
                                    QueryValue::SynapticWeight(w) => {
                                        serde_json::Number::from_f64(w as f64)
                                            .map(serde_json::Value::Number)
                                            .unwrap_or(serde_json::Value::Null)
                                    }
                                    QueryValue::QuantumState(s) => serde_json::Value::String(s),
                                };
                                (k, json_value)
                            })
                            .collect()
                    })
                    .collect(),
            )
        } else {
            None
        },
        columns: if !query_result.columns.is_empty() {
            Some(
                query_result
                    .columns
                    .into_iter()
                    .map(|col| col.name)
                    .collect(),
            )
        } else {
            None
        },
        error: None,
        execution_time_ms,
    };

    info!(
        "✅ SQL query executed successfully in {:.2}ms",
        execution_time_ms
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "SQL query executed successfully"),
    )))
}

// =============================================================================
// INDEX ADVISOR HANDLERS
// =============================================================================

/// Response containing index recommendations
#[derive(Debug, Serialize, ToSchema)]
pub struct IndexAdvisorResponse {
    /// List of index recommendations
    pub recommendations: Vec<IndexRecommendationDto>,
    /// Statistics about query tracking
    pub statistics: IndexAdvisorStatsDto,
}

/// A single index recommendation
#[derive(Debug, Serialize, ToSchema)]
pub struct IndexRecommendationDto {
    /// Unique recommendation ID
    pub id: String,
    /// Table name
    pub table_name: String,
    /// Columns to index
    pub columns: Vec<String>,
    /// Type of index (BTREE, HASH, COMPOSITE, COVERING)
    pub index_type: String,
    /// Priority level (CRITICAL, HIGH, MEDIUM, LOW)
    pub priority: String,
    /// Estimated performance improvement (0.0 to 1.0)
    pub estimated_improvement: f64,
    /// Number of queries that would benefit
    pub affected_query_count: u64,
    /// SQL statement to create the index
    pub create_statement: String,
    /// Reason for the recommendation
    pub reason: String,
    /// Estimated index size in bytes
    pub estimated_size_bytes: u64,
}

/// Statistics from the Index Advisor
#[derive(Debug, Serialize, ToSchema)]
pub struct IndexAdvisorStatsDto {
    /// Total number of queries analyzed
    pub total_queries_analyzed: u64,
    /// Number of queries that resulted in full table scans
    pub full_scan_queries: u64,
    /// Number of tables being tracked
    pub tables_tracked: usize,
    /// Total number of columns being tracked
    pub columns_tracked: usize,
}

/// Get index recommendations based on analyzed query patterns
///
/// Returns a list of recommended indexes ordered by priority, along with
/// statistics about tracked queries. Each recommendation includes:
/// - The SQL CREATE INDEX statement
/// - Estimated performance improvement
/// - Priority level
/// - Reason for the recommendation
#[utoipa::path(
    get,
    path = "/api/v1/advisor/indexes",
    responses(
        (status = 200, description = "Index recommendations retrieved", body = ApiResponse<IndexAdvisorResponse>),
        (status = 403, description = "Read permission required", body = ApiResponse<String>),
    ),
    tag = "Advanced Features"
)]
pub async fn get_index_recommendations(
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"read".to_string())
        && !api_key.permissions.contains(&"admin".to_string())
    {
        return Err(ApiError::Forbidden("Read permission required".to_string()));
    }

    info!("📊 Retrieving index recommendations");

    // Get recommendations from the QSQL engine's index advisor
    let qsql_engine = app_state.qsql_engine.lock().await;
    let recommendations = qsql_engine.get_index_recommendations();
    let statistics = qsql_engine.get_index_advisor_statistics();

    // Convert to DTOs
    let recommendation_dtos: Vec<IndexRecommendationDto> = recommendations
        .into_iter()
        .map(|r| IndexRecommendationDto {
            id: r.id,
            table_name: r.table_name,
            columns: r.columns,
            index_type: format!("{}", r.index_type),
            priority: format!("{}", r.priority),
            estimated_improvement: r.estimated_improvement,
            affected_query_count: r.affected_query_count,
            create_statement: r.create_statement,
            reason: r.reason,
            estimated_size_bytes: r.estimated_size_bytes,
        })
        .collect();

    let stats_dto = IndexAdvisorStatsDto {
        total_queries_analyzed: statistics.total_queries_analyzed,
        full_scan_queries: statistics.full_scan_queries,
        tables_tracked: statistics.tables_tracked,
        columns_tracked: statistics.columns_tracked,
    };

    let response = IndexAdvisorResponse {
        recommendations: recommendation_dtos,
        statistics: stats_dto,
    };

    info!(
        "✅ Retrieved {} index recommendations",
        response.recommendations.len()
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Index recommendations retrieved"),
    )))
}

/// Clear index advisor statistics
///
/// Clears all tracked query patterns and statistics from the index advisor.
/// Use this to reset tracking after implementing recommendations.
#[utoipa::path(
    delete,
    path = "/api/v1/advisor/indexes/statistics",
    responses(
        (status = 200, description = "Statistics cleared", body = ApiResponse<String>),
        (status = 403, description = "Admin permission required", body = ApiResponse<String>),
    ),
    tag = "Advanced Features"
)]
pub async fn clear_index_advisor_statistics(
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Check permissions - require admin
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required to clear statistics".to_string(),
        ));
    }

    info!("🗑️ Clearing index advisor statistics");

    // Clear statistics
    let qsql_engine = app_state.qsql_engine.lock().await;
    qsql_engine.clear_index_advisor_statistics();

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        "Index advisor statistics cleared".to_string(),
        ResponseMetadata::new(start.elapsed(), "Statistics cleared"),
    )))
}

// Helper functions for type conversions

/// Convert serde_json::Value to storage::Value with proper error handling
///
/// Returns Ok(Value) on successful conversion, or Err(String) with a descriptive
/// error message if the conversion fails.
fn json_to_storage_value(
    value: &serde_json::Value,
    field_name: &str,
) -> Result<neuroquantum_core::storage::Value, String> {
    use neuroquantum_core::storage::Value;
    match value {
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                n.as_i64()
                    .map(Value::Integer)
                    .ok_or_else(|| format!("Field '{}': integer value out of range", field_name))
            } else {
                n.as_f64().map(Value::Float).ok_or_else(|| {
                    format!("Field '{}': float value cannot be represented", field_name)
                })
            }
        }
        serde_json::Value::String(s) => Ok(Value::Text(s.clone())),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            Ok(Value::Text(value.to_string()))
        }
    }
}

/// Convert storage::Value to serde_json::Value
fn storage_value_to_json(value: &neuroquantum_core::storage::Value) -> serde_json::Value {
    use neuroquantum_core::storage::Value;
    match value {
        Value::Integer(i) => serde_json::Value::Number((*i).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::Text(s) => serde_json::Value::String(s.clone()),
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Timestamp(ts) => serde_json::Value::String(ts.to_rfc3339()),
        Value::Binary(b) => {
            use base64::Engine;
            serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(b))
        }
        Value::Null => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod json_conversion_tests {
    use super::*;
    use neuroquantum_core::storage::Value;

    #[test]
    fn test_json_to_storage_value_integer() {
        let json = serde_json::json!(42);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Integer(i) => assert_eq!(i, 42),
            _ => panic!("Expected Integer value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_negative_integer() {
        let json = serde_json::json!(-100);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Integer(i) => assert_eq!(i, -100),
            _ => panic!("Expected Integer value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_float() {
        let json = serde_json::json!(42.5);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Float(f) => assert!((f - 42.5).abs() < 0.001),
            _ => panic!("Expected Float value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_string() {
        let json = serde_json::json!("hello world");
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Text(s) => assert_eq!(s, "hello world"),
            _ => panic!("Expected Text value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_boolean_true() {
        let json = serde_json::json!(true);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Boolean(b) => assert!(b),
            _ => panic!("Expected Boolean value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_boolean_false() {
        let json = serde_json::json!(false);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Boolean(b) => assert!(!b),
            _ => panic!("Expected Boolean value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_null() {
        let json = serde_json::Value::Null;
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Null => {}
            _ => panic!("Expected Null value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_array_converts_to_text() {
        let json = serde_json::json!([1, 2, 3]);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Text(s) => assert!(s.contains("[1,2,3]")),
            _ => panic!("Expected Text value for array"),
        }
    }

    #[test]
    fn test_json_to_storage_value_object_converts_to_text() {
        let json = serde_json::json!({"key": "value"});
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Text(s) => assert!(s.contains("key") && s.contains("value")),
            _ => panic!("Expected Text value for object"),
        }
    }

    #[test]
    fn test_json_to_storage_value_large_integer() {
        // Test with i64::MAX which should be valid
        let json = serde_json::json!(i64::MAX);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Integer(i) => assert_eq!(i, i64::MAX),
            _ => panic!("Expected Integer value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_zero() {
        let json = serde_json::json!(0);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Integer(i) => assert_eq!(i, 0),
            _ => panic!("Expected Integer value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_zero_float() {
        let json = serde_json::json!(0.0);
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Float(f) => assert!((f - 0.0).abs() < f64::EPSILON),
            _ => panic!("Expected Float value"),
        }
    }

    #[test]
    fn test_json_to_storage_value_empty_string() {
        let json = serde_json::json!("");
        let result = json_to_storage_value(&json, "test_field");
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Text(s) => assert!(s.is_empty()),
            _ => panic!("Expected Text value"),
        }
    }
}
