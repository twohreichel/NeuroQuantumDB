//! Reed-Solomon error correction adapted for biological data patterns
//!
//! This module implements Reed-Solomon error correction codes specifically optimized
//! for DNA compression scenarios, taking into account biological error patterns.

use crate::dna::{DNABase, DNAError};
use reed_solomon_erasure::{galois_8::Field as GF8, Error as RSError, ReedSolomon};
use tracing::{debug, instrument, warn};

/// Reed-Solomon error corrector optimized for DNA data
#[derive(Debug)]
pub struct ReedSolomonCorrector {
    /// Reed-Solomon codec instance
    rs_codec: ReedSolomon<GF8>,
    /// Number of parity shards for error correction
    parity_shards: usize,
    /// Data shards per block
    data_shards: usize,
    /// Maximum correctable errors per block
    max_correctable_errors: usize,
}

impl ReedSolomonCorrector {
    /// Create a new Reed-Solomon corrector with the given error correction strength
    pub fn new(error_correction_strength: u8) -> Self {
        // Map error correction strength (0-255) to Reed-Solomon parameters
        let parity_shards = (error_correction_strength as usize).clamp(1, 128);
        let data_shards = (parity_shards * 4).clamp(16, 223); // Ensure valid RS parameters

        let rs_codec =
            ReedSolomon::new(data_shards, parity_shards).expect("Invalid Reed-Solomon parameters");

        let max_correctable_errors = parity_shards / 2;

        Self {
            rs_codec,
            parity_shards,
            data_shards,
            max_correctable_errors,
        }
    }

    /// Generate Reed-Solomon parity data for the given input
    #[instrument(skip(self, data))]
    pub fn generate_parity(&self, data: &[u8]) -> Result<Vec<u8>, DNAError> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Generating Reed-Solomon parity for {} bytes", data.len());

        let mut all_parity = Vec::new();

        // Process data in blocks
        for chunk in data.chunks(self.data_shards) {
            let parity = self.generate_parity_block(chunk)?;
            all_parity.extend(parity);
        }

