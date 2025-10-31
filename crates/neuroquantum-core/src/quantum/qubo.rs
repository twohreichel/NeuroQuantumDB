//! # QUBO (Quadratic Unconstrained Binary Optimization) Solver
//!
//! Implements quantum-inspired optimization for solving QUBO problems
//! commonly found in database query optimization, graph problems, and combinatorial tasks.

use crate::error::{CoreError, CoreResult};
use nalgebra::DMatrix;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

/// QUBO problem representation with Q matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QUBOProblem {
    /// Q matrix (upper triangular)
    pub q_matrix: DMatrix<f64>,
    /// Number of binary variables
    pub num_vars: usize,
    /// Problem name/description
    pub name: String,
}

/// QUBO solution with binary variable assignments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QUBOSolution {
    /// Binary variable assignments (0 or 1)
    pub variables: Vec<u8>,
    /// Objective function value
    pub energy: f64,
    /// Number of iterations to convergence
    pub iterations: usize,
    /// Solution quality (0.0 to 1.0)
    pub quality: f64,
    /// Computation time in milliseconds
    pub computation_time_ms: f64,
}

/// Configuration for QUBO solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QUBOConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Initial temperature for simulated annealing
    pub initial_temperature: f64,
    /// Cooling rate (0.0 to 1.0)
    pub cooling_rate: f64,
    /// Minimum temperature threshold
    pub min_temperature: f64,
    /// Number of samples to try at each temperature
    pub samples_per_temp: usize,
    /// Enable quantum tunneling enhancement
    pub quantum_tunneling: bool,
}

impl Default for QUBOConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10000,
            initial_temperature: 1000.0,
            cooling_rate: 0.95,
            min_temperature: 0.01,
            samples_per_temp: 10,
            quantum_tunneling: true,
        }
    }
}

/// QUBO Solver using quantum-inspired simulated annealing
pub struct QUBOSolver {
    config: QUBOConfig,
}

impl QUBOSolver {
    /// Create a new QUBO solver with default configuration
    pub fn new() -> Self {
        Self {
            config: QUBOConfig::default(),
        }
    }

    /// Create a new QUBO solver with custom configuration
    pub fn with_config(config: QUBOConfig) -> Self {
        Self { config }
    }

    /// Solve a QUBO problem using quantum-inspired annealing
    #[instrument(skip(self, problem))]
    pub fn solve(&self, problem: &QUBOProblem) -> CoreResult<QUBOSolution> {
        let start_time = std::time::Instant::now();

        if problem.num_vars == 0 {
            return Err(CoreError::invalid_operation("Empty QUBO problem"));
        }

        debug!(
            "Solving QUBO problem '{}' with {} variables",
            problem.name, problem.num_vars
        );

        let mut rng = rand::thread_rng();

        // Initialize with random solution
        let mut current_solution: Vec<u8> = (0..problem.num_vars)
            .map(|_| if rng.gen::<f64>() < 0.5 { 0 } else { 1 })
            .collect();

        let mut current_energy = self.evaluate_energy(problem, &current_solution);
        let mut best_solution = current_solution.clone();
        let mut best_energy = current_energy;

        let mut temperature = self.config.initial_temperature;
        let mut iterations = 0;

        // Simulated annealing with quantum tunneling
        while temperature > self.config.min_temperature && iterations < self.config.max_iterations {
            for _ in 0..self.config.samples_per_temp {
                iterations += 1;

                // Generate neighbor solution by flipping a random bit
                let mut new_solution = current_solution.clone();
                let flip_idx = rng.gen_range(0..problem.num_vars);
                new_solution[flip_idx] = 1 - new_solution[flip_idx];

                let new_energy = self.evaluate_energy(problem, &new_solution);
                let delta_e = new_energy - current_energy;

                // Acceptance criterion with quantum tunneling
                let accept_prob = if delta_e < 0.0 {
                    1.0
                } else {
                    let base_prob = (-delta_e / temperature).exp();
                    if self.config.quantum_tunneling {
                        base_prob * self.quantum_tunneling_factor(delta_e, temperature)
                    } else {
                        base_prob
                    }
                };

                if rng.gen::<f64>() < accept_prob {
                    current_solution = new_solution;
                    current_energy = new_energy;

                    if current_energy < best_energy {
                        best_solution = current_solution.clone();
                        best_energy = current_energy;
                    }
                }
            }

            // Cool down
            temperature *= self.config.cooling_rate;
        }

        let computation_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        // Calculate solution quality (normalized to 0-1 range)
        let quality = self.calculate_quality(problem, best_energy);

        info!(
            "QUBO solved: energy={:.4}, iterations={}, quality={:.2}%, time={:.2}ms",
            best_energy,
            iterations,
            quality * 100.0,
            computation_time_ms
        );

        Ok(QUBOSolution {
            variables: best_solution,
            energy: best_energy,
            iterations,
            quality,
            computation_time_ms,
        })
    }

