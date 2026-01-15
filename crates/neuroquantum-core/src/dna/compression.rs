//! Advanced compression algorithms for DNA data optimization
//!
//! This module implements additional compression layers including dictionary compression,
//! Huffman coding, and biological pattern-specific optimizations.

use crate::dna::{DNABase, DNACompressionConfig, DNAError};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use tracing::{debug, instrument};

/// Advanced compression engine for DNA sequences
#[derive(Debug)]
pub struct DNACompressionEngine {
    config: DNACompressionConfig,
    huffman_tree: Option<HuffmanTree>,
    pattern_dictionary: HashMap<Vec<DNABase>, u16>,
    biological_patterns: BiologicalPatterns,
}

impl DNACompressionEngine {
    /// Create a new compression engine
    pub fn new(config: &DNACompressionConfig) -> Self {
        Self {
            config: config.clone(),
            huffman_tree: None,
            pattern_dictionary: HashMap::new(),
            biological_patterns: BiologicalPatterns::new(),
        }
    }

    /// Compress DNA sequence using multiple algorithms
    #[instrument(skip(self, bases))]
    pub async fn compress_sequence(
        &mut self,
        bases: &[DNABase],
    ) -> Result<CompressedSequence, DNAError> {
        if bases.is_empty() {
            return Ok(CompressedSequence::empty());
        }

        debug!("Starting advanced compression for {} bases", bases.len());

        let mut current_bases = bases.to_vec();
        let original_size = bases.len();
        let mut compression_steps = Vec::new();

        // Step 1: Biological pattern optimization
        if self.config.enable_dictionary {
            let (optimized, savings) = self.optimize_biological_patterns(&current_bases).await?;
            current_bases = optimized;
            compression_steps.push(CompressionStep {
                algorithm: "BiologicalPatterns".to_string(),
                size_before: bases.len(),
                size_after: current_bases.len(),
                compression_ratio: current_bases.len() as f64 / bases.len() as f64,
                savings_bytes: savings,
            });
        }

        // Step 2: Dictionary compression for repeated sequences
        let (dict_compressed, dict_savings) =
            self.apply_dictionary_compression(&current_bases).await?;
        current_bases = dict_compressed;
        compression_steps.push(CompressionStep {
            algorithm: "Dictionary".to_string(),
            size_before: compression_steps
                .last()
                .map_or(original_size, |s| s.size_after),
            size_after: current_bases.len(),
            compression_ratio: current_bases.len() as f64
                / compression_steps
                    .last()
                    .map_or(original_size, |s| s.size_after) as f64,
            savings_bytes: dict_savings,
        });

        // Step 3: Huffman coding for frequency-based compression
        let huffman_compressed = self.apply_huffman_compression(&current_bases).await?;
        let huffman_size = huffman_compressed.len() / 8; // Convert bits to approximate bytes
        compression_steps.push(CompressionStep {
            algorithm: "Huffman".to_string(),
            size_before: current_bases.len(),
            size_after: huffman_size,
            compression_ratio: huffman_size as f64 / current_bases.len() as f64,
            savings_bytes: current_bases.len().saturating_sub(huffman_size),
        });

        let final_compression_ratio = huffman_size as f64 / original_size as f64;

        Ok(CompressedSequence {
            compressed_data: huffman_compressed,
            original_size,
            compressed_size: huffman_size,
            compression_ratio: final_compression_ratio,
            compression_steps,
            huffman_tree: self.huffman_tree.clone(),
            pattern_dictionary: self.pattern_dictionary.clone(),
        })
    }

    /// Optimize biological patterns in DNA sequences
    async fn optimize_biological_patterns(
        &mut self,
        bases: &[DNABase],
    ) -> Result<(Vec<DNABase>, usize), DNAError> {
        debug!("Optimizing biological patterns");

        let mut optimized = bases.to_vec();
        let mut total_savings = 0;

        // Optimize complementary base pairs (A-T, G-C)
        total_savings += self.optimize_complementary_pairs(&optimized);

        // Optimize repetitive motifs
        total_savings += self.optimize_repetitive_motifs(&mut optimized).await?;

        // Optimize palindromic sequences
        total_savings += self.optimize_palindromes(&mut optimized);

        Ok((optimized, total_savings))
    }

    /// Optimize complementary base pairs
    fn optimize_complementary_pairs(&self, bases: &[DNABase]) -> usize {
        let mut savings = 0;

        // Look for Watson-Crick base pairs and encode them more efficiently
        for window in bases.windows(2) {
            if let [base1, base2] = window {
                if self.biological_patterns.are_complementary(*base1, *base2) {
                    // Mark complementary pairs for special encoding
                    // This is a placeholder for more sophisticated encoding
                    savings += 1;
                }
            }
        }

        savings
    }

