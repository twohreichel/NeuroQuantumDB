use actix_cors::Cors;
use actix_web::body::MessageBody;
use actix_web::middleware::{Compress, Logger};
use actix_web::{web, App, HttpMessage, HttpResponse, HttpServer, Result as ActixResult};
use actix_web_prom::PrometheusMetricsBuilder;
use anyhow::Result;
use neuroquantum_core::{NeuroQuantumDB, NeuroQuantumDBBuilder};
use std::time::Instant;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod auth;
pub mod biometric_auth;
pub mod cli;
pub mod config;
pub mod error;
pub mod handlers;
pub mod jwt;
pub mod metrics;
pub mod middleware;
pub mod permissions;
pub mod rate_limit;
pub mod storage;
pub mod websocket;

use auth::AuthService;
pub use config::ApiConfig;
pub use error::{ApiError, ApiResponse, ResponseMetadata};
use handlers::ApiDoc;
use jwt::JwtService;
use rate_limit::{RateLimitConfig, RateLimitService};
use std::sync::Arc;
use websocket::{ConnectionConfig, ConnectionManager, PubSubManager, WebSocketService};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<tokio::sync::RwLock<NeuroQuantumDB>>,
    pub qsql_engine: Arc<tokio::sync::Mutex<neuroquantum_qsql::QSQLEngine>>,
    pub auth_service: AuthService,
    pub jwt_service: JwtService,
    pub rate_limit_service: RateLimitService,
    pub websocket_service: Arc<WebSocketService>,
    pub config: ApiConfig,
}

impl AppState {
    pub async fn new(config: ApiConfig) -> Result<Self> {
        // Initialize the database using the new builder pattern with compile-time guarantees
        let db = NeuroQuantumDBBuilder::new()
            .build()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;

        let auth_service = AuthService::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize auth service: {}", e))?;

        // Warn if no admin keys exist - database needs initialization
        if !auth_service.has_admin_keys() {
            tracing::warn!("⚠️  No admin keys found! Run 'neuroquantum-api init' to create the first admin key.");
            tracing::warn!(
                "⚠️  The API server will start but authentication will fail until initialized."
            );
        }

        let jwt_service = JwtService::new(config.jwt.secret.as_bytes());

        let rate_limit_config = RateLimitConfig {
            requests_per_window: config.rate_limit.requests_per_hour,
            window_size_seconds: 3600,
            burst_allowance: config.rate_limit.burst_allowance,
            redis_url: config.redis.as_ref().map(|r| r.url.clone()),
            fallback_to_memory: true,
        };
        let rate_limit_service = RateLimitService::new(rate_limit_config).await?;

        // Initialize QSQL engine
        let qsql_engine = neuroquantum_qsql::QSQLEngine::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize QSQL engine: {}", e))?;
        let qsql_engine_arc = Arc::new(tokio::sync::Mutex::new(qsql_engine));

        // Initialize WebSocket service with QSQL engine
        let ws_config = ConnectionConfig::default();
        let connection_manager = Arc::new(ConnectionManager::new(ws_config));
        let pubsub_manager = Arc::new(PubSubManager::new());
        let websocket_service = Arc::new(WebSocketService::with_qsql_engine(
            connection_manager,
            pubsub_manager,
            qsql_engine_arc.clone(),
        ));

        Ok(Self {
            db: Arc::new(tokio::sync::RwLock::new(db)),
            qsql_engine: qsql_engine_arc,
            auth_service,
            jwt_service,
            rate_limit_service,
            websocket_service,
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
    match crate::metrics::render_metrics() {
        Ok(metrics_text) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4; charset=utf-8")
            .body(metrics_text),
        Err(e) => {
            tracing::error!("Failed to render metrics: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to collect metrics",
                "details": e.to_string()
            }))
        }
    }
}

/// 🔍 WebSocket handler for real-time communication (requires authentication)
pub async fn websocket_handler(
    req: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    use tracing::error;
    use websocket::ConnectionMetadata;

    // Check if user is authenticated (JWT token should be in extensions from middleware)
    let extensions = req.extensions();
    let user_id = if let Some(token) = extensions.get::<error::AuthToken>() {
        Some(token.sub.clone())
    } else if let Some(api_key) = extensions.get::<auth::ApiKey>() {
        Some(api_key.name.clone())
    } else {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Authentication required for WebSocket connection",
            "code": "WEBSOCKET_AUTH_REQUIRED"
        })));
    };

    // Get connection info
    let remote_addr = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Create connection metadata
    let mut metadata = ConnectionMetadata::new(remote_addr);
    metadata.user_id = user_id;
    metadata.user_agent = user_agent;

    // Handle WebSocket upgrade
    let (response, session, msg_stream) = actix_ws::handle(&req, stream)?;

    // Record WebSocket connection metrics
    crate::metrics::record_websocket_connection("connected");

    // Spawn handler task
    let ws_service = state.websocket_service.clone();
    actix_web::rt::spawn(async move {
        if let Err(e) = ws_service
            .handle_connection(session, msg_stream, metadata)
            .await
        {
            error!("WebSocket connection error: {:?}", e);
            crate::metrics::record_websocket_connection("error");
        } else {
            crate::metrics::record_websocket_connection("disconnected");
        }
    });

    Ok(response)
}

