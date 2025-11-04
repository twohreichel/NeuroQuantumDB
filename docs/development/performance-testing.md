# Performance Testing Guide

## Overview

This guide covers performance benchmarking, profiling, and optimization for NeuroQuantumDB.

---

## Quick Start

### Run All Benchmarks

```bash
make benchmark
# or
cargo bench --workspace --all-features
```

### Generate Performance Report

```bash
./scripts/performance-report.sh
```

### Load Testing

```bash
# Install k6
brew install k6  # macOS
# or
curl -L https://github.com/grafana/k6/releases/download/v0.45.0/k6-v0.45.0-linux-arm64.tar.gz | tar xvz

# Run load test
k6 run scripts/load-test.js

# With custom parameters
BASE_URL=http://localhost:8080 API_KEY=your-key k6 run scripts/load-test.js
```

---

## Benchmarks

### Available Benchmarks

1. **BTree Storage Benchmark** (`btree_benchmark.rs`)
   - Insert performance
   - Search performance
   - Range query performance
   - Memory usage

2. **DNA Compression Benchmark** (`dna_compression.rs`)
   - Compression speed
   - Compression ratio
   - Decompression speed
   - Memory overhead

3. **Quantum Algorithms** (`grover_search.rs`, `quantum_annealing.rs`)
   - Grover's search algorithm
   - Quantum annealing optimization
   - Qubit simulation overhead

4. **NEON Optimizations** (`neon_optimization.rs`)
   - SIMD vs Scalar performance
   - ARM64 NEON intrinsics
   - Vectorized operations

5. **Page Storage** (`page_storage_benchmark.rs`)
   - Page read/write speed
   - Buffer pool hit rate
   - Cache efficiency

### Running Specific Benchmarks

```bash
# Run only BTree benchmarks
cargo bench --bench btree_benchmark

# Run only DNA compression benchmarks
cargo bench --bench dna_compression

# Run NEON benchmarks (requires ARM64 or emulation)
cargo bench --bench neon_optimization
```

### Benchmark Results Location

Results are saved to:
- **Criterion reports:** `target/criterion/`
- **HTML reports:** `target/criterion/report/index.html`

---

## Performance Profiling

### CPU Profiling (Flamegraph)

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
make profile-flamegraph
# or
cargo flamegraph --bench btree_benchmark --root

# View flamegraph
open flamegraph.svg
```

**Interpreting Flamegraphs:**
- Wider boxes = more CPU time
- Height = call stack depth
- Look for wide boxes at the top = hot paths

### Memory Profiling

```bash
# Install valgrind (macOS)
brew install valgrind

# Profile memory usage
make profile-memory
# or
cargo build --release
valgrind --tool=massif --massif-out-file=massif.out target/release/neuroquantum-api

# Visualize with massif-visualizer
massif-visualizer massif.out
```

### Cache Profiling

```bash
# Profile cache behavior
make profile-cache
# or
valgrind --tool=cachegrind target/release/neuroquantum-api

# Analyze results
cg_annotate cachegrind.out
```

### All Profiling Tools

```bash
make profile-all
```

---

## Load Testing

### K6 Load Test

The `load-test.js` script simulates realistic user behavior:

**Test Scenarios:**
1. Health checks
2. Database queries
3. DNA compression
4. Quantum search
5. Transactions

**Load Profile:**
- Ramp up: 0 → 10 users (30s)
- Ramp up: 10 → 50 users (1m)
- Peak: 50 → 100 users (2m)
- Ramp down: 100 → 50 users (1m)
- Ramp down: 50 → 0 users (30s)

**Performance Thresholds:**
- 95% of requests < 500ms
- Error rate < 5%
- 99% of queries < 1s

### Custom Load Test

```bash
# Light load (10 users)
k6 run --vus 10 --duration 1m scripts/load-test.js

# Heavy load (200 users)
k6 run --vus 200 --duration 5m scripts/load-test.js

# Stress test (find breaking point)
k6 run --stages '[{"duration":"2m","target":500}]' scripts/load-test.js
```

### Analyzing Load Test Results

K6 outputs:
- **http_req_duration:** Request latency percentiles
- **http_req_failed:** Error rate
- **iterations:** Completed scenarios
- **vus:** Virtual users over time

---

## Performance Targets

### Raspberry Pi 4 (ARM64)

| Metric | Target | Status |
|--------|--------|--------|
| Startup Time | < 5s | ⏳ |
| Memory (Idle) | < 50MB | ⏳ |
| Memory (Load) | < 100MB | ⏳ |
| Query Latency | < 10ms | ⏳ |
| Throughput | > 1000 qps | ⏳ |
| DNA Compression | > 5MB/s | ⏳ |
| Backup Speed | > 10MB/s | ⏳ |
| Binary Size | < 15MB | ⏳ |
| Power Consumption | < 2W idle | ⏳ |

---

## Optimization Strategies

### 1. Compiler Optimizations

**Cargo.toml:**
```toml
[profile.release]
opt-level = 3           # Maximum optimization
codegen-units = 1       # Better optimization, slower compile
lto = "fat"             # Full Link Time Optimization
panic = "abort"         # Smaller binary
strip = true            # Remove debug symbols

