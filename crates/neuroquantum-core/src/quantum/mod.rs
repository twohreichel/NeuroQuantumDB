//! # Quantum Extensions Module
//!
//! This module provides both quantum circuit implementations and quantum-inspired
//! classical algorithms for optimization and search.
//!
//! ## Quantum Circuit Implementation
//!
//! - **Grover's Search (grover.rs)**: Real quantum circuit implementation of
//!   Grover's algorithm with support for multiple quantum backends (IBM Quantum,
//!   IonQ, Rigetti, and local simulator). Achieves O(√N) search complexity.
//!
//! ## Classical Simulations
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
//! - **Legacy Grover's Search**: Classical state vector simulation of Grover's
//!   quantum search algorithm. Useful for validation but does NOT provide
//!   quantum speedup.
//!
//! ## Performance Notes
//!
//! The grover module provides real quantum circuit execution capability with
//! quadratic speedup when run on actual quantum hardware. The quantum-inspired
//! classical algorithms provide heuristic advantages for optimization problems.
//!
//! ## Use Cases
//!
//! - **Grover's Algorithm**: Fast database search, pattern matching, index lookups
//! - **QUBO/TFIM**: Combinatorial optimization (scheduling, routing)
//! - **Parallel Tempering**: Global optimization, escape from local minima

// Grover's algorithm with quantum circuit implementation
pub mod grover;

// Legacy quantum algorithms (classical simulation)
pub mod legacy;

// Quantum extensions (Phase 3)
pub mod parallel_tempering;
pub mod qubo;
pub mod tfim;

// Re-export Grover's algorithm quantum circuit types (new implementation)
pub use grover::{
    BackendConfig, BackendType, DiffusionOperator, GroverCircuit, GroverDatabaseSearch,
    GroverResult, OracleGenerator, QuantumCircuit, QuantumDatabaseSearch, QuantumGate,
};

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
