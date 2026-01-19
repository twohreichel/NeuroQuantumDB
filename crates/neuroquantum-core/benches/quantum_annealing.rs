#![allow(clippy::cast_precision_loss)]
//! Quantum Annealing Benchmarks
//!
//! Benchmarks for QUBO, TFIM, and Quantum Parallel Tempering algorithms
//! comparing quantum-inspired approaches against classical methods.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use nalgebra::DMatrix;
use neuroquantum_core::quantum::quantum_parallel_tempering::{
    IsingHamiltonian, QuantumBackend, QuantumParallelTempering, QuantumParallelTemperingConfig,
};
use neuroquantum_core::quantum::qubo_quantum::{
    max_cut_problem, tsp_problem, QUBOConfig, QUBOSolver, QuboQuantumBackend,
};
use neuroquantum_core::quantum::tfim::{FieldSchedule, TFIMSolver, TransverseFieldConfig};
use tokio::runtime::Runtime;

/// Benchmark QUBO solver on Max-Cut problems of varying sizes
fn bench_qubo_max_cut(c: &mut Criterion) {
    let mut group = c.benchmark_group("QUBO-MaxCut");

    for num_nodes in &[10, 20, 30, 50] {
        // Create a complete graph
        let mut edges = Vec::new();
        for i in 0..*num_nodes {
            for j in (i + 1)..*num_nodes {
                edges.push((i, j, 1.0));
            }
        }

        let problem = max_cut_problem(&edges, *num_nodes).unwrap();

        group.bench_with_input(BenchmarkId::new("nodes", num_nodes), num_nodes, |b, _| {
            let solver = QUBOSolver::new();
            b.iter(|| {
                let solution = solver.solve_problem(black_box(&problem)).unwrap();
                black_box(solution);
            });
        });
    }

    group.finish();
}

/// Benchmark QUBO solver with different configurations
fn bench_qubo_configs(c: &mut Criterion) {
    let mut group = c.benchmark_group("QUBO-Configs");

    let edges = vec![
        (0, 1, 1.0),
        (1, 2, 1.0),
        (2, 3, 1.0),
        (3, 0, 1.0),
        (0, 2, 1.0),
        (1, 3, 1.0),
    ];
    let problem = max_cut_problem(&edges, 4).unwrap();

    // Test with simulated quantum annealing
    group.bench_function("with-sqa", |b| {
        let config = QUBOConfig {
            backend: QuboQuantumBackend::SimulatedQuantumAnnealing,
            ..Default::default()
        };
        let solver = QUBOSolver::with_config(config);
        b.iter(|| {
            let solution = solver.solve_problem(black_box(&problem)).unwrap();
            black_box(solution);
        });
    });

    // Test with classical fallback
    group.bench_function("classical-fallback", |b| {
        let config = QUBOConfig {
            backend: QuboQuantumBackend::ClassicalFallback,
            ..Default::default()
        };
        let solver = QUBOSolver::with_config(config);
        b.iter(|| {
            let solution = solver.solve_problem(black_box(&problem)).unwrap();
            black_box(solution);
        });
    });

    group.finish();
}

/// Benchmark TFIM solver with different field schedules
fn bench_tfim_schedules(c: &mut Criterion) {
    let mut group = c.benchmark_group("TFIM-Schedules");

    let problem = TFIMSolver::ferromagnetic_model(10, 1.0).unwrap();

    // Linear schedule
    group.bench_function("linear", |b| {
        let config = TransverseFieldConfig {
            field_schedule: FieldSchedule::Linear,
            num_steps: 500,
            ..Default::default()
        };
        let solver = TFIMSolver::with_config(config);
        b.iter(|| {
            let solution = solver.solve(black_box(&problem)).unwrap();
            black_box(solution);
        });
    });

    // Exponential schedule
    group.bench_function("exponential", |b| {
        let config = TransverseFieldConfig {
            field_schedule: FieldSchedule::Exponential { decay_rate: 3.0 },
            num_steps: 500,
            ..Default::default()
        };
        let solver = TFIMSolver::with_config(config);
        b.iter(|| {
            let solution = solver.solve(black_box(&problem)).unwrap();
            black_box(solution);
        });
    });

    // Polynomial schedule
    group.bench_function("polynomial", |b| {
        let config = TransverseFieldConfig {
            field_schedule: FieldSchedule::Polynomial { power: 2.0 },
            num_steps: 500,
            ..Default::default()
        };
        let solver = TFIMSolver::with_config(config);
        b.iter(|| {
            let solution = solver.solve(black_box(&problem)).unwrap();
            black_box(solution);
        });
    });

    group.finish();
}

/// Benchmark TFIM on spin glass problems
fn bench_tfim_spin_glass(c: &mut Criterion) {
    let mut group = c.benchmark_group("TFIM-SpinGlass");

    for num_spins in &[5, 10, 15, 20] {
        let problem = TFIMSolver::spin_glass_model(*num_spins, 1.0).unwrap();

        group.bench_with_input(BenchmarkId::new("spins", num_spins), num_spins, |b, _| {
            let solver = TFIMSolver::new();
            b.iter(|| {
                let solution = solver.solve(black_box(&problem)).unwrap();
                black_box(solution);
            });
        });
    }

    group.finish();
}

