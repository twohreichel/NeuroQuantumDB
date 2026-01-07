# Quantum Processor

## Algorithms

| Algorithm | Module | Use Case |
|-----------|--------|----------|
| Grover's Search | `quantum_processor.rs` | Unstructured search |
| QUBO (Classical) | `quantum/qubo.rs` | Fast optimization |
| QUBO (Quantum) | `quantum/qubo_quantum.rs` | Real quantum optimization |
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

### Usage

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
