pub mod error;
pub mod handlers;
pub mod auth;
pub mod middleware;
pub mod config;

use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_cors::Cors;
use std::io;
use tracing::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub use error::{ApiError, ApiResponse, ResponseMetadata};
pub use handlers::*;
pub use auth::*;
pub use middleware::*;
pub use config::*;

/// OpenAPI documentation structure
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::quantum_search,
        handlers::execute_qsql,
        handlers::health_check,
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
            error::ApiError,
            error::ApiResponse<handlers::QuantumSearchResponse>,
            error::ResponseMetadata,
        )
    ),
    tags(
        (name = "Quantum Operations", description = "Quantum-enhanced database operations"),
        (name = "QSQL Operations", description = "QSQL query language operations"),
        (name = "System", description = "System health and monitoring"),
        (name = "Authentication", description = "Quantum-resistant authentication")
    ),
    info(
        title = "NeuroQuantumDB REST API",
        version = "1.0.0",
        description = "Production-ready REST API for NeuroQuantumDB with quantum-resistant encryption and neuromorphic optimization",
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
                        .service(
                            web::resource("/quantum-search")
                                .route(web::get().to(handlers::quantum_search))
                        )
                        .service(
                            web::resource("/qsql/execute")
                                .route(web::post().to(handlers::execute_qsql))
                        )
                        .service(
                            web::resource("/health")
                                .route(web::get().to(handlers::health_check))
                        )
                        .service(
                            web::resource("/auth/login")
                                .route(web::post().to(auth::login))
                        )
                        .service(
                            web::resource("/auth/refresh")
                                .route(web::post().to(auth::refresh_token))
                        )
                        .service(
                            web::resource("/metrics")
                                .route(web::get().to(handlers::prometheus_metrics))
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
pub fn init_observability(config: &ApiConfig) -> anyhow::Result<()> {
    // Initialize structured logging
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    // Initialize Prometheus metrics
    if config.metrics.enabled {
        let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
        builder
            .with_http_listener(([0, 0, 0, 0], config.metrics.port))
            .install()?;

        info!("Prometheus metrics enabled on port {}", config.metrics.port);
    }

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
        let db = neuroquantum_core::NeuroQuantumDB::new_test().await.unwrap();
        let auth_service = QuantumAuthService::new_test().unwrap();

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
        assert!(!openapi.paths.is_empty());
        assert!(openapi.info.title == "NeuroQuantumDB REST API");
    }
}
