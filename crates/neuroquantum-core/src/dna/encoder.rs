//! Quaternary encoder for DNA compression
//!
//! This module implements the encoding phase of DNA compression, converting binary data
//! to quaternary DNA base sequences with optional dictionary compression for patterns.

use crate::dna::{DNABase, DNACompressionConfig, DNAError};
use rayon::prelude::*;
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Quaternary encoder that converts binary data to DNA bases
#[derive(Debug)]
pub struct QuaternaryEncoder {
    config: DNACompressionConfig,
    dictionary: Option<HashMap<Vec<u8>, u16>>,
}

impl QuaternaryEncoder {
    /// Create a new encoder with the given configuration
    pub fn new(config: &DNACompressionConfig) -> Self {
        Self {
            config: config.clone(),
            dictionary: None,
        }
    }

    /// Apply dictionary compression to find and replace common patterns
    #[instrument(skip(self, data))]
    pub async fn compress_with_dictionary(&mut self, data: &[u8]) -> Result<Vec<u8>, DNAError> {
        if !self.config.enable_dictionary || data.len() < 64 {
            return Ok(data.to_vec());
        }

        debug!("Building compression dictionary");

        // Build frequency map of byte patterns
        let mut pattern_freq = HashMap::new();
        let min_pattern_len = 4;
        let max_pattern_len = 32;

        // Use parallel processing to analyze patterns
        let patterns: Vec<_> = (min_pattern_len..=max_pattern_len.min(data.len()))
            .into_par_iter()
            .flat_map(|len| {
                data.windows(len)
                    .enumerate()
                    .map(|(i, window)| (i, window.to_vec()))
                    .collect::<Vec<_>>()
            })
            .collect();

        // Count pattern frequencies
        for (_, pattern) in patterns {
            *pattern_freq.entry(pattern).or_insert(0usize) += 1;
        }

        // Select most frequent patterns for dictionary
        let mut frequent_patterns: Vec<_> = pattern_freq
            .into_iter()
            .filter(|(pattern, freq)| *freq >= 3 && pattern.len() >= min_pattern_len)
            .collect();

        frequent_patterns.sort_by_key(|(pattern, freq)| {
            std::cmp::Reverse(*freq * pattern.len()) // Prioritize by space savings
        });

        // Build dictionary (limited size)
        let mut dictionary = HashMap::new();
        let mut dict_id = 256u16; // Start after single-byte values

        for (pattern, _) in frequent_patterns
            .iter()
            .take(self.config.max_dictionary_size / 32)
        {
            if dictionary.len() >= (u16::MAX as usize - 256) {
                break;
            }
            dictionary.insert(pattern.clone(), dict_id);
            dict_id += 1;
        }

        if dictionary.is_empty() {
            return Ok(data.to_vec());
        }

        debug!("Built dictionary with {} patterns", dictionary.len());

        // Apply dictionary compression
        let mut compressed = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            let mut matched = false;

            // Try to match longest pattern first
            for len in (min_pattern_len..=max_pattern_len.min(data.len() - i)).rev() {
                if i + len <= data.len() {
                    let pattern = &data[i..i + len];
                    if let Some(&dict_id) = dictionary.get(pattern) {
                        // Encode as dictionary reference: [0xFF][dict_id_high][dict_id_low]
                        compressed.push(0xFF);
                        compressed.push((dict_id >> 8) as u8);
                        compressed.push(dict_id as u8);
                        i += len;
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                // Copy literal byte
                compressed.push(data[i]);
                i += 1;
            }
        }

        self.dictionary = Some(dictionary);
        Ok(compressed)
    }

    /// Convert binary data to DNA bases using quaternary encoding
    #[instrument(skip(self, data))]
    pub async fn encode_to_bases(&self, data: &[u8]) -> Result<Vec<DNABase>, DNAError> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Encoding {} bytes to DNA bases", data.len());

        let capacity = data.len() * 4; // Each byte becomes 4 DNA bases
        let mut bases = Vec::with_capacity(capacity);

        if self.config.enable_simd && self.config.thread_count > 1 {
            // Parallel SIMD-optimized encoding
            self.encode_parallel_simd(data, &mut bases).await?;
        } else {
            // Sequential encoding
            self.encode_sequential(data, &mut bases)?;
        }

        Ok(bases)
    }

