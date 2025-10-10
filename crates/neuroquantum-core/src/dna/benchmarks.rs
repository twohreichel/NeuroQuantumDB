//! Comprehensive benchmarking suite for DNA compression performance
//!
//! This module provides detailed benchmarks comparing DNA compression against
//! standard algorithms and measuring performance across different data patterns.

#[cfg(feature = "benchmarks")]
use crate::dna::{DNACompressor, QuantumDNACompressor};
#[cfg(feature = "benchmarks")]
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
#[cfg(feature = "benchmarks")]
use rand::prelude::*;

// Only compile benchmarks when the feature is enabled
#[cfg(feature = "benchmarks")]
pub use self::benchmark_functions::*;

#[cfg(feature = "benchmarks")]
mod benchmark_functions {
    use super::*;
    use std::hint::black_box;

    /// Benchmark data generator for different biological patterns
    pub struct BenchmarkDataGenerator {
        rng: StdRng,
    }

    impl BenchmarkDataGenerator {
        pub fn new(seed: u64) -> Self {
            Self {
                rng: StdRng::seed_from_u64(seed),
            }
        }

        /// Generate random binary data
        pub fn generate_random_data(&mut self, size: usize) -> Vec<u8> {
            (0..size).map(|_| self.rng.gen()).collect()
        }

        /// Generate text-like data (common in databases)
        pub fn generate_text_data(&mut self, size: usize) -> Vec<u8> {
            let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?";
            (0..size)
                .map(|_| chars[self.rng.gen_range(0..chars.len())])
                .collect()
        }

        /// Generate JSON-like structured data
        pub fn generate_json_data(&mut self, size: usize) -> Vec<u8> {
            let mut data = Vec::with_capacity(size);
            let patterns = [
                b"{\"id\":" as &[u8],
                b",\"name\":\"" as &[u8],
                b"\",\"value\":" as &[u8],
                b",\"timestamp\":\"" as &[u8],
                b"\"}" as &[u8],
            ];

            while data.len() < size {
                let pattern = patterns[self.rng.gen_range(0..patterns.len())];
                data.extend_from_slice(pattern);

                // Add some random content
                for _ in 0..self.rng.gen_range(5..20) {
                    if data.len() >= size {
                        break;
                    }
                    data.push(self.rng.gen_range(b'0'..=b'9'));
                }
            }

            data.truncate(size);
            data
        }

        /// Generate repetitive data with patterns
        pub fn generate_repetitive_data(&mut self, size: usize) -> Vec<u8> {
            let pattern = b"PATTERN123";
            let mut data = Vec::with_capacity(size);

            while data.len() < size {
                data.extend_from_slice(pattern);
                // Add some variation
                if self.rng.gen_bool(0.1) {
                    data.push(self.rng.gen());
                }
            }

            data.truncate(size);
            data
        }
    }

    /// Benchmark DNA compression performance
    pub fn benchmark_dna_compression(c: &mut Criterion) {
        let mut generator = BenchmarkDataGenerator::new(42);
        let sizes = vec![1024, 8192, 65536]; // Reduced for faster testing

        let mut group = c.benchmark_group("dna_compression");

        for size in sizes {
            group.throughput(Throughput::Bytes(size as u64));

            let data = generator.generate_text_data(size);
            let compressor = QuantumDNACompressor::new();

            group.bench_with_input(
                BenchmarkId::new("compress", format!("{}KB", size / 1024)),
                &data,
                |b, data| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    b.iter(|| {
                        rt.block_on(async {
                            black_box(compressor.compress(black_box(data)).await.unwrap())
                        })
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark DNA decompression performance
    pub fn benchmark_dna_decompression(c: &mut Criterion) {
        let mut generator = BenchmarkDataGenerator::new(42);
        let compressor = QuantumDNACompressor::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let mut group = c.benchmark_group("dna_decompression");

        for size in [1024, 8192, 65536] {
            group.throughput(Throughput::Bytes(size as u64));

            let data = generator.generate_text_data(size);
            let compressed = rt.block_on(compressor.compress(&data)).unwrap();

            group.bench_with_input(
                BenchmarkId::new("decompress", format!("{}KB", size / 1024)),
                &compressed,
                |b, compressed| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    b.iter(|| {
                        rt.block_on(async {
                            black_box(compressor.decompress(black_box(compressed)).await.unwrap())
                        })
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark SIMD performance
    pub fn benchmark_simd_performance(_c: &mut Criterion) {
        // SIMD benchmarks would go here
        // Placeholder for now
    }

    /// Benchmark compression comparison
    pub fn benchmark_compression_comparison(_c: &mut Criterion) {
        // Comparison benchmarks would go here
        // Placeholder for now
    }

    /// Benchmark error correction
    pub fn benchmark_error_correction(_c: &mut Criterion) {
        // Error correction benchmarks would go here
        // Placeholder for now
    }

    /// Benchmark memory usage
    pub fn benchmark_memory_usage(_c: &mut Criterion) {
        // Memory usage benchmarks would go here
        // Placeholder for now
    }

    /// Benchmark parallel scaling
    pub fn benchmark_parallel_scaling(_c: &mut Criterion) {
        // Parallel scaling benchmarks would go here
        // Placeholder for now
    }

    // Criterion benchmark groups - only when benchmarks feature is enabled
    criterion_group!(
        benches,
        benchmark_dna_compression,
        benchmark_dna_decompression
    );
    criterion_main!(benches);
}

// Stub implementations when benchmarks are disabled
#[cfg(not(feature = "benchmarks"))]
pub fn run_benchmarks() {
    println!("Benchmarks are disabled. Enable with --features benchmarks");
}
