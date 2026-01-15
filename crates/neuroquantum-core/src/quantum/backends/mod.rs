//! # Unified Quantum Hardware Backends
//!
//! This module provides a unified interface for accessing real quantum computing hardware
//! from various providers. It consolidates all hardware-specific implementations into
//! provider-focused modules for better organization and maintainability.
//!
//! ## Supported Providers
//!
//! - **IBM Quantum**: Gate-based superconducting quantum computers via Qiskit Runtime
//! - **AWS Braket**: Multi-vendor access to `IonQ`, Rigetti, D-Wave, and simulators
//! - **D-Wave**: Native quantum annealing for optimization problems
//! - **`IonQ`**: High-fidelity trapped-ion quantum computers
//!
//! ## Architecture
//!
//! Each provider module implements the common backend traits for various quantum algorithms:
//!
//! | Algorithm | IBM Quantum | AWS Braket | D-Wave | `IonQ` |
//! |-----------|-------------|------------|--------|------|
//! | Grover's Search | ✓ | ✓ | ✗ | ✓ |
//! | QUBO/QAOA | ✓ | ✓ | ✓ | ✗ |
//! | TFIM | ✗ | ✓ | ✓ | ✗ |
//! | Parallel Tempering | ✓ | ✓ | ✓ | ✓ |
//!
//! ## Configuration
//!
//! All backends support configuration via:
//! - Environment variables (recommended for production)
//! - Configuration files (TOML format)
//! - Direct API injection
//!
//! ### Environment Variables
//!
//! | Provider | Variables |
//! |----------|-----------|
//! | IBM Quantum | `IBM_QUANTUM_API_KEY`, `IBM_QUANTUM_BACKEND` |
//! | AWS Braket | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `AWS_BRAKET_DEVICE_ARN` |
//! | D-Wave | `DWAVE_API_TOKEN`, `DWAVE_SOLVER` |
//! | `IonQ` | `IONQ_API_KEY` |
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use neuroquantum_core::quantum::backends::{
//!     ibm::IBMQuantumBackend,
//!     dwave::DWaveBackend,
//!     braket::BraketBackend,
//!     QuantumBackendConfig,
//! };
//!
//! // Auto-detect from environment
//! let ibm = IBMQuantumBackend::from_env();
//! let dwave = DWaveBackend::from_env();
//!
//! // Or configure explicitly
//! let config = IBMQuantumConfig {
//!     api_token: Some("your-token".to_string()),
//!     backend_name: "ibm_brisbane".to_string(),
//!     ..Default::default()
//! };
//! let ibm = IBMQuantumBackend::new(config);
//! ```

pub mod braket;
pub mod dwave;
pub mod ibm;
pub mod ionq;

// Re-export all backend types for convenience
pub use braket::*;
pub use dwave::*;
pub use ibm::*;
pub use ionq::*;
use serde::{Deserialize, Serialize};

// =============================================================================
// Common Backend Traits
// =============================================================================

/// Common trait for all quantum backends providing basic availability and info
pub trait QuantumBackendInfo: Send + Sync {
    /// Check if the backend is available and properly configured
    fn is_available(&self) -> bool;

    /// Get the maximum number of qubits/variables this backend can handle
    fn max_qubits(&self) -> usize;

    /// Get the backend name for logging and diagnostics
    fn name(&self) -> &str;

    /// Get the provider type
    fn provider(&self) -> QuantumProvider;
}

/// Enumeration of supported quantum computing providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuantumProvider {
    /// IBM Quantum - Superconducting gate-based
    IBMQuantum,
    /// AWS Braket - Multi-vendor access
    AWSBraket,
    /// D-Wave - Quantum annealing
    DWave,
    /// `IonQ` - Trapped-ion gate-based
    IonQ,
    /// Local simulator (classical fallback)
    LocalSimulator,
}

impl std::fmt::Display for QuantumProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            | Self::IBMQuantum => write!(f, "IBM Quantum"),
            | Self::AWSBraket => write!(f, "AWS Braket"),
            | Self::DWave => write!(f, "D-Wave"),
            | Self::IonQ => write!(f, "IonQ"),
            | Self::LocalSimulator => write!(f, "Local Simulator"),
        }
    }
}

/// Common configuration for quantum backend connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumBackendConfig {
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Enable verbose logging
    pub verbose: bool,
    /// Use fallback to local simulation when hardware unavailable
    pub fallback_to_simulation: bool,
}

impl Default for QuantumBackendConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 300,
            max_retries: 3,
            verbose: false,
            fallback_to_simulation: true,
        }
    }
}

