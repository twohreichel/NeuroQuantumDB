//! DNA-based compression system for NeuroQuantumDB
//!
//! This module implements a biologically-inspired compression algorithm using quaternary encoding
//! (4 DNA bases: A, T, G, C) with Reed-Solomon error correction and SIMD optimizations.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, instrument, warn};

pub mod encoder;
pub mod decoder;
pub mod error_correction;
pub mod compression;
pub mod simd;
pub mod benchmarks;

#[cfg(test)]
pub mod tests;

// Re-export types for easier access
pub use encoder::QuaternaryEncoder;
pub use decoder::QuaternaryDecoder;
pub use error_correction::ReedSolomonCorrector;

// Type alias for backward compatibility
pub type EncodedData = CompressedDNA;

/// DNA base representation using quaternary encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DNABase {
    /// Adenine - represents 00 in binary
    Adenine = 0b00,
    /// Thymine - represents 01 in binary
    Thymine = 0b01,
    /// Guanine - represents 10 in binary
    Guanine = 0b10,
    /// Cytosine - represents 11 in binary
    Cytosine = 0b11,
}

impl DNABase {
    /// Create DNABase from 2-bit value
    pub fn from_bits(bits: u8) -> Result<Self, DNAError> {
        match bits & 0b11 {
            0b00 => Ok(Self::Adenine),
            0b01 => Ok(Self::Thymine),
            0b10 => Ok(Self::Guanine),
            0b11 => Ok(Self::Cytosine),
            _ => unreachable!("Masked to 2 bits, impossible case"),
        }
    }

    /// Get the 2-bit representation of this base
    pub fn to_bits(self) -> u8 {
        self as u8
    }

    /// Get the ASCII character representation
    pub fn to_char(self) -> char {
        match self {
            Self::Adenine => 'A',
            Self::Thymine => 'T',
            Self::Guanine => 'G',
            Self::Cytosine => 'C',
        }
    }

    /// Create DNABase from ASCII character
    pub fn from_char(c: char) -> Result<Self, DNAError> {
        match c.to_ascii_uppercase() {
            'A' => Ok(Self::Adenine),
            'T' => Ok(Self::Thymine),
            'G' => Ok(Self::Guanine),
            'C' => Ok(Self::Cytosine),
            _ => Err(DNAError::InvalidBase(c as u8)),
        }
    }
}

/// Comprehensive error types for DNA compression operations
#[derive(Debug, Error)]
pub enum DNAError {
    #[error("Invalid DNA base: {0}")]
    InvalidBase(u8),

    #[error("Sequence length mismatch: expected {expected}, got {actual}")]
    LengthMismatch { expected: usize, actual: usize },

    #[error("Reed-Solomon error correction failed: {0}")]
    ErrorCorrectionFailed(String),

    #[error("Checksum verification failed: expected {expected:08x}, got {actual:08x}")]
    ChecksumMismatch { expected: u32, actual: u32 },

    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    #[error("SIMD operation failed: {0}")]
    SimdError(String),

    #[error("Memory allocation failed: {0}")]
    MemoryError(String),

    #[error("Invalid compression version: {0}")]
    InvalidVersion(u8),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// DNA sequence with metadata and error correction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNASequence {
    /// The actual DNA bases
    pub bases: Vec<DNABase>,
    /// Reed-Solomon parity data for error correction
    pub parity: Vec<u8>,
    /// Checksum for integrity verification
    pub checksum: u32,
    /// Original data length before compression
    pub original_length: usize,
    /// Compression metadata
    pub metadata: CompressionMetadata,
}

/// Metadata about the compression process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetadata {
    /// Algorithm version for backwards compatibility
    pub version: u8,
    /// Compression ratio achieved
    pub compression_ratio: f64,
    /// Error correction strength (number of correctable errors)
    pub error_correction_strength: u8,
    /// Dictionary used for pattern compression
    pub dictionary: Option<HashMap<Vec<u8>, u16>>,
    /// Timestamp of compression
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Compressed DNA data ready for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedDNA {
    /// The DNA sequence with error correction
    pub sequence: DNASequence,
    /// Compressed size in bytes
    pub compressed_size: usize,
    /// Performance metrics from compression
    pub metrics: CompressionMetrics,
}

/// Performance metrics collected during compression/decompression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetrics {
    /// Time taken for compression in microseconds
    pub compression_time_us: u64,
    /// Time taken for decompression in microseconds
    pub decompression_time_us: Option<u64>,
    /// Memory usage peak during operation
    pub peak_memory_bytes: usize,
    /// Number of errors corrected during decompression
    pub errors_corrected: usize,
}

