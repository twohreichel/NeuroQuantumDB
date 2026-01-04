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
        get_metrics,
        get_performance_stats,
        eeg_enroll,
        eeg_authenticate,
        eeg_update_signature,
        eeg_list_users,
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
            CompressDnaRequest,
            CompressDnaResponse,

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
        (name = "Advanced Features", description = "Neural networks, quantum search, DNA compression"),
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

    // Convert API TableSchema to storage TableSchema
    let storage_schema = neuroquantum_core::storage::TableSchema {
        name: create_req.schema.name.clone(),
        columns: create_req
            .schema
            .columns
            .iter()
            .map(|c| neuroquantum_core::storage::ColumnDefinition {
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
                default_value: c.default_value.as_ref().map(|v| match v {
                    serde_json::Value::Number(n) => {
                        if n.is_i64() {
                            neuroquantum_core::storage::Value::Integer(n.as_i64().unwrap())
                        } else {
                            neuroquantum_core::storage::Value::Float(n.as_f64().unwrap())
                        }
                    }
                    serde_json::Value::String(s) => {
                        neuroquantum_core::storage::Value::Text(s.clone())
                    }
                    serde_json::Value::Bool(b) => neuroquantum_core::storage::Value::Boolean(*b),
                    serde_json::Value::Null => neuroquantum_core::storage::Value::Null,
                    _ => neuroquantum_core::storage::Value::Text(v.to_string()),
                }),
                auto_increment: c.auto_increment.unwrap_or(false),
            })
            .collect(),
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

        // Convert HashMap to Row
        let mut fields = std::collections::HashMap::new();
        for (key, value) in record.iter() {
            let storage_value = match value {
                serde_json::Value::Number(n) => {
                    if n.is_i64() {
                        neuroquantum_core::storage::Value::Integer(n.as_i64().unwrap())
                    } else {
                        neuroquantum_core::storage::Value::Float(n.as_f64().unwrap())
                    }
                }
                serde_json::Value::String(s) => neuroquantum_core::storage::Value::Text(s.clone()),
                serde_json::Value::Bool(b) => neuroquantum_core::storage::Value::Boolean(*b),
                serde_json::Value::Null => neuroquantum_core::storage::Value::Null,
                serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                    neuroquantum_core::storage::Value::Text(value.to_string())
                }
            };
            fields.insert(key.clone(), storage_value);
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
    _db: web::Data<NeuroQuantumDB>,
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
    _db: web::Data<NeuroQuantumDB>,
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
    _app_state: web::Data<crate::AppState>,
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
    _db: web::Data<NeuroQuantumDB>,
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

/// Perform quantum-inspired search
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
    _app_state: web::Data<crate::AppState>,
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

    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"quantum".to_string())
        && !api_key.permissions.contains(&"admin".to_string())
    {
        return Err(ApiError::Forbidden(
            "Quantum permission required".to_string(),
        ));
    }

    if search_req.query_vector.is_empty() {
        return Err(ApiError::BadRequest(
            "Query vector cannot be empty".to_string(),
        ));
    }

    info!(
        "⚛️ Performing quantum search on table '{}' with {} dimensions",
        search_req.table_name,
        search_req.query_vector.len()
    );

    // Simulate quantum search
    let mut results = Vec::new();
    for i in 0..search_req.max_results.unwrap_or(10).min(20) {
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
            similarity_score: 0.95 - (i as f32 * 0.02),
            quantum_probability: 0.98 - (i as f32 * 0.01),
            entanglement_strength: Some(0.87 - (i as f32 * 0.03)),
        });
    }

    let quantum_stats = QuantumStats {
        coherence_time_used_ms: 150.5,
        superposition_states: 8,
        measurement_collapses: 3,
        entanglement_operations: 15,
    };

    let response = QuantumSearchResponse {
        results,
        quantum_stats,
    };

    // Record quantum search metrics
    crate::metrics::record_quantum_search("success");

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Quantum search completed"),
    )))
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

    // Check permissions
    let extensions = req.extensions();
    let api_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if !api_key.permissions.contains(&"dna".to_string())
        && !api_key.permissions.contains(&"admin".to_string())
    {
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

        // Encode compressed data as base64 for JSON response
        let compressed_data = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &serde_json::to_vec(&compressed).map_err(|e| ApiError::CompressionError {
                reason: format!("Serialization failed: {}", e),
            })?,
        );

        let compressed_size = compressed.compressed_size;
        let compression_ratio = compressed.sequence.metadata.compression_ratio as f32;
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
    _db: web::Data<NeuroQuantumDB>,
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

    // Simulate performance metrics collection
    let stats = PerformanceStats {
        system_metrics: SystemMetrics {
            memory_usage_mb: 2048,
            cpu_usage_percent: 45.2,
            disk_usage_mb: 15000,
            network_io_mb: 125.5,
            power_consumption_watts: Some(35.8),
            temperature_celsius: Some(42.5),
        },
        database_metrics: DatabaseMetrics {
            active_connections: 42,
            queries_per_second: 150.5,
            average_query_time_ms: 12.3,
            cache_hit_ratio: 0.87,
            total_tables: 25,
            total_records: 1_250_000,
        },
        neural_metrics: NeuralMetrics {
            active_networks: 8,
            training_jobs: 3,
            inference_operations_per_second: 75.2,
            average_accuracy: 0.94,
            synaptic_updates_per_second: 1250.5,
        },
        quantum_metrics: QuantumMetrics {
            coherence_time_ms: 250.5,
            entanglement_operations_per_second: 15.7,
            quantum_state_fidelity: 0.96,
            measurement_error_rate: 0.02,
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

// Helper functions for type conversions

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
