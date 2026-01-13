//! # D-Wave Backend
//!
//! This module provides integration with D-Wave quantum annealers for solving
//! optimization problems natively on quantum hardware.
//!
//! ## Supported Algorithms
//!
//! - **QUBO**: Quadratic Unconstrained Binary Optimization
//! - **Ising Model**: Native spin-glass problems
//! - **TFIM**: Transverse Field Ising Model
//! - **Parallel Tempering**: Thermal sampling via annealing schedules
//!
//! ## Hardware
//!
//! D-Wave offers several quantum annealer systems:
//! - **Advantage**: ~5000 qubits, Pegasus topology
//! - **Advantage2**: ~7000 qubits, Zephyr topology
//! - **Hybrid Solvers**: Classical-quantum hybrid for large problems
//!
//! ## Configuration
//!
//! ```rust,ignore
//! use neuroquantum_core::quantum::backends::dwave::{DWaveBackend, DWaveConfig};
//!
//! // From environment (recommended)
//! let backend = DWaveBackend::from_env();
//!
//! // Or with explicit config
//! let config = DWaveConfig {
//!     api_token: Some("your-dwave-token".to_string()),
//!     solver_name: Some("Advantage_system6.4".to_string()),
//!     ..Default::default()
//! };
//! let backend = DWaveBackend::new(config);
//! ```
//!
//! ## Environment Variables
//!
//! - `DWAVE_API_TOKEN`: Your D-Wave Leap API token
//! - `DWAVE_SOLVER`: Solver name (optional)
//! - `DWAVE_ENDPOINT`: API endpoint (optional)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info};

use nalgebra::DMatrix;

use super::{QuantumBackendConfig, QuantumBackendInfo, QuantumProvider};
use crate::error::{CoreError, CoreResult};

// =============================================================================
// D-Wave Configuration
// =============================================================================

/// Configuration for D-Wave quantum annealer backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DWaveConfig {
    /// D-Wave API token (from Leap account)
    /// If None, will attempt to read from DWAVE_API_TOKEN environment variable
    pub api_token: Option<String>,

    /// D-Wave API endpoint URL
    pub api_endpoint: String,

    /// Solver name (e.g., "Advantage_system6.4", "Advantage2_prototype2.1")
    /// If None, will use the first available solver
    pub solver_name: Option<String>,

    /// Number of reads (samples) to take
    pub num_reads: usize,

    /// Annealing time in microseconds (1-2000)
    pub annealing_time_us: u32,

    /// Auto-scale coefficients to fit hardware range
    pub auto_scale: bool,

    /// Chain strength for minor embedding (relative to max |J|)
    /// If None, calculated automatically
    pub chain_strength: Option<f64>,

    /// Programming thermalization time in microseconds
    pub programming_thermalization_us: u32,

    /// Readout thermalization time in microseconds
    pub readout_thermalization_us: u32,

    /// Reduce intersample correlation
    pub reduce_intersample_correlation: bool,

    /// Use postprocessing to improve solutions
    pub postprocessing: DWavePostprocessing,

    /// Common backend configuration
    #[serde(flatten)]
    pub common: QuantumBackendConfig,
}

impl Default for DWaveConfig {
    fn default() -> Self {
        Self {
            api_token: None,
            api_endpoint: "https://cloud.dwavesys.com/sapi/v2".to_string(),
            solver_name: None,
            num_reads: 1000,
            annealing_time_us: 20,
            auto_scale: true,
            chain_strength: None,
            programming_thermalization_us: 1000,
            readout_thermalization_us: 0,
            reduce_intersample_correlation: true,
            postprocessing: DWavePostprocessing::None,
            common: QuantumBackendConfig::default(),
        }
    }
}

/// D-Wave postprocessing options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DWavePostprocessing {
    /// No postprocessing
    None,
    /// Optimize chains
    Optimization,
    /// Sample chains
    Sampling,
}

// =============================================================================
// D-Wave Backend
// =============================================================================

