//! Fuzz target for DNA Encoder
//!
//! This fuzz target tests the DNA quaternary encoder with arbitrary binary data
//! to find encoding/compression bugs and edge cases.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neuroquantum_core::dna::encoder::QuaternaryEncoder;
use neuroquantum_core::DNACompressionConfig;

fuzz_target!(|data: &[u8]| {
    // Skip empty inputs
    if data.is_empty() {
        return;
    }

    // Create encoder with default configuration
    let config = DNACompressionConfig::default();
    let mut encoder = QuaternaryEncoder::new(&config);

    // Create a tokio runtime for async operations
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Test dictionary compression with arbitrary data
    // The encoder should never panic on any input
    rt.block_on(async {
        let _ = encoder.compress_with_dictionary(data).await;
    });

    // Test with different configurations
    let configs = [
        DNACompressionConfig {
            enable_simd: true,
            enable_dictionary: true,
            max_dictionary_size: 1024,
            error_correction_strength: 32,
            memory_limit: 1024 * 1024,
            thread_count: 1,
        },
        DNACompressionConfig {
            enable_simd: false,
            enable_dictionary: false,
            max_dictionary_size: 0,
            error_correction_strength: 0,
            memory_limit: 1024,
            thread_count: 1,
        },
    ];

    for config in configs {
        let mut encoder = QuaternaryEncoder::new(&config);
        rt.block_on(async {
            let _ = encoder.compress_with_dictionary(data).await;
        });
    }
});