/// Configuration for DNA compression operations
#[derive(Debug, Clone)]
pub struct DNACompressionConfig {
    /// Error correction strength (0-255, higher means more redundancy)
    pub error_correction_strength: u8,
    /// Enable SIMD optimizations
    pub enable_simd: bool,
    /// Enable dictionary compression for patterns
    pub enable_dictionary: bool,
    /// Maximum dictionary size
    pub max_dictionary_size: usize,
    /// Memory limit for compression operations (in bytes)
    pub memory_limit: usize,
    /// Number of threads for parallel operations
    pub thread_count: usize,
}

impl Default for DNACompressionConfig {
    fn default() -> Self {
        Self {
            error_correction_strength: 32, // Can correct up to 32 byte errors
            enable_simd: true,
            enable_dictionary: true,
            max_dictionary_size: 65536, // 64KB dictionary
            memory_limit: 1024 * 1024 * 1024, // 1GB limit
            thread_count: rayon::current_num_threads(),
        }
    }
}

/// Main DNA compression trait - async interface for database integration
#[async_trait]
pub trait DNACompressor: Send + Sync {
    /// Compress binary data into DNA-encoded format
    async fn compress(&self, data: &[u8]) -> Result<CompressedDNA, DNAError>;

    /// Decompress DNA-encoded data back to binary
    async fn decompress(&self, compressed: &CompressedDNA) -> Result<Vec<u8>, DNAError>;

    /// Get the compression ratio achieved by the last operation
    fn compression_ratio(&self) -> f64;

    /// Get the error correction strength
    fn error_correction_strength(&self) -> u8;

    /// Get performance statistics
    fn get_metrics(&self) -> CompressionMetrics;

    /// Validate compressed data integrity without full decompression
    async fn validate(&self, compressed: &CompressedDNA) -> Result<bool, DNAError>;
}

/// High-performance DNA compressor implementation
#[derive(Debug, Clone)]
pub struct QuantumDNACompressor {
    config: DNACompressionConfig,
    metrics: Arc<std::sync::Mutex<CompressionMetrics>>,
}

impl QuantumDNACompressor {
    /// Create a new DNA compressor with default configuration
    pub fn new() -> Self {
        Self::with_config(DNACompressionConfig::default())
    }

    /// Create a new DNA compressor with custom configuration
    pub fn with_config(config: DNACompressionConfig) -> Self {
        let metrics = Arc::new(std::sync::Mutex::new(CompressionMetrics {
            compression_time_us: 0,
            decompression_time_us: None,
            peak_memory_bytes: 0,
            errors_corrected: 0,
        }));

        Self {
            config,
            metrics,
        }
    }

    /// Update configuration
    pub fn update_config(&mut self, config: DNACompressionConfig) {
        self.config = config;
    }
}

#[async_trait]
impl DNACompressor for QuantumDNACompressor {
    #[instrument(skip(self, data), fields(data_len = data.len()))]
    async fn compress(&self, data: &[u8]) -> Result<CompressedDNA, DNAError> {
        let start_time = std::time::Instant::now();

        info!("Starting DNA compression for {} bytes", data.len());

        // Check memory limits
        if data.len() * 2 > self.config.memory_limit {
            return Err(DNAError::MemoryError(
                format!("Data size {} exceeds memory limit {}", data.len() * 2, self.config.memory_limit)
            ));
        }

        // Create encoder and corrector
        let mut encoder = QuaternaryEncoder::new(&self.config);
        let error_corrector = ReedSolomonCorrector::new(self.config.error_correction_strength);

        // Step 1: Dictionary compression if enabled
        let processed_data = if self.config.enable_dictionary {
            debug!("Applying dictionary compression");
            encoder.compress_with_dictionary(data).await?
        } else {
            data.to_vec()
        };

        // Step 2: Quaternary encoding
        debug!("Encoding to DNA bases");
        let bases = encoder.encode_to_bases(&processed_data).await?;

        // Step 3: Add Reed-Solomon error correction
        debug!("Adding Reed-Solomon error correction");
        let parity = error_corrector.generate_parity(&processed_data)?;

        // Step 4: Calculate checksum
        let checksum = crc32fast::hash(&processed_data);

        // Create metadata
        let compression_ratio = processed_data.len() as f64 / data.len() as f64;
        let metadata = CompressionMetadata {
            version: 1,
            compression_ratio,
            error_correction_strength: self.config.error_correction_strength,
            dictionary: encoder.get_dictionary(),
            timestamp: chrono::Utc::now(),
        };

        let sequence = DNASequence {
            bases,
            parity,
            checksum,
            original_length: data.len(),
            metadata,
        };

        let elapsed = start_time.elapsed();
        let compressed_size = sequence.bases.len() / 4 + sequence.parity.len();

        let metrics = CompressionMetrics {
            compression_time_us: elapsed.as_micros() as u64,
            decompression_time_us: None,
            peak_memory_bytes: processed_data.len() + sequence.bases.len() + sequence.parity.len(),
            errors_corrected: 0,
        };

        // Update stored metrics
        if let Ok(mut stored_metrics) = self.metrics.lock() {
            *stored_metrics = metrics.clone();
        }

        info!("DNA compression completed: {:.2}% ratio, {} μs",
              compression_ratio * 100.0, elapsed.as_micros());

        Ok(CompressedDNA {
            sequence,
            compressed_size,
            metrics,
        })
    }