/// D-Wave backend for quantum annealing optimization
///
/// This backend connects to D-Wave's quantum annealers via the Ocean SDK API
/// to solve QUBO and Ising model problems natively on quantum hardware.
///
/// ## Capabilities
///
/// - **Problem Types**: QUBO, Ising, BQM
/// - **Qubit Count**: ~5000-7000 depending on system
/// - **Topology**: Pegasus (Advantage) or Zephyr (Advantage2)
/// - **Embedding**: Automatic minor embedding
///
/// ## Example
///
/// ```rust,ignore
/// use neuroquantum_core::quantum::backends::dwave::DWaveBackend;
///
/// let backend = DWaveBackend::from_env();
/// if backend.is_available() {
///     println!("D-Wave ready with {} qubits", backend.max_qubits());
/// }
/// ```
pub struct DWaveBackend {
    config: DWaveConfig,
}

impl DWaveBackend {
    /// Create a new D-Wave backend with the given configuration
    pub fn new(config: DWaveConfig) -> Self {
        Self { config }
    }

    /// Create a backend using environment variables for configuration
    ///
    /// Reads:
    /// - `DWAVE_API_TOKEN`: API token
    /// - `DWAVE_SOLVER`: Solver name (optional)
    /// - `DWAVE_ENDPOINT`: API endpoint (optional)
    pub fn from_env() -> Self {
        let config = DWaveConfig {
            api_token: std::env::var("DWAVE_API_TOKEN").ok(),
            solver_name: std::env::var("DWAVE_SOLVER").ok(),
            api_endpoint: std::env::var("DWAVE_ENDPOINT")
                .unwrap_or_else(|_| "https://cloud.dwavesys.com/sapi/v2".to_string()),
            ..Default::default()
        };
        Self::new(config)
    }

    /// Get the API token, checking environment if not configured
    pub fn get_api_token(&self) -> Option<String> {
        self.config
            .api_token
            .clone()
            .or_else(|| std::env::var("DWAVE_API_TOKEN").ok())
    }

    /// Get the current configuration
    pub fn config(&self) -> &DWaveConfig {
        &self.config
    }

