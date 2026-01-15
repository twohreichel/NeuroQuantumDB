//! Neuromorphic Index Performance Benchmarks
//!
//! This module benchmarks neuromorphic (Hebbian) indexes against classical B+ Tree indexes
//! to demonstrate the adaptive learning capabilities and performance characteristics.
//!
//! Run with: cargo bench --features benchmarks --bench `neuromorphic_index`

use std::collections::HashMap;
use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use neuroquantum_core::learning::AntiHebbianLearning;
use neuroquantum_core::storage::btree::BTree;
use neuroquantum_core::synaptic::{
    ActivationFunction, ConnectionType, SynapticNetwork, SynapticNode,
};
use rand::prelude::*;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Benchmark Hebbian index insertion performance
fn bench_hebbian_index_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("hebbian_index_insert");
    group.measurement_time(Duration::from_secs(10));

    for size in &[100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                // Create a new synaptic network for each iteration
                let network = SynapticNetwork::new(size + 10, 0.5).unwrap();

                // Insert nodes with connections (simulating index entries)
                for i in 0..size {
                    let node =
                        SynapticNode::with_data(i as u64, format!("key{i:010}").into_bytes());
                    let _ = network.add_node(node);

                    // Create connections to simulate index relationships
                    if i > 0 {
                        // Connect to previous nodes with decreasing weight
                        let prev_id = (i - 1) as u64;
                        let _ = network.connect_nodes(
                            i as u64,
                            prev_id,
                            0.5,
                            ConnectionType::Excitatory,
                        );
                    }
                }

                black_box(network)
            });
        });
    }

    group.finish();
}

/// Benchmark Hebbian index lookup with synaptic activation
fn bench_hebbian_index_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("hebbian_index_lookup");
    group.measurement_time(Duration::from_secs(10));

    for size in &[1_000, 10_000, 50_000] {
        group.throughput(Throughput::Elements(1));

        // Setup: Create network with data
        let network = SynapticNetwork::new(*size + 10, 0.5).unwrap();
        for i in 0..*size {
            let node = SynapticNode::with_data(i as u64, format!("key{i:010}").into_bytes());
            let _ = network.add_node(node);

            if i > 0 {
                let prev_id = (i - 1) as u64;
                let _ = network.connect_nodes(i as u64, prev_id, 0.5, ConnectionType::Excitatory);
            }
        }

        let mut rng = StdRng::seed_from_u64(42);
        let search_keys: Vec<u64> = (0..100).map(|_| rng.gen_range(0..*size) as u64).collect();
        let mut counter = 0;

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let key = search_keys[counter % search_keys.len()];
                counter += 1;

                // Simulate neuromorphic lookup with activation propagation
                let result = network.get_node(key);
                if let Some(ref node) = result {
                    // Propagate activation through connections (strengthen connected nodes)
                    let connections = node.connections.clone();
                    for conn in &connections {
                        network.modify_node(conn.target_id, |target_node| {
                            target_node.strengthen(0.1);
                        });
                    }
                }

                black_box(result.map(|n| n.data_payload))
            });
        });
    }

    group.finish();
}

/// Benchmark Hebbian learning with synaptic weight updates
fn bench_hebbian_learning_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("hebbian_learning_update");
    group.measurement_time(Duration::from_secs(10));

    for size in &[100, 1_000, 5_000] {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            // Setup: Create network
            let network = SynapticNetwork::new(size + 10, 0.5).unwrap();
            for i in 0..size {
                let node = SynapticNode::with_data(i as u64, format!("key{i:010}").into_bytes());
                let _ = network.add_node(node);

                if i > 0 {
                    let _ = network.connect_nodes(
                        i as u64,
                        (i - 1) as u64,
                        0.5,
                        ConnectionType::Excitatory,
                    );
                }
                if i > 1 {
                    let _ = network.connect_nodes(
                        i as u64,
                        (i - 2) as u64,
                        0.3,
                        ConnectionType::Excitatory,
                    );
                }
            }

            b.iter(|| {
                // Simulate Hebbian learning updates
                let node_ids = network.get_node_ids();
                for &node_id in node_ids.iter().take(10) {
                    network.modify_node(node_id, |node| {
                        for conn in &mut node.connections {
                            // Hebbian update: Δw = η * pre * post
                            let pre_activation = node.strength;
                            let post_activation = 0.5; // Simulated
                            let learning_rate = 0.01;
                            conn.weight += learning_rate * pre_activation * post_activation;
                            conn.weight = conn.weight.clamp(-2.0, 2.0);
                        }
                    });
                }
                black_box(&network)
            });
        });
    }

    group.finish();
}

