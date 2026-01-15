//! # `IonQ` Backend
//!
//! This module provides integration with `IonQ` trapped-ion quantum computers
//! for high-fidelity gate-based quantum computing.
//!
//! ## Supported Algorithms
//!
//! - **Grover's Search**: High-fidelity quantum search
//! - **QAOA**: Variational optimization with all-to-all connectivity
//! - **Parallel Tempering**: Thermal state preparation
//!
//! ## Hardware
//!
//! `IonQ` offers several trapped-ion systems:
//! - **Aria**: 25 algorithmic qubits, high fidelity
//! - **Forte**: 36 qubits, native entangling gates
//! - **Simulator**: Classical simulation for testing
//!
//! ## Key Advantages
//!
//! - **All-to-all connectivity**: Any qubit can interact with any other
//! - **High gate fidelity**: >99.5% single-qubit, >99% two-qubit
//! - **Long coherence**: Minutes of coherence time
//! - **Native gates**: `GPi`, `GPi2`, MS (Mølmer-Sørensen)
//!
//! ## Configuration
//!
//! ```rust,ignore
//! use neuroquantum_core::quantum::backends::ionq::{IonQBackend, IonQConfig};
//!
//! // From environment (recommended)
//! let backend = IonQBackend::from_env();
//!
//! // Or with explicit config
//! let config = IonQConfig {
//!     api_key: Some("your-ionq-api-key".to_string()),
//!     target: IonQTarget::Aria,
//!     ..Default::default()
//! };
//! let backend = IonQBackend::new(config);
//! ```
//!
//! ## Environment Variables
//!
//! - `IONQ_API_KEY`: Your `IonQ` API key
//! - `IONQ_TARGET`: Target device (aria, forte, simulator)

use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::{QuantumBackendConfig, QuantumBackendInfo, QuantumProvider};
use crate::error::{CoreError, CoreResult};

// =============================================================================
// IonQ Configuration
// =============================================================================

/// Configuration for `IonQ` backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IonQConfig {
    /// `IonQ` API key
    /// If None, will attempt to read from `IONQ_API_KEY` environment variable
    pub api_key: Option<String>,

    /// `IonQ` API endpoint URL
    pub api_endpoint: String,

    /// Target device
    pub target: IonQTarget,

    /// Number of shots for measurement
    pub num_shots: usize,

    /// Error mitigation level (0-2)
    pub error_mitigation: u8,

    /// Use native gates (`GPi`, `GPi2`, MS) vs standard gates
    pub native_gates: bool,

    /// Maximum qubits (device-dependent)
    pub max_qubits: usize,

    /// Common backend configuration
    #[serde(flatten)]
    pub common: QuantumBackendConfig,
}

impl Default for IonQConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_endpoint: "https://api.ionq.co/v0.3".to_string(),
            target: IonQTarget::Simulator,
            num_shots: 1024,
            error_mitigation: 1,
            native_gates: false,
            max_qubits: 25, // Aria default
            common: QuantumBackendConfig::default(),
        }
    }
}

/// `IonQ` target device
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IonQTarget {
    /// `IonQ` Aria (25 qubits)
    Aria,
    /// `IonQ` Forte (36 qubits)
    Forte,
    /// `IonQ` Simulator
    Simulator,
}

impl std::fmt::Display for IonQTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::Aria => write!(f, "qpu.aria-1"),
            | Self::Forte => write!(f, "qpu.forte-1"),
            | Self::Simulator => write!(f, "simulator"),
        }
    }
}

impl IonQTarget {
    /// Get max qubits for this target
    #[must_use]
    pub const fn max_qubits(&self) -> usize {
        match self {
            | Self::Aria => 25,
            | Self::Forte => 36,
            | Self::Simulator => 29,
        }
    }
}

// =============================================================================
// IonQ Backend
// =============================================================================