    /// Sequential encoding implementation
    fn encode_sequential(&self, data: &[u8], bases: &mut Vec<DNABase>) -> Result<(), DNAError> {
        for &byte in data {
            // Convert each byte to 4 DNA bases (2 bits each)
            for shift in (0..8).step_by(2).rev() {
                let two_bits = (byte >> shift) & 0b11;
                let base = DNABase::from_bits(two_bits)?;
                bases.push(base);
            }
        }
        Ok(())
    }

    /// Parallel SIMD-optimized encoding
    async fn encode_parallel_simd(
        &self,
        data: &[u8],
        bases: &mut Vec<DNABase>,
    ) -> Result<(), DNAError> {
        let chunk_size = (data.len() / self.config.thread_count).max(1024);

        // Process chunks in parallel
        let results: Result<Vec<_>, DNAError> = data
            .par_chunks(chunk_size)
            .map(|chunk| self.encode_chunk_simd(chunk))
            .collect();

        let chunk_results = results?;

        // Combine results
        for chunk_bases in chunk_results {
            bases.extend(chunk_bases);
        }

        Ok(())
    }

    /// Encode a single chunk using SIMD operations where available
    fn encode_chunk_simd(&self, chunk: &[u8]) -> Result<Vec<DNABase>, DNAError> {
        let mut chunk_bases = Vec::with_capacity(chunk.len() * 4);

        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                return self.encode_chunk_neon(chunk);
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return self.encode_chunk_avx2(chunk);
            }
        }

        // Fallback to sequential encoding for this chunk
        self.encode_sequential(chunk, &mut chunk_bases)?;
        Ok(chunk_bases)
    }

    #[cfg(target_arch = "aarch64")]
    fn encode_chunk_neon(&self, chunk: &[u8]) -> Result<Vec<DNABase>, DNAError> {
        let mut result = Vec::with_capacity(chunk.len() * 4);

        // Use the safe wrapper from SIMD module which handles feature detection and unsafe internally
        crate::dna::simd::safe_encode_chunk_neon(chunk, &mut result)?;

        Ok(result)
    }

    #[cfg(target_arch = "x86_64")]
    fn encode_chunk_avx2(&self, chunk: &[u8]) -> Result<Vec<DNABase>, DNAError> {
        let mut result = Vec::with_capacity(chunk.len() * 4);

        // Use the safe wrapper from SIMD module which handles feature detection and unsafe internally
        crate::dna::simd::safe_encode_chunk_avx2(chunk, &mut result)?;

        Ok(result)
    }

    /// Get the dictionary built during compression (if any)
    pub fn get_dictionary(&self) -> Option<HashMap<Vec<u8>, u16>> {
        self.dictionary.clone()
    }

    /// Estimate compression ratio without full compression
    pub fn estimate_compression_ratio(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 1.0;
        }

        // Base quaternary encoding: 1 byte -> 4 bases -> 1 byte (no compression from this step)
        let base_size = data.len();

        // Estimate dictionary compression savings
        let dict_savings = if self.config.enable_dictionary {
            self.estimate_dictionary_savings(data)
        } else {
            0.0
        };

        // Account for Reed-Solomon parity overhead
        let parity_overhead = (self.config.error_correction_strength as f64 / 255.0) * 0.2;

        let effective_ratio =
            (base_size as f64 - dict_savings) / data.len() as f64 + parity_overhead;
        effective_ratio.max(0.1) // Minimum 10% of original size
    }

    fn estimate_dictionary_savings(&self, data: &[u8]) -> f64 {
        // Quick heuristic: count repeated 4-byte patterns
        let mut pattern_count = HashMap::new();

        for window in data.windows(4) {
            *pattern_count.entry(window).or_insert(0) += 1;
        }

        let repeated_bytes: usize = pattern_count
            .values()
            .filter(|&&count| count > 2)
            .map(|&count| (count - 1) * 3) // Each replacement saves ~3 bytes
            .sum();

        repeated_bytes as f64
    }
}
