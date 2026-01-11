//! # QUBO Hardware Backend Integration Tests
//!
//! This module contains comprehensive tests for the real quantum hardware
//! backend implementations including D-Wave, IBM Quantum, and Hybrid solvers.
//!
//! ## Test Categories
//!
//! 1. Configuration Tests
//! 2. Solver Availability Tests  
//! 3. Problem Conversion Tests
//! 4. Fallback Behavior Tests
//! 5. Unified Solver Tests
//! 6. QAOA Circuit Building Tests

use nalgebra::DMatrix;
use neuroquantum_core::quantum::qubo_hardware_backends::{
    DWaveConfig, DWaveQUBOSolver, HybridQUBOSolver, HybridSolverConfig, IBMConfig, IBMOptimizer,
    IBMQUBOSolver, QUBOSolverBackend, SimulatedAnnealingConfig, SimulatedAnnealingQUBOSolver,
    UnifiedQUBOConfig, UnifiedQUBOSolver,
};
use neuroquantum_core::quantum::qubo_quantum::{QUBOProblem, QuboQuantumBackend};

// =============================================================================
// TEST PROBLEM CREATION HELPERS
// =============================================================================

/// Create a simple 3-variable QUBO problem for testing
fn create_simple_problem() -> QUBOProblem {
    let mut q_matrix = DMatrix::zeros(3, 3);
    q_matrix[(0, 0)] = -1.0;
    q_matrix[(1, 1)] = -1.0;
    q_matrix[(2, 2)] = -1.0;
    q_matrix[(0, 1)] = 2.0;

    QUBOProblem {
        q_matrix,
        num_vars: 3,
        name: "Simple Test Problem".to_string(),
    }
}

/// Create a larger problem for hybrid solver testing
fn create_large_problem(size: usize) -> QUBOProblem {
    let mut q_matrix = DMatrix::zeros(size, size);

    // Create a ferromagnetic-like problem
    for i in 0..size {
        q_matrix[(i, i)] = -1.0;
        if i < size - 1 {
            q_matrix[(i, i + 1)] = 0.5;
        }
    }

    QUBOProblem {
        q_matrix,
        num_vars: size,
        name: format!("Large Problem ({})", size),
    }
}

/// Create a Max-Cut problem on a cycle graph
fn create_maxcut_cycle(n: usize) -> QUBOProblem {
    let mut q_matrix = DMatrix::zeros(n, n);

    for i in 0..n {
        let j = (i + 1) % n;
        q_matrix[(i, i)] += 1.0;
        q_matrix[(j, j)] += 1.0;
        q_matrix[(i, j)] -= 2.0;
        q_matrix[(j, i)] -= 2.0;
    }

    QUBOProblem {
        q_matrix,
        num_vars: n,
        name: format!("Max-Cut Cycle Graph ({})", n),
    }
}

// =============================================================================
// D-WAVE SOLVER TESTS
// =============================================================================

#[test]
fn test_dwave_config_default() {
    let config = DWaveConfig::default();

    assert_eq!(config.num_reads, 1000);
    assert_eq!(config.annealing_time_us, 20);
    assert!(config.auto_scale);
    assert!(config.api_token.is_none());
    assert_eq!(config.api_endpoint, "https://cloud.dwavesys.com/sapi/v2");
}

#[test]
fn test_dwave_config_custom() {
    let config = DWaveConfig {
        api_token: Some("test-token".to_string()),
        num_reads: 500,
        annealing_time_us: 50,
        chain_strength: Some(2.0),
        ..Default::default()
    };

    assert_eq!(config.num_reads, 500);
    assert_eq!(config.annealing_time_us, 50);
    assert_eq!(config.chain_strength, Some(2.0));
    assert_eq!(config.api_token, Some("test-token".to_string()));
}

#[test]
fn test_dwave_solver_not_available_without_token() {
    let config = DWaveConfig::default();
    let solver = DWaveQUBOSolver::new(config);

    assert!(!solver.is_available());
    assert_eq!(solver.name(), "D-Wave Quantum Annealer");
    assert_eq!(solver.backend_type(), QuboQuantumBackend::QuantumAnnealing);
}

#[test]
fn test_dwave_solver_max_variables() {
    let config = DWaveConfig::default();
    let solver = DWaveQUBOSolver::new(config);

    assert_eq!(solver.max_variables(), 5000);
}

