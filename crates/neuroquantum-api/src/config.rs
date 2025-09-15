use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub max_connections: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub neuromorphic: NeuromorphicConfig,
    pub quantum: QuantumConfig,
    pub dna: DnaConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NeuromorphicConfig {
    pub learning_rate: f32,
    pub plasticity_threshold: f32,
    pub max_synapses: u64,
    pub auto_optimization: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuantumConfig {
    pub processors: u32,
    pub grover_iterations: u32,
    pub annealing_steps: u32,
    pub error_correction: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DnaConfig {
    pub compression_level: u8,
    pub error_correction: bool,
    pub cache_size_mb: u32,
    pub biological_patterns: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt_secret: String,
    pub api_key_expiry_hours: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub prometheus_enabled: bool,
    pub metrics_path: String,
    pub health_check_interval: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                workers: 4,
                max_connections: 1000,
            },
            database: DatabaseConfig {
                neuromorphic: NeuromorphicConfig {
                    learning_rate: 0.012,
                    plasticity_threshold: 0.5,
                    max_synapses: 1000000,
                    auto_optimization: true,
                },
                quantum: QuantumConfig {
                    processors: 4,
                    grover_iterations: 15,
                    annealing_steps: 1000,
                    error_correction: true,
                },
                dna: DnaConfig {
                    compression_level: 9,
                    error_correction: true,
                    cache_size_mb: 512,
                    biological_patterns: true,
                },
            },
            auth: AuthConfig {
                enabled: true,
                jwt_secret: "neuroquantum-secret-key".to_string(),
                api_key_expiry_hours: 24,
            },
            monitoring: MonitoringConfig {
                prometheus_enabled: true,
                metrics_path: "/metrics".to_string(),
                health_check_interval: 30,
            },
        }
    }
}

impl ApiConfig {
    pub fn load() -> Result<Self> {
        // For now, return default config
        // In the future, this could load from TOML files, environment variables, etc.
        Ok(Self::default())
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        if self.server.workers == 0 {
            return Err(anyhow::anyhow!("Server workers cannot be 0"));
        }

        if self.database.neuromorphic.learning_rate <= 0.0 || self.database.neuromorphic.learning_rate > 1.0 {
            return Err(anyhow::anyhow!("Neuromorphic learning rate must be between 0.0 and 1.0"));
        }

        if self.database.quantum.processors == 0 {
            return Err(anyhow::anyhow!("Quantum processors cannot be 0"));
        }

        if self.database.dna.compression_level > 9 {
            return Err(anyhow::anyhow!("DNA compression level cannot exceed 9"));
        }

        Ok(())
    }
}
