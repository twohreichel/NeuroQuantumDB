//! WASM DNA compression tests
//!
//! Tests for the DNA compression module including:
//! - DNA base encoding/decoding
//! - Sequence compression roundtrip
//! - Binary data compression
//! - Compression ratio verification
//! - Error handling for invalid input

use wasm_bindgen_test::*;

use neuroquantum_wasm::dna_compression::{CompressedDNA, DNABase, WasmDNACompressor};

wasm_bindgen_test_configure!(run_in_browser);

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
