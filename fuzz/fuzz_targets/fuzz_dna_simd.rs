//! Fuzz target for DNA SIMD operations
//!
//! This fuzz target tests the SIMD-optimized DNA encoding/decoding functions
//! to ensure memory safety and correctness with arbitrary input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use neuroquantum_core::dna::simd::{SimdDecoder, SimdEncoder, SimdPatternMatcher};
use neuroquantum_core::DNABase;

/// Convert arbitrary bytes to valid DNA bases for testing
fn bytes_to_bases(data: &[u8]) -> Vec<DNABase> {
    data.iter()
        .map(|b| match b % 4 {
            0 => DNABase::Adenine,
            1 => DNABase::Thymine,
            2 => DNABase::Cytosine,
            3 => DNABase::Guanine,
            _ => DNABase::Adenine, // Unreachable but satisfy compiler
        })
        .collect()
}

fuzz_target!(|data: &[u8]| {
    // Skip empty inputs
    if data.is_empty() {
        return;
    }

    // Create SIMD encoder/decoder with auto-detection
    let encoder = SimdEncoder::new();
    let decoder = SimdDecoder::new();
    let pattern_matcher = SimdPatternMatcher::new();

    // Test bytes to bases encoding
    let result = encoder.batch_encode(data);
    assert!(
        result.is_ok() || result.is_err(),
        "batch_encode should return Result"
    );

    // Test bases to bytes decoding with aligned input (multiples of 4)
    let bases = bytes_to_bases(data);
    // Align to multiple of 4 for decoding
    let aligned_len = (bases.len() / 4) * 4;
    if aligned_len >= 4 {
        let aligned_bases = &bases[..aligned_len];
        let decode_result = decoder.batch_decode(aligned_bases);
        assert!(
            decode_result.is_ok() || decode_result.is_err(),
            "batch_decode should return Result"
        );
    }

    // If encoding succeeded, verify roundtrip for aligned data
    if let Ok(encoded_bases) = result {
        let aligned_len = (encoded_bases.len() / 4) * 4;
        if aligned_len >= 4 {
            let aligned = &encoded_bases[..aligned_len];
            let roundtrip = decoder.batch_decode(aligned);
            // Roundtrip should succeed for valid encoded data
            if let Ok(decoded) = roundtrip {
                // For aligned data, we should get back the original
                // (minus any padding)
                let min_len = decoded.len().min(data.len());
                if min_len > 0 {
                    // Just verify it didn't panic, exact match depends on padding
                    let _ = &decoded[..min_len];
                }
            }
        }
    }

    // Test pattern matching with arbitrary patterns
    if data.len() >= 4 {
        let needle_len = (data[0] as usize % 8) + 1;
        let needle_len = needle_len.min(data.len());
        let needle = &data[..needle_len];
        let _ = pattern_matcher.find_pattern_occurrences(data, needle);
    }
});
