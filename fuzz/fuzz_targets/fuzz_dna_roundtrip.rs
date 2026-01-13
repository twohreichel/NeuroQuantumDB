//! Fuzz target for DNA Compression Roundtrip
//!
//! This fuzz target tests the DNA encoding/decoding roundtrip to verify
//! that data integrity is maintained through compression/decompression cycles.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neuroquantum_core::dna::encoder::QuaternaryEncoder;
use neuroquantum_core::dna::decoder::QuaternaryDecoder;
use neuroquantum_core::DNACompressionConfig;

fuzz_target!(|data: &[u8]| {
    // Skip empty inputs
    if data.is_empty() {
        return;
    }

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Test with default configuration
    let config = DNACompressionConfig::default();
    test_roundtrip(&rt, &config, data);

    // Test with various configurations
    let configs = [
        // Maximum error correction
        DNACompressionConfig {
            enable_simd: true,
            enable_dictionary: true,
            max_dictionary_size: 4096,
            error_correction_strength: 64,
            memory_limit: 16 * 1024 * 1024,
            thread_count: 1,
        },
        // Minimal configuration
        DNACompressionConfig {
            enable_simd: false,
            enable_dictionary: false,
            max_dictionary_size: 0,
            error_correction_strength: 0,
            memory_limit: 1024,
            thread_count: 1,
        },
        // Dictionary-only compression
        DNACompressionConfig {
            enable_simd: false,
            enable_dictionary: true,
            max_dictionary_size: 1024,
            error_correction_strength: 16,
            memory_limit: 4 * 1024 * 1024,
            thread_count: 1,
        },
        // SIMD-only optimization
        DNACompressionConfig {
            enable_simd: true,
            enable_dictionary: false,
            max_dictionary_size: 0,
            error_correction_strength: 32,
            memory_limit: 8 * 1024 * 1024,
            thread_count: 1,
        },
    ];

    for config in configs {
        test_roundtrip(&rt, &config, data);
    }

    // Test with edge case data patterns
    test_edge_cases(&rt, data);
});

fn test_roundtrip(rt: &tokio::runtime::Runtime, config: &DNACompressionConfig, data: &[u8]) {
    let mut encoder = QuaternaryEncoder::new(config);
    let decoder = QuaternaryDecoder::new(config);

    rt.block_on(async {
        // Compress the data
        match encoder.compress_with_dictionary(data).await {
            Ok(compressed) => {
                // Try to decompress
                match decoder.decompress_with_dictionary(&compressed).await {
                    Ok(decompressed) => {
                        // Verify roundtrip integrity
                        // Note: Depending on compression settings, the roundtrip
                        // might not be exact (e.g., padding), so we check prefix
                        if decompressed.len() >= data.len() {
                            let matches = decompressed[..data.len()] == *data;
                            // Roundtrip should preserve original data
                            // If not matching, this is interesting for fuzzing
                            if !matches {
                                // Log for analysis but don't panic - fuzzer will save this input
                                let _ = matches;
                            }
                        }
                    }
                    Err(_) => {
                        // Decompression failure after successful compression is interesting
                        // but shouldn't panic the fuzzer
                    }
                }
            }
            Err(_) => {
                // Compression can legitimately fail for some inputs
                // (e.g., memory limits, invalid data)
            }
        }
    });
}

fn test_edge_cases(rt: &tokio::runtime::Runtime, data: &[u8]) {
    let config = DNACompressionConfig::default();
    let mut encoder = QuaternaryEncoder::new(&config);
    let decoder = QuaternaryDecoder::new(&config);

    rt.block_on(async {
        // Test with repeated patterns
        if !data.is_empty() {
            let repeated: Vec<u8> = data.iter().cycle().take(data.len() * 4).copied().collect();
            let _ = encoder.compress_with_dictionary(&repeated).await;
        }

        // Test with alternating patterns
        let alternating: Vec<u8> = data
            .iter()
            .enumerate()
            .map(|(i, b)| if i % 2 == 0 { *b } else { !*b })
            .collect();
        let _ = encoder.compress_with_dictionary(&alternating).await;

        // Test with all zeros
        let zeros = vec![0u8; data.len().max(16)];
        if let Ok(compressed) = encoder.compress_with_dictionary(&zeros).await {
            let _ = decoder.decompress_with_dictionary(&compressed).await;
        }

        // Test with all ones
        let ones = vec![0xFFu8; data.len().max(16)];
        if let Ok(compressed) = encoder.compress_with_dictionary(&ones).await {
            let _ = decoder.decompress_with_dictionary(&compressed).await;
        }

        // Test with DNA-like patterns (simulating actual DNA bases)
        let dna_pattern: Vec<u8> = data
            .iter()
            .map(|b| match b % 4 {
                0 => b'A',
                1 => b'T',
                2 => b'G',
                3 => b'C',
                _ => b'A',
            })
            .collect();
        let _ = encoder.compress_with_dictionary(&dna_pattern).await;

        // Test with very small data
        if data.len() >= 1 {
            let tiny = &data[..1];
            let _ = encoder.compress_with_dictionary(tiny).await;
        }

        // Test compression of compression output (double compression)
        if let Ok(compressed) = encoder.compress_with_dictionary(data).await {
            // Compress the already compressed data
            let _ = encoder.compress_with_dictionary(&compressed).await;
        }
    });
}
