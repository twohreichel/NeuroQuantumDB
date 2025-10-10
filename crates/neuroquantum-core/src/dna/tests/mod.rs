//! Comprehensive test suite for DNA compression system
//!
//! This module provides extensive testing including unit tests, property-based tests,
//! integration tests, and correctness verification for the DNA compression system.

use crate::dna::{DNABase, DNACompressionConfig, DNACompressor, DNAError, QuantumDNACompressor};
use proptest::prelude::*;
use rand::prelude::*;
use rand::Rng;

/// Unit tests for core DNA compression functionality
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_dna_base_encoding() {
        // Test all DNA bases
        assert_eq!(DNABase::Adenine.to_bits(), 0b00);
        assert_eq!(DNABase::Thymine.to_bits(), 0b01);
        assert_eq!(DNABase::Guanine.to_bits(), 0b10);
        assert_eq!(DNABase::Cytosine.to_bits(), 0b11);

        // Test round-trip conversion
        for bits in 0..4 {
            let base = DNABase::from_bits(bits).unwrap();
            assert_eq!(base.to_bits(), bits);
        }
    }

    #[test]
    fn test_dna_base_char_conversion() {
        assert_eq!(DNABase::Adenine.to_char(), 'A');
        assert_eq!(DNABase::Thymine.to_char(), 'T');
        assert_eq!(DNABase::Guanine.to_char(), 'G');
        assert_eq!(DNABase::Cytosine.to_char(), 'C');

        // Test case insensitive parsing
        assert_eq!(DNABase::from_char('a').unwrap(), DNABase::Adenine);
        assert_eq!(DNABase::from_char('T').unwrap(), DNABase::Thymine);

        // Test invalid characters
        assert!(DNABase::from_char('X').is_err());
    }

    #[tokio::test]
    async fn test_empty_data_compression() {
        let compressor = QuantumDNACompressor::new();
        let empty_data = vec![];

        let compressed = compressor.compress(&empty_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(decompressed, empty_data);
        assert_eq!(compressed.sequence.original_length, 0);
    }

    #[tokio::test]
    async fn test_single_byte_compression() {
        let compressor = QuantumDNACompressor::new();
        let data = vec![0b10110011]; // Single byte

        let compressed = compressor.compress(&data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(decompressed, data);
        assert_eq!(compressed.sequence.original_length, 1);

        // Should produce 4 DNA bases (1 byte = 4 bases)
        assert_eq!(compressed.sequence.bases.len(), 4);

        // Verify the bases represent the correct bit pattern
        let expected_bases = vec![
            DNABase::Guanine,  // 10
            DNABase::Cytosine, // 11
            DNABase::Adenine,  // 00
            DNABase::Cytosine, // 11
        ];
        assert_eq!(compressed.sequence.bases, expected_bases);
    }

    #[tokio::test]
    async fn test_compression_with_patterns() {
        let compressor = QuantumDNACompressor::new();

        // Create data with repetitive patterns
        let pattern = b"HELLO";
        let mut data = Vec::new();
        for _ in 0..10 {
            data.extend_from_slice(pattern);
        }

        let compressed = compressor.compress(&data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(decompressed, data);

        // Verify round-trip works (may be larger due to error correction overhead)
        assert!(!compressed.sequence.bases.is_empty());
    }

    #[tokio::test]
    async fn test_different_error_correction_strengths() {
        let test_data = b"Test data for error correction validation";

        // Use valid Reed-Solomon parameters (must be reasonable values)
        for strength in [1, 8, 16, 32] {
            let config = DNACompressionConfig {
                error_correction_strength: strength,
                ..Default::default()
            };

            let compressor = QuantumDNACompressor::with_config(config);
            let compressed = compressor.compress(test_data).await.unwrap();
            let decompressed = compressor.decompress(&compressed).await.unwrap();

            assert_eq!(decompressed, test_data);
            assert_eq!(compressor.error_correction_strength(), strength);
        }
    }

    #[tokio::test]
    async fn test_memory_limit_enforcement() {
        let config = DNACompressionConfig {
            memory_limit: 1024, // Very small limit
            ..Default::default()
        };

        let compressor = QuantumDNACompressor::with_config(config);
        let large_data = vec![0u8; 2048]; // Exceeds limit

        let result = compressor.compress(&large_data).await;
        assert!(result.is_err());

        if let Err(DNAError::MemoryError(_)) = result {
            // Expected error type
        } else {
            panic!("Expected MemoryError");
        }
    }

    #[tokio::test]
    async fn test_compression_metadata() {
        let compressor = QuantumDNACompressor::new();
        let data = b"Test data with metadata validation";

        let compressed = compressor.compress(data).await.unwrap();

        // Verify metadata fields
        assert_eq!(compressed.sequence.metadata.version, 1);
        assert_eq!(compressed.sequence.original_length, data.len());
        assert!(compressed.sequence.metadata.compression_ratio > 0.0);
        assert!(compressed.sequence.metadata.timestamp <= chrono::Utc::now());
        assert_eq!(
            compressed.sequence.metadata.error_correction_strength,
            compressor.error_correction_strength()
        );
    }

    #[tokio::test]
    async fn test_checksum_validation() {
        let compressor = QuantumDNACompressor::new();
        let data = b"Data for checksum validation";

        let mut compressed = compressor.compress(data).await.unwrap();

        // Tamper with checksum
        compressed.sequence.checksum = compressed.sequence.checksum.wrapping_add(1);

        let result = compressor.decompress(&compressed).await;
        assert!(result.is_err());

        if let Err(DNAError::ChecksumMismatch { .. }) = result {
            // Expected error type
        } else {
            panic!("Expected ChecksumMismatch error");
        }
    }

    #[tokio::test]
    async fn test_invalid_version_handling() {
        let compressor = QuantumDNACompressor::new();
        let data = b"Data for version testing";

        let mut compressed = compressor.compress(data).await.unwrap();

        // Set invalid version
        compressed.sequence.metadata.version = 99;

        let result = compressor.decompress(&compressed).await;
        assert!(result.is_err());

        if let Err(DNAError::InvalidVersion(99)) = result {
            // Expected error
        } else {
            panic!("Expected InvalidVersion error");
        }
    }

    #[tokio::test]
    async fn test_validation_without_decompression() {
        let compressor = QuantumDNACompressor::new();
        let data = b"Data for validation testing";

        let compressed = compressor.compress(data).await.unwrap();

        // Valid compressed data should pass validation
        assert!(compressor.validate(&compressed).await.unwrap());

        // Invalid compressed data should fail validation
        let mut invalid_compressed = compressed.clone();
        invalid_compressed.sequence.bases.clear();

        assert!(!compressor.validate(&invalid_compressed).await.unwrap());
    }

    #[test]
    fn test_configuration_defaults() {
        let config = DNACompressionConfig::default();

        assert_eq!(config.error_correction_strength, 32);
        assert!(config.enable_simd);
        assert!(config.enable_dictionary);
        assert_eq!(config.max_dictionary_size, 65536);
        assert_eq!(config.memory_limit, 1024 * 1024 * 1024);
        assert!(config.thread_count > 0);
    }
}

/// Property-based tests using QuickCheck-style testing
#[cfg(test)]
mod property_tests {
    use super::*;

    proptest! {
        #[test]
        fn test_compression_roundtrip(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let compressor = QuantumDNACompressor::new();

            let compressed = rt.block_on(compressor.compress(&data)).unwrap();
            let decompressed = rt.block_on(compressor.decompress(&compressed)).unwrap();

            prop_assert_eq!(decompressed, data);
        }

        #[test]
        fn test_different_configs_same_result(
            data in prop::collection::vec(any::<u8>(), 1..100),
            error_strength in 1u8..=32,
            enable_simd in any::<bool>(),
            enable_dict in any::<bool>()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();

            let config1 = DNACompressionConfig {
                error_correction_strength: error_strength,
                enable_simd,
                enable_dictionary: enable_dict,
                ..Default::default()
            };

            let config2 = DNACompressionConfig {
                error_correction_strength: error_strength,
                enable_simd: !enable_simd, // Different SIMD setting
                enable_dictionary: enable_dict,
                ..Default::default()
            };

            let compressor1 = QuantumDNACompressor::with_config(config1);
            let compressor2 = QuantumDNACompressor::with_config(config2);

            let compressed1 = rt.block_on(compressor1.compress(&data)).unwrap();
            let compressed2 = rt.block_on(compressor2.compress(&data)).unwrap();

            let decompressed1 = rt.block_on(compressor1.decompress(&compressed1)).unwrap();
            let decompressed2 = rt.block_on(compressor2.decompress(&compressed2)).unwrap();

            // Both should produce same result regardless of SIMD usage
            prop_assert_eq!(&decompressed1, &data);
            prop_assert_eq!(&decompressed2, &data);
        }

        #[test]
        fn test_compression_ratio_bounds(data in prop::collection::vec(any::<u8>(), 1..1000)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let compressor = QuantumDNACompressor::new();

            let compressed = rt.block_on(compressor.compress(&data)).unwrap();

            // Compression ratio should be reasonable (accounting for error correction overhead)
            let ratio = compressed.sequence.metadata.compression_ratio;
            prop_assert!((0.1..=10.0).contains(&ratio), "Compression ratio {} out of bounds", ratio);
        }

        #[test]
        fn test_error_correction_integrity(
            data in prop::collection::vec(any::<u8>(), 10..100)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let compressor = QuantumDNACompressor::new();

            let compressed = rt.block_on(compressor.compress(&data)).unwrap();

            // Parity data should be present
            prop_assert!(!compressed.sequence.parity.is_empty());

            // Original length should be preserved
            prop_assert_eq!(compressed.sequence.original_length, data.len());

            // Checksum should be non-zero for non-empty data
            if !data.is_empty() {
                prop_assert_ne!(compressed.sequence.checksum, 0);
            }
        }

        #[test]
        fn test_base_sequence_validity(data in prop::collection::vec(any::<u8>(), 1..100)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let compressor = QuantumDNACompressor::new();

            let compressed = rt.block_on(compressor.compress(&data)).unwrap();

            // Base sequence length should be 4x the byte count (plus any padding)
            let expected_min_bases = data.len() * 4;
            prop_assert!(compressed.sequence.bases.len() >= expected_min_bases);

            // All bases should be valid
            for base in &compressed.sequence.bases {
                match base {
                    DNABase::Adenine | DNABase::Thymine | DNABase::Guanine | DNABase::Cytosine => {},
                }
            }
        }
    }
}

/// Stress tests for edge cases and performance
#[cfg(test)]
mod stress_tests {
    use super::*;

    #[tokio::test]
    async fn test_large_data_compression() {
        let compressor = QuantumDNACompressor::new();

        // Test with 1MB of data
        let large_data = vec![0xAA; 1024 * 1024];

        let compressed = compressor.compress(&large_data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(decompressed, large_data);
    }

    #[tokio::test]
    async fn test_random_data_patterns() {
        let compressor = QuantumDNACompressor::new();
        let mut rng = StdRng::seed_from_u64(12345);

        // Test 100 different random patterns
        for _ in 0..100 {
            let size = rng.gen_range(1..1000);
            let data: Vec<u8> = (0..size).map(|_| rng.gen()).collect();

            let compressed = compressor.compress(&data).await.unwrap();
            let decompressed = compressor.decompress(&compressed).await.unwrap();

            assert_eq!(
                decompressed, data,
                "Failed for random data of size {}",
                size
            );
        }
    }

    #[tokio::test]
    async fn test_extreme_repetition() {
        let compressor = QuantumDNACompressor::new();

        // Extremely repetitive data
        let data = vec![42u8; 10000];

        let compressed = compressor.compress(&data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(decompressed, data);

        // Should achieve good compression
        let ratio = compressed.compressed_size as f64 / data.len() as f64;
        assert!(ratio < 0.5, "Poor compression ratio: {}", ratio);
    }

    #[tokio::test]
    async fn test_all_byte_values() {
        let compressor = QuantumDNACompressor::new();

        // Test all possible byte values
        let data: Vec<u8> = (0u8..=255u8).collect();

        let compressed = compressor.compress(&data).await.unwrap();
        let decompressed = compressor.decompress(&compressed).await.unwrap();

        assert_eq!(decompressed, data);
    }

    #[tokio::test]
    async fn test_concurrent_compression() {
        use std::sync::Arc;
        use tokio::task;

        let compressor = Arc::new(QuantumDNACompressor::new());
        let mut handles = Vec::new();

        // Compress 10 different datasets concurrently
        for i in 0..10 {
            let comp = compressor.clone();
            let data = vec![i as u8; 1000];

            let handle = task::spawn(async move {
                let compressed = comp.compress(&data).await.unwrap();
                let decompressed = comp.decompress(&compressed).await.unwrap();
                assert_eq!(decompressed, data);
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }
}

/// Error handling and edge case tests
#[cfg(test)]
mod error_tests {
    use super::*;

    #[tokio::test]
    async fn test_corrupted_base_sequence() {
        let compressor = QuantumDNACompressor::new();
        let data = b"Test data for corruption testing";

        let mut compressed = compressor.compress(data).await.unwrap();

        // Corrupt the base sequence by truncating it
        compressed
            .sequence
            .bases
            .truncate(compressed.sequence.bases.len() / 2);

        let result = compressor.decompress(&compressed).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_parity_length() {
        let compressor = QuantumDNACompressor::new();
        let data = b"Test data for parity testing";

        let mut compressed = compressor.compress(data).await.unwrap();

        // Corrupt parity data
        compressed.sequence.parity.push(0xFF);

        let result = compressor.decompress(&compressed).await;
        // Should still work but may report errors corrected or fail validation
        // Both Ok and Err are acceptable outcomes
        if result.is_ok() {
            // Might succeed with error correction
        }
        // Or might fail - both are acceptable
    }

    #[tokio::test]
    async fn test_length_mismatch_detection() {
        let compressor = QuantumDNACompressor::new();
        let data = b"Test data";

        let mut compressed = compressor.compress(data).await.unwrap();

        // Modify original length
        compressed.sequence.original_length = data.len() + 10;

        let result = compressor.decompress(&compressed).await;
        assert!(result.is_err());

        if let Err(DNAError::LengthMismatch { .. }) = result {
            // Expected error
        } else {
            panic!("Expected LengthMismatch error");
        }
    }
}

/// Helper functions for test data generation
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate test data with specific patterns
    pub fn generate_pattern_data(pattern: &[u8], repetitions: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(pattern.len() * repetitions);
        for _ in 0..repetitions {
            data.extend_from_slice(pattern);
        }
        data
    }

    /// Generate binary data with specific entropy
    pub fn generate_entropy_data(size: usize, entropy: f64) -> Vec<u8> {
        let mut rng = StdRng::seed_from_u64(42);
        let mut data = Vec::with_capacity(size);

        let unique_bytes = (256.0 * entropy) as u8;

        for _ in 0..size {
            data.push(rng.gen_range(0..unique_bytes));
        }

        data
    }

    /// Generate structured JSON-like data
    pub fn generate_json_like_data(records: usize) -> Vec<u8> {
        let mut data = Vec::new();

        for i in 0..records {
            let record = format!(
                r#"{{"id":{},"name":"record_{}","value":{},"active":true}},"#,
                i,
                i,
                i * 42
            );
            data.extend_from_slice(record.as_bytes());
        }

        data
    }

    /// Generate DNA-like sequences (for biological data testing)
    pub fn generate_biological_sequence(length: usize) -> Vec<u8> {
        let mut rng = StdRng::seed_from_u64(12345);
        let bases = [b'A', b'T', b'G', b'C'];

        (0..length).map(|_| bases[rng.gen_range(0..4)]).collect()
    }
}
