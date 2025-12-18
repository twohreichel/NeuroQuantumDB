//! ARM64 NEON SIMD optimizations for general operations
//!
//! This module provides ARM64 NEON SIMD implementations for various
//! high-performance operations including matrix multiplication, quantum operations,
//! and pattern searching.

#![cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]

use crate::error::CoreResult;
use std::arch::aarch64::*;

/// NEON-optimized DNA compression using quaternary encoding
///
/// # Safety
///
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions (aarch64 target)
/// - Use `std::arch::is_aarch64_feature_detected!("neon")` to check at runtime
#[target_feature(enable = "neon")]
pub unsafe fn neon_dna_compression(data: &[u8]) -> CoreResult<Vec<u8>> {
    let mut result = Vec::with_capacity(data.len() / 4);

    // Process 16 bytes at a time using NEON 128-bit registers
    for chunk in data.chunks_exact(16) {
        // Load 16 bytes into NEON register
        let vec = vld1q_u8(chunk.as_ptr());

        // Quaternary encode: each byte (8 bits) becomes 4 bases (2 bits each)
        // We pack 4 bases into 1 byte for 4:1 compression
        let encoded = quaternary_encode_neon(vec);

        // Store compressed result (4 bytes from 16 input bytes)
        let mut temp = [0u8; 16];
        vst1q_u8(temp.as_mut_ptr(), encoded);

        // Only take the first 4 bytes (4:1 compression ratio)
        result.extend_from_slice(&temp[..4]);
    }

    // Handle remaining bytes with scalar code
    let remainder_start = (data.len() / 16) * 16;
    if remainder_start < data.len() {
        let remainder = &data[remainder_start..];
        let scalar_result = scalar_dna_compression(remainder)?;
        result.extend_from_slice(&scalar_result);
    }

    Ok(result)
}

/// NEON-optimized quaternary encoding
/// Converts 8-bit bytes to 2-bit DNA bases (A=00, T=01, G=10, C=11)
///
/// # Safety
///
/// This function uses NEON intrinsics and must only be called when NEON is available.
#[target_feature(enable = "neon")]
unsafe fn quaternary_encode_neon(input: uint8x16_t) -> uint8x16_t {
    // Create mask for extracting 2-bit pairs
    let mask_2bit = vdupq_n_u8(0b11);

    // Shift and mask to extract quaternary digits
    let bits_6_7 = vshrq_n_u8(input, 6);
    let bits_4_5 = vshrq_n_u8(input, 4);
    let bits_2_3 = vshrq_n_u8(input, 2);
    let bits_0_1 = input;

    // Mask each to 2 bits
    let q0 = vandq_u8(bits_6_7, mask_2bit);
    let q1 = vandq_u8(bits_4_5, mask_2bit);
    let q2 = vandq_u8(bits_2_3, mask_2bit);
    let q3 = vandq_u8(bits_0_1, mask_2bit);

    // Pack 4 quaternary digits into each byte
    let packed_high = vorrq_u8(vshlq_n_u8(q0, 6), vshlq_n_u8(q1, 4));
    let packed_low = vorrq_u8(vshlq_n_u8(q2, 2), q3);

    vorrq_u8(packed_high, packed_low)
}

