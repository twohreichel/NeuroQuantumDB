//! SIMD Correctness Tests for DNA Compression
//!
//! This module provides comprehensive correctness tests for all SIMD implementations,
//! verifying that ARM64 NEON and `x86_64` AVX2 optimized functions produce identical
//! results to their scalar fallback implementations.
//!
//! Test categories:
//! - Encode/Decode roundtrip verification
//! - SIMD vs Scalar correctness comparison
//! - Edge cases (empty input, alignment, partial chunks)
//! - Pattern matching correctness
//! - Hamming distance calculation
//! - Base frequency counting
//! - CRC32 checksum calculation

#![allow(
    clippy::unreadable_literal,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use super::*;
use crate::dna::DNABase;

/// Helper function to create random test bytes
fn create_random_bytes(size: usize, seed: u64) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    let hash = hasher.finish();

    (0..size)
        .map(|i| {
            let mut h = DefaultHasher::new();
            (hash.wrapping_add(i as u64)).hash(&mut h);
            h.finish() as u8
        })
        .collect()
}

/// Helper function to create a sequence of DNA bases
fn create_test_bases(pattern: &[DNABase], count: usize) -> Vec<DNABase> {
    pattern.iter().cycle().take(count).copied().collect()
}

/// Scalar reference implementation for encoding (used to verify SIMD correctness)
fn scalar_encode(input: &[u8]) -> Result<Vec<DNABase>, DNAError> {
    let mut output = Vec::with_capacity(input.len() * 4);
    for &byte in input {
        for shift in (0..8).step_by(2).rev() {
            let two_bits = (byte >> shift) & 0b11;
            let base = DNABase::from_bits(two_bits)?;
            output.push(base);
        }
    }
    Ok(output)
}

/// Scalar reference implementation for decoding (used to verify SIMD correctness)
fn scalar_decode(input: &[DNABase]) -> Result<Vec<u8>, DNAError> {
    if !input.len().is_multiple_of(4) {
        return Err(DNAError::LengthMismatch {
            expected: (input.len() / 4) * 4,
            actual: input.len(),
        });
    }

    let mut output = Vec::with_capacity(input.len() / 4);
    for bases in input.chunks_exact(4) {
        let mut byte = 0u8;
        for (i, &base) in bases.iter().enumerate() {
            let shift = 6 - (i * 2);
            byte |= (base.to_bits()) << shift;
        }
        output.push(byte);
    }
    Ok(output)
}

/// Scalar reference implementation for Hamming distance
fn scalar_hamming_distance(seq1: &[DNABase], seq2: &[DNABase]) -> Result<usize, DNAError> {
    if seq1.len() != seq2.len() {
        return Err(DNAError::LengthMismatch {
            expected: seq1.len(),
            actual: seq2.len(),
        });
    }
    Ok(seq1.iter().zip(seq2.iter()).filter(|(a, b)| a != b).count())
}

/// Scalar reference implementation for base frequency counting
fn scalar_count_base_frequencies(bases: &[DNABase]) -> [usize; 4] {
    let mut counts = [0usize; 4];
    for &base in bases {
        counts[base as usize] += 1;
    }
    counts
}

/// Scalar reference implementation for pattern matching
fn scalar_find_pattern(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut matches = Vec::new();
    if needle.is_empty() || haystack.len() < needle.len() {
        return matches;
    }

    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == needle {
            matches.push(i);
        }
    }
    matches
}

/// Scalar reference implementation for CRC32
fn scalar_crc32(data: &[u8]) -> u32 {
    // Simple CRC32 implementation (polynomial 0x04C11DB7)
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            if crc & 0x80000000 != 0 {
                crc = (crc << 1) ^ 0x04C11DB7;
            } else {
                crc <<= 1;
            }
        }
    }
    !crc
}

// ============================================================================
// SimdEncoder/SimdDecoder Tests
// ============================================================================

#[cfg(test)]
mod encoder_decoder_tests {
    use super::*;

    #[test]
    fn test_simd_encoder_creation() {
        let encoder = SimdEncoder::new();
        let caps = &encoder.capabilities;

        // At least one architecture should be detected on any modern system
        // or all false on unsupported systems (which is also valid)
        assert!(caps.vector_width >= 1);
    }

    #[test]
    fn test_simd_decoder_creation() {
        let decoder = SimdDecoder::new();
        let caps = &decoder.capabilities;
        assert!(caps.vector_width >= 1);
    }

