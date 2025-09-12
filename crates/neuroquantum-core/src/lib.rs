//! # NeuroQuantumDB Core
//!
//! Neuromorphic computing core implementing synaptic networks, Hebbian learning,
//! and adaptive plasticity for ultra-efficient edge database operations.

pub mod synaptic;
pub mod learning;
pub mod plasticity;
pub mod query;
pub mod error;
pub mod neon_optimization;

pub use synaptic::{SynapticNode, SynapticNetwork, ConnectionType};
pub use learning::{HebbianLearningEngine, LearningStats, AntiHebbianLearning};
pub use plasticity::{PlasticityMatrix, PlasticityParams, AccessPatterns};
pub use query::{NeuromorphicQueryProcessor, QueryResult, Query};
pub use error::{CoreError, CoreResult};

use std::time::Instant;
use tracing::{info, debug, error, instrument};
use serde::{Deserialize, Serialize};

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
            neon_optimizations: cfg!(target_arch = "aarch64"),
            power_management: true,
            query_timeout_us: 1, // <1μs target
            max_connections: 500_000,
            learning_enabled: true,
            plasticity_threshold: 0.6,
        }
    }
}

/// Main neuromorphic core implementation with enterprise features
pub struct NeuromorphicCore {
    config: CoreConfig,
    network: SynapticNetwork,
    learning_engine: HebbianLearningEngine,
    anti_hebbian: AntiHebbianLearning,
    plasticity_matrix: PlasticityMatrix,
    query_processor: NeuromorphicQueryProcessor,
    stats: CoreStats,
    access_patterns: AccessPatterns,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CoreStats {
    pub nodes_created: u64,
    pub connections_formed: u64,
    pub learning_events: u64,
    pub queries_processed: u64,
    pub avg_response_time_ns: u64,
    pub memory_usage_bytes: usize,
    pub power_consumption_mw: f32,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub optimization_cycles: u64,
    pub error_count: u64,
}

impl NeuromorphicCore {
    /// Create a new neuromorphic core with default configuration
    #[instrument(name = "neuromorphic_core_new")]
    pub fn new() -> CoreResult<Self> {
        Self::with_config(CoreConfig::default())
    }

    /// Create a new neuromorphic core with custom configuration
    #[instrument(name = "neuromorphic_core_with_config", skip(config))]
    pub fn with_config(config: CoreConfig) -> CoreResult<Self> {
        info!("Initializing NeuroQuantumDB Neuromorphic Core v0.1.0");
        debug!("Config: {:?}", config);

        // Validate configuration
        if config.learning_rate < 0.0 || config.learning_rate > 1.0 {
            return Err(CoreError::config_error("Learning rate must be between 0.0 and 1.0"));
        }

        if config.max_nodes == 0 {
            return Err(CoreError::config_error("Max nodes must be greater than 0"));
        }

        let network = SynapticNetwork::new(config.max_nodes)?;
        let learning_engine = HebbianLearningEngine::new(config.learning_rate);
        let anti_hebbian = AntiHebbianLearning::new(0.2); // 20% inhibition rate
        let plasticity_matrix = PlasticityMatrix::new(config.max_nodes)?;
        let query_processor = NeuromorphicQueryProcessor::new()?;
        let access_patterns = AccessPatterns::new();

        info!("Neuromorphic core initialized successfully");

        Ok(Self {
            config,
            network,
            learning_engine,
            anti_hebbian,
            plasticity_matrix,
            query_processor,
            stats: CoreStats::default(),
            access_patterns,
        })
    }

    /// Create a new synaptic node in the network
    #[instrument(name = "create_node", skip(self))]
    pub async fn create_node(&mut self, data: Option<&[u8]>) -> CoreResult<u64> {
        let node_id = self.network.create_node()?;

        if let Some(data) = data {
            let node_ref = self.network.get_node(node_id)?;
            let mut node = node_ref.write();
            node.set_data(data.to_vec(), "binary".to_string());
        }

        self.stats.nodes_created += 1;
        self.update_power_consumption().await?;

        info!("Created new synaptic node: {}", node_id);
        Ok(node_id)
    }