/// NEON-optimized matrix multiplication
///
/// # Safety
///
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions
/// - Matrix dimensions are valid
/// - Use `std::arch::is_aarch64_feature_detected!("neon")` to check at runtime
#[target_feature(enable = "neon")]
pub unsafe fn neon_matrix_multiply(
    matrix_a: &[f32],
    matrix_b: &[f32],
    rows_a: usize,
    cols_a: usize,
    cols_b: usize,
) -> CoreResult<Vec<f32>> {
    let mut result = vec![0.0f32; rows_a * cols_b];

    for i in 0..rows_a {
        for j in 0..cols_b {
            let mut sum = vdupq_n_f32(0.0);

            // Process 4 elements at a time
            let chunks = cols_a / 4;
            for k in 0..chunks {
                let k_base = k * 4;

                // Load 4 elements from row of A
                let a_vec = vld1q_f32(&matrix_a[i * cols_a + k_base]);

                // Load 4 elements from column of B
                let b_vals = [
                    matrix_b[k_base * cols_b + j],
                    matrix_b[(k_base + 1) * cols_b + j],
                    matrix_b[(k_base + 2) * cols_b + j],
                    matrix_b[(k_base + 3) * cols_b + j],
                ];
                let b_vec = vld1q_f32(b_vals.as_ptr());

                // Multiply and accumulate
                sum = vfmaq_f32(sum, a_vec, b_vec);
            }

            // Sum the 4 partial results
            let sum_array = [
                vgetq_lane_f32(sum, 0),
                vgetq_lane_f32(sum, 1),
                vgetq_lane_f32(sum, 2),
                vgetq_lane_f32(sum, 3),
            ];
            let mut total = sum_array.iter().sum::<f32>();

            // Handle remaining elements
            for k in (chunks * 4)..cols_a {
                total += matrix_a[i * cols_a + k] * matrix_b[k * cols_b + j];
            }

            result[i * cols_b + j] = total;
        }
    }

    Ok(result)
}

// Re-export QuantumOperation from parent module to avoid duplication
pub use crate::neon_optimization::QuantumOperation;

// Safe wrapper functions that perform feature detection

/// Safe wrapper for NEON DNA compression with automatic feature detection
pub fn safe_neon_dna_compression(data: &[u8]) -> CoreResult<Vec<u8>> {
    if std::arch::is_aarch64_feature_detected!("neon") {
        // SAFETY: We've checked that NEON is available
        unsafe { neon_dna_compression(data) }
    } else {
        scalar_dna_compression(data)
    }
}

/// Safe wrapper for NEON matrix multiply with automatic feature detection
pub fn safe_neon_matrix_multiply(
    matrix_a: &[f32],
    matrix_b: &[f32],
    rows_a: usize,
    cols_a: usize,
    cols_b: usize,
) -> CoreResult<Vec<f32>> {
    if std::arch::is_aarch64_feature_detected!("neon") {
        // SAFETY: We've checked that NEON is available
        unsafe { neon_matrix_multiply(matrix_a, matrix_b, rows_a, cols_a, cols_b) }
    } else {
        scalar_matrix_multiply(matrix_a, matrix_b, rows_a, cols_a, cols_b)
    }
}

/// Safe wrapper for NEON quantum operations with automatic feature detection
pub fn safe_neon_quantum_operation(
    real_parts: &mut [f32],
    imag_parts: &mut [f32],
    operation: QuantumOperation,
) -> CoreResult<()> {
    if std::arch::is_aarch64_feature_detected!("neon") {
        // SAFETY: We've checked that NEON is available
        unsafe { neon_quantum_operation(real_parts, imag_parts, operation) }
    } else {
        scalar_quantum_operation(real_parts, imag_parts, operation)
    }
}

/// Safe wrapper for NEON parallel search with automatic feature detection
pub fn safe_neon_parallel_search(haystack: &[u8], needle: &[u8]) -> CoreResult<Vec<usize>> {
    if std::arch::is_aarch64_feature_detected!("neon") {
        // SAFETY: We've checked that NEON is available
        unsafe { neon_parallel_search(haystack, needle) }
    } else {
        scalar_search(haystack, needle)
    }
}

