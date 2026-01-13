# Benchmarking Guide

This guide documents how to run, analyze, and extend the NeuroQuantumDB benchmark suite.

## Overview

NeuroQuantumDB includes a comprehensive benchmark suite covering all major system components:

| Category | Benchmarks | Description |
|----------|------------|-------------|
| 🧬 DNA Compression | `dna_compression` | Quaternary encoding, Reed-Solomon error correction, comparison with gzip/lz4 |
| 🧠 Neuromorphic Indexes | `neuromorphic_index` | Hebbian learning, synaptic networks, comparison with B+ Trees |
| 💳 Transactions | `transactions` | Concurrent transactions, savepoints, WAL operations, lock manager |
| ⚛️ Quantum Algorithms | `grover_search`, `quantum_annealing` | Quantum search and optimization algorithms |
| 💾 Storage | `btree_benchmark`, `page_storage_benchmark` | B+ Tree and page storage operations |
| 🚀 SIMD | `neon_optimization` | NEON (ARM64) and AVX2 (x86-64) optimizations |
| 📊 QSQL Functions | `qsql_functions` | NEUROMATCH, QUANTUM_SEARCH, HEBBIAN_LEARNING |

## Running Benchmarks

### Prerequisites

Ensure you have Rust installed and the `benchmarks` feature enabled:

```bash
# Install criterion for HTML reports
cargo install cargo-criterion
```

### Running All Benchmarks

```bash
# Run all benchmarks in neuroquantum-core
cargo bench --package neuroquantum-core --features benchmarks

# Run all benchmarks in neuroquantum-qsql
cargo bench --package neuroquantum-qsql --features benchmarks
```

### Running Specific Benchmarks

```bash
# DNA Compression benchmarks
cargo bench --package neuroquantum-core --bench dna_compression --features benchmarks

# Neuromorphic Index benchmarks
cargo bench --package neuroquantum-core --bench neuromorphic_index --features benchmarks

# Transaction benchmarks
cargo bench --package neuroquantum-core --bench transactions --features benchmarks

# QSQL Function benchmarks
cargo bench --package neuroquantum-qsql --bench qsql_functions --features benchmarks
```

### Filtering Benchmark Tests

Run specific benchmark functions within a file:

```bash
# Only run Hebbian vs B+Tree comparison
cargo bench --package neuroquantum-core --bench neuromorphic_index --features benchmarks -- "hebbian_vs_btree"

# Only run NEUROMATCH benchmarks
cargo bench --package neuroquantum-qsql --bench qsql_functions --features benchmarks -- "neuromatch"
```

## Benchmark Results

### HTML Reports

After running benchmarks, find detailed HTML reports in:
```
target/criterion/<benchmark_name>/report/index.html
```

### Understanding Results

Each benchmark reports:
- **Mean time**: Average execution time
- **Standard deviation**: Variation in timing
- **Throughput**: Operations/bytes per second (where applicable)
- **Comparison**: Change vs previous run (regression/improvement)

### Example Output

```
neuromorphic_index/hebbian_index_insert/1000
                        time:   [45.123 µs 45.456 µs 45.789 µs]
                        thrpt:  [21.839 Melem/s 22.000 Melem/s 22.161 Melem/s]
                 change: [-2.1234% -1.5678% -1.0123%] (p = 0.00 < 0.05)
                        Performance has improved.
```

## Benchmark Categories

### DNA Compression Benchmarks

Compares DNA-based compression against standard algorithms:

| Benchmark | Description |
|-----------|-------------|
| `benchmark_dna_compression` | DNA encoding performance at various sizes |
| `benchmark_dna_decompression` | Decompression throughput |
| `benchmark_simd_performance` | SIMD-accelerated encoding |
| `benchmark_compression_comparison` | DNA vs gzip vs lz4 |
| `benchmark_error_correction` | Reed-Solomon overhead |

### Neuromorphic Index Benchmarks

Tests adaptive synaptic network-based indexing:

| Benchmark | Description |
|-----------|-------------|
| `bench_hebbian_index_insert` | Insertion into synaptic network |
| `bench_hebbian_index_lookup` | Lookup with activation propagation |
| `bench_hebbian_learning_update` | Synaptic weight updates |
| `bench_hebbian_vs_btree_insert` | Comparison: Hebbian vs B+ Tree insert |
| `bench_hebbian_vs_btree_lookup` | Comparison: Hebbian vs B+ Tree lookup |
| `bench_synaptic_weight_calculation` | Weight update performance |
| `bench_anti_hebbian_pruning` | Connection pruning and decay |
| `bench_competitive_learning` | Winner-Takes-All learning |
| `bench_activation_functions` | Sigmoid, ReLU, Tanh, etc. |

### Transaction Benchmarks

Tests ACID transaction performance:

| Benchmark | Description |
|-----------|-------------|
| `bench_transaction_lifecycle` | Create → commit cycle |
| `bench_concurrent_transactions` | 10/50/100 concurrent transactions |
| `bench_concurrent_mixed_operations` | Mixed read/write with aborts |
| `bench_savepoint_overhead` | Savepoint creation and rollback |
| `bench_lock_manager` | Lock acquisition performance |
| `bench_deadlock_detection` | Deadlock detection algorithm |
| `bench_wal_operations` | Write-Ahead Log throughput |
| `bench_transaction_throughput` | Overall TPS under load |

### QSQL Function Benchmarks

Tests neuromorphic SQL extensions:

| Benchmark | Description |
|-----------|-------------|
| `bench_neuromatch` | NEUROMATCH pattern matching |
| `bench_neuromatch_vs_like` | NEUROMATCH vs SQL LIKE comparison |
| `bench_quantum_search` | QUANTUM_SEARCH performance |
| `bench_quantum_search_vs_linear` | Quantum vs linear search comparison |
| `bench_hebbian_learning_optimization` | Query path learning |
| `bench_sql_parsing` | SQL parser performance |
| `bench_function_composition` | Chained function evaluation |
| `bench_neuromatch_thresholds` | Impact of match thresholds |

## CI Integration

Benchmarks run automatically on:
- Push to `main` branch
- Pull requests to `main` or `develop`
- Manual workflow dispatch

### Regression Detection

The CI will:
1. Run benchmarks on PR branch
2. Compare against base branch
3. Alert if performance degrades >10%
4. Fail if performance degrades >30%

### Viewing Results

- **PR Comments**: Benchmark comparison posted automatically
- **Artifacts**: Full Criterion reports uploaded as artifacts
- **GitHub Pages**: Historical trends at `https://<org>.github.io/<repo>/dev/bench`

## Extending the Benchmark Suite

### Adding a New Benchmark File

1. Create the benchmark file in `crates/<package>/benches/`:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn my_new_benchmark(c: &mut Criterion) {
    c.bench_function("my_operation", |b| {
        b.iter(|| {
            // Operation to benchmark
        });
    });
}

criterion_group!(benches, my_new_benchmark);
criterion_main!(benches);
```

2. Add to `Cargo.toml`:

```toml
[[bench]]
name = "my_benchmark"
harness = false
required-features = ["benchmarks"]
```

3. Update CI workflow if needed

### Best Practices

1. **Isolate setup from measurement**: Use `b.iter_with_setup()` for expensive setup
2. **Use black_box**: Prevent compiler optimizations with `std::hint::black_box()`
3. **Set appropriate measurement time**: Increase for slow operations
4. **Parameterize tests**: Use `BenchmarkId` for testing different sizes
5. **Report throughput**: Use `Throughput::Bytes()` or `Throughput::Elements()`

## Interpreting Results

### DNA Compression vs Standard

Expected results:
- DNA compression excels with structured/repetitive data
- gzip better for already-compressed data
- lz4 faster but lower compression ratio

### Hebbian vs B+ Tree

Expected results:
- Hebbian: Faster lookups after learning phase
- B+ Tree: More predictable, better for cold data
- Hebbian: Self-optimizing for access patterns

### Transaction Performance

Key metrics:
- TPS (Transactions Per Second)
- Latency percentiles (p50, p95, p99)
- Lock contention under load

## Troubleshooting

### Noisy Results

```bash
# Increase measurement time
cargo bench -- --measurement-time 30

# Increase sample size
cargo bench -- --sample-size 100
```

### Memory Issues

```bash
# Reduce concurrent operations in transaction benchmarks
RUST_TEST_THREADS=1 cargo bench
```

### Missing Features

```bash
# Ensure benchmarks feature is enabled
cargo bench --features benchmarks
```