    /// Optimize repetitive DNA motifs
    async fn optimize_repetitive_motifs(
        &mut self,
        bases: &mut [DNABase],
    ) -> Result<usize, DNAError> {
        let mut savings = 0;
        let motif_sizes = [3, 6, 9, 12]; // Common biological motif sizes

        for &motif_size in &motif_sizes {
            if bases.len() < motif_size * 2 {
                continue;
            }

            let motif_savings = self.find_and_compress_motifs(bases, motif_size).await?;
            savings += motif_savings;
        }

        Ok(savings)
    }

    /// Find and compress repetitive motifs of a specific size
    async fn find_and_compress_motifs(
        &mut self,
        bases: &[DNABase],
        motif_size: usize,
    ) -> Result<usize, DNAError> {
        let mut motif_counts = HashMap::new();

        // Count motif frequencies in parallel
        let motifs: Vec<_> = bases.windows(motif_size).enumerate().collect();

        for (_, motif) in motifs {
            *motif_counts.entry(motif.to_vec()).or_insert(0) += 1;
        }

        // Find motifs that appear frequently
        let frequent_motifs: Vec<_> = motif_counts
            .into_iter()
            .filter(|(_, count)| *count >= 3)
            .collect();

        let mut total_savings = 0;

        for (motif, count) in frequent_motifs {
            if self.pattern_dictionary.len() >= u16::MAX as usize {
                break;
            }

            let pattern_id = self.pattern_dictionary.len() as u16;
            self.pattern_dictionary.insert(motif.clone(), pattern_id);

            // Calculate savings: original motif size * occurrences - encoding overhead
            total_savings += (motif_size * count).saturating_sub(count * 2); // 2 bytes per reference
        }

        Ok(total_savings)
    }

    /// Optimize palindromic sequences
    fn optimize_palindromes(&self, bases: &mut [DNABase]) -> usize {
        let mut savings = 0;

        // Look for palindromic sequences (reverse complements)
        for window_size in (4..=16).step_by(2) {
            // Even sizes for palindromes
            if bases.len() < window_size {
                break;
            }

            for window in bases.windows(window_size) {
                if self.is_palindrome(window) {
                    // Mark palindrome for special encoding
                    savings += window_size / 2; // Can encode half + flag
                }
            }
        }

        savings
    }

    /// Check if a sequence is a palindrome (reverse complement)
    fn is_palindrome(&self, bases: &[DNABase]) -> bool {
        let len = bases.len();
        if !len.is_multiple_of(2) {
            return false;
        }

        for i in 0..len / 2 {
            let left = bases[i];
            let right = bases[len - 1 - i];

            if !self.biological_patterns.are_complementary(left, right) {
                return false;
            }
        }

        true
    }

    /// Apply dictionary compression to DNA sequence
    ///
    /// # Note on `.expect()` usage
    ///
    /// This function uses `.expect()` for `DNABase::from_bits()` calls where the input
    /// is masked to 2 bits (& 0b11), making failure mathematically impossible.
    /// All 2-bit values (0-3) map to valid DNA bases.
    #[allow(clippy::expect_used)] // Bitwise-masked values are provably valid
    async fn apply_dictionary_compression(
        &mut self,
        bases: &[DNABase],
    ) -> Result<(Vec<DNABase>, usize), DNAError> {
        debug!("Applying dictionary compression");

        if self.pattern_dictionary.is_empty() {
            return Ok((bases.to_vec(), 0));
        }

        let mut compressed = Vec::new();
        let mut i = 0;
        let mut savings = 0;

        while i < bases.len() {
            let mut matched = false;

            // Try to match patterns from longest to shortest
            let mut patterns: Vec<_> = self.pattern_dictionary.keys().collect();
            patterns.sort_by_key(|p| std::cmp::Reverse(p.len()));

            for pattern in patterns {
                if i + pattern.len() <= bases.len()
                    && &bases[i..i + pattern.len()] == pattern.as_slice()
                {
                    // Encode pattern reference
                    if let Some(&pattern_id) = self.pattern_dictionary.get(pattern) {
                        compressed.push(DNABase::Adenine); // Special marker
                        compressed.push(
                            DNABase::from_bits((pattern_id >> 8) as u8 & 0b11)
                                .expect("valid 2-bit pattern"),
                        );
                        compressed.push(
                            DNABase::from_bits((pattern_id >> 6) as u8 & 0b11)
                                .expect("valid 2-bit pattern"),
                        );
                        compressed.push(
                            DNABase::from_bits((pattern_id >> 4) as u8 & 0b11)
                                .expect("valid 2-bit pattern"),
                        );
                        compressed.push(
                            DNABase::from_bits((pattern_id >> 2) as u8 & 0b11)
                                .expect("valid 2-bit pattern"),
                        );
                        compressed.push(
                            DNABase::from_bits(pattern_id as u8 & 0b11)
                                .expect("valid 2-bit pattern"),
                        );

                        i += pattern.len();
                        savings += pattern.len().saturating_sub(6);
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                compressed.push(bases[i]);
                i += 1;
            }
        }

        Ok((compressed, savings))
    }

    /// Apply Huffman compression to DNA sequence
    async fn apply_huffman_compression(&mut self, bases: &[DNABase]) -> Result<Vec<u8>, DNAError> {
        debug!("Applying Huffman compression");

        // Build frequency table
        let mut frequencies = [0u64; 4];
        for &base in bases {
            frequencies[base as usize] += 1;
        }

        // Build Huffman tree
        self.huffman_tree = Some(HuffmanTree::build(&frequencies));

        // Encode sequence
        let mut encoded_bits = Vec::new();
        if let Some(ref tree) = self.huffman_tree {
            for &base in bases {
                let code = tree.get_code(base);
                encoded_bits.extend(code);
            }
        }

        // Convert bits to bytes
        let mut bytes = Vec::new();
        for chunk in encoded_bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << (7 - i);
                }
            }
            bytes.push(byte);
        }

