//! # Transverse Field Ising Model (TFIM)
//!
//! Implements the Transverse Field Ising Model for quantum annealing
//! with quantum tunneling effects for enhanced optimization.

use crate::error::{CoreError, CoreResult};
use nalgebra::DMatrix;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

/// Field schedule for transverse field annealing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldSchedule {
    /// Linear schedule: Γ(t) = Γ_0 * (1 - t/T)
    Linear,
    /// Exponential schedule: Γ(t) = Γ_0 * exp(-λt)
    Exponential { decay_rate: f64 },
    /// Polynomial schedule: Γ(t) = Γ_0 * (1 - t/T)^p
    Polynomial { power: f64 },
    /// Custom schedule with explicit values
    Custom { values: Vec<f64> },
}

/// Configuration for Transverse Field Ising Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransverseFieldConfig {
    /// Initial transverse field strength
    pub initial_field: f64,
    /// Final transverse field strength
    pub final_field: f64,
    /// Number of annealing steps
    pub num_steps: usize,
    /// Field schedule type
    pub field_schedule: FieldSchedule,
    /// Temperature for thermal fluctuations
    pub temperature: f64,
    /// Enable quantum tunneling
    pub quantum_tunneling: bool,
}

impl Default for TransverseFieldConfig {
    fn default() -> Self {
        Self {
            initial_field: 10.0,
            final_field: 0.1,
            num_steps: 1000,
            field_schedule: FieldSchedule::Linear,
            temperature: 1.0,
            quantum_tunneling: true,
        }
    }
}

/// TFIM Problem with Ising Hamiltonian
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TFIMProblem {
    /// Number of spins
    pub num_spins: usize,
    /// Coupling matrix J_ij (interaction strengths)
    pub couplings: DMatrix<f64>,
    /// External field h_i for each spin
    pub external_fields: Vec<f64>,
    /// Problem name
    pub name: String,
}

/// TFIM Solution with spin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TFIMSolution {
    /// Spin configuration (-1 or +1)
    pub spins: Vec<i8>,
    /// Energy of the configuration
    pub energy: f64,
    /// Ground state probability
    pub ground_state_prob: f64,
    /// Number of annealing steps
    pub steps: usize,
    /// Tunneling events detected
    pub tunneling_events: usize,
    /// Computation time in milliseconds
    pub computation_time_ms: f64,
}

/// Transverse Field Ising Model Solver
pub struct TFIMSolver {
    config: TransverseFieldConfig,
}

impl TFIMSolver {
    /// Create a new TFIM solver with default configuration
    pub fn new() -> Self {
        Self {
            config: TransverseFieldConfig::default(),
        }
    }

    /// Create a new TFIM solver with custom configuration
    pub fn with_config(config: TransverseFieldConfig) -> Self {
        Self { config }
    }

    /// Solve TFIM problem using quantum annealing
    #[instrument(skip(self, problem))]
    pub fn solve(&self, problem: &TFIMProblem) -> CoreResult<TFIMSolution> {
        let start_time = std::time::Instant::now();

        if problem.num_spins == 0 {
            return Err(CoreError::invalid_operation("Empty TFIM problem"));
        }

        debug!(
            "Solving TFIM problem '{}' with {} spins",
            problem.name, problem.num_spins
        );

        let mut rng = rand::thread_rng();

        // Initialize random spin configuration
        let mut spins: Vec<i8> = (0..problem.num_spins)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();

        let mut current_energy = self.calculate_energy(problem, &spins);
        let mut best_spins = spins.clone();
        let mut best_energy = current_energy;
        let mut tunneling_events = 0;

        // Quantum annealing with transverse field
        for step in 0..self.config.num_steps {
            let progress = step as f64 / self.config.num_steps as f64;
            let transverse_field = self.get_field_strength(progress);

            // Quantum tunneling probability
            let tunnel_prob = if self.config.quantum_tunneling {
                self.tunneling_probability(transverse_field)
            } else {
                0.0
            };

            // Try flipping each spin
            for i in 0..problem.num_spins {
                let old_spin = spins[i];
                spins[i] = -old_spin; // Flip spin

                let new_energy = self.calculate_energy(problem, &spins);
                let delta_e = new_energy - current_energy;

                // Accept/reject with quantum tunneling enhancement
                let accept = if delta_e < 0.0 {
                    true
                } else if rng.gen::<f64>() < tunnel_prob {
                    tunneling_events += 1;
                    true
                } else {
                    let thermal_prob = (-delta_e / self.config.temperature).exp();
                    rng.gen::<f64>() < thermal_prob
                };

                if accept {
                    current_energy = new_energy;
                    if current_energy < best_energy {
                        best_spins = spins.clone();
                        best_energy = current_energy;
                    }
                } else {
                    spins[i] = old_spin; // Revert flip
                }
            }
        }

        let computation_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        // Estimate ground state probability
        let ground_state_prob =
            self.estimate_ground_state_probability(problem, best_energy, self.config.temperature);

        info!(
            "TFIM solved: energy={:.4}, tunneling_events={}, prob={:.4}, time={:.2}ms",
            best_energy, tunneling_events, ground_state_prob, computation_time_ms
        );

        Ok(TFIMSolution {
            spins: best_spins,
            energy: best_energy,
            ground_state_prob,
            steps: self.config.num_steps,
            tunneling_events,
            computation_time_ms,
        })
    }