/// Benchmark comparison: Hebbian Index vs B+ Tree for sequential inserts
fn bench_hebbian_vs_btree_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("hebbian_vs_btree_insert");
    group.measurement_time(Duration::from_secs(15));

    for size in &[100, 1_000, 5_000] {
        group.throughput(Throughput::Elements(*size as u64));

        // B+ Tree insertion
        group.bench_with_input(BenchmarkId::new("btree", size), size, |b, &size| {
            let rt = Runtime::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let temp_dir = TempDir::new().unwrap();
                    let mut btree = BTree::new(temp_dir.path()).await.unwrap();

                    for i in 0..size {
                        let key = format!("key{i:010}").into_bytes();
                        btree.insert(key, i as u64).await.unwrap();
                    }

                    black_box(btree)
                })
            });
        });

        // Hebbian/Synaptic Index insertion
        group.bench_with_input(BenchmarkId::new("hebbian", size), size, |b, &size| {
            b.iter(|| {
                let network = SynapticNetwork::new(size + 10, 0.5).unwrap();

                for i in 0..size {
                    let node =
                        SynapticNode::with_data(i as u64, format!("key{i:010}").into_bytes());
                    let _ = network.add_node(node);

                    if i > 0 {
                        let _ = network.connect_nodes(
                            i as u64,
                            (i - 1) as u64,
                            0.5,
                            ConnectionType::Excitatory,
                        );
                    }
                }

                black_box(network)
            });
        });
    }

    group.finish();
}

/// Benchmark comparison: Hebbian Index vs B+ Tree for lookups
fn bench_hebbian_vs_btree_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("hebbian_vs_btree_lookup");
    group.measurement_time(Duration::from_secs(15));

    for size in &[1_000, 10_000] {
        group.throughput(Throughput::Elements(1));

        // Setup B+ Tree
        let rt = Runtime::new().unwrap();
        let (temp_dir, btree) = rt.block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let mut btree = BTree::new(temp_dir.path()).await.unwrap();

            for i in 0..*size {
                let key = format!("key{i:010}").into_bytes();
                btree.insert(key, i as u64).await.unwrap();
            }

            (temp_dir, btree)
        });

        // Setup Hebbian Index
        let network = SynapticNetwork::new(*size + 10, 0.5).unwrap();
        for i in 0..*size {
            let node = SynapticNode::with_data(i as u64, format!("key{i:010}").into_bytes());
            let _ = network.add_node(node);

            if i > 0 {
                let _ = network.connect_nodes(
                    i as u64,
                    (i - 1) as u64,
                    0.5,
                    ConnectionType::Excitatory,
                );
            }
        }

        let mut rng = StdRng::seed_from_u64(42);
        let search_indices: Vec<usize> = (0..100).map(|_| rng.gen_range(0..*size)).collect();
        let mut counter = 0;

        // B+ Tree lookup
        group.bench_with_input(BenchmarkId::new("btree", size), size, |b, _| {
            b.iter(|| {
                let idx = search_indices[counter % search_indices.len()];
                counter += 1;
                let key = format!("key{idx:010}").into_bytes();
                rt.block_on(async { black_box(btree.search(&key).await.unwrap()) })
            });
        });

        // Hebbian Index lookup
        let mut counter2 = 0;
        group.bench_with_input(BenchmarkId::new("hebbian", size), size, |b, _| {
            b.iter(|| {
                let idx = search_indices[counter2 % search_indices.len()];
                counter2 += 1;
                black_box(network.get_node(idx as u64))
            });
        });

        drop(temp_dir);
    }

    group.finish();
}

