//! Example demonstration of the DNA compression system
//!
//! This example shows how to use the integrated DNA compression system
//! within the NeuroQuantumDB engine.

use neuroquantum_core::{
    NeuroQuantumDB, NeuroQuantumConfig, DNACompressionConfig,
    QuantumDNACompressor, DNACompressor, DNABase
};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::init();

    println!("🧬 DNA Compression System Demonstration");
    println!("=======================================");

    // Create DNA compression configuration
    let dna_config = DNACompressionConfig {
        error_correction_strength: 32,
        enable_simd: true,
        enable_dictionary: true,
        max_dictionary_size: 65536,
        memory_limit: 1024 * 1024 * 1024, // 1GB
        thread_count: 4,
    };

    // Create NeuroQuantumDB configuration
    let config = NeuroQuantumConfig {
        dna_compression: dna_config,
        storage_path: std::path::PathBuf::from("./demo_data"),
        memory_limit_gb: 8,
        enable_quantum_optimization: true,
        enable_neuromorphic_learning: true,
    };

    // Initialize the database
    let db = NeuroQuantumDB::with_config(config);

    // Demo 1: Basic DNA base encoding
    demo_basic_encoding().await?;

    // Demo 2: Compression of different data types
    demo_compression_types(&db).await?;

    // Demo 3: Performance benchmarking
    demo_performance_benchmark().await?;

    // Demo 4: Error correction demonstration
    demo_error_correction().await?;

    println!("\n✅ DNA Compression System demonstration completed successfully!");

    Ok(())
}

async fn demo_basic_encoding() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Demo 1: Basic DNA Base Encoding");
    println!("----------------------------------");

    // Show quaternary encoding
    let test_byte = 0b10110011u8; // Example byte
    println!("Original byte: {:08b} ({})", test_byte, test_byte);

    // Convert to DNA bases
    let mut bases = Vec::new();
    for shift in (0..8).step_by(2).rev() {
        let two_bits = (test_byte >> shift) & 0b11;
        let base = DNABase::from_bits(two_bits)?;
        bases.push(base);
        println!("  Bits {:02b} -> {} ({})", two_bits, base.to_char(), base.to_bits());
    }

    // Show the DNA sequence
    let dna_sequence: String = bases.iter().map(|b| b.to_char()).collect();
    println!("DNA sequence: {}", dna_sequence);

    // Verify round-trip conversion
    let mut reconstructed = 0u8;
    for (i, base) in bases.iter().enumerate() {
        let shift = 6 - (i * 2);
        reconstructed |= base.to_bits() << shift;
    }
    println!("Reconstructed: {:08b} ({})", reconstructed, reconstructed);
    println!("Round-trip success: {}", test_byte == reconstructed);

    Ok(())
}

async fn demo_compression_types(db: &NeuroQuantumDB) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🗜️ Demo 2: Compression of Different Data Types");
    println!("----------------------------------------------");

    let test_cases = vec![
        ("text", b"Hello, World! This is a test of DNA compression with repeated patterns. Hello, World!".to_vec()),
        ("json", br#"{"name":"John","age":30,"city":"New York","data":{"nested":true,"values":[1,2,3,4,5]}}"#.to_vec()),
        ("binary", (0..=255u8).cycle().take(1000).collect()),
        ("repetitive", b"ABCDABCDABCDABCD".repeat(50)),
        ("random", (0..500).map(|_| rand::random::<u8>()).collect()),
    ];

    for (name, data) in test_cases {
        println!("\nTesting {} data ({} bytes):", name, data.len());

        let start = Instant::now();

        // Store with DNA compression
        let key = format!("demo_{}", name);
        db.store_compressed(&key, &data).await?;

        let compression_time = start.elapsed();

        // Retrieve and verify
        let start = Instant::now();
        let retrieved = db.retrieve_compressed(&key).await?;
        let decompression_time = start.elapsed();

        // Verify correctness
        let is_correct = retrieved == data;
        println!("  Compression time: {:?}", compression_time);
        println!("  Decompression time: {:?}", decompression_time);
        println!("  Data integrity: {}", if is_correct { "✅" } else { "❌" });

        if !is_correct {
            println!("  ⚠️ Data mismatch detected!");
        }
    }

    Ok(())
}