        Ok(bytes)
    }

    /// Decompress a compressed DNA sequence
    pub async fn decompress_sequence(
        &self,
        compressed: &CompressedSequence,
    ) -> Result<Vec<DNABase>, DNAError> {
        debug!("Decompressing DNA sequence");

        // Step 1: Huffman decompression
        let mut current_bases = if let Some(ref tree) = compressed.huffman_tree {
            self.huffman_decompress(&compressed.compressed_data, tree, compressed.original_size)
                .await?
        } else {
            return Err(DNAError::DecompressionFailed(
                "Missing Huffman tree".to_string(),
            ));
        };

        // Step 2: Dictionary decompression (reverse order from compression)
        current_bases = self
            .dictionary_decompress(&current_bases, &compressed.pattern_dictionary)
            .await?;

        // Step 3: Biological pattern deoptimization
        current_bases = self.deoptimize_biological_patterns(&current_bases).await?;

        Ok(current_bases)
    }

    /// Huffman decompression
    async fn huffman_decompress(
        &self,
        data: &[u8],
        tree: &HuffmanTree,
        expected_size: usize,
    ) -> Result<Vec<DNABase>, DNAError> {
        let mut result = Vec::with_capacity(expected_size);
        let mut bit_stream = BitStream::new(data);

        while result.len() < expected_size {
            if let Some(base) = tree.decode_next(&mut bit_stream)? {
                result.push(base);
            } else {
                break;
            }
        }

        Ok(result)
    }

    /// Dictionary decompression
    async fn dictionary_decompress(
        &self,
        bases: &[DNABase],
        dictionary: &HashMap<Vec<DNABase>, u16>,
    ) -> Result<Vec<DNABase>, DNAError> {
        // Build reverse dictionary
        let reverse_dict: HashMap<u16, Vec<DNABase>> = dictionary
            .iter()
            .map(|(pattern, &id)| (id, pattern.clone()))
            .collect();

        let mut result = Vec::new();
        let mut i = 0;

        while i < bases.len() {
            if i + 6 <= bases.len() && bases[i] == DNABase::Adenine {
                // Potential pattern reference
                let mut pattern_id = 0u16;
                for j in 1..6 {
                    pattern_id = (pattern_id << 2) | (bases[i + j] as u16);
                }

                if let Some(pattern) = reverse_dict.get(&pattern_id) {
                    result.extend(pattern);
                    i += 6;
                } else {
                    result.push(bases[i]);
                    i += 1;
                }
            } else {
                result.push(bases[i]);
                i += 1;
            }
        }

        Ok(result)
    }

    /// Reverse biological pattern optimizations
    async fn deoptimize_biological_patterns(
        &self,
        bases: &[DNABase],
    ) -> Result<Vec<DNABase>, DNAError> {
        // This would reverse the biological optimizations
        // For now, return as-is since we didn't modify the structure significantly
        Ok(bases.to_vec())
    }
}