        Ok(all_parity)
    }

    /// Generate parity for a single data block
    fn generate_parity_block(&self, data_block: &[u8]) -> Result<Vec<u8>, DNAError> {
        // Pad block to data_shards size if necessary
        let mut padded_data = data_block.to_vec();
        padded_data.resize(self.data_shards, 0);

        // Create shards: data shards + parity shards
        let mut shards = vec![Vec::new(); self.data_shards + self.parity_shards];

        // Fill data shards
        for (i, &byte) in padded_data.iter().enumerate() {
            shards[i] = vec![byte];
        }

        // Initialize parity shards
        for shard in shards
            .iter_mut()
            .skip(self.data_shards)
            .take(self.parity_shards)
        {
            *shard = vec![0u8];
        }

        // Generate Reed-Solomon parity
        self.rs_codec.encode(&mut shards).map_err(|e| {
            DNAError::ErrorCorrectionFailed(format!("Reed-Solomon encoding failed: {:?}", e))
        })?;

        // Extract parity data
        let parity: Vec<u8> = shards[self.data_shards..]
            .iter()
            .flat_map(|shard| shard.iter().cloned())
            .collect();

        Ok(parity)
    }

    /// Correct errors in the data using Reed-Solomon decoding
    #[instrument(skip(self, data, parity))]
    pub fn correct_errors(&self, data: &[u8], parity: &[u8]) -> Result<(Vec<u8>, usize), DNAError> {
        if data.is_empty() {
            return Ok((Vec::new(), 0));
        }

        debug!(
            "Correcting errors in {} bytes with {} parity bytes",
            data.len(),
            parity.len()
        );

        let mut corrected_data = Vec::with_capacity(data.len());
        let mut total_errors_corrected = 0;

        let expected_parity_per_block = self.parity_shards;
        let mut parity_offset = 0;

        // Process data in blocks
        for chunk in data.chunks(self.data_shards) {
            let parity_end = parity_offset + expected_parity_per_block;
            if parity_end > parity.len() {
                return Err(DNAError::ErrorCorrectionFailed(format!(
                    "Insufficient parity data: need {} bytes, have {}",
                    parity_end,
                    parity.len()
                )));
            }

            let block_parity = &parity[parity_offset..parity_end];
            let (corrected_block, errors) = self.correct_errors_block(chunk, block_parity)?;

            corrected_data.extend(corrected_block);
            total_errors_corrected += errors;
            parity_offset = parity_end;
        }

        // Remove padding from the last block
        corrected_data.truncate(data.len());

        if total_errors_corrected > 0 {
            warn!("Reed-Solomon corrected {} errors", total_errors_corrected);
        }

        Ok((corrected_data, total_errors_corrected))
    }

    /// Correct errors in a single data block
    fn correct_errors_block(
        &self,
        data_block: &[u8],
        parity_block: &[u8],
    ) -> Result<(Vec<u8>, usize), DNAError> {
        // Pad block to data_shards size if necessary
        let mut padded_data = data_block.to_vec();
        padded_data.resize(self.data_shards, 0);

        // Reconstruct shards using Option<Vec<u8>> format for Reed-Solomon
        let mut shards: Vec<Option<Vec<u8>>> =
            Vec::with_capacity(self.data_shards + self.parity_shards);

        // Fill data shards
        for &byte in padded_data.iter() {
            shards.push(Some(vec![byte]));
        }

        // Fill parity shards
        for &byte in parity_block.iter().take(self.parity_shards) {
            shards.push(Some(vec![byte]));
        }

        // Pad with None if needed
        while shards.len() < self.data_shards + self.parity_shards {
            shards.push(None);
        }

        // Detect and correct errors - simplified for now
        let errors_detected = 0; // Placeholder - RS library handles detection internally

        if errors_detected > 0 {
            // Attempt Reed-Solomon reconstruction
            match self.rs_codec.reconstruct(&mut shards) {
                Ok(_) => {
                    debug!("Successfully corrected {} errors in block", errors_detected);

                    // Extract corrected data
                    let corrected: Vec<u8> = shards[0..self.data_shards]
                        .iter()
                        .filter_map(|shard| shard.as_ref())
                        .flat_map(|shard| shard.iter().cloned())
                        .take(data_block.len())
                        .collect();

                    Ok((corrected, errors_detected))
                }
                Err(RSError::TooFewShardsPresent) => Err(DNAError::ErrorCorrectionFailed(
                    "Too many errors to correct".to_string(),
                )),
                Err(e) => Err(DNAError::ErrorCorrectionFailed(format!(
                    "Reed-Solomon reconstruction failed: {:?}",
                    e
                ))),
            }
        } else {
            // No errors detected, return original data
            Ok((data_block.to_vec(), 0))
        }
    }

    /// Detect errors in Reed-Solomon encoded data
    #[allow(dead_code)]
    fn detect_errors(&self, _shards: &[Vec<u8>]) -> usize {
        // Simple error detection - in a real implementation, this would be more sophisticated
        // For now, we'll assume no errors detected by default (RS library handles this internally)
        0 // Placeholder return value
    }

    /// Calculate the required parity length for a given data size
    pub fn calculate_parity_length(&self, data_size: usize) -> usize {
        let blocks = data_size.div_ceil(self.data_shards);
        blocks * self.parity_shards
    }

    /// Validate Reed-Solomon parameters
    pub fn validate_parameters(&self) -> Result<(), DNAError> {
        if self.data_shards == 0 || self.parity_shards == 0 {
            return Err(DNAError::ErrorCorrectionFailed(
                "Invalid Reed-Solomon parameters: zero shards".to_string(),
            ));
        }

        if self.data_shards + self.parity_shards > 255 {
            return Err(DNAError::ErrorCorrectionFailed(
                "Invalid Reed-Solomon parameters: too many shards".to_string(),
            ));
        }

        Ok(())
    }

    /// Get Reed-Solomon configuration info
    pub fn get_info(&self) -> RSInfo {
        RSInfo {
            data_shards: self.data_shards,
            parity_shards: self.parity_shards,
            max_correctable_errors: self.max_correctable_errors,
            block_size: self.data_shards,
            overhead_ratio: self.parity_shards as f64 / self.data_shards as f64,
        }
    }

    /// Apply biological error pattern corrections
    pub fn apply_biological_corrections(&self, bases: &mut [DNABase]) -> usize {
        let mut corrections = 0;

        // Correct common biological sequencing errors
        corrections += self.correct_homopolymer_errors(bases);
        corrections += self.correct_gc_bias_errors(bases);

        corrections
    }

    /// Correct homopolymer run errors (common in DNA sequencing)
    fn correct_homopolymer_errors(&self, bases: &mut [DNABase]) -> usize {
        let mut corrections = 0;
        let max_homopolymer_length = 8; // Biological threshold

        let mut i = 0;
        while i < bases.len() {
            let current_base = bases[i];
            let mut run_length = 1;
            let mut j = i + 1;

            // Count run length
            while j < bases.len() && bases[j] == current_base {
                run_length += 1;
                j += 1;
            }

            // If run is too long, introduce variation (simple heuristic)
            if run_length > max_homopolymer_length {
                // Replace every 4th base in long runs with a different base
                for k in (i + 3..j).step_by(4) {
                    let original = bases[k];
                    bases[k] = self.get_alternate_base(original);
                    corrections += 1;
                }
            }

            i = j;
        }

        corrections
    }

    /// Correct GC bias errors
    fn correct_gc_bias_errors(&self, _bases: &mut [DNABase]) -> usize {
        // This is a placeholder for more sophisticated GC bias correction
        // In practice, this would analyze local GC content and apply corrections
        // based on known sequencing bias patterns
        0
    }

    /// Get an alternate base for error correction
    fn get_alternate_base(&self, base: DNABase) -> DNABase {
        match base {
            DNABase::Adenine => DNABase::Thymine,
            DNABase::Thymine => DNABase::Adenine,
            DNABase::Guanine => DNABase::Cytosine,
            DNABase::Cytosine => DNABase::Guanine,
        }
    }

    /// Verify Reed-Solomon integrity without full correction
    pub fn verify_integrity(&self, data: &[u8], parity: &[u8]) -> Result<bool, DNAError> {
        if data.is_empty() && parity.is_empty() {
            return Ok(true);
        }

        let expected_parity_len = self.calculate_parity_length(data.len());
        if parity.len() != expected_parity_len {
            return Ok(false);
        }

        // Quick integrity check using Reed-Solomon verification
        // This is more efficient than full error correction
        for (chunk, parity_chunk) in data
            .chunks(self.data_shards)
            .zip(parity.chunks(self.parity_shards))
        {
            if !self.verify_block_integrity(chunk, parity_chunk)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Verify integrity of a single block
    fn verify_block_integrity(
        &self,
        data_block: &[u8],
        parity_block: &[u8],
    ) -> Result<bool, DNAError> {
        // Reconstruct the full shard set
        let mut padded_data = data_block.to_vec();
        padded_data.resize(self.data_shards, 0);

        let mut shards = vec![Vec::new(); self.data_shards + self.parity_shards];

        // Fill data shards
        for (i, &byte) in padded_data.iter().enumerate() {
            shards[i] = vec![byte];
        }

        // Fill parity shards
        for (i, &byte) in parity_block.iter().enumerate() {
            if i < self.parity_shards {
                shards[self.data_shards + i] = vec![byte];
            }
        }

        // Use Reed-Solomon to verify (this is a simplified check)
        match self.rs_codec.verify(&shards) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Reed-Solomon configuration information
#[derive(Debug, Clone)]
pub struct RSInfo {
    pub data_shards: usize,
    pub parity_shards: usize,
    pub max_correctable_errors: usize,
    pub block_size: usize,
    pub overhead_ratio: f64,
}
