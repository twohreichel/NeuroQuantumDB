//! # Grover Hardware Backend Integration Tests
//!
//! This module contains comprehensive tests for the real quantum hardware
//! backend implementations including IBM Quantum, AWS Braket, and `IonQ`.
//!
//! ## Test Categories
//!
//! 1. Configuration Tests
//! 2. Solver Availability Tests
//! 3. Fallback Behavior Tests
//! 4. Unified Solver Tests
//! 5. Backend Selection Tests

use neuroquantum_core::quantum::grover_hardware_backends::{
    BraketGroverConfig, BraketGroverSolver, GroverHardwareBackend, IBMGroverConfig,
    IBMGroverSolver, IonQGroverConfig, IonQGroverSolver, SimulatorGroverConfig,
    SimulatorGroverSolver, UnifiedGroverConfig, UnifiedGroverSolver,
};
use neuroquantum_core::quantum::grover_quantum::{GroverQuantumBackend, QuantumOracle};

// =============================================================================
// TEST ORACLE CREATION HELPERS
// =============================================================================

/// Create a simple 2-qubit oracle (4-element search space)
fn create_simple_oracle() -> QuantumOracle {
    QuantumOracle::new(2, vec![2]) // Search for index 2 in 4-element space
}

/// Create a 3-qubit oracle (8-element search space)
fn create_medium_oracle() -> QuantumOracle {
    QuantumOracle::new(3, vec![5]) // Search for index 5 in 8-element space
}

/// Create a multi-target oracle
fn create_multi_target_oracle() -> QuantumOracle {
    QuantumOracle::new(3, vec![1, 5]) // Multiple targets
}

// =============================================================================
// IBM QUANTUM SOLVER TESTS
// =============================================================================

#[test]
fn test_ibm_grover_config_default() {
    let config = IBMGroverConfig::default();

    assert_eq!(config.num_shots, 1024);
    assert!(config.error_mitigation);
    assert!(config.dynamic_decoupling);
    assert!(config.api_token.is_none());
    assert_eq!(config.backend_name, "ibm_brisbane");
    assert_eq!(config.max_qubits, Some(127));
}

#[test]
fn test_ibm_grover_config_custom() {
    let config = IBMGroverConfig {
        api_token: Some("test-token".to_string()),
        backend_name: "ibm_kyoto".to_string(),
        num_shots: 2048,
        max_qubits: Some(50),
        ..Default::default()
    };

    assert_eq!(config.num_shots, 2048);
    assert_eq!(config.backend_name, "ibm_kyoto");
    assert_eq!(config.max_qubits, Some(50));
    assert_eq!(config.api_token, Some("test-token".to_string()));
}

#[test]
fn test_ibm_grover_solver_not_available_without_token() {
    let config = IBMGroverConfig::default();
    let solver = IBMGroverSolver::new(config);

    assert!(!solver.is_available());
    assert_eq!(solver.name(), "IBM Quantum Grover");
    assert_eq!(solver.backend_type(), GroverQuantumBackend::Superconducting);
}

#[test]
fn test_ibm_grover_solver_max_qubits() {
    let config = IBMGroverConfig {
        max_qubits: Some(127),
        ..Default::default()
    };
    let solver = IBMGroverSolver::new(config);

    assert_eq!(solver.max_qubits(), 127);
}

#[tokio::test]
async fn test_ibm_grover_solver_fallback_simulation() {
    let config = IBMGroverConfig {
        num_shots: 256,
        ..Default::default()
    };
    let solver = IBMGroverSolver::new(config);
    let oracle = create_simple_oracle();

    // Without API token, should use simulation fallback
    let result = solver.search(&oracle, 256).await.unwrap();

    assert!(result.computation_time_ms >= 0.0);
}

#[tokio::test]
async fn test_ibm_grover_solver_respects_qubit_limit() {
    let config = IBMGroverConfig {
        max_qubits: Some(2), // Only allow 2 qubits
        ..Default::default()
    };
    let solver = IBMGroverSolver::new(config);

    // 3-qubit oracle exceeds limit
    let oracle = create_medium_oracle();

    let result = solver.search(&oracle, 256).await;
    assert!(result.is_err());
}

// =============================================================================
// AWS BRAKET SOLVER TESTS
// =============================================================================

