//! SIMD optimizations for DNA compression
//!
//! This module provides SIMD-optimized implementations for DNA compression operations
//! targeting ARM64 NEON and `x86_64` AVX2 instruction sets.

use crate::dna::{DNABase, DNAError};

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
pub mod arm64_neon;
pub mod x86_avx2;

#[cfg(test)]
mod tests;

// Safe wrapper functions that perform feature detection internally

/// Safe wrapper for NEON DNA encoding with automatic feature detection
///
/// This function provides a safe interface to NEON-accelerated DNA encoding.
/// It automatically detects NEON availability at runtime and falls back to
/// scalar implementation if NEON is not available.
///
/// # Arguments
/// * `input` - Byte slice to encode into DNA bases
/// * `output` - Vector to append encoded DNA bases to
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(DNAError)` if encoding fails (e.g., invalid bit pattern)
#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
pub fn safe_encode_chunk_neon(input: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
    if std::arch::is_aarch64_feature_detected!("neon") {
        // SAFETY: We have verified NEON is available via runtime feature detection.
        // The `encode_chunk_neon` function requires:
        // - NEON support: verified by `is_aarch64_feature_detected!("neon")`
        // - Valid input slice: guaranteed by Rust's slice safety
        // - Valid output vector: guaranteed by mutable borrow rules
        unsafe { arm64_neon::encode_chunk_neon(input, output) }
    } else {
        // Fallback to scalar encoding
        for &byte in input {
            for shift in (0..8).step_by(2).rev() {
                let two_bits = (byte >> shift) & 0b11;
                let base = DNABase::from_bits(two_bits)?;
                output.push(base);
            }
        }
        Ok(())
    }
}

/// Safe wrapper for AVX2 DNA encoding with automatic feature detection
///
/// This function provides a safe interface to AVX2-accelerated DNA encoding.
/// It automatically detects AVX2 availability at runtime and falls back to
/// scalar implementation if AVX2 is not available.
///
/// # Arguments
/// * `input` - Byte slice to encode into DNA bases
/// * `output` - Vector to append encoded DNA bases to
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(DNAError)` if encoding fails (e.g., invalid bit pattern)
#[cfg(target_arch = "x86_64")]
pub fn safe_encode_chunk_avx2(input: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: We have verified AVX2 is available via runtime feature detection.
        // The `encode_chunk_avx2` function requires:
        // - AVX2 support: verified by `is_x86_feature_detected!("avx2")`
        // - Valid input slice: guaranteed by Rust's slice safety
        // - Valid output vector: guaranteed by mutable borrow rules
        unsafe { x86_avx2::encode_chunk_avx2(input, output) }
    } else {
        // Fallback to scalar encoding
        for &byte in input {
            for shift in (0..8).step_by(2).rev() {
                let two_bits = (byte >> shift) & 0b11;
                let base = DNABase::from_bits(two_bits)?;
                output.push(base);
            }
        }
        Ok(())
    }
}

/// Safe wrapper for NEON DNA decoding with automatic feature detection
///
/// This function provides a safe interface to NEON-accelerated DNA decoding.
/// It automatically detects NEON availability at runtime and falls back to
/// scalar implementation if NEON is not available.
///
/// # Arguments
/// * `input` - DNA bases slice to decode into bytes
/// * `output` - Vector to append decoded bytes to
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(DNAError)` if decoding fails (e.g., input length not multiple of 4)
#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
pub fn safe_decode_chunk_neon(input: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
    if std::arch::is_aarch64_feature_detected!("neon") {
        // SAFETY: We have verified NEON is available via runtime feature detection.
        // The `decode_chunk_neon` function requires:
        // - NEON support: verified by `is_aarch64_feature_detected!("neon")`
        // - Valid input slice with DNABase values: guaranteed by type system
        // - Valid output vector: guaranteed by mutable borrow rules
        unsafe { arm64_neon::decode_chunk_neon(input, output) }
    } else {
        // Fallback to scalar decoding
        for bases in input.chunks_exact(4) {
            let mut byte = 0u8;
            for (i, &base) in bases.iter().enumerate() {
                let shift = 6 - (i * 2);
                byte |= (base.to_bits()) << shift;
            }
            output.push(byte);
        }
        Ok(())
    }
}

/// Safe wrapper for AVX2 DNA decoding with automatic feature detection
///
/// This function provides a safe interface to AVX2-accelerated DNA decoding.
/// It automatically detects AVX2 availability at runtime and falls back to
/// scalar implementation if AVX2 is not available.
///
/// # Arguments
/// * `input` - DNA bases slice to decode into bytes
/// * `output` - Vector to append decoded bytes to
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(DNAError)` if decoding fails (e.g., input length not multiple of 4)
#[cfg(target_arch = "x86_64")]
pub fn safe_decode_chunk_avx2(input: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: We have verified AVX2 is available via runtime feature detection.
        // The `decode_chunk_avx2` function requires:
        // - AVX2 support: verified by `is_x86_feature_detected!("avx2")`
        // - Valid input slice with DNABase values: guaranteed by type system
        // - Valid output vector: guaranteed by mutable borrow rules
        unsafe { x86_avx2::decode_chunk_avx2(input, output) }
    } else {
        // Fallback to scalar decoding
        for bases in input.chunks_exact(4) {
            let mut byte = 0u8;
            for (i, &base) in bases.iter().enumerate() {
                let shift = 6 - (i * 2);
                byte |= (base.to_bits()) << shift;
            }
            output.push(byte);
        }
        Ok(())
    }
}

