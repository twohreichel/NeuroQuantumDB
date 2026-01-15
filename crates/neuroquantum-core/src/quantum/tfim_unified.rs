//! # Unified TFIM Interface with Automatic Fallback
//!
//! This module provides a unified interface that automatically chooses between:
//! 1. Real quantum TFIM implementation (when available)
//! 2. Classical TFIM simulation (fallback)
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use neuroquantum_core::quantum::{UnifiedTFIMSolver, UnifiedTFIMConfig};
//!
//! let config = UnifiedTFIMConfig::default();
//! let solver = UnifiedTFIMSolver::new(config);
//! let result = solver.solve(&problem)?;
//! ```

use crate::error::{CoreError, CoreResult};
use crate::quantum::{
    tfim::{TFIMProblem, TFIMSolution, TFIMSolver, TransverseFieldConfig},
    tfim_quantum::{QuantumTFIMConfig, QuantumTFIMSolution, QuantumTFIMSolver},
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Unified TFIM configuration with automatic fallback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTFIMConfig {
    /// Try to use quantum implementation first
    pub prefer_quantum: bool,
    /// Quantum configuration (if available)
    pub quantum_config: Option<QuantumTFIMConfig>,
    /// Classical configuration (fallback)
    pub classical_config: TransverseFieldConfig,
    /// Transverse field strength for quantum conversion
    pub transverse_field_strength: f64,
}

impl Default for UnifiedTFIMConfig {
    fn default() -> Self {
        Self {
            prefer_quantum: true,
            quantum_config: Some(QuantumTFIMConfig::default()),
            classical_config: TransverseFieldConfig::default(),
            transverse_field_strength: 1.0,
        }
    }
}

/// Unified TFIM result containing both quantum and classical info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTFIMResult {
    /// Whether quantum implementation was used
    pub used_quantum: bool,
    /// Quantum solution (if quantum was used)
    pub quantum_solution: Option<QuantumTFIMSolution>,
    /// Classical solution (if classical was used)
    pub classical_solution: Option<TFIMSolution>,
    /// Final energy
    pub energy: f64,
    /// Ground state probability/fidelity
    pub quality_metric: f64,
}

/// Unified TFIM solver with automatic fallback
pub struct UnifiedTFIMSolver {
    config: UnifiedTFIMConfig,
}

impl UnifiedTFIMSolver {
    /// Create new unified solver
    pub fn new(config: UnifiedTFIMConfig) -> Self {
        Self { config }
    }

    /// Solve TFIM problem with automatic quantum/classical selection
    pub fn solve(&self, classical_problem: &TFIMProblem) -> CoreResult<UnifiedTFIMResult> {
        if self.config.prefer_quantum && self.config.quantum_config.is_some() {
            // Try quantum first
            match self.solve_quantum(classical_problem) {
                | Ok(result) => {
                    info!("Quantum TFIM succeeded: energy={:.6}", result.energy);
                    return Ok(result);
                },
                | Err(e) => {
                    warn!("Quantum TFIM failed ({}), falling back to classical", e);
                },
            }
        }

        // Fallback to classical
        self.solve_classical(classical_problem)
    }

    /// Solve using quantum implementation
    fn solve_quantum(&self, classical_problem: &TFIMProblem) -> CoreResult<UnifiedTFIMResult> {
        let quantum_config = self
            .config
            .quantum_config
            .as_ref()
            .ok_or_else(|| CoreError::invalid_operation("Quantum config not available"))?;

        // Convert classical problem to quantum problem
        let quantum_problem = QuantumTFIMSolver::from_classical_problem(
            classical_problem,
            self.config.transverse_field_strength,
        )?;

        // Solve using quantum
        let quantum_solver = QuantumTFIMSolver::with_config(quantum_config.clone());
        let quantum_solution = quantum_solver.solve(&quantum_problem)?;

        let energy = quantum_solution.energy;
        let quality_metric = quantum_solution.fidelity.unwrap_or(1.0);

        Ok(UnifiedTFIMResult {
            used_quantum: true,
            quantum_solution: Some(quantum_solution),
            classical_solution: None,
            energy,
            quality_metric,
        })
    }

    /// Solve using classical implementation
    fn solve_classical(&self, classical_problem: &TFIMProblem) -> CoreResult<UnifiedTFIMResult> {
        let classical_solver = TFIMSolver::with_config(self.config.classical_config.clone());
        let classical_solution = classical_solver.solve(classical_problem)?;

        let energy = classical_solution.energy;
        let quality_metric = classical_solution.ground_state_prob;

        info!(
            "Classical TFIM: energy={:.6}, prob={:.6}",
            energy, quality_metric
        );

        Ok(UnifiedTFIMResult {
            used_quantum: false,
            quantum_solution: None,
            classical_solution: Some(classical_solution),
            energy,
            quality_metric,
        })
    }

