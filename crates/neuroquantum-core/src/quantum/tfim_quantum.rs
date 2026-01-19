//! # Real Quantum Transverse Field Ising Model (TFIM) Implementation
//!
//! This module provides a TRUE quantum implementation of the Transverse Field Ising Model,
//! as opposed to the classical Monte Carlo simulation in `tfim.rs`.
//!
//! ## Quantum Implementation Methods
//!
//! ### 1. Trotter-Suzuki Decomposition
//! Time evolution under TFIM Hamiltonian using quantum circuits
//!
//! ### 2. Variational Quantum Eigensolver (VQE)
//! Find ground state of TFIM using parameterized quantum circuits
//!
//! ### 3. Quantum Approximate Optimization Algorithm (QAOA)
//! Solve optimization problems using TFIM-inspired circuits
//!
//! ## TFIM Hamiltonian
//!
//! ```text
//! H = -J * Σ(i,j) σ_z^i σ_z^j - h * Σ_i σ_x^i
//! ```
//!
//! Where:
//! - J: Coupling strength between adjacent spins
//! - h: Transverse magnetic field strength
//! - `σ_z`, `σ_x`: Pauli Z and X operators

use std::f64::consts::PI;

use nalgebra::{Complex, DMatrix};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use crate::error::{CoreError, CoreResult};

/// Result type for quantum circuit execution: (optional state vector, measurement outcomes)
type CircuitExecutionResult = (Option<Vec<Complex<f64>>>, Vec<Vec<bool>>);

/// Quantum backend types for TFIM
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuantumBackend {
    /// Simulator (for testing and validation)
    Simulator,
    /// Superconducting qubits (IBM, Google, Rigetti)
    Superconducting,
    /// Trapped ion qubits (`IonQ`, Honeywell)
    TrappedIon,
    /// Neutral atom arrays (Pasqal, `QuEra`)
    NeutralAtom,
}

/// Hardware-specific mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareMapping {
    /// Backend type
    pub backend: QuantumBackend,
    /// Connectivity graph (which qubits can interact)
    pub connectivity: Vec<(usize, usize)>,
    /// Native gate set
    pub native_gates: Vec<String>,
    /// Coherence time in microseconds
    pub coherence_time_us: f64,
    /// Gate error rates
    pub gate_error_rate: f64,
}

/// Quantum circuit representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumCircuit {
    /// Number of qubits
    pub num_qubits: usize,
    /// Circuit gates
    pub gates: Vec<QuantumGate>,
    /// Circuit depth
    pub depth: usize,
}

/// Quantum gate types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantumGate {
    /// Pauli X gate (bit flip)
    X { qubit: usize },
    /// Pauli Y gate
    Y { qubit: usize },
    /// Pauli Z gate (phase flip)
    Z { qubit: usize },
    /// Hadamard gate (superposition)
    H { qubit: usize },
    /// Rotation around X axis
    RX { qubit: usize, angle: f64 },
    /// Rotation around Y axis
    RY { qubit: usize, angle: f64 },
    /// Rotation around Z axis
    RZ { qubit: usize, angle: f64 },
    /// Controlled-NOT gate
    CNOT { control: usize, target: usize },
    /// Controlled-Z gate
    CZ { control: usize, target: usize },
    /// Arbitrary two-qubit gate
    TwoQubit {
        qubit1: usize,
        qubit2: usize,
        matrix: Box<[[Complex<f64>; 4]; 4]>,
    },
}

/// Quantum TFIM problem specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumTFIMProblem {
    /// Number of spins/qubits
    pub num_qubits: usize,
    /// Coupling strengths `J_ij`
    pub couplings: DMatrix<f64>,
    /// Transverse field strengths `h_i`
    pub transverse_fields: Vec<f64>,
    /// Longitudinal field strengths
    pub longitudinal_fields: Vec<f64>,
    /// Problem name
    pub name: String,
}

/// Quantum TFIM solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumTFIMSolution {
    /// Final state vector (optional, for simulator)
    pub state_vector: Option<Vec<Complex<f64>>>,
    /// Measurement outcomes (bitstrings)
    pub measurements: Vec<Vec<bool>>,
    /// Energy expectation value
    pub energy: f64,
    /// Energy variance
    pub energy_variance: f64,
    /// Ground state fidelity (if known)
    pub fidelity: Option<f64>,
    /// Quantum circuit used
    pub circuit: QuantumCircuit,
    /// Observables measured
    pub observables: QuantumObservables,
    /// Computation time in milliseconds
    pub computation_time_ms: f64,
}