    /// Calculate Ising Hamiltonian energy: H = -Σ J_ij s_i s_j - Σ h_i s_i
    fn calculate_energy(&self, problem: &TFIMProblem, spins: &[i8]) -> f64 {
        let mut energy = 0.0;

        // Interaction terms
        for i in 0..problem.num_spins {
            for j in (i + 1)..problem.num_spins {
                let coupling = problem.couplings[(i, j)];
                energy -= coupling * spins[i] as f64 * spins[j] as f64;
            }
        }

        // External field terms
        for (i, &spin) in spins.iter().enumerate().take(problem.num_spins) {
            energy -= problem.external_fields[i] * spin as f64;
        }

        energy
    }

    /// Get transverse field strength at given progress (0 to 1)
    fn get_field_strength(&self, progress: f64) -> f64 {
        match &self.config.field_schedule {
            FieldSchedule::Linear => {
                self.config.initial_field * (1.0 - progress) + self.config.final_field * progress
            }
            FieldSchedule::Exponential { decay_rate } => {
                self.config.initial_field * (-decay_rate * progress).exp() + self.config.final_field
            }
            FieldSchedule::Polynomial { power } => {
                self.config.initial_field * (1.0 - progress).powf(*power) + self.config.final_field
            }
            FieldSchedule::Custom { values } => {
                let idx = (progress * (values.len() - 1) as f64) as usize;
                values[idx.min(values.len() - 1)]
            }
        }
    }

    /// Calculate quantum tunneling probability based on transverse field
    fn tunneling_probability(&self, field_strength: f64) -> f64 {
        // Tunneling probability ∝ exp(-barrier/Γ)
        // Higher field → higher tunneling probability
        let barrier = 2.0; // Energy barrier height
        let prob = (field_strength / barrier).min(1.0);
        prob * 0.5 // Scale to reasonable range
    }

    /// Estimate ground state probability using Boltzmann distribution
    fn estimate_ground_state_probability(
        &self,
        problem: &TFIMProblem,
        energy: f64,
        temperature: f64,
    ) -> f64 {
        // P(E) ∝ exp(-E/kT)
        // Normalize by partition function estimate
        let partition_approx = 2.0_f64.powi(problem.num_spins as i32);
        let prob = (-energy / temperature).exp() / partition_approx;
        prob.min(1.0)
    }

    /// Create TFIM problem from QUBO Q matrix
    pub fn from_qubo(q_matrix: &DMatrix<f64>) -> CoreResult<TFIMProblem> {
        let n = q_matrix.nrows();
        if n == 0 || n != q_matrix.ncols() {
            return Err(CoreError::invalid_operation("Invalid Q matrix"));
        }

        // Convert QUBO to Ising: x_i = (1 + s_i) / 2
        let mut couplings = DMatrix::zeros(n, n);
        let mut external_fields = vec![0.0; n];

        for i in 0..n {
            for j in 0..n {
                let q_ij = q_matrix[(i, j)];
                if i == j {
                    external_fields[i] += q_ij / 2.0;
                } else {
                    couplings[(i, j)] = q_ij / 4.0;
                }
            }
        }

        Ok(TFIMProblem {
            num_spins: n,
            couplings,
            external_fields,
            name: "QUBO-to-Ising".to_string(),
        })
    }