/// Result from quantum hardware execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumExecutionResult {
    /// Provider that executed the computation
    pub provider: QuantumProvider,
    /// Whether hardware was used (false = simulation fallback)
    pub hardware_execution: bool,
    /// Total execution time in milliseconds
    pub execution_time_ms: f64,
    /// Number of shots/samples taken
    pub num_shots: usize,
    /// Job ID from the provider (if applicable)
    pub job_id: Option<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl Default for QuantumExecutionResult {
    fn default() -> Self {
        Self {
            provider: QuantumProvider::LocalSimulator,
            hardware_execution: false,
            execution_time_ms: 0.0,
            num_shots: 0,
            job_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }
}

// =============================================================================
// Unified Backend Factory
// =============================================================================

/// Factory for creating quantum backends from environment configuration
pub struct QuantumBackendFactory;

impl QuantumBackendFactory {
    /// Check which quantum backends are available based on environment configuration
    #[must_use]
    pub fn available_providers() -> Vec<QuantumProvider> {
        let mut providers = vec![QuantumProvider::LocalSimulator];

        // Check IBM Quantum
        if std::env::var("IBM_QUANTUM_API_KEY").is_ok() {
            providers.push(QuantumProvider::IBMQuantum);
        }

        // Check D-Wave
        if std::env::var("DWAVE_API_TOKEN").is_ok() {
            providers.push(QuantumProvider::DWave);
        }

        // Check AWS Braket
        if std::env::var("AWS_ACCESS_KEY_ID").is_ok()
            || std::env::var("AWS_PROFILE").is_ok()
            || std::path::Path::new(
                &std::env::var("HOME")
                    .map(|h| format!("{h}/.aws/credentials"))
                    .unwrap_or_default(),
            )
            .exists()
        {
            providers.push(QuantumProvider::AWSBraket);
        }

        // Check IonQ
        if std::env::var("IONQ_API_KEY").is_ok() {
            providers.push(QuantumProvider::IonQ);
        }

        providers
    }

    /// Get the best available provider for a specific algorithm
    #[must_use]
    pub fn best_provider_for_algorithm(algorithm: &str) -> QuantumProvider {
        let available = Self::available_providers();

        match algorithm.to_lowercase().as_str() {
            | "grover" | "grovers" | "grover_search" => {
                // Prefer IonQ for high-fidelity gate operations
                if available.contains(&QuantumProvider::IonQ) {
                    return QuantumProvider::IonQ;
                }
                if available.contains(&QuantumProvider::IBMQuantum) {
                    return QuantumProvider::IBMQuantum;
                }
                if available.contains(&QuantumProvider::AWSBraket) {
                    return QuantumProvider::AWSBraket;
                }
            },
            | "qubo" | "qaoa" | "optimization" => {
                // Prefer D-Wave for native QUBO
                if available.contains(&QuantumProvider::DWave) {
                    return QuantumProvider::DWave;
                }
                if available.contains(&QuantumProvider::IBMQuantum) {
                    return QuantumProvider::IBMQuantum;
                }
                if available.contains(&QuantumProvider::AWSBraket) {
                    return QuantumProvider::AWSBraket;
                }
            },
            | "tfim" | "ising" | "annealing" => {
                // Prefer D-Wave for native Ising model
                if available.contains(&QuantumProvider::DWave) {
                    return QuantumProvider::DWave;
                }
                if available.contains(&QuantumProvider::AWSBraket) {
                    return QuantumProvider::AWSBraket;
                }
            },
            | "parallel_tempering" | "pt" | "thermal" => {
                // Prefer D-Wave for thermal sampling
                if available.contains(&QuantumProvider::DWave) {
                    return QuantumProvider::DWave;
                }
                if available.contains(&QuantumProvider::IBMQuantum) {
                    return QuantumProvider::IBMQuantum;
                }
                if available.contains(&QuantumProvider::IonQ) {
                    return QuantumProvider::IonQ;
                }
            },
            | _ => {},
        }

        QuantumProvider::LocalSimulator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_provider_display() {
        assert_eq!(format!("{}", QuantumProvider::IBMQuantum), "IBM Quantum");
        assert_eq!(format!("{}", QuantumProvider::DWave), "D-Wave");
        assert_eq!(format!("{}", QuantumProvider::AWSBraket), "AWS Braket");
        assert_eq!(format!("{}", QuantumProvider::IonQ), "IonQ");
        assert_eq!(
            format!("{}", QuantumProvider::LocalSimulator),
            "Local Simulator"
        );
    }

    #[test]
    fn test_default_config() {
        let config = QuantumBackendConfig::default();
        assert_eq!(config.timeout_secs, 300);
        assert_eq!(config.max_retries, 3);
        assert!(!config.verbose);
        assert!(config.fallback_to_simulation);
    }

    #[test]
    fn test_available_providers_always_includes_simulator() {
        let providers = QuantumBackendFactory::available_providers();
        assert!(providers.contains(&QuantumProvider::LocalSimulator));
    }
}