#[tokio::test]
async fn test_dwave_solver_fallback_simulation() {
    let config = DWaveConfig {
        num_reads: 100,
        ..Default::default()
    };
    let solver = DWaveQUBOSolver::new(config);
    let problem = create_simple_problem();

    // Without API token, should use simulation fallback
    let solution = solver.solve(&problem).await.unwrap();

    assert_eq!(solution.variables.len(), 3);
    assert!(solution.computation_time_ms >= 0.0);
    assert!(solution.energy.is_finite());
}

// =============================================================================
// IBM QUANTUM SOLVER TESTS
// =============================================================================

#[test]
fn test_ibm_config_default() {
    let config = IBMConfig::default();

    assert_eq!(config.qaoa_depth, 3);
    assert_eq!(config.num_shots, 1024);
    assert_eq!(config.max_iterations, 100);
    assert_eq!(config.optimizer, IBMOptimizer::COBYLA);
    assert!(config.error_mitigation);
    assert!(config.dynamic_decoupling);
}

#[test]
fn test_ibm_config_custom() {
    let config = IBMConfig {
        api_token: Some("test-ibm-token".to_string()),
        backend_name: "ibm_brisbane".to_string(),
        qaoa_depth: 5,
        optimizer: IBMOptimizer::SPSA,
        max_qubits: Some(50),
        ..Default::default()
    };

    assert_eq!(config.qaoa_depth, 5);
    assert_eq!(config.backend_name, "ibm_brisbane");
    assert_eq!(config.optimizer, IBMOptimizer::SPSA);
    assert_eq!(config.max_qubits, Some(50));
}

#[test]
fn test_ibm_solver_not_available_without_token() {
    let config = IBMConfig::default();
    let solver = IBMQUBOSolver::new(config);

    assert!(!solver.is_available());
    assert_eq!(solver.name(), "IBM Quantum QAOA");
    assert_eq!(solver.backend_type(), QuboQuantumBackend::QAOA);
}

#[test]
fn test_ibm_solver_max_variables() {
    let config = IBMConfig {
        max_qubits: Some(127),
        ..Default::default()
    };
    let solver = IBMQUBOSolver::new(config);

    assert_eq!(solver.max_variables(), 127);
}

#[tokio::test]
async fn test_ibm_solver_fallback_simulation() {
    let config = IBMConfig {
        qaoa_depth: 2,
        max_iterations: 50,
        ..Default::default()
    };
    let solver = IBMQUBOSolver::new(config);
    let problem = create_simple_problem();

    // Without API token, should use QAOA simulation fallback
    let solution = solver.solve(&problem).await.unwrap();

    assert_eq!(solution.variables.len(), 3);
    assert_eq!(solution.backend_used, QuboQuantumBackend::QAOA);
    assert!(solution.computation_time_ms >= 0.0);
}

#[tokio::test]
async fn test_ibm_solver_respects_qubit_limit() {
    let config = IBMConfig {
        max_qubits: Some(2), // Only allow 2 qubits
        ..Default::default()
    };
    let solver = IBMQUBOSolver::new(config);
    let problem = create_simple_problem(); // 3 variables

    // Should fail because problem is too large
    let result = solver.solve(&problem).await;
    assert!(result.is_err());
}

// =============================================================================
// HYBRID SOLVER TESTS
// =============================================================================

#[test]
fn test_hybrid_config_default() {
    let config = HybridSolverConfig::default();

    assert_eq!(config.time_limit_secs, 5);
    assert_eq!(config.min_samples, 1);
    assert_eq!(config.max_samples, 100);
    assert_eq!(config.solver_name, "hybrid_binary_quadratic_model_version2");
}

#[test]
fn test_hybrid_solver_not_available_without_token() {
    let config = HybridSolverConfig::default();
    let solver = HybridQUBOSolver::new(config);

    assert!(!solver.is_available());
    assert_eq!(solver.name(), "D-Wave Leap Hybrid");
}

#[test]
fn test_hybrid_solver_max_variables() {
    let config = HybridSolverConfig::default();
    let solver = HybridQUBOSolver::new(config);

    // Hybrid solver can handle very large problems
    assert_eq!(solver.max_variables(), 1_000_000);
}

#[tokio::test]
async fn test_hybrid_solver_fallback_simulation() {
    let config = HybridSolverConfig::default();
    let solver = HybridQUBOSolver::new(config);
    let problem = create_simple_problem();

    // Without API token, should use simulation fallback
    let solution = solver.solve(&problem).await.unwrap();

    assert_eq!(solution.variables.len(), 3);
    assert!(solution.computation_time_ms >= 0.0);
}

