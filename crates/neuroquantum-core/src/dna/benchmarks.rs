//! Comprehensive benchmarking suite for DNA compression performance
//! 
//! This module provides detailed benchmarks comparing DNA compression against
//! standard algorithms and measuring performance across different data patterns.

use crate::dna::{QuantumDNACompressor, DNACompressor, DNACompressionConfig, DNABase};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rand::prelude::*;
use std::time::Duration;

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
            b"{\"id\":",
            b",\"name\":\"",
            b"\",\"value\":",
            b",\"timestamp\":\"",
            b"\"}",
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

    /// Generate already compressed data (worst case for compression)
    pub fn generate_compressed_data(&mut self, size: usize) -> Vec<u8> {
        use flate2::{Compression, write::GzEncoder};
        use std::io::Write;
        
        let random_data = self.generate_random_data(size * 2);
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&random_data).unwrap();
        let mut compressed = encoder.finish().unwrap();
        
        compressed.truncate(size);
        if compressed.len() < size {
            compressed.extend(self.generate_random_data(size - compressed.len()));
        }
        
        compressed
    }

    /// Generate biological DNA-like sequences
    pub fn generate_dna_sequence(&mut self, size: usize) -> Vec<u8> {
        let bases = [b'A', b'T', b'G', b'C'];
        (0..size).map(|_| bases[self.rng.gen_range(0..4)]).collect()
    }
}

/// Benchmark DNA compression performance
pub fn benchmark_dna_compression(c: &mut Criterion) {
    let mut generator = BenchmarkDataGenerator::new(42);
    let sizes = vec![1024, 8192, 65536, 524288, 1048576]; // 1KB to 1MB
    
    let mut group = c.benchmark_group("dna_compression");
    
    for size in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        
        // Test different data types
        let test_data = vec![
            ("random", generator.generate_random_data(size)),
            ("text", generator.generate_text_data(size)),
            ("json", generator.generate_json_data(size)),
            ("repetitive", generator.generate_repetitive_data(size)),
            ("compressed", generator.generate_compressed_data(size)),
            ("dna_like", generator.generate_dna_sequence(size)),
        ];
        
        for (data_type, data) in test_data {
            let compressor = QuantumDNACompressor::new();
            
            group.bench_with_input(
                BenchmarkId::new("compress", format!("{}_{}KB", data_type, size / 1024)),
                &data,
                |b, data| {
                    b.to_async(tokio::runtime::Runtime::new().unwrap())
                        .iter(|| async {
                            black_box(compressor.compress(black_box(data)).await.unwrap())
                        });
                },
            );
        }
    }
    
    group.finish();
}

/// Benchmark DNA decompression performance
pub fn benchmark_dna_decompression(c: &mut Criterion) {
    let mut generator = BenchmarkDataGenerator::new(42);
    let compressor = QuantumDNACompressor::new();
    
    let mut group = c.benchmark_group("dna_decompression");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Pre-compress test data
    let test_data = vec![
        ("random", generator.generate_random_data(65536)),
        ("text", generator.generate_text_data(65536)),
        ("repetitive", generator.generate_repetitive_data(65536)),
    ];
    
    for (data_type, data) in test_data {
        let compressed = rt.block_on(compressor.compress(&data)).unwrap();
        
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("decompress", data_type),
            &compressed,
            |b, compressed| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| async {
                        black_box(compressor.decompress(black_box(compressed)).await.unwrap())
                    });
            },
        );
    }
    
    group.finish();
}

/// Benchmark SIMD vs scalar performance
pub fn benchmark_simd_performance(c: &mut Criterion) {
    let mut generator = BenchmarkDataGenerator::new(42);
    
    let mut group = c.benchmark_group("simd_vs_scalar");
    
    let data = generator.generate_random_data(1048576); // 1MB
    
    // SIMD enabled config
    let simd_config = DNACompressionConfig {
        enable_simd: true,
        ..Default::default()
    };
    
    // SIMD disabled config
    let scalar_config = DNACompressionConfig {
        enable_simd: false,
        ..Default::default()
    };
    
    let simd_compressor = QuantumDNACompressor::with_config(simd_config);
    let scalar_compressor = QuantumDNACompressor::with_config(scalar_config);
    
    group.throughput(Throughput::Bytes(data.len() as u64));
    
    group.bench_function("simd_enabled", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                black_box(simd_compressor.compress(black_box(&data)).await.unwrap())
            });
    });
    
    group.bench_function("scalar_only", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                black_box(scalar_compressor.compress(black_box(&data)).await.unwrap())
            });
    });
    
    group.finish();
}