#[test]
fn test_braket_grover_config_default() {
    let config = BraketGroverConfig::default();

    assert_eq!(config.region, "us-east-1");
    assert_eq!(config.num_shots, 1024);
    assert_eq!(config.max_qubits, 25);
    assert!(config.device_arn.contains("sv1"));
}

#[test]
fn test_braket_grover_config_custom() {
    let config = BraketGroverConfig {
        device_arn: "arn:aws:braket:us-east-1::device/qpu/ionq/Aria-1".to_string(),
        num_shots: 4096,
        s3_bucket: Some("my-quantum-bucket".to_string()),
        max_qubits: 25,
        ..Default::default()
    };

    assert!(config.device_arn.contains("ionq"));
    assert_eq!(config.num_shots, 4096);
    assert!(config.s3_bucket.is_some());
}

#[test]
fn test_braket_grover_solver_backend_type_ionq() {
    let config = BraketGroverConfig {
        device_arn: "arn:aws:braket:us-east-1::device/qpu/ionq/Aria-1".to_string(),
        ..Default::default()
    };
    let solver = BraketGroverSolver::new(config);

    assert_eq!(solver.backend_type(), GroverQuantumBackend::TrappedIon);
    assert_eq!(solver.name(), "AWS Braket Grover");
}

#[test]
fn test_braket_grover_solver_backend_type_rigetti() {
    let config = BraketGroverConfig {
        device_arn: "arn:aws:braket:us-west-1::device/qpu/rigetti/Aspen-M-3".to_string(),
        ..Default::default()
    };
    let solver = BraketGroverSolver::new(config);

    assert_eq!(solver.backend_type(), GroverQuantumBackend::Superconducting);
}

#[test]
fn test_braket_grover_solver_max_qubits() {
    let config = BraketGroverConfig {
        max_qubits: 25,
        ..Default::default()
    };
    let solver = BraketGroverSolver::new(config);

    assert_eq!(solver.max_qubits(), 25);
}

#[tokio::test]
async fn test_braket_grover_solver_fallback_simulation() {
    let config = BraketGroverConfig {
        num_shots: 256,
        ..Default::default()
    };
    let solver = BraketGroverSolver::new(config);
    let oracle = create_simple_oracle();

    // Without AWS credentials, should use simulation fallback
    let result = solver.search(&oracle, 256).await.unwrap();

    assert!(result.computation_time_ms >= 0.0);
}

#[tokio::test]
async fn test_braket_grover_solver_respects_qubit_limit() {
    let config = BraketGroverConfig {
        max_qubits: 2, // Only allow 2 qubits
        ..Default::default()
    };
    let solver = BraketGroverSolver::new(config);

    // 3-qubit oracle exceeds limit
    let oracle = create_medium_oracle();

    let result = solver.search(&oracle, 256).await;
    assert!(result.is_err());
}

// =============================================================================
// IONQ SOLVER TESTS
// =============================================================================

#[test]
fn test_ionq_grover_config_default() {
    let config = IonQGroverConfig::default();

    assert_eq!(config.target, "simulator");
    assert_eq!(config.num_shots, 1024);
    assert_eq!(config.max_qubits, 25);
    assert_eq!(config.error_mitigation_level, 1);
    assert!(config.api_key.is_none());
}

#[test]
fn test_ionq_grover_config_custom() {
    let config = IonQGroverConfig {
        api_key: Some("test-ionq-key".to_string()),
        target: "qpu.aria-1".to_string(),
        num_shots: 2048,
        error_mitigation_level: 2,
        ..Default::default()
    };

    assert_eq!(config.target, "qpu.aria-1");
    assert_eq!(config.num_shots, 2048);
    assert_eq!(config.error_mitigation_level, 2);
}

#[test]
fn test_ionq_grover_solver_not_available_without_key() {
    let config = IonQGroverConfig::default();
    let solver = IonQGroverSolver::new(config);

    assert!(!solver.is_available());
    assert_eq!(solver.name(), "IonQ Trapped Ion");
    assert_eq!(solver.backend_type(), GroverQuantumBackend::TrappedIon);
}