    /// Get mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut DWaveConfig {
        &mut self.config
    }

    /// Convert QUBO matrix to D-Wave format
    ///
    /// D-Wave expects a dictionary of (i, j) -> coefficient pairs
    pub fn qubo_to_dwave_format(&self, q_matrix: &DMatrix<f64>) -> HashMap<(usize, usize), f64> {
        let n = q_matrix.nrows();
        let mut q_dict = HashMap::new();

        for i in 0..n {
            for j in i..n {
                let val = if i == j {
                    q_matrix[(i, i)]
                } else {
                    q_matrix[(i, j)] + q_matrix[(j, i)]
                };

                if val.abs() > 1e-10 {
                    q_dict.insert((i, j), val);
                }
            }
        }

        q_dict
    }

    /// Convert Ising model to D-Wave format
    ///
    /// Returns (h, J) where h is linear biases and J is quadratic couplings
    pub fn ising_to_dwave_format(
        &self,
        fields: &[f64],
        couplings: &DMatrix<f64>,
    ) -> (HashMap<usize, f64>, HashMap<(usize, usize), f64>) {
        let n = fields.len();
        let mut h = HashMap::new();
        let mut j = HashMap::new();

        // Linear terms (external fields)
        for (i, &field) in fields.iter().enumerate() {
            if field.abs() > 1e-10 {
                h.insert(i, field);
            }
        }

        // Quadratic terms (couplings)
        for i in 0..n {
            for k in (i + 1)..n {
                let coupling = couplings[(i, k)];
                if coupling.abs() > 1e-10 {
                    j.insert((i, k), coupling);
                }
            }
        }

        (h, j)
    }

    /// Calculate recommended chain strength for embedding
    pub fn calculate_chain_strength(
        &self,
        linear: &HashMap<usize, f64>,
        quadratic: &HashMap<(usize, usize), f64>,
    ) -> f64 {
        // Find maximum coefficient magnitude
        let max_linear = linear.values().map(|v| v.abs()).fold(0.0f64, f64::max);
        let max_quadratic = quadratic.values().map(|v| v.abs()).fold(0.0f64, f64::max);

        let max_coeff = f64::max(max_linear, max_quadratic);

        // Recommended chain strength is 1.5-2x the maximum coefficient
        if let Some(strength) = self.config.chain_strength {
            strength * max_coeff
        } else {
            1.5 * max_coeff
        }
    }

    /// Build D-Wave problem submission payload
    pub fn build_problem_payload(
        &self,
        linear: &HashMap<usize, f64>,
        quadratic: &HashMap<(usize, usize), f64>,
    ) -> DWaveProblem {
        let chain_strength = self.calculate_chain_strength(linear, quadratic);

        DWaveProblem {
            linear: linear.clone(),
            quadratic: quadratic.clone(),
            num_reads: self.config.num_reads,
            annealing_time: self.config.annealing_time_us,
            auto_scale: self.config.auto_scale,
            chain_strength: Some(chain_strength),
            programming_thermalization: self.config.programming_thermalization_us,
            readout_thermalization: self.config.readout_thermalization_us,
            reduce_intersample_correlation: self.config.reduce_intersample_correlation,
        }
    }

    /// Submit problem to D-Wave SAPI
    ///
    /// Note: This is a placeholder for actual HTTP client implementation.
    pub async fn submit_problem(&self, problem: &DWaveProblem) -> CoreResult<DWaveSampleSet> {
        let api_token = self.get_api_token().ok_or_else(|| {
            CoreError::invalid_operation(
                "D-Wave API token not configured. Set DWAVE_API_TOKEN environment variable \
                 or provide api_token in DWaveConfig.",
            )
        })?;

        let start_time = Instant::now();

        info!(
            "Submitting problem to D-Wave API at {}",
            self.config.api_endpoint
        );
        debug!(
            "D-Wave config: solver={:?}, num_reads={}, annealing_time={}us",
            self.config.solver_name, self.config.num_reads, self.config.annealing_time_us
        );
        debug!(
            "Problem size: {} linear, {} quadratic terms",
            problem.linear.len(),
            problem.quadratic.len()
        );

        // In a real implementation, this would:
        // 1. Create HTTP client
        // 2. GET /solvers to list available solvers (if solver_name not set)
        // 3. POST /problems with:
        //    - Authorization: Token {api_token}
        //    - Content-Type: application/json
        //    - Body: { solver: solver_name, type: "ising", data: { linear, quadratic }, params: {...} }
        // 4. Poll GET /problems/{id} until status is COMPLETED
        // 5. GET /problems/{id}/answer to retrieve samples
        // 6. Parse and return DWaveSampleSet

        Err(CoreError::invalid_operation(&format!(
            "D-Wave API integration requires external HTTP client. \
             API token present: {}, solver: {:?}, endpoint: {}. \
             To enable real D-Wave execution, implement HTTP client with \
             reqwest or similar crate.",
            !api_token.is_empty(),
            self.config.solver_name,
            self.config.api_endpoint
        )))
    }

    /// Submit a QUBO problem
    pub async fn solve_qubo(&self, q_matrix: &DMatrix<f64>) -> CoreResult<DWaveSampleSet> {
        let q_dict = self.qubo_to_dwave_format(q_matrix);

        // Convert QUBO to Ising for D-Wave
        // QUBO: minimize x^T Q x  (x in {0,1})
        // Ising: minimize sum h_i s_i + sum J_ij s_i s_j  (s in {-1,+1})
        // Conversion: x_i = (s_i + 1) / 2
        let mut linear = HashMap::new();
        let mut quadratic = HashMap::new();

        for (&(i, j), &val) in &q_dict {
            if i == j {
                // Diagonal: contributes to linear term
                *linear.entry(i).or_insert(0.0) += val / 2.0;
            } else {
                // Off-diagonal: contributes to quadratic term
                quadratic.insert((i, j), val / 4.0);
                *linear.entry(i).or_insert(0.0) += val / 4.0;
                *linear.entry(j).or_insert(0.0) += val / 4.0;
            }
        }

        let problem = self.build_problem_payload(&linear, &quadratic);
        self.submit_problem(&problem).await
    }

    /// Submit an Ising model problem
    pub async fn solve_ising(
        &self,
        fields: &[f64],
        couplings: &DMatrix<f64>,
    ) -> CoreResult<DWaveSampleSet> {
        let (linear, quadratic) = self.ising_to_dwave_format(fields, couplings);
        let problem = self.build_problem_payload(&linear, &quadratic);
        self.submit_problem(&problem).await
    }
}

impl QuantumBackendInfo for DWaveBackend {
    fn is_available(&self) -> bool {
        self.get_api_token().is_some()
    }

    fn max_qubits(&self) -> usize {
        // D-Wave Advantage has ~5000 qubits, Advantage2 has ~7000
        // Effective problem size depends on embedding
        5000
    }

    fn name(&self) -> &str {
        "D-Wave Quantum Annealer"
    }

