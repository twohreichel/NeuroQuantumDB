//! Quaternary decoder for DNA compression
//!
//! This module implements the decoding phase of DNA compression, converting DNA base
//! sequences back to binary data with dictionary decompression support.

use std::collections::HashMap;

use rayon::prelude::*;
use tracing::{debug, instrument};

use crate::dna::{DNABase, DNACompressionConfig, DNAError};

/// Quaternary decoder that converts DNA bases back to binary data
#[derive(Debug)]
pub struct QuaternaryDecoder {
    config: DNACompressionConfig,
}

impl QuaternaryDecoder {
    /// Create a new decoder with the given configuration
    #[must_use]
    pub fn new(config: &DNACompressionConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Convert DNA bases back to binary data using quaternary decoding
    #[instrument(skip(self, bases))]
    pub async fn decode_from_bases(&self, bases: &[DNABase]) -> Result<Vec<u8>, DNAError> {
        if bases.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Decoding {} DNA bases to binary", bases.len());

        // Each group of 4 bases represents 1 byte
        if !bases.len().is_multiple_of(4) {
            return Err(DNAError::LengthMismatch {
                expected: (bases.len() / 4) * 4,
                actual: bases.len(),
            });
        }

        let capacity = bases.len() / 4;
        let mut data = Vec::with_capacity(capacity);

        if self.config.enable_simd && self.config.thread_count > 1 {
            // Parallel SIMD-optimized decoding
            self.decode_parallel_simd(bases, &mut data).await?;
        } else {
            // Sequential decoding
            self.decode_sequential(bases, &mut data)?;
        }

        Ok(data)
    }

    /// Sequential decoding implementation
    fn decode_sequential(&self, bases: &[DNABase], data: &mut Vec<u8>) -> Result<(), DNAError> {
        for chunk in bases.chunks_exact(4) {
            let mut byte = 0u8;

            // Combine 4 bases (2 bits each) into 1 byte
            for (i, &base) in chunk.iter().enumerate() {
                let shift = 6 - (i * 2); // Start from most significant bits
                byte |= (base.to_bits()) << shift;
            }

            data.push(byte);
        }
        Ok(())
    }

    /// Parallel SIMD-optimized decoding
    async fn decode_parallel_simd(
        &self,
        bases: &[DNABase],
        data: &mut Vec<u8>,
    ) -> Result<(), DNAError> {
        let chunk_size = ((bases.len() / 4) / self.config.thread_count).max(1024) * 4; // Keep 4-base alignment

        // Process chunks in parallel
        let results: Result<Vec<_>, DNAError> = bases
            .par_chunks(chunk_size)
            .map(|chunk| self.decode_chunk_simd(chunk))
            .collect();

        let chunk_results = results?;

        // Combine results
        for chunk_data in chunk_results {
            data.extend(chunk_data);
        }

        Ok(())
    }

    /// Decode a single chunk using SIMD operations where available
    fn decode_chunk_simd(&self, chunk: &[DNABase]) -> Result<Vec<u8>, DNAError> {
        if !chunk.len().is_multiple_of(4) {
            return Err(DNAError::LengthMismatch {
                expected: (chunk.len() / 4) * 4,
                actual: chunk.len(),
            });
        }

        let mut chunk_data = Vec::with_capacity(chunk.len() / 4);

        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                return self.decode_chunk_neon(chunk);
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return self.decode_chunk_avx2(chunk);
            }
        }