/// NEON capability information
#[derive(Debug, Clone)]
pub struct NeonCapabilities {
    pub has_neon: bool,
    pub has_crc32: bool,
    pub has_sha2: bool,
    pub vector_width: usize,
    pub parallel_lanes: usize,
}

/// Detect NEON capabilities (delegates to platform-specific implementation)
#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
#[must_use]
pub fn detect_neon_capabilities() -> NeonCapabilities {
    arm64_neon::detect_neon_capabilities()
}

/// Stub for non-ARM64 platforms
#[cfg(not(any(target_arch = "aarch64", target_arch = "arm64ec")))]
pub fn detect_neon_capabilities() -> NeonCapabilities {
    NeonCapabilities {
        has_neon: false,
        has_crc32: false,
        has_sha2: false,
        vector_width: 0,
        parallel_lanes: 0,
    }
}

/// SIMD capability detection and dispatch
#[derive(Debug, Clone)]
pub struct SimdCapabilities {
    pub has_neon: bool,
    pub has_avx2: bool,
    pub has_sse42: bool,
    pub vector_width: usize,
}

impl SimdCapabilities {
    /// Detect available SIMD capabilities on the current CPU
    #[must_use]
    pub fn detect() -> Self {
        let mut caps = Self {
            has_neon: false,
            has_avx2: false,
            has_sse42: false,
            vector_width: 1,
        };

        #[cfg(target_arch = "aarch64")]
        {
            caps.has_neon = std::arch::is_aarch64_feature_detected!("neon");
            if caps.has_neon {
                caps.vector_width = 16; // 128-bit NEON vectors
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            caps.has_avx2 = is_x86_feature_detected!("avx2");
            caps.has_sse42 = is_x86_feature_detected!("sse4.2");

            if caps.has_avx2 {
                caps.vector_width = 32; // 256-bit AVX2 vectors
            } else if caps.has_sse42 {
                caps.vector_width = 16; // 128-bit SSE vectors
            }
        }

        caps
    }

    /// Get the optimal chunk size for SIMD operations
    #[must_use]
    pub const fn optimal_chunk_size(&self) -> usize {
        self.vector_width * 4 // Process multiple vectors per chunk
    }
}

/// SIMD-optimized DNA encoding operations
pub struct SimdEncoder {
    capabilities: SimdCapabilities,
}

impl SimdEncoder {
    /// Create a new SIMD encoder
    #[must_use]
    pub fn new() -> Self {
        Self {
            capabilities: SimdCapabilities::detect(),
        }
    }

    /// Encode bytes to DNA bases using SIMD when available
    pub fn encode_bytes_to_bases(
        &self,
        input: &[u8],
        output: &mut Vec<DNABase>,
    ) -> Result<(), DNAError> {
        if input.is_empty() {
            return Ok(());
        }

        let chunk_size = self.capabilities.optimal_chunk_size();

        // Process large chunks with SIMD
        for chunk in input.chunks(chunk_size) {
            if chunk.len() == chunk_size {
                self.encode_chunk_simd(chunk, output)?;
            } else {
                // Handle remainder with scalar code
                self.encode_chunk_scalar(chunk, output)?;
            }
        }

        Ok(())
    }