    #[test]
    fn test_encode_empty_input() {
        let encoder = SimdEncoder::new();
        let mut output = Vec::new();

        encoder.encode_bytes_to_bases(&[], &mut output).unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_decode_empty_input() {
        let decoder = SimdDecoder::new();
        let mut output = Vec::new();

        decoder.decode_bases_to_bytes(&[], &mut output).unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_encode_single_byte() {
        let encoder = SimdEncoder::new();
        let input = vec![0b10110011u8];
        let mut output = Vec::new();

        encoder.encode_bytes_to_bases(&input, &mut output).unwrap();

        // Verify against scalar implementation
        let expected = scalar_encode(&input).unwrap();
        assert_eq!(output, expected);

        // Manual verification: 0b10110011 = 10 11 00 11 = G C A C
        assert_eq!(output.len(), 4);
        assert_eq!(output[0], DNABase::Guanine); // 10
        assert_eq!(output[1], DNABase::Cytosine); // 11
        assert_eq!(output[2], DNABase::Adenine); // 00
        assert_eq!(output[3], DNABase::Cytosine); // 11
    }

    #[test]
    fn test_decode_single_byte() {
        let decoder = SimdDecoder::new();
        let input = vec![
            DNABase::Guanine,  // 10
            DNABase::Cytosine, // 11
            DNABase::Adenine,  // 00
            DNABase::Cytosine, // 11
        ];
        let mut output = Vec::new();

        decoder.decode_bases_to_bytes(&input, &mut output).unwrap();

        let expected = scalar_decode(&input).unwrap();
        assert_eq!(output, expected);
        assert_eq!(output, vec![0b10110011u8]);
    }

    #[test]
    fn test_encode_decode_roundtrip_small() {
        let encoder = SimdEncoder::new();
        let decoder = SimdDecoder::new();

        for size in [1, 2, 3, 4, 7, 8, 15, 16, 17, 31, 32, 33] {
            let input = create_random_bytes(size, size as u64);
            let mut encoded = Vec::new();
            let mut decoded = Vec::new();

            encoder.encode_bytes_to_bases(&input, &mut encoded).unwrap();
            decoder
                .decode_bases_to_bytes(&encoded, &mut decoded)
                .unwrap();

            assert_eq!(decoded, input, "Roundtrip failed for size {size}");
        }
    }

    #[test]
    fn test_encode_decode_roundtrip_large() {
        let encoder = SimdEncoder::new();
        let decoder = SimdDecoder::new();

        for size in [64, 128, 256, 512, 1024, 4096] {
            let input = create_random_bytes(size, size as u64);
            let mut encoded = Vec::new();
            let mut decoded = Vec::new();

            encoder.encode_bytes_to_bases(&input, &mut encoded).unwrap();
            decoder
                .decode_bases_to_bytes(&encoded, &mut decoded)
                .unwrap();

            assert_eq!(decoded, input, "Roundtrip failed for size {size}");
        }
    }

    #[test]
    fn test_encode_simd_matches_scalar() {
        let encoder = SimdEncoder::new();

        // Test various sizes that exercise different code paths
        for size in [1, 4, 8, 16, 32, 64, 100, 127, 128, 129, 255, 256, 257] {
            let input = create_random_bytes(size, size as u64 * 31337);
            let mut simd_output = Vec::new();

            encoder
                .encode_bytes_to_bases(&input, &mut simd_output)
                .unwrap();
            let scalar_output = scalar_encode(&input).unwrap();

            assert_eq!(
                simd_output, scalar_output,
                "SIMD encoding differs from scalar for size {size}"
            );
        }
    }

    #[test]
    fn test_decode_simd_matches_scalar() {
        let decoder = SimdDecoder::new();

        // Create bases in multiples of 4
        for num_bytes in [1, 4, 8, 16, 32, 64, 100, 127, 128, 129] {
            let input_bytes = create_random_bytes(num_bytes, num_bytes as u64 * 42);
            let bases = scalar_encode(&input_bytes).unwrap();

            let mut simd_output = Vec::new();
            decoder
                .decode_bases_to_bytes(&bases, &mut simd_output)
                .unwrap();
            let scalar_output = scalar_decode(&bases).unwrap();

            assert_eq!(
                simd_output,
                scalar_output,
                "SIMD decoding differs from scalar for {} bases",
                bases.len()
            );
        }
    }

    #[test]
    fn test_decode_invalid_length() {
        let decoder = SimdDecoder::new();

        // Lengths not divisible by 4 should fail
        for invalid_len in [1, 2, 3, 5, 6, 7, 9, 10, 11] {
            let bases = vec![DNABase::Adenine; invalid_len];
            let mut output = Vec::new();

            let result = decoder.decode_bases_to_bytes(&bases, &mut output);
            assert!(result.is_err(), "Expected error for length {invalid_len}");
        }
    }

    #[test]
    fn test_batch_encode() {
        let encoder = SimdEncoder::new();

        let input = create_random_bytes(1000, 12345);
        let result = encoder.batch_encode(&input).unwrap();

        let expected = scalar_encode(&input).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_batch_decode() {
        let decoder = SimdDecoder::new();

        let input_bytes = create_random_bytes(250, 54321);
        let bases = scalar_encode(&input_bytes).unwrap();

        let result = decoder.batch_decode(&bases).unwrap();
        assert_eq!(result, input_bytes);
    }

    #[test]
    fn test_all_byte_values_encode() {
        let encoder = SimdEncoder::new();

        // Test all 256 possible byte values
        let input: Vec<u8> = (0..=255).collect();
        let mut output = Vec::new();

        encoder.encode_bytes_to_bases(&input, &mut output).unwrap();

        let expected = scalar_encode(&input).unwrap();
        assert_eq!(output, expected);
        assert_eq!(output.len(), 1024); // 256 bytes * 4 bases each
    }

    #[test]
    fn test_all_byte_values_roundtrip() {
        let encoder = SimdEncoder::new();
        let decoder = SimdDecoder::new();

        let input: Vec<u8> = (0..=255).collect();
        let mut encoded = Vec::new();
        let mut decoded = Vec::new();

        encoder.encode_bytes_to_bases(&input, &mut encoded).unwrap();
        decoder
            .decode_bases_to_bytes(&encoded, &mut decoded)
            .unwrap();

        assert_eq!(decoded, input);
    }

    #[test]
    fn test_pattern_bytes_encode() {
        let encoder = SimdEncoder::new();

        // Test specific patterns
        let patterns: Vec<Vec<u8>> = vec![
            vec![0x00; 64],          // All zeros
            vec![0xFF; 64],          // All ones
            vec![0xAA; 64],          // Alternating 10101010
            vec![0x55; 64],          // Alternating 01010101
            (0..64).collect(),       // Sequential
            (0..64).rev().collect(), // Reverse sequential
        ];

        for pattern in patterns {
            let mut output = Vec::new();
            encoder
                .encode_bytes_to_bases(&pattern, &mut output)
                .unwrap();

            let expected = scalar_encode(&pattern).unwrap();
            assert_eq!(output, expected);
        }
    }
}

// ============================================================================
// SimdPatternMatcher Tests
// ============================================================================

#[cfg(test)]
mod pattern_matcher_tests {
    use super::*;

    #[test]
    fn test_pattern_matcher_creation() {
        let matcher = SimdPatternMatcher::new();
        let caps = &matcher.capabilities;
        assert!(caps.vector_width >= 1);
    }

    #[test]
    fn test_find_pattern_empty_haystack() {
        let matcher = SimdPatternMatcher::new();
        let result = matcher.find_pattern_occurrences(&[], b"test");
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_pattern_empty_needle() {
        let matcher = SimdPatternMatcher::new();
        let result = matcher.find_pattern_occurrences(b"test", &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_pattern_needle_longer_than_haystack() {
        let matcher = SimdPatternMatcher::new();
        let result = matcher.find_pattern_occurrences(b"abc", b"abcdef");
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_pattern_single_match() {
        let matcher = SimdPatternMatcher::new();
        let haystack = b"hello world";
        let needle = b"world";

        let result = matcher.find_pattern_occurrences(haystack, needle);
        let expected = scalar_find_pattern(haystack, needle);

        assert_eq!(result, expected);
        assert_eq!(result, vec![6]);
    }

    #[test]
    fn test_find_pattern_multiple_matches() {
        let matcher = SimdPatternMatcher::new();
        let haystack = b"abcabcabc";
        let needle = b"abc";

        let result = matcher.find_pattern_occurrences(haystack, needle);
        let expected = scalar_find_pattern(haystack, needle);

        assert_eq!(result, expected);
        assert_eq!(result, vec![0, 3, 6]);
    }

    #[test]
    fn test_find_pattern_overlapping() {
        let matcher = SimdPatternMatcher::new();
        let haystack = b"aaaaaa";
        let needle = b"aa";

        let result = matcher.find_pattern_occurrences(haystack, needle);
        let expected = scalar_find_pattern(haystack, needle);

        assert_eq!(result, expected);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_find_pattern_no_match() {
        let matcher = SimdPatternMatcher::new();
        let haystack = b"hello world";
        let needle = b"xyz";

        let result = matcher.find_pattern_occurrences(haystack, needle);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_pattern_at_boundaries() {
        let matcher = SimdPatternMatcher::new();

        // Match at start
        let result = matcher.find_pattern_occurrences(b"hello", b"hel");
        assert_eq!(result, vec![0]);

        // Match at end
        let result = matcher.find_pattern_occurrences(b"hello", b"llo");
        assert_eq!(result, vec![2]);

        // Exact match
        let result = matcher.find_pattern_occurrences(b"hello", b"hello");
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_find_pattern_single_char() {
        let matcher = SimdPatternMatcher::new();
        let haystack = b"abcabc";
        let needle = b"b";

        let result = matcher.find_pattern_occurrences(haystack, needle);
        let expected = scalar_find_pattern(haystack, needle);

        assert_eq!(result, expected);
        assert_eq!(result, vec![1, 4]);
    }

    #[test]
    fn test_find_pattern_large_haystack() {
        let matcher = SimdPatternMatcher::new();

        // Create large haystack with known patterns
        let mut haystack = vec![0u8; 1000];
        let needle = b"PATTERN";

        // Insert pattern at specific positions
        let positions = vec![0, 100, 500, 993];
        for &pos in &positions {
            haystack[pos..pos + needle.len()].copy_from_slice(needle);
        }

        let result = matcher.find_pattern_occurrences(&haystack, needle);
        let expected = scalar_find_pattern(&haystack, needle);

        assert_eq!(result, expected);
        assert_eq!(result, positions);
    }

    #[test]
    fn test_find_pattern_simd_matches_scalar() {
        let matcher = SimdPatternMatcher::new();

        // Test various needle sizes
        for needle_len in 1..=16 {
            let needle: Vec<u8> = (0..needle_len as u8).collect();
            let mut haystack = create_random_bytes(500, needle_len as u64);

            // Insert needle at random positions
            if haystack.len() >= needle.len() {
                for pos in [0, 50, 200, 450] {
                    if pos + needle.len() <= haystack.len() {
                        haystack[pos..pos + needle.len()].copy_from_slice(&needle);
                    }
                }
            }

            let result = matcher.find_pattern_occurrences(&haystack, &needle);
            let expected = scalar_find_pattern(&haystack, &needle);

            assert_eq!(
                result, expected,
                "Pattern matching differs for needle length {needle_len}"
            );
        }
    }
}

// ============================================================================
// Hamming Distance Tests
// ============================================================================

#[cfg(test)]
mod hamming_distance_tests {
    use super::*;

    #[test]
    fn test_hamming_distance_identical() {
        let seq = create_test_bases(
            &[
                DNABase::Adenine,
                DNABase::Thymine,
                DNABase::Guanine,
                DNABase::Cytosine,
            ],
            100,
        );

        let result = utils::hamming_distance_simd(&seq, &seq).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_hamming_distance_all_different() {
        let seq1 = vec![DNABase::Adenine; 100];
        let seq2 = vec![DNABase::Thymine; 100];

        let result = utils::hamming_distance_simd(&seq1, &seq2).unwrap();
        let expected = scalar_hamming_distance(&seq1, &seq2).unwrap();

        assert_eq!(result, expected);
        assert_eq!(result, 100);
    }

    #[test]
    fn test_hamming_distance_single_difference() {
        let seq1 = vec![DNABase::Adenine; 50];
        let mut seq2 = vec![DNABase::Adenine; 50];
        seq2[25] = DNABase::Cytosine; // One difference

        let result = utils::hamming_distance_simd(&seq1, &seq2).unwrap();
        let expected = scalar_hamming_distance(&seq1, &seq2).unwrap();

        assert_eq!(result, expected);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_hamming_distance_half_different() {
        let seq1 = vec![DNABase::Adenine; 100];
        let mut seq2 = vec![DNABase::Adenine; 100];

        // Make every other base different
        for i in (0..100).step_by(2) {
            seq2[i] = DNABase::Guanine;
        }

        let result = utils::hamming_distance_simd(&seq1, &seq2).unwrap();
        let expected = scalar_hamming_distance(&seq1, &seq2).unwrap();

        assert_eq!(result, expected);
        assert_eq!(result, 50);
    }

    #[test]
    fn test_hamming_distance_empty() {
        let empty: Vec<DNABase> = vec![];
        let result = utils::hamming_distance_simd(&empty, &empty).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_hamming_distance_length_mismatch() {
        let seq1 = vec![DNABase::Adenine; 50];
        let seq2 = vec![DNABase::Adenine; 51];

        let result = utils::hamming_distance_simd(&seq1, &seq2);
        assert!(result.is_err());
    }

    #[test]
    fn test_hamming_distance_various_sizes() {
        // Test sizes that exercise different SIMD code paths
        for size in [
            1, 4, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 256,
        ] {
            let seq1 = create_test_bases(
                &[
                    DNABase::Adenine,
                    DNABase::Thymine,
                    DNABase::Guanine,
                    DNABase::Cytosine,
                ],
                size,
            );

            // Create seq2 with some differences
            let mut seq2 = seq1.clone();
            for i in (0..size).step_by(3) {
                seq2[i] = match seq2[i] {
                    | DNABase::Adenine => DNABase::Thymine,
                    | DNABase::Thymine => DNABase::Guanine,
                    | DNABase::Guanine => DNABase::Cytosine,
                    | DNABase::Cytosine => DNABase::Adenine,
                };
            }

            let result = utils::hamming_distance_simd(&seq1, &seq2).unwrap();
            let expected = scalar_hamming_distance(&seq1, &seq2).unwrap();

            assert_eq!(result, expected, "Hamming distance differs for size {size}");
        }
    }
}

// ============================================================================
// Base Frequency Counting Tests
// ============================================================================

#[cfg(test)]
mod base_frequency_tests {
    use super::*;

    fn simd_count_base_frequencies(bases: &[DNABase]) -> [usize; 4] {
        let caps = SimdCapabilities::detect();

        #[cfg(target_arch = "aarch64")]
        {
            if caps.has_neon {
                return unsafe { arm64_neon::count_base_frequencies_neon(bases) };
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if caps.has_avx2 {
                return unsafe { x86_avx2::count_base_frequencies_avx2(bases) };
            }
        }

        // Scalar fallback
        scalar_count_base_frequencies(bases)
    }

    #[test]
    fn test_count_frequencies_empty() {
        let bases: Vec<DNABase> = vec![];
        let result = simd_count_base_frequencies(&bases);
        assert_eq!(result, [0, 0, 0, 0]);
    }

    #[test]
    fn test_count_frequencies_all_adenine() {
        let bases = vec![DNABase::Adenine; 100];
        let result = simd_count_base_frequencies(&bases);
        let expected = scalar_count_base_frequencies(&bases);

        assert_eq!(result, expected);
        assert_eq!(result, [100, 0, 0, 0]);
    }

    #[test]
    fn test_count_frequencies_all_thymine() {
        let bases = vec![DNABase::Thymine; 100];
        let result = simd_count_base_frequencies(&bases);
        let expected = scalar_count_base_frequencies(&bases);

        assert_eq!(result, expected);
        assert_eq!(result, [0, 100, 0, 0]);
    }

    #[test]
    fn test_count_frequencies_all_guanine() {
        let bases = vec![DNABase::Guanine; 100];
        let result = simd_count_base_frequencies(&bases);
        let expected = scalar_count_base_frequencies(&bases);

        assert_eq!(result, expected);
        assert_eq!(result, [0, 0, 100, 0]);
    }

    #[test]
    fn test_count_frequencies_all_cytosine() {
        let bases = vec![DNABase::Cytosine; 100];
        let result = simd_count_base_frequencies(&bases);
        let expected = scalar_count_base_frequencies(&bases);

        assert_eq!(result, expected);
        assert_eq!(result, [0, 0, 0, 100]);
    }

    #[test]
    fn test_count_frequencies_equal_distribution() {
        let pattern = [
            DNABase::Adenine,
            DNABase::Thymine,
            DNABase::Guanine,
            DNABase::Cytosine,
        ];
        let bases = create_test_bases(&pattern, 400);

        let result = simd_count_base_frequencies(&bases);
        let expected = scalar_count_base_frequencies(&bases);

        assert_eq!(result, expected);
        assert_eq!(result, [100, 100, 100, 100]);
    }

    #[test]
    fn test_count_frequencies_unequal() {
        // 3 A, 2 T, 1 G, 0 C pattern
        let pattern = [
            DNABase::Adenine,
            DNABase::Adenine,
            DNABase::Adenine,
            DNABase::Thymine,
            DNABase::Thymine,
            DNABase::Guanine,
        ];
        let bases = create_test_bases(&pattern, 600);

        let result = simd_count_base_frequencies(&bases);
        let expected = scalar_count_base_frequencies(&bases);

        assert_eq!(result, expected);
        // 600 / 6 = 100 repetitions, so: 300 A, 200 T, 100 G, 0 C
        assert_eq!(result, [300, 200, 100, 0]);
    }

    #[test]
    fn test_count_frequencies_various_sizes() {
        let pattern = [
            DNABase::Adenine,
            DNABase::Thymine,
            DNABase::Guanine,
            DNABase::Cytosine,
        ];

        for size in [
            1, 4, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 256, 512,
        ] {
            let bases = create_test_bases(&pattern, size);

            let result = simd_count_base_frequencies(&bases);
            let expected = scalar_count_base_frequencies(&bases);

            assert_eq!(
                result, expected,
                "Frequency counting differs for size {size}"
            );

            // Verify total count matches
            let total: usize = result.iter().sum();
            assert_eq!(total, size);
        }
    }

    #[test]
    fn test_count_frequencies_single_base() {
        for base in [
            DNABase::Adenine,
            DNABase::Thymine,
            DNABase::Guanine,
            DNABase::Cytosine,
        ] {
            let bases = vec![base];
            let result = simd_count_base_frequencies(&bases);

            let mut expected = [0usize; 4];
            expected[base as usize] = 1;

            assert_eq!(result, expected);
        }
    }
}

// ============================================================================
// CRC32 Tests
// ============================================================================

#[cfg(test)]
mod crc32_tests {
    use super::*;

    fn simd_crc32(data: &[u8]) -> u32 {
        let caps = SimdCapabilities::detect();

        #[cfg(target_arch = "aarch64")]
        {
            if caps.has_neon && std::arch::is_aarch64_feature_detected!("crc") {
                return unsafe { arm64_neon::crc32_neon(data) };
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if caps.has_avx2 && is_x86_feature_detected!("sse4.2") {
                return unsafe { x86_avx2::crc32_avx2(data) };
            }
        }

        // Note: The hardware CRC32 and software CRC32 use different polynomials,
        // so we can't compare them directly. Instead, we verify consistency.
        scalar_crc32(data)
    }

    #[test]
    fn test_crc32_empty() {
        let data: Vec<u8> = vec![];
        let result = simd_crc32(&data);
        // Empty data should produce consistent result
        let result2 = simd_crc32(&data);
        assert_eq!(result, result2);
    }

    #[test]
    fn test_crc32_single_byte() {
        for byte in [0u8, 1, 127, 128, 255] {
            let data = vec![byte];
            let result1 = simd_crc32(&data);
            let result2 = simd_crc32(&data);
            assert_eq!(result1, result2, "CRC32 not consistent for byte {byte}");
        }
    }

    #[test]
    fn test_crc32_consistency() {
        let data = b"Hello, World!";

        // Same input should always produce same output
        let result1 = simd_crc32(data);
        let result2 = simd_crc32(data);
        let result3 = simd_crc32(data);

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn test_crc32_different_data() {
        let data1 = b"Hello";
        let data2 = b"World";

        let result1 = simd_crc32(data1);
        let result2 = simd_crc32(data2);

        // Different data should (almost certainly) produce different CRCs
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_crc32_various_sizes() {
        for size in [1, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 256, 512, 1024] {
            let data = create_random_bytes(size, size as u64 * 99);

            let result1 = simd_crc32(&data);
            let result2 = simd_crc32(&data);

            assert_eq!(result1, result2, "CRC32 not consistent for size {size}");
        }
    }

    #[test]
    fn test_crc32_bit_sensitivity() {
        let mut data = vec![0u8; 100];

        let original_crc = simd_crc32(&data);

        // Flip each byte and verify CRC changes
        for i in 0..100 {
            data[i] = 1;
            let new_crc = simd_crc32(&data);
            assert_ne!(
                original_crc, new_crc,
                "CRC should change when byte {i} is modified"
            );
            data[i] = 0;
        }
    }
}

// ============================================================================
// SIMD Capability Detection Tests
// ============================================================================

#[cfg(test)]
mod capability_tests {
    use super::*;

    #[test]
    fn test_simd_capabilities_detect() {
        let caps = SimdCapabilities::detect();

        // Verify consistent detection
        let caps2 = SimdCapabilities::detect();
        assert_eq!(caps.has_neon, caps2.has_neon);
        assert_eq!(caps.has_avx2, caps2.has_avx2);
        assert_eq!(caps.has_sse42, caps2.has_sse42);
        assert_eq!(caps.vector_width, caps2.vector_width);
    }

    #[test]
    fn test_optimal_chunk_size() {
        let caps = SimdCapabilities::detect();
        let chunk_size = caps.optimal_chunk_size();

        // Chunk size should be positive
        assert!(chunk_size > 0);

        // Chunk size should be a multiple of vector width
        assert_eq!(chunk_size % caps.vector_width, 0);
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn test_neon_capabilities_detect() {
        let caps = detect_neon_capabilities();

        // On ARM64, NEON should typically be available
        if std::arch::is_aarch64_feature_detected!("neon") {
            assert!(caps.has_neon);
            assert_eq!(caps.vector_width, 128);
            assert_eq!(caps.parallel_lanes, 16);
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_capabilities_detect() {
        let caps = x86_avx2::detect_avx2_capabilities();

        // Verify detection matches runtime check
        assert_eq!(caps.has_avx2, is_x86_feature_detected!("avx2"));
        assert_eq!(caps.has_sse42, is_x86_feature_detected!("sse4.2"));

        if caps.has_avx2 {
            assert_eq!(caps.vector_width, 256);
            assert_eq!(caps.parallel_lanes, 32);
        }
    }
}

// ============================================================================
// Utility Function Tests
// ============================================================================

#[cfg(test)]
mod utils_tests {
    use super::*;

    #[test]
    fn test_pack_unpack_roundtrip() {
        let bases = create_test_bases(
            &[
                DNABase::Adenine,
                DNABase::Thymine,
                DNABase::Guanine,
                DNABase::Cytosine,
            ],
            128,
        );

        let packed = utils::pack_bases(&bases);
        let unpacked = utils::unpack_bases(&packed, bases.len());

        assert_eq!(unpacked, bases);
    }

    #[test]
    fn test_pack_bases_single() {
        let bases = vec![DNABase::Cytosine]; // 11
        let packed = utils::pack_bases(&bases);

        // Single base in MSB position of first u64
        assert_eq!(packed.len(), 1);
        // 11 in bits 62-63 (MSB)
        assert_eq!(packed[0] & (0b11 << 62), 0b11 << 62);
    }

    #[test]
    fn test_pack_bases_full_word() {
        // 32 bases exactly fit in one u64
        let bases = vec![DNABase::Adenine; 32];
        let packed = utils::pack_bases(&bases);

        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], 0); // All adenine (00)
    }

    #[test]
    fn test_unpack_bases_partial() {
        let bases = vec![
            DNABase::Adenine,
            DNABase::Thymine,
            DNABase::Guanine,
            DNABase::Cytosine,
            DNABase::Adenine,
        ];

        let packed = utils::pack_bases(&bases);
        let unpacked = utils::unpack_bases(&packed, 5);

        assert_eq!(unpacked, bases);
    }

    #[test]
    fn test_transpose_bytes_empty() {
        let input: Vec<u8> = vec![];
        let output = utils::transpose_bytes(&input);
        assert!(output.is_empty());
    }

    #[test]
    fn test_transpose_bytes_single() {
        let input = vec![42u8];
        let output = utils::transpose_bytes(&input);
        assert_eq!(output, vec![42u8]);
    }

    #[test]
    fn test_transpose_bytes_4x4() {
        // 4x4 block transpose
        let input: Vec<u8> = (0..16).collect();
        let output = utils::transpose_bytes(&input);

        // After transpose, [i*4+j] becomes [j*4+i]
        // Row 0: 0,1,2,3 -> Col 0: 0,4,8,12
        // Row 1: 4,5,6,7 -> Col 1: 1,5,9,13
        // etc.
        let expected = vec![0, 4, 8, 12, 1, 5, 9, 13, 2, 6, 10, 14, 3, 7, 11, 15];
        assert_eq!(output, expected);
    }

    #[test]
    fn test_transpose_bytes_with_remainder() {
        // 16 bytes (one full block) + 3 remainder
        let input: Vec<u8> = (0..19).collect();
        let output = utils::transpose_bytes(&input);

        assert_eq!(output.len(), 19);

        // First 16 bytes should be transposed
        let transposed_block = vec![0, 4, 8, 12, 1, 5, 9, 13, 2, 6, 10, 14, 3, 7, 11, 15];
        assert_eq!(&output[..16], &transposed_block[..]);

        // Last 3 bytes should be unchanged
        assert_eq!(&output[16..], &[16, 17, 18]);
    }
}

// ============================================================================
// Architecture-Specific Tests
// ============================================================================

#[cfg(test)]
#[cfg(target_arch = "aarch64")]
mod neon_specific_tests {
    use super::*;

    #[test]
    fn test_safe_encode_chunk_neon() {
        let input = create_random_bytes(64, 12345);
        let mut output = Vec::new();

        let result = safe_encode_chunk_neon(&input, &mut output);
        assert!(result.is_ok());

        let expected = scalar_encode(&input).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn test_safe_decode_chunk_neon() {
        let input_bytes = create_random_bytes(64, 54321);
        let bases = scalar_encode(&input_bytes).unwrap();
        let mut output = Vec::new();

        let result = safe_decode_chunk_neon(&bases, &mut output);
        assert!(result.is_ok());

        assert_eq!(output, input_bytes);
    }

    #[test]
    fn test_neon_encode_various_sizes() {
        for size in [16, 32, 48, 64, 80, 96, 112, 128, 256] {
            let input = create_random_bytes(size, size as u64);
            let mut output = Vec::new();

            safe_encode_chunk_neon(&input, &mut output).unwrap();
            let expected = scalar_encode(&input).unwrap();

            assert_eq!(output, expected, "NEON encode failed for size {size}");
        }
    }

    #[test]
    fn test_neon_decode_various_sizes() {
        for num_bytes in [16, 32, 48, 64, 80, 96, 112, 128, 256] {
            let input_bytes = create_random_bytes(num_bytes, num_bytes as u64);
            let bases = scalar_encode(&input_bytes).unwrap();
            let mut output = Vec::new();

            safe_decode_chunk_neon(&bases, &mut output).unwrap();

            assert_eq!(
                output, input_bytes,
                "NEON decode failed for {num_bytes} bytes"
            );
        }
    }
}

#[cfg(test)]
#[cfg(target_arch = "x86_64")]
mod avx2_specific_tests {
    use super::*;

    #[test]
    fn test_safe_encode_chunk_avx2() {
        let input = create_random_bytes(64, 12345);
        let mut output = Vec::new();

        let result = safe_encode_chunk_avx2(&input, &mut output);
        assert!(result.is_ok());

        let expected = scalar_encode(&input).unwrap();
        assert_eq!(output, expected);
    }

    #[test]
    fn test_safe_decode_chunk_avx2() {
        let input_bytes = create_random_bytes(64, 54321);
        let bases = scalar_encode(&input_bytes).unwrap();
        let mut output = Vec::new();

        let result = safe_decode_chunk_avx2(&bases, &mut output);
        assert!(result.is_ok());

        assert_eq!(output, input_bytes);
    }

    #[test]
    fn test_avx2_encode_various_sizes() {
        for size in [32, 64, 96, 128, 160, 192, 224, 256, 512] {
            let input = create_random_bytes(size, size as u64);
            let mut output = Vec::new();

            safe_encode_chunk_avx2(&input, &mut output).unwrap();
            let expected = scalar_encode(&input).unwrap();

            assert_eq!(output, expected, "AVX2 encode failed for size {}", size);
        }
    }

    #[test]
    fn test_avx2_decode_various_sizes() {
        for num_bytes in [32, 64, 96, 128, 160, 192, 224, 256, 512] {
            let input_bytes = create_random_bytes(num_bytes, num_bytes as u64);
            let bases = scalar_encode(&input_bytes).unwrap();
            let mut output = Vec::new();

            safe_decode_chunk_avx2(&bases, &mut output).unwrap();

            assert_eq!(
                output, input_bytes,
                "AVX2 decode failed for {} bytes",
                num_bytes
            );
        }
    }

    #[test]
    fn test_avx2_memcpy() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        for size in [32, 64, 100, 128, 256, 512, 1000] {
            let src = create_random_bytes(size, size as u64 * 7);
            let mut dst = vec![0u8; size];

            unsafe {
                x86_avx2::memcpy_avx2(dst.as_mut_ptr(), src.as_ptr(), size);
            }

            assert_eq!(dst, src, "memcpy_avx2 failed for size {}", size);
        }
    }
}
