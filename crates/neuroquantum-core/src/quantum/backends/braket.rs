//! # AWS Braket Backend
//!
//! This module provides integration with AWS Braket for accessing multiple
//! quantum computing vendors through a unified API.
//!
//! ## Supported Devices
//!
//! - **`IonQ`**: Trapped-ion quantum computers (Aria, Forte)
//! - **Rigetti**: Superconducting processors (Aspen-M)
//! - **OQC**: Oxford Quantum Circuits (Lucy)
//! - **D-Wave**: Quantum annealers via Braket (Advantage)
//! - **Simulators**: SV1, DM1, TN1
//!
//! ## Supported Algorithms
//!
//! - **Grover's Search**: On gate-based devices (`IonQ`, Rigetti, OQC)
//! - **QUBO/QAOA**: On gate-based devices and D-Wave annealers
//! - **TFIM**: On D-Wave annealers
//! - **Parallel Tempering**: On D-Wave and gate-based devices
//!
//! ## Configuration
//!
//! ```no_run
//! use neuroquantum_core::quantum::backends::braket::{BraketBackend, BraketConfig};
//!
//! // From environment (uses AWS credential chain)
//! let backend = BraketBackend::from_env();
//!
//! // Or with explicit config
//! let config = BraketConfig {
//!     region: "us-east-1".to_string(),
//!     device_arn: "arn:aws:braket:::device/qpu/ionq/Aria-1".to_string(),
//!     ..Default::default()
//! };
//! let backend = BraketBackend::new(config);
//! ```
//!
//! ## Environment Variables
//!
//! - `AWS_ACCESS_KEY_ID`: AWS access key
//! - `AWS_SECRET_ACCESS_KEY`: AWS secret key
//! - `AWS_SESSION_TOKEN`: Session token (optional)
//! - `AWS_REGION`: AWS region (default: us-east-1)
//! - `AWS_BRAKET_DEVICE_ARN`: Device ARN
//! - `AWS_BRAKET_S3_BUCKET`: S3 bucket for results

use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::{QuantumBackendConfig, QuantumBackendInfo, QuantumExecutionResult, QuantumProvider};
use crate::error::{CoreError, CoreResult};

// =============================================================================
// AWS Braket Configuration
// =============================================================================

/// Configuration for AWS Braket backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BraketConfig {
    /// AWS region
    pub region: String,

    /// Device ARN (Amazon Resource Name)
    /// Examples:
    /// - `IonQ` Aria: "`arn:aws:braket:us-east-1::device/qpu/ionq/Aria-1`"
    /// - Rigetti: "`arn:aws:braket:us-west-1::device/qpu/rigetti/Aspen-M-3`"
    /// - D-Wave: "arn:aws:braket:::device/qpu/d-wave/Advantage_system6"
    /// - Simulator: "`arn:aws:braket:::device/quantum-simulator/amazon/sv1`"
    pub device_arn: String,

    /// Number of shots for measurement
    pub num_shots: usize,

    /// S3 bucket for results storage
    pub s3_bucket: Option<String>,

    /// S3 prefix for results
    pub s3_prefix: String,

    /// Maximum qubits allowed (device-dependent)
    pub max_qubits: usize,

    /// Poll interval in seconds when waiting for results
    pub poll_interval_secs: u64,

    /// Maximum wait time in seconds
    pub max_wait_secs: u64,

    /// Common backend configuration
    #[serde(flatten)]
    pub common: QuantumBackendConfig,
}

impl Default for BraketConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            device_arn: "arn:aws:braket:::device/quantum-simulator/amazon/sv1".to_string(),
            num_shots: 1024,
            s3_bucket: None,
            s3_prefix: "braket-results".to_string(),
            max_qubits: 34, // SV1 simulator limit
            poll_interval_secs: 5,
            max_wait_secs: 600,
            common: QuantumBackendConfig::default(),
        }
    }
}

// =============================================================================
// AWS Braket Backend
// =============================================================================