    /// SIMD-optimized chunk encoding
    fn encode_chunk_simd(&self, chunk: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
        #[cfg(target_arch = "aarch64")]
        {
            if self.capabilities.has_neon {
                return unsafe { arm64_neon::encode_chunk_neon(chunk, output) };
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if self.capabilities.has_avx2 {
                return unsafe { x86_avx2::encode_chunk_avx2(chunk, output) };
            }
        }

        // Fallback to scalar implementation
        self.encode_chunk_scalar(chunk, output)
    }

    /// Scalar fallback for chunk encoding
    fn encode_chunk_scalar(&self, chunk: &[u8], output: &mut Vec<DNABase>) -> Result<(), DNAError> {
        for &byte in chunk {
            for shift in (0..8).step_by(2).rev() {
                let two_bits = (byte >> shift) & 0b11;
                let base = DNABase::from_bits(two_bits)?;
                output.push(base);
            }
        }
        Ok(())
    }

    /// Batch convert multiple bytes to bases with maximum SIMD utilization
    pub fn batch_encode(&self, input: &[u8]) -> Result<Vec<DNABase>, DNAError> {
        let mut output = Vec::with_capacity(input.len() * 4);
        self.encode_bytes_to_bases(input, &mut output)?;
        Ok(output)
    }
}

/// SIMD-optimized DNA decoding operations
pub struct SimdDecoder {
    capabilities: SimdCapabilities,
}

impl SimdDecoder {
    /// Create a new SIMD decoder
    #[must_use]
    pub fn new() -> Self {
        Self {
            capabilities: SimdCapabilities::detect(),
        }
    }

    /// Decode DNA bases to bytes using SIMD when available
    pub fn decode_bases_to_bytes(
        &self,
        input: &[DNABase],
        output: &mut Vec<u8>,
    ) -> Result<(), DNAError> {
        if input.is_empty() {
            return Ok(());
        }

        if !input.len().is_multiple_of(4) {
            return Err(DNAError::LengthMismatch {
                expected: (input.len() / 4) * 4,
                actual: input.len(),
            });
        }

        let chunk_size = self.capabilities.optimal_chunk_size() * 4; // 4 bases per byte

        // Process large chunks with SIMD
        for chunk in input.chunks(chunk_size) {
            if chunk.len() == chunk_size {
                self.decode_chunk_simd(chunk, output)?;
            } else {
                // Handle remainder with scalar code
                self.decode_chunk_scalar(chunk, output)?;
            }
        }

        Ok(())
    }