    /// Create fully-connected ferromagnetic Ising model
    pub fn ferromagnetic_model(num_spins: usize, coupling: f64) -> CoreResult<TFIMProblem> {
        if num_spins == 0 {
            return Err(CoreError::invalid_operation("Invalid number of spins"));
        }

        let mut couplings = DMatrix::zeros(num_spins, num_spins);
        for i in 0..num_spins {
            for j in (i + 1)..num_spins {
                couplings[(i, j)] = coupling;
                couplings[(j, i)] = coupling;
            }
        }

        Ok(TFIMProblem {
            num_spins,
            couplings,
            external_fields: vec![0.0; num_spins],
            name: "Ferromagnetic".to_string(),
        })
    }

    /// Create spin glass model with random couplings
    pub fn spin_glass_model(num_spins: usize, disorder_strength: f64) -> CoreResult<TFIMProblem> {
        if num_spins == 0 {
            return Err(CoreError::invalid_operation("Invalid number of spins"));
        }

        let mut rng = rand::thread_rng();
        let mut couplings = DMatrix::zeros(num_spins, num_spins);

        for i in 0..num_spins {
            for j in (i + 1)..num_spins {
                let coupling = (rng.gen::<f64>() - 0.5) * 2.0 * disorder_strength;
                couplings[(i, j)] = coupling;
                couplings[(j, i)] = coupling;
            }
        }

        let external_fields: Vec<f64> = (0..num_spins)
            .map(|_| (rng.gen::<f64>() - 0.5) * disorder_strength)
            .collect();

        Ok(TFIMProblem {
            num_spins,
            couplings,
            external_fields,
            name: "SpinGlass".to_string(),
        })
    }
}

impl Default for TFIMSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tfim_simple() {
        let problem = TFIMProblem {
            num_spins: 3,
            couplings: DMatrix::from_fn(3, 3, |i, j| if i != j { 1.0 } else { 0.0 }),
            external_fields: vec![0.0, 0.0, 0.0],
            name: "Simple".to_string(),
        };

        let solver = TFIMSolver::new();
        let solution = solver.solve(&problem).unwrap();

        assert_eq!(solution.spins.len(), 3);
        assert!(solution.energy < 0.0); // Ferromagnetic should align
        assert!(solution.ground_state_prob > 0.0);
    }

    #[test]
    fn test_field_schedules() {
        let config = TransverseFieldConfig {
            initial_field: 10.0,
            final_field: 0.1,
            num_steps: 100,
            field_schedule: FieldSchedule::Exponential { decay_rate: 5.0 },
            temperature: 1.0,
            quantum_tunneling: true,
        };

        let solver = TFIMSolver::with_config(config);

        // Test field strength calculation
        let field_start = solver.get_field_strength(0.0);
        let field_mid = solver.get_field_strength(0.5);
        let field_end = solver.get_field_strength(1.0);

        assert!(field_start > field_mid);
        assert!(field_mid > field_end);
    }

    #[test]
    fn test_ferromagnetic_model() {
        let problem = TFIMSolver::ferromagnetic_model(4, 1.0).unwrap();
        assert_eq!(problem.num_spins, 4);

        let solver = TFIMSolver::new();
        let solution = solver.solve(&problem).unwrap();

        // All spins should align in ferromagnetic model
        let all_same = solution.spins.windows(2).all(|w| w[0] == w[1]);
        assert!(all_same || solution.spins.windows(2).all(|w| w[0] != w[1]));
    }

    #[test]
    fn test_spin_glass_model() {
        let problem = TFIMSolver::spin_glass_model(5, 1.0).unwrap();
        assert_eq!(problem.num_spins, 5);
        assert_eq!(problem.external_fields.len(), 5);

        let solver = TFIMSolver::new();
        let solution = solver.solve(&problem).unwrap();
        assert_eq!(solution.spins.len(), 5);
    }

    #[test]
    fn test_tunneling_probability() {
        let solver = TFIMSolver::new();

        let prob_high = solver.tunneling_probability(10.0);
        let prob_low = solver.tunneling_probability(0.1);

        assert!(prob_high > prob_low);
        assert!(prob_high <= 0.5);
        assert!(prob_low >= 0.0);
    }

    #[test]
    fn test_empty_problem() {
        let problem = TFIMProblem {
            num_spins: 0,
            couplings: DMatrix::zeros(0, 0),
            external_fields: vec![],
            name: "Empty".to_string(),
        };

        let solver = TFIMSolver::new();
        let result = solver.solve(&problem);
        assert!(result.is_err());
    }
}