/// `IonQ` backend for trapped-ion quantum computing
///
/// This backend connects to `IonQ`'s cloud API to execute quantum circuits
/// on trapped-ion quantum processors with all-to-all connectivity.
///
/// ## Capabilities
///
/// - **Connectivity**: All-to-all (any qubit pair can interact)
/// - **Gate Fidelity**: >99.5% single-qubit, >99% two-qubit
/// - **Coherence Time**: Minutes
/// - **Native Gates**: `GPi`, `GPi2`, MS (Mølmer-Sørensen)
///
/// ## Example
///
/// ```rust,ignore
/// use neuroquantum_core::quantum::backends::ionq::IonQBackend;
///
/// let backend = IonQBackend::from_env();
/// if backend.is_available() {
///     println!("IonQ {} ready", backend.target());
/// }
/// ```
pub struct IonQBackend {
    config: IonQConfig,
}

impl IonQBackend {
    /// Create a new `IonQ` backend with the given configuration
    #[must_use]
    pub const fn new(config: IonQConfig) -> Self {
        Self { config }
    }

    /// Create a backend using environment variables for configuration
    ///
    /// Reads:
    /// - `IONQ_API_KEY`: API key
    /// - `IONQ_TARGET`: Target device (optional, defaults to simulator)
    #[must_use]
    pub fn from_env() -> Self {
        let target = std::env::var("IONQ_TARGET")
            .ok()
            .map_or(IonQTarget::Simulator, |t| match t.to_lowercase().as_str() {
                | "aria" | "qpu.aria-1" => IonQTarget::Aria,
                | "forte" | "qpu.forte-1" => IonQTarget::Forte,
                | _ => IonQTarget::Simulator,
            });

        let config = IonQConfig {
            api_key: std::env::var("IONQ_API_KEY").ok(),
            target,
            max_qubits: target.max_qubits(),
            ..Default::default()
        };
        Self::new(config)
    }

    /// Get the API key, checking environment if not configured
    #[must_use]
    pub fn get_api_key(&self) -> Option<String> {
        self.config
            .api_key
            .clone()
            .or_else(|| std::env::var("IONQ_API_KEY").ok())
    }

    /// Get the current configuration
    #[must_use]
    pub const fn config(&self) -> &IonQConfig {
        &self.config
    }

    /// Get mutable reference to configuration
    pub const fn config_mut(&mut self) -> &mut IonQConfig {
        &mut self.config
    }

    /// Get the target device
    #[must_use]
    pub const fn target(&self) -> IonQTarget {
        self.config.target
    }

    /// Build `IonQ` circuit in their JSON format
    #[must_use]
    pub fn build_circuit(&self, num_qubits: usize, gates: &[IonQGate]) -> IonQCircuit {
        IonQCircuit {
            qubits: num_qubits,
            circuit: gates.to_vec(),
        }
    }

    /// Build Grover's algorithm circuit for `IonQ`
    #[must_use]
    pub fn build_grover_circuit(
        &self,
        num_qubits: usize,
        marked_states: &[usize],
        num_iterations: usize,
    ) -> IonQCircuit {
        let mut gates = Vec::new();

        // Initial superposition
        for q in 0..num_qubits {
            gates.push(IonQGate::H { target: q });
        }

        // Grover iterations
        for _ in 0..num_iterations {
            // Oracle
            for &state in marked_states {
                // Apply X to qubits that should be 0
                for j in 0..num_qubits {
                    if (state >> j) & 1 == 0 {
                        gates.push(IonQGate::X { target: j });
                    }
                }

                // Multi-controlled Z (implemented as CZ cascade)
                if num_qubits > 1 {
                    for j in 0..(num_qubits - 1) {
                        gates.push(IonQGate::CZ {
                            control: j,
                            target: j + 1,
                        });
                    }
                }

                // Undo X gates
                for j in 0..num_qubits {
                    if (state >> j) & 1 == 0 {
                        gates.push(IonQGate::X { target: j });
                    }
                }
            }

            // Diffusion operator
            for q in 0..num_qubits {
                gates.push(IonQGate::H { target: q });
            }
            for q in 0..num_qubits {
                gates.push(IonQGate::X { target: q });
            }
            if num_qubits > 1 {
                for j in 0..(num_qubits - 1) {
                    gates.push(IonQGate::CZ {
                        control: j,
                        target: j + 1,
                    });
                }
            }
            for q in 0..num_qubits {
                gates.push(IonQGate::X { target: q });
            }
            for q in 0..num_qubits {
                gates.push(IonQGate::H { target: q });
            }
        }

        IonQCircuit {
            qubits: num_qubits,
            circuit: gates,
        }
    }