/// AWS Braket backend for multi-vendor quantum access
///
/// This backend connects to AWS Braket to execute quantum circuits on various
/// quantum computing hardware from different vendors.
///
/// ## Vendor Capabilities
///
/// | Vendor | Technology | Max Qubits | Connectivity |
/// |--------|------------|------------|--------------|
/// | `IonQ` | Trapped-ion | 25 | All-to-all |
/// | Rigetti | Superconducting | 80 | Limited |
/// | OQC | Superconducting | 8 | Limited |
/// | D-Wave | Annealing | 5000+ | Pegasus/Zephyr |
///
/// ## Example
///
/// ```no_run
/// use neuroquantum_core::quantum::backends::braket::BraketBackend;
/// use neuroquantum_core::quantum::backends::QuantumBackendInfo;
///
/// let backend = BraketBackend::from_env();
/// if backend.is_available() {
///     println!("AWS Braket ready: {}", backend.name());
/// }
/// ```
pub struct BraketBackend {
    config: BraketConfig,
}

impl BraketBackend {
    /// Create a new AWS Braket backend with the given configuration
    #[must_use]
    pub const fn new(config: BraketConfig) -> Self {
        Self { config }
    }

    /// Create a backend using environment variables for configuration
    ///
    /// Uses AWS default credential chain:
    /// 1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
    /// 2. Shared credentials file (~/.aws/credentials)
    /// 3. IAM role for EC2/ECS/Lambda
    #[must_use]
    pub fn from_env() -> Self {
        let config = BraketConfig {
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            device_arn: std::env::var("AWS_BRAKET_DEVICE_ARN").unwrap_or_else(|_| {
                "arn:aws:braket:::device/quantum-simulator/amazon/sv1".to_string()
            }),
            s3_bucket: std::env::var("AWS_BRAKET_S3_BUCKET").ok(),
            max_qubits: std::env::var("AWS_BRAKET_MAX_QUBITS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(34),
            ..Default::default()
        };
        Self::new(config)
    }

    /// Check if AWS credentials are available
    #[must_use]
    pub fn has_credentials(&self) -> bool {
        // Check for AWS credentials via environment variables or default credential chain
        std::env::var("AWS_ACCESS_KEY_ID").is_ok()
            || std::env::var("AWS_SESSION_TOKEN").is_ok()
            || std::env::var("AWS_PROFILE").is_ok()
            || std::path::Path::new(
                &std::env::var("HOME")
                    .map(|h| format!("{h}/.aws/credentials"))
                    .unwrap_or_default(),
            )
            .exists()
    }

    /// Get the current configuration
    #[must_use]
    pub const fn config(&self) -> &BraketConfig {
        &self.config
    }

    /// Get mutable reference to configuration
    pub const fn config_mut(&mut self) -> &mut BraketConfig {
        &mut self.config
    }

    /// Get the device type from the ARN
    #[must_use]
    pub fn device_type(&self) -> BraketDeviceType {
        let arn = &self.config.device_arn;
        if arn.contains("ionq") {
            BraketDeviceType::IonQ
        } else if arn.contains("rigetti") {
            BraketDeviceType::Rigetti
        } else if arn.contains("oqc") {
            BraketDeviceType::OQC
        } else if arn.contains("d-wave") || arn.contains("dwave") {
            BraketDeviceType::DWave
        } else if arn.contains("simulator") {
            BraketDeviceType::Simulator
        } else {
            BraketDeviceType::Unknown
        }
    }

    /// Check if this is an annealing device
    #[must_use]
    pub fn is_annealer(&self) -> bool {
        matches!(self.device_type(), BraketDeviceType::DWave)
    }

    /// Check if this is a gate-based device
    #[must_use]
    pub fn is_gate_based(&self) -> bool {
        matches!(
            self.device_type(),
            BraketDeviceType::IonQ
                | BraketDeviceType::Rigetti
                | BraketDeviceType::OQC
                | BraketDeviceType::Simulator
        )
    }

    /// Build `OpenQASM` 3.0 circuit for Braket
    #[must_use]
    pub fn build_qasm_circuit(&self, num_qubits: usize, gates: &[BraketGate]) -> String {
        let mut qasm = String::new();
        qasm.push_str("OPENQASM 3.0;\n");
        qasm.push_str("include \"stdgates.inc\";\n\n");
        qasm.push_str(&format!("qubit[{num_qubits}] q;\n"));
        qasm.push_str(&format!("bit[{num_qubits}] c;\n\n"));

        for gate in gates {
            match gate {
                | BraketGate::H(q) => qasm.push_str(&format!("h q[{q}];\n")),
                | BraketGate::X(q) => qasm.push_str(&format!("x q[{q}];\n")),
                | BraketGate::Y(q) => qasm.push_str(&format!("y q[{q}];\n")),
                | BraketGate::Z(q) => qasm.push_str(&format!("z q[{q}];\n")),
                | BraketGate::Rx(q, angle) => qasm.push_str(&format!("rx({angle}) q[{q}];\n")),
                | BraketGate::Ry(q, angle) => qasm.push_str(&format!("ry({angle}) q[{q}];\n")),
                | BraketGate::Rz(q, angle) => qasm.push_str(&format!("rz({angle}) q[{q}];\n")),
                | BraketGate::CX(c, t) => qasm.push_str(&format!("cx q[{c}], q[{t}];\n")),
                | BraketGate::CZ(c, t) => qasm.push_str(&format!("cz q[{c}], q[{t}];\n")),
                | BraketGate::RZZ(q1, q2, angle) => {
                    qasm.push_str(&format!("rzz({angle}) q[{q1}], q[{q2}];\n"));
                },
                | BraketGate::Measure(q, c) => {
                    qasm.push_str(&format!("c[{c}] = measure q[{q}];\n"));
                },
            }
        }

        qasm
    }

    /// Build annealing problem for D-Wave via Braket
    #[must_use]
    pub fn build_annealing_problem(
        &self,
        linear: &std::collections::HashMap<usize, f64>,
        quadratic: &std::collections::HashMap<(usize, usize), f64>,
    ) -> BraketAnnealingProblem {
        BraketAnnealingProblem {
            linear: linear.clone(),
            quadratic: quadratic.clone(),
            problem_type: BraketProblemType::Ising,
        }
    }

    /// Submit a quantum task to AWS Braket
    ///
    /// Note: This is a placeholder for actual AWS SDK integration.
    pub async fn submit_task(&self, circuit: &str) -> CoreResult<QuantumExecutionResult> {
        if !self.has_credentials() {
            return Err(CoreError::invalid_operation(
                "AWS credentials not configured. Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY \
                 environment variables or configure AWS credentials file.",
            ));
        }

        let _start_time = Instant::now();

        info!(
            "Submitting task to AWS Braket in region {}",
            self.config.region
        );
        debug!(
            "Braket config: device={}, shots={}",
            self.config.device_arn, self.config.num_shots
        );
        debug!("Circuit length: {} chars", circuit.len());

        // In a real implementation, this would use aws-sdk-braket:
        // 1. Create BraketClient with region
        // 2. Build CreateQuantumTaskInput with:
        //    - device_arn: self.config.device_arn
        //    - shots: self.config.num_shots
        //    - output_s3_bucket/prefix
        //    - action: OpenQasmProgram { source: circuit }
        // 3. Call create_quantum_task
        // 4. Poll get_quantum_task until terminal state
        // 5. Retrieve and parse results from S3

        Err(CoreError::invalid_operation(&format!(
            "AWS Braket integration requires aws-sdk-braket crate. \
             Credentials available: {}, device: {}, region: {}",
            self.has_credentials(),
            self.config.device_arn,
            self.config.region
        )))
    }

    /// Submit an annealing problem to D-Wave via Braket
    pub async fn submit_annealing(
        &self,
        problem: &BraketAnnealingProblem,
        _num_reads: usize,
    ) -> CoreResult<QuantumExecutionResult> {
        if !self.has_credentials() {
            return Err(CoreError::invalid_operation(
                "AWS credentials not configured for Braket annealing.",
            ));
        }

        if !self.is_annealer() {
            return Err(CoreError::invalid_operation(&format!(
                "Device {} is not a quantum annealer. Use a D-Wave device ARN.",
                self.config.device_arn
            )));
        }

        info!(
            "Submitting annealing problem to AWS Braket D-Wave: {} vars",
            problem.linear.len()
        );

        // In a real implementation:
        // 1. Create AnnealingTaskAction with problem definition
        // 2. Submit via create_quantum_task
        // 3. Wait for completion and retrieve samples

        Err(CoreError::invalid_operation(
            "AWS Braket annealing integration requires aws-sdk-braket crate.",
        ))
    }
}

impl QuantumBackendInfo for BraketBackend {
    fn is_available(&self) -> bool {
        self.has_credentials()
    }