async fn demo_performance_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ Demo 3: Performance Benchmarking");
    println!("----------------------------------");

    let compressor = QuantumDNACompressor::new();
    let sizes = vec![1024, 8192, 65536, 262144]; // 1KB to 256KB

    for size in sizes {
        println!("\nBenchmarking {} bytes:", size);

        // Generate test data
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        // Compression benchmark
        let start = Instant::now();
        let compressed = compressor.compress(&data).await?;
        let compression_time = start.elapsed();

        // Decompression benchmark
        let start = Instant::now();
        let decompressed = compressor.decompress(&compressed).await?;
        let decompression_time = start.elapsed();

        // Calculate metrics
        let compression_ratio = compressed.compressed_size as f64 / data.len() as f64;
        let compression_mbps = (data.len() as f64) / (compression_time.as_secs_f64() * 1024.0 * 1024.0);
        let decompression_mbps = (data.len() as f64) / (decompression_time.as_secs_f64() * 1024.0 * 1024.0);

        println!("  Compression ratio: {:.2}%", compression_ratio * 100.0);
        println!("  Compression speed: {:.2} MB/s", compression_mbps);
        println!("  Decompression speed: {:.2} MB/s", decompression_mbps);
        println!("  Data integrity: {}", if decompressed == data { "✅" } else { "❌" });
    }

    Ok(())
}

async fn demo_error_correction() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 Demo 4: Error Correction Capabilities");
    println!("----------------------------------------");

    let compressor = QuantumDNACompressor::new();
    let test_data = b"This is test data for error correction demonstration with DNA compression!";

    println!("Original data: {:?}", std::str::from_utf8(test_data).unwrap());

    // Compress the data
    let compressed = compressor.compress(test_data).await?;
    println!("Compressed size: {} bytes", compressed.compressed_size);
    println!("Error correction strength: {}", compressed.sequence.metadata.error_correction_strength);

    // Validate integrity without corruption
    let is_valid = compressor.validate(&compressed).await?;
    println!("Pre-corruption validation: {}", if is_valid { "✅" } else { "❌" });

    // Simulate some corruption (modify a few DNA bases)
    let mut corrupted = compressed.clone();
    if corrupted.sequence.bases.len() > 10 {
        // Flip a few bases to simulate transmission errors
        corrupted.sequence.bases[5] = DNABase::Adenine;
        corrupted.sequence.bases[15] = DNABase::Cytosine;
        println!("Simulated corruption in DNA sequence...");
    }

    // Try to decompress corrupted data
    match compressor.decompress(&corrupted).await {
        Ok(recovered_data) => {
            let is_recovered = recovered_data == test_data;
            println!("Error recovery: {}", if is_recovered { "✅ Successful" } else { "⚠️ Partial" });
        }
        Err(e) => {
            println!("Error recovery: ❌ Failed - {}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dna_compression_integration() {
        let config = NeuroQuantumConfig::default();
        let db = NeuroQuantumDB::with_config(config);

        let test_data = b"Integration test data for DNA compression";
        let key = "test_key";

        // Store and retrieve
        db.store_compressed(key, test_data).await.unwrap();
        let retrieved = db.retrieve_compressed(key).await.unwrap();

        assert_eq!(retrieved, test_data);
    }

    #[tokio::test]
    async fn test_compression_roundtrip() {
        let compressor = QuantumDNACompressor::new();
        let test_data = b"Test data for roundtrip verification";

        let compressed = compressor.compress(test_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(decompressed, test_data);
    }
}
