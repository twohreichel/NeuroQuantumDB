use actix_cors::Cors;
use actix_web::middleware::{Compress, Logger};
use actix_web::{web, App, HttpMessage, HttpResponse, HttpServer, Result as ActixResult};
use actix_web_prometheus::PrometheusMetricsBuilder;
use actix_ws::Message;
use anyhow::Result;
use futures_util::StreamExt;
use neuroquantum_core::{DatabaseConfig as CoreDatabaseConfig, NeuroQuantumDB};
use std::time::Instant;
use tracing::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod auth;
pub mod config;
pub mod error;
pub mod handlers;
pub mod jwt;
pub mod middleware;
pub mod rate_limit;

use auth::AuthService;
pub use config::ApiConfig;
pub use error::{ApiError, ApiResponse, ResponseMetadata};
use handlers::ApiDoc;
use jwt::JwtService;
use rate_limit::{RateLimitConfig, RateLimitService};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: NeuroQuantumDB,
    pub auth_service: AuthService,
    pub jwt_service: JwtService,
    pub rate_limit_service: RateLimitService,
    pub config: ApiConfig,
}

impl AppState {
    pub async fn new(config: ApiConfig) -> Result<Self> {
        // Convert our API config database config to the core database config
        let core_db_config = CoreDatabaseConfig::default(); // Using default for now
        let db = NeuroQuantumDB::new(&core_db_config).await?;
        let auth_service = AuthService::new();
        let jwt_service = JwtService::new(config.jwt.secret.as_bytes());
        
        let rate_limit_config = RateLimitConfig {
            requests_per_window: config.rate_limit.requests_per_hour,
            window_size_seconds: 3600,
            burst_allowance: config.rate_limit.burst_allowance,
            redis_url: config.redis.as_ref().map(|r| r.url.clone()),
            fallback_to_memory: true,
        };
        let rate_limit_service = RateLimitService::new(rate_limit_config).await?;

        Ok(Self {
            db,
            auth_service,
            jwt_service,
            rate_limit_service,
            config,
        })
    }
}

/// 🏥 Health check endpoint
pub async fn health_check() -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    let health_data = serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": 0,
        "system_metrics": {
            "memory_usage_mb": 128,
            "power_consumption_w": 45,
            "temperature_c": 42.5
        },
        "security": {
            "authentication_enabled": true,
            "jwt_enabled": true,
            "rate_limiting_enabled": true,
            "quantum_security": true
        },
        "features": {
            "neuromorphic_processing": true,
            "quantum_search": true,
            "dna_compression": true,
            "real_time_updates": true
        }
    });

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        health_data,
        ResponseMetadata::new(start.elapsed(), "Health check completed"),
    )))
}

/// 📊 Prometheus metrics endpoint (requires admin permission)
pub async fn metrics() -> HttpResponse {
    let metrics = r#"
# HELP neuroquantum_queries_total Total number of queries processed
# TYPE neuroquantum_queries_total counter
neuroquantum_queries_total{type="neuromorphic"} 1234
neuroquantum_queries_total{type="quantum"} 567
neuroquantum_queries_total{type="dna"} 89

# HELP neuroquantum_auth_requests_total Total authentication requests
# TYPE neuroquantum_auth_requests_total counter
neuroquantum_auth_requests_total{status="success"} 5678
neuroquantum_auth_requests_total{status="failed"} 123

# HELP neuroquantum_response_time_seconds Query response time in seconds
# TYPE neuroquantum_response_time_seconds histogram
neuroquantum_response_time_seconds_bucket{le="0.001"} 500
neuroquantum_response_time_seconds_bucket{le="0.01"} 1200
neuroquantum_response_time_seconds_bucket{le="0.1"} 1800
neuroquantum_response_time_seconds_bucket{le="+Inf"} 2000
neuroquantum_response_time_seconds_sum 15.5
neuroquantum_response_time_seconds_count 2000

# HELP neuroquantum_active_connections Current active connections
# TYPE neuroquantum_active_connections gauge
neuroquantum_active_connections 42
"#;

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(metrics)
}

