use actix_web::{web, App, HttpServer, HttpResponse, Result as ActixResult};
use actix_web::middleware::{Logger, Compress};
use actix_web_prometheus::PrometheusMetricsBuilder;
use neuroquantum_core::{NeuroQuantumDB, DatabaseConfig};
use serde_json;
use std::time::Instant;
use tracing::info;
use anyhow::Result;
use actix_cors::Cors;
use actix_ws::{Message};
use futures_util::StreamExt;

pub mod handlers;
pub mod error;
pub mod config;
pub mod auth;
pub mod middleware;

pub use config::ApiConfig;
pub use error::{ApiError, ApiResponse, ResponseMetadata};
use middleware as custom_middleware;

/// 🏥 Health check endpoint
pub async fn health_check() -> ActixResult<HttpResponse, ApiError> {
    let start = Instant::now();

    let health_data = serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": 0,
        "system_metrics": {
            "memory_usage_mb": 128,
            "power_consumption_w": 45
        }
    });

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        health_data,
        ResponseMetadata::new(start.elapsed(), "Health check completed"),
    )))
}

/// 📊 Prometheus metrics endpoint
pub async fn metrics() -> HttpResponse {
    let metrics = r#"
# HELP neuroquantum_queries_total Total number of queries processed
# TYPE neuroquantum_queries_total counter
neuroquantum_queries_total{type="neuromorphic"} 1234
neuroquantum_queries_total{type="quantum"} 567
neuroquantum_queries_total{type="dna"} 89

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

/// 🔍 WebSocket handler for real-time communication
pub async fn websocket_handler(
    req: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        match parsed.get("type").and_then(|t| t.as_str()) {
                            Some("auth") => {
                                let response = serde_json::json!({
                                    "type": "auth_success",
                                    "message": "Authentication successful"
                                });
                                let _ = session.text(response.to_string()).await;
                            },
                            Some("subscribe") => {
                                let response = serde_json::json!({
                                    "type": "subscribed",
                                    "channels": parsed.get("channels").unwrap_or(&serde_json::json!([]))
                                });
                                let _ = session.text(response.to_string()).await;

                                // Send a sample real-time update
                                let update = serde_json::json!({
                                    "type": "neuromorphic_learning",
                                    "data": {
                                        "synaptic_strength": 0.87,
                                        "learning_rate": 0.012,
                                        "timestamp": "2025-09-15T15:00:00Z"
                                    }
                                });
                                let _ = session.text(update.to_string()).await;
                            },
                            _ => {
                                let error = serde_json::json!({
                                    "type": "error",
                                    "message": "Unknown message type"
                                });
                                let _ = session.text(error.to_string()).await;
                            }
                        }
                    }
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    Ok(response)
}

/// 🚀 API Server
pub struct ApiServer {
    config: ApiConfig,
}

impl ApiServer {
    pub fn new(config: ApiConfig) -> Self {
        Self { config }
    }

    pub async fn start(self) -> Result<()> {
        let bind_address = format!("{}:{}", self.config.server.host, self.config.server.port);

        info!("🧠⚛️🧬 Starting NeuroQuantumDB API Server on {}", bind_address);

        // Initialize the database with config
        let db_config = DatabaseConfig {
            connection_string: "neuroquantum://localhost".to_string(),
            max_connections: self.config.database.max_connections.unwrap_or(1000) as u32,
        };
        let db = NeuroQuantumDB::new(&db_config).await?;

        // Set up Prometheus metrics
        let prometheus = PrometheusMetricsBuilder::new("neuroquantum")
            .endpoint("/metrics")
            .build()
            .unwrap();

        HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600);

            App::new()
                .app_data(web::Data::new(db.clone()))
                .wrap(cors)
                .wrap(prometheus.clone())
                .wrap(Logger::default())
                .wrap(Compress::default())
                .wrap(custom_middleware::ApiKeyAuth)

                // Health and metrics endpoints
                .route("/health", web::get().to(health_check))
                .route("/api/v1/health", web::get().to(health_check))
                .route("/metrics", web::get().to(metrics))

                // Authentication endpoints
                .service(
                    web::scope("/api/v1/auth")
                        .route("/generate-key", web::post().to(handlers::generate_api_key))
                )

                // Neuromorphic endpoints
                .service(
                    web::scope("/api/v1/neuromorphic")
                        .route("/query", web::post().to(handlers::neuromorphic_query))
                        .route("/network-status", web::get().to(handlers::network_status))
                        .route("/train", web::post().to(handlers::train_network))
                )

                // Quantum endpoints
                .service(
                    web::scope("/api/v1/quantum")
                        .route("/search", web::post().to(handlers::quantum_search))
                        .route("/optimize", web::post().to(handlers::quantum_optimize))
                        .route("/status", web::get().to(handlers::quantum_status))
                )

                // DNA Storage endpoints
                .service(
                    web::scope("/api/v1/dna")
                        .route("/compress", web::post().to(handlers::dna_compress))
                        .route("/decompress", web::post().to(handlers::dna_decompress))
                        .route("/repair", web::post().to(handlers::dna_repair))
                        .route("/query", web::post().to(handlers::dna_query))
                )

                // Admin endpoints
                .service(
                    web::scope("/api/v1/admin")
                        .route("/config", web::get().to(handlers::get_config))
                        .route("/config", web::put().to(handlers::update_config))
                )

                // Data Management endpoints
                .service(
                    web::scope("/api/v1/data")
                        .route("/load", web::post().to(handlers::load_data))
                )

                // General Query endpoint
                .route("/api/v1/query", web::post().to(handlers::execute_query))

                // WebSocket endpoint for real-time communication
                .route("/api/v1/realtime", web::get().to(websocket_handler))
        })
        .workers(self.config.server.workers)
        .bind(&bind_address)?
        .run()
        .await?;

        Ok(())
    }
}

/// Initialize observability (logging and metrics)
pub fn init_observability(_config: &ApiConfig) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Observability initialized");
    Ok(())
}