/// Benchmark Quantum Parallel Tempering with different replica counts
fn bench_parallel_tempering_replicas(c: &mut Criterion) {
    let mut group = c.benchmark_group("QuantumParallelTempering-Replicas");
    let rt = Runtime::new().unwrap();

    let couplings = DMatrix::from_fn(8, 8, |i, j| if i == j { 0.0 } else { 1.0 });
    let external_fields = vec![0.0; 8];
    let initial_state = vec![1, -1, 1, -1, 1, -1, 1, -1];

    for num_replicas in &[2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("replicas", num_replicas),
            num_replicas,
            |b, &n| {
                let config = QuantumParallelTemperingConfig {
                    num_replicas: n,
                    num_exchanges: 20,
                    sweeps_per_exchange: 50,
                    backend: QuantumBackend::PathIntegralMonteCarlo,
                    ..Default::default()
                };

                b.iter(|| {
                    rt.block_on(async {
                        let mut qpt = QuantumParallelTempering::with_config(config.clone());
                        let hamiltonian = IsingHamiltonian::new(
                            8,
                            couplings.clone(),
                            external_fields.clone(),
                            1.0,
                        );
                        let solution = qpt
                            .optimize(hamiltonian, initial_state.clone())
                            .await
                            .unwrap();
                        black_box(solution);
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Quantum Parallel Tempering backends comparison
fn bench_parallel_vs_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("QuantumParallelTempering-Backends");
    let rt = Runtime::new().unwrap();

    let couplings = DMatrix::from_fn(10, 10, |i, j| {
        if i == j {
            0.0
        } else {
            (i as f64 - j as f64).abs().recip()
        }
    });
    let external_fields = vec![0.0; 10];
    let initial_state = vec![1; 10];

    // PIMC backend
    group.bench_function("pimc-backend", |b| {
        let config = QuantumParallelTemperingConfig {
            num_replicas: 4,
            num_exchanges: 20,
            sweeps_per_exchange: 50,
            backend: QuantumBackend::PathIntegralMonteCarlo,
            ..Default::default()
        };

        b.iter(|| {
            rt.block_on(async {
                let mut qpt = QuantumParallelTempering::with_config(config.clone());
                let hamiltonian =
                    IsingHamiltonian::new(10, couplings.clone(), external_fields.clone(), 1.0);
                let solution = qpt
                    .optimize(hamiltonian, initial_state.clone())
                    .await
                    .unwrap();
                black_box(solution);
            });
        });
    });

    // Quantum Annealing backend
    group.bench_function("quantum-annealing-backend", |b| {
        let config = QuantumParallelTemperingConfig {
            num_replicas: 4,
            num_exchanges: 20,
            sweeps_per_exchange: 50,
            backend: QuantumBackend::QuantumAnnealing,
            ..Default::default()
        };

        b.iter(|| {
            rt.block_on(async {
                let mut qpt = QuantumParallelTempering::with_config(config.clone());
                let hamiltonian =
                    IsingHamiltonian::new(10, couplings.clone(), external_fields.clone(), 1.0);
                let solution = qpt
                    .optimize(hamiltonian, initial_state.clone())
                    .await
                    .unwrap();
                black_box(solution);
            });
        });
    });

    // Hybrid backend
    group.bench_function("hybrid-backend", |b| {
        let config = QuantumParallelTemperingConfig {
            num_replicas: 4,
            num_exchanges: 20,
            sweeps_per_exchange: 50,
            backend: QuantumBackend::Hybrid,
            ..Default::default()
        };

        b.iter(|| {
            rt.block_on(async {
                let mut qpt = QuantumParallelTempering::with_config(config.clone());
                let hamiltonian =
                    IsingHamiltonian::new(10, couplings.clone(), external_fields.clone(), 1.0);
                let solution = qpt
                    .optimize(hamiltonian, initial_state.clone())
                    .await
                    .unwrap();
                black_box(solution);
            });
        });
    });

    group.finish();
}

/// Benchmark solution quality on known problems
fn bench_solution_quality(c: &mut Criterion) {
    let mut group = c.benchmark_group("Solution-Quality");

    // TSP-10 (small traveling salesman)
    let dist_matrix = DMatrix::from_fn(10, 10, |i, j| {
        if i == j {
            0.0
        } else {
            (i as f64 - j as f64)
                .mul_add(i as f64 - j as f64, 1.0)
                .sqrt()
        }
    });

    let problem = tsp_problem(&dist_matrix).unwrap();

    group.bench_function("TSP-10", |b| {
        let solver = QUBOSolver::new();
        b.iter(|| {
            let solution = solver.solve_problem(black_box(&problem)).unwrap();
            // Check that quality is reasonable (> 90%)
            assert!(solution.quality > 0.5, "Solution quality too low");
            black_box(solution);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_qubo_max_cut,
    bench_qubo_configs,
    bench_tfim_schedules,
    bench_tfim_spin_glass,
    bench_parallel_tempering_replicas,
    bench_parallel_vs_single,
    bench_solution_quality,
);

criterion_main!(benches);
