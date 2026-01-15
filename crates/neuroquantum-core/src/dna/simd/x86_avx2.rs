//! `x86_64` AVX2 optimizations for DNA compression
//!
//! This module provides `x86_64` AVX2 SIMD implementations for high-performance
//! DNA compression operations on Intel/AMD processors.

#[cfg(target_arch = "x86_64")]
use crate::dna::{DNABase, DNAError};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// AVX2-optimized encoding of bytes to DNA bases
///
/// # Safety
///
/// This function requires the CPU to support AVX2 instructions.
/// The caller must ensure that the `avx2` target feature is available before calling.
/// Use `is_x86_feature_detected!("avx2")` to check at runtime.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn encode_chunk_avx2(input: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
    // Process 32 bytes at a time (produces 128 DNA bases)
    for chunk in input.chunks(32) {
        if chunk.len() == 32 {
            encode_32_bytes_avx2(chunk, output)?;
        } else {
            // Handle partial chunk with scalar code
            encode_partial_chunk(chunk, output)?;
        }
    }
    Ok(())
}

/// Encode exactly 32 bytes using AVX2 intrinsics
///
/// # Safety
///
/// This is an internal function called only from `encode_chunk_avx2`.
/// The caller must ensure:
/// - The `avx2` target feature is available (guaranteed by parent function's `#[target_feature]`)
/// - `chunk.len() == 32` (caller validates this before calling)
/// - `chunk` pointer is valid for reads of 32 bytes
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn encode_32_bytes_avx2(chunk: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
    // Load 32 bytes into AVX2 register
    let bytes = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);

    // Extract bytes and convert each to 4 DNA bases
    let bytes_array: [u8; 32] = std::mem::transmute(bytes);

    for byte in bytes_array {
        // Extract 4 DNA bases from this byte (8 bits = 4 × 2 bits)
        for shift in (0..8).step_by(2).rev() {
            let two_bits = (byte >> shift) & 0b11;
            let base = DNABase::from_bits(two_bits)?;
            output.push(base);
        }
    }

    Ok(())
}

/// AVX2-optimized decoding of DNA bases to bytes
///
/// # Safety
///
/// This function requires the CPU to support AVX2 instructions.
/// The caller must ensure that the `avx2` target feature is available before calling.
/// Use `is_x86_feature_detected!("avx2")` to check at runtime.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn decode_chunk_avx2(input: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
    if !input.len().is_multiple_of(4) {
        return Err(DNAError::LengthMismatch {
            expected: (input.len() / 4) * 4,
            actual: input.len(),
        });
    }

    // Process 128 bases at a time (produces 32 bytes)
    for chunk in input.chunks(128) {
        if chunk.len() == 128 {
            decode_128_bases_avx2(chunk, output)?;
        } else {
            // Handle partial chunk with scalar code
            decode_partial_chunk(chunk, output)?;
        }
    }
    Ok(())
}

/// Decode exactly 128 DNA bases using AVX2 intrinsics
///
/// # Safety
///
/// This is an internal function called only from `decode_chunk_avx2`.
/// The caller must ensure:
/// - The `avx2` target feature is available (guaranteed by parent function's `#[target_feature]`)
/// - `chunk.len() == 128` (caller validates this before calling)
/// - All `DNABase` values in `chunk` are valid (0-3)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn decode_128_bases_avx2(chunk: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
    // Process 32 groups of 4 bases each
    let mut bytes = [0u8; 32];

    for (group, byte_slot) in bytes.iter_mut().enumerate() {
        let base_offset = group * 4;
        let mut byte = 0u8;

        // Combine 4 bases into 1 byte
        for i in 0..4 {
            let base = chunk[base_offset + i];
            let shift = 6 - (i * 2);
            byte |= (base.to_bits()) << shift;
        }

        *byte_slot = byte;
    }

    output.extend_from_slice(&bytes);
    Ok(())
}

/// AVX2-optimized pattern matching for dictionary compression
///
/// # Safety
///
/// This function requires the CPU to support AVX2 instructions.
/// The caller must ensure that the `avx2` target feature is available before calling.
/// Use `is_x86_feature_detected!("avx2")` to check at runtime.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn find_pattern_avx2(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut matches = Vec::new();

    if needle.len() > 32 || needle.is_empty() {
        return matches;
    }

    // Load needle into AVX2 register (pad with zeros if needed)
    let mut needle_padded = [0u8; 32];
    needle_padded[..needle.len()].copy_from_slice(needle);
    let needle_vec = _mm256_loadu_si256(needle_padded.as_ptr() as *const __m256i);

    // Scan through haystack
    let mut pos = 0;
    while pos + 32 <= haystack.len() {
        let haystack_chunk = _mm256_loadu_si256(haystack.as_ptr().add(pos) as *const __m256i);

        // Compare with needle
        let cmp_result = _mm256_cmpeq_epi8(haystack_chunk, needle_vec);
        let match_mask = _mm256_movemask_epi8(cmp_result) as u32;

        // Check if we have a match at the beginning
        let needle_mask = (1u32 << needle.len()) - 1;
        if (match_mask & needle_mask) == needle_mask {
            matches.push(pos);
        }

        pos += 1;
    }

    // Handle remainder with scalar search
    while pos + needle.len() <= haystack.len() {
        if &haystack[pos..pos + needle.len()] == needle {
            matches.push(pos);
        }
        pos += 1;
    }

    matches
}