    /// Connect two nodes with a synaptic connection
    #[instrument(name = "connect_nodes", skip(self))]
    pub async fn connect_nodes(
        &mut self,
        source_id: u64,
        target_id: u64,
        weight: f32,
        connection_type: ConnectionType,
    ) -> CoreResult<()> {
        self.network.connect_nodes(source_id, target_id, weight, connection_type)?;
        self.stats.connections_formed += 1;

        // Apply learning if enabled
        if self.config.learning_enabled {
            self.learning_engine.strengthen_pathway(&self.network, source_id, target_id, weight.abs())?;
            self.stats.learning_events += 1;
        }

        debug!("Connected nodes {} -> {} with weight {}", source_id, target_id, weight);
        Ok(())
    }

    /// Strengthen a synaptic connection using Hebbian learning
    #[instrument(name = "strengthen_connection", skip(self))]
    pub async fn strengthen_connection(
        &mut self,
        source_id: u64,
        target_id: u64,
        amount: f32,
    ) -> CoreResult<()> {
        self.learning_engine.strengthen_pathway(&self.network, source_id, target_id, amount)?;
        self.stats.learning_events += 1;

        debug!("Strengthened connection {} -> {} by {}", source_id, target_id, amount);
        Ok(())
    }

    /// Process a query using neuromorphic optimization
    #[instrument(name = "process_query", skip(self, query))]
    pub async fn process_query(&mut self, query: &Query) -> CoreResult<QueryResult> {
        let start_time = Instant::now();

        // Record access pattern
        for &node_id in &query.target_nodes {
            self.access_patterns.record_access(node_id);
        }

        // Process query with neuromorphic optimization
        let result = self.query_processor.process_query(
            &query.sql,
            &self.network,
            &self.learning_engine,
            &self.plasticity_matrix,
        ).await?;

        // Update statistics
        let execution_time = start_time.elapsed().as_nanos() as u64;
        self.stats.queries_processed += 1;
        self.stats.avg_response_time_ns =
            (self.stats.avg_response_time_ns + execution_time) / 2;

        // Check performance target
        if execution_time > self.config.query_timeout_us * 1000 {
            error!("Query exceeded timeout: {}μs > {}μs",
                   execution_time / 1000, self.config.query_timeout_us);
        }

        debug!("Query processed in {}ns", execution_time);
        Ok(result)
    }

    /// Optimize the entire network using plasticity and learning
    #[instrument(name = "optimize_network", skip(self))]
    pub async fn optimize_network(&mut self) -> CoreResult<()> {
        info!("Starting network optimization cycle");

        // Apply competitive inhibition
        let activations: Vec<_> = (0..self.stats.nodes_created)
            .map(|id| {
                let node_ref = self.network.get_node(id).ok()?;
                let node = node_ref.read();
                Some((id, node.activation))
            })
            .filter_map(|x| x)
            .collect();

        self.anti_hebbian.apply_competition(&self.network, &activations)?;

        // Reorganize data based on access patterns
        let reorganization_result = self.plasticity_matrix
            .reorganize_data(&self.network, &self.access_patterns)?;

        info!("Network optimization completed: {} nodes moved, {:.2}% performance improvement",
              reorganization_result.nodes_moved,
              (reorganization_result.performance_improvement - 1.0) * 100.0);

        self.stats.optimization_cycles += 1;
        Ok(())
    }

