# Quantum Processor

## Algorithms

| Algorithm | Module | Use Case |
|-----------|--------|----------|
| Grover's Search | `quantum_processor.rs` | Unstructured search |
| QUBO (Classical) | `quantum/qubo.rs` | Fast optimization |
| QUBO (Quantum) | `quantum/qubo_quantum.rs` | Real quantum optimization |
| QUBO (Hardware) | `quantum/qubo_hardware_backends.rs` | Real quantum hardware integration |
| TFIM | `quantum/tfim.rs` | Ising simulation |
| Parallel Tempering | `quantum/parallel_tempering.rs` | Global optimization |

## Grover's Algorithm

Quadratic speedup for search: O(√N) vs O(N)

```rust
pub struct GroverSearch {
    pub fn search<F>(&self, oracle: F, n_qubits: usize) -> Option<usize>
    where
        F: Fn(usize) -> bool;
}
```

### Implementation

```
1. Initialize uniform superposition |ψ⟩ = H⊗n|0⟩
2. Repeat √N times:
   a. Apply oracle (mark target)
   b. Apply diffusion operator
3. Measure result
```

## QUBO Solver

Quadratic Unconstrained Binary Optimization with **real quantum backends**:

### Quantum Backends

| Backend | Description | Hardware |
|---------|-------------|----------|
| VQE | Variational Quantum Eigensolver | Gate-based (IBM Q, IonQ) |
| QAOA | Quantum Approximate Optimization | Gate-based |
| Quantum Annealing | Native QUBO solving | D-Wave |
| SQA | Simulated Quantum Annealing (PIMC) | Classical simulation |
| Classical Fallback | Simulated annealing | Classical |

### Real Quantum Hardware Backends

The `qubo_hardware_backends` module provides production-ready integration with real quantum hardware:

| Backend | Class | Hardware | Max Variables |
|---------|-------|----------|---------------|
| D-Wave Quantum Annealer | `DWaveQUBOSolver` | D-Wave Advantage | ~5000 |
| IBM Quantum QAOA | `IBMQUBOSolver` | IBM Quantum | ~100 |
| D-Wave Hybrid | `HybridQUBOSolver` | D-Wave Leap | 1,000,000+ |
| Classical Fallback | `SimulatedAnnealingQUBOSolver` | Classical | 100,000+ |

### Real Hardware Usage

```rust
use neuroquantum_core::quantum::{
    DWaveQUBOSolver, DWaveConfig, QUBOSolverBackend, QUBOProblem
};

// Create D-Wave solver (requires DWAVE_API_TOKEN env var)
let config = DWaveConfig {
    num_reads: 1000,
    annealing_time_us: 20,
    auto_scale: true,
    ..Default::default()
};
let solver = DWaveQUBOSolver::new(config);

// Solve QUBO problem on real quantum hardware
let solution = solver.solve(&problem).await?;
println!("Energy: {}", solution.energy);
```

### Unified Solver with Auto-Selection

```rust
use neuroquantum_core::quantum::{
    UnifiedQUBOSolver, UnifiedQUBOConfig
};

// Auto-detects available backends from environment
let solver = UnifiedQUBOSolver::from_env();

// Automatically selects best available backend
let solution = solver.solve(&problem).await?;
println!("Used backend: {:?}", solution.backend_used);
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `DWAVE_API_TOKEN` | D-Wave Leap API token |
| `DWAVE_SOLVER` | D-Wave solver name (optional) |
| `IBM_QUANTUM_TOKEN` | IBM Quantum Experience token |
| `IBM_QUANTUM_BACKEND` | IBM backend name (optional) |

### Usage (Simulation Backends)

```rust
use neuroquantum_core::quantum::{
    QuantumQuboSolver, QuantumQuboConfig, QuboQuantumBackend
};

// Create solver with quantum backend
let config = QuantumQuboConfig {
    backend: QuboQuantumBackend::SimulatedQuantumAnnealing,
    trotter_slices: 32,
    max_iterations: 500,
    auto_fallback: true,
    ..Default::default()
};

let solver = QuantumQuboSolver::with_config(config);
let solution = solver.solve(&q_matrix, "my-problem")?;

println!("Energy: {}", solution.energy);
println!("Backend: {:?}", solution.backend_used);
```

### QUBO to Ising Mapping

The solver automatically converts QUBO problems to Ising Hamiltonians:

- QUBO: $\min f(x) = x^T Q x$ where $x \in \{0,1\}^n$
- Ising: $\min H = \sum_{ij} J_{ij} s_i s_j + \sum_i h_i s_i$ where $s \in \{-1,+1\}^n$

Mapping: $x_i = (1 + s_i) / 2$

## Parallel Tempering

Multiple replicas at different temperatures:

```rust
pub struct ParallelTempering {
    temperatures: Vec<f64>,
    replicas: Vec<State>,
}

impl ParallelTempering {
    pub fn step(&mut self);
    pub fn best_solution(&self) -> &State;
}
```

## Performance Threshold

```rust
// Only use quantum search when beneficial
const MIN_QUANTUM_SEARCH_SPACE: usize = 4;
const MIN_QUANTUM_SPEEDUP: f32 = 1.01;
```
