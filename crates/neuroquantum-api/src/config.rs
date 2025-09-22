use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub keep_alive: Option<u32>,
    pub client_timeout: Option<u32>,
    pub shutdown_timeout: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub connection_string: Option<String>,
    pub max_connections: Option<u32>,
    pub connection_timeout: Option<u32>,
    pub query_timeout: Option<u32>,
    pub neuromorphic_config: Option<NeuromorphicConfig>,
    pub quantum_config: Option<QuantumConfig>,
    pub dna_config: Option<DnaConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NeuromorphicConfig {
    pub synaptic_strength_threshold: Option<f32>,
    pub learning_rate: f32,
    pub plasticity_decay: Option<f32>,
    pub max_synaptic_connections: Option<u64>,
    pub hebbian_window_ms: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuantumConfig {
    pub default_quantum_level: Option<u32>,
    pub grovers_iterations: Option<u32>,
    pub annealing_temperature: Option<f32>,
    pub superposition_depth: Option<u32>,
    pub quantum_error_correction: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DnaConfig {
    pub target_compression_ratio: Option<f32>,
    pub error_correction_redundancy: Option<u32>,
    pub quaternary_encoding_block_size: Option<u32>,
    pub protein_folding_levels: Option<u32>,
    pub cache_size_mb: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiry_seconds: Option<u32>,
    pub quantum_level: Option<u32>,
    pub kyber_key_size: Option<u32>,
    pub dilithium_signature_size: Option<u32>,
    pub password_hash_cost: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
    pub path: String,
    pub collection_interval_ms: Option<u32>,
    pub retention_hours: Option<u32>,
    pub export_format: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: 4,
                keep_alive: None,
                client_timeout: None,
                shutdown_timeout: None,
            },
            database: DatabaseConfig {
                connection_string: None,
                max_connections: None,
                connection_timeout: None,
                query_timeout: None,
                neuromorphic_config: None,
                quantum_config: None,
                dna_config: None,
            },
            auth: AuthConfig {
                jwt_secret: "neuroquantum-secret-key".to_string(),
                token_expiry_seconds: Some(3600), // 1 hour
                quantum_level: Some(4),
                kyber_key_size: Some(1024),
                dilithium_signature_size: Some(2420),
                password_hash_cost: Some(12),
            },
            metrics: MetricsConfig {
                enabled: true,
                port: 9090,
                path: "/metrics".to_string(),
                collection_interval_ms: None,
                retention_hours: None,
                export_format: None,
            },
        }
    }
}

impl ApiConfig {
    pub fn load() -> Result<Self> {
        // Check for config file path from environment variable or use default
        let config_path =
            env::var("NEUROQUANTUM_CONFIG").unwrap_or_else(|_| "config/prod.toml".to_string());

        // Try to load from file first
        if let Ok(content) = fs::read_to_string(&config_path) {
            let mut config: ApiConfig = toml::from_str(&content)?;

            // Override with environment variables if present
            if let Ok(host) = env::var("NEUROQUANTUM_HOST") {
                config.server.host = host;
            }
            if let Ok(port) = env::var("NEUROQUANTUM_PORT") {
                config.server.port = port.parse()?;
            }
            if let Ok(workers) = env::var("NEUROQUANTUM_WORKERS") {
                config.server.workers = workers.parse()?;
            }

            Ok(config)
        } else {
            // Fallback to default if file doesn't exist
            Ok(Self::default())
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        if self.server.workers == 0 {
            return Err(anyhow::anyhow!("Server workers cannot be 0"));
        }

        if let Some(neuromorphic) = &self.database.neuromorphic_config {
            if neuromorphic.learning_rate <= 0.0 || neuromorphic.learning_rate > 1.0 {
                return Err(anyhow::anyhow!(
                    "Neuromorphic learning rate must be between 0.0 and 1.0"
                ));
            }
        }

        if let Some(quantum) = &self.database.quantum_config {
            if quantum.default_quantum_level.unwrap_or(1) == 0 {
                return Err(anyhow::anyhow!("Quantum default level cannot be 0"));
            }
        }

        if let Some(dna) = &self.database.dna_config {
            if let Some(compression_ratio) = dna.target_compression_ratio {
                if compression_ratio > 1000.0 {
                    return Err(anyhow::anyhow!(
                        "DNA target compression ratio cannot exceed 1000:1"
                    ));
                }
                if compression_ratio < 1.0 {
                    return Err(anyhow::anyhow!(
                        "DNA target compression ratio must be at least 1:1"
                    ));
                }
            }
        }

        Ok(())
    }
}