    fn max_qubits(&self) -> usize {
        self.config.max_qubits
    }

    fn name(&self) -> &'static str {
        "AWS Braket"
    }

    fn provider(&self) -> QuantumProvider {
        QuantumProvider::AWSBraket
    }
}

// =============================================================================
// AWS Braket Types
// =============================================================================

/// AWS Braket device types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BraketDeviceType {
    /// `IonQ` trapped-ion
    IonQ,
    /// Rigetti superconducting
    Rigetti,
    /// OQC superconducting
    OQC,
    /// D-Wave quantum annealer
    DWave,
    /// Amazon simulator (SV1, DM1, TN1)
    Simulator,
    /// Unknown device
    Unknown,
}

impl std::fmt::Display for BraketDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::IonQ => write!(f, "IonQ"),
            | Self::Rigetti => write!(f, "Rigetti"),
            | Self::OQC => write!(f, "OQC"),
            | Self::DWave => write!(f, "D-Wave"),
            | Self::Simulator => write!(f, "Simulator"),
            | Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Gate operations for Braket circuits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BraketGate {
    /// Hadamard gate
    H(usize),
    /// Pauli X gate
    X(usize),
    /// Pauli Y gate
    Y(usize),
    /// Pauli Z gate
    Z(usize),
    /// X rotation
    Rx(usize, f64),
    /// Y rotation
    Ry(usize, f64),
    /// Z rotation
    Rz(usize, f64),
    /// CNOT gate
    CX(usize, usize),
    /// CZ gate
    CZ(usize, usize),
    /// ZZ rotation
    RZZ(usize, usize, f64),
    /// Measurement
    Measure(usize, usize),
}

