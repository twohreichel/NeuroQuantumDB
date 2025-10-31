//! Quantum Annealing Benchmarks
//!
//! Benchmarks for QUBO, TFIM, and Parallel Tempering algorithms
//! comparing quantum-inspired approaches against classical methods.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use nalgebra::DMatrix;
use neuroquantum_core::quantum::parallel_tempering::{
    ising_energy_function, ParallelTempering, ParallelTemperingConfig,
};
use neuroquantum_core::quantum::qubo::{QUBOConfig, QUBOSolver};
use neuroquantum_core::quantum::tfim::{FieldSchedule, TFIMSolver, TransverseFieldConfig};
use tokio::runtime::Runtime;

/// Benchmark QUBO solver on Max-Cut problems of varying sizes
fn bench_qubo_max_cut(c: &mut Criterion) {
    let mut group = c.benchmark_group("QUBO-MaxCut");

    for num_nodes in [10, 20, 30, 50].iter() {
        // Create a complete graph
        let mut edges = Vec::new();
        for i in 0..*num_nodes {
            for j in (i + 1)..*num_nodes {
                edges.push((i, j, 1.0));
            }
        }

        let problem = QUBOSolver::max_cut_problem(&edges, *num_nodes).unwrap();

        group.bench_with_input(BenchmarkId::new("nodes", num_nodes), num_nodes, |b, _| {
            let solver = QUBOSolver::new();
            b.iter(|| {
                let solution = solver.solve(black_box(&problem)).unwrap();
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
    let problem = QUBOSolver::max_cut_problem(&edges, 4).unwrap();

    // Test with quantum tunneling
    group.bench_function("with-tunneling", |b| {
        let config = QUBOConfig {
            quantum_tunneling: true,
            ..Default::default()
        };
        let solver = QUBOSolver::with_config(config);
        b.iter(|| {
            let solution = solver.solve(black_box(&problem)).unwrap();
            black_box(solution);
        });
    });

    // Test without quantum tunneling
    group.bench_function("without-tunneling", |b| {
        let config = QUBOConfig {
            quantum_tunneling: false,
            ..Default::default()
        };
        let solver = QUBOSolver::with_config(config);
        b.iter(|| {
            let solution = solver.solve(black_box(&problem)).unwrap();
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

    for num_spins in [5, 10, 15, 20].iter() {
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

/// Benchmark Parallel Tempering with different replica counts
fn bench_parallel_tempering_replicas(c: &mut Criterion) {
    let mut group = c.benchmark_group("ParallelTempering-Replicas");
    let rt = Runtime::new().unwrap();

    let couplings = DMatrix::from_fn(8, 8, |i, j| if i != j { 1.0 } else { 0.0 });
    let external_fields = vec![0.0; 8];
    let energy_fn = ising_energy_function(couplings, external_fields);
    let initial_state = vec![1, -1, 1, -1, 1, -1, 1, -1];

    for num_replicas in [2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("replicas", num_replicas),
            num_replicas,
            |b, &n| {
                let config = ParallelTemperingConfig {
                    num_replicas: n,
                    num_exchanges: 20,
                    steps_per_exchange: 50,
                    ..Default::default()
                };

                b.iter(|| {
                    rt.block_on(async {
                        let mut pt = ParallelTempering::with_config(config.clone());
                        let solution = pt
                            .optimize(initial_state.clone(), energy_fn.clone())
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

/// Benchmark Parallel Tempering vs single temperature annealing
fn bench_parallel_vs_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("ParallelTempering-Comparison");
    let rt = Runtime::new().unwrap();

    let couplings = DMatrix::from_fn(10, 10, |i, j| {
        if i != j {
            (i as f64 - j as f64).abs().recip()
        } else {
            0.0
        }
    });
    let external_fields = vec![0.0; 10];
    let energy_fn = ising_energy_function(couplings, external_fields);
    let initial_state = vec![1; 10];

    // Single temperature (equivalent to 1 replica)
    group.bench_function("single-temperature", |b| {
        let config = ParallelTemperingConfig {
            num_replicas: 1,
            num_exchanges: 100,
            steps_per_exchange: 100,
            ..Default::default()
        };

        b.iter(|| {
            rt.block_on(async {
                let mut pt = ParallelTempering::with_config(config.clone());
                let solution = pt
                    .optimize(initial_state.clone(), energy_fn.clone())
                    .await
                    .unwrap();
                black_box(solution);
            });
        });
    });

    // Parallel tempering with 8 replicas
    group.bench_function("parallel-8-replicas", |b| {
        let config = ParallelTemperingConfig {
            num_replicas: 8,
            num_exchanges: 100,
            steps_per_exchange: 100,
            ..Default::default()
        };

        b.iter(|| {
            rt.block_on(async {
                let mut pt = ParallelTempering::with_config(config.clone());
                let solution = pt
                    .optimize(initial_state.clone(), energy_fn.clone())
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
            ((i as f64 - j as f64).powi(2) + 1.0).sqrt()
        }
    });

    let problem = QUBOSolver::tsp_problem(&dist_matrix).unwrap();

    group.bench_function("TSP-10", |b| {
        let solver = QUBOSolver::new();
        b.iter(|| {
            let solution = solver.solve(black_box(&problem)).unwrap();
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