    /// Evaluate the objective function E(x) = x^T Q x
    fn evaluate_energy(&self, problem: &QUBOProblem, solution: &[u8]) -> f64 {
        let mut energy = 0.0;
        let n = problem.num_vars;

        for i in 0..n {
            for j in 0..n {
                if solution[i] == 1 && solution[j] == 1 {
                    energy += problem.q_matrix[(i, j)];
                }
            }
        }

        energy
    }

    /// Quantum tunneling enhancement factor
    fn quantum_tunneling_factor(&self, delta_e: f64, temperature: f64) -> f64 {
        let tunneling_strength = 0.2;
        1.0 + tunneling_strength * (-delta_e / (2.0 * temperature)).exp()
    }

    /// Calculate solution quality (0.0 = worst, 1.0 = best)
    fn calculate_quality(&self, problem: &QUBOProblem, energy: f64) -> f64 {
        // For minimization problems, lower energy is better
        // Normalize based on problem size and matrix values
        let max_possible = problem.q_matrix.sum().abs();
        if max_possible < 1e-10 {
            return 1.0;
        }

        // Quality = 1 - (normalized_energy)
        let normalized = (energy / max_possible).abs().min(1.0);
        1.0 - normalized
    }

    /// Create Max-Cut QUBO problem from graph
    pub fn max_cut_problem(
        edges: &[(usize, usize, f64)],
        num_nodes: usize,
    ) -> CoreResult<QUBOProblem> {
        if num_nodes == 0 {
            return Err(CoreError::invalid_operation("Empty graph"));
        }

        let mut q_matrix = DMatrix::zeros(num_nodes, num_nodes);

        // Max-Cut QUBO formulation: maximize sum of weights for cut edges
        // E(x) = sum_{(i,j)} w_ij * (x_i - x_j)^2 = sum w_ij * (x_i + x_j - 2*x_i*x_j)
        for &(i, j, weight) in edges {
            if i >= num_nodes || j >= num_nodes {
                return Err(CoreError::invalid_operation("Invalid node index"));
            }

            // Diagonal terms
            q_matrix[(i, i)] += weight;
            q_matrix[(j, j)] += weight;

            // Off-diagonal terms (convert to minimization)
            q_matrix[(i, j)] -= 2.0 * weight;
            q_matrix[(j, i)] -= 2.0 * weight;
        }

        Ok(QUBOProblem {
            q_matrix,
            num_vars: num_nodes,
            name: "Max-Cut".to_string(),
        })
    }

    /// Create Graph Coloring QUBO problem
    pub fn graph_coloring_problem(
        edges: &[(usize, usize)],
        num_nodes: usize,
        num_colors: usize,
    ) -> CoreResult<QUBOProblem> {
        if num_nodes == 0 || num_colors == 0 {
            return Err(CoreError::invalid_operation(
                "Invalid graph coloring parameters",
            ));
        }

        let num_vars = num_nodes * num_colors;
        let mut q_matrix = DMatrix::zeros(num_vars, num_vars);

        let penalty = 10.0;

        // Constraint: each node must have exactly one color
        for node in 0..num_nodes {
            for c1 in 0..num_colors {
                for c2 in 0..num_colors {
                    let var1 = node * num_colors + c1;
                    let var2 = node * num_colors + c2;
                    if c1 == c2 {
                        q_matrix[(var1, var1)] -= penalty;
                    } else {
                        q_matrix[(var1, var2)] += penalty;
                    }
                }
            }
        }

        // Constraint: adjacent nodes must have different colors
        for &(i, j) in edges {
            if i >= num_nodes || j >= num_nodes {
                return Err(CoreError::invalid_operation("Invalid edge"));
            }

            for c in 0..num_colors {
                let var_i = i * num_colors + c;
                let var_j = j * num_colors + c;
                q_matrix[(var_i, var_j)] += penalty;
            }
        }

        Ok(QUBOProblem {
            q_matrix,
            num_vars,
            name: format!("Graph-Coloring-{}-colors", num_colors),
        })
    }

