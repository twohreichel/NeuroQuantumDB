use serde::{Deserialize, Serialize};
use std::env;
use anyhow::Result;

/// Main API configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub metrics: MetricsConfig,
    pub security: SecurityConfig,
    pub performance: PerformanceConfig,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub keep_alive: u64,
    pub client_timeout: u64,
    pub shutdown_timeout: u64,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub connection_string: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
    pub query_timeout: u64,
    pub neuromorphic_config: NeuromorphicConfig,
    pub quantum_config: QuantumConfig,
    pub dna_config: DnaConfig,
}

/// Neuromorphic layer configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NeuromorphicConfig {
    pub synaptic_strength_threshold: f32,
    pub learning_rate: f32,
    pub plasticity_decay: f32,
    pub max_synaptic_connections: u32,
    pub hebbian_window_ms: u64,
}

/// Quantum layer configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuantumConfig {
    pub default_quantum_level: u8,
    pub grovers_iterations: u32,
    pub annealing_temperature: f32,
    pub superposition_depth: u8,
    pub quantum_error_correction: bool,
}

/// DNA compression configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DnaConfig {
    pub target_compression_ratio: f32,
    pub error_correction_redundancy: u8,
    pub quaternary_encoding_block_size: u32,
    pub protein_folding_levels: u8,
    pub cache_size_mb: u32,
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiry_seconds: u64,
    pub quantum_level: u8,
    pub kyber_key_size: u16,
    pub dilithium_signature_size: u16,
    pub password_hash_cost: u32,
}

/// Metrics and monitoring configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
    pub path: String,
    pub collection_interval_ms: u64,
    pub retention_hours: u32,
    pub export_format: String,
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub quantum_resistant_encryption: bool,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub cors_origins: Vec<String>,
    pub rate_limit_requests_per_minute: u32,
    pub max_request_size_mb: u32,
}

/// Performance optimization configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub arm64_neon_enabled: bool,
    pub power_management_enabled: bool,
    pub max_power_consumption_mw: f32,
    pub cpu_frequency_scaling: bool,
    pub memory_pool_size_mb: u32,
    pub gc_threshold_mb: u32,
}

impl ApiConfig {
    /// Load configuration from environment and config files
    pub fn load() -> Result<Self> {
        let env = env::var("NEUROQUANTUM_ENV").unwrap_or_else(|_| "development".to_string());

        let config_path = match env.as_str() {
            "production" => "config/prod.toml",
            "test" => "config/test.toml",
            _ => "config/dev.toml",
        };

        let config_content = std::fs::read_to_string(config_path)
            .unwrap_or_else(|_| Self::default_config_toml());

        let mut config: ApiConfig = toml::from_str(&config_content)?;

        // Override with environment variables
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(host) = env::var("NEUROQUANTUM_HOST") {
            self.server.host = host;
        }

        if let Ok(port) = env::var("NEUROQUANTUM_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                self.server.port = port_num;
            }
        }

        if let Ok(workers) = env::var("NEUROQUANTUM_WORKERS") {
            if let Ok(worker_count) = workers.parse::<usize>() {
                self.server.workers = worker_count;
            }
        }

        if let Ok(jwt_secret) = env::var("NEUROQUANTUM_JWT_SECRET") {
            self.auth.jwt_secret = jwt_secret;
        }

        if let Ok(db_url) = env::var("NEUROQUANTUM_DATABASE_URL") {
            self.database.connection_string = db_url;
        }

