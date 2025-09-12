//! # NeuroQuantumDB Core
//!
//! Neuromorphic computing core implementing synaptic networks, Hebbian learning,
//! quantum-inspired algorithms, and adaptive plasticity for ultra-efficient edge database operations.

pub mod synaptic;
pub mod learning;
pub mod plasticity;
pub mod query;
pub mod error;
pub mod neon_optimization;
pub mod quantum;

pub use synaptic::{SynapticNode, SynapticNetwork, ConnectionType};
pub use learning::{HebbianLearningEngine, LearningStats, AntiHebbianLearning};
pub use plasticity::{PlasticityMatrix, PlasticityParams, AccessPatterns};
pub use query::{NeuromorphicQueryProcessor, QueryResult, Query};
pub use error::{CoreError, CoreResult};
pub use quantum::{
    QuantumSearch, GroverSearch, QuantumConfig, QuantumError,
    QuantumSearchResult, OptimizedIndex, QuantumQueryResults,
    QuantumProcessorFactory
};

use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{info, debug, instrument, warn};
use serde::{Deserialize, Serialize};
use dashmap::DashMap;

/// Neuromorphic core configuration with enterprise-grade parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Power management enabled
    pub power_management: bool,
    /// Query timeout in microseconds
    pub query_timeout_us: u64,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Enable neuromorphic learning
    pub learning_enabled: bool,
    /// Plasticity reorganization threshold
    pub plasticity_threshold: f32,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            max_nodes: 1_000_000,
            learning_rate: 0.01,
            activation_threshold: 0.5,
            memory_limit: 100 * 1024 * 1024, // 100MB
            neon_optimizations: true,
            power_management: true,
            query_timeout_us: 1, // <1μs target
            max_connections: 500_000,
            learning_enabled: true,
            plasticity_threshold: 0.1,
        }
    }
}

/// Performance metrics for monitoring and optimization
#[derive(Debug, Clone, Default, Serialize)]
pub struct PerformanceMetrics {
    pub total_queries: u64,
    pub avg_response_time_ns: u64,
    pub memory_usage_bytes: usize,
    pub power_consumption_mw: u32,
    pub connection_count: u32,
    pub learning_iterations: u64,
    pub plasticity_reorganizations: u64,
    pub cache_hit_rate: f32,
    pub synaptic_strength_avg: f32,
}

/// Main neuromorphic core implementing the intelligent database engine
pub struct NeuroQuantumCore {
    config: CoreConfig,
    synaptic_network: Arc<RwLock<SynapticNetwork>>,
    learning_engine: Arc<RwLock<HebbianLearningEngine>>,
    plasticity_matrix: Arc<RwLock<PlasticityMatrix>>,
    query_processor: Arc<RwLock<NeuromorphicQueryProcessor>>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    active_connections: Arc<DashMap<u64, Instant>>,
    #[allow(dead_code)] // Used for uptime tracking in future features
    start_time: Instant,
}

impl NeuroQuantumCore {
    /// Create a new neuromorphic core with the given configuration
    #[instrument(level = "info")]
    pub fn new(config: CoreConfig) -> CoreResult<Self> {
        info!("Initializing NeuroQuantumDB neuromorphic core");

        // Validate configuration
        Self::validate_config(&config)?;

        let synaptic_network = Arc::new(RwLock::new(
            SynapticNetwork::new(config.max_nodes, config.activation_threshold)?
        ));

        let learning_engine = Arc::new(RwLock::new(
            HebbianLearningEngine::new(config.learning_rate)?
        ));

        let plasticity_matrix = Arc::new(RwLock::new(
            PlasticityMatrix::new(config.max_nodes, config.plasticity_threshold)?
        ));

        let query_processor = Arc::new(RwLock::new(
            NeuromorphicQueryProcessor::new(
                Arc::clone(&synaptic_network),
                Arc::clone(&learning_engine),
                config.neon_optimizations,
            )?
        ));

        let core = Self {
            config,
            synaptic_network,
            learning_engine,
            plasticity_matrix,
            query_processor,
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            active_connections: Arc::new(DashMap::new()),
            start_time: Instant::now(),
        };

        info!("Neuromorphic core initialized successfully");
        Ok(core)
    }

