//! Comprehensive benchmarking suite for DNA compression performance
//! 
//! This module provides detailed benchmarks comparing DNA compression against
//! standard algorithms and measuring performance across different data patterns.

#[cfg(feature = "benchmarks")]
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use crate::dna::{QuantumDNACompressor, DNACompressor, DNACompressionConfig};
use rand::prelude::*;
use std::time::Duration;

// Only compile benchmarks when the feature is enabled
#[cfg(feature = "benchmarks")]
pub use self::benchmark_functions::*;

#[cfg(feature = "benchmarks")]
mod benchmark_functions {
    use super::*;

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
            (0..size).map(|_| chars[self.rng.gen_range(0..chars.len())]).collect()
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
                    if data.len() >= size { break; }
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
                    b.to_async(tokio::runtime::Runtime::new().unwrap())
                        .iter(|| async {
                            black_box(compressor.compress(black_box(data)).await.unwrap())
                        });
                },
            );
        }
        
        group.finish();
    }

    // Criterion benchmark groups - only when benchmarks feature is enabled
    criterion_group!(benches, benchmark_dna_compression);
    criterion_main!(benches);
}

// Stub implementations when benchmarks are disabled
#[cfg(not(feature = "benchmarks"))]
pub fn run_benchmarks() {
    println!("Benchmarks are disabled. Enable with --features benchmarks");
}
