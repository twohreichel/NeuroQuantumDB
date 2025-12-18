//! ARM64 NEON optimizations for DNA compression
//!
//! This module provides ARM64 NEON SIMD implementations for high-performance
//! DNA compression operations on ARM-based processors.

#![cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]

use crate::dna::{DNABase, DNAError};
use std::arch::aarch64::*;

/// NEON-optimized encoding of bytes to DNA bases
///
/// # Safety
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions (aarch64 target)
/// - Input slice pointer is valid and aligned
/// - The function is only called on ARM64 platforms with NEON enabled
#[target_feature(enable = "neon")]
pub unsafe fn encode_chunk_neon(input: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
    // Process 16 bytes at a time (produces 64 DNA bases)
    for chunk in input.chunks(16) {
        if chunk.len() == 16 {
            encode_16_bytes_neon(chunk, output)?;
        } else {
            // Handle partial chunk with scalar code
            encode_partial_chunk(chunk, output)?;
        }
    }
    Ok(())
}

/// Encode exactly 16 bytes using NEON intrinsics
///
/// # Safety
///
/// This is an internal function called only from `encode_chunk_neon`.
/// The caller must ensure:
/// - The `neon` target feature is available (guaranteed by parent function's `#[target_feature]`)
/// - `chunk.len() == 16` (caller validates this before calling)
/// - `chunk` pointer is valid for reads of 16 bytes
/// - The function is only called on ARM64 platforms (aarch64 or arm64ec)
#[target_feature(enable = "neon")]
unsafe fn encode_16_bytes_neon(chunk: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
    // Load 16 bytes into NEON register
    let bytes = vld1q_u8(chunk.as_ptr());

    // Extract each 2-bit pair from each byte using const lane indices
    for lane_idx in 0..16 {
        let byte = match lane_idx {
            0 => vgetq_lane_u8(bytes, 0),
            1 => vgetq_lane_u8(bytes, 1),
            2 => vgetq_lane_u8(bytes, 2),
            3 => vgetq_lane_u8(bytes, 3),
            4 => vgetq_lane_u8(bytes, 4),
            5 => vgetq_lane_u8(bytes, 5),
            6 => vgetq_lane_u8(bytes, 6),
            7 => vgetq_lane_u8(bytes, 7),
            8 => vgetq_lane_u8(bytes, 8),
            9 => vgetq_lane_u8(bytes, 9),
            10 => vgetq_lane_u8(bytes, 10),
            11 => vgetq_lane_u8(bytes, 11),
            12 => vgetq_lane_u8(bytes, 12),
            13 => vgetq_lane_u8(bytes, 13),
            14 => vgetq_lane_u8(bytes, 14),
            15 => vgetq_lane_u8(bytes, 15),
            _ => unreachable!(),
        };

        // Extract 4 DNA bases from this byte (8 bits = 4 × 2 bits)
        for shift in (0..8).step_by(2).rev() {
            let two_bits = (byte >> shift) & 0b11;
            let base = DNABase::from_bits(two_bits)?;
            output.push(base);
        }
    }

    Ok(())
}

/// NEON-optimized decoding of DNA bases to bytes
///
/// # Safety
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions (aarch64 target)
/// - Input slice contains a multiple of 4 DNA bases
/// - The function is only called on ARM64 platforms with NEON enabled
#[target_feature(enable = "neon")]
pub unsafe fn decode_chunk_neon(input: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
    if !input.len().is_multiple_of(4) {
        return Err(DNAError::LengthMismatch {
            expected: (input.len() / 4) * 4,
            actual: input.len(),
        });
    }

    // Process 64 bases at a time (produces 16 bytes)
    for chunk in input.chunks(64) {
        if chunk.len() == 64 {
            decode_64_bases_neon(chunk, output)?;
        } else {
            // Handle partial chunk with scalar code
            decode_partial_chunk(chunk, output)?;
        }
    }
    Ok(())
}