    /// Create TSP (Traveling Salesman Problem) QUBO
    pub fn tsp_problem(distance_matrix: &DMatrix<f64>) -> CoreResult<QUBOProblem> {
        let n = distance_matrix.nrows();
        if n == 0 || n != distance_matrix.ncols() {
            return Err(CoreError::invalid_operation("Invalid distance matrix"));
        }

        let num_vars = n * n;
        let mut q_matrix = DMatrix::zeros(num_vars, num_vars);

        let penalty = distance_matrix.max() * 2.0;

        // Objective: minimize total distance
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let dist = distance_matrix[(i, j)];
                    for t in 0..(n - 1) {
                        let var1 = i * n + t;
                        let var2 = j * n + (t + 1);
                        q_matrix[(var1, var2)] += dist;
                    }
                }
            }
        }

        // Constraint: each city visited exactly once
        for i in 0..n {
            for t1 in 0..n {
                for t2 in 0..n {
                    let var1 = i * n + t1;
                    let var2 = i * n + t2;
                    if t1 == t2 {
                        q_matrix[(var1, var1)] -= penalty;
                    } else {
                        q_matrix[(var1, var2)] += penalty;
                    }
                }
            }
        }

        // Constraint: each time step has exactly one city
        for t in 0..n {
            for i1 in 0..n {
                for i2 in 0..n {
                    let var1 = i1 * n + t;
                    let var2 = i2 * n + t;
                    if i1 == i2 {
                        q_matrix[(var1, var1)] -= penalty;
                    } else {
                        q_matrix[(var1, var2)] += penalty;
                    }
                }
            }
        }

        Ok(QUBOProblem {
            q_matrix,
            num_vars,
            name: format!("TSP-{}-cities", n),
        })
    }
}

impl Default for QUBOSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_qubo() {
        let mut q_matrix = DMatrix::zeros(3, 3);
        q_matrix[(0, 0)] = -1.0;
        q_matrix[(1, 1)] = -1.0;
        q_matrix[(2, 2)] = -1.0;
        q_matrix[(0, 1)] = 2.0;
        q_matrix[(1, 2)] = 2.0;

        let problem = QUBOProblem {
            q_matrix,
            num_vars: 3,
            name: "Simple".to_string(),
        };

        let solver = QUBOSolver::new();
        let solution = solver.solve(&problem).unwrap();

        assert_eq!(solution.variables.len(), 3);
        assert!(solution.energy < 0.0); // Should find negative energy
                                        // Quality calculation depends on problem structure, so we just check it's valid
        assert!(solution.quality >= 0.0 && solution.quality <= 1.0);
        assert!(solution.iterations > 0);
    }

    #[test]
    fn test_max_cut_problem() {
        let edges = vec![(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 0, 1.0)];

        let problem = QUBOSolver::max_cut_problem(&edges, 4).unwrap();
        assert_eq!(problem.num_vars, 4);
        assert_eq!(problem.name, "Max-Cut");

        let solver = QUBOSolver::new();
        let solution = solver.solve(&problem).unwrap();
        assert_eq!(solution.variables.len(), 4);
    }

    #[test]
    fn test_graph_coloring_problem() {
        let edges = vec![(0, 1), (1, 2), (2, 0)];
        let problem = QUBOSolver::graph_coloring_problem(&edges, 3, 3).unwrap();

        assert_eq!(problem.num_vars, 9); // 3 nodes * 3 colors

        let solver = QUBOSolver::new();
        let solution = solver.solve(&problem).unwrap();
        assert_eq!(solution.variables.len(), 9);
    }

    #[test]
    fn test_empty_problem() {
        let q_matrix = DMatrix::zeros(0, 0);
        let problem = QUBOProblem {
            q_matrix,
            num_vars: 0,
            name: "Empty".to_string(),
        };

        let solver = QUBOSolver::new();
        let result = solver.solve(&problem);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_config() {
        let config = QUBOConfig {
            max_iterations: 100,
            initial_temperature: 100.0,
            cooling_rate: 0.9,
            min_temperature: 0.1,
            samples_per_temp: 5,
            quantum_tunneling: false,
        };

        let solver = QUBOSolver::with_config(config);

        let mut q_matrix = DMatrix::zeros(2, 2);
        q_matrix[(0, 0)] = -1.0;
        q_matrix[(1, 1)] = -1.0;

        let problem = QUBOProblem {
            q_matrix,
            num_vars: 2,
            name: "Test".to_string(),
        };

        let solution = solver.solve(&problem).unwrap();
        assert_eq!(solution.variables.len(), 2);
    }
}