/// Benchmark synaptic weight calculation performance
fn bench_synaptic_weight_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("synaptic_weight_calculation");
    group.measurement_time(Duration::from_secs(10));

    for connection_count in &[100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(*connection_count as u64));

        // Setup network with many connections
        let node_count = f64::from(*connection_count).sqrt() as usize + 1;
        let network = SynapticNetwork::new(node_count + 10, 0.5).unwrap();

        for i in 0..node_count {
            let node = SynapticNode::new(i as u64);
            let _ = network.add_node(node);
        }

        // Create connections
        let mut conn_created = 0;
        'outer: for i in 0..node_count {
            for j in 0..node_count {
                if i != j {
                    let _ =
                        network.connect_nodes(i as u64, j as u64, 0.5, ConnectionType::Excitatory);
                    conn_created += 1;
                    if conn_created >= *connection_count {
                        break 'outer;
                    }
                }
            }
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(connection_count),
            connection_count,
            |b, _| {
                b.iter(|| {
                    // Simulate weight updates using Hebbian rule
                    let node_ids = network.get_node_ids();
                    for &node_id in node_ids.iter().take(10) {
                        network.modify_node(node_id, |node| {
                            for conn in &mut node.connections {
                                // Hebbian update: Δw = η * pre * post
                                let pre_activation = node.strength;
                                let post_activation = 0.5; // Simulated
                                let learning_rate = 0.01;
                                conn.weight += learning_rate * pre_activation * post_activation;
                                conn.weight = conn.weight.clamp(-2.0, 2.0);
                            }
                        });
                    }

                    black_box(&network)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Anti-Hebbian learning (pruning and decay)
fn bench_anti_hebbian_pruning(c: &mut Criterion) {
    let mut group = c.benchmark_group("anti_hebbian_pruning");
    group.measurement_time(Duration::from_secs(10));

    for size in &[1_000, 5_000, 10_000] {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            // Setup network with connections of varying weights
            let network = SynapticNetwork::new(size + 10, 0.5).unwrap();
            let mut rng = StdRng::seed_from_u64(12345);

            for i in 0..size {
                let node = SynapticNode::new(i as u64);
                let _ = network.add_node(node);

                // Create connections with random weights (some below threshold)
                if i > 0 {
                    let weight = rng.gen_range(0.0..1.0);
                    let _ = network.connect_nodes(
                        i as u64,
                        (i - 1) as u64,
                        weight,
                        ConnectionType::Excitatory,
                    );
                }
            }

            let mut anti_hebbian = AntiHebbianLearning::new(0.1, 0.3);

            b.iter(|| {
                // Apply anti-Hebbian learning
                let _ = anti_hebbian.apply_synaptic_decay(&network);
                let _ = anti_hebbian.apply_weakening(&network);
                black_box(&network)
            });
        });
    }

    group.finish();
}

/// Benchmark competitive learning (Winner-Takes-All)
fn bench_competitive_learning(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitive_learning");
    group.measurement_time(Duration::from_secs(10));

    for neuron_count in &[50, 100, 500] {
        group.throughput(Throughput::Elements(*neuron_count as u64));

        // Setup network
        let network = SynapticNetwork::new(*neuron_count + 10, 0.5).unwrap();
        let mut rng = StdRng::seed_from_u64(54321);

        for i in 0..*neuron_count {
            let node = SynapticNode::new(i as u64);
            let _ = network.add_node(node);

            // Create some connections
            if i > 0 {
                let _ = network.connect_nodes(
                    i as u64,
                    (i - 1) as u64,
                    rng.gen_range(0.3..0.8),
                    ConnectionType::Excitatory,
                );
            }
        }

        // Generate random activations
        let activations: HashMap<u64, f32> = (0..*neuron_count)
            .map(|i| (i as u64, rng.gen_range(0.0..1.0)))
            .collect();

        let mut anti_hebbian = AntiHebbianLearning::new(0.1, 0.2);

        group.bench_with_input(
            BenchmarkId::from_parameter(neuron_count),
            neuron_count,
            |b, _| {
                b.iter(|| {
                    // Apply competitive learning with k=5 winners
                    let winners = anti_hebbian
                        .apply_competitive_learning(&network, &activations, 5)
                        .unwrap();
                    black_box(winners)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark activation function performance
fn bench_activation_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("activation_functions");

    let mut rng = StdRng::seed_from_u64(99999);
    let inputs: Vec<f32> = (0..10_000).map(|_| rng.gen_range(-5.0..5.0)).collect();

    let functions = [
        ActivationFunction::Sigmoid,
        ActivationFunction::ReLU,
        ActivationFunction::Tanh,
        ActivationFunction::Linear,
        ActivationFunction::LeakyReLU,
    ];

    for func in &functions {
        group.throughput(Throughput::Elements(inputs.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("activate", format!("{func:?}")),
            &inputs,
            |b, inputs| {
                b.iter(|| {
                    let results: Vec<f32> = inputs.iter().map(|&x| func.activate(x)).collect();
                    black_box(results)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("derivative", format!("{func:?}")),
            &inputs,
            |b, inputs| {
                b.iter(|| {
                    let results: Vec<f32> = inputs.iter().map(|&x| func.derivative(x)).collect();
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_hebbian_index_insert,
    bench_hebbian_index_lookup,
    bench_hebbian_learning_update,
    bench_hebbian_vs_btree_insert,
    bench_hebbian_vs_btree_lookup,
    bench_synaptic_weight_calculation,
    bench_anti_hebbian_pruning,
    bench_competitive_learning,
    bench_activation_functions,
);

criterion_main!(benches);