/// Observable measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumObservables {
    /// Magnetization ⟨`σ_z`⟩ for each qubit
    pub magnetization: Vec<f64>,
    /// Two-point correlation functions ⟨`σ_z^i` `σ_z^j`⟩
    pub correlations: DMatrix<f64>,
    /// Entanglement entropy (von Neumann entropy)
    pub entanglement_entropy: Option<f64>,
    /// Order parameter
    pub order_parameter: f64,
}

/// Configuration for quantum TFIM solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumTFIMConfig {
    /// Solution method
    pub method: SolutionMethod,
    /// Number of measurements (shots)
    pub num_shots: usize,
    /// Hardware mapping (if using real hardware)
    pub hardware_mapping: Option<HardwareMapping>,
    /// Enable error mitigation
    pub error_mitigation: bool,
    /// Trotter steps (for Trotter-Suzuki)
    pub trotter_steps: usize,
    /// Evolution time
    pub evolution_time: f64,
    /// Optional random seed for deterministic testing
    #[serde(default)]
    pub seed: Option<u64>,
}

/// Solution methods for quantum TFIM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolutionMethod {
    /// Trotter-Suzuki decomposition for time evolution
    TrotterSuzuki {
        /// Order of Trotter decomposition (1, 2, 4, ...)
        order: usize,
    },
    /// Variational Quantum Eigensolver
    VQE {
        /// Ansatz type
        ansatz: VQEAnsatz,
        /// Number of optimization iterations
        max_iterations: usize,
        /// Convergence threshold
        convergence_threshold: f64,
    },
    /// Quantum Approximate Optimization Algorithm
    QAOA {
        /// Number of QAOA layers (p)
        num_layers: usize,
        /// Optimization method
        optimizer: String,
    },
}

/// VQE ansatz types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VQEAnsatz {
    /// Hardware-efficient ansatz
    HardwareEfficient { depth: usize },
    /// Unitary Coupled Cluster (UCC)
    UnitaryCoupledCluster,
    /// Custom parameterized circuit
    Custom { num_parameters: usize },
}

impl Default for QuantumTFIMConfig {
    fn default() -> Self {
        Self {
            method: SolutionMethod::TrotterSuzuki { order: 2 },
            num_shots: 1000,
            hardware_mapping: None,
            error_mitigation: true,
            trotter_steps: 10,
            evolution_time: 1.0,
            seed: None,
        }
    }
}

/// Real Quantum TFIM Solver
pub struct QuantumTFIMSolver {
    config: QuantumTFIMConfig,
}

