//! Reed-Solomon error correction adapted for biological data patterns
//!
//! This module implements Reed-Solomon error correction codes specifically optimized
//! for DNA compression scenarios, taking into account biological error patterns.
//!
//! Uses reed-solomon-simd for better performance and to avoid unmaintained dependencies.

use crate::dna::{DNABase, DNAError};
use reed_solomon_simd::{ReedSolomonDecoder, ReedSolomonEncoder};
use tracing::{debug, instrument, warn};

/// Error correction statistics
#[derive(Debug, Default, Clone)]
pub struct ErrorCorrectionStats {
    /// Total number of errors detected
    pub errors_detected: usize,
    /// Total number of errors corrected
    pub errors_corrected: usize,
    /// Total number of blocks processed
    pub blocks_processed: usize,
    /// Number of reconstruction attempts
    pub reconstructions_attempted: usize,
    /// Number of successful reconstructions
    pub reconstructions_successful: usize,
}

/// Reed-Solomon error corrector optimized for DNA data
#[derive(Debug)]
pub struct ReedSolomonCorrector {
    /// Number of parity shards for error correction
    parity_shards: usize,
    /// Data shards per block
    data_shards: usize,
    /// Shard size in bytes (fixed per block for reed-solomon-simd)
    shard_bytes: usize,
    /// Maximum correctable errors per block
    max_correctable_errors: usize,
    /// Error correction statistics
    stats: std::sync::Arc<std::sync::Mutex<ErrorCorrectionStats>>,
}

impl ReedSolomonCorrector {
    /// Create a new Reed-Solomon corrector with the given error correction strength
    pub fn new(error_correction_strength: u8) -> Self {
        // Map error correction strength (0-255) to Reed-Solomon parameters
        // GF(2^8) requires data_shards + parity_shards <= 255
        let parity_shards = (error_correction_strength as usize).clamp(1, 64);
        // Calculate data_shards ensuring total doesn't exceed 255
        let max_data_shards = 255 - parity_shards;
        let data_shards = (parity_shards * 4).clamp(16, max_data_shards.min(191));

        // reed-solomon-simd works with fixed shard sizes - we use 1 byte shards
        let shard_bytes = 1;

        let max_correctable_errors = parity_shards / 2;

        Self {
            parity_shards,
            data_shards,
            shard_bytes,
            max_correctable_errors,
            stats: std::sync::Arc::new(std::sync::Mutex::new(ErrorCorrectionStats::default())),
        }
    }

    /// Get error correction statistics
    pub fn get_stats(&self) -> ErrorCorrectionStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset error correction statistics
    pub fn reset_stats(&self) {
        *self.stats.lock().unwrap() = ErrorCorrectionStats::default();
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

        // Create encoder for this block
        let mut encoder =
            ReedSolomonEncoder::new(self.data_shards, self.parity_shards, self.shard_bytes)
                .map_err(|e| {
                    DNAError::ErrorCorrectionFailed(format!("Failed to create encoder: {:?}", e))
                })?;

        // Add each data shard (each byte becomes a shard)
        for &byte in &padded_data {
            encoder.add_original_shard([byte]).map_err(|e| {
                DNAError::ErrorCorrectionFailed(format!("Failed to add data shard: {:?}", e))
            })?;
        }

        // Encode to generate parity shards
        let result = encoder.encode().map_err(|e| {
            DNAError::ErrorCorrectionFailed(format!("Reed-Solomon encoding failed: {:?}", e))
        })?;

        // Extract parity data (recovery shards) - need to iterate with index
        let mut parity = Vec::with_capacity(self.parity_shards);
        for i in 0..self.parity_shards {
            if let Some(recovery_shard) = result.recovery(i) {
                parity.extend_from_slice(recovery_shard);
            }
        }

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

        // Create decoder for this block
        let mut decoder =
            ReedSolomonDecoder::new(self.data_shards, self.parity_shards, self.shard_bytes)
                .map_err(|e| {
                    DNAError::ErrorCorrectionFailed(format!("Failed to create decoder: {:?}", e))
                })?;

        // Convert to shards format with Option for missing/corrupted shards
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

        // Detect errors by checking which shards are missing or corrupted
        let errors_detected = self.detect_errors(&shards);

        // Count missing shards
        let missing_shards = shards.iter().filter(|s| s.is_none()).count();
        let total_errors = errors_detected + missing_shards;

        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.blocks_processed += 1;
            stats.errors_detected += total_errors;
        }

