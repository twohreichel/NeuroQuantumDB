//! NEON Optimization Tests
//!
//! Tests for ARM NEON SIMD optimizations including dot products, activation functions,
//! matrix operations, DNA compression, and quantum state operations.

use neuroquantum_core::neon_optimization::{NeonOptimizer, QuantumOperation};

#[test]
fn test_neon_optimizer_creation() {
    let _optimizer = NeonOptimizer::new().unwrap();
    // Should not fail regardless of platform
}

#[test]
fn test_dot_product() {
    let optimizer = NeonOptimizer::new().unwrap();
    let a = vec![1.0, 2.0, 3.0, 4.0];
    let b = vec![2.0, 3.0, 4.0, 5.0];

    let result = optimizer.dot_product(&a, &b).unwrap();
    let expected = 4.0f32.mul_add(5.0, 3.0f32.mul_add(4.0, 1.0f32.mul_add(2.0, 2.0 * 3.0)));
    assert!((result - expected).abs() < 1e-6);
}

#[test]
fn test_dot_product_mismatched_lengths() {
    let optimizer = NeonOptimizer::new().unwrap();
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![2.0, 3.0];

    let result = optimizer.dot_product(&a, &b);
    assert!(result.is_err());
}

#[test]
fn test_activation_function() {
    let optimizer = NeonOptimizer::new().unwrap();
    let mut inputs = vec![0.5, 1.5, -0.5, 2.0];
    let threshold = 1.0;

    optimizer
        .apply_activation_function(&mut inputs, threshold)
        .unwrap();

    // Expected: max(0, input - threshold)
    assert!((inputs[0] - 0.0).abs() < 1e-6); // 0.5 - 1.0 = -0.5 -> 0.0
    assert!((inputs[1] - 0.5).abs() < 1e-6); // 1.5 - 1.0 = 0.5
    assert!((inputs[2] - 0.0).abs() < 1e-6); // -0.5 - 1.0 = -1.5 -> 0.0
    assert!((inputs[3] - 1.0).abs() < 1e-6); // 2.0 - 1.0 = 1.0
}

#[test]
fn test_matrix_operations() {
    let optimizer = NeonOptimizer::new().unwrap();
    let mut matrix = vec![1.0, -1.0, 2.0, -2.0];

    optimizer.optimize_matrix_operations(&mut matrix).unwrap();

    // Should apply sigmoid-like function: 1/(1 + |x|)
    for &value in &matrix {
        assert!(value > 0.0 && value <= 1.0);
    }
}

#[test]
fn test_enable_disable() {
    let mut optimizer = NeonOptimizer::new().unwrap();
    let _initial_state = optimizer.is_enabled();

    optimizer.set_enabled(false);
    assert!(!optimizer.is_enabled());

    optimizer.set_enabled(true);
    // Should be enabled only if platform supports it
    #[cfg(target_arch = "aarch64")]
    assert!(optimizer.is_enabled());
}

#[test]
fn test_dna_compression() {
    let mut optimizer = NeonOptimizer::new().unwrap();
    let data = vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];

    let compressed = optimizer.vectorized_dna_compression(&data).unwrap();

    // Should compress data (not necessarily 4:1 due to chunking)
    assert!(!compressed.is_empty());
    assert!(compressed.len() <= data.len());
}

#[test]
fn test_dna_compression_empty() {
    let mut optimizer = NeonOptimizer::new().unwrap();
    let data = vec![];

    let compressed = optimizer.vectorized_dna_compression(&data).unwrap();
    assert!(compressed.is_empty());
}

#[test]
fn test_matrix_multiply() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    // 2x3 * 3x2 = 2x2
    let matrix_a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let matrix_b = vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0];

    let result = optimizer
        .matrix_multiply_neon(&matrix_a, &matrix_b, 2, 3, 2)
        .unwrap();

    assert_eq!(result.len(), 4); // 2x2 matrix

    // Verify basic matrix multiplication properties
    // First element should be: 1*7 + 2*9 + 3*11 = 7 + 18 + 33 = 58
    assert!((result[0] - 58.0).abs() < 1e-4);
}

#[test]
fn test_matrix_multiply_dimension_mismatch() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    let matrix_a = vec![1.0, 2.0, 3.0];
    let matrix_b = vec![4.0, 5.0];

    let result = optimizer.matrix_multiply_neon(&matrix_a, &matrix_b, 1, 3, 2);
    assert!(result.is_err());
}

