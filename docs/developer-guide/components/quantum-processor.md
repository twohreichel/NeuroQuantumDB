# Quantum Processor

## Algorithms

| Algorithm | Module | Use Case |
|-----------|--------|----------|
| Grover's Search | `quantum_processor.rs` | Unstructured search |
| QUBO | `quantum/qubo.rs` | Optimization |
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

Quadratic Unconstrained Binary Optimization:

```rust
pub struct QuboSolver {
    /// Q matrix coefficients
    pub fn add_linear(&mut self, i: usize, coeff: f64);
    pub fn add_quadratic(&mut self, i: usize, j: usize, coeff: f64);
    
    /// Solve
    pub fn solve(&self) -> Vec<bool>;
}
```

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
