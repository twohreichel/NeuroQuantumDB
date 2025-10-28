//! Benchmarks for ARM64 NEON optimizations
//!
//! This benchmark suite measures the performance gains from NEON SIMD
//! optimizations compared to scalar implementations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use neuroquantum_core::neon_optimization::{NeonOptimizer, QuantumOperation};

/// Benchmark DNA compression with varying data sizes
fn bench_dna_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("dna_compression");

    // Test with different data sizes
    for size in [64, 256, 1024, 4096, 16384].iter() {
        let data: Vec<u8> = (0..*size).map(|i| (i % 256) as u8).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let mut optimizer = NeonOptimizer::new().unwrap();
            b.iter(|| {
                black_box(
                    optimizer
                        .vectorized_dna_compression(black_box(&data))
                        .unwrap(),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark matrix multiplication with varying sizes
fn bench_matrix_multiply(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix_multiply");

    // Test with different matrix sizes
    for size in [4, 8, 16, 32, 64].iter() {
        let rows_a = *size;
        let cols_a = *size;
        let cols_b = *size;

        let matrix_a: Vec<f32> = (0..rows_a * cols_a).map(|i| i as f32 * 0.1).collect();
        let matrix_b: Vec<f32> = (0..cols_a * cols_b).map(|i| i as f32 * 0.2).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let mut optimizer = NeonOptimizer::new().unwrap();
            b.iter(|| {
                black_box(
                    optimizer
                        .matrix_multiply_neon(
                            black_box(&matrix_a),
                            black_box(&matrix_b),
                            rows_a,
                            cols_a,
                            cols_b,
                        )
                        .unwrap(),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark quantum state operations
fn bench_quantum_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("quantum_operations");

    // Test with different state vector sizes (powers of 2 for quantum states)
    for qubits in [4, 6, 8, 10, 12].iter() {
        let size = 1 << qubits; // 2^qubits
        let real_parts: Vec<f32> = (0..size).map(|i| (i as f32) / (size as f32)).collect();
        let imag_parts: Vec<f32> = (0..size)
            .map(|i| (size as f32 - i as f32) / (size as f32))
            .collect();

        // Benchmark Normalize operation
        group.bench_with_input(BenchmarkId::new("normalize", qubits), qubits, |b, _| {
            let mut optimizer = NeonOptimizer::new().unwrap();
            b.iter(|| {
                let mut r = real_parts.clone();
                let mut i = imag_parts.clone();
                optimizer
                .quantum_state_operation(
                    black_box(&mut r),
                    black_box(&mut i),
                    QuantumOperation::Normalize,
                )
                .unwrap();
                black_box(
                    (),
                );
            });
        });

        // Benchmark PhaseFlip operation
        group.bench_with_input(BenchmarkId::new("phase_flip", qubits), qubits, |b, _| {
            let mut optimizer = NeonOptimizer::new().unwrap();
            b.iter(|| {
                let mut r = real_parts.clone();
                let mut i = imag_parts.clone();
                optimizer
                .quantum_state_operation(
                    black_box(&mut r),
                    black_box(&mut i),
                    QuantumOperation::PhaseFlip,
                )
                .unwrap();
                black_box(
                    (),
                );
            });
        });

        // Benchmark Hadamard operation
        group.bench_with_input(BenchmarkId::new("hadamard", qubits), qubits, |b, _| {
            let mut optimizer = NeonOptimizer::new().unwrap();
            b.iter(|| {
                let mut r = real_parts.clone();
                let mut i = imag_parts.clone();
                optimizer
                .quantum_state_operation(
                    black_box(&mut r),
                    black_box(&mut i),
                    QuantumOperation::Hadamard,
                )
                .unwrap();
                black_box(
                    (),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark parallel search operations
fn bench_parallel_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_search");

    // Test with different haystack sizes
    for size in [256, 1024, 4096, 16384].iter() {
        let haystack: Vec<u8> = (0..*size).map(|i| (i % 256) as u8).collect();
        let needle = vec![42u8, 43, 44, 45];

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let mut optimizer = NeonOptimizer::new().unwrap();
            b.iter(|| {
                black_box(
                    optimizer
                        .parallel_search(black_box(&haystack), black_box(&needle))
                        .unwrap(),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark dot product operations
fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("dot_product");

    // Test with different vector sizes
    for size in [16, 64, 256, 1024, 4096].iter() {
        let a: Vec<f32> = (0..*size).map(|i| i as f32 * 0.1).collect();
        let b: Vec<f32> = (0..*size).map(|i| i as f32 * 0.2).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b_bench, _| {
            let optimizer = NeonOptimizer::new().unwrap();
            b_bench.iter(|| {
                black_box(optimizer.dot_product(black_box(&a), black_box(&b)).unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark activation function operations
fn bench_activation_function(c: &mut Criterion) {
    let mut group = c.benchmark_group("activation_function");

    // Test with different input sizes (typical neural network layer sizes)
    for size in [64, 128, 256, 512, 1024].iter() {
        let inputs: Vec<f32> = (0..*size).map(|i| (i as f32) * 0.01 - 5.0).collect();
        let threshold = 0.5;

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let optimizer = NeonOptimizer::new().unwrap();
            b.iter(|| {
                let mut input_copy = inputs.clone();
                optimizer
                .apply_activation_function(black_box(&mut input_copy), threshold)
                .unwrap();
                black_box(
                    (),
                );
            });
        });
    }

    group.finish();
}

/// Compare NEON vs Scalar implementations directly
fn bench_neon_vs_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("neon_vs_scalar");

    let data: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();

    // NEON enabled
    group.bench_function("dna_compression_neon", |b| {
        let mut optimizer = NeonOptimizer::new().unwrap();
        optimizer.set_enabled(true);
        b.iter(|| {
            black_box(
                optimizer
                    .vectorized_dna_compression(black_box(&data))
                    .unwrap(),
            );
        });
    });

    // NEON disabled (scalar fallback)
    group.bench_function("dna_compression_scalar", |b| {
        let mut optimizer = NeonOptimizer::new().unwrap();
        optimizer.set_enabled(false);
        b.iter(|| {
            black_box(
                optimizer
                    .vectorized_dna_compression(black_box(&data))
                    .unwrap(),
            );
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_dna_compression,
    bench_matrix_multiply,
    bench_quantum_operations,
    bench_parallel_search,
    bench_dot_product,
    bench_activation_function,
    bench_neon_vs_scalar
);
criterion_main!(benches);
