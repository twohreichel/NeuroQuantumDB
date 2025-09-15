pub mod error;
pub mod handlers;
pub mod auth;
pub mod middleware;
pub mod config;

use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_cors::Cors;
use std::io;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub use error::{ApiError, ApiResponse, ResponseMetadata};
pub use handlers::{
    quantum_search, execute_qsql, health_check, generate_api_key, neuromorphic_query,
    network_status, train_network, quantum_optimize, quantum_status, dna_compress,
    dna_decompress, dna_repair, get_config, update_config, prometheus_metrics
};
pub use auth::*;
pub use middleware::*;
pub use config::{ApiConfig, ServerConfig, DatabaseConfig, AuthConfig};

/// OpenAPI documentation structure
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::quantum_search,
        handlers::execute_qsql,
        handlers::health_check,
        handlers::generate_api_key,
        handlers::neuromorphic_query,
        handlers::network_status,
        handlers::train_network,
        handlers::quantum_optimize,
        handlers::quantum_status,
        handlers::dna_compress,
        handlers::dna_decompress,
        handlers::dna_repair,
        handlers::get_config,
        handlers::update_config,
    ),
    components(
        schemas(
            handlers::QuantumSearchRequest,
            handlers::QuantumSearchResponse,
            handlers::QSQLRequest,
            handlers::QSQLResponse,
            handlers::SystemHealth,
            handlers::SearchResult,
            handlers::QueryMetrics,
            handlers::GenerateKeyRequest,
            handlers::GenerateKeyResponse,
            handlers::NeuromorphicQueryRequest,
            handlers::NeuromorphicQueryResponse,
            handlers::NetworkStatusResponse,
            handlers::TrainingRequest,
            handlers::OptimizationRequest,
            handlers::OptimizationResponse,
            handlers::QuantumStatusResponse,
            handlers::CompressionRequest,
            handlers::CompressionResponse,
            handlers::DecompressionRequest,
            handlers::DecompressionResponse,
            handlers::RepairRequest,
            handlers::RepairResponse,
            handlers::ConfigResponse,
            handlers::ConfigUpdateRequest,
            handlers::ConfigUpdateResponse,
            error::ApiError,
        )
    ),
    tags(
        (name = "Authentication", description = "API key management and quantum-resistant authentication"),
        (name = "Neuromorphic", description = "Neuromorphic computing and synaptic network operations"),
        (name = "Quantum Operations", description = "Quantum-enhanced database operations"),
        (name = "DNA Storage", description = "DNA-based compression and storage operations"),
        (name = "QSQL Operations", description = "QSQL query language operations"),
        (name = "Admin", description = "Configuration and administration"),
        (name = "System", description = "System health and monitoring"),
    ),
    info(
        title = "NeuroQuantumDB REST API",
        version = "1.0.0",
        description = "Production-ready REST API for NeuroQuantumDB with neuromorphic computing, quantum processing, and DNA storage",
        contact(
            name = "NeuroQuantumDB Team",
            email = "api@neuroquantumdb.org"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Development server"),
        (url = "https://api.neuroquantumdb.org", description = "Production server")
    )
)]
pub struct ApiDoc;

/// Main API server configuration and startup
pub struct ApiServer {
    config: ApiConfig,
}

impl ApiServer {
    pub fn new(config: ApiConfig) -> Self {
        Self { config }
    }