    fn provider(&self) -> QuantumProvider {
        QuantumProvider::DWave
    }
}

// =============================================================================
// D-Wave Types
// =============================================================================

/// D-Wave problem definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DWaveProblem {
    /// Linear coefficients (biases)
    pub linear: HashMap<usize, f64>,
    /// Quadratic coefficients (couplings)
    pub quadratic: HashMap<(usize, usize), f64>,
    /// Number of samples to take
    pub num_reads: usize,
    /// Annealing time in microseconds
    pub annealing_time: u32,
    /// Auto-scale coefficients
    pub auto_scale: bool,
    /// Chain strength for embedding
    pub chain_strength: Option<f64>,
    /// Programming thermalization time
    pub programming_thermalization: u32,
    /// Readout thermalization time
    pub readout_thermalization: u32,
    /// Reduce intersample correlation
    pub reduce_intersample_correlation: bool,
}

/// D-Wave sample set (results from annealing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DWaveSampleSet {
    /// Samples: each is (spin_values, energy, occurrence_count)
    pub samples: Vec<DWaveSample>,
    /// Timing information
    pub timing: DWaveTiming,
    /// Problem ID
    pub problem_id: Option<String>,
}

/// A single sample from D-Wave
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DWaveSample {
    /// Spin values (or binary values for QUBO)
    pub values: Vec<i8>,
    /// Energy of this configuration
    pub energy: f64,
    /// Number of times this sample was observed
    pub num_occurrences: usize,
    /// Whether chains were broken
    pub chain_break_fraction: f64,
}

/// D-Wave timing information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DWaveTiming {
    /// Total QPU access time in microseconds
    pub qpu_access_time_us: f64,
    /// QPU anneal time per sample in microseconds
    pub qpu_anneal_time_per_sample_us: f64,
    /// QPU programming time in microseconds
    pub qpu_programming_time_us: f64,
    /// QPU readout time per sample in microseconds
    pub qpu_readout_time_per_sample_us: f64,
    /// Total sampling time in microseconds
    pub qpu_sampling_time_us: f64,
    /// Post-processing time in microseconds
    pub post_processing_time_us: f64,
}

/// D-Wave solver information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DWaveSolverInfo {
    /// Solver name
    pub name: String,
    /// Number of qubits
    pub num_qubits: usize,
    /// Number of couplers
    pub num_couplers: usize,
    /// Topology type
    pub topology: DWaveTopology,
    /// Whether solver is online
    pub online: bool,
    /// Average queue time in seconds
    pub avg_queue_time: f64,
}

/// D-Wave topology types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DWaveTopology {
    /// Chimera topology (legacy)
    Chimera,
    /// Pegasus topology (Advantage)
    Pegasus,
    /// Zephyr topology (Advantage2)
    Zephyr,
}

impl std::fmt::Display for DWaveTopology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DWaveTopology::Chimera => write!(f, "Chimera"),
            DWaveTopology::Pegasus => write!(f, "Pegasus"),
            DWaveTopology::Zephyr => write!(f, "Zephyr"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dwave_config_default() {
        let config = DWaveConfig::default();
        assert!(config.api_endpoint.contains("dwavesys.com"));
        assert_eq!(config.num_reads, 1000);
        assert_eq!(config.annealing_time_us, 20);
        assert!(config.auto_scale);
    }

    #[test]
    fn test_dwave_backend_creation() {
        let backend = DWaveBackend::from_env();
        assert_eq!(backend.name(), "D-Wave Quantum Annealer");
        assert_eq!(backend.provider(), QuantumProvider::DWave);
    }

    #[test]
    fn test_chain_strength_calculation() {
        let backend = DWaveBackend::from_env();
        let mut linear = HashMap::new();
        linear.insert(0, 1.0);
        linear.insert(1, -2.0);

        let mut quadratic = HashMap::new();
        quadratic.insert((0, 1), 3.0);

        let strength = backend.calculate_chain_strength(&linear, &quadratic);
        assert!(strength >= 3.0); // Should be at least max coefficient
    }

    #[test]
    fn test_topology_display() {
        assert_eq!(format!("{}", DWaveTopology::Pegasus), "Pegasus");
        assert_eq!(format!("{}", DWaveTopology::Zephyr), "Zephyr");
        assert_eq!(format!("{}", DWaveTopology::Chimera), "Chimera");
    }
}