    /// Perform comprehensive health check
    #[instrument(name = "health_check", skip(self))]
    pub async fn health_check(&self) -> CoreResult<HealthStatus> {
        debug!("Performing neuromorphic core health check");

        let mut status = HealthStatus::default();

        // Check memory usage
        if self.stats.memory_usage_bytes > self.config.memory_limit {
            error!("Memory usage {} exceeds limit {}",
                   self.stats.memory_usage_bytes, self.config.memory_limit);
            status.memory_status = HealthLevel::Critical;
        } else if self.stats.memory_usage_bytes > self.config.memory_limit * 8 / 10 {
            status.memory_status = HealthLevel::Warning;
        }

        // Check power consumption (target <2W = 2000mW)
        if self.stats.power_consumption_mw > 2000.0 {
            error!("Power consumption {:.2}mW exceeds 2W limit", self.stats.power_consumption_mw);
            status.power_status = HealthLevel::Critical;
        } else if self.stats.power_consumption_mw > 1600.0 {
            status.power_status = HealthLevel::Warning;
        }

        // Check query performance
        if self.stats.avg_response_time_ns > self.config.query_timeout_us * 1000 {
            error!("Average query time {}μs exceeds {}μs target",
                   self.stats.avg_response_time_ns / 1000, self.config.query_timeout_us);
            status.performance_status = HealthLevel::Critical;
        }

        // Check network integrity
        self.network.validate()?;
        status.network_status = HealthLevel::Healthy;

        info!("Health check completed: {:?}", status);
        Ok(status)
    }