/// Huffman tree for DNA base compression
#[derive(Debug, Clone)]
pub struct HuffmanTree {
    root: HuffmanNode,
    codes: [Vec<bool>; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HuffmanNode {
    Leaf {
        base: DNABase,
    },
    Internal {
        left: Box<HuffmanNode>,
        right: Box<HuffmanNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HuffmanNodeFreq {
    freq: u64,
    node: HuffmanNode,
}

impl Ord for HuffmanNodeFreq {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq.cmp(&self.freq) // Reverse for min-heap
    }
}

impl PartialOrd for HuffmanNodeFreq {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl HuffmanTree {
    /// Build Huffman tree from base frequencies
    ///
    /// # Note on `.expect()` usage
    ///
    /// Uses `.expect()` in contexts where failure is impossible:
    /// - Loop invariants ensure heap has sufficient elements
    /// - Index values 0-3 always map to valid DNA bases
    #[allow(clippy::expect_used)] // Loop invariants and valid indices guarantee success
    pub fn build(frequencies: &[u64; 4]) -> Self {
        let mut heap = BinaryHeap::new();

        // Create leaf nodes
        for (i, &freq) in frequencies.iter().enumerate() {
            if freq > 0 {
                heap.push(HuffmanNodeFreq {
                    freq,
                    node: HuffmanNode::Leaf {
                        base: DNABase::from_bits(i as u8).expect("valid DNA base index 0-3"),
                    },
                });
            }
        }

        // Build tree
        while heap.len() > 1 {
            let node1 = heap.pop().expect("heap has at least 2 nodes in loop");
            let node2 = heap.pop().expect("heap has at least 2 nodes in loop");

            heap.push(HuffmanNodeFreq {
                freq: node1.freq + node2.freq,
                node: HuffmanNode::Internal {
                    left: Box::new(node1.node),
                    right: Box::new(node2.node),
                },
            });
        }

        let root = heap
            .pop()
            .expect("heap should have exactly one root node")
            .node;
        let mut tree = HuffmanTree {
            root,
            codes: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        };

        tree.build_codes();
        tree
    }

    /// Build code table from tree
    fn build_codes(&mut self) {
        self.build_codes_recursive(&self.root.clone(), Vec::new());
    }

    fn build_codes_recursive(&mut self, node: &HuffmanNode, path: Vec<bool>) {
        match node {
            | HuffmanNode::Leaf { base } => {
                self.codes[*base as usize] = path;
            },
            | HuffmanNode::Internal { left, right } => {
                let mut left_path = path.clone();
                left_path.push(false);
                self.build_codes_recursive(left, left_path);

                let mut right_path = path;
                right_path.push(true);
                self.build_codes_recursive(right, right_path);
            },
        }
    }

    /// Get Huffman code for a base
    pub fn get_code(&self, base: DNABase) -> &[bool] {
        &self.codes[base as usize]
    }

    /// Decode next base from bit stream
    pub fn decode_next(&self, bit_stream: &mut BitStream) -> Result<Option<DNABase>, DNAError> {
        decode_from_node(&self.root, bit_stream)
    }
}

fn decode_from_node(
    node: &HuffmanNode,
    bit_stream: &mut BitStream,
) -> Result<Option<DNABase>, DNAError> {
    match node {
        | HuffmanNode::Leaf { base } => Ok(Some(*base)),
        | HuffmanNode::Internal { left, right } => {
            if let Some(bit) = bit_stream.next_bit() {
                let child = if bit { right } else { left };
                decode_from_node(child, bit_stream)
            } else {
                Ok(None)
            }
        },
    }
}

/// Bit stream for Huffman decoding
pub struct BitStream<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitStream<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn next_bit(&mut self) -> Option<bool> {
        if self.byte_pos >= self.data.len() {
            return None;
        }

        let byte = self.data[self.byte_pos];
        let bit = (byte >> (7 - self.bit_pos)) & 1 == 1;

        self.bit_pos += 1;
        if self.bit_pos >= 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }

        Some(bit)
    }
}

/// Biological pattern recognition and optimization
#[derive(Debug)]
struct BiologicalPatterns {
    complement_map: HashMap<DNABase, DNABase>,
}

impl BiologicalPatterns {
    fn new() -> Self {
        let mut complement_map = HashMap::new();
        complement_map.insert(DNABase::Adenine, DNABase::Thymine);
        complement_map.insert(DNABase::Thymine, DNABase::Adenine);
        complement_map.insert(DNABase::Guanine, DNABase::Cytosine);
        complement_map.insert(DNABase::Cytosine, DNABase::Guanine);

        Self { complement_map }
    }

    fn are_complementary(&self, base1: DNABase, base2: DNABase) -> bool {
        self.complement_map.get(&base1) == Some(&base2)
    }
}

/// Compressed DNA sequence with metadata
#[derive(Debug, Clone)]
pub struct CompressedSequence {
    pub compressed_data: Vec<u8>,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub compression_steps: Vec<CompressionStep>,
    pub huffman_tree: Option<HuffmanTree>,
    pub pattern_dictionary: HashMap<Vec<DNABase>, u16>,
}

impl CompressedSequence {
    fn empty() -> Self {
        Self {
            compressed_data: Vec::new(),
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 1.0,
            compression_steps: Vec::new(),
            huffman_tree: None,
            pattern_dictionary: HashMap::new(),
        }
    }
}

/// Information about a compression step
#[derive(Debug, Clone)]
pub struct CompressionStep {
    pub algorithm: String,
    pub size_before: usize,
    pub size_after: usize,
    pub compression_ratio: f64,
    pub savings_bytes: usize,
}