#[test]
fn test_quantum_normalize() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    let mut real_parts = vec![3.0, 4.0, 0.0, 0.0];
    let mut imag_parts = vec![0.0, 0.0, 0.0, 0.0];

    optimizer
        .quantum_state_operation(
            &mut real_parts,
            &mut imag_parts,
            QuantumOperation::Normalize,
        )
        .unwrap();

    // Check normalization: |3|^2 + |4|^2 = 9 + 16 = 25, sqrt(25) = 5
    // So normalized should be [3/5, 4/5, 0, 0] = [0.6, 0.8, 0, 0]
    assert!((real_parts[0] - 0.6).abs() < 1e-4);
    assert!((real_parts[1] - 0.8).abs() < 1e-4);

    // Verify total norm is 1
    let norm_sq: f32 = real_parts
        .iter()
        .zip(imag_parts.iter())
        .map(|(r, i)| r * r + i * i)
        .sum();
    assert!((norm_sq - 1.0).abs() < 1e-4);
}

#[test]
fn test_quantum_phase_flip() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    let mut real_parts = vec![1.0, 2.0, 3.0, 4.0];
    let mut imag_parts = vec![0.5, 1.0, 1.5, 2.0];

    optimizer
        .quantum_state_operation(
            &mut real_parts,
            &mut imag_parts,
            QuantumOperation::PhaseFlip,
        )
        .unwrap();

    // All values should be negated
    assert_eq!(real_parts, vec![-1.0, -2.0, -3.0, -4.0]);
    assert_eq!(imag_parts, vec![-0.5, -1.0, -1.5, -2.0]);
}

#[test]
fn test_quantum_hadamard() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    // Simple 2-element state |0⟩
    let mut real_parts = vec![1.0, 0.0];
    let mut imag_parts = vec![0.0, 0.0];

    optimizer
        .quantum_state_operation(&mut real_parts, &mut imag_parts, QuantumOperation::Hadamard)
        .unwrap();

    // H|0⟩ = (|0⟩ + |1⟩)/√2
    let expected = std::f32::consts::FRAC_1_SQRT_2;
    assert!((real_parts[0] - expected).abs() < 1e-4);
    assert!((real_parts[1] - expected).abs() < 1e-4);
}

#[test]
fn test_parallel_search() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    let haystack = b"Hello world, this is a test. Hello again!";
    let needle = b"Hello";

    let matches = optimizer.parallel_search(haystack, needle).unwrap();

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0], 0);
    assert_eq!(matches[1], 29);
}

#[test]
fn test_parallel_search_no_match() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    let haystack = b"Hello world";
    let needle = b"xyz";

    let matches = optimizer.parallel_search(haystack, needle).unwrap();
    assert!(matches.is_empty());
}

#[test]
fn test_parallel_search_empty_needle() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    let haystack = b"Hello world";
    let needle = b"";

    let matches = optimizer.parallel_search(haystack, needle).unwrap();
    assert!(matches.is_empty());
}

#[test]
fn test_performance_stats() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    // Update some stats
    optimizer.update_performance_stats("dna_compression", 1000);
    optimizer.update_performance_stats("matrix_ops", 2000);
    optimizer.update_performance_stats("quantum_ops", 1500);

    let stats = optimizer.get_stats();

    // Speedup should be calculated
    assert!(stats.dna_compression_speedup > 0.0);
    assert!(stats.matrix_ops_speedup > 0.0);
    assert!(stats.quantum_ops_speedup > 0.0);
    assert!(stats.performance_gain > 0.0);
}

#[test]
fn test_optimization_stats() {
    let mut optimizer = NeonOptimizer::new().unwrap();

    // Perform some operations to generate stats
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let _ = optimizer.vectorized_dna_compression(&data);

    let stats = optimizer.get_stats();

    if optimizer.is_enabled() {
        assert!(stats.simd_operations > 0 || stats.scalar_fallbacks > 0);
    }
}

#[test]
#[cfg(target_arch = "aarch64")]
fn test_neon_feature_detection() {
    // On ARM64, NEON should always be available
    assert!(std::arch::is_aarch64_feature_detected!("neon"));
}
