//! # NeuroQuantumDB Core
//!
//! Neuromorphic computing core implementing synaptic networks, Hebbian learning,
//! and adaptive plasticity for ultra-efficient edge database operations.

pub mod synaptic;
pub mod learning;
pub mod plasticity;
pub mod query;
pub mod error;

pub use synaptic::{SynapticNode, SynapticNetwork, ConnectionType};
pub use learning::{HebbianLearningEngine, LearningStats};
pub use plasticity::{PlasticityMatrix, PlasticityParams};
pub use query::{NeuromorphicQueryProcessor, QueryResult};
pub use error::{CoreError, CoreResult};

use std::time::Instant;
use tracing::{info, debug, error};

/// Neuromorphic core configuration
#[derive(Debug, Clone)]
pub struct CoreConfig {
    /// Maximum number of synaptic nodes
    pub max_nodes: usize,
    /// Learning rate for Hebbian learning (0.0 - 1.0)
    pub learning_rate: f32,
    /// Threshold for synaptic activation
    pub activation_threshold: f32,
    /// Memory limit in bytes
    pub memory_limit: usize,
    /// Enable ARM64/NEON optimizations
    pub neon_optimizations: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            max_nodes: 1_000_000,
            learning_rate: 0.01,
            activation_threshold: 0.5,
            memory_limit: 100 * 1024 * 1024, // 100MB
            neon_optimizations: cfg!(target_arch = "aarch64"),
        }
    }
}

/// Main neuromorphic core implementation
pub struct NeuromorphicCore {
    config: CoreConfig,
    network: SynapticNetwork,
    learning_engine: HebbianLearningEngine,
    plasticity_matrix: PlasticityMatrix,
    query_processor: NeuromorphicQueryProcessor,
    stats: CoreStats,
}

#[derive(Debug, Default)]
pub struct CoreStats {
    pub nodes_created: u64,
    pub connections_formed: u64,
    pub learning_events: u64,
    pub queries_processed: u64,
    pub avg_response_time_ns: u64,
    pub memory_usage_bytes: usize,
}

impl NeuromorphicCore {
    /// Create a new neuromorphic core with default configuration
    pub fn new() -> CoreResult<Self> {
        Self::with_config(CoreConfig::default())
    }

    /// Create a new neuromorphic core with custom configuration
    pub fn with_config(config: CoreConfig) -> CoreResult<Self> {
        info!("Initializing NeuroQuantumDB Neuromorphic Core");
        debug!("Config: {:?}", config);

        let network = SynapticNetwork::new(config.max_nodes)?;
        let learning_engine = HebbianLearningEngine::new(config.learning_rate);
        let plasticity_matrix = PlasticityMatrix::new(config.max_nodes)?;
        let query_processor = NeuromorphicQueryProcessor::new()?;

        Ok(Self {
            config,
            network,
            learning_engine,
            plasticity_matrix,
            query_processor,
            stats: CoreStats::default(),
        })
    }

    /// Get current statistics
    pub fn stats(&self) -> &CoreStats {
        &self.stats
    }

    /// Perform health check
    pub async fn health_check(&self) -> CoreResult<()> {
        debug!("Performing neuromorphic core health check");

        // Check memory usage
        if self.stats.memory_usage_bytes > self.config.memory_limit {
            error!("Memory usage {} exceeds limit {}",
                   self.stats.memory_usage_bytes, self.config.memory_limit);
            return Err(CoreError::MemoryLimitExceeded);
        }

        // Check network integrity
        self.network.validate()?;

        info!("Neuromorphic core health check passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_core_initialization() {
        let core = NeuromorphicCore::new().unwrap();
        assert_eq!(core.stats().nodes_created, 0);

        // Health check should pass for new core
        core.health_check().await.unwrap();
    }

    #[test]
    fn test_config_validation() {
        let config = CoreConfig {
            max_nodes: 1000,
            learning_rate: 0.05,
            activation_threshold: 0.7,
            memory_limit: 50 * 1024 * 1024,
            neon_optimizations: true,
        };

        let core = NeuromorphicCore::with_config(config).unwrap();
        assert_eq!(core.config.max_nodes, 1000);
        assert_eq!(core.config.learning_rate, 0.05);
    }
}

#[cfg(all(test, feature = "neon-optimizations", target_arch = "aarch64"))]
mod neon_tests {
    use super::*;

    #[test]
    fn test_neon_optimizations_enabled() {
        let core = NeuromorphicCore::new().unwrap();
        assert!(core.config.neon_optimizations);
    }
}
