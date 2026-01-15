use anyhow;
use serde::{Deserialize, Serialize};
use std::path::Path;

// Create a simple database config wrapper that's compatible
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub data_path: String,
    pub max_connections: u32,
    pub connection_timeout_seconds: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            data_path: "./data".to_string(),
            max_connections: 100,
            connection_timeout_seconds: 30,
        }
    }
}

/// Main API configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
    pub redis: Option<RedisConfig>,
    pub logging: LoggingConfig,
    pub tracing: TracingConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub max_connections: usize,
    pub keep_alive: u64,
    pub client_timeout: u64,
    pub client_shutdown: u64,
    pub tls: Option<TlsConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: None, // Use default (number of CPUs)
            max_connections: 25000,
            keep_alive: 75,
            client_timeout: 5000,
            client_shutdown: 5000,
            tls: None,
        }
    }
}

/// TLS configuration for HTTPS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_file: String,
    pub key_file: String,
    pub ca_file: Option<String>,
}

/// JWT authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: u32,
    pub refresh_threshold_minutes: u32,
    pub quantum_enabled: bool,
    pub algorithm: String,
    pub issuer: String,
    pub audience: String,
}

/// List of known insecure default secrets that must not be used in production
const INSECURE_DEFAULT_SECRETS: &[&str] = &[
    "your-super-secret-jwt-key-change-this-in-production",
    "CHANGE_THIS_IMMEDIATELY_USE_openssl_rand_base64_48_MINIMUM_32_CHARS",
    "dev_secret_key_min_32_chars_1234567890abcdef",
    "dev-secret-key-32-characters-long!",
    "",
];

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            // Empty by default - MUST be provided via environment variable or config
            secret: String::new(),
            expiration_hours: 24,
            refresh_threshold_minutes: 60,
            quantum_enabled: false,
            algorithm: "HS256".to_string(),
            issuer: "neuroquantum-db".to_string(),
            audience: "neuroquantum-api".to_string(),
        }
    }
}

impl JwtConfig {
    /// Check if the current secret is a known insecure default
    #[must_use] 
    pub fn is_insecure_default_secret(&self) -> bool {
        INSECURE_DEFAULT_SECRETS.contains(&self.secret.as_str())
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_hour: u32,
    pub burst_allowance: Option<u32>,
    pub enabled: bool,
    pub strategy: RateLimitStrategy,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_hour: 1000,
            burst_allowance: Some(50),
            enabled: true,
            strategy: RateLimitStrategy::TokenBucket,
        }
    }
}

/// Rate limiting strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitStrategy {
    TokenBucket,
    SlidingWindow,
    FixedWindow,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub max_age: u32,
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:8080".to_string(),
                "https://app.neuroquantumdb.com".to_string(),
            ],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Authorization".to_string(),
                "Content-Type".to_string(),
                "X-API-Key".to_string(),
                "X-Request-ID".to_string(),
                "X-Quantum-Level".to_string(),
            ],
            expose_headers: vec![
                "X-RateLimit-Limit".to_string(),
                "X-RateLimit-Remaining".to_string(),
                "X-RateLimit-Reset".to_string(),
                "X-Request-ID".to_string(),
            ],
            max_age: 3600,
            allow_credentials: true,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub max_payload_size: usize,
    pub request_timeout_seconds: u64,
    pub security_headers: bool,
    pub csrf_protection: bool,
    pub quantum_encryption: bool,
    pub circuit_breaker: CircuitBreakerConfig,
    /// Whitelist of IP addresses allowed to access admin endpoints
    /// Empty list = no restrictions (not recommended for production)
    pub admin_ip_whitelist: Vec<String>,
    /// Encryption-at-rest configuration
    #[serde(default)]
    pub encryption: EncryptionAtRestConfig,
}

/// Encryption-at-rest configuration for production security
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EncryptionAtRestConfig {
    /// If true, file-based key storage fallback is forbidden.
    /// The application will fail to start if the OS keychain is unavailable.
    ///
    /// **Recommended: `true` for production deployments.**
    ///
    /// When enabled, the encryption manager will refuse to fall back to
    /// file-based key storage, ensuring that encryption keys are always
    /// stored in the OS keychain (macOS Keychain, Windows Credential Manager,
    /// or Linux Secret Service).
    #[serde(default)]
    pub forbid_file_fallback: bool,

    /// Treat this as a production environment with stricter security checks.
    /// When combined with `forbid_file_fallback`, this provides maximum security.
    ///
    /// When enabled:
    /// - File-based key storage is not allowed
    /// - Keychain unavailability causes startup failure
    /// - Additional security warnings are logged
    #[serde(default)]
    pub production_mode: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_payload_size: 16 * 1024 * 1024, // 16MB
            request_timeout_seconds: 30,
            security_headers: true,
            csrf_protection: false, // Disabled for API-only service
            quantum_encryption: false,
            circuit_breaker: CircuitBreakerConfig::default(),
            admin_ip_whitelist: vec![
                "127.0.0.1".to_string(),
                "::1".to_string(), // IPv6 localhost
            ],
            encryption: EncryptionAtRestConfig::default(),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u64,
    pub success_threshold: u64,
    pub timeout_seconds: u64,
    pub enabled: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_seconds: 60,
            enabled: true,
        }
    }
}