    /// SIMD-optimized chunk decoding
    fn decode_chunk_simd(&self, chunk: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
        #[cfg(target_arch = "aarch64")]
        {
            if self.capabilities.has_neon {
                return unsafe { arm64_neon::decode_chunk_neon(chunk, output) };
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if self.capabilities.has_avx2 {
                return unsafe { x86_avx2::decode_chunk_avx2(chunk, output) };
            }
        }

        // Fallback to scalar implementation
        self.decode_chunk_scalar(chunk, output)
    }

    /// Scalar fallback for chunk decoding
    fn decode_chunk_scalar(&self, chunk: &[DNABase], output: &mut Vec<u8>) -> Result<(), DNAError> {
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

    /// Batch convert multiple bases to bytes with maximum SIMD utilization
    pub fn batch_decode(&self, input: &[DNABase]) -> Result<Vec<u8>, DNAError> {
        let mut output = Vec::with_capacity(input.len() / 4);
        self.decode_bases_to_bytes(input, &mut output)?;
        Ok(output)
    }
}

/// SIMD-optimized pattern matching for dictionary compression
pub struct SimdPatternMatcher {
    capabilities: SimdCapabilities,
}

impl SimdPatternMatcher {
    /// Create a new SIMD pattern matcher
    #[must_use]
    pub fn new() -> Self {
        Self {
            capabilities: SimdCapabilities::detect(),
        }
    }

    /// Find pattern occurrences using SIMD string matching
    #[must_use]
    pub fn find_pattern_occurrences(&self, haystack: &[u8], needle: &[u8]) -> Vec<usize> {
        if needle.is_empty() || haystack.len() < needle.len() {
            return Vec::new();
        }

        let mut matches = Vec::new();

        #[cfg(target_arch = "aarch64")]
        {
            if self.capabilities.has_neon && needle.len() <= 16 {
                return unsafe { arm64_neon::find_pattern_neon(haystack, needle) };
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if self.capabilities.has_avx2 && needle.len() <= 32 {
                return unsafe { x86_avx2::find_pattern_avx2(haystack, needle) };
            }
        }

        // Fallback to scalar implementation
        self.find_pattern_scalar(haystack, needle, &mut matches);
        matches
    }

    /// Scalar pattern matching fallback
    fn find_pattern_scalar(&self, haystack: &[u8], needle: &[u8], matches: &mut Vec<usize>) {
        for i in 0..=haystack.len().saturating_sub(needle.len()) {
            if haystack[i..i + needle.len()] == *needle {
                matches.push(i);
            }
        }
    }
}

/// SIMD utilities and helper functions
pub mod utils {
    use super::{arm64_neon, DNABase, DNAError, SimdCapabilities};

    /// Convert DNA bases to packed binary representation for SIMD processing
    #[must_use]
    pub fn pack_bases(bases: &[DNABase]) -> Vec<u64> {
        let mut packed = Vec::new();

        for chunk in bases.chunks(32) {
            // 32 bases = 64 bits
            let mut value = 0u64;
            for (i, &base) in chunk.iter().enumerate() {
                value |= u64::from(base.to_bits()) << (62 - i * 2);
            }
            packed.push(value);
        }

        packed
    }

    /// Unpack binary representation back to DNA bases
    #[must_use]
    pub fn unpack_bases(packed: &[u64], expected_count: usize) -> Vec<DNABase> {
        let mut bases = Vec::with_capacity(expected_count);

        for &value in packed {
            for i in 0..32 {
                if bases.len() >= expected_count {
                    break;
                }
                let shift = 62 - i * 2;
                let two_bits = ((value >> shift) & 0b11) as u8;
                if let Ok(base) = DNABase::from_bits(two_bits) {
                    bases.push(base);
                }
            }
        }

        bases.truncate(expected_count);
        bases
    }

    /// Transpose bytes for more efficient SIMD processing
    ///
    /// Converts Array-of-Structures to Structure-of-Arrays layout
    /// for better SIMD vectorization. For example, converts:
    /// \[ABCD\]\[EFGH\]\[IJKL\] to \[AEI\]\[BFJ\]\[CGK\]\[DHL\]
    #[must_use]
    pub fn transpose_bytes(input: &[u8]) -> Vec<u8> {
        if input.is_empty() {
            return Vec::new();
        }

        // Process in 4x4 blocks for optimal SIMD performance
        const BLOCK_SIZE: usize = 4;
        let mut output = vec![0u8; input.len()];

        let full_blocks = input.len() / (BLOCK_SIZE * BLOCK_SIZE);
        let remainder = input.len() % (BLOCK_SIZE * BLOCK_SIZE);

        // Transpose 4x4 blocks
        for block_idx in 0..full_blocks {
            let base_in = block_idx * BLOCK_SIZE * BLOCK_SIZE;
            let base_out = block_idx * BLOCK_SIZE * BLOCK_SIZE;

            // Manual 4x4 transpose
            for i in 0..BLOCK_SIZE {
                for j in 0..BLOCK_SIZE {
                    output[base_out + j * BLOCK_SIZE + i] = input[base_in + i * BLOCK_SIZE + j];
                }
            }
        }

        // Handle remainder bytes without transposition
        if remainder > 0 {
            let start = full_blocks * BLOCK_SIZE * BLOCK_SIZE;
            output[start..].copy_from_slice(&input[start..]);
        }

        output
    }

    /// Calculate Hamming distance between DNA sequences using SIMD
    pub fn hamming_distance_simd(seq1: &[DNABase], seq2: &[DNABase]) -> Result<usize, DNAError> {
        if seq1.len() != seq2.len() {
            return Err(DNAError::LengthMismatch {
                expected: seq1.len(),
                actual: seq2.len(),
            });
        }

        let capabilities = SimdCapabilities::detect();

        #[cfg(target_arch = "aarch64")]
        {
            if capabilities.has_neon {
                return unsafe { arm64_neon::hamming_distance_neon(seq1, seq2) };
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if capabilities.has_avx2 {
                return unsafe { x86_avx2::hamming_distance_avx2(seq1, seq2) };
            }
        }

        // Scalar fallback
        Ok(seq1.iter().zip(seq2.iter()).filter(|(a, b)| a != b).count())
    }
}

impl Default for SimdEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SimdDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SimdPatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}