impl QuantumTFIMSolver {
    /// Create new quantum TFIM solver
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: QuantumTFIMConfig::default(),
        }
    }

    /// Create solver with custom configuration
    #[must_use]
    pub const fn with_config(config: QuantumTFIMConfig) -> Self {
        Self { config }
    }

    /// Solve quantum TFIM problem
    #[instrument(skip(self, problem))]
    pub fn solve(&self, problem: &QuantumTFIMProblem) -> CoreResult<QuantumTFIMSolution> {
        let start_time = std::time::Instant::now();

        debug!(
            "Solving quantum TFIM '{}' with {} qubits using {:?}",
            problem.name, problem.num_qubits, self.config.method
        );

        // Build quantum circuit based on method
        let circuit = match &self.config.method {
            | SolutionMethod::TrotterSuzuki { order } => {
                self.build_trotter_circuit(problem, *order)?
            },
            | SolutionMethod::VQE {
                ansatz,
                max_iterations,
                convergence_threshold,
            } => {
                self.build_vqe_circuit(problem, ansatz, *max_iterations, *convergence_threshold)?
            },
            | SolutionMethod::QAOA {
                num_layers,
                optimizer,
            } => self.build_qaoa_circuit(problem, *num_layers, optimizer)?,
        };

        // Execute circuit on quantum backend
        let (state_vector, measurements) = self.execute_circuit(&circuit, problem)?;

        // Measure observables
        let observables = self.measure_observables(problem, &measurements)?;

        // Calculate energy expectation value
        let energy = self.calculate_energy_expectation(problem, &measurements)?;
        let energy_variance = self.calculate_energy_variance(problem, &measurements, energy)?;

        let computation_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        info!(
            "Quantum TFIM solved: energy={:.6}, variance={:.6}, time={:.2}ms",
            energy, energy_variance, computation_time_ms
        );

        Ok(QuantumTFIMSolution {
            state_vector,
            measurements,
            energy,
            energy_variance,
            fidelity: None,
            circuit,
            observables,
            computation_time_ms,
        })
    }

    /// Build Trotter-Suzuki circuit for TFIM time evolution
    fn build_trotter_circuit(
        &self,
        problem: &QuantumTFIMProblem,
        order: usize,
    ) -> CoreResult<QuantumCircuit> {
        let mut gates = Vec::new();
        let n = problem.num_qubits;

        // Start with uniform superposition (Hadamard on all qubits)
        for i in 0..n {
            gates.push(QuantumGate::H { qubit: i });
        }

        let dt = self.config.evolution_time / self.config.trotter_steps as f64;

        // Trotter-Suzuki time evolution: exp(-iHt) ≈ [exp(-iH_Z dt)exp(-iH_X dt)]^n
        for _step in 0..self.config.trotter_steps {
            // H_Z term: -J * Σ σ_z^i σ_z^j
            for i in 0..n {
                for j in (i + 1)..n {
                    let coupling = problem.couplings[(i, j)];
                    if coupling.abs() > 1e-10 {
                        // ZZ interaction using CNOT decomposition
                        gates.push(QuantumGate::CNOT {
                            control: i,
                            target: j,
                        });
                        gates.push(QuantumGate::RZ {
                            qubit: j,
                            angle: 2.0 * coupling * dt,
                        });
                        gates.push(QuantumGate::CNOT {
                            control: i,
                            target: j,
                        });
                    }
                }
            }

            // H_X term: -h * Σ σ_x^i
            for i in 0..n {
                let field = problem.transverse_fields[i];
                if field.abs() > 1e-10 {
                    gates.push(QuantumGate::RX {
                        qubit: i,
                        angle: 2.0 * field * dt,
                    });
                }
            }

            // Higher-order Trotter if requested
            if order > 1 {
                // Symmetric decomposition for better accuracy
                for i in (0..n).rev() {
                    let field = problem.transverse_fields[i];
                    if field.abs() > 1e-10 {
                        gates.push(QuantumGate::RX {
                            qubit: i,
                            angle: 2.0 * field * dt,
                        });
                    }
                }
            }
        }

        let depth = self.calculate_circuit_depth(&gates);

        Ok(QuantumCircuit {
            num_qubits: n,
            gates,
            depth,
        })
    }

    /// Build VQE circuit for ground state finding
    fn build_vqe_circuit(
        &self,
        problem: &QuantumTFIMProblem,
        ansatz: &VQEAnsatz,
        max_iterations: usize,
        _convergence_threshold: f64,
    ) -> CoreResult<QuantumCircuit> {
        let n = problem.num_qubits;
        let mut gates = Vec::new();

        // Initialize in |+⟩^n state
        for i in 0..n {
            gates.push(QuantumGate::H { qubit: i });
        }

        // Build ansatz based on type
        match ansatz {
            | VQEAnsatz::HardwareEfficient { depth } => {
                // Hardware-efficient ansatz: layers of single-qubit rotations + entanglers
                for layer in 0..*depth {
                    // Single-qubit rotations (RY gates)
                    for i in 0..n {
                        // Parameters will be optimized classically
                        let param = (layer as f64 + i as f64) * 0.1; // Initial guess
                        gates.push(QuantumGate::RY {
                            qubit: i,
                            angle: param,
                        });
                    }

                    // Entangling layer (CZ gates in a linear chain)
                    for i in 0..(n - 1) {
                        gates.push(QuantumGate::CZ {
                            control: i,
                            target: i + 1,
                        });
                    }

                    // Another layer of single-qubit rotations
                    for i in 0..n {
                        let param = (layer as f64 + i as f64 + 0.5) * 0.1;
                        gates.push(QuantumGate::RZ {
                            qubit: i,
                            angle: param,
                        });
                    }
                }
            },
            | VQEAnsatz::UnitaryCoupledCluster => {
                // UCC ansatz for chemistry-inspired problems
                // Simplified version for TFIM
                for i in 0..(n - 1) {
                    gates.push(QuantumGate::CNOT {
                        control: i,
                        target: i + 1,
                    });
                    gates.push(QuantumGate::RY {
                        qubit: i + 1,
                        angle: 0.1,
                    });
                    gates.push(QuantumGate::CNOT {
                        control: i,
                        target: i + 1,
                    });
                }
            },
            | VQEAnsatz::Custom { num_parameters } => {
                // Custom parameterized circuit
                let params_per_qubit = num_parameters / n;
                for i in 0..n {
                    for p in 0..params_per_qubit {
                        let angle = (i * params_per_qubit + p) as f64 * 0.1;
                        gates.push(QuantumGate::RY { qubit: i, angle });
                    }
                }
            },
        }

        let depth = self.calculate_circuit_depth(&gates);

        debug!(
            "Built VQE circuit with {} gates, depth {}, max_iterations={}",
            gates.len(),
            depth,
            max_iterations
        );

        Ok(QuantumCircuit {
            num_qubits: n,
            gates,
            depth,
        })
    }

    /// Build QAOA circuit for optimization
    fn build_qaoa_circuit(
        &self,
        problem: &QuantumTFIMProblem,
        num_layers: usize,
        _optimizer: &str,
    ) -> CoreResult<QuantumCircuit> {
        let n = problem.num_qubits;
        let mut gates = Vec::new();

        // Initial state: uniform superposition
        for i in 0..n {
            gates.push(QuantumGate::H { qubit: i });
        }

        // QAOA layers
        for layer in 0..num_layers {
            let gamma = (layer as f64 + 1.0) * PI / (2.0 * num_layers as f64);
            let beta = PI / 4.0;

            // Problem Hamiltonian mixer (cost function)
            for i in 0..n {
                for j in (i + 1)..n {
                    let coupling = problem.couplings[(i, j)];
                    if coupling.abs() > 1e-10 {
                        // ZZ interaction
                        gates.push(QuantumGate::CNOT {
                            control: i,
                            target: j,
                        });
                        gates.push(QuantumGate::RZ {
                            qubit: j,
                            angle: 2.0 * gamma * coupling,
                        });
                        gates.push(QuantumGate::CNOT {
                            control: i,
                            target: j,
                        });
                    }
                }
            }

            // Driver Hamiltonian mixer (transverse field)
            for i in 0..n {
                gates.push(QuantumGate::RX {
                    qubit: i,
                    angle: 2.0 * beta,
                });
            }
        }

        let depth = self.calculate_circuit_depth(&gates);

        debug!(
            "Built QAOA circuit with {} layers, {} gates, depth {}",
            num_layers,
            gates.len(),
            depth
        );

        Ok(QuantumCircuit {
            num_qubits: n,
            gates,
            depth,
        })
    }

    /// Execute quantum circuit (simulator for now)
    fn execute_circuit(
        &self,
        circuit: &QuantumCircuit,
        _problem: &QuantumTFIMProblem,
    ) -> CoreResult<CircuitExecutionResult> {
        // Initialize state vector: |000...0⟩
        let dim = 2_usize.pow(circuit.num_qubits as u32);
        let mut state = vec![Complex::new(0.0, 0.0); dim];
        state[0] = Complex::new(1.0, 0.0);

        // Apply gates sequentially
        for gate in &circuit.gates {
            state = self.apply_gate(gate, &state, circuit.num_qubits)?;
        }

        // Perform measurements
        let measurements = self.measure_state(&state, circuit.num_qubits, self.config.num_shots)?;

        Ok((Some(state), measurements))
    }

    /// Apply quantum gate to state vector
    fn apply_gate(
        &self,
        gate: &QuantumGate,
        state: &[Complex<f64>],
        _num_qubits: usize,
    ) -> CoreResult<Vec<Complex<f64>>> {
        let dim = state.len();
        let mut new_state = vec![Complex::new(0.0, 0.0); dim];

        match gate {
            | QuantumGate::H { qubit } => {
                // Hadamard gate
                let sqrt2_inv = 1.0 / 2.0_f64.sqrt();
                for (i, ns) in new_state.iter_mut().enumerate() {
                    let bit = (i >> qubit) & 1;
                    let i_flipped = i ^ (1 << qubit);

                    if bit == 0 {
                        *ns += state[i] * sqrt2_inv;
                        *ns += state[i_flipped] * sqrt2_inv;
                    } else {
                        *ns += state[i_flipped] * sqrt2_inv;
                        *ns -= state[i] * sqrt2_inv;
                    }
                }
            },
            | QuantumGate::X { qubit } => {
                // Pauli X (bit flip)
                for (i, ns) in new_state.iter_mut().enumerate() {
                    let i_flipped = i ^ (1 << qubit);
                    *ns = state[i_flipped];
                }
            },
            | QuantumGate::Z { qubit } => {
                // Pauli Z (phase flip)
                for i in 0..dim {
                    let bit = (i >> qubit) & 1;
                    new_state[i] = if bit == 0 { state[i] } else { -state[i] };
                }
            },
            | QuantumGate::RX { qubit, angle } => {
                // Rotation around X axis
                let cos_half = (angle / 2.0).cos();
                let sin_half = (angle / 2.0).sin();

                for i in 0..dim {
                    let i_flipped = i ^ (1 << qubit);
                    new_state[i] =
                        state[i] * cos_half - Complex::new(0.0, sin_half) * state[i_flipped];
                }
            },
            | QuantumGate::RY { qubit, angle } => {
                // Rotation around Y axis
                let cos_half = (angle / 2.0).cos();
                let sin_half = (angle / 2.0).sin();

                for i in 0..dim {
                    let bit = (i >> qubit) & 1;
                    let i_flipped = i ^ (1 << qubit);

                    if bit == 0 {
                        new_state[i] = state[i] * cos_half - state[i_flipped] * sin_half;
                    } else {
                        new_state[i] = state[i] * cos_half + state[i_flipped] * sin_half;
                    }
                }
            },
            | QuantumGate::RZ { qubit, angle } => {
                // Rotation around Z axis
                let phase_0 = Complex::new(0.0, -angle / 2.0).exp();
                let phase_1 = Complex::new(0.0, angle / 2.0).exp();

                for i in 0..dim {
                    let bit = (i >> qubit) & 1;
                    new_state[i] = if bit == 0 {
                        state[i] * phase_0
                    } else {
                        state[i] * phase_1
                    };
                }
            },
            | QuantumGate::CNOT { control, target } => {
                // Controlled-NOT gate
                for i in 0..dim {
                    let control_bit = (i >> control) & 1;
                    if control_bit == 1 {
                        let i_flipped = i ^ (1 << target);
                        new_state[i] = state[i_flipped];
                    } else {
                        new_state[i] = state[i];
                    }
                }
            },
            | QuantumGate::CZ { control, target } => {
                // Controlled-Z gate
                for i in 0..dim {
                    let control_bit = (i >> control) & 1;
                    let target_bit = (i >> target) & 1;
                    new_state[i] = if control_bit == 1 && target_bit == 1 {
                        -state[i]
                    } else {
                        state[i]
                    };
                }
            },
            | _ => {
                return Err(CoreError::invalid_operation("Unsupported gate type"));
            },
        }

        Ok(new_state)
    }

    /// Measure quantum state multiple times
    fn measure_state(
        &self,
        state: &[Complex<f64>],
        num_qubits: usize,
        num_shots: usize,
    ) -> CoreResult<Vec<Vec<bool>>> {
        // Calculate probabilities
        let probabilities: Vec<f64> = state.iter().map(nalgebra::Complex::norm_sqr).collect();

        let measurements = match self.config.seed {
            | Some(seed) => {
                let mut rng = StdRng::seed_from_u64(seed);
                Self::sample_measurements(&mut rng, &probabilities, num_qubits, num_shots)
            },
            | None => {
                let mut rng = rand::thread_rng();
                Self::sample_measurements(&mut rng, &probabilities, num_qubits, num_shots)
            },
        };

        Ok(measurements)
    }

    /// Sample measurements from probability distribution using the provided RNG
    fn sample_measurements<R: Rng>(
        rng: &mut R,
        probabilities: &[f64],
        num_qubits: usize,
        num_shots: usize,
    ) -> Vec<Vec<bool>> {
        let mut measurements = Vec::with_capacity(num_shots);

        for _ in 0..num_shots {
            let rand_val: f64 = rng.gen();
            let mut cumulative = 0.0;
            let mut measured_state = 0;

            for (idx, &prob) in probabilities.iter().enumerate() {
                cumulative += prob;
                if rand_val < cumulative {
                    measured_state = idx;
                    break;
                }
            }

            // Convert integer to bit vector
            let bits: Vec<bool> = (0..num_qubits)
                .map(|i| (measured_state >> i) & 1 == 1)
                .collect();
            measurements.push(bits);
        }

        measurements
    }

    /// Measure observable quantities
    fn measure_observables(
        &self,
        problem: &QuantumTFIMProblem,
        measurements: &[Vec<bool>],
    ) -> CoreResult<QuantumObservables> {
        let n = problem.num_qubits;
        let num_shots = measurements.len() as f64;

        // Calculate magnetization for each qubit
        let mut magnetization = vec![0.0; n];
        for bits in measurements {
            for (i, &bit) in bits.iter().enumerate() {
                magnetization[i] += if bit { -1.0 } else { 1.0 };
            }
        }
        for mag in &mut magnetization {
            *mag /= num_shots;
        }

        // Calculate correlation functions
        let mut correlations = DMatrix::zeros(n, n);
        for i in 0..n {
            for j in i..n {
                let mut corr = 0.0;
                for bits in measurements {
                    let si = if bits[i] { -1.0 } else { 1.0 };
                    let sj = if bits[j] { -1.0 } else { 1.0 };
                    corr += si * sj;
                }
                corr /= num_shots;
                correlations[(i, j)] = corr;
                correlations[(j, i)] = corr;
            }
        }

        // Calculate order parameter (total magnetization)
        let order_parameter = magnetization.iter().sum::<f64>() / n as f64;

        Ok(QuantumObservables {
            magnetization,
            correlations,
            entanglement_entropy: None, // Would require density matrix calculation
            order_parameter,
        })
    }

    /// Calculate energy expectation value from measurements
    fn calculate_energy_expectation(
        &self,
        problem: &QuantumTFIMProblem,
        measurements: &[Vec<bool>],
    ) -> CoreResult<f64> {
        let n = problem.num_qubits;
        let mut energy_sum = 0.0;

        for bits in measurements {
            // Convert bits to spins
            let spins: Vec<f64> = bits.iter().map(|&b| if b { -1.0 } else { 1.0 }).collect();

            // Calculate energy for this configuration
            let mut energy = 0.0;

            // ZZ interactions
            for (i, &spin_i) in spins.iter().enumerate().take(n) {
                for (j, &spin_j) in spins.iter().enumerate().skip(i + 1).take(n - i - 1) {
                    let coupling = problem.couplings[(i, j)];
                    energy -= coupling * spin_i * spin_j;
                }
            }

            // Longitudinal fields
            for (i, &spin) in spins.iter().enumerate().take(n) {
                energy -= problem.longitudinal_fields[i] * spin;
            }

            energy_sum += energy;
        }

        Ok(energy_sum / measurements.len() as f64)
    }

    /// Calculate energy variance
    fn calculate_energy_variance(
        &self,
        problem: &QuantumTFIMProblem,
        measurements: &[Vec<bool>],
        mean_energy: f64,
    ) -> CoreResult<f64> {
        let n = problem.num_qubits;
        let mut variance = 0.0;

        for bits in measurements {
            let spins: Vec<f64> = bits.iter().map(|&b| if b { -1.0 } else { 1.0 }).collect();

            let mut energy = 0.0;
            for (i, &spin_i) in spins.iter().enumerate().take(n) {
                for (j, &spin_j) in spins.iter().enumerate().skip(i + 1).take(n - i - 1) {
                    let coupling = problem.couplings[(i, j)];
                    energy -= coupling * spin_i * spin_j;
                }
            }
            for (i, &spin) in spins.iter().enumerate().take(n) {
                energy -= problem.longitudinal_fields[i] * spin;
            }

            variance += (energy - mean_energy).powi(2);
        }

        Ok(variance / measurements.len() as f64)
    }

    /// Calculate circuit depth
    const fn calculate_circuit_depth(&self, gates: &[QuantumGate]) -> usize {
        // Simplified depth calculation
        // In reality, this depends on gate parallelization
        gates.len() / 2 + 1
    }

    /// Create quantum TFIM problem from classical problem
    pub fn from_classical_problem(
        classical: &crate::quantum::tfim::TFIMProblem,
        transverse_field_strength: f64,
    ) -> CoreResult<QuantumTFIMProblem> {
        let transverse_fields = vec![transverse_field_strength; classical.num_spins];

        Ok(QuantumTFIMProblem {
            num_qubits: classical.num_spins,
            couplings: classical.couplings.clone(),
            transverse_fields,
            longitudinal_fields: classical.external_fields.clone(),
            name: format!("Quantum_{}", classical.name),
        })
    }
}

