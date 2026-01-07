//! # Quantum Extensions Module
//!
//! # ⚠️ Classical Simulation Notice
//!
//! **All algorithms in this module are QUANTUM-INSPIRED CLASSICAL SIMULATIONS.**
//! They do NOT require or interface with quantum hardware. These are classical
//! algorithms that borrow concepts from quantum mechanics to solve optimization
//! problems efficiently on classical computers.
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
//! - **Parallel Tempering (Replica Exchange Monte Carlo)**: Classical MCMC
//!   algorithm with multiple temperature replicas. Inspired by quantum thermal
//!   fluctuations but fully classical.
//!
//! - **Grover's Search (Legacy)**: Classical state vector simulation of Grover's
//!   quantum search algorithm. Useful for validation but does NOT provide
//!   quantum speedup.
//!
//! ## Performance Notes
//!
//! These quantum-inspired algorithms often outperform naive classical approaches
//! for optimization problems, but they do NOT achieve true quantum speedup.
//! The advantage comes from:
//!
//! - Better exploration of solution space via "tunneling" heuristics
//! - Thermal fluctuation-inspired escapes from local minima
//! - Parallel replica exchanges for global optimization
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
pub mod parallel_tempering;
pub mod quantum_parallel_tempering;
pub mod qubo;
pub mod tfim;

// Re-export legacy quantum types for backwards compatibility
pub use legacy::{
    GroverSearch, OptimizedIndex, QuantumConfig, QuantumError, QuantumProcessor,
    QuantumQueryResults, QuantumSearch, QuantumSearchResult, QuantumStatistics,
};

// Re-export new quantum extension types
pub use parallel_tempering::{
    ising_energy_function, ParallelTempering, ParallelTemperingConfig, ParallelTemperingSolution,
    TemperatureDistribution,
};
pub use quantum_parallel_tempering::{
    create_quantum_ising_optimizer, IsingHamiltonian, QuantumBackend, QuantumParallelTempering,
    QuantumParallelTemperingConfig, QuantumParallelTemperingSolution, QuantumReplica, QuantumState,
    ThermodynamicObservables,
};
pub use qubo::{QUBOConfig, QUBOProblem, QUBOSolution, QUBOSolver};
pub use tfim::{FieldSchedule, TFIMProblem, TFIMSolution, TFIMSolver, TransverseFieldConfig};