    /// Validate configuration parameters
    fn validate_config(config: &CoreConfig) -> CoreResult<()> {
        if config.max_nodes == 0 {
            return Err(CoreError::InvalidConfig("max_nodes must be > 0".to_string()));
        }
        if !(0.0..=1.0).contains(&config.learning_rate) {
            return Err(CoreError::InvalidConfig("learning_rate must be between 0.0 and 1.0".to_string()));
        }
        if !(0.0..=1.0).contains(&config.activation_threshold) {
            return Err(CoreError::InvalidConfig("activation_threshold must be between 0.0 and 1.0".to_string()));
        }
        if config.memory_limit < 1024 * 1024 { // Minimum 1MB
            return Err(CoreError::InvalidConfig("memory_limit must be at least 1MB".to_string()));
        }
        Ok(())
    }

    /// Create a new synaptic node
    #[instrument(level = "debug", skip(self))]
    pub fn create_node(&self, id: u64) -> CoreResult<()> {
        let mut network = self.synaptic_network.write()
            .map_err(|_| CoreError::LockError("Failed to acquire network write lock".to_string()))?;

        let node = SynapticNode::new(id);
        network.add_node(node)?;

        // Update metrics
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.memory_usage_bytes = network.memory_usage();
        }

        debug!("Created synaptic node with ID: {}", id);
        Ok(())
    }

    /// Connect two synaptic nodes with specified weight
    #[instrument(level = "debug", skip(self))]
    pub fn connect_nodes(&self, source: u64, target: u64, weight: f32) -> CoreResult<()> {
        let mut network = self.synaptic_network.write()
            .map_err(|_| CoreError::LockError("Failed to acquire network write lock".to_string()))?;

        network.connect_nodes(source, target, weight, ConnectionType::Excitatory)?;

        debug!("Connected nodes {} -> {} with weight {}", source, target, weight);
        Ok(())
    }

    /// Strengthen connection between nodes using Hebbian learning
    #[instrument(level = "debug", skip(self))]
    pub fn strengthen_connection(&self, source: u64, target: u64, amount: f32) -> CoreResult<()> {
        let mut learning_engine = self.learning_engine.write()
            .map_err(|_| CoreError::LockError("Failed to acquire learning engine write lock".to_string()))?;

        let mut network = self.synaptic_network.write()
            .map_err(|_| CoreError::LockError("Failed to acquire network write lock".to_string()))?;

        learning_engine.strengthen_connection(&mut network, source, target, amount)?;

        // Update learning metrics
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.learning_iterations += 1;
            metrics.synaptic_strength_avg = network.average_connection_strength();
        }

        debug!("Strengthened connection {} -> {} by {}", source, target, amount);
        Ok(())
    }

    /// Process a query using neuromorphic intelligence
    #[instrument(level = "debug", skip(self, query))]
    pub fn process_query(&self, query: &Query) -> CoreResult<QueryResult> {
        let start_time = Instant::now();

        // Check connection limit
        if self.active_connections.len() >= self.config.max_connections as usize {
            return Err(CoreError::ResourceExhausted("Maximum connections exceeded".to_string()));
        }

        // Register connection
        let connection_id = rand::random::<u64>();
        self.active_connections.insert(connection_id, start_time);

        // Process query
        let result = {
            let processor = self.query_processor.read()
                .map_err(|_| CoreError::LockError("Failed to acquire query processor read lock".to_string()))?;

            processor.process_query(query)?
        };

        // Cleanup connection
        self.active_connections.remove(&connection_id);

        let elapsed = start_time.elapsed();

        // Update performance metrics
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.total_queries += 1;
            metrics.avg_response_time_ns = (metrics.avg_response_time_ns + elapsed.as_nanos() as u64) / 2;
            metrics.connection_count = self.active_connections.len() as u32;
        }

        // Check if query exceeded timeout
        if elapsed.as_micros() > self.config.query_timeout_us as u128 {
            warn!("Query exceeded timeout: {}μs > {}μs", elapsed.as_micros(), self.config.query_timeout_us);
        }

        debug!("Processed query in {}μs", elapsed.as_micros());
        Ok(result)
    }

    /// Optimize the synaptic network using plasticity algorithms
    #[instrument(level = "info", skip(self))]
    pub fn optimize_network(&self) -> CoreResult<()> {
        info!("Starting network optimization");

        let mut plasticity = self.plasticity_matrix.write()
            .map_err(|_| CoreError::LockError("Failed to acquire plasticity matrix write lock".to_string()))?;

        let mut network = self.synaptic_network.write()
            .map_err(|_| CoreError::LockError("Failed to acquire network write lock".to_string()))?;

        let reorganized = plasticity.reorganize_network(&mut network)?;

        if reorganized {
            if let Ok(mut metrics) = self.metrics.write() {
                metrics.plasticity_reorganizations += 1;
            }
            info!("Network successfully reorganized for optimal performance");
        }

        Ok(())
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> CoreResult<PerformanceMetrics> {
        let metrics = self.metrics.read()
            .map_err(|_| CoreError::LockError("Failed to acquire metrics read lock".to_string()))?;

        Ok(metrics.clone())
    }

    /// Get configuration
    pub fn get_config(&self) -> &CoreConfig {
        &self.config
    }

    /// Shutdown the neuromorphic core gracefully
    #[instrument(level = "info", skip(self))]
    pub fn shutdown(&self) -> CoreResult<()> {
        info!("Shutting down neuromorphic core");

        // Wait for active connections to complete (with timeout)
        let shutdown_start = Instant::now();
        while !self.active_connections.is_empty() && shutdown_start.elapsed().as_secs() < 5 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        if !self.active_connections.is_empty() {
            warn!("Forced shutdown with {} active connections", self.active_connections.len());
        }

        info!("Neuromorphic core shutdown completed");
        Ok(())
    }
}