/// Annealing problem for D-Wave via Braket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BraketAnnealingProblem {
    /// Linear coefficients (biases)
    pub linear: std::collections::HashMap<usize, f64>,
    /// Quadratic coefficients (couplings)
    pub quadratic: std::collections::HashMap<(usize, usize), f64>,
    /// Problem type
    pub problem_type: BraketProblemType,
}

/// Problem type for annealing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BraketProblemType {
    /// Ising model: spins in {-1, +1}
    Ising,
    /// QUBO: binary variables in {0, 1}
    Qubo,
}

/// AWS Braket task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BraketTaskStatus {
    /// Task is queued
    Queued,
    /// Task is running
    Running,
    /// Task completed
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

/// AWS Braket task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BraketTaskInfo {
    /// Task ARN
    pub task_arn: String,
    /// Current status
    pub status: BraketTaskStatus,
    /// Device ARN
    pub device_arn: String,
    /// S3 output location
    pub s3_output: Option<String>,
    /// Creation time
    pub created: Option<String>,
    /// Completion time
    pub finished: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braket_config_default() {
        let config = BraketConfig::default();
        assert_eq!(config.region, "us-east-1");
        assert!(config.device_arn.contains("simulator"));
        assert_eq!(config.num_shots, 1024);
        assert_eq!(config.max_qubits, 34);
    }

    #[test]
    fn test_braket_backend_creation() {
        let backend = BraketBackend::from_env();
        assert_eq!(backend.name(), "AWS Braket");
        assert_eq!(backend.provider(), QuantumProvider::AWSBraket);
    }

    #[test]
    fn test_device_type_detection() {
        let config = BraketConfig {
            device_arn: "arn:aws:braket:us-east-1::device/qpu/ionq/Aria-1".to_string(),
            ..BraketConfig::default()
        };
        let backend = BraketBackend::new(config);
        assert_eq!(backend.device_type(), BraketDeviceType::IonQ);

        let config = BraketConfig {
            device_arn: "arn:aws:braket:::device/qpu/d-wave/Advantage_system6".to_string(),
            ..BraketConfig::default()
        };
        let backend = BraketBackend::new(config);
        assert_eq!(backend.device_type(), BraketDeviceType::DWave);
        assert!(backend.is_annealer());

        let config = BraketConfig {
            device_arn: "arn:aws:braket:us-west-1::device/qpu/rigetti/Aspen-M-3".to_string(),
            ..BraketConfig::default()
        };
        let backend = BraketBackend::new(config);
        assert_eq!(backend.device_type(), BraketDeviceType::Rigetti);
        assert!(backend.is_gate_based());
    }

    #[test]
    fn test_qasm_circuit_generation() {
        let backend = BraketBackend::from_env();
        let gates = vec![
            BraketGate::H(0),
            BraketGate::CX(0, 1),
            BraketGate::Measure(0, 0),
            BraketGate::Measure(1, 1),
        ];
        let qasm = backend.build_qasm_circuit(2, &gates);
        assert!(qasm.contains("OPENQASM 3.0"));
        assert!(qasm.contains("h q[0]"));
        assert!(qasm.contains("cx q[0], q[1]"));
    }
}