[profile.production]
inherits = "release"
opt-level = "z"         # Optimize for size
```

**Build commands:**
```bash
# Size optimization
make optimize-size

# Speed optimization
make optimize-speed

# Production build (balanced)
cargo build --profile production
```

### 2. Code-Level Optimizations

**Hot Path Optimization:**
- Use `#[inline]` for small, frequently-called functions
- Reduce allocations (use `SmallVec`, stack arrays)
- Minimize `clone()` calls
- Use `&str` instead of `String` where possible

**Example:**
```rust
#[inline]
fn hot_function(data: &[u8]) -> u32 {
    // Use stack allocation for small arrays
    let mut buffer = [0u8; 64];
    // ... process data
}
```

**Data Structure Optimization:**
- Use `Vec` with pre-allocated capacity
- Prefer `HashMap` with capacity hints
- Use `Arc` instead of `Rc` for thread-safe sharing
- Consider `parking_lot::RwLock` over `std::sync::RwLock`

### 3. NEON SIMD Optimizations (ARM64)

Enable NEON in `Cargo.toml`:
```toml
[target.'cfg(target_arch = "aarch64")'.dependencies]
# NEON intrinsics enabled automatically
```

Build with NEON:
```bash
RUSTFLAGS="-C target-feature=+neon" cargo build --release
```

### 4. Memory Optimization

**Use jemalloc (better for concurrent workloads):**
```rust
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

**Profile with heaptrack:**
```bash
heaptrack target/release/neuroquantum-api
heaptrack_gui heaptrack.neuroquantum-api.*.gz
```

### 5. Async Runtime Tuning

**Tokio configuration:**
```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    // ...
}
```

---

## Continuous Performance Monitoring

### 1. Regression Testing

Add benchmark to CI/CD:
```yaml
# .github/workflows/benchmark.yml
- name: Run benchmarks
  run: cargo bench --workspace -- --save-baseline main
  
- name: Compare to baseline
  run: cargo bench --workspace -- --baseline main
```

### 2. Performance Metrics in Production

**Prometheus metrics:**
- Query latency histograms
- Throughput (queries/second)
- Buffer pool hit rate
- Memory usage
- CPU utilization

**Grafana dashboards:**
- Real-time performance graphs
- Alerts on performance degradation

### 3. Automated Performance Reports

```bash
# Schedule daily reports
crontab -e
0 2 * * * cd /path/to/NeuroQuantumDB && ./scripts/performance-report.sh
```

---

## Troubleshooting Performance Issues

### High CPU Usage

1. **Profile with flamegraph:**
   ```bash
   make profile-flamegraph
   ```

2. **Check for tight loops:**
   ```bash
   grep -r "loop\|while" crates/ | grep -v "test"
   ```

3. **Review async task spawning:**
   - Too many tasks can cause overhead
   - Consider batching operations

### High Memory Usage

1. **Profile with massif:**
   ```bash
   make profile-memory
   ```

2. **Check for memory leaks:**
   ```bash
   valgrind --leak-check=full target/release/neuroquantum-api
   ```

3. **Review cache sizes:**
   - Buffer pool size
   - Query cache size
   - Connection pool size

### Slow Queries

1. **Enable query logging:**
   ```rust
   RUST_LOG=debug cargo run
   ```

2. **Use EXPLAIN ANALYZE:**
   ```sql
   EXPLAIN ANALYZE SELECT * FROM large_table WHERE condition;
   ```

3. **Check buffer pool hit rate:**
   ```bash
   curl http://localhost:9090/metrics | grep buffer_pool_hit_rate
   ```

### High Latency

1. **Check network latency:**
   ```bash
   ping localhost
   curl -w "@curl-format.txt" http://localhost:8080/health
   ```

2. **Profile request handling:**
   ```bash
   cargo flamegraph --bin neuroquantum-api
   ```

3. **Review middleware stack:**
   - Rate limiting overhead
   - Authentication latency
   - Logging overhead

---

## Best Practices

### Before Optimizing

1. **Profile first** - Don't optimize blindly
2. **Measure baseline** - Know current performance
3. **Set targets** - Define acceptable thresholds
4. **Test on real hardware** - Raspberry Pi 4 performance differs

### During Optimization

1. **One change at a time** - Isolate improvements
2. **Benchmark after each change** - Verify impact
3. **Document findings** - Track what works
4. **Consider trade-offs** - Speed vs size vs memory

### After Optimization

1. **Verify correctness** - Run full test suite
2. **Benchmark on ARM64** - Test on target platform
3. **Load test** - Ensure stability under load
4. **Document changes** - Update performance docs

---

## Resources

- **Rust Performance Book:** https://nnethercote.github.io/perf-book/
- **Criterion.rs Docs:** https://bheisler.github.io/criterion.rs/book/
- **K6 Documentation:** https://k6.io/docs/
- **Flamegraph Guide:** https://www.brendangregg.com/flamegraphs.html
- **ARM NEON Intrinsics:** https://developer.arm.com/architectures/instruction-sets/intrinsics/

---

## Next Steps

1. ✅ Run initial benchmarks
2. ⏳ Profile hot paths
3. ⏳ Implement top 3 optimizations
4. ⏳ Re-benchmark and verify
5. ⏳ Load test on Raspberry Pi 4
6. ⏳ Document final performance