/// Monitoring and metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_enabled: bool,
    pub prometheus_endpoint: String,
    pub health_check_endpoint: String,
    pub performance_stats: bool,
    pub detailed_logging: bool,
    pub websocket_enabled: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            prometheus_endpoint: "/metrics".to_string(),
            health_check_endpoint: "/health".to_string(),
            performance_stats: true,
            detailed_logging: true,
            websocket_enabled: true,
        }
    }
}

/// Redis configuration for caching and rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout_seconds: u64,
    pub command_timeout_seconds: u64,
    pub retry_attempts: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            connection_timeout_seconds: 5,
            command_timeout_seconds: 3,
            retry_attempts: 3,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub file_path: Option<String>,
    pub max_file_size_mb: Option<u64>,
    pub max_files: Option<u32>,
    pub structured_logging: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            file_path: None,
            max_file_size_mb: Some(100),
            max_files: Some(10),
            structured_logging: true,
        }
    }
}

/// Log format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Plain,
    Compact,
}

/// Distributed tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable distributed tracing
    pub enabled: bool,
    /// Exporter type: "jaeger", "otlp", "zipkin", or "console"
    pub exporter: TracingExporter,
    /// Endpoint URL for the tracing backend
    pub endpoint: String,
    /// Sampling rate (0.0 to 1.0, where 1.0 = 100% of traces)
    pub sampling_rate: f64,
    /// Service name for trace identification
    pub service_name: String,
    /// Trace level: "minimal", "detailed", or "debug"
    pub trace_level: TraceLevel,
    /// Additional resource attributes
    pub resource_attributes: Option<std::collections::HashMap<String, String>>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            exporter: TracingExporter::Console,
            endpoint: "http://localhost:4317".to_string(),
            sampling_rate: 0.1,
            service_name: "neuroquantumdb".to_string(),
            trace_level: TraceLevel::Detailed,
            resource_attributes: None,
        }
    }
}

/// Tracing exporter types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TracingExporter {
    /// Jaeger exporter (uses OTLP gRPC)
    Jaeger,
    /// OTLP exporter (gRPC)
    Otlp,
    /// Zipkin exporter
    Zipkin,
    /// Console exporter (for development/debugging)
    Console,
}

/// Trace detail level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceLevel {
    /// Minimal tracing - only HTTP endpoints
    Minimal,
    /// Detailed tracing - includes query execution
    Detailed,
    /// Debug tracing - includes all internal operations
    Debug,
}