    #[instrument(skip(self, compressed), fields(compressed_size = compressed.compressed_size))]
    async fn decompress(&self, compressed: &CompressedDNA) -> Result<Vec<u8>, DNAError> {
        let start_time = std::time::Instant::now();

        info!("Starting DNA decompression for {} compressed bytes", compressed.compressed_size);

        // Validate version compatibility
        if compressed.sequence.metadata.version != 1 {
            return Err(DNAError::InvalidVersion(compressed.sequence.metadata.version));
        }

        // Create decoder and corrector
        let decoder = QuaternaryDecoder::new(&self.config);
        let error_corrector = ReedSolomonCorrector::new(self.config.error_correction_strength);

        // Step 1: Decode DNA bases to binary
        debug!("Decoding DNA bases to binary");
        let mut decoded_data = decoder.decode_from_bases(&compressed.sequence.bases).await?;

        // Step 2: Apply Reed-Solomon error correction
        debug!("Applying Reed-Solomon error correction");
        let (corrected_data, errors_corrected) = error_corrector
            .correct_errors(&decoded_data, &compressed.sequence.parity)?;

        if errors_corrected > 0 {
            warn!("Corrected {} errors during decompression", errors_corrected);
            decoded_data = corrected_data;
        }

        // Step 3: Verify checksum
        let calculated_checksum = crc32fast::hash(&decoded_data);
        if calculated_checksum != compressed.sequence.checksum {
            return Err(DNAError::ChecksumMismatch {
                expected: compressed.sequence.checksum,
                actual: calculated_checksum,
            });
        }

        // Step 4: Apply dictionary decompression if needed
        let final_data = if let Some(ref dictionary) = compressed.sequence.metadata.dictionary {
            debug!("Applying dictionary decompression");
            decoder.decompress_with_dictionary(&decoded_data, dictionary).await?
        } else {
            decoded_data
        };

        // Verify final length
        if final_data.len() != compressed.sequence.original_length {
            return Err(DNAError::LengthMismatch {
                expected: compressed.sequence.original_length,
                actual: final_data.len(),
            });
        }

        let elapsed = start_time.elapsed();

        // Update metrics
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.decompression_time_us = Some(elapsed.as_micros() as u64);
            metrics.errors_corrected = errors_corrected;
        }

        info!("DNA decompression completed: {} bytes restored, {} μs, {} errors corrected",
              final_data.len(), elapsed.as_micros(), errors_corrected);

        Ok(final_data)
    }

    fn compression_ratio(&self) -> f64 {
        self.metrics.lock()
            .map(|m| m.compression_time_us as f64 / (m.decompression_time_us.unwrap_or(1) as f64))
            .unwrap_or(1.0)
    }

    fn error_correction_strength(&self) -> u8 {
        self.config.error_correction_strength
    }

    fn get_metrics(&self) -> CompressionMetrics {
        self.metrics.lock()
            .map(|m| m.clone())
            .unwrap_or_default()
    }

    async fn validate(&self, compressed: &CompressedDNA) -> Result<bool, DNAError> {
        // Quick validation without full decompression

        // Check version
        if compressed.sequence.metadata.version != 1 {
            return Ok(false);
        }

        // Check base sequence integrity
        if compressed.sequence.bases.is_empty() {
            return Ok(false);
        }

        // Verify Reed-Solomon parity length
        let error_corrector = ReedSolomonCorrector::new(self.config.error_correction_strength);
        let expected_parity_len = error_corrector.calculate_parity_length(compressed.compressed_size);
        if compressed.sequence.parity.len() != expected_parity_len {
            return Ok(false);
        }

        Ok(true)
    }
}

impl Default for QuantumDNACompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CompressionMetrics {
    fn default() -> Self {
        Self {
            compression_time_us: 0,
            decompression_time_us: None,
            peak_memory_bytes: 0,
            errors_corrected: 0,
        }
    }
}