/// NEON-optimized quantum state operations
///
/// # Safety
///
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions
/// - real_parts and imag_parts have the same length
/// - Use `std::arch::is_aarch64_feature_detected!("neon")` to check at runtime
#[target_feature(enable = "neon")]
pub unsafe fn neon_quantum_operation(
    real_parts: &mut [f32],
    imag_parts: &mut [f32],
    operation: QuantumOperation,
) -> CoreResult<()> {
    let len = real_parts.len();
    let chunks = len / 4;

    match operation {
        QuantumOperation::Normalize => {
            // Calculate magnitude squared
            let mut norm_sq = vdupq_n_f32(0.0);

            for i in 0..chunks {
                let idx = i * 4;
                let real = vld1q_f32(&real_parts[idx]);
                let imag = vld1q_f32(&imag_parts[idx]);

                let real_sq = vmulq_f32(real, real);
                let imag_sq = vmulq_f32(imag, imag);
                norm_sq = vaddq_f32(norm_sq, vaddq_f32(real_sq, imag_sq));
            }

            // Sum the 4 partial norms
            let norm_array = [
                vgetq_lane_f32(norm_sq, 0),
                vgetq_lane_f32(norm_sq, 1),
                vgetq_lane_f32(norm_sq, 2),
                vgetq_lane_f32(norm_sq, 3),
            ];
            let mut total_norm: f32 = norm_array.iter().sum();

            // Handle remainder
            for i in (chunks * 4)..len {
                total_norm += real_parts[i] * real_parts[i] + imag_parts[i] * imag_parts[i];
            }

            let norm = total_norm.sqrt();
            if norm > 1e-10 {
                let inv_norm = vdupq_n_f32(1.0 / norm);

                // Normalize all elements
                for i in 0..chunks {
                    let idx = i * 4;
                    let real = vld1q_f32(&real_parts[idx]);
                    let imag = vld1q_f32(&imag_parts[idx]);

                    let real_norm = vmulq_f32(real, inv_norm);
                    let imag_norm = vmulq_f32(imag, inv_norm);

                    vst1q_f32(&mut real_parts[idx], real_norm);
                    vst1q_f32(&mut imag_parts[idx], imag_norm);
                }

                // Handle remainder
                let inv_norm_scalar = 1.0 / norm;
                for i in (chunks * 4)..len {
                    real_parts[i] *= inv_norm_scalar;
                    imag_parts[i] *= inv_norm_scalar;
                }
            }
        }
        QuantumOperation::PhaseFlip => {
            let neg_one = vdupq_n_f32(-1.0);

            for i in 0..chunks {
                let idx = i * 4;
                let real = vld1q_f32(&real_parts[idx]);
                let imag = vld1q_f32(&imag_parts[idx]);

                vst1q_f32(&mut real_parts[idx], vmulq_f32(real, neg_one));
                vst1q_f32(&mut imag_parts[idx], vmulq_f32(imag, neg_one));
            }

            // Handle remainder
            for i in (chunks * 4)..len {
                real_parts[i] = -real_parts[i];
                imag_parts[i] = -imag_parts[i];
            }
        }
        QuantumOperation::Hadamard => {
            // Hadamard gate: H|0⟩ = (|0⟩ + |1⟩)/√2, H|1⟩ = (|0⟩ - |1⟩)/√2
            let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
            for i in 0..(len / 2) {
                let r0 = real_parts[i * 2];
                let i0 = imag_parts[i * 2];
                let r1 = real_parts[i * 2 + 1];
                let i1 = imag_parts[i * 2 + 1];

                real_parts[i * 2] = (r0 + r1) * inv_sqrt2;
                imag_parts[i * 2] = (i0 + i1) * inv_sqrt2;
                real_parts[i * 2 + 1] = (r0 - r1) * inv_sqrt2;
                imag_parts[i * 2 + 1] = (i0 - i1) * inv_sqrt2;
            }
        }
    }

    Ok(())
}

