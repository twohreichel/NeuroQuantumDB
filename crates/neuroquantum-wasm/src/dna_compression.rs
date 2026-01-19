#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value,
    clippy::unused_self,
    clippy::missing_const_for_fn
)]
//! WASM-compatible DNA Compression Module
//!
//! This module provides a browser-compatible DNA compression implementation
//! using quaternary (2-bit) encoding. Unlike the core implementation, this
//! version avoids SIMD, rayon, and Reed-Solomon dependencies for WASM compatibility.
//!
//! ## Compression Algorithm
//!
//! 1. **Quaternary Encoding**: Each byte (8 bits) is encoded as 4 DNA bases (2 bits each)
//!    - A = 00, T = 01, G = 10, C = 11
//!
//! 2. **K-mer Dictionary Compression**: Common patterns are replaced with dictionary references
//!
//! 3. **CRC32 Checksum**: Data integrity verification

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

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
    /// Create `DNABase` from 2-bit value
    #[inline]
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            | 0b00 => Self::Adenine,
            | 0b01 => Self::Thymine,
            | 0b10 => Self::Guanine,
            | 0b11 => Self::Cytosine,
            | _ => unreachable!(),
        }
    }

    /// Get the 2-bit representation of this base
    #[inline]
    #[must_use]
    pub const fn to_bits(self) -> u8 {
        self as u8
    }

    /// Get the ASCII character representation
    #[must_use]
    pub const fn to_char(self) -> char {
        match self {
            | Self::Adenine => 'A',
            | Self::Thymine => 'T',
            | Self::Guanine => 'G',
            | Self::Cytosine => 'C',
        }
    }

    /// Create `DNABase` from ASCII character
    pub fn from_char(c: char) -> Result<Self, String> {
        match c.to_ascii_uppercase() {
            | 'A' => Ok(Self::Adenine),
            | 'T' => Ok(Self::Thymine),
            | 'G' => Ok(Self::Guanine),
            | 'C' => Ok(Self::Cytosine),
            | _ => Err(format!("Invalid DNA base character: {c}")),
        }
    }
}

/// Compressed DNA data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedDNA {
    /// Compressed binary data (quaternary encoded + dictionary compressed)
    pub data: Vec<u8>,
    /// Original data length before compression
    pub original_length: usize,
    /// CRC32 checksum for integrity verification
    pub checksum: u32,
    /// Compression metadata
    pub metadata: CompressionMetadata,
}

/// Metadata about the compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetadata {
    /// Algorithm version
    pub version: u8,
    /// Compression ratio achieved
    pub compression_ratio: f64,
    /// Dictionary for decompression (pattern -> id mapping as serializable format)
    pub dictionary: Option<Vec<(Vec<u8>, u16)>>,
    /// Whether the input was a DNA sequence string
    pub is_dna_sequence: bool,
}

/// WASM-compatible DNA compressor
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmDNACompressor {
    /// Enable dictionary compression
    enable_dictionary: bool,
    /// Maximum dictionary size
    max_dictionary_size: usize,
}