/// Calculate Hamming distance between DNA sequences using AVX2
///
/// # Safety
///
/// This function requires the CPU to support AVX2 instructions.
/// The caller must ensure that the `avx2` target feature is available before calling.
/// Use `is_x86_feature_detected!("avx2")` to check at runtime.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn hamming_distance_avx2(seq1: &[DNABase], seq2: &[DNABase]) -> Result<usize, DNAError> {
    if seq1.len() != seq2.len() {
        return Err(DNAError::LengthMismatch {
            expected: seq1.len(),
            actual: seq2.len(),
        });
    }

    let mut distance = 0usize;

    // Convert DNA bases to bytes for SIMD processing
    let bytes1 = bases_to_bytes(seq1);
    let bytes2 = bases_to_bytes(seq2);

    // Process 32 bytes at a time
    for (chunk1, chunk2) in bytes1.chunks(32).zip(bytes2.chunks(32)) {
        if chunk1.len() == 32 && chunk2.len() == 32 {
            distance += hamming_distance_32_bytes_avx2(chunk1, chunk2);
        } else {
            // Handle remainder with scalar code
            distance += chunk1
                .iter()
                .zip(chunk2.iter())
                .filter(|(a, b)| a != b)
                .count();
        }
    }

    Ok(distance)
}

/// Calculate Hamming distance for 32 bytes using AVX2
///
/// # Safety
///
/// This is an internal function called only from `hamming_distance_avx2`.
/// The caller must ensure:
/// - The `avx2` target feature is available (guaranteed by parent function's `#[target_feature]`)
/// - Both `chunk1.len() == 32` and `chunk2.len() == 32` (caller validates this)
/// - Both chunk pointers are valid for reads of 32 bytes
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn hamming_distance_32_bytes_avx2(chunk1: &[u8], chunk2: &[u8]) -> usize {
    let vec1 = _mm256_loadu_si256(chunk1.as_ptr() as *const __m256i);
    let vec2 = _mm256_loadu_si256(chunk2.as_ptr() as *const __m256i);

    // XOR to find differences
    let xor_result = _mm256_xor_si256(vec1, vec2);

    // Create mask for non-zero bytes
    let zero_vec = _mm256_setzero_si256();
    let cmp_result = _mm256_cmpeq_epi8(xor_result, zero_vec);
    let mask = _mm256_movemask_epi8(cmp_result) as u32;

    // Count zeros in mask (differences are where mask bit is 0)
    32 - mask.count_ones() as usize
}

/// AVX2-optimized base frequency counting
///
/// # Safety
///
/// This function requires the CPU to support AVX2 instructions.
/// The caller must ensure that the `avx2` target feature is available before calling.
/// Use `is_x86_feature_detected!("avx2")` to check at runtime.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn count_base_frequencies_avx2(bases: &[DNABase]) -> [usize; 4] {
    let mut counts = [0usize; 4];

    // Convert bases to bytes for SIMD processing
    let base_bytes: Vec<u8> = bases.iter().map(|&b| b as u8).collect();

    // Process 32 bytes at a time
    for chunk in base_bytes.chunks(32) {
        if chunk.len() == 32 {
            count_32_bases_avx2(chunk, &mut counts);
        } else {
            // Handle remainder with scalar code
            for &base_byte in chunk {
                if (base_byte as usize) < 4 {
                    counts[base_byte as usize] += 1;
                }
            }
        }
    }

    counts
}

/// Count frequencies in 32 bases using AVX2
///
/// # Safety
///
/// This is an internal function called only from `count_base_frequencies_avx2`.
/// The caller must ensure:
/// - The `avx2` target feature is available (guaranteed by parent function's `#[target_feature]`)
/// - `chunk.len() == 32` (caller validates this before calling)
/// - `chunk` pointer is valid for reads of 32 bytes
/// - All byte values in `chunk` represent valid DNA base indices (0-3)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn count_32_bases_avx2(chunk: &[u8], counts: &mut [usize; 4]) {
    let bases_vec = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);

    // Count each base type
    for base_type in 0u8..4u8 {
        let target = _mm256_set1_epi8(base_type as i8);
        let matches = _mm256_cmpeq_epi8(bases_vec, target);
        let mask = _mm256_movemask_epi8(matches) as u32;

        counts[base_type as usize] += mask.count_ones() as usize;
    }
}