        if let Ok(metrics_enabled) = env::var("NEUROQUANTUM_METRICS_ENABLED") {
            self.metrics.enabled = metrics_enabled.to_lowercase() == "true";
        }
    }

    /// Generate default configuration as TOML string
    fn default_config_toml() -> String {
        r#"
[server]
host = "0.0.0.0"
port = 8080
workers = 4
keep_alive = 300
client_timeout = 30
shutdown_timeout = 30

[database]
connection_string = "neuroquantum://localhost:5432/neuroquantumdb"
max_connections = 100
connection_timeout = 30
query_timeout = 60

[database.neuromorphic_config]
synaptic_strength_threshold = 0.5
learning_rate = 0.01
plasticity_decay = 0.001
max_synaptic_connections = 10000
hebbian_window_ms = 1000

[database.quantum_config]
default_quantum_level = 128
grovers_iterations = 1000
annealing_temperature = 1.0
superposition_depth = 8
quantum_error_correction = true

[database.dna_config]
target_compression_ratio = 1000.0
error_correction_redundancy = 3
quaternary_encoding_block_size = 1024
protein_folding_levels = 4
cache_size_mb = 100

[auth]
jwt_secret = "quantum_resistant_secret_key_change_in_production"
token_expiry_seconds = 3600
quantum_level = 128
kyber_key_size = 1184
dilithium_signature_size = 1952
password_hash_cost = 12

[metrics]
enabled = true
port = 9090
path = "/metrics"
collection_interval_ms = 1000
retention_hours = 24
export_format = "prometheus"

[security]
quantum_resistant_encryption = true
tls_enabled = false
cors_origins = ["http://localhost:3000", "https://neuroquantumdb.org"]
rate_limit_requests_per_minute = 1000
max_request_size_mb = 10

[performance]
arm64_neon_enabled = true
power_management_enabled = true
max_power_consumption_mw = 2000.0
cpu_frequency_scaling = true
memory_pool_size_mb = 512
gc_threshold_mb = 100
"#.to_string()
    }

    /// Get optimized configuration for Raspberry Pi 4
    pub fn raspberry_pi_optimized() -> Self {
        let mut config = Self::load().unwrap_or_else(|_| Self::default());

        // Optimize for Raspberry Pi 4 constraints
        config.server.workers = 4; // Match CPU cores
        config.database.max_connections = 50; // Reduced for 4GB RAM
        config.performance.memory_pool_size_mb = 256; // Conservative memory usage
        config.performance.max_power_consumption_mw = 2000.0; // 2W target
        config.performance.arm64_neon_enabled = true;
        config.performance.power_management_enabled = true;
        config.performance.cpu_frequency_scaling = true;

        // DNA compression optimizations for limited storage
        config.database.dna_config.target_compression_ratio = 1000.0;
        config.database.dna_config.cache_size_mb = 64;

        // Neuromorphic optimizations for edge computing
        config.database.neuromorphic_config.max_synaptic_connections = 5000;
        config.database.neuromorphic_config.plasticity_decay = 0.01; // Faster adaptation

        config
    }

    /// Create test configuration for development and testing
    pub fn test_config() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: 2,
                keep_alive: 30,
                client_timeout: 5000,
                shutdown_timeout: 5000,
            },
            database: DatabaseConfig {
                connection_string: "neuroquantum://localhost:5432/test".to_string(),
                max_connections: 10,
                connection_timeout: 5000,
                query_timeout: 30000,
                neuromorphic_config: NeuromorphicConfig {
                    synaptic_strength_threshold: 0.5,
                    learning_rate: 0.01,
                    plasticity_decay: 0.001,
                    max_synaptic_connections: 1000,
                    hebbian_window_ms: 100,
                },
                quantum_config: QuantumConfig {
                    default_quantum_level: 128,
                    grovers_iterations: 10,
                    annealing_temperature: 1.0,
                    superposition_depth: 4,
                    quantum_error_correction: true,
                },
                dna_config: DnaConfig {
                    target_compression_ratio: 100.0,
                    error_correction_redundancy: 3,
                    quaternary_encoding_block_size: 1024,
                    protein_folding_levels: 2,
                    cache_size_mb: 32,
                },
            },
            auth: AuthConfig {
                jwt_secret: "test_secret_key_for_quantum_auth_development_only".to_string(),
                token_expiry_seconds: 3600,
                quantum_level: 128,
                kyber_key_size: 1184,
                dilithium_signature_size: 1952,
                password_hash_cost: 8,
            },
            metrics: MetricsConfig {
                enabled: true,
                port: 9090,
                path: "/metrics".to_string(),
                collection_interval_ms: 1000,
                retention_hours: 24,
                export_format: "prometheus".to_string(),
            },
            security: SecurityConfig {
                quantum_resistant_encryption: true,
                tls_enabled: false,
                tls_cert_path: None,
                tls_key_path: None,
                cors_origins: vec!["http://localhost:3000".to_string()],
                rate_limit_requests_per_minute: 1000,
                max_request_size_mb: 10,
            },
            performance: PerformanceConfig {
                arm64_neon_enabled: true,
                power_management_enabled: true,
                max_power_consumption_mw: 2000.0,
                cpu_frequency_scaling: true,
                memory_pool_size_mb: 64,
                gc_threshold_mb: 32,
            },
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate server configuration
        if self.server.port == 0 && !cfg!(test) {
            return Err(anyhow::anyhow!("Server port must be specified"));
        }

        if self.server.workers == 0 {
            return Err(anyhow::anyhow!("Number of workers must be greater than 0"));
        }

        // Validate authentication configuration
        if self.auth.jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!("JWT secret must be at least 32 characters"));
        }

        // Validate performance configuration
        if self.performance.max_power_consumption_mw <= 0.0 {
            return Err(anyhow::anyhow!("Maximum power consumption must be positive"));
        }

        // Validate compression configuration
        if self.database.dna_config.target_compression_ratio < 1.0 {
            return Err(anyhow::anyhow!("Compression ratio must be >= 1.0"));
        }

        Ok(())
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self::load().unwrap_or_else(|_| Self::test_config())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = ApiConfig::test_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_raspberry_pi_optimization() {
        let config = ApiConfig::raspberry_pi_optimized();
        assert_eq!(config.server.workers, 4);
        assert!(config.performance.arm64_neon_enabled);
        assert!(config.performance.power_management_enabled);
        assert_eq!(config.performance.max_power_consumption_mw, 2000.0);
    }

    #[test]
    fn test_env_override() {
        env::set_var("NEUROQUANTUM_PORT", "9999");
        let mut config = ApiConfig::test_config();
        config.apply_env_overrides();
        assert_eq!(config.server.port, 9999);
        env::remove_var("NEUROQUANTUM_PORT");
    }

    #[test]
    fn test_config_validation_errors() {
        let mut config = ApiConfig::test_config();

        // Test invalid JWT secret (too short)
        config.auth.jwt_secret = "short".to_string();
        assert!(config.validate().is_err());

        // Test invalid power consumption
        config.auth.jwt_secret = "this_is_a_very_long_secret_key_for_testing".to_string();
        config.performance.max_power_consumption_mw = -100.0;
        assert!(config.validate().is_err());

        // Test invalid compression ratio
        config.performance.max_power_consumption_mw = 2000.0;
        config.database.dna_config.target_compression_ratio = 0.5;
        assert!(config.validate().is_err());
    }
}
