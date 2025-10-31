//! # Quantum Extensions Module
//!
//! Advanced quantum-inspired optimization algorithms including:
//! - QUBO (Quadratic Unconstrained Binary Optimization)
//! - TFIM (Transverse Field Ising Model)
//! - Parallel Tempering (Replica Exchange Monte Carlo)
//! - Legacy Grover's search and quantum annealing

// Legacy quantum algorithms (Grover's search, basic annealing)
pub mod legacy;

// New quantum extensions (Phase 3)
pub mod parallel_tempering;
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
pub use qubo::{QUBOConfig, QUBOProblem, QUBOSolution, QUBOSolver};
pub use tfim::{FieldSchedule, TFIMProblem, TFIMSolution, TFIMSolver, TransverseFieldConfig};