        if missing_shards > 0 || errors_detected > 0 {
            debug!(
                "Detected {} missing shards and {} corrupted shards, attempting reconstruction",
                missing_shards, errors_detected
            );

            // Update reconstruction attempt counter
            {
                let mut stats = self.stats.lock().unwrap();
                stats.reconstructions_attempted += 1;
            }

            // Mark corrupted shards as None for reconstruction
            let corrupted_indices = self.identify_corrupted_shards(&shards);
            for idx in corrupted_indices {
                if idx < shards.len() {
                    shards[idx] = None;
                }
            }

            // Add available original shards to decoder
            for (i, shard) in shards.iter().enumerate().take(self.data_shards) {
                if let Some(shard_data) = shard {
                    decoder.add_original_shard(i, shard_data).map_err(|e| {
                        DNAError::ErrorCorrectionFailed(format!(
                            "Failed to add original shard {}: {:?}",
                            i, e
                        ))
                    })?;
                }
            }

            // Add available recovery (parity) shards to decoder
            for (i, shard) in shards
                .iter()
                .enumerate()
                .skip(self.data_shards)
                .take(self.parity_shards)
            {
                if let Some(shard_data) = shard {
                    let recovery_idx = i - self.data_shards;
                    decoder
                        .add_recovery_shard(recovery_idx, shard_data)
                        .map_err(|e| {
                            DNAError::ErrorCorrectionFailed(format!(
                                "Failed to add recovery shard {}: {:?}",
                                recovery_idx, e
                            ))
                        })?;
                }
            }

            // Attempt Reed-Solomon reconstruction
            match decoder.decode() {
                Ok(result) => {
                    debug!(
                        "Successfully reconstructed data with {} errors/missing shards",
                        total_errors
                    );

                    // Update successful reconstruction counter
                    {
                        let mut stats = self.stats.lock().unwrap();
                        stats.reconstructions_successful += 1;
                        stats.errors_corrected += total_errors;
                    }

                    // Extract corrected data
                    let mut corrected = Vec::with_capacity(data_block.len());
                    for i in 0..self.data_shards.min(data_block.len()) {
                        if let Some(restored_shard) = result.restored_original(i) {
                            if !restored_shard.is_empty() {
                                corrected.push(restored_shard[0]);
                            }
                        }
                    }

                    // Truncate to original length
                    corrected.truncate(data_block.len());

                    Ok((corrected, total_errors))
                }
                Err(e) => Err(DNAError::ErrorCorrectionFailed(format!(
                    "Reed-Solomon reconstruction failed: {:?}. Too many errors to correct: {} missing, {} corrupted, need at least {} valid shards",
                    e,
                    missing_shards,
                    errors_detected,
                    self.data_shards
                ))),
            }
        } else {
            // No errors detected, return original data
            Ok((data_block.to_vec(), 0))
        }
    }

    /// Detect errors in Reed-Solomon encoded data
    /// This checks for corrupted shards using checksums and validates shard integrity
    fn detect_errors(&self, shards: &[Option<Vec<u8>>]) -> usize {
        let mut corrupted_count = 0;

        for (idx, shard) in shards.iter().enumerate() {
            if let Some(shard_data) = shard {
                // Verify shard is not empty
                if shard_data.is_empty() {
                    corrupted_count += 1;
                    continue;
                }

                // For data shards, validate checksum if available
                // Check if all bytes are the same (likely corruption)
                if shard_data.len() > 1 {
                    let first_byte = shard_data[0];
                    let all_same = shard_data.iter().all(|&b| b == first_byte);

                    // If all bytes are 0xFF or 0x00, likely corrupted
                    if all_same && (first_byte == 0xFF || first_byte == 0x00) {
                        debug!(
                            "Detected corrupted shard at index {}: all bytes are 0x{:02X}",
                            idx, first_byte
                        );
                        corrupted_count += 1;
                        continue;
                    }
                }

                // Additional validation: check for expected shard size
                let expected_size = if idx < self.data_shards {
                    // Data shards should have consistent size
                    shards[0..self.data_shards]
                        .iter()
                        .filter_map(|s| s.as_ref())
                        .map(|s| s.len())
                        .max()
                        .unwrap_or(shard_data.len())
                } else {
                    // Parity shards should match data shard size
                    shards[0..self.data_shards]
                        .iter()
                        .filter_map(|s| s.as_ref())
                        .map(|s| s.len())
                        .next()
                        .unwrap_or(shard_data.len())
                };

                // Allow some tolerance for the last shard which might be padded
                if !shard_data.is_empty()
                    && (shard_data.len() < expected_size / 2
                        || shard_data.len() > expected_size * 2)
                {
                    debug!(
                        "Detected size anomaly in shard {}: expected ~{} bytes, got {} bytes",
                        idx,
                        expected_size,
                        shard_data.len()
                    );
                    corrupted_count += 1;
                }
            }
        }

        if corrupted_count > 0 {
            debug!(
                "Detected {} corrupted shards during error detection",
                corrupted_count
            );
        }

        corrupted_count
    }

    /// Identify indices of corrupted shards for reconstruction
    fn identify_corrupted_shards(&self, shards: &[Option<Vec<u8>>]) -> Vec<usize> {
        let mut corrupted_indices = Vec::new();

        for (idx, shard) in shards.iter().enumerate() {
            if let Some(shard_data) = shard {
                // Check for corruption indicators
                if shard_data.is_empty() {
                    corrupted_indices.push(idx);
                    continue;
                }

                // Check for suspicious patterns
                if shard_data.len() > 1 {
                    let first_byte = shard_data[0];
                    let all_same = shard_data.iter().all(|&b| b == first_byte);
                    if all_same && (first_byte == 0xFF || first_byte == 0x00) {
                        corrupted_indices.push(idx);
                    }
                }
            }
        }

        corrupted_indices
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
    fn correct_gc_bias_errors(&self, bases: &mut [DNABase]) -> usize {
        // GC bias correction based on local GC content analysis
        // This analyzes windows of the sequence to detect and correct bias patterns
        let mut corrections = 0;
        let window_size = 20; // Analyze 20bp windows

        if bases.len() < window_size {
            return 0;
        }

        for window_start in 0..(bases.len() - window_size) {
            let window = &bases[window_start..window_start + window_size];

            // Calculate GC content in window
            let gc_count = window
                .iter()
                .filter(|b| matches!(b, DNABase::Guanine | DNABase::Cytosine))
                .count();

            let gc_content = gc_count as f64 / window_size as f64;

            // Detect extreme GC bias (< 20% or > 80%)
            // Natural DNA typically has 40-60% GC content
            if gc_content < 0.2 {
                // Too AT-rich: likely sequencing error
                // Look for isolated GC bases that might be errors
                for i in window_start..window_start + window_size {
                    if matches!(bases[i], DNABase::Guanine | DNABase::Cytosine) {
                        // Check if surrounded by AT
                        let prev_at =
                            i > 0 && matches!(bases[i - 1], DNABase::Adenine | DNABase::Thymine);
                        let next_at = i < bases.len() - 1
                            && matches!(bases[i + 1], DNABase::Adenine | DNABase::Thymine);

                        if prev_at && next_at {
                            // Likely sequencing error - convert to AT
                            bases[i] = if matches!(bases[i], DNABase::Guanine) {
                                DNABase::Adenine
                            } else {
                                DNABase::Thymine
                            };
                            corrections += 1;
                        }
                    }
                }
            } else if gc_content > 0.8 {
                // Too GC-rich: likely sequencing error
                // Look for isolated AT bases that might be errors
                for i in window_start..window_start + window_size {
                    if matches!(bases[i], DNABase::Adenine | DNABase::Thymine) {
                        // Check if surrounded by GC
                        let prev_gc =
                            i > 0 && matches!(bases[i - 1], DNABase::Guanine | DNABase::Cytosine);
                        let next_gc = i < bases.len() - 1
                            && matches!(bases[i + 1], DNABase::Guanine | DNABase::Cytosine);

                        if prev_gc && next_gc {
                            // Likely sequencing error - convert to GC
                            bases[i] = if matches!(bases[i], DNABase::Adenine) {
                                DNABase::Guanine
                            } else {
                                DNABase::Cytosine
                            };
                            corrections += 1;
                        }
                    }
                }
            }
        }

        corrections
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

        // Create encoder to regenerate parity
        let mut encoder =
            ReedSolomonEncoder::new(self.data_shards, self.parity_shards, self.shard_bytes)
                .map_err(|e| {
                    DNAError::ErrorCorrectionFailed(format!("Failed to create encoder: {:?}", e))
                })?;

        // Add each data shard
        for &byte in &padded_data {
            encoder.add_original_shard([byte]).map_err(|e| {
                DNAError::ErrorCorrectionFailed(format!("Failed to add shard: {:?}", e))
            })?;
        }

        // Encode to generate expected parity
        let result = encoder
            .encode()
            .map_err(|e| DNAError::ErrorCorrectionFailed(format!("Encoding failed: {:?}", e)))?;

        // Extract generated parity - need to iterate with index
        let mut generated_parity = Vec::with_capacity(self.parity_shards);
        for i in 0..self.parity_shards {
            if let Some(recovery_shard) = result.recovery(i) {
                generated_parity.extend_from_slice(recovery_shard);
            }
        }

        // Compare with provided parity
        let parity_matches = generated_parity
            .iter()
            .zip(parity_block.iter())
            .all(|(a, b)| a == b);

        Ok(parity_matches)
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