#[wasm_bindgen]
impl WasmDNACompressor {
    /// Create a new DNA compressor with default settings
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            enable_dictionary: true,
            max_dictionary_size: 256,
        }
    }

    /// Create a compressor with custom dictionary settings
    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(enable_dictionary: bool, max_dictionary_size: usize) -> Self {
        Self {
            enable_dictionary,
            max_dictionary_size,
        }
    }

    /// Compress a DNA sequence string (A, T, G, C characters)
    ///
    /// This is optimized for DNA sequence data where each character
    /// represents a DNA base and can be encoded in 2 bits.
    #[wasm_bindgen(js_name = compressDnaSequence)]
    pub fn compress_dna_sequence(&self, sequence: &str) -> Result<Vec<u8>, JsValue> {
        // Validate and convert DNA sequence to bases
        let bases = self.parse_dna_sequence(sequence)?;

        // Pack bases into bytes (4 bases per byte)
        let packed = self.pack_bases(&bases);

        // Build compressed structure
        let checksum = self.calculate_crc32(&packed);
        let compression_ratio = if sequence.is_empty() {
            1.0
        } else {
            packed.len() as f64 / sequence.len() as f64
        };

        let compressed = CompressedDNA {
            data: packed,
            original_length: sequence.len(),
            checksum,
            metadata: CompressionMetadata {
                version: 1,
                compression_ratio,
                dictionary: None,
                is_dna_sequence: true,
            },
        };

        // Serialize to bytes
        serde_json::to_vec(&compressed)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Decompress a DNA sequence back to string
    #[wasm_bindgen(js_name = decompressDnaSequence)]
    pub fn decompress_dna_sequence(&self, compressed: Vec<u8>) -> Result<String, JsValue> {
        // Deserialize
        let compressed_data: CompressedDNA = serde_json::from_slice(&compressed)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {e}")))?;

        if !compressed_data.metadata.is_dna_sequence {
            return Err(JsValue::from_str("Data was not compressed as DNA sequence"));
        }

        // Verify checksum
        let calculated_checksum = self.calculate_crc32(&compressed_data.data);
        if calculated_checksum != compressed_data.checksum {
            return Err(JsValue::from_str(&format!(
                "Checksum mismatch: expected {:08x}, got {:08x}",
                compressed_data.checksum, calculated_checksum
            )));
        }

        // Unpack bases
        let bases = self.unpack_bases(&compressed_data.data, compressed_data.original_length);

        // Convert to string
        Ok(bases.iter().map(|b| b.to_char()).collect())
    }

    /// Compress arbitrary binary data using DNA-style encoding
    ///
    /// This applies quaternary encoding and optional dictionary compression
    /// to any binary data, not just DNA sequences.
    #[wasm_bindgen(js_name = compressBytes)]
    pub fn compress_bytes(&self, data: &[u8]) -> Result<Vec<u8>, JsValue> {
        if data.is_empty() {
            let compressed = CompressedDNA {
                data: Vec::new(),
                original_length: 0,
                checksum: 0,
                metadata: CompressionMetadata {
                    version: 1,
                    compression_ratio: 1.0,
                    dictionary: None,
                    is_dna_sequence: false,
                },
            };
            return serde_json::to_vec(&compressed)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")));
        }

        // Apply dictionary compression if enabled
        let (compressed_data, dictionary) = if self.enable_dictionary && data.len() >= 64 {
            self.apply_dictionary_compression(data)
        } else {
            (data.to_vec(), None)
        };

        let checksum = self.calculate_crc32(&compressed_data);
        let compression_ratio = compressed_data.len() as f64 / data.len() as f64;

        let compressed = CompressedDNA {
            data: compressed_data,
            original_length: data.len(),
            checksum,
            metadata: CompressionMetadata {
                version: 1,
                compression_ratio,
                dictionary,
                is_dna_sequence: false,
            },
        };

        serde_json::to_vec(&compressed)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Decompress binary data
    #[wasm_bindgen(js_name = decompressBytes)]
    pub fn decompress_bytes(&self, compressed: Vec<u8>) -> Result<Vec<u8>, JsValue> {
        let compressed_data: CompressedDNA = serde_json::from_slice(&compressed)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {e}")))?;

        // Verify checksum
        let calculated_checksum = self.calculate_crc32(&compressed_data.data);
        if calculated_checksum != compressed_data.checksum {
            return Err(JsValue::from_str(&format!(
                "Checksum mismatch: expected {:08x}, got {:08x}",
                compressed_data.checksum, calculated_checksum
            )));
        }

        // Apply dictionary decompression if needed
        if let Some(ref dict_entries) = compressed_data.metadata.dictionary {
            let dictionary: HashMap<u16, Vec<u8>> =
                dict_entries.iter().map(|(k, v)| (*v, k.clone())).collect();
            self.apply_dictionary_decompression(&compressed_data.data, &dictionary)
        } else {
            Ok(compressed_data.data)
        }
    }

    /// Get compression statistics as JSON string
    #[wasm_bindgen(js_name = getCompressionStats)]
    pub fn get_compression_stats(&self, compressed: Vec<u8>) -> Result<JsValue, JsValue> {
        let compressed_data: CompressedDNA = serde_json::from_slice(&compressed)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {e}")))?;

        let stats = serde_json::json!({
            "original_size": compressed_data.original_length,
            "compressed_size": compressed_data.data.len(),
            "compression_ratio": compressed_data.metadata.compression_ratio,
            "version": compressed_data.metadata.version,
            "has_dictionary": compressed_data.metadata.dictionary.is_some(),
            "is_dna_sequence": compressed_data.metadata.is_dna_sequence,
        });

        serde_wasm_bindgen::to_value(&stats)
            .map_err(|e| JsValue::from_str(&format!("Stats serialization error: {e}")))
    }
}

// Internal implementation methods
impl WasmDNACompressor {
    /// Parse a DNA sequence string into bases
    fn parse_dna_sequence(&self, sequence: &str) -> Result<Vec<DNABase>, JsValue> {
        sequence
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| DNABase::from_char(c).map_err(|e| JsValue::from_str(&e)))
            .collect()
    }

    /// Pack DNA bases into bytes (4 bases per byte)
    fn pack_bases(&self, bases: &[DNABase]) -> Vec<u8> {
        let mut packed = Vec::with_capacity(bases.len().div_ceil(4));

        for chunk in bases.chunks(4) {
            let mut byte = 0u8;
            for (i, base) in chunk.iter().enumerate() {
                byte |= base.to_bits() << (6 - i * 2);
            }
            packed.push(byte);
        }

        packed
    }

    /// Unpack bytes back to DNA bases
    fn unpack_bases(&self, packed: &[u8], original_length: usize) -> Vec<DNABase> {
        let mut bases = Vec::with_capacity(original_length);

        for &byte in packed {
            if bases.len() >= original_length {
                break;
            }

            for i in 0..4 {
                if bases.len() >= original_length {
                    break;
                }
                let bits = (byte >> (6 - i * 2)) & 0b11;
                bases.push(DNABase::from_bits(bits));
            }
        }

        bases
    }

    /// Calculate CRC32 checksum
    fn calculate_crc32(&self, data: &[u8]) -> u32 {
        // Simple CRC32 implementation for WASM (no external dependencies)
        let mut crc = 0xFFFF_FFFFu32;
        for byte in data {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB8_8320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    /// Apply dictionary compression to data
    #[allow(clippy::type_complexity)]
    fn apply_dictionary_compression(&self, data: &[u8]) -> (Vec<u8>, Option<Vec<(Vec<u8>, u16)>>) {
        const MIN_PATTERN_LEN: usize = 4;
        const MAX_PATTERN_LEN: usize = 16;
        const MIN_FREQUENCY: usize = 3;

        // Build frequency map of patterns
        let mut pattern_freq: HashMap<Vec<u8>, usize> = HashMap::new();

        for len in MIN_PATTERN_LEN..=MAX_PATTERN_LEN.min(data.len()) {
            for window in data.windows(len) {
                *pattern_freq.entry(window.to_vec()).or_insert(0) += 1;
            }
        }

        // Select patterns that provide compression benefit
        let mut patterns: Vec<_> = pattern_freq
            .into_iter()
            .filter(|(pattern, freq)| *freq >= MIN_FREQUENCY && pattern.len() >= MIN_PATTERN_LEN)
            .collect();

        // Sort by compression benefit (frequency * length)
        patterns.sort_by_key(|(pattern, freq)| std::cmp::Reverse(*freq * pattern.len()));

        // Build dictionary
        let mut dictionary: HashMap<Vec<u8>, u16> = HashMap::new();
        let mut dict_id = 256u16; // Start after single-byte values

        for (pattern, _) in patterns.iter().take(self.max_dictionary_size) {
            if dict_id > 0xFEFF {
                break;
            }
            dictionary.insert(pattern.clone(), dict_id);
            dict_id += 1;
        }

        if dictionary.is_empty() {
            return (data.to_vec(), None);
        }

        // Apply compression
        let mut compressed = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            let mut matched = false;

            // Try to match longest pattern first
            for len in (MIN_PATTERN_LEN..=MAX_PATTERN_LEN.min(data.len() - i)).rev() {
                if i + len <= data.len() {
                    let pattern = &data[i..i + len];
                    if let Some(&id) = dictionary.get(pattern) {
                        // Encode as: [0xFF][id_high][id_low]
                        compressed.push(0xFF);
                        compressed.push((id >> 8) as u8);
                        compressed.push(id as u8);
                        i += len;
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                // Escape 0xFF bytes
                if data[i] == 0xFF {
                    compressed.push(0xFF);
                    compressed.push(0xFF);
                } else {
                    compressed.push(data[i]);
                }
                i += 1;
            }
        }

        let dict_vec: Vec<(Vec<u8>, u16)> = dictionary.into_iter().collect();
        (compressed, Some(dict_vec))
    }

    /// Apply dictionary decompression
    fn apply_dictionary_decompression(
        &self,
        data: &[u8],
        dictionary: &HashMap<u16, Vec<u8>>,
    ) -> Result<Vec<u8>, JsValue> {
        let mut decompressed = Vec::with_capacity(data.len() * 2);
        let mut i = 0;

        while i < data.len() {
            if data[i] == 0xFF {
                if i + 1 >= data.len() {
                    return Err(JsValue::from_str("Truncated escape sequence"));
                }

                if data[i + 1] == 0xFF {
                    // Escaped 0xFF byte
                    decompressed.push(0xFF);
                    i += 2;
                } else if i + 2 < data.len() {
                    // Dictionary reference
                    let id = u16::from(data[i + 1]) << 8 | u16::from(data[i + 2]);
                    if let Some(pattern) = dictionary.get(&id) {
                        decompressed.extend_from_slice(pattern);
                    } else {
                        return Err(JsValue::from_str(&format!(
                            "Unknown dictionary reference: {id}"
                        )));
                    }
                    i += 3;
                } else {
                    return Err(JsValue::from_str("Truncated dictionary reference"));
                }
            } else {
                decompressed.push(data[i]);
                i += 1;
            }
        }

        Ok(decompressed)
    }
}

impl Default for WasmDNACompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    use super::*;

    #[wasm_bindgen_test]
    fn test_dna_base_encoding() {
        assert_eq!(DNABase::from_bits(0b00), DNABase::Adenine);
        assert_eq!(DNABase::from_bits(0b01), DNABase::Thymine);
        assert_eq!(DNABase::from_bits(0b10), DNABase::Guanine);
        assert_eq!(DNABase::from_bits(0b11), DNABase::Cytosine);

        assert_eq!(DNABase::Adenine.to_char(), 'A');
        assert_eq!(DNABase::Thymine.to_char(), 'T');
        assert_eq!(DNABase::Guanine.to_char(), 'G');
        assert_eq!(DNABase::Cytosine.to_char(), 'C');
    }

    #[wasm_bindgen_test]
    fn test_dna_sequence_compression_roundtrip() {
        let compressor = WasmDNACompressor::new();
        let sequence = "ATGCATGCATGC";

        let compressed = compressor.compress_dna_sequence(sequence).unwrap();
        let decompressed = compressor.decompress_dna_sequence(compressed).unwrap();

        assert_eq!(decompressed, sequence);
    }

    #[wasm_bindgen_test]
    fn test_empty_sequence() {
        let compressor = WasmDNACompressor::new();

        let compressed = compressor.compress_dna_sequence("").unwrap();
        let decompressed = compressor.decompress_dna_sequence(compressed).unwrap();

        assert_eq!(decompressed, "");
    }

    #[wasm_bindgen_test]
    fn test_binary_compression_roundtrip() {
        let compressor = WasmDNACompressor::new();
        let data = b"Hello, WASM DNA compression!";

        let compressed = compressor.compress_bytes(data).unwrap();
        let decompressed = compressor.decompress_bytes(compressed).unwrap();

        assert_eq!(decompressed, data);
    }

    #[wasm_bindgen_test]
    fn test_compression_ratio() {
        let compressor = WasmDNACompressor::new();
        let sequence = "ATGCATGCATGCATGCATGCATGCATGCATGC"; // 32 bases

        let compressed = compressor.compress_dna_sequence(sequence).unwrap();
        let compressed_data: CompressedDNA = serde_json::from_slice(&compressed).unwrap();

        // 32 DNA bases should pack into 8 bytes (4 bases per byte)
        assert_eq!(compressed_data.data.len(), 8);
        assert!(compressed_data.metadata.compression_ratio < 1.0);
    }

    #[wasm_bindgen_test]
    fn test_invalid_dna_character() {
        let compressor = WasmDNACompressor::new();

        let result = compressor.compress_dna_sequence("ATGX");
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_pack_unpack_bases() {
        let compressor = WasmDNACompressor::new();
        let bases = vec![
            DNABase::Adenine,
            DNABase::Thymine,
            DNABase::Guanine,
            DNABase::Cytosine,
            DNABase::Adenine,
        ];

        let packed = compressor.pack_bases(&bases);
        let unpacked = compressor.unpack_bases(&packed, bases.len());

        assert_eq!(unpacked, bases);
    }
}
