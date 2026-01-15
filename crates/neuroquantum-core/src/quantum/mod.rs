//! # Quantum Extensions Module
//!
//! ## Implemented Algorithms
//!
//! - **QUBO (Quadratic Unconstrained Binary Optimization)**: Now with **real quantum
//!   implementations** including VQE, QAOA, and Simulated Quantum Annealing (SQA).
//!   Features automatic fallback to classical solver when quantum backends unavailable.
//!
//! - **TFIM (Transverse Field Ising Model)**: Now with **real quantum hardware integration**:
//!   - **Quantum Annealing Backends**: D-Wave and AWS Braket quantum annealers for native
//!     Ising model solving (NEW!)
//!   - **Gate-Based Quantum**: Trotter-Suzuki time evolution, VQE for ground state
//!     finding, and QAOA for optimization
//!   - **Classical Fallback**: Monte Carlo simulation when quantum hardware unavailable
//!
//! - **Quantum Parallel Tempering**: Real quantum algorithms for parallel tempering
//!   including Path Integral Monte Carlo (PIMC), Quantum Monte Carlo (QMC),
//!   and Quantum Annealing with multi-temperature support. Now with **real quantum
//!   hardware integration**:
//!   - **IBM Quantum**: QITE (Quantum Imaginary Time Evolution) for thermal state preparation
//!   - **AWS Braket**: Multi-vendor access for thermal sampling
//!   - **D-Wave**: Native quantum annealing with reverse annealing for temperature control
//!   - **`IonQ`**: High-fidelity trapped-ion thermal state preparation
//!   - **Local Simulator**: Classical fallback (always available)
//!
//! - **Grover's Search**: Now with **real quantum hardware integration** supporting:
//!   - **IBM Quantum**: Execute on IBM gate-based quantum computers via Qiskit Runtime
//!   - **AWS Braket**: Execute on `IonQ`, Rigetti, and OQC devices via AWS Braket
//!   - **`IonQ` Direct**: Direct API access to `IonQ` trapped-ion quantum computers
//!   - **Local Simulator**: State vector simulation for development and testing
//!   
//!   The `UnifiedGroverSolver` provides automatic backend selection with fallback
//!   to local simulation when quantum hardware is unavailable.
//!
//! - **Grover's Search (Legacy)**: Classical state vector simulation of Grover's
//!   quantum search algorithm. Useful for validation but does NOT provide
//!   quantum speedup on classical hardware.
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
//! ## Quantum Backends for TFIM (NEW!)
//!
//! The `tfim_hardware_backends` module provides real quantum annealing hardware:
//!
//! - **D-Wave Quantum Annealer**: Native Ising model solving on D-Wave quantum annealers
//!   - Direct problem embedding on quantum hardware
//!   - Supports Advantage and Advantage2 systems with Pegasus/Zephyr topology
//!   - Up to ~5000 qubits
//! - **AWS Braket Annealer**: D-Wave access via AWS Braket
//!   - Integrated AWS billing and resource management
//!   - Same underlying quantum annealing technology
//! - **Unified Solver**: Automatic backend selection with classical fallback
//!
//! ### TFIM Configuration Example
//!
//! ```rust,ignore
//! use neuroquantum_core::quantum::{UnifiedTFIMAnnealingSolver, TFIMProblem};
//!
//! // Auto-detect available backends from environment
//! let solver = UnifiedTFIMAnnealingSolver::from_env();
//!
//! // Create TFIM problem
//! let problem = TFIMProblem { /* ... */ };
//!
//! // Execute on best available backend (D-Wave, Braket, or classical)
//! let result = solver.solve(&problem).await?;
//! ```
//!
//! ## Quantum Backends for Grover's Search
//!
//! The `grover_hardware_backends` module provides real quantum hardware integration:
//!
//! - **IBM Quantum**: Gate-based superconducting qubits via IBM Quantum Experience
//! - **AWS Braket**: Multi-vendor access (`IonQ`, Rigetti, OQC) via AWS
//! - **`IonQ` Direct**: Native trapped-ion quantum computing API
//! - **Local Simulator**: State vector simulation (always available)
//!
//! ### Configuration Example
//!
//! ```rust,ignore
//! use neuroquantum_core::quantum::{UnifiedGroverSolver, QuantumOracle};
//!
//! // Auto-detect available backends from environment
//! let solver = UnifiedGroverSolver::from_env();
//!
//! // Create search oracle
//! let oracle = QuantumOracle::new(4, vec![7, 12]); // 16-element search for items 7 and 12
//!
//! // Execute on best available backend
//! let result = solver.search(&oracle, 1024).await?;
//! ```
//!
//! ## Quantum Backends for Parallel Tempering (NEW!)
//!
//! The `parallel_tempering_hardware_backends` module provides real quantum hardware integration
//! for Parallel Tempering optimization:
//!
//! - **IBM Quantum**: QITE (Quantum Imaginary Time Evolution) for thermal state preparation
//!   - Trotter-based thermal state preparation at multiple temperatures
//!   - Supports all IBM Quantum Experience backends
//! - **D-Wave Quantum Annealer**: Native quantum annealing with temperature control
//!   - Reverse annealing for thermal state preparation
//!   - Multi-temperature annealing schedules
//! - **AWS Braket**: Multi-vendor quantum access including D-Wave annealers
//! - **`IonQ`**: High-fidelity trapped-ion thermal state preparation
//! - **Local Simulator**: Classical PIMC/QMC fallback (always available)
//!
//! ### Parallel Tempering Configuration Example
//!
//! ```rust,ignore
//! use neuroquantum_core::quantum::{
//!     UnifiedPTSolver, IsingHamiltonian, QuantumParallelTemperingConfig
//! };
//!
//! // Auto-detect available backends from environment
//! let solver = UnifiedPTSolver::from_env();
//!
//! // Create Ising Hamiltonian
//! let hamiltonian = IsingHamiltonian::new(num_spins, couplings, fields, transverse_field);
//!
//! // Configure parallel tempering
//! let config = QuantumParallelTemperingConfig {
//!     num_replicas: 8,
//!     min_temperature: 0.1,
//!     max_temperature: 10.0,
//!     ..Default::default()
//! };
//!
//! // Execute on best available backend (D-Wave, IBM, Braket, IonQ, or classical)
//! let result = solver.optimize(&hamiltonian, &initial_config, &config).await?;
//! ```
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