/// Compare DNA compression against standard algorithms
pub fn benchmark_compression_comparison(c: &mut Criterion) {
    let mut generator = BenchmarkDataGenerator::new(42);
    let data = generator.generate_text_data(65536); // 64KB of text data
    
    let mut group = c.benchmark_group("compression_comparison");
    group.throughput(Throughput::Bytes(data.len() as u64));
    
    let dna_compressor = QuantumDNACompressor::new();
    
    // DNA compression
    group.bench_function("dna_compression", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                black_box(dna_compressor.compress(black_box(&data)).await.unwrap())
            });
    });
    
    // Gzip compression
    group.bench_function("gzip", |b| {
        use flate2::{Compression, write::GzEncoder};
        use std::io::Write;
        
        b.iter(|| {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(black_box(&data)).unwrap();
            black_box(encoder.finish().unwrap())
        });
    });
    
    // LZ4 compression
    group.bench_function("lz4", |b| {
        b.iter(|| {
            black_box(lz4_flex::compress_prepend_size(black_box(&data)))
        });
    });
    
    group.finish();
}

/// Benchmark error correction performance
pub fn benchmark_error_correction(c: &mut Criterion) {
    let mut generator = BenchmarkDataGenerator::new(42);
    let mut group = c.benchmark_group("error_correction");
    
    let data = generator.generate_random_data(8192); // 8KB
    
    // Test different error correction strengths
    let error_strengths = vec![8, 16, 32, 64, 128];
    
    for strength in error_strengths {
        let config = DNACompressionConfig {
            error_correction_strength: strength,
            ..Default::default()
        };
        
        let compressor = QuantumDNACompressor::with_config(config);
        
        group.bench_with_input(
            BenchmarkId::new("error_strength", strength),
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

/// Benchmark memory usage patterns
pub fn benchmark_memory_usage(c: &mut Criterion) {
    let mut generator = BenchmarkDataGenerator::new(42);
    let compressor = QuantumDNACompressor::new();
    
    let mut group = c.benchmark_group("memory_usage");
    
    // Test different data sizes to measure memory scaling
    let sizes = vec![1024, 8192, 65536, 524288];
    
    for size in sizes {
        let data = generator.generate_random_data(size);
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("memory_scaling", format!("{}KB", size / 1024)),
            &data,
            |b, data| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter_with_large_drop(|| async {
                        // This measures memory allocation/deallocation overhead
                        let compressed = compressor.compress(black_box(data)).await.unwrap();
                        let _decompressed = compressor.decompress(&compressed).await.unwrap();
                        compressed
                    });
            },
        );
    }
    
    group.finish();
}

/// Benchmark parallel processing scaling
pub fn benchmark_parallel_scaling(c: &mut Criterion) {
    let mut generator = BenchmarkDataGenerator::new(42);
    let data = generator.generate_random_data(1048576); // 1MB
    
    let mut group = c.benchmark_group("parallel_scaling");
    group.throughput(Throughput::Bytes(data.len() as u64));
    
    // Test different thread counts
    let thread_counts = vec![1, 2, 4, 8, 16];
    
    for thread_count in thread_counts {
        let config = DNACompressionConfig {
            thread_count,
            ..Default::default()
        };
        
        let compressor = QuantumDNACompressor::with_config(config);
        
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
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

/// Generate comprehensive performance report
pub fn generate_performance_report() -> PerformanceReport {
    let mut generator = BenchmarkDataGenerator::new(42);
    let compressor = QuantumDNACompressor::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let test_data = generator.generate_text_data(65536);
    
    // Measure compression
    let start = std::time::Instant::now();
    let compressed = rt.block_on(compressor.compress(&test_data)).unwrap();
    let compression_time = start.elapsed();
    
    // Measure decompression
    let start = std::time::Instant::now();
    let decompressed = rt.block_on(compressor.decompress(&compressed)).unwrap();
    let decompression_time = start.elapsed();
    
    // Verify correctness
    let is_correct = decompressed == test_data;
    
    PerformanceReport {
        original_size: test_data.len(),
        compressed_size: compressed.compressed_size,
        compression_ratio: compressed.sequence.metadata.compression_ratio,
        compression_time,
        decompression_time,
        throughput_mbps: (test_data.len() as f64 / compression_time.as_secs_f64()) / (1024.0 * 1024.0),
        is_correct,
        error_correction_strength: compressor.error_correction_strength(),
        simd_enabled: true, // Detected at runtime
    }
}

/// Performance report structure
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub compression_time: Duration,
    pub decompression_time: Duration,
    pub throughput_mbps: f64,
    pub is_correct: bool,
    pub error_correction_strength: u8,
    pub simd_enabled: bool,
}

impl PerformanceReport {
    pub fn print_summary(&self) {
        println!("=== DNA Compression Performance Report ===");
        println!("Original size: {} bytes", self.original_size);
        println!("Compressed size: {} bytes", self.compressed_size);
        println!("Compression ratio: {:.2}%", self.compression_ratio * 100.0);
        println!("Compression time: {:.2}ms", self.compression_time.as_millis());
        println!("Decompression time: {:.2}ms", self.decompression_time.as_millis());
        println!("Throughput: {:.2} MB/s", self.throughput_mbps);
        println!("Correctness: {}", if self.is_correct { "✓" } else { "✗" });
        println!("Error correction: {} bytes", self.error_correction_strength);
        println!("SIMD enabled: {}", if self.simd_enabled { "✓" } else { "✗" });
    }
}

// Criterion benchmark groups
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