/// NEON-optimized parallel pattern search
///
/// # Safety
///
/// This function requires ARM64 NEON support. The caller must ensure:
/// - The CPU supports NEON instructions
/// - haystack and needle are valid slices
/// - Use `std::arch::is_aarch64_feature_detected!("neon")` to check at runtime
#[target_feature(enable = "neon")]
pub unsafe fn neon_parallel_search(haystack: &[u8], needle: &[u8]) -> CoreResult<Vec<usize>> {
    let mut matches = Vec::new();

    if needle.len() > 16 {
        // Fallback for large patterns
        return scalar_search(haystack, needle);
    }

    // Load first byte of needle for quick rejection
    let first_byte = vdupq_n_u8(needle[0]);

    let mut pos = 0;
    while pos + 16 <= haystack.len() {
        // Load 16 bytes from haystack
        let chunk = vld1q_u8(haystack.as_ptr().add(pos));

        // Compare with first byte of needle
        let cmp = vceqq_u8(chunk, first_byte);

        // Check which lanes matched
        let mut mask_bytes = [0u8; 16];
        vst1q_u8(mask_bytes.as_mut_ptr(), cmp);

        for (i, &mask) in mask_bytes.iter().enumerate() {
            if mask == 0xFF && pos + i + needle.len() <= haystack.len() {
                // Potential match - verify with full comparison
                if &haystack[pos + i..pos + i + needle.len()] == needle {
                    matches.push(pos + i);
                }
            }
        }

        pos += 16;
    }

    // Handle remainder with scalar search
    while pos + needle.len() <= haystack.len() {
        if &haystack[pos..pos + needle.len()] == needle {
            matches.push(pos);
        }
        pos += 1;
    }

    Ok(matches)
}

// Scalar fallback implementations

fn scalar_dna_compression(data: &[u8]) -> CoreResult<Vec<u8>> {
    let mut result = Vec::with_capacity(data.len() / 4);

    for chunk in data.chunks(4) {
        if chunk.len() == 4 {
            let mut compressed = 0u8;
            for (i, &byte) in chunk.iter().enumerate() {
                let shift = 6 - (i * 2);
                compressed |= ((byte >> 6) & 0b11) << shift;
            }
            result.push(compressed);
        }
    }

    Ok(result)
}

fn scalar_search(haystack: &[u8], needle: &[u8]) -> CoreResult<Vec<usize>> {
    let mut matches = Vec::new();

    for (i, window) in haystack.windows(needle.len()).enumerate() {
        if window == needle {
            matches.push(i);
        }
    }

    Ok(matches)
}

fn scalar_matrix_multiply(
    matrix_a: &[f32],
    matrix_b: &[f32],
    rows_a: usize,
    cols_a: usize,
    cols_b: usize,
) -> CoreResult<Vec<f32>> {
    let mut result = vec![0.0f32; rows_a * cols_b];

    for i in 0..rows_a {
        for j in 0..cols_b {
            let mut sum = 0.0;
            for k in 0..cols_a {
                sum += matrix_a[i * cols_a + k] * matrix_b[k * cols_b + j];
            }
            result[i * cols_b + j] = sum;
        }
    }

    Ok(result)
}

fn scalar_quantum_operation(
    real_parts: &mut [f32],
    imag_parts: &mut [f32],
    operation: QuantumOperation,
) -> CoreResult<()> {
    match operation {
        QuantumOperation::Normalize => {
            let norm_sq: f32 = real_parts
                .iter()
                .zip(imag_parts.iter())
                .map(|(r, i)| r * r + i * i)
                .sum();
            let norm = norm_sq.sqrt();

            if norm > 1e-10 {
                for (r, i) in real_parts.iter_mut().zip(imag_parts.iter_mut()) {
                    *r /= norm;
                    *i /= norm;
                }
            }
        }
        QuantumOperation::PhaseFlip => {
            for (r, i) in real_parts.iter_mut().zip(imag_parts.iter_mut()) {
                *r = -*r;
                *i = -*i;
            }
        }
        QuantumOperation::Hadamard => {
            let inv_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
            for i in 0..(real_parts.len() / 2) {
                let r0 = real_parts[i * 2];
                let i0 = imag_parts[i * 2];
                let r1 = real_parts[i * 2 + 1];
                let i1 = imag_parts[i * 2 + 1];

                real_parts[i * 2] = (r0 + r1) * inv_sqrt2;
                imag_parts[i * 2] = (i0 + i1) * inv_sqrt2;
                real_parts[i * 2 + 1] = (r0 - r1) * inv_sqrt2;
                imag_parts[i * 2 + 1] = (i0 - i1) * inv_sqrt2;
            }
        }
    }
    Ok(())
}