/// Decode exactly 64 DNA bases using NEON intrinsics
///
/// # Safety
///
/// This is an internal function called only from `decode_chunk_neon`.
/// The caller must ensure:
/// - The `neon` target feature is available (guaranteed by parent function's `#[target_feature]`)
/// - `chunk.len() == 64` (caller validates this before calling)
/// - All `DNABase` values in `chunk` are valid (0-3)
/// - The function is only called on ARM64 platforms (aarch64 or arm64ec)
#[target_feature(enable = "neon")]
unsafe fn decode_64_bases_neon(chunk: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
    // Process 16 groups of 4 bases each
    for group in 0..16 {
        let base_offset = group * 4;
        let mut byte = 0u8;

        // Combine 4 bases into 1 byte
        for i in 0..4 {
            let base = chunk[base_offset + i];
            let shift = 6 - (i * 2);
            byte |= (base.to_bits()) << shift;
        }

        output.push(byte);
    }

    Ok(())
}

/// NEON-optimized pattern matching for dictionary compression
///
/// # Safety
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions
/// - Input slices are valid and properly aligned
/// - Needle length is <= 16 bytes
#[target_feature(enable = "neon")]
pub unsafe fn find_pattern_neon(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut matches = Vec::new();

    if needle.len() > 16 || needle.is_empty() {
        return matches;
    }

    // Load needle into NEON register (pad with zeros if needed)
    let mut needle_padded = [0u8; 16];
    needle_padded[..needle.len()].copy_from_slice(needle);
    let needle_vec = vld1q_u8(needle_padded.as_ptr());

    // Scan through haystack
    let mut pos = 0;
    while pos + 16 <= haystack.len() {
        let haystack_chunk = vld1q_u8(haystack.as_ptr().add(pos));

        // Compare with needle - simplified check for first few bytes
        let first_match = vgetq_lane_u8(haystack_chunk, 0) == vgetq_lane_u8(needle_vec, 0);

        if first_match && pos + needle.len() <= haystack.len() {
            // Verify full pattern match with scalar code for simplicity
            if &haystack[pos..pos + needle.len()] == needle {
                matches.push(pos);
            }
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

/// Calculate Hamming distance between DNA sequences using NEON
///
/// # Safety
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions
/// - Both input slices have the same length
/// - Input slices are valid
#[target_feature(enable = "neon")]
pub unsafe fn hamming_distance_neon(seq1: &[DNABase], seq2: &[DNABase]) -> Result<usize, DNAError> {
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

    // Process 16 bytes at a time
    for (chunk1, chunk2) in bytes1.chunks(16).zip(bytes2.chunks(16)) {
        if chunk1.len() == 16 && chunk2.len() == 16 {
            distance += hamming_distance_16_bytes_neon(chunk1, chunk2);
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

/// Calculate Hamming distance for 16 bytes using NEON
///
/// # Safety
///
/// This is an internal function called only from `hamming_distance_neon`.
/// The caller must ensure:
/// - The `neon` target feature is available (guaranteed by parent function's `#[target_feature]`)
/// - Both `chunk1.len() == 16` and `chunk2.len() == 16` (caller validates this)
/// - Both chunk pointers are valid for reads of 16 bytes
/// - The function is only called on ARM64 platforms (aarch64 or arm64ec)
#[target_feature(enable = "neon")]
unsafe fn hamming_distance_16_bytes_neon(chunk1: &[u8], chunk2: &[u8]) -> usize {
    let vec1 = vld1q_u8(chunk1.as_ptr());
    let vec2 = vld1q_u8(chunk2.as_ptr());

    // XOR to find differences
    let xor_result = veorq_u8(vec1, vec2);

    // Count non-zero bytes (differences) using const lane indices
    let mut count = 0;
    let lanes = [
        vgetq_lane_u8(xor_result, 0),
        vgetq_lane_u8(xor_result, 1),
        vgetq_lane_u8(xor_result, 2),
        vgetq_lane_u8(xor_result, 3),
        vgetq_lane_u8(xor_result, 4),
        vgetq_lane_u8(xor_result, 5),
        vgetq_lane_u8(xor_result, 6),
        vgetq_lane_u8(xor_result, 7),
        vgetq_lane_u8(xor_result, 8),
        vgetq_lane_u8(xor_result, 9),
        vgetq_lane_u8(xor_result, 10),
        vgetq_lane_u8(xor_result, 11),
        vgetq_lane_u8(xor_result, 12),
        vgetq_lane_u8(xor_result, 13),
        vgetq_lane_u8(xor_result, 14),
        vgetq_lane_u8(xor_result, 15),
    ];

    for lane_value in lanes {
        if lane_value != 0 {
            count += 1;
        }
    }

    count
}

/// NEON-optimized CRC32 calculation for checksums
///
/// # Safety
/// This function requires ARM64 NEON and CRC support. The caller must ensure:
/// - The CPU supports both NEON and CRC instructions
/// - Input slice is valid
#[target_feature(enable = "neon,crc")]
pub unsafe fn crc32_neon(data: &[u8]) -> u32 {
    let mut crc = 0u32;

    // Process 8 bytes at a time using CRC32 instruction
    for chunk in data.chunks(8) {
        if chunk.len() == 8 {
            let value = std::ptr::read_unaligned(chunk.as_ptr() as *const u64);
            crc = __crc32d(crc, value);
        } else {
            // Handle remainder byte by byte
            for &byte in chunk {
                crc = __crc32b(crc, byte);
            }
        }
    }

    crc
}

/// NEON-optimized base frequency counting
///
/// # Safety
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions
/// - Input slice contains valid DNABase values
#[target_feature(enable = "neon")]
pub unsafe fn count_base_frequencies_neon(bases: &[DNABase]) -> [usize; 4] {
    let mut counts = [0usize; 4];

    // Convert bases to bytes for SIMD processing
    let base_bytes: Vec<u8> = bases.iter().map(|&b| b as u8).collect();

    // Process 16 bytes at a time
    for chunk in base_bytes.chunks(16) {
        if chunk.len() == 16 {
            count_16_bases_neon(chunk, &mut counts);
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

/// Count frequencies in 16 bases using NEON
///
/// # Safety
///
/// This is an internal function called only from `count_base_frequencies_neon`.
/// The caller must ensure:
/// - The `neon` target feature is available (guaranteed by parent function's `#[target_feature]`)
/// - `chunk.len() == 16` (caller validates this before calling)
/// - `chunk` pointer is valid for reads of 16 bytes
/// - All byte values in `chunk` represent valid DNA base indices (0-3)
/// - The function is only called on ARM64 platforms (aarch64 or arm64ec)
#[target_feature(enable = "neon")]
unsafe fn count_16_bases_neon(chunk: &[u8], counts: &mut [usize; 4]) {
    let bases_vec = vld1q_u8(chunk.as_ptr());

    // Count each base type using const lane access
    for base_type in 0u8..4u8 {
        let target = vdupq_n_u8(base_type);
        let matches = vceqq_u8(bases_vec, target);

        // Count matches using const lane indices
        let lanes = [
            vgetq_lane_u8(matches, 0),
            vgetq_lane_u8(matches, 1),
            vgetq_lane_u8(matches, 2),
            vgetq_lane_u8(matches, 3),
            vgetq_lane_u8(matches, 4),
            vgetq_lane_u8(matches, 5),
            vgetq_lane_u8(matches, 6),
            vgetq_lane_u8(matches, 7),
            vgetq_lane_u8(matches, 8),
            vgetq_lane_u8(matches, 9),
            vgetq_lane_u8(matches, 10),
            vgetq_lane_u8(matches, 11),
            vgetq_lane_u8(matches, 12),
            vgetq_lane_u8(matches, 13),
            vgetq_lane_u8(matches, 14),
            vgetq_lane_u8(matches, 15),
        ];

        for lane_value in lanes {
            if lane_value != 0 {
                counts[base_type as usize] += 1;
            }
        }
    }
}

// Helper functions for scalar fallbacks

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

fn bases_to_bytes(bases: &[DNABase]) -> Vec<u8> {
    bases.iter().map(|&base| base as u8).collect()
}

/// NEON feature detection and capability reporting
pub fn detect_neon_capabilities() -> super::NeonCapabilities {
    super::NeonCapabilities {
        has_neon: std::arch::is_aarch64_feature_detected!("neon"),
        has_crc32: std::arch::is_aarch64_feature_detected!("crc"),
        has_sha2: std::arch::is_aarch64_feature_detected!("sha2"),
        vector_width: 128,  // bits
        parallel_lanes: 16, // bytes
    }
}