    /// Update power consumption estimate
    async fn update_power_consumption(&mut self) -> CoreResult<()> {
        // Estimate power consumption based on activity
        let base_power = 500.0; // 500mW base consumption
        let node_power = self.stats.nodes_created as f32 * 0.001; // 1μW per node
        let query_power = self.stats.queries_processed as f32 * 0.01; // 10μW per query

        self.stats.power_consumption_mw = base_power + node_power + query_power;
        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> &CoreStats {
        &self.stats
    }

    /// Get configuration
    pub fn config(&self) -> &CoreConfig {
        &self.config
    }

    /// Get network reference
    pub fn network(&self) -> &SynapticNetwork {
        &self.network
    }
}

/// Health status for different system components
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HealthStatus {
    pub memory_status: HealthLevel,
    pub power_status: HealthLevel,
    pub performance_status: HealthLevel,
    pub network_status: HealthLevel,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HealthLevel {
    Healthy,
    Warning,
    Critical,
}

impl Default for HealthLevel {
    fn default() -> Self {
        Self::Healthy
    }
}

/// Query structure for neuromorphic processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub sql: String,
    pub target_nodes: Vec<u64>,
    pub priority: QueryPriority,
    pub optimization_hints: Vec<OptimizationHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationHint {
    UseCache,
    SkipLearning,
    ForceReorganization,
    PreferSpeed,
    PreferAccuracy,
}

// Implement the NeuromorphicCore trait for external interfaces
#[async_trait::async_trait]
pub trait NeuromorphicCoreInterface: Send + Sync {
    async fn create_node(&mut self, data: Option<&[u8]>) -> CoreResult<u64>;
    async fn connect_nodes(&mut self, source: u64, target: u64, weight: f32) -> CoreResult<()>;
    async fn strengthen_connection(&mut self, source: u64, target: u64, amount: f32) -> CoreResult<()>;
    async fn process_query(&mut self, query: &Query) -> CoreResult<QueryResult>;
    async fn optimize_network(&mut self) -> CoreResult<()>;
    async fn health_check(&self) -> CoreResult<HealthStatus>;
}

#[async_trait::async_trait]
impl NeuromorphicCoreInterface for NeuromorphicCore {
    async fn create_node(&mut self, data: Option<&[u8]>) -> CoreResult<u64> {
        self.create_node(data).await
    }

    async fn connect_nodes(&mut self, source: u64, target: u64, weight: f32) -> CoreResult<()> {
        self.connect_nodes(source, target, weight, ConnectionType::Excitatory).await
    }

    async fn strengthen_connection(&mut self, source: u64, target: u64, amount: f32) -> CoreResult<()> {
        self.strengthen_connection(source, target, amount).await
    }

    async fn process_query(&mut self, query: &Query) -> CoreResult<QueryResult> {
        self.process_query(query).await
    }

    async fn optimize_network(&mut self) -> CoreResult<()> {
        self.optimize_network().await
    }

    async fn health_check(&self) -> CoreResult<HealthStatus> {
        self.health_check().await
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
        let health = core.health_check().await.unwrap();
        assert!(matches!(health.memory_status, HealthLevel::Healthy));
    }

    #[tokio::test]
    async fn test_node_creation_and_connection() {
        let mut core = NeuromorphicCore::new().unwrap();

        let node1 = core.create_node(Some(b"test_data_1")).await.unwrap();
        let node2 = core.create_node(Some(b"test_data_2")).await.unwrap();

        core.connect_nodes(node1, node2, 0.8, ConnectionType::Excitatory).await.unwrap();

        assert_eq!(core.stats().nodes_created, 2);
        assert_eq!(core.stats().connections_formed, 1);
    }

    #[tokio::test]
    async fn test_query_processing() {
        let mut core = NeuromorphicCore::new().unwrap();

        let node1 = core.create_node(Some(b"SELECT * FROM users")).await.unwrap();

        let query = Query {
            sql: "SELECT * FROM users WHERE id = 1".to_string(),
            target_nodes: vec![node1],
            priority: QueryPriority::Normal,
            optimization_hints: vec![OptimizationHint::PreferSpeed],
        };

        let result = core.process_query(&query).await.unwrap();
        assert!(result.execution_time_ns > 0);
        assert_eq!(core.stats().queries_processed, 1);
    }

    #[tokio::test]
    async fn test_network_optimization() {
        let mut core = NeuromorphicCore::new().unwrap();

        // Create some nodes and connections
        let node1 = core.create_node(None).await.unwrap();
        let node2 = core.create_node(None).await.unwrap();
        let node3 = core.create_node(None).await.unwrap();

        core.connect_nodes(node1, node2, 0.7, ConnectionType::Excitatory).await.unwrap();
        core.connect_nodes(node2, node3, 0.6, ConnectionType::Excitatory).await.unwrap();

        // Optimize network
        core.optimize_network().await.unwrap();

        assert_eq!(core.stats().optimization_cycles, 1);
    }

    #[tokio::test]
    async fn test_power_consumption_monitoring() {
        let mut core = NeuromorphicCore::new().unwrap();

        // Create nodes and check power consumption increases
        let initial_power = core.stats().power_consumption_mw;

        for _ in 0..100 {
            core.create_node(None).await.unwrap();
        }

        assert!(core.stats().power_consumption_mw > initial_power);
        assert!(core.stats().power_consumption_mw < 2000.0); // Under 2W limit
    }

    #[test]
    fn test_config_validation() {
        let mut config = CoreConfig::default();
        config.learning_rate = 1.5; // Invalid

        let result = NeuromorphicCore::with_config(config);
        assert!(result.is_err());
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

    #[tokio::test]
    async fn test_neon_simd_performance() {
        let mut core = NeuromorphicCore::new().unwrap();

        // Create a large number of nodes to test SIMD optimizations
        let start = Instant::now();
        for _ in 0..1000 {
            core.create_node(None).await.unwrap();
        }
        let duration = start.elapsed();

        // With NEON optimizations, this should be very fast
        assert!(duration.as_millis() < 100);
    }
}

#[cfg(feature = "benchmarks")]
mod benchmarks {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn benchmark_node_creation(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        c.bench_function("node_creation", |b| {
            b.iter(|| {
                rt.block_on(async {
                    let mut core = NeuromorphicCore::new().unwrap();
                    black_box(core.create_node(Some(b"test_data")).await.unwrap());
                });
            });
        });
    }

    fn benchmark_query_processing(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        c.bench_function("query_processing", |b| {
            b.iter(|| {
                rt.block_on(async {
                    let mut core = NeuromorphicCore::new().unwrap();
                    let node_id = core.create_node(None).await.unwrap();

                    let query = Query {
                        sql: "SELECT * FROM test".to_string(),
                        target_nodes: vec![node_id],
                        priority: QueryPriority::Normal,
                        optimization_hints: vec![],
                    };

                    black_box(core.process_query(&query).await.unwrap());
                });
            });
        });
    }

    criterion_group!(benches, benchmark_node_creation, benchmark_query_processing);
    criterion_main!(benches);
}
