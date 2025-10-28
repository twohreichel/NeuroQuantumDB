# NeuroQuantumDB Examples

This directory contains example programs demonstrating the key features of NeuroQuantumDB.

## Running Examples

All examples can be run with:

```bash
cargo run --example <example_name> --release
```

The `--release` flag is recommended for performance benchmarks.

## Available Examples

### 1. DNA Compression Demo

**File:** `dna_compression_demo.rs`

Demonstrates the DNA-based quaternary compression system with error correction.

```bash
cargo run --example dna_compression_demo --release
```

**Features shown:**
- Quaternary DNA encoding (A, T, G, C)
- Reed-Solomon error correction
- Compression ratio analysis
- NEON SIMD acceleration (on ARM64)

---

### 2. Grover's Algorithm Demo

**File:** `grover_algorithm_demo.rs`

Shows quantum-inspired search using Grover's algorithm implementation.

```bash
cargo run --example grover_algorithm_demo --release
```

**Features shown:**
- Quantum state vector simulation
- Oracle function implementation
- Diffusion operator
- Quadratic speedup demonstration

---

### 3. Neuromorphic Learning Demo

**File:** `neuromorphic_learning_demo.rs`

Demonstrates the brain-inspired synaptic learning system.

```bash
cargo run --example neuromorphic_learning_demo --release
```

**Features shown:**
- Hebbian learning rules
- Spike-timing-dependent plasticity (STDP)
- Dynamic synaptic weights
- NEON-optimized weight updates (on ARM64)

---

### 4. NEON Optimization Demo ⭐ NEW

**File:** `neon_optimization_demo.rs`

Comprehensive demonstration of ARM64 NEON SIMD optimizations.

```bash
cargo run --example neon_optimization_demo --release
```

**Features shown:**
- DNA compression with NEON (4:1 compression ratio)
- Matrix multiplication for neural networks
- Quantum state operations (Normalize, PhaseFlip, Hadamard)
- Parallel pattern search
- Neural network operations (dot product, activation functions)
- Performance statistics and speedup measurements

**Expected output:**
```
🧬 NeuroQuantumDB - ARM64 NEON Optimization Demo
================================================

✅ ARM64 NEON SIMD detected and enabled

1️⃣  DNA Compression Demo
   ---------------------
   1024 bytes → 256 bytes (4.0:1 ratio) in 15.2µs
   4096 bytes → 1024 bytes (4.0:1 ratio) in 52.8µs
   ...

📊 Performance Statistics
   =====================
   SIMD operations:      15
   Scalar fallbacks:     0
   
   Speedup factors:
   - DNA compression:    3.47x
   - Matrix operations:  2.83x
   - Quantum operations: 3.21x
   - Overall gain:       3.17x
```

---

## Performance Notes

### ARM64 / Raspberry Pi

On ARM64 systems (Raspberry Pi 4, Apple Silicon), the examples will automatically use NEON SIMD acceleration for:
- DNA compression
- Matrix operations
- Quantum state calculations
- Pattern search

Expected speedups: **2-4x** compared to scalar implementations.

### x86_64 / Intel/AMD

On x86_64, some operations may use AVX2/SSE4.2 SIMD instructions where available, but performance characteristics will differ from ARM64.

### Benchmarking

For detailed performance measurements, use the benchmark suite:

```bash
# Run all benchmarks
cargo bench --features benchmarks

# Run specific benchmark
cargo bench --features benchmarks dna_compression
cargo bench --features benchmarks neon_optimization

# Compare NEON vs Scalar
cargo bench --features benchmarks -- neon_vs_scalar
```

## Example Output Locations

Some examples generate output files:
- Performance metrics: Written to stdout
- Benchmark results: `target/criterion/`
- Test data: Temporary files (auto-cleaned)

## Troubleshooting

### "NEON not available"

This is normal on non-ARM64 platforms. The code will automatically fall back to scalar implementations.

### Performance slower than expected

1. Always use `--release` flag
2. On Raspberry Pi, ensure thermal throttling isn't active
3. Check CPU governor is in "performance" mode: `cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor`

### Out of memory

Some examples (especially Grover's algorithm with many qubits) can be memory-intensive. Reduce problem sizes if needed.

## Contributing

To add a new example:
1. Create a new file in `examples/`
2. Add documentation here
3. Test on both ARM64 and x86_64
4. Submit a PR

## Further Reading

- [NEON Optimizations Guide](../docs/dev/neon-optimizations.md)
- [Performance Tuning](../docs/dev/performance.md)
- [Architecture Overview](../docs/dev/architecture.md)