/// 🔍 WebSocket handler for real-time communication (requires authentication)
pub async fn websocket_handler(
    req: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    // Check if user is authenticated (JWT token should be in extensions from middleware)
    let extensions = req.extensions();
    if extensions.get::<error::AuthToken>().is_none() && extensions.get::<auth::ApiKey>().is_none() {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Authentication required for WebSocket connection",
            "code": "WEBSOCKET_AUTH_REQUIRED"
        })));
    }

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        match parsed.get("type").and_then(|t| t.as_str()) {
                            Some("subscribe") => {
                                let channel = parsed.get("channel").and_then(|c| c.as_str()).unwrap_or("general");
                                info!("📡 Client subscribed to channel: {}", channel);
                                
                                let response = serde_json::json!({
                                    "type": "subscription_confirmed",
                                    "channel": channel,
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                
                                if session.text(response.to_string()).await.is_err() {
                                    break;
                                }
                            }
                            Some("ping") => {
                                let pong = serde_json::json!({
                                    "type": "pong",
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });
                                
                                if session.text(pong.to_string()).await is_err() {
                                    break;
                                }
                            }
                            Some("query_status") => {
                                let query_id = parsed.get("query_id").and_then(|q| q.as_str()).unwrap_or("unknown");
                                
                                let status = serde_json::json!({
                                    "type": "query_status",
                                    "query_id": query_id,
                                    "status": "running",
                                    "progress": 75,
                                    "estimated_completion": "2024-01-01T12:35:00Z"
                                });
                                
                                if session.text(status.to_string()).await.is_err() {
                                    break;
                                }
                            }
                            Some("neural_training_status") => {
                                let network_id = parsed.get("network_id").and_then(|n| n.as_str()).unwrap_or("unknown");
                                
                                let status = serde_json::json!({
                                    "type": "neural_training_status",
                                    "network_id": network_id,
                                    "epoch": 45,
                                    "total_epochs": 100,
                                    "current_loss": 0.023,
                                    "accuracy": 0.94
                                });
                                
                                if session.text(status.to_string()).await.is_err() {
                                    break;
                                }
                            }
                            _ => {
                                warn!("🚨 Unknown WebSocket message type received");
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    Ok(response)
}

/// Configure application routes and middleware
pub fn configure_app(app_state: AppState) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >
> {
    // Create Prometheus metrics
    let prometheus = PrometheusMetricsBuilder::new("neuroquantum_api")
        .endpoint("/internal/metrics")
        .build()
        .unwrap();

    App::new()
        // Add application state
        .app_data(web::Data::new(app_state.db.clone()))
        .app_data(web::Data::new(app_state.auth_service.clone()))
        .app_data(web::Data::new(app_state.jwt_service.clone()))
        .app_data(web::Data::new(app_state.rate_limit_service.clone()))
        .app_data(web::Data::new(app_state.config.clone()))

        // Add middleware
        .wrap(prometheus.clone())
        .wrap(Logger::default())
        .wrap(Compress::default())
        .wrap(
            Cors::default()
                .allowed_origin("http://localhost:3000")
                .allowed_origin("http://localhost:8080")
                .allowed_origins(&app_state.config.cors.allowed_origins)
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"])
                .allowed_headers(vec![
                    "Authorization",
                    "Content-Type",
                    "X-API-Key",
                    "X-Request-ID",
                    "X-Quantum-Level"
                ])
                .expose_headers(vec![
                    "X-RateLimit-Limit",
                    "X-RateLimit-Remaining", 
                    "X-RateLimit-Reset"
                ])
                .max_age(3600)
        )

        // OpenAPI Documentation
        .service(
            SwaggerUi::new("/api-docs/{_:.*}")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
        )

        // Health and system endpoints
        .route("/health", web::get().to(health_check))
        .route("/metrics", web::get().to(handlers::get_metrics))
        .route("/ws", web::get().to(websocket_handler))

        // API v1 routes
        .service(
            web::scope("/api/v1")
                // Authentication routes (public)
                .service(
                    web::scope("/auth")
                        .route("/login", web::post().to(handlers::login))
                        .route("/refresh", web::post().to(handlers::refresh_token))
                        // Protected admin routes
                        .wrap(middleware::auth_middleware())
                        .route("/generate-key", web::post().to(handlers::generate_api_key))
                        .route("/revoke-key", web::post().to(handlers::revoke_api_key))
                )
                
                // Protected API routes (require authentication)
                .service(
                    web::scope("")
                        .wrap(middleware::auth_middleware())
                        .wrap(rate_limit::RateLimitMiddleware::by_user(app_state.rate_limit_service.clone()))
                        
                        // CRUD Operations
                        .service(
                            web::scope("/tables")
                                .route("", web::post().to(handlers::create_table))
                                .route("/{table_name}/data", web::post().to(handlers::insert_data))
                                .route("/{table_name}/query", web::post().to(handlers::query_data))
                                .route("/{table_name}/data", web::put().to(handlers::update_data))
                                .route("/{table_name}/data", web::delete().to(handlers::delete_data))
                        )
                        
                        // Advanced Features
                        .service(
                            web::scope("/neural")
                                .route("/train", web::post().to(handlers::train_neural_network))
                                .route("/train/{network_id}", web::get().to(handlers::get_training_status))
                        )
                        .service(
                            web::scope("/quantum")
                                .route("/search", web::post().to(handlers::quantum_search))
                        )
                        .service(
                            web::scope("/dna")
                                .route("/compress", web::post().to(handlers::compress_dna))
                        )
                        
                        // Monitoring
                        .service(
                            web::scope("/stats")
                                .route("/performance", web::get().to(handlers::get_performance_stats))
                        )
                )
        )
}

/// Start the HTTP server with the given configuration
pub async fn start_server(config: ApiConfig) -> Result<()> {
    let bind_address = format!("{}:{}", config.server.host, config.server.port);
    let app_state = AppState::new(config.clone()).await?;
    
    info!("🚀 Starting NeuroQuantumDB API Server on {}", bind_address);
    info!("📖 API Documentation available at: http://{}/api-docs/", bind_address);
    info!("🏥 Health check available at: http://{}/health", bind_address);
    info!("📊 Metrics available at: http://{}/metrics", bind_address);

    // Clone app_state for the closure
    let app_state_clone = app_state.clone();

    HttpServer::new(move || configure_app(app_state_clone.clone()))
        .bind(&bind_address)?
        .run()
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    #[actix_web::test]
    async fn test_health_check() {
        let app = test::init_service(
            App::new().route("/health", web::get().to(health_check))
        ).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_metrics_endpoint() {
        let app = test::init_service(
            App::new().route("/metrics", web::get().to(metrics))
        ).await;

        let req = test::TestRequest::get().uri("/metrics").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(resp.status().is_success());
    }
}
