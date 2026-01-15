use criterion::{criterion_group, criterion_main};
use neuroquantum_core::dna::benchmarks::{
    benchmark_compression_comparison, benchmark_dna_compression, benchmark_dna_decompression,
    benchmark_error_correction, benchmark_memory_usage, benchmark_parallel_scaling,
    benchmark_simd_performance,
};

criterion_group!(
    benches,
    benchmark_dna_compression,
    benchmark_dna_decompression,
    benchmark_simd_performance,
    benchmark_compression_comparison,
    benchmark_error_correction,
    benchmark_memory_usage,
    benchmark_parallel_scaling
);

criterion_main!(benches);