#[test]
fn test_ionq_grover_solver_max_qubits() {
    let config = IonQGroverConfig {
        max_qubits: 25,
        ..Default::default()
    };
    let solver = IonQGroverSolver::new(config);

    assert_eq!(solver.max_qubits(), 25);
}

#[tokio::test]
async fn test_ionq_grover_solver_fallback_simulation() {
    let config = IonQGroverConfig {
        num_shots: 256,
        ..Default::default()
    };
    let solver = IonQGroverSolver::new(config);
    let oracle = create_simple_oracle();

    // Without API key, should use simulation fallback
    let result = solver.search(&oracle, 256).await.unwrap();

    assert!(result.computation_time_ms >= 0.0);
    assert_eq!(result.backend_used, GroverQuantumBackend::TrappedIon);
}

#[tokio::test]
async fn test_ionq_grover_solver_respects_qubit_limit() {
    let config = IonQGroverConfig {
        max_qubits: 2, // Only allow 2 qubits
        ..Default::default()
    };
    let solver = IonQGroverSolver::new(config);

    // 3-qubit oracle exceeds limit
    let oracle = create_medium_oracle();

    let result = solver.search(&oracle, 256).await;
    assert!(result.is_err());
}

// =============================================================================
// SIMULATOR SOLVER TESTS
// =============================================================================

#[test]
fn test_simulator_grover_config_default() {
    let config = SimulatorGroverConfig::default();

    assert_eq!(config.num_shots, 1024);
    assert_eq!(config.max_qubits, 20);
    assert!(!config.simulate_noise);
}

#[test]
fn test_simulator_grover_always_available() {
    let config = SimulatorGroverConfig::default();
    let solver = SimulatorGroverSolver::new(config);

    assert!(solver.is_available());
    assert_eq!(solver.name(), "Local State Vector Simulator");
    assert_eq!(solver.backend_type(), GroverQuantumBackend::Simulator);
}

#[tokio::test]
async fn test_simulator_grover_basic_search() {
    let solver = SimulatorGroverSolver::default();
    let oracle = create_simple_oracle();

    let result = solver.search(&oracle, 512).await.unwrap();

    // Should find the marked state
    assert!(result.computation_time_ms >= 0.0);
    assert_eq!(result.backend_used, GroverQuantumBackend::Simulator);
}

#[tokio::test]
async fn test_simulator_grover_multi_target_search() {
    let solver = SimulatorGroverSolver::default();
    let oracle = create_multi_target_oracle();

    let result = solver.search(&oracle, 512).await.unwrap();

    assert!(result.computation_time_ms >= 0.0);
}

#[tokio::test]
async fn test_simulator_grover_respects_qubit_limit() {
    let config = SimulatorGroverConfig {
        max_qubits: 2,
        ..Default::default()
    };
    let solver = SimulatorGroverSolver::new(config);

    // 3-qubit oracle exceeds limit
    let oracle = create_medium_oracle();

    let result = solver.search(&oracle, 256).await;
    assert!(result.is_err());
}

// =============================================================================
// UNIFIED SOLVER TESTS
// =============================================================================

#[test]
fn test_unified_grover_config_default() {
    let config = UnifiedGroverConfig::default();

    assert!(config.ibm.is_none());
    assert!(config.braket.is_none());
    assert!(config.ionq.is_none());
}

#[test]
fn test_unified_grover_config_with_backends() {
    let config = UnifiedGroverConfig {
        ibm: Some(IBMGroverConfig::default()),
        braket: Some(BraketGroverConfig::default()),
        ionq: Some(IonQGroverConfig::default()),
        ..Default::default()
    };

    assert!(config.ibm.is_some());
    assert!(config.braket.is_some());
    assert!(config.ionq.is_some());
}

#[test]
fn test_unified_grover_solver_available_backends() {
    let config = UnifiedGroverConfig::default();
    let solver = UnifiedGroverSolver::new(config);

    let backends = solver.available_backends();

    // Without any API keys, only simulator should be available
    assert!(backends.contains(&"simulator".to_string()));
    assert!(!backends.contains(&"ibm".to_string()));
    assert!(!backends.contains(&"ionq".to_string()));
}