// Unified quantum hardware backends (NEW!)
// Provides consolidated access to IBM Quantum, AWS Braket, D-Wave, and IonQ
pub mod backends;

// Legacy quantum algorithms (Grover's search, basic annealing)
pub mod legacy;

// Real quantum Grover's search algorithm
pub mod grover_quantum;

// Real quantum hardware backends for Grover's search
pub mod grover_hardware_backends;

// Quantum extensions
pub mod parallel_tempering_hardware_backends;
pub mod quantum_parallel_tempering;
pub mod qubo_hardware_backends;
pub mod qubo_quantum;
pub mod tfim;
pub mod tfim_hardware_backends;
pub mod tfim_quantum;
pub mod tfim_unified;

// Re-export legacy quantum types for backwards compatibility
// Re-export AWS Braket backend (new provider-centric implementation)
pub use backends::braket::{
    BraketAnnealingProblem, BraketBackend, BraketConfig as UnifiedBraketConfig, BraketDeviceType,
    BraketGate, BraketProblemType, BraketTaskInfo, BraketTaskStatus,
};
// Re-export D-Wave backend (new provider-centric implementation)
// Use aliases to avoid conflicts with qubo_hardware_backends exports
pub use backends::dwave::{
    DWaveBackend, DWaveConfig as UnifiedDWaveConfig, DWavePostprocessing, DWaveProblem,
    DWaveSample, DWaveSampleSet, DWaveSolverInfo, DWaveTiming as UnifiedDWaveTiming, DWaveTopology,
};
// Re-export IBM Quantum backend (new provider-centric implementation)
pub use backends::ibm::{
    IBMJobInfo, IBMJobStatus, IBMProcessorFamily, IBMQuantumBackend, IBMQuantumConfig,
};
// Re-export IonQ backend (new provider-centric implementation)
pub use backends::ionq::{
    IonQBackend, IonQCircuit, IonQConfig as UnifiedIonQConfig, IonQDeviceSpec, IonQGate,
    IonQJobStatus, IonQMetadata, IonQResult, IonQTarget,
};
// Unified quantum hardware backends (NEW!)
// These provide a reorganized, provider-centric view of quantum backends
// The module is exported for direct use; common types are re-exported with prefixes
// to avoid conflicts with the algorithm-specific hardware backends above
pub use backends::{
    // Common types (no conflicts)
    QuantumBackendConfig as UnifiedBackendConfig,
    QuantumBackendFactory,
    QuantumBackendInfo,
    QuantumExecutionResult,
    QuantumProvider,
};
// Real quantum hardware backends for Grover's search
pub use grover_hardware_backends::{
    // AWS Braket
    BraketGroverConfig,
    BraketGroverSolver,
    // Backend trait
    GroverHardwareBackend,
    // IBM Quantum
    IBMGroverConfig,
    IBMGroverSolver,
    // IonQ
    IonQGroverConfig,
    IonQGroverSolver,
    // Local simulator
    SimulatorGroverConfig,
    SimulatorGroverSolver,
    // Unified solver with auto-selection
    UnifiedGroverConfig,
    UnifiedGroverSolver,
};
// Real quantum Grover's search exports
pub use grover_quantum::{
    GroverCircuit, GroverGate, GroverMeasurementStats, GroverQuantumBackend, OracleType,
    QuantumGroverConfig, QuantumGroverResult, QuantumGroverSolver, QuantumOracle,
};
pub use legacy::{
    GroverSearch, OptimizedIndex, QuantumConfig, QuantumError, QuantumProcessor,
    QuantumQueryResults, QuantumSearch, QuantumSearchResult, QuantumStatistics,
};
// Real quantum hardware backends for Parallel Tempering
pub use parallel_tempering_hardware_backends::{
    BraketPTConfig,
    // AWS Braket
    BraketParallelTemperingSolver,
    DWavePTConfig,
    // D-Wave
    DWaveParallelTemperingSolver,
    IBMPTConfig,
    // IBM Quantum
    IBMParallelTemperingSolver,
    IonQPTConfig,
    // IonQ
    IonQParallelTemperingSolver,
    // Backend trait
    PTBackendType,
    PTHardwareBackend,
    PTMeasurementResult,
    PTQuantumGate,
    QITECircuit,
    SimulatorPTConfig,
    // Local simulator
    SimulatorParallelTemperingSolver,
    // Unified solver with auto-selection
    UnifiedPTConfig,
    UnifiedPTSolver,
};
// Re-export new quantum extension types
pub use quantum_parallel_tempering::{
    create_quantum_ising_optimizer, IsingHamiltonian, QuantumBackend, QuantumParallelTempering,
    QuantumParallelTemperingConfig, QuantumParallelTemperingSolution, QuantumReplica, QuantumState,
    ThermodynamicObservables,
};
// Real quantum hardware backends for QUBO
pub use qubo_hardware_backends::{
    // D-Wave quantum annealer
    DWaveConfig,
    DWaveQUBOSolver,
    DWaveTiming,
    // D-Wave Hybrid solver
    HybridQUBOSolver,
    HybridSolverConfig,
    // IBM Quantum QAOA
    IBMConfig,
    IBMOptimizer,
    IBMQUBOSolver,
    QAOACircuit,
    QAOAGate,
    // Unified solver with auto-selection
    QUBOSolverBackend,
    // Classical fallback
    SimulatedAnnealingConfig,
    SimulatedAnnealingQUBOSolver,
    UnifiedQUBOConfig,
    UnifiedQUBOSolver,
};
// QUBO exports (consolidated from qubo.rs into qubo_quantum.rs)
pub use qubo_quantum::{
    // Problem builders
    graph_coloring_problem,
    max_cut_problem,
    tsp_problem,
    // Quantum solver and config
    AnnealingSchedule,
    ClassicalOptimizer,
    CloudQuantumBackend,
    IsingModel,
    MeasurementStats,
    // Legacy type aliases for backwards compatibility
    QUBOConfig,
    QUBOProblem,
    QUBOSolution,
    QUBOSolver,
    QuantumHardwareBackend,
    QuantumQuboConfig,
    QuantumQuboSolution,
    QuantumQuboSolver,
    QuboQuantumBackend,
    VqeAnsatz,
};
pub use tfim::{FieldSchedule, TFIMProblem, TFIMSolution, TFIMSolver, TransverseFieldConfig};
// Real quantum hardware backends for TFIM (Quantum Annealing)
pub use tfim_hardware_backends::{
    // Backend trait
    AnnealingBackend,
    // Binary Quadratic Model
    BinaryQuadraticModel,
    // AWS Braket annealer
    BraketTFIMConfig,
    BraketTFIMSolver,
    // D-Wave quantum annealer
    DWaveTFIMConfig,
    DWaveTFIMSolver,
    // Unified solver with auto-selection
    TFIMBackendPreference,
    UnifiedTFIMAnnealingConfig,
    UnifiedTFIMAnnealingSolver,
    VarType,
};
// Real quantum TFIM exports (VQE, QAOA, Trotter-Suzuki)
pub use tfim_quantum::{
    HardwareMapping, QuantumBackend as TFIMQuantumBackend, QuantumCircuit, QuantumGate,
    QuantumObservables, QuantumTFIMConfig, QuantumTFIMProblem, QuantumTFIMSolution,
    QuantumTFIMSolver, SolutionMethod, VQEAnsatz,
};
// Unified TFIM interface (automatic quantum/classical selection)
pub use tfim_unified::{UnifiedTFIMConfig, UnifiedTFIMResult, UnifiedTFIMSolver};