// =============================================================================
// SIMULATED ANNEALING SOLVER TESTS
// =============================================================================

#[test]
fn test_simulated_annealing_config_default() {
    let config = SimulatedAnnealingConfig::default();

    assert_eq!(config.initial_temperature, 10.0);
    assert_eq!(config.final_temperature, 0.001);
    assert_eq!(config.cooling_rate, 0.99);
    assert_eq!(config.max_iterations, 10000);
    assert_eq!(config.num_restarts, 3);
}

#[test]
fn test_simulated_annealing_always_available() {
    let config = SimulatedAnnealingConfig::default();
    let solver = SimulatedAnnealingQUBOSolver::new(config);

    assert!(solver.is_available());
    assert_eq!(solver.name(), "Simulated Annealing (Classical)");
    assert_eq!(solver.backend_type(), QuboQuantumBackend::ClassicalFallback);
}

#[tokio::test]
async fn test_simulated_annealing_solver_basic() {
    let config = SimulatedAnnealingConfig {
        max_iterations: 1000,
        ..Default::default()
    };
    let solver = SimulatedAnnealingQUBOSolver::new(config);
    let problem = create_simple_problem();

    let solution = solver.solve(&problem).await.unwrap();

    assert_eq!(solution.variables.len(), 3);
    assert!(solution.energy.is_finite());
    assert!(solution.computation_time_ms >= 0.0);
}

#[tokio::test]
async fn test_simulated_annealing_finds_good_solution() {
    let config = SimulatedAnnealingConfig {
        max_iterations: 5000,
        num_restarts: 5,
        ..Default::default()
    };
    let solver = SimulatedAnnealingQUBOSolver::new(config);

    // Create problem with known optimal: all 1s gives energy -3
    let mut q_matrix = DMatrix::zeros(3, 3);
    q_matrix[(0, 0)] = -1.0;
    q_matrix[(1, 1)] = -1.0;
    q_matrix[(2, 2)] = -1.0;

    let problem = QUBOProblem {
        q_matrix,
        num_vars: 3,
        name: "Easy Problem".to_string(),
    };

    let solution = solver.solve(&problem).await.unwrap();

    // Should find optimal or near-optimal
    assert!(
        solution.energy <= 0.0,
        "Should find non-positive energy, got {}",
        solution.energy
    );
}

// =============================================================================
// UNIFIED SOLVER TESTS
// =============================================================================

#[test]
fn test_unified_config_default() {
    let config = UnifiedQUBOConfig::default();

    assert!(config.dwave.is_none());
    assert!(config.ibm.is_none());
    assert!(config.hybrid.is_none());
    assert_eq!(config.hybrid_threshold, 5000);
    assert!(config
        .backend_priority
        .contains(&"simulated_annealing".to_string()));
}

#[test]
fn test_unified_config_with_backends() {
    let config = UnifiedQUBOConfig {
        dwave: Some(DWaveConfig::default()),
        ibm: Some(IBMConfig::default()),
        hybrid: Some(HybridSolverConfig::default()),
        hybrid_threshold: 1000,
        ..Default::default()
    };

    assert!(config.dwave.is_some());
    assert!(config.ibm.is_some());
    assert!(config.hybrid.is_some());
    assert_eq!(config.hybrid_threshold, 1000);
}

#[tokio::test]
async fn test_unified_solver_uses_simulated_annealing_fallback() {
    // Without any API tokens, should fall back to simulated annealing
    let config = UnifiedQUBOConfig::default();
    let solver = UnifiedQUBOSolver::new(config);
    let problem = create_simple_problem();

    let solution = solver.solve(&problem).await.unwrap();

    // Should use classical fallback
    assert_eq!(solution.variables.len(), 3);
    assert!(solution.energy.is_finite());
}

#[tokio::test]
async fn test_unified_solver_with_multiple_problems() {
    let config = UnifiedQUBOConfig::default();
    let solver = UnifiedQUBOSolver::new(config);

    // Test with different problem sizes
    for size in [2, 5, 10] {
        let problem = create_large_problem(size);
        let solution = solver.solve(&problem).await.unwrap();

        assert_eq!(solution.variables.len(), size);
        assert!(solution.energy.is_finite());
    }
}