    /// Create solver that forces quantum implementation (no fallback)
    pub fn quantum_only(quantum_config: QuantumTFIMConfig) -> Self {
        Self {
            config: UnifiedTFIMConfig {
                prefer_quantum: true,
                quantum_config: Some(quantum_config),
                classical_config: TransverseFieldConfig::default(),
                transverse_field_strength: 1.0,
            },
        }
    }

    /// Create solver that forces classical implementation
    pub fn classical_only(classical_config: TransverseFieldConfig) -> Self {
        Self {
            config: UnifiedTFIMConfig {
                prefer_quantum: false,
                quantum_config: None,
                classical_config,
                transverse_field_strength: 1.0,
            },
        }
    }
}

impl Default for UnifiedTFIMSolver {
    fn default() -> Self {
        Self::new(UnifiedTFIMConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantum::SolutionMethod;
    use nalgebra::DMatrix;

    #[test]
    fn test_unified_solver_quantum() {
        let problem = TFIMProblem {
            num_spins: 3,
            couplings: DMatrix::from_fn(3, 3, |i, j| if i != j { 1.0 } else { 0.0 }),
            external_fields: vec![0.0, 0.0, 0.0],
            name: "Test_Unified".to_string(),
        };

        let quantum_config = QuantumTFIMConfig {
            method: SolutionMethod::TrotterSuzuki { order: 2 },
            num_shots: 100,
            hardware_mapping: None,
            error_mitigation: false,
            trotter_steps: 5,
            evolution_time: 1.0,
            seed: None,
        };

        let config = UnifiedTFIMConfig {
            prefer_quantum: true,
            quantum_config: Some(quantum_config),
            classical_config: TransverseFieldConfig::default(),
            transverse_field_strength: 0.5,
        };

        let solver = UnifiedTFIMSolver::new(config);
        let result = solver.solve(&problem).unwrap();

        assert!(result.used_quantum);
        assert!(result.quantum_solution.is_some());
        assert!(result.classical_solution.is_none());
    }

    #[test]
    fn test_unified_solver_classical_fallback() {
        let problem = TFIMProblem {
            num_spins: 3,
            couplings: DMatrix::from_fn(3, 3, |i, j| if i != j { 1.0 } else { 0.0 }),
            external_fields: vec![0.0, 0.0, 0.0],
            name: "Test_Classical".to_string(),
        };

        let config = UnifiedTFIMConfig {
            prefer_quantum: false,
            quantum_config: None,
            classical_config: TransverseFieldConfig::default(),
            transverse_field_strength: 0.5,
        };

        let solver = UnifiedTFIMSolver::new(config);
        let result = solver.solve(&problem).unwrap();

        assert!(!result.used_quantum);
        assert!(result.quantum_solution.is_none());
        assert!(result.classical_solution.is_some());
    }

    #[test]
    fn test_unified_solver_forced_quantum() {
        let quantum_config = QuantumTFIMConfig {
            method: SolutionMethod::QAOA {
                num_layers: 2,
                optimizer: "COBYLA".to_string(),
            },
            num_shots: 100,
            hardware_mapping: None,
            error_mitigation: false,
            trotter_steps: 10,
            evolution_time: 1.0,
            seed: None,
        };

        let solver = UnifiedTFIMSolver::quantum_only(quantum_config);

        let problem = TFIMProblem {
            num_spins: 2,
            couplings: DMatrix::from_fn(2, 2, |i, j| if i != j { 1.0 } else { 0.0 }),
            external_fields: vec![0.0, 0.0],
            name: "Test_Forced".to_string(),
        };

        let result = solver.solve(&problem).unwrap();
        assert!(result.used_quantum);
    }

    #[test]
    fn test_unified_solver_forced_classical() {
        let classical_config = TransverseFieldConfig {
            initial_field: 5.0,
            final_field: 0.1,
            num_steps: 500,
            field_schedule: crate::quantum::tfim::FieldSchedule::Linear,
            temperature: 0.5,
            quantum_tunneling: true,
        };

        let solver = UnifiedTFIMSolver::classical_only(classical_config);

        let problem = TFIMProblem {
            num_spins: 4,
            couplings: DMatrix::from_fn(4, 4, |i, j| if i != j { 1.0 } else { 0.0 }),
            external_fields: vec![0.0, 0.0, 0.0, 0.0],
            name: "Test_Classical_Only".to_string(),
        };

        let result = solver.solve(&problem).unwrap();
        assert!(!result.used_quantum);
    }
}