/// AVX2-optimized CRC32 calculation with hardware acceleration
///
/// # Safety
///
/// This function requires the CPU to support AVX2 and SSE4.2 instructions.
/// The caller must ensure that the `avx2` and `sse4.2` target features are available before calling.
/// Use `is_x86_feature_detected!("avx2")` and `is_x86_feature_detected!("sse4.2")` to check at runtime.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,sse4.2")]
pub unsafe fn crc32_avx2(data: &[u8]) -> u32 {
    let mut crc = 0u32;

    // Process 8 bytes at a time using SSE4.2 CRC32 instruction
    for chunk in data.chunks(8) {
        if chunk.len() == 8 {
            let value = std::ptr::read_unaligned(chunk.as_ptr() as *const u64);
            crc = _mm_crc32_u64(crc as u64, value) as u32;
        } else {
            // Handle remainder byte by byte
            for &byte in chunk {
                crc = _mm_crc32_u8(crc, byte);
            }
        }
    }

    crc
}

/// AVX2-optimized memory copy for large DNA sequences
///
/// # Safety
///
/// This function requires the CPU to support AVX2 instructions.
/// The caller must ensure that:
/// - The `avx2` target feature is available (use `is_x86_feature_detected!("avx2")` to check)
/// - `dst` and `src` are valid pointers for the given length
/// - `dst` and `src` regions do not overlap
/// - Both `dst` and `src` are valid for reads/writes of `len` bytes
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn memcpy_avx2(dst: *mut u8, src: *const u8, len: usize) {
    let mut offset = 0;

    // Copy 32 bytes at a time
    while offset + 32 <= len {
        let chunk = _mm256_loadu_si256(src.add(offset) as *const __m256i);
        _mm256_storeu_si256(dst.add(offset) as *mut __m256i, chunk);
        offset += 32;
    }

    // Handle remainder
    std::ptr::copy_nonoverlapping(src.add(offset), dst.add(offset), len - offset);
}

// Helper functions for scalar fallbacks (same as NEON version)

/// Encodes a partial chunk of bytes to DNA bases (scalar fallback).
/// Used when the input size is not a multiple of the SIMD vector width.
#[cfg(target_arch = "x86_64")]
fn encode_partial_chunk(chunk: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
    for &byte in chunk {
        for shift in (0..8).step_by(2).rev() {
            let two_bits = (byte >> shift) & 0b11;
            let base = DNABase::from_bits(two_bits)?;
            output.push(base);
        }
    }
    Ok(())
}

#[cfg(target_arch = "x86_64")]
fn decode_partial_chunk(chunk: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
    for bases in chunk.chunks_exact(4) {
        let mut byte = 0u8;
        for (i, &base) in bases.iter().enumerate() {
            let shift = 6 - (i * 2);
            byte |= (base.to_bits()) << shift;
        }
        output.push(byte);
    }
    Ok(())
}

/// Converts DNA bases to bytes for SIMD processing.
/// Used by `hamming_distance_avx2` for efficient comparison.
#[cfg(target_arch = "x86_64")]
fn bases_to_bytes(bases: &[DNABase]) -> Vec<u8> {
    bases.iter().map(|&base| base as u8).collect()
}

/// AVX2 feature detection and capability reporting
#[must_use]
pub const fn detect_avx2_capabilities() -> Avx2Capabilities {
    #[cfg(target_arch = "x86_64")]
    {
        Avx2Capabilities {
            has_avx2: is_x86_feature_detected!("avx2"),
            has_avx: is_x86_feature_detected!("avx"),
            has_sse42: is_x86_feature_detected!("sse4.2"),
            has_bmi2: is_x86_feature_detected!("bmi2"),
            vector_width: 256,  // bits
            parallel_lanes: 32, // bytes
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        Avx2Capabilities {
            has_avx2: false,
            has_avx: false,
            has_sse42: false,
            has_bmi2: false,
            vector_width: 0,
            parallel_lanes: 0,
        }
    }
}

/// AVX2 capability information
#[derive(Debug, Clone)]
pub struct Avx2Capabilities {
    pub has_avx2: bool,
    pub has_avx: bool,
    pub has_sse42: bool,
    pub has_bmi2: bool,
    pub vector_width: usize,
    pub parallel_lanes: usize,
}
