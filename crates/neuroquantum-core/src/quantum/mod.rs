//! # Quantum Extensions Module
//!
//! ## Implemented Algorithms
//!
//! - **QUBO (Quadratic Unconstrained Binary Optimization)**: Classical simulated
//!   annealing solver for binary optimization problems. Uses quantum-inspired
//!   tunneling heuristics but runs entirely on classical hardware.
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
//! ## Performance Notes
//!
//! These quantum-inspired algorithms often outperform naive classical approaches
//! for optimization problems. The advantage comes from:
//!
//! - Better exploration of solution space via "tunneling" heuristics
//! - Thermal fluctuation-inspired escapes from local minima
//! - Parallel replica exchanges for global optimization
//! - Quantum thermal state preparation (PIMC/QMC)
//!
//! ## Use Cases
//!
//! - Database query optimization
//! - Combinatorial optimization (scheduling, routing)
//! - Graph problems (partitioning, coloring)
//! - Machine learning hyperparameter tuning

// Legacy quantum algorithms (Grover's search, basic annealing)
pub mod legacy;

// New quantum extensions (Phase 3)
pub mod quantum_parallel_tempering;
pub mod qubo;
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
pub use qubo::{QUBOConfig, QUBOProblem, QUBOSolution, QUBOSolver};
pub use tfim::{FieldSchedule, TFIMProblem, TFIMSolution, TFIMSolver, TransverseFieldConfig};