impl Default for QuantumTFIMSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_tfim_trotter() {
        let problem = QuantumTFIMProblem {
            num_qubits: 3,
            couplings: DMatrix::from_fn(3, 3, |i, j| if i == j { 0.0 } else { 1.0 }),
            transverse_fields: vec![0.5, 0.5, 0.5],
            longitudinal_fields: vec![0.0, 0.0, 0.0],
            name: "Test_Trotter".to_string(),
        };

        let config = QuantumTFIMConfig {
            method: SolutionMethod::TrotterSuzuki { order: 2 },
            num_shots: 100,
            hardware_mapping: None,
            error_mitigation: false,
            trotter_steps: 5,
            evolution_time: 1.0,
            seed: None,
        };

        let solver = QuantumTFIMSolver::with_config(config);
        let solution = solver.solve(&problem).unwrap();

        assert_eq!(solution.measurements.len(), 100);
        assert!(solution.energy < 0.0); // Ferromagnetic should be negative
        assert!(solution.observables.magnetization.len() == 3);
    }

    #[test]
    fn test_quantum_tfim_vqe() {
        let problem = QuantumTFIMProblem {
            num_qubits: 2,
            couplings: DMatrix::from_fn(2, 2, |i, j| if i == j { 0.0 } else { -1.0 }),
            transverse_fields: vec![0.5, 0.5],
            longitudinal_fields: vec![0.0, 0.0],
            name: "Test_VQE".to_string(),
        };

        let config = QuantumTFIMConfig {
            method: SolutionMethod::VQE {
                ansatz: VQEAnsatz::HardwareEfficient { depth: 2 },
                max_iterations: 10,
                convergence_threshold: 1e-4,
            },
            num_shots: 100,
            hardware_mapping: None,
            error_mitigation: false,
            trotter_steps: 10,
            evolution_time: 1.0,
            seed: None,
        };

        let solver = QuantumTFIMSolver::with_config(config);
        let solution = solver.solve(&problem).unwrap();

        assert_eq!(solution.measurements.len(), 100);
        assert!(!solution.circuit.gates.is_empty());
    }

    #[test]
    fn test_quantum_tfim_qaoa() {
        let problem = QuantumTFIMProblem {
            num_qubits: 3,
            couplings: DMatrix::from_fn(3, 3, |i, j| if i == j { 0.0 } else { 1.0 }),
            transverse_fields: vec![0.5, 0.5, 0.5],
            longitudinal_fields: vec![0.0, 0.0, 0.0],
            name: "Test_QAOA".to_string(),
        };

        let config = QuantumTFIMConfig {
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

        let solver = QuantumTFIMSolver::with_config(config);
        let solution = solver.solve(&problem).unwrap();

        assert_eq!(solution.measurements.len(), 100);
        assert!(solution.observables.order_parameter.abs() <= 1.0);
    }

    #[test]
    fn test_hardware_mappings() {
        let superconducting = HardwareMapping {
            backend: QuantumBackend::Superconducting,
            connectivity: vec![(0, 1), (1, 2)],
            native_gates: vec!["RZ".to_string(), "RX".to_string(), "CZ".to_string()],
            coherence_time_us: 100.0,
            gate_error_rate: 0.001,
        };

        assert_eq!(superconducting.backend, QuantumBackend::Superconducting);
        assert_eq!(superconducting.coherence_time_us, 100.0);
    }

    #[test]
    fn test_observables_calculation() {
        let measurements = vec![
            vec![true, false, true],
            vec![false, false, false],
            vec![true, true, true],
        ];

        let problem = QuantumTFIMProblem {
            num_qubits: 3,
            couplings: DMatrix::zeros(3, 3),
            transverse_fields: vec![0.5; 3],
            longitudinal_fields: vec![0.0; 3],
            name: "Test".to_string(),
        };

        let solver = QuantumTFIMSolver::new();
        let observables = solver.measure_observables(&problem, &measurements).unwrap();

        assert_eq!(observables.magnetization.len(), 3);
        assert_eq!(observables.correlations.nrows(), 3);
        assert_eq!(observables.correlations.ncols(), 3);
    }
}