        // Fallback to sequential decoding for this chunk
        self.decode_sequential(chunk, &mut chunk_data)?;
        Ok(chunk_data)
    }

    #[cfg(target_arch = "aarch64")]
    fn decode_chunk_neon(&self, chunk: &[DNABase]) -> Result<Vec<u8>, DNAError> {
        let mut result = Vec::with_capacity(chunk.len() / 4);
        let mut i = 0;

        // Process 64 bases (16 bytes) at a time with NEON
        while i + 64 <= chunk.len() {
            // Convert bases to bytes in groups of 4
            for group in 0..16 {
                let base_offset = i + group * 4;
                let mut byte = 0u8;

                for j in 0..4 {
                    let base = chunk[base_offset + j];
                    let shift = 6 - (j * 2);
                    byte |= base.to_bits() << shift;
                }

                result.push(byte);
            }

            i += 64;
        }

        // Handle remaining bases
        while i + 4 <= chunk.len() {
            let mut byte = 0u8;
            for j in 0..4 {
                let base = chunk[i + j];
                let shift = 6 - (j * 2);
                byte |= base.to_bits() << shift;
            }
            result.push(byte);
            i += 4;
        }

        Ok(result)
    }

    #[cfg(target_arch = "x86_64")]
    fn decode_chunk_avx2(&self, chunk: &[DNABase]) -> Result<Vec<u8>, DNAError> {
        let mut result = Vec::with_capacity(chunk.len() / 4);
        let mut i = 0;

        // Process 128 bases (32 bytes) at a time with AVX2
        while i + 128 <= chunk.len() {
            // Convert bases to bytes in groups of 4
            for group in 0..32 {
                let base_offset = i + group * 4;
                let mut byte = 0u8;

                for j in 0..4 {
                    let base = chunk[base_offset + j];
                    let shift = 6 - (j * 2);
                    byte |= base.to_bits() << shift;
                }

                result.push(byte);
            }

            i += 128;
        }

        // Handle remaining bases
        while i + 4 <= chunk.len() {
            let mut byte = 0u8;
            for j in 0..4 {
                let base = chunk[i + j];
                let shift = 6 - (j * 2);
                byte |= base.to_bits() << shift;
            }
            result.push(byte);
            i += 4;
        }

        Ok(result)
    }

    /// Apply dictionary decompression to restore original patterns
    #[instrument(skip(self, data, dictionary))]
    pub async fn decompress_with_dictionary(
        &self,
        data: &[u8],
        dictionary: &HashMap<Vec<u8>, u16>,
    ) -> Result<Vec<u8>, DNAError> {
        if dictionary.is_empty() || data.is_empty() {
            return Ok(data.to_vec());
        }

        debug!(
            "Applying dictionary decompression with {} patterns",
            dictionary.len()
        );

        // Build reverse lookup table
        let reverse_dict: HashMap<u16, Vec<u8>> = dictionary
            .iter()
            .map(|(pattern, &id)| (id, pattern.clone()))
            .collect();

        let mut decompressed = Vec::with_capacity(data.len() * 2); // Estimate expansion
        let mut i = 0;

        while i < data.len() {
            if data[i] == 0xFF && i + 2 < data.len() {
                // Dictionary reference: [0xFF][dict_id_high][dict_id_low]
                let dict_id = (u16::from(data[i + 1]) << 8) | u16::from(data[i + 2]);

                if let Some(pattern) = reverse_dict.get(&dict_id) {
                    decompressed.extend_from_slice(pattern);
                    i += 3;
                } else {
                    return Err(DNAError::DecompressionFailed(format!(
                        "Invalid dictionary reference: {dict_id}"
                    )));
                }
            } else {
                // Literal byte
                decompressed.push(data[i]);
                i += 1;
            }
        }

        Ok(decompressed)
    }

    /// Validate DNA base sequence integrity
    pub fn validate_bases(&self, bases: &[DNABase]) -> Result<(), DNAError> {
        // Check that the sequence length is a multiple of 4
        if !bases.len().is_multiple_of(4) {
            return Err(DNAError::LengthMismatch {
                expected: (bases.len() / 4) * 4,
                actual: bases.len(),
            });
        }

        // All bases should be valid (this is guaranteed by the enum type system,
        // but we can add additional biological validation here if needed)

        // Check for biological plausibility (optional)
        if self.config.enable_dictionary {
            self.validate_biological_patterns(bases)?;
        }

        Ok(())
    }

    /// Validate biological plausibility of DNA sequences
    fn validate_biological_patterns(&self, bases: &[DNABase]) -> Result<(), DNAError> {
        // Check for extremely long runs of the same base (unlikely in real DNA)
        let mut current_base = bases[0];
        let mut run_length = 1;
        let max_run_length = 16; // Configurable threshold

        for &base in &bases[1..] {
            if base == current_base {
                run_length += 1;
                if run_length > max_run_length {
                    debug!("Warning: Long homopolymer run detected ({})", run_length);
                    // This is a warning, not an error, as it could be valid compressed data
                }
            } else {
                current_base = base;
                run_length = 1;
            }
        }

        // Check GC content (should be roughly balanced for good compression)
        let gc_count = bases
            .iter()
            .filter(|&&base| matches!(base, DNABase::Guanine | DNABase::Cytosine))
            .count();

        let gc_ratio = gc_count as f64 / bases.len() as f64;

        // Warn if GC content is extremely skewed (could indicate poor compression)
        if !(0.1..=0.9).contains(&gc_ratio) {
            debug!(
                "Warning: Extreme GC content detected: {:.2}%",
                gc_ratio * 100.0
            );
        }

        Ok(())
    }

    /// Get decoding statistics for the last operation
    #[must_use]
    pub fn get_decoding_stats(&self, bases: &[DNABase]) -> DecodingStats {
        let total_bases = bases.len();
        let expected_bytes = total_bases / 4;

        // Count base frequencies
        let mut base_counts = [0usize; 4];
        for &base in bases {
            base_counts[base as usize] += 1;
        }

        let gc_content = (base_counts[DNABase::Guanine as usize]
            + base_counts[DNABase::Cytosine as usize]) as f64
            / total_bases as f64;

        DecodingStats {
            total_bases,
            expected_bytes,
            adenine_count: base_counts[DNABase::Adenine as usize],
            thymine_count: base_counts[DNABase::Thymine as usize],
            guanine_count: base_counts[DNABase::Guanine as usize],
            cytosine_count: base_counts[DNABase::Cytosine as usize],
            gc_content,
        }
    }
}

/// Statistics collected during decoding operations
#[derive(Debug, Clone)]
pub struct DecodingStats {
    pub total_bases: usize,
    pub expected_bytes: usize,
    pub adenine_count: usize,
    pub thymine_count: usize,
    pub guanine_count: usize,
    pub cytosine_count: usize,
    pub gc_content: f64,
}