    /// Start the API server with all middleware and routes configured
    pub async fn start(self) -> io::Result<()> {
        info!(
            "🚀 Starting NeuroQuantumDB API server on {}:{}",
            self.config.server.host, self.config.server.port
        );

        // Initialize database connection
        let db = neuroquantum_core::NeuroQuantumDB::new(&neuroquantum_core::DatabaseConfig {
            connection_string: self.config.database.connection_string.clone(),
            max_connections: self.config.database.max_connections,
        }).await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Initialize quantum authentication service
        let auth_service = QuantumAuthService::new(&self.config.auth)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        HttpServer::new(move || {
            // Configure CORS for edge deployment
            let cors = Cors::default()
                .allowed_origin_fn(|origin, _req_head| {
                    origin.as_bytes().starts_with(b"https://") ||
                    origin.as_bytes().starts_with(b"http://localhost")
                })
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec!["Authorization", "Content-Type", "X-Quantum-Level"])
                .supports_credentials()
                .max_age(3600);

            App::new()
                .app_data(web::Data::new(db.clone()))
                .app_data(web::Data::new(auth_service.clone()))

                // Middleware stack
                .wrap(cors)
                .wrap(Logger::default())
                .wrap(QuantumSecurityMiddleware::new())
                .wrap(RateLimitMiddleware::new(1000, 60)) // 1000 requests per minute
                .wrap(PowerOptimizationMiddleware::new())

                // API routes
                .service(
                    web::scope("/api/v1")
                        // 🔑 Authentication endpoints
                        .service(
                            web::resource("/auth/generate-key")
                                .route(web::post().to(handlers::generate_api_key))
                        )
                        .service(
                            web::resource("/auth/login")
                                .route(web::post().to(auth::login))
                        )
                        .service(
                            web::resource("/auth/refresh")
                                .route(web::post().to(auth::refresh_token))
                        )

                        // 🧠 Neuromorphic endpoints
                        .service(
                            web::resource("/neuromorphic/query")
                                .route(web::post().to(handlers::neuromorphic_query))
                        )
                        .service(
                            web::resource("/neuromorphic/network-status")
                                .route(web::get().to(handlers::network_status))
                        )
                        .service(
                            web::resource("/neuromorphic/train")
                                .route(web::post().to(handlers::train_network))
                        )

                        // ⚛️ Quantum endpoints
                        .service(
                            web::resource("/quantum/search")
                                .route(web::post().to(handlers::quantum_search))
                        )
                        .service(
                            web::resource("/quantum/optimize")
                                .route(web::post().to(handlers::quantum_optimize))
                        )
                        .service(
                            web::resource("/quantum/status")
                                .route(web::get().to(handlers::quantum_status))
                        )

                        // 🧬 DNA Storage endpoints
                        .service(
                            web::resource("/dna/compress")
                                .route(web::post().to(handlers::dna_compress))
                        )
                        .service(
                            web::resource("/dna/decompress")
                                .route(web::post().to(handlers::dna_decompress))
                        )
                        .service(
                            web::resource("/dna/repair")
                                .route(web::post().to(handlers::dna_repair))
                        )

                        // 📊 Admin & Monitoring endpoints
                        .service(
                            web::resource("/admin/config")
                                .route(web::get().to(handlers::get_config))
                                .route(web::put().to(handlers::update_config))
                        )
                        .service(
                            web::resource("/health")
                                .route(web::get().to(handlers::health_check))
                        )
                        .service(
                            web::resource("/metrics")
                                .route(web::get().to(handlers::prometheus_metrics))
                        )

                        // Legacy QSQL endpoint
                        .service(
                            web::resource("/qsql/execute")
                                .route(web::post().to(handlers::execute_qsql))
                        )
                )

                // OpenAPI documentation
                .service(
                    SwaggerUi::new("/swagger-ui/{_:.*}")
                        .url("/api-docs/openapi.json", ApiDoc::openapi())
                )

                // Health check at root for load balancers
                .route("/", web::get().to(handlers::health_check))
        })
        .bind(format!("{}:{}", self.config.server.host, self.config.server.port))?
        .workers(self.config.server.workers)
        .run()
        .await
    }
}

/// Initialize tracing and metrics for production deployment
pub fn init_observability(_config: &ApiConfig) -> anyhow::Result<()> {
    // Initialize structured logging
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .with_thread_ids(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Note: Prometheus metrics are disabled for now due to dependency issues
    // This can be re-enabled once the metrics dependencies are resolved
    info!("Observability initialized (metrics disabled)");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    async fn create_test_app() -> App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        let config = ApiConfig::test_config();

        // Create test database with test configuration
        let db_config = neuroquantum_core::DatabaseConfig {
            connection_string: "test://localhost".to_string(),
            max_connections: 1,
        };
        let db = neuroquantum_core::NeuroQuantumDB::new(&db_config).await.unwrap();

        // Create test auth service with test configuration
        let auth_service = QuantumAuthService::new(&config.auth).unwrap();

        App::new()
            .app_data(web::Data::new(db))
            .app_data(web::Data::new(auth_service))
            .service(
                web::scope("/api/v1")
                    .service(
                        web::resource("/health")
                            .route(web::get().to(handlers::health_check))
                    )
            )
    }

    #[actix_web::test]
    async fn test_health_endpoint() {
        let app = test::init_service(create_test_app().await).await;
        let req = test::TestRequest::get()
            .uri("/api/v1/health")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_openapi_spec() {
        let openapi = ApiDoc::openapi();
        assert!(openapi.info.title == "NeuroQuantumDB REST API");
    }
}
