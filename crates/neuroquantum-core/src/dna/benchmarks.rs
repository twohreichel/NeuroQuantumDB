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
    use super::{
        criterion_group, criterion_main, BenchmarkId, Criterion, DNACompressor, Distribution,
        QuantumDNACompressor, Rng, SeedableRng, StdRng, Throughput,
    };
    use futures::future::join_all;
    use std::hint::black_box;

    /// Benchmark data generator for different biological patterns
    pub struct BenchmarkDataGenerator {
        rng: StdRng,
    }

    impl BenchmarkDataGenerator {
        #[must_use]
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
    pub fn benchmark_simd_performance(c: &mut Criterion) {
        let mut generator = BenchmarkDataGenerator::new(12345);
        let mut group = c.benchmark_group("simd_operations");

        // Test different data sizes
        for size in [1024, 8192, 65536] {
            group.throughput(Throughput::Bytes(size as u64));

            let data = generator.generate_random_data(size);

            // Benchmark scalar encoding (baseline)
            group.bench_with_input(
                BenchmarkId::new("scalar_encode", format!("{}KB", size / 1024)),
                &data,
                |b, data| {
                    b.iter(|| {
                        let mut output = Vec::new();
                        for &byte in data {
                            for shift in (0..8).step_by(2).rev() {
                                let two_bits = (byte >> shift) & 0b11;
                                let base = crate::dna::DNABase::from_bits(two_bits).unwrap();
                                output.push(base);
                            }
                        }
                        black_box(output)
                    });
                },
            );

            // Benchmark SIMD encoding if available
            #[cfg(target_arch = "x86_64")]
            if is_x86_feature_detected!("avx2") {
                group.bench_with_input(
                    BenchmarkId::new("avx2_encode", format!("{}KB", size / 1024)),
                    &data,
                    |b, data| {
                        b.iter(|| {
                            let mut output = Vec::new();
                            crate::dna::simd::safe_encode_chunk_avx2(data, &mut output).unwrap();
                            black_box(output)
                        });
                    },
                );
            }

            #[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
            if std::arch::is_aarch64_feature_detected!("neon") {
                group.bench_with_input(
                    BenchmarkId::new("neon_encode", format!("{}KB", size / 1024)),
                    &data,
                    |b, data| {
                        b.iter(|| {
                            let mut output = Vec::new();
                            crate::dna::simd::safe_encode_chunk_neon(data, &mut output).unwrap();
                            black_box(output)
                        });
                    },
                );
            }
        }

        group.finish();
    }

    /// Benchmark compression comparison
    pub fn benchmark_compression_comparison(c: &mut Criterion) {
        let mut generator = BenchmarkDataGenerator::new(67890);
        let compressor = QuantumDNACompressor::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let mut group = c.benchmark_group("compression_algorithms");

        let test_size = 8192;
        group.throughput(Throughput::Bytes(test_size as u64));

        // Different data patterns
        let patterns = vec![
            ("random", generator.generate_random_data(test_size)),
            ("text", generator.generate_text_data(test_size)),
            ("json", generator.generate_json_data(test_size)),
            ("repetitive", generator.generate_repetitive_data(test_size)),
        ];

        for (name, data) in patterns {
            // DNA compression
            group.bench_with_input(BenchmarkId::new("dna", name), &data, |b, data| {
                b.iter(|| {
                    rt.block_on(async {
                        black_box(compressor.compress(black_box(data)).await.unwrap())
                    })
                });
            });

            // Standard compression algorithms for comparison
            #[cfg(feature = "benchmarks")]
            {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;

                // GZIP compression
                group.bench_with_input(BenchmarkId::new("gzip", name), &data, |b, data| {
                    b.iter(|| {
                        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                        encoder.write_all(data).unwrap();
                        black_box(encoder.finish().unwrap())
                    });
                });

                // LZ4 compression
                #[cfg(feature = "benchmarks")]
                {
                    group.bench_with_input(BenchmarkId::new("lz4", name), &data, |b, data| {
                        b.iter(|| black_box(lz4_flex::compress_prepend_size(data)));
                    });
                }
            }
        }

        group.finish();
    }

    /// Benchmark error correction
    pub fn benchmark_error_correction(c: &mut Criterion) {
        use crate::dna::error_correction::ReedSolomonCorrector;

        let mut generator = BenchmarkDataGenerator::new(11111);
        let mut group = c.benchmark_group("error_correction");

        // Test different error correction strengths
        // Note: GF(2^8) limits parity_shards to max 64 to ensure data+parity <= 255
        for strength in [10u8, 32u8, 64u8] {
            let corrector = ReedSolomonCorrector::new(strength);
            let test_size = 4096;

            group.throughput(Throughput::Bytes(test_size as u64));

            let data = generator.generate_random_data(test_size);

            // Benchmark parity generation
            group.bench_with_input(
                BenchmarkId::new("generate_parity", strength),
                &data,
                |b, data| {
                    b.iter(|| black_box(corrector.generate_parity(black_box(data)).unwrap()));
                },
            );

            // Benchmark error correction with clean data
            let parity = corrector.generate_parity(&data).unwrap();
            group.bench_with_input(
                BenchmarkId::new("correct_clean", strength),
                &(data.clone(), parity.clone()),
                |b, (data, parity)| {
                    b.iter(|| {
                        black_box(
                            corrector
                                .correct_errors(black_box(data), black_box(parity))
                                .unwrap(),
                        )
                    });
                },
            );

            // Benchmark error correction with simulated errors
            let mut corrupted = data.clone();
            let max_errors = ((strength as usize) / 2).min(10);
            for i in 0..max_errors {
                if i < corrupted.len() {
                    corrupted[i] ^= 0xFF; // Flip all bits
                }
            }

            group.bench_with_input(
                BenchmarkId::new("correct_errors", strength),
                &(corrupted, parity),
                |b, (corrupted, parity)| {
                    b.iter(|| {
                        black_box(
                            corrector
                                .correct_errors(black_box(corrupted), black_box(parity))
                                .unwrap(),
                        )
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark memory usage
    pub fn benchmark_memory_usage(c: &mut Criterion) {
        let mut generator = BenchmarkDataGenerator::new(22222);
        let compressor = QuantumDNACompressor::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let mut group = c.benchmark_group("memory_efficiency");

        // Test memory efficiency with different sizes
        for size in [1024, 16384, 131072] {
            group.throughput(Throughput::Bytes(size as u64));

            let data = generator.generate_text_data(size);

            group.bench_with_input(
                BenchmarkId::new("compress_memory", format!("{}KB", size / 1024)),
                &data,
                |b, data| {
                    b.iter(|| {
                        rt.block_on(async {
                            let compressed = compressor.compress(black_box(data)).await.unwrap();
                            // Calculate memory overhead
                            let original_size = data.len();
                            let compressed_size = compressed.compressed_size;
                            let overhead = (compressed_size as f64 / original_size as f64) * 100.0;
                            black_box((compressed, overhead))
                        })
                    });
                },
            );

            // Benchmark memory during decompression
            let compressed = rt.block_on(compressor.compress(&data)).unwrap();
            group.bench_with_input(
                BenchmarkId::new("decompress_memory", format!("{}KB", size / 1024)),
                &compressed,
                |b, compressed| {
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

    /// Benchmark parallel scaling
    pub fn benchmark_parallel_scaling(c: &mut Criterion) {
        let mut generator = BenchmarkDataGenerator::new(33333);
        let compressor = QuantumDNACompressor::new();

        let mut group = c.benchmark_group("parallel_scaling");

        let test_size = 65536; // 64KB
        group.throughput(Throughput::Bytes(test_size as u64));

        let data = generator.generate_text_data(test_size);

        // Single-threaded baseline
        group.bench_with_input(
            BenchmarkId::new("single_thread", "64KB"),
            &data,
            |b, data| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap();
                b.iter(|| {
                    rt.block_on(async {
                        black_box(compressor.compress(black_box(data)).await.unwrap())
                    })
                });
            },
        );

        // Multi-threaded with different thread counts
        for threads in [2, 4, 8] {
            group.bench_with_input(
                BenchmarkId::new("multi_thread", format!("{threads}threads")),
                &data,
                |b, data| {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(threads)
                        .build()
                        .unwrap();
                    b.iter(|| {
                        rt.block_on(async {
                            black_box(compressor.compress(black_box(data)).await.unwrap())
                        })
                    });
                },
            );
        }

        // Parallel batch processing
        let batch_count = 8;
        let batch_data: Vec<_> = (0..batch_count)
            .map(|_| generator.generate_text_data(test_size / batch_count))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("parallel_batch", format!("{batch_count}batches")),
            &batch_data,
            |b, batches| {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(4)
                    .build()
                    .unwrap();
                b.iter(|| {
                    rt.block_on(async {
                        let futures: Vec<_> = batches
                            .iter()
                            .map(|data| compressor.compress(data))
                            .collect();
                        let results: Vec<_> = join_all(futures).await;
                        black_box(results)
                    })
                });
            },
        );

        group.finish();
    }

    // Criterion benchmark groups - only when benchmarks feature is enabled
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
}

// Stub implementations when benchmarks are disabled
#[cfg(not(feature = "benchmarks"))]
pub fn run_benchmarks() {
    println!("Benchmarks are disabled. Enable with --features benchmarks");
}