/// Trait defining the neuromorphic core interface for integration
pub trait NeuromorphicCoreInterface {
    fn create_node(&mut self, id: u64) -> CoreResult<()>;
    fn connect_nodes(&mut self, source: u64, target: u64, weight: f32) -> CoreResult<()>;
    fn strengthen_connection(&mut self, source: u64, target: u64, amount: f32) -> CoreResult<()>;
    fn process_query(&self, query: &Query) -> CoreResult<QueryResult>;
    fn optimize_network(&mut self) -> CoreResult<()>;
}

impl Drop for NeuroQuantumCore {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_creation() {
        let config = CoreConfig::default();
        let core = NeuroQuantumCore::new(config).expect("Failed to create core");
        assert_eq!(core.get_config().max_nodes, 1_000_000);
    }

    #[test]
    fn test_config_validation() {
        let mut config = CoreConfig::default();
        config.learning_rate = 2.0; // Invalid

        let result = NeuroQuantumCore::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_node_creation() {
        let config = CoreConfig::default();
        let core = NeuroQuantumCore::new(config).expect("Failed to create core");

        core.create_node(1).expect("Failed to create node");
        core.create_node(2).expect("Failed to create node");
    }

    #[test]
    fn test_node_connection() {
        let config = CoreConfig::default();
        let core = NeuroQuantumCore::new(config).expect("Failed to create core");

        core.create_node(1).expect("Failed to create node");
        core.create_node(2).expect("Failed to create node");
        core.connect_nodes(1, 2, 0.5).expect("Failed to connect nodes");
    }

    #[test]
    fn test_performance_metrics() {
        let config = CoreConfig::default();
        let core = NeuroQuantumCore::new(config).expect("Failed to create core");

        let metrics = core.get_metrics().expect("Failed to get metrics");
        assert_eq!(metrics.total_queries, 0);
    }
}