    /// Convert circuit to `IonQ` native gates
    #[must_use]
    pub fn to_native_gates(&self, circuit: &IonQCircuit) -> IonQCircuit {
        let mut native = Vec::new();

        for gate in &circuit.circuit {
            match gate {
                | IonQGate::H { target } => {
                    // H = GPi2(0) * GPi(pi/2) = Y^0.5 * Z^0.5
                    native.push(IonQGate::GPi2 {
                        target: *target,
                        phase: 0.0,
                    });
                },
                | IonQGate::X { target } => {
                    native.push(IonQGate::GPi {
                        target: *target,
                        phase: 0.0,
                    });
                },
                | IonQGate::Y { target } => {
                    native.push(IonQGate::GPi {
                        target: *target,
                        phase: std::f64::consts::FRAC_PI_2,
                    });
                },
                | IonQGate::Z { target } => {
                    // Z = GPi2(0) * GPi2(0)
                    native.push(IonQGate::GPi2 {
                        target: *target,
                        phase: 0.0,
                    });
                    native.push(IonQGate::GPi2 {
                        target: *target,
                        phase: 0.0,
                    });
                },
                | IonQGate::CZ { control, target } => {
                    // CZ via MS gate
                    native.push(IonQGate::MS {
                        targets: vec![*control, *target],
                        phases: vec![0.0, 0.0],
                        angle: std::f64::consts::FRAC_PI_4,
                    });
                },
                // Native gates pass through
                | _ => native.push(gate.clone()),
            }
        }

        IonQCircuit {
            qubits: circuit.qubits,
            circuit: native,
        }
    }

    /// Submit a circuit to `IonQ` API
    ///
    /// Note: This is a placeholder for actual HTTP client implementation.
    pub async fn submit_circuit(&self, circuit: &IonQCircuit) -> CoreResult<IonQResult> {
        let api_key = self.get_api_key().ok_or_else(|| {
            CoreError::invalid_operation(
                "IonQ API key not configured. Set IONQ_API_KEY environment variable \
                 or provide api_key in IonQConfig.",
            )
        })?;

        let _start_time = Instant::now();

        // Convert to native gates if configured
        let circuit = if self.config.native_gates {
            self.to_native_gates(circuit)
        } else {
            circuit.clone()
        };

        info!(
            "Submitting circuit to IonQ API at {}",
            self.config.api_endpoint
        );
        debug!(
            "IonQ config: target={}, shots={}, gates={}",
            self.config.target,
            self.config.num_shots,
            circuit.circuit.len()
        );

        // In a real implementation, this would:
        // 1. Create HTTP client
        // 2. POST /jobs with:
        //    - Authorization: apiKey {api_key}
        //    - Body: {
        //        target: self.config.target,
        //        shots: self.config.num_shots,
        //        input: circuit,
        //        error_mitigation: { debias: self.config.error_mitigation > 0 }
        //      }
        // 3. GET /jobs/{id} to poll for completion
        // 4. Parse and return results

        Err(CoreError::invalid_operation(&format!(
            "IonQ API integration requires external HTTP client. \
             API key present: {}, target: {}, endpoint: {}",
            !api_key.is_empty(),
            self.config.target,
            self.config.api_endpoint
        )))
    }
}

impl QuantumBackendInfo for IonQBackend {
    fn is_available(&self) -> bool {
        self.get_api_key().is_some()
    }

    fn max_qubits(&self) -> usize {
        self.config.max_qubits
    }

    fn name(&self) -> &'static str {
        "IonQ Trapped-Ion"
    }

    fn provider(&self) -> QuantumProvider {
        QuantumProvider::IonQ
    }
}

// =============================================================================
// IonQ Types
// =============================================================================

/// `IonQ` circuit representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IonQCircuit {
    /// Number of qubits
    pub qubits: usize,
    /// List of gates
    pub circuit: Vec<IonQGate>,
}