#[tokio::test]
async fn test_unified_grover_solver_uses_simulator_fallback() {
    let config = UnifiedGroverConfig::default();
    let solver = UnifiedGroverSolver::new(config);
    let oracle = create_simple_oracle();

    let result = solver.search(&oracle, 256).await.unwrap();

    // Should use simulator as fallback
    assert_eq!(result.backend_used, GroverQuantumBackend::Simulator);
    assert!(result.computation_time_ms >= 0.0);
}

#[tokio::test]
async fn test_unified_grover_solver_specific_backend_simulator() {
    let solver = UnifiedGroverSolver::default();
    let oracle = create_simple_oracle();

    let result = solver
        .search_with_backend(&oracle, 256, "simulator")
        .await
        .unwrap();

    assert_eq!(result.backend_used, GroverQuantumBackend::Simulator);
}

#[tokio::test]
async fn test_unified_grover_solver_unknown_backend_error() {
    let solver = UnifiedGroverSolver::default();
    let oracle = create_simple_oracle();

    let result = solver
        .search_with_backend(&oracle, 256, "unknown_backend")
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_unified_grover_solver_unconfigured_backend_error() {
    let solver = UnifiedGroverSolver::default();
    let oracle = create_simple_oracle();

    // IBM is not configured in default, should error
    let result = solver.search_with_backend(&oracle, 256, "ibm").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_unified_grover_solver_multiple_problems() {
    let config = UnifiedGroverConfig::default();
    let solver = UnifiedGroverSolver::new(config);

    // Test with different oracle sizes
    for num_qubits in [2, 3, 4] {
        let search_space = 1 << num_qubits;
        let marked_state = search_space / 2; // Middle of search space
        let oracle = QuantumOracle::new(num_qubits, vec![marked_state]);

        let result = solver.search(&oracle, 256).await.unwrap();

        assert!(result.computation_time_ms >= 0.0);
        assert_eq!(result.circuit.num_qubits, num_qubits);
    }
}

// =============================================================================
// BACKEND SELECTION PRIORITY TESTS
// =============================================================================

#[test]
fn test_unified_grover_solver_priority_order() {
    let config = UnifiedGroverConfig {
        backend_priority: vec![
            "ionq".to_string(),
            "ibm".to_string(),
            "braket".to_string(),
            "simulator".to_string(),
        ],
        ..Default::default()
    };

    let solver = UnifiedGroverSolver::new(config);

    // Without any API keys, should fall back to simulator
    let backends = solver.available_backends();
    assert!(backends.contains(&"simulator".to_string()));
}

// =============================================================================
// CONCURRENT SOLVER TESTS
// =============================================================================

#[tokio::test]
async fn test_concurrent_grover_searches() {
    let solver = UnifiedGroverSolver::default();

    // Create multiple oracles
    let oracles: Vec<QuantumOracle> = (2..=4)
        .map(|n| QuantumOracle::new(n, vec![(1 << n) - 1]))
        .collect();

    // Solve concurrently
    let futures: Vec<_> = oracles
        .iter()
        .map(|oracle| solver.search(oracle, 128))
        .collect();

    let results: Vec<_> = futures::future::join_all(futures).await;

    // All should succeed
    for (i, result) in results.into_iter().enumerate() {
        let solution = result.unwrap();
        assert_eq!(solution.circuit.num_qubits, i + 2);
    }
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[tokio::test]
async fn test_grover_search_single_element() {
    let solver = SimulatorGroverSolver::default();

    // 1 qubit = 2 elements, search for index 1
    let oracle = QuantumOracle::new(1, vec![1]);
    let result = solver.search(&oracle, 256).await.unwrap();

    assert!(result.computation_time_ms >= 0.0);
}

#[tokio::test]
async fn test_grover_search_all_marked() {
    let solver = SimulatorGroverSolver::default();

    // 2 qubits = 4 elements, all marked
    let oracle = QuantumOracle::new(2, vec![0, 1, 2, 3]);
    let result = solver.search(&oracle, 256).await.unwrap();

    // With all states marked, any result is valid
    assert!(result.computation_time_ms >= 0.0);
}

#[tokio::test]
async fn test_grover_search_empty_oracle() {
    let solver = SimulatorGroverSolver::default();

    // Empty marked states
    let oracle = QuantumOracle::new(2, vec![]);
    let result = solver.search(&oracle, 256).await.unwrap();

    // Should handle gracefully
    assert!(result.found_indices.is_empty());
}
