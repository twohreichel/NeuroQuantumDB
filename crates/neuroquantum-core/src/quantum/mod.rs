//! # Quantum Extensions Module
//!
//! ## Implemented Algorithms
//!
//! - **QUBO (Quadratic Unconstrained Binary Optimization)**: Now with **real quantum
//!   implementations** including VQE, QAOA, and Simulated Quantum Annealing (SQA).
//!   Features automatic fallback to classical solver when quantum backends unavailable.
//!
//! - **TFIM (Transverse Field Ising Model)**: Classical simulation of quantum
//!   annealing dynamics. Implements the transverse field Ising Hamiltonian
//!   using Monte Carlo methods.
//!
//! - **Quantum Parallel Tempering**: Real quantum algorithms for parallel tempering
//!   including Path Integral Monte Carlo (PIMC), Quantum Monte Carlo (QMC),
//!   and Quantum Annealing with multi-temperature support.
//!
//! - **Grover's Search (Legacy)**: Classical state vector simulation of Grover's
//!   quantum search algorithm. Useful for validation but does NOT provide
//!   quantum speedup.
//!
//! ## Quantum Backends for QUBO
//!
//! The `qubo_quantum` module provides multiple quantum solving approaches:
//!
//! - **VQE (Variational Quantum Eigensolver)**: For gate-based quantum computers
//! - **QAOA (Quantum Approximate Optimization Algorithm)**: Variational algorithm
//!   specifically designed for combinatorial optimization
//! - **Quantum Annealing**: For D-Wave style quantum annealers
//! - **Simulated Quantum Annealing (SQA)**: Path Integral Monte Carlo simulation
//! - **Classical Fallback**: Simulated annealing when quantum unavailable
//!
//! ## Performance Notes
//!
//! These quantum algorithms provide advantages through:
//!
//! - Quantum superposition for exploring multiple solutions simultaneously
//! - Quantum tunneling for escaping local minima
//! - Entanglement for correlated variable updates
//! - Thermal quantum fluctuations for global optimization
//!
//! ## Use Cases
//!
//! - Database query optimization
//! - Combinatorial optimization (scheduling, routing)
//! - Graph problems (partitioning, coloring, max-cut)
//! - Machine learning hyperparameter tuning

// Legacy quantum algorithms (Grover's search, basic annealing)
pub mod legacy;

// Quantum extensions
pub mod quantum_parallel_tempering;
pub mod qubo_quantum;
pub mod tfim;

// Re-export legacy quantum types for backwards compatibility
pub use legacy::{
    GroverSearch, OptimizedIndex, QuantumConfig, QuantumError, QuantumProcessor,
    QuantumQueryResults, QuantumSearch, QuantumSearchResult, QuantumStatistics,
};

// Re-export new quantum extension types
pub use quantum_parallel_tempering::{
    create_quantum_ising_optimizer, IsingHamiltonian, QuantumBackend, QuantumParallelTempering,
    QuantumParallelTemperingConfig, QuantumParallelTemperingSolution, QuantumReplica, QuantumState,
    ThermodynamicObservables,
};

// QUBO exports (consolidated from qubo.rs into qubo_quantum.rs)
pub use qubo_quantum::{
    // Quantum solver and config
    AnnealingSchedule, ClassicalOptimizer, CloudQuantumBackend, IsingModel, MeasurementStats,
    QuantumHardwareBackend, QuantumQuboConfig, QuantumQuboSolution, QuantumQuboSolver,
    QuboQuantumBackend, VqeAnsatz,
    // Problem builders
    graph_coloring_problem, max_cut_problem, tsp_problem, QUBOProblem,
    // Legacy type aliases for backwards compatibility
    QUBOConfig, QUBOSolution, QUBOSolver,
};

pub use tfim::{FieldSchedule, TFIMProblem, TFIMSolution, TFIMSolver, TransverseFieldConfig};