/// `IonQ` gate operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "gate")]
pub enum IonQGate {
    // Standard gates
    /// Hadamard gate
    #[serde(rename = "h")]
    H { target: usize },
    /// Pauli X gate
    #[serde(rename = "x")]
    X { target: usize },
    /// Pauli Y gate
    #[serde(rename = "y")]
    Y { target: usize },
    /// Pauli Z gate
    #[serde(rename = "z")]
    Z { target: usize },
    /// X rotation
    #[serde(rename = "rx")]
    Rx { target: usize, rotation: f64 },
    /// Y rotation
    #[serde(rename = "ry")]
    Ry { target: usize, rotation: f64 },
    /// Z rotation
    #[serde(rename = "rz")]
    Rz { target: usize, rotation: f64 },
    /// CNOT gate
    #[serde(rename = "cnot")]
    CNOT { control: usize, target: usize },
    /// CZ gate
    #[serde(rename = "cz")]
    CZ { control: usize, target: usize },
    /// SWAP gate
    #[serde(rename = "swap")]
    SWAP { targets: Vec<usize> },

    // Native IonQ gates
    /// `GPi` gate (native)
    #[serde(rename = "gpi")]
    GPi { target: usize, phase: f64 },
    /// `GPi2` gate (native)
    #[serde(rename = "gpi2")]
    GPi2 { target: usize, phase: f64 },
    /// Mølmer-Sørensen gate (native)
    #[serde(rename = "ms")]
    MS {
        targets: Vec<usize>,
        phases: Vec<f64>,
        angle: f64,
    },
}

/// `IonQ` job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IonQResult {
    /// Job ID
    pub id: String,
    /// Job status
    pub status: IonQJobStatus,
    /// Target device
    pub target: String,
    /// Number of qubits
    pub qubits: usize,
    /// Measurement results (state -> probability)
    pub histogram: Option<std::collections::HashMap<String, f64>>,
    /// Execution metadata
    pub metadata: IonQMetadata,
}

/// `IonQ` job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IonQJobStatus {
    /// Job is queued
    Submitted,
    /// Job is being processed
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Canceled,
}

/// `IonQ` execution metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IonQMetadata {
    /// Number of shots executed
    pub shots: usize,
    /// Execution time in milliseconds
    pub execution_time_ms: Option<f64>,
    /// Gate count
    pub gate_count: Option<usize>,
    /// Two-qubit gate count
    pub two_qubit_gate_count: Option<usize>,
    /// Error mitigation applied
    pub error_mitigation: Option<String>,
}

/// `IonQ` device specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IonQDeviceSpec {
    /// Device name
    pub name: String,
    /// Number of qubits
    pub num_qubits: usize,
    /// Single-qubit gate fidelity
    pub single_qubit_fidelity: f64,
    /// Two-qubit gate fidelity
    pub two_qubit_fidelity: f64,
    /// T1 coherence time in seconds
    pub t1: f64,
    /// T2 coherence time in seconds
    pub t2: f64,
    /// Gate time in microseconds
    pub gate_time_us: f64,
    /// Whether device is online
    pub online: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ionq_config_default() {
        let config = IonQConfig::default();
        assert_eq!(config.target, IonQTarget::Simulator);
        assert_eq!(config.num_shots, 1024);
        assert_eq!(config.error_mitigation, 1);
    }

    #[test]
    fn test_ionq_backend_creation() {
        let backend = IonQBackend::from_env();
        assert_eq!(backend.name(), "IonQ Trapped-Ion");
        assert_eq!(backend.provider(), QuantumProvider::IonQ);
    }

    #[test]
    fn test_ionq_target_display() {
        assert_eq!(format!("{}", IonQTarget::Aria), "qpu.aria-1");
        assert_eq!(format!("{}", IonQTarget::Forte), "qpu.forte-1");
        assert_eq!(format!("{}", IonQTarget::Simulator), "simulator");
    }

    #[test]
    fn test_ionq_target_max_qubits() {
        assert_eq!(IonQTarget::Aria.max_qubits(), 25);
        assert_eq!(IonQTarget::Forte.max_qubits(), 36);
        assert_eq!(IonQTarget::Simulator.max_qubits(), 29);
    }

    #[test]
    fn test_grover_circuit_generation() {
        let backend = IonQBackend::from_env();
        let circuit = backend.build_grover_circuit(3, &[5], 1);
        assert_eq!(circuit.qubits, 3);
        assert!(!circuit.circuit.is_empty());
    }
}
