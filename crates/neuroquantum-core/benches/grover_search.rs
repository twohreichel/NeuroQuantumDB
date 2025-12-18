//! Benchmarks for Grover's Algorithm Implementation
//!
//! Compares quantum search performance vs classical search

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use neuroquantum_core::quantum_processor::{create_byte_search_processor, QuantumProcessorConfig};
use std::hint::black_box;

/// Classical linear search for comparison
fn classical_search(data: &[u8], pattern: &[u8]) -> Option<usize> {
    (0..=data.len().saturating_sub(pattern.len())).find(|&i| &data[i..i + pattern.len()] == pattern)
}

/// Benchmark Grover's search vs classical search
fn bench_grover_vs_classical(c: &mut Criterion) {
    let mut group = c.benchmark_group("grover_vs_classical");

    for size in [16, 32, 64, 128, 256].iter() {
        let mut data: Vec<u8> = (0..*size).map(|i| (i % 256) as u8).collect();
        // Place target at random position
        let target_pos = size / 2;
        data[target_pos] = 42;
        let pattern = vec![42u8];

        // Benchmark classical search
        group.bench_with_input(BenchmarkId::new("classical", size), size, |b, _| {
            b.iter(|| classical_search(black_box(&data), black_box(&pattern)));
        });

        // Benchmark Grover's quantum search
        group.bench_with_input(BenchmarkId::new("grover", size), size, |b, _| {
            b.iter(|| {
                let config = QuantumProcessorConfig::default();
                let mut processor =
                    create_byte_search_processor(data.clone(), pattern.clone(), config).unwrap();
                processor.grovers_search().unwrap()
            });
        });
    }

    group.finish();
}

/// Benchmark superposition initialization
fn bench_superposition_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("superposition_init");

    for qubits in [4, 6, 8, 10].iter() {
        let size = 1 << qubits;
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let pattern = vec![42u8];

        group.bench_with_input(BenchmarkId::new("qubits", qubits), qubits, |b, _| {
            b.iter(|| {
                let config = QuantumProcessorConfig::default();
                let mut processor =
                    create_byte_search_processor(data.clone(), pattern.clone(), config).unwrap();
                processor.initialize_superposition().unwrap();
                black_box(processor.verify_normalization())
            });
        });
    }

    group.finish();
}

/// Benchmark oracle application
fn bench_oracle_application(c: &mut Criterion) {
    let mut group = c.benchmark_group("oracle_application");

    for qubits in [4, 6, 8].iter() {
        let size = 1 << qubits;
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let pattern = vec![42u8];

        group.bench_with_input(BenchmarkId::new("qubits", qubits), qubits, |b, _| {
            let config = QuantumProcessorConfig::default();
            let mut processor =
                create_byte_search_processor(data.clone(), pattern.clone(), config).unwrap();
            processor.initialize_superposition().unwrap();

            b.iter(|| {
                processor.apply_oracle().unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark diffusion operator
fn bench_diffusion_operator(c: &mut Criterion) {
    let mut group = c.benchmark_group("diffusion_operator");

    for qubits in [4, 6, 8].iter() {
        let size = 1 << qubits;
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let pattern = vec![42u8];

        group.bench_with_input(BenchmarkId::new("qubits", qubits), qubits, |b, _| {
            let config = QuantumProcessorConfig::default();
            let mut processor =
                create_byte_search_processor(data.clone(), pattern.clone(), config).unwrap();
            processor.initialize_superposition().unwrap();

            b.iter(|| {
                processor.apply_diffusion_operator().unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark full Grover iterations
fn bench_grover_iterations(c: &mut Criterion) {
    let mut group = c.benchmark_group("grover_iterations");

    for size in [16, 32, 64].iter() {
        let data: Vec<u8> = (0..*size).map(|i| (i % 256) as u8).collect();
        let pattern = vec![42u8];

        group.bench_with_input(BenchmarkId::new("size", size), size, |b, _| {
            b.iter(|| {
                let config = QuantumProcessorConfig {
                    max_grover_iterations: 100,
                    verify_normalization: false, // Disable for speed
                    measurement_threshold: 0.1,
                };
                let mut processor =
                    create_byte_search_processor(data.clone(), pattern.clone(), config).unwrap();
                processor.grovers_search().unwrap()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_grover_vs_classical,
    bench_superposition_init,
    bench_oracle_application,
    bench_diffusion_operator,
    bench_grover_iterations
);
criterion_main!(benches);
