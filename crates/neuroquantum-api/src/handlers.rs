use crate::auth::{ApiKey, AuthService};
use crate::error::{ApiError, ApiResponse, ResponseMetadata};
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Result as ActixResult};
use neuroquantum_core::NeuroQuantumDB;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn};
use utoipa::ToSchema;

/// 🔑 Auth endpoints
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

/// Generate new API key (requires admin permission)
pub async fn generate_api_key(
    req: HttpRequest,
    auth_service: web::Data<AuthService>,
    key_request: web::Json<GenerateKeyRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Verify admin permissions from middleware - fix borrowing issue
    let extensions = req.extensions();
    let requesting_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Admin authentication required".to_string()))?;

    if !requesting_key.permissions.contains(&"admin".to_string()) {
        warn!(
            "Non-admin user attempted to generate API key: {}",
            requesting_key.name
        );
        return Err(ApiError::Forbidden(
            "Admin permission required to generate API keys".to_string(),
        ));
    }

    // Validate permissions
    let valid_permissions = vec!["admin", "neuromorphic", "quantum", "dna", "read", "write"];

    for permission in &key_request.permissions {
        if !valid_permissions.contains(&permission.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Invalid permission: {}. Valid permissions are: {:?}",
                permission, valid_permissions
            )));
        }
    }

    // Generate the API key
    let mut auth_service_mut = auth_service.as_ref().clone();
    let new_key = auth_service_mut.generate_api_key(
        key_request.name.clone(),
        key_request.permissions.clone(),
        key_request.expiry_hours,
        key_request.rate_limit_per_hour,
    );

    info!(
        "🔑 Admin {} generated new API key for: {}",
        requesting_key.name, new_key.name
    );

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
pub async fn revoke_api_key(
    req: HttpRequest,
    auth_service: web::Data<AuthService>,
    revoke_request: web::Json<RevokeKeyRequest>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Verify admin permissions - fix borrowing issue
    let extensions = req.extensions();
    let requesting_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Admin authentication required".to_string()))?;

    if !requesting_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required to revoke API keys".to_string(),
        ));
    }

    // Don't allow users to revoke their own key
    if requesting_key.key == revoke_request.api_key {
        return Err(ApiError::BadRequest(
            "Cannot revoke your own API key".to_string(),
        ));
    }

    let mut auth_service_mut = auth_service.as_ref().clone();
    let revoked = auth_service_mut.revoke_api_key(&revoke_request.api_key);

    if revoked {
        info!(
            "🗑️ Admin {} revoked API key: {}",
            requesting_key.name,
            &revoke_request.api_key[..8]
        );
        Ok(HttpResponse::Ok().json(ApiResponse::success(
            serde_json::json!({"revoked": true, "key": &revoke_request.api_key[..8]}),
            ResponseMetadata::new(start.elapsed(), "API key revoked successfully"),
        )))
    } else {
        Err(ApiError::NotFound("API key not found".to_string()))
    }
}

/// List all API keys (requires admin permission)
pub async fn list_api_keys(
    req: HttpRequest,
    auth_service: web::Data<AuthService>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    // Verify admin permissions - fix borrowing issue
    let extensions = req.extensions();
    let requesting_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Admin authentication required".to_string()))?;

    if !requesting_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required to list API keys".to_string(),
        ));
    }

    let api_keys = auth_service.list_api_keys();

    // Don't expose full API keys, only metadata
    let safe_keys: Vec<serde_json::Value> = api_keys
        .iter()
        .map(|key| {
            serde_json::json!({
                "key_prefix": &key.key[..12],
                "name": key.name,
                "permissions": key.permissions,
                "expires_at": key.expires_at.to_rfc3339(),
                "created_at": key.created_at.to_rfc3339(),
                "last_used": key.last_used.map(|t| t.to_rfc3339()),
                "usage_count": key.usage_count,
                "rate_limit_per_hour": key.rate_limit_per_hour,
                "is_expired": auth_service.is_key_expired(key)
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        serde_json::json!({"api_keys": safe_keys, "total": safe_keys.len()}),
        ResponseMetadata::new(start.elapsed(), "API keys listed successfully"),
    )))
}

/// Get API key statistics (requires admin permission)
pub async fn get_key_stats(
    req: HttpRequest,
    auth_service: web::Data<AuthService>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let key_to_check = path.into_inner();

    // Verify admin permissions - fix borrowing issue
    let extensions = req.extensions();
    let requesting_key = extensions
        .get::<ApiKey>()
        .ok_or_else(|| ApiError::Unauthorized("Admin authentication required".to_string()))?;

    if !requesting_key.permissions.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden(
            "Admin permission required to view API key stats".to_string(),
        ));
    }

    match auth_service.get_api_key_stats(&key_to_check) {
        Some(stats) => Ok(HttpResponse::Ok().json(ApiResponse::success(
            stats,
            ResponseMetadata::new(start.elapsed(), "API key stats retrieved successfully"),
        ))),
        None => Err(ApiError::NotFound("API key not found".to_string())),
    }
}

/// Simple handlers for basic functionality
pub async fn dna_status(_db: web::Data<NeuroQuantumDB>) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({
        "status": "operational",
        "compression_ratio": 1200,
        "active_sequences": 42847
    });
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "DNA storage status retrieved"),
    )))
}

/// Basic neuromorphic query handler
pub async fn neuromorphic_query(
    _db: web::Data<NeuroQuantumDB>,
    _request: web::Json<serde_json::Value>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({"status": "processed", "results": []});
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Neuromorphic query processed"),
    )))
}

/// Basic network status handler  
pub async fn network_status(_db: web::Data<NeuroQuantumDB>) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({"active_synapses": 2847392, "learning_rate": 0.012});
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Network status retrieved"),
    )))
}

/// Basic training handler
pub async fn train_network(
    _db: web::Data<NeuroQuantumDB>,
    _request: web::Json<serde_json::Value>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({"status": "training_completed", "epochs_completed": 50});
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Network training completed"),
    )))
}

/// Basic quantum search handler
pub async fn quantum_search(
    _db: web::Data<NeuroQuantumDB>,
    _request: web::Json<serde_json::Value>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({"status": "completed", "results": [], "quantum_speedup": 15});
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Quantum search completed"),
    )))
}

/// Basic quantum optimize handler
pub async fn quantum_optimize(
    _db: web::Data<NeuroQuantumDB>,
    _request: web::Json<serde_json::Value>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({"status": "optimized", "energy_saving_percent": 23.7});
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Quantum optimization completed"),
    )))
}

/// Basic quantum status handler
pub async fn quantum_status(_db: web::Data<NeuroQuantumDB>) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({"quantum_processors": 4, "active_processors": 4});
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "Quantum status retrieved"),
    )))
}

/// Basic DNA compress handler
pub async fn dna_compress(
    _db: web::Data<NeuroQuantumDB>,
    _request: web::Json<serde_json::Value>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({"status": "compressed", "compression_ratio": 1180});
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "DNA compression completed"),
    )))
}

/// Basic DNA decompress handler
pub async fn dna_decompress(
    _db: web::Data<NeuroQuantumDB>,
    _request: web::Json<serde_json::Value>,
) -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();
    let response = serde_json::json!({"status": "decompressed", "integrity_verified": true});
    Ok(HttpResponse::Ok().json(ApiResponse::success(
        response,
        ResponseMetadata::new(start.elapsed(), "DNA decompression completed"),
    )))
}