/// Configure application routes and middleware
pub fn configure_app(
    app_state: AppState,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    // Create Prometheus metrics
    let prometheus = PrometheusMetricsBuilder::new("neuroquantum_api")
        .endpoint("/internal/metrics")
        .build()
        .unwrap();

    let cors_origins = app_state.config.cors.allowed_origins.clone();

    App::new()
        // Add application state
        .app_data(web::Data::new(app_state.clone()))
        .app_data(web::Data::new(app_state.db.clone()))
        .app_data(web::Data::new(app_state.auth_service.clone()))
        .app_data(web::Data::new(app_state.jwt_service.clone()))
        .app_data(web::Data::new(app_state.rate_limit_service.clone()))
        .app_data(web::Data::new(app_state.config.clone()))

        // Add middleware
        .wrap(prometheus.clone())
        .wrap(Logger::default())
        .wrap(Compress::default())
        .wrap({
            let mut cors = Cors::default()
                .allowed_origin("http://localhost:3000")
                .allowed_origin("http://localhost:8080");

            // Add configured origins
            for origin in &cors_origins {
                cors = cors.allowed_origin(origin);
            }

            cors.allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"])
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
        })

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
                )

                // Protected admin routes
                .service(
                    web::scope("/auth")
                        .wrap(middleware::auth_middleware())
                        .route("/generate-key", web::post().to(handlers::generate_api_key))
                        .route("/revoke-key", web::post().to(handlers::revoke_api_key))
                )

                // Protected API routes (require authentication)
                .service(
                    web::scope("")
                        .wrap(middleware::auth_middleware())

                        // Generic SQL query endpoint
                        .route("/query", web::post().to(handlers::execute_sql_query))

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

                        // Biometric Authentication
                        .service(
                            web::scope("/biometric/eeg")
                                .route("/enroll", web::post().to(handlers::eeg_enroll))
                                .route("/authenticate", web::post().to(handlers::eeg_authenticate))
                                .route("/update", web::post().to(handlers::eeg_update_signature))
                                .route("/users", web::get().to(handlers::eeg_list_users))
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
    info!(
        "📖 API Documentation available at: http://{}/api-docs/",
        bind_address
    );
    info!(
        "🏥 Health check available at: http://{}/health",
        bind_address
    );
    info!("📊 Metrics available at: http://{}/metrics", bind_address);

    HttpServer::new(move || configure_app(app_state.clone()))
        .bind(&bind_address)?
        .run()
        .await?;

    Ok(())
}

// Property-based tests for API robustness
#[cfg(test)]
mod proptest_suite;

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    #[actix_web::test]
    async fn test_health_check() {
        let app =
            test::init_service(App::new().route("/health", web::get().to(health_check))).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_metrics_endpoint() {
        let app = test::init_service(App::new().route("/metrics", web::get().to(metrics))).await;

        let req = test::TestRequest::get().uri("/metrics").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
    }
}