impl ApiConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration with environment overrides
    pub fn load() -> anyhow::Result<Self> {
        let mut config = if let Ok(config_path) = std::env::var("NEUROQUANTUM_CONFIG") {
            Self::from_file(config_path)?
        } else if std::path::Path::new("config/dev.toml").exists() {
            Self::from_file("config/dev.toml")?
        } else {
            Self::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides();

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(host) = std::env::var("NEUROQUANTUM_HOST") {
            self.server.host = host;
        }

        if let Ok(port) = std::env::var("NEUROQUANTUM_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                self.server.port = port_num;
            }
        }

        if let Ok(jwt_secret) = std::env::var("NEUROQUANTUM_JWT_SECRET") {
            self.jwt.secret = jwt_secret;
        }

        if let Ok(redis_url) = std::env::var("NEUROQUANTUM_REDIS_URL") {
            self.redis = Some(RedisConfig {
                url: redis_url,
                ..RedisConfig::default()
            });
        }

        if let Ok(log_level) = std::env::var("NEUROQUANTUM_LOG_LEVEL") {
            self.logging.level = log_level;
        }

        if let Ok(max_payload) = std::env::var("NEUROQUANTUM_MAX_PAYLOAD_SIZE") {
            if let Ok(size) = max_payload.parse::<usize>() {
                self.security.max_payload_size = size;
            }
        }

        if let Ok(rate_limit) = std::env::var("NEUROQUANTUM_RATE_LIMIT") {
            if let Ok(limit) = rate_limit.parse::<u32>() {
                self.rate_limit.requests_per_hour = limit;
            }
        }
    }

    /// Validate configuration settings
    fn validate(&self) -> anyhow::Result<()> {
        // Check if JWT secret is a known insecure default
        if self.jwt.is_insecure_default_secret() {
            return Err(anyhow::anyhow!(
                "\n\n✘ CRITICAL SECURITY ERROR: Insecure JWT secret detected!\n\n\
                 The JWT secret is either empty or a known insecure default value.\n\
                 This is NOT allowed in production.\n\n\
                 To fix this:\n\
                 1. Generate a secure secret: openssl rand -base64 48\n\
                 2. Set the environment variable: export NEUROQUANTUM_JWT_SECRET=<your-secret>\n\
                 3. Restart the application\n\n\
                 The secret must be at least 32 characters long.\n"
            ));
        }

        // Validate JWT secret strength
        if self.jwt.secret.len() < 32 {
            return Err(anyhow::anyhow!(
                "JWT secret must be at least 32 characters long for security"
            ));
        }

        // Validate server configuration
        if self.server.port < 1024 && std::env::var("USER").unwrap_or_default() != "root" {
            return Err(anyhow::anyhow!(
                "Port {} requires root privileges. Use port >= 1024 for non-root users",
                self.server.port
            ));
        }

        // Validate rate limiting
        if self.rate_limit.enabled && self.rate_limit.requests_per_hour == 0 {
            return Err(anyhow::anyhow!(
                "Rate limit requests_per_hour must be greater than 0 when rate limiting is enabled"
            ));
        }

        // Validate payload size
        if self.security.max_payload_size > 100 * 1024 * 1024 {
            tracing::warn!(
                "Max payload size is very large ({}MB). This may impact performance.",
                self.security.max_payload_size / (1024 * 1024)
            );
        }

        // Validate Redis URL if provided
        if let Some(redis_config) = &self.redis {
            if !redis_config.url.starts_with("redis://")
                && !redis_config.url.starts_with("rediss://")
            {
                return Err(anyhow::anyhow!(
                    "Invalid Redis URL format. Must start with redis:// or rediss://"
                ));
            }
        }

        Ok(())
    }

    /// Get the bind address for the server
    #[must_use] 
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Check if TLS is enabled
    #[must_use] 
    pub const fn is_tls_enabled(&self) -> bool {
        self.server.tls.is_some()
    }

    /// Get the base URL for the API
    #[must_use] 
    pub fn base_url(&self) -> String {
        let protocol = if self.is_tls_enabled() {
            "https"
        } else {
            "http"
        };
        format!("{}://{}", protocol, self.bind_address())
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Create a development configuration
    #[must_use] 
    pub fn development() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: Some(2),
                ..ServerConfig::default()
            },
            jwt: JwtConfig {
                secret: "dev-secret-key-32-characters-long!".to_string(),
                expiration_hours: 24,
                quantum_enabled: false,
                ..JwtConfig::default()
            },
            rate_limit: RateLimitConfig {
                requests_per_hour: 10000,
                enabled: true,
                ..RateLimitConfig::default()
            },
            security: SecurityConfig {
                max_payload_size: 32 * 1024 * 1024, // 32MB for dev
                quantum_encryption: false,
                ..SecurityConfig::default()
            },
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: LogFormat::Plain,
                structured_logging: false,
                ..LoggingConfig::default()
            },
            ..Self::default()
        }
    }

    /// Create a production configuration
    #[must_use] 
    pub fn production() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 443,
                workers: None,
                max_connections: 50000,
                tls: Some(TlsConfig {
                    cert_file: "/etc/ssl/certs/neuroquantum.crt".to_string(),
                    key_file: "/etc/ssl/private/neuroquantum.key".to_string(),
                    ca_file: None,
                }),
                ..ServerConfig::default()
            },
            jwt: JwtConfig {
                expiration_hours: 8, // Shorter expiration for production
                quantum_enabled: true,
                ..JwtConfig::default()
            },
            rate_limit: RateLimitConfig {
                requests_per_hour: 1000,
                enabled: true,
                ..RateLimitConfig::default()
            },
            security: SecurityConfig {
                max_payload_size: 8 * 1024 * 1024, // 8MB for production
                quantum_encryption: true,
                security_headers: true,
                ..SecurityConfig::default()
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: LogFormat::Json,
                structured_logging: true,
                file_path: Some("/var/log/neuroquantum/api.log".to_string()),
                ..LoggingConfig::default()
            },
            redis: Some(RedisConfig {
                url: "redis://redis.neuroquantum.internal:6379".to_string(),
                pool_size: 20,
                ..RedisConfig::default()
            }),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ApiConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert!(config.rate_limit.enabled);
        assert!(config.monitoring.metrics_enabled);
    }

    #[test]
    fn test_development_config() {
        let config = ApiConfig::development();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.logging.level, "debug");
        assert!(!config.jwt.quantum_enabled);
    }

    #[test]
    fn test_production_config() {
        let config = ApiConfig::production();
        assert_eq!(config.server.port, 443);
        assert_eq!(config.logging.level, "info");
        assert!(config.jwt.quantum_enabled);
        assert!(config.security.quantum_encryption);
        assert!(config.server.tls.is_some());
    }

    #[test]
    fn test_config_validation() {
        let mut config = ApiConfig::default();

        // Test invalid JWT secret
        config.jwt.secret = "short".to_string();
        assert!(config.validate().is_err());

        // Test valid config
        config.jwt.secret = "this-is-a-valid-32-character-secret!".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bind_address() {
        let config = ApiConfig::default();
        assert_eq!(config.bind_address(), "127.0.0.1:8080");
    }

    #[test]
    fn test_base_url() {
        let config = ApiConfig::default();
        assert_eq!(config.base_url(), "http://127.0.0.1:8080");

        let mut tls_config = ApiConfig::default();
        tls_config.server.tls = Some(TlsConfig {
            cert_file: "test.crt".to_string(),
            key_file: "test.key".to_string(),
            ca_file: None,
        });
        assert_eq!(tls_config.base_url(), "https://127.0.0.1:8080");
    }
}