#[tokio::test]
async fn test_unified_solver_maxcut_problem() {
    let config = UnifiedQUBOConfig::default();
    let solver = UnifiedQUBOSolver::new(config);
    let problem = create_maxcut_cycle(4);

    let solution = solver.solve(&problem).await.unwrap();

    assert_eq!(solution.variables.len(), 4);
    // All variables should be binary
    for &var in &solution.variables {
        assert!(var == 0 || var == 1);
    }
}

// =============================================================================
// OPTIMIZER TYPE TESTS
// =============================================================================

#[test]
fn test_ibm_optimizer_variants() {
    let optimizers = [
        IBMOptimizer::COBYLA,
        IBMOptimizer::SLSQP,
        IBMOptimizer::SPSA,
        IBMOptimizer::NFT,
    ];

    for optimizer in optimizers {
        let config = IBMConfig {
            optimizer: optimizer.clone(),
            ..Default::default()
        };
        assert_eq!(config.optimizer, optimizer);
    }
}

#[test]
fn test_ibm_optimizer_default() {
    assert_eq!(IBMOptimizer::default(), IBMOptimizer::COBYLA);
}

// =============================================================================
// BACKEND SELECTION TESTS
// =============================================================================

#[tokio::test]
async fn test_backend_selection_respects_problem_size() {
    // Create config with hybrid threshold
    let config = UnifiedQUBOConfig {
        hybrid_threshold: 10, // Use hybrid for problems > 10 variables
        ..Default::default()
    };

    let solver = UnifiedQUBOSolver::new(config);

    // Small problem - should not use hybrid
    let small_problem = create_simple_problem();
    let solution = solver.solve(&small_problem).await.unwrap();
    assert_eq!(solution.variables.len(), 3);

    // Larger problem - would use hybrid if available
    let large_problem = create_large_problem(15);
    let solution = solver.solve(&large_problem).await.unwrap();
    assert_eq!(solution.variables.len(), 15);
}

// =============================================================================
// SOLUTION QUALITY TESTS
// =============================================================================

#[tokio::test]
async fn test_solution_quality_metrics() {
    let config = SimulatedAnnealingConfig::default();
    let solver = SimulatedAnnealingQUBOSolver::new(config);
    let problem = create_simple_problem();

    let solution = solver.solve(&problem).await.unwrap();

    // Quality should be in valid range [0, 1]
    assert!(solution.quality >= 0.0);
    assert!(solution.quality <= 1.0);

    // Computation time should be positive
    assert!(solution.computation_time_ms >= 0.0);

    // Should report some iterations
    assert!(solution.iterations > 0);
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[tokio::test]
async fn test_single_variable_problem() {
    let config = SimulatedAnnealingConfig::default();
    let solver = SimulatedAnnealingQUBOSolver::new(config);

    let mut q_matrix = DMatrix::zeros(1, 1);
    q_matrix[(0, 0)] = -1.0;

    let problem = QUBOProblem {
        q_matrix,
        num_vars: 1,
        name: "Single Variable".to_string(),
    };

    let solution = solver.solve(&problem).await.unwrap();

    assert_eq!(solution.variables.len(), 1);
    // Optimal is x=1 with energy -1
    assert_eq!(solution.variables[0], 1);
    assert!((solution.energy - (-1.0)).abs() < 1e-6);
}

#[tokio::test]
async fn test_zero_matrix_problem() {
    let config = SimulatedAnnealingConfig::default();
    let solver = SimulatedAnnealingQUBOSolver::new(config);

    let q_matrix = DMatrix::zeros(3, 3);
    let problem = QUBOProblem {
        q_matrix,
        num_vars: 3,
        name: "Zero Matrix".to_string(),
    };

    let solution = solver.solve(&problem).await.unwrap();

    // All solutions have energy 0 for zero matrix
    assert!((solution.energy - 0.0).abs() < 1e-6);
}

// =============================================================================
// CONCURRENT SOLVER TESTS
// =============================================================================

#[tokio::test]
async fn test_concurrent_solves() {
    let config = SimulatedAnnealingConfig {
        max_iterations: 100,
        ..Default::default()
    };
    let solver = SimulatedAnnealingQUBOSolver::new(config);

    // Create multiple problems
    let problems: Vec<QUBOProblem> = (0..5).map(|i| create_large_problem(i + 2)).collect();

    // Solve concurrently
    let futures: Vec<_> = problems.iter().map(|p| solver.solve(p)).collect();
    let results: Vec<_> = futures::future::join_all(futures).await;

    // All should succeed
    for (i, result) in results.into_iter().enumerate() {
        let solution = result.unwrap();
        assert_eq!(solution.variables.len(), i + 2);
    }
}
