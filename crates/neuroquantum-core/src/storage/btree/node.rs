//! B+ Tree Node Implementation
//!
//! Contains internal nodes and leaf nodes for the B+ Tree structure
//! with optional prefix compression for memory efficiency.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::compression::{longest_common_prefix, KeyDelta};
use super::{Key, Value};

/// Internal node in the B+ Tree
///
/// Contains keys and child pointers. Does not contain actual values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalNode {
    /// Keys for routing to children
    pub keys: Vec<Key>,
    /// Child page IDs (always one more than keys)
    pub children: Vec<u64>,
    /// Maximum capacity (order - 1)
    max_keys: usize,
}

impl InternalNode {
    /// Create a new internal node
    #[must_use]
    pub fn new(order: usize) -> Self {
        Self {
            keys: Vec::with_capacity(order),
            children: Vec::with_capacity(order + 1),
            max_keys: order - 1,
        }
    }

    /// Check if the node is full
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.keys.len() >= self.max_keys
    }

    /// Find the index of the child to follow for a given key
    #[must_use]
    pub fn find_child_index(&self, key: &Key) -> usize {
        for (i, k) in self.keys.iter().enumerate() {
            if key < k {
                return i;
            }
        }
        self.keys.len()
    }

    /// Insert a child pointer with its associated key
    pub fn insert_child(&mut self, key: Key, child_page_id: u64) -> Result<()> {
        if self.is_full() {
            return Err(anyhow!("Internal node is full"));
        }

        // Find insertion position
        let pos = self.keys.binary_search(&key).unwrap_or_else(|e| e);

        // Insert key and child
        self.keys.insert(pos, key);
        self.children.insert(pos + 1, child_page_id);

        Ok(())
    }

    /// Split the internal node when it becomes full
    ///
    /// Returns: (`promoted_key`, `new_right_node`)
    pub fn split(&mut self) -> (Key, Self) {
        let mid = self.keys.len() / 2;

        // Key to promote to parent
        let promoted_key = self.keys[mid].clone();

        // Create new right node with upper half
        let mut right_node = Self::new(self.max_keys + 1);
        right_node.keys = self.keys.split_off(mid + 1);
        right_node.children = self.children.split_off(mid + 1);

        // Remove the promoted key from left node
        self.keys.pop();

        (promoted_key, right_node)
    }
}

/// Leaf node in the B+ Tree
///
/// Contains actual key-value pairs and a pointer to the next leaf for range scans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafNode {
    /// Key-value entries
    pub entries: Vec<(Key, Value)>,
    /// Pointer to next leaf node (for range scans)
    pub next_leaf: Option<u64>,
    /// Maximum capacity
    max_entries: usize,
}

impl LeafNode {
    /// Create a new leaf node
    #[must_use]
    pub fn new(order: usize) -> Self {
        Self {
            entries: Vec::with_capacity(order),
            next_leaf: None,
            max_entries: order - 1,
        }
    }

    /// Check if the node is full
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.entries.len() >= self.max_entries
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: Key, value: Value) -> Result<()> {
        if self.is_full() {
            return Err(anyhow!("Leaf node is full"));
        }

        // Find insertion position
        match self.entries.binary_search_by(|(k, _)| k.cmp(&key)) {
            | Ok(_) => Err(anyhow!("Duplicate key")),
            | Err(pos) => {
                self.entries.insert(pos, (key, value));
                Ok(())
            },
        }
    }

    /// Insert or update a key-value pair (upsert)
    pub fn upsert(&mut self, key: Key, value: Value) -> Result<bool> {
        // Find insertion position
        match self.entries.binary_search_by(|(k, _)| k.cmp(&key)) {
            | Ok(pos) => {
                // Key exists, update the value
                self.entries[pos].1 = value;
                Ok(false) // false = updated existing key
            },
            | Err(pos) => {
                // Key doesn't exist, insert new entry
                if self.is_full() {
                    return Err(anyhow!("Leaf node is full"));
                }
                self.entries.insert(pos, (key, value));
                Ok(true) // true = inserted new key
            },
        }
    }

    /// Search for a value by key
    #[must_use]
    pub fn search(&self, key: &Key) -> Option<Value> {
        self.entries
            .binary_search_by(|(k, _)| k.cmp(key))
            .ok()
            .map(|idx| self.entries[idx].1)
    }

    /// Delete a key from the leaf
    pub fn delete(&mut self, key: &Key) -> bool {
        if let Ok(pos) = self.entries.binary_search_by(|(k, _)| k.cmp(key)) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// Split the leaf node when it becomes full
    ///
    /// Returns: (`promoted_key`, `new_right_leaf`)
    pub fn split(&mut self) -> (Key, Self) {
        let mid = self.entries.len() / 2;

        // Create new right leaf with upper half
        let mut right_leaf = Self::new(self.max_entries + 1);
        right_leaf.entries = self.entries.split_off(mid);

        // Key to promote (smallest key in right leaf)
        let promoted_key = right_leaf.entries[0].0.clone();

        (promoted_key, right_leaf)
    }

    /// Get the minimum key in this leaf
    #[must_use]
    pub fn min_key(&self) -> Option<&Key> {
        self.entries.first().map(|(k, _)| k)
    }

    /// Get the maximum key in this leaf
    #[must_use]
    pub fn max_key(&self) -> Option<&Key> {
        self.entries.last().map(|(k, _)| k)
    }
}

/// Compressed leaf node using prefix compression
///
/// Stores a common prefix for all keys in the node and only the suffixes,
/// resulting in 30-60% memory savings for keys with common prefixes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedLeafNode {
    /// Common prefix for all keys in this node
    pub prefix: Vec<u8>,
    /// Suffixes only - much smaller!
    pub entries: Vec<(Vec<u8>, Value)>,
    /// Pointer to next leaf node (for range scans)
    pub next_leaf: Option<u64>,
    /// Maximum capacity
    max_entries: usize,
}

impl CompressedLeafNode {
    /// Create a new compressed leaf node
    #[must_use]
    pub fn new(order: usize) -> Self {
        Self {
            prefix: Vec::new(),
            entries: Vec::with_capacity(order),
            next_leaf: None,
            max_entries: order - 1,
        }
    }

    /// Create from an uncompressed leaf node
    #[must_use]
    pub fn from_leaf_node(leaf: &LeafNode) -> Self {
        let mut compressed = Self::new(leaf.max_entries + 1);
        compressed.next_leaf = leaf.next_leaf;

        if leaf.entries.is_empty() {
            return compressed;
        }

        // Calculate common prefix for all keys
        let keys: Vec<Key> = leaf.entries.iter().map(|(k, _)| k.clone()).collect();
        compressed.prefix = longest_common_prefix(&keys);

        // Store only suffixes
        let prefix_len = compressed.prefix.len();
        for (key, value) in &leaf.entries {
            let suffix = key[prefix_len..].to_vec();
            compressed.entries.push((suffix, *value));
        }

        compressed
    }

    /// Convert to an uncompressed leaf node
    #[must_use]
    pub fn to_leaf_node(&self) -> LeafNode {
        let mut leaf = LeafNode::new(self.max_entries + 1);
        leaf.next_leaf = self.next_leaf;

        for (suffix, value) in &self.entries {
            let mut key = self.prefix.clone();
            key.extend_from_slice(suffix);
            leaf.entries.push((key, *value));
        }

        leaf
    }

    /// Get the full key at the specified index
    #[must_use]
    pub fn get_full_key(&self, idx: usize) -> Option<Vec<u8>> {
        self.entries.get(idx).map(|(suffix, _)| {
            let mut key = self.prefix.clone();
            key.extend_from_slice(suffix);
            key
        })
    }

    /// Check if the node is full
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.entries.len() >= self.max_entries
    }

    /// Insert a key-value pair
    pub fn insert(&mut self, key: Key, value: Value) -> Result<()> {
        if self.is_full() {
            return Err(anyhow!("Compressed leaf node is full"));
        }

        // Compute new common prefix
        let new_prefix = if self.entries.is_empty() {
            key.clone()
        } else {
            let mut keys = vec![self.prefix.clone()];
            keys.push(key.clone());
            longest_common_prefix(&keys)
        };

        // If prefix changes, need to extend all existing suffixes
        if new_prefix.len() < self.prefix.len() {
            self.recompute_suffixes(&new_prefix);
        }

        // Insert the new entry
        let suffix = key[new_prefix.len()..].to_vec();
        let full_key_for_search = key;

        // Find insertion position
        let pos = self
            .entries
            .binary_search_by(|(suf, _)| {
                let mut full = self.prefix.clone();
                full.extend_from_slice(suf);
                full.cmp(&full_key_for_search)
            })
            .unwrap_or_else(|e| e);

        self.entries.insert(pos, (suffix, value));
        self.prefix = new_prefix;

        Ok(())
    }

    /// Recompute all suffixes when the prefix changes
    fn recompute_suffixes(&mut self, new_prefix: &[u8]) {
        let old_prefix_len = self.prefix.len();
        let new_prefix_len = new_prefix.len();

        if new_prefix_len >= old_prefix_len {
            // Prefix got longer - just truncate suffixes
            for (suffix, _) in &mut self.entries {
                *suffix = suffix[(new_prefix_len - old_prefix_len)..].to_vec();
            }
        } else {
            // Prefix got shorter - need to prepend to suffixes
            for (suffix, _) in &mut self.entries {
                let mut new_suffix = self.prefix[new_prefix_len..old_prefix_len].to_vec();
                new_suffix.extend_from_slice(suffix);
                *suffix = new_suffix;
            }
        }
    }

    /// Search for a value by key
    #[must_use]
    pub fn search(&self, key: &Key) -> Option<Value> {
        // Quick check: if key doesn't start with prefix, it's not here
        if !key.starts_with(&self.prefix) {
            return None;
        }

        let suffix_to_find = &key[self.prefix.len()..];

        self.entries
            .binary_search_by(|(suf, _)| suf.as_slice().cmp(suffix_to_find))
            .ok()
            .map(|idx| self.entries[idx].1)
    }

    /// Delete a key from the leaf
    pub fn delete(&mut self, key: &Key) -> bool {
        if !key.starts_with(&self.prefix) {
            return false;
        }

        let suffix_to_find = &key[self.prefix.len()..];

        if let Ok(pos) = self
            .entries
            .binary_search_by(|(suf, _)| suf.as_slice().cmp(suffix_to_find))
        {
            self.entries.remove(pos);

            // Recompute prefix after deletion
            if !self.entries.is_empty() {
                let keys: Vec<Key> = self
                    .entries
                    .iter()
                    .map(|(suf, _)| {
                        let mut k = self.prefix.clone();
                        k.extend_from_slice(suf);
                        k
                    })
                    .collect();
                self.prefix = longest_common_prefix(&keys);
            }

            true
        } else {
            false
        }
    }

    /// Split the compressed leaf node when it becomes full
    pub fn split(&mut self) -> (Key, Self) {
        let mid = self.entries.len() / 2;

        // Create new right leaf with upper half
        let mut right_leaf = Self::new(self.max_entries + 1);
        right_leaf.entries = self.entries.split_off(mid);

        // Recompute prefixes for both nodes
        let left_keys: Vec<Key> = self
            .entries
            .iter()
            .map(|(suf, _)| {
                let mut k = self.prefix.clone();
                k.extend_from_slice(suf);
                k
            })
            .collect();
        self.prefix = longest_common_prefix(&left_keys);

        let right_keys: Vec<Key> = right_leaf
            .entries
            .iter()
            .map(|(suf, _)| {
                let mut k = right_leaf.prefix.clone();
                k.extend_from_slice(suf);
                k
            })
            .collect();
        right_leaf.prefix = longest_common_prefix(&right_keys);

        // Recompute suffixes for both nodes
        let left_prefix_len = self.prefix.len();
        for (suffix, _) in &mut self.entries {
            let full_key = {
                let mut k = left_keys[0][..left_prefix_len].to_vec();
                k.extend_from_slice(suffix);
                k
            };
            *suffix = full_key[left_prefix_len..].to_vec();
        }

        let right_prefix_len = right_leaf.prefix.len();
        for (i, (suffix, _)) in right_leaf.entries.iter_mut().enumerate() {
            let full_key = &right_keys[i];
            *suffix = full_key[right_prefix_len..].to_vec();
        }

        // Key to promote (smallest key in right leaf)
        let promoted_key = right_leaf
            .get_full_key(0)
            .expect("Right leaf should not be empty");

        (promoted_key, right_leaf)
    }

    /// Calculate memory usage
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        let mut size = self.prefix.len();
        for (suffix, _) in &self.entries {
            size += suffix.len() + std::mem::size_of::<Value>();
        }
        size
    }

    /// Calculate compression ratio compared to uncompressed storage
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        if self.entries.is_empty() {
            return 1.0;
        }

        let compressed_size = self.memory_usage();
        let uncompressed_size: usize = self
            .entries
            .iter()
            .map(|(suf, _)| self.prefix.len() + suf.len() + std::mem::size_of::<Value>())
            .sum();

        compressed_size as f64 / uncompressed_size as f64
    }
}

/// Compressed internal node using delta encoding
///
/// Stores the first key fully and subsequent keys as deltas,
/// reducing memory usage for keys with common prefixes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedInternalNode {
    /// First key stored fully
    pub first_key: Option<Key>,
    /// Subsequent keys as deltas
    pub key_deltas: Vec<KeyDelta>,
    /// Child page IDs (always one more than keys)
    pub children: Vec<u64>,
    /// Maximum capacity (order - 1)
    max_keys: usize,
}

impl CompressedInternalNode {
    /// Create a new compressed internal node
    #[must_use]
    pub fn new(order: usize) -> Self {
        Self {
            first_key: None,
            key_deltas: Vec::with_capacity(order),
            children: Vec::with_capacity(order + 1),
            max_keys: order - 1,
        }
    }

    /// Create from an uncompressed internal node
    #[must_use]
    pub fn from_internal_node(internal: &InternalNode) -> Self {
        let mut compressed = Self::new(internal.max_keys + 1);
        compressed.children = internal.children.clone();

        if internal.keys.is_empty() {
            return compressed;
        }

        // Store first key fully
        compressed.first_key = Some(internal.keys[0].clone());

        // Store subsequent keys as deltas
        for i in 1..internal.keys.len() {
            let delta = KeyDelta::from_keys(&internal.keys[i - 1], &internal.keys[i]);
            compressed.key_deltas.push(delta);
        }

        compressed
    }

    /// Convert to an uncompressed internal node
    #[must_use]
    pub fn to_internal_node(&self) -> InternalNode {
        let mut internal = InternalNode::new(self.max_keys + 1);
        internal.children = self.children.clone();

        if let Some(first_key) = &self.first_key {
            internal.keys.push(first_key.clone());

            // Reconstruct keys from deltas
            for delta in &self.key_deltas {
                let prev_key = internal.keys.last().unwrap();
                let key = delta.reconstruct(prev_key);
                internal.keys.push(key);
            }
        }

        internal
    }

    /// Get the full key at the specified index
    #[must_use]
    pub fn get_full_key(&self, idx: usize) -> Option<Vec<u8>> {
        if idx == 0 {
            return self.first_key.clone();
        }

        let mut current = self.first_key.clone()?;

        for i in 0..idx {
            if i >= self.key_deltas.len() {
                return None;
            }
            current = self.key_deltas[i].reconstruct(&current);
        }

        Some(current)
    }

    /// Check if the node is full
    #[must_use]
    pub const fn is_full(&self) -> bool {
        let num_keys = if self.first_key.is_some() {
            1 + self.key_deltas.len()
        } else {
            0
        };
        num_keys >= self.max_keys
    }

    /// Find the index of the child to follow for a given key
    #[must_use]
    pub fn find_child_index(&self, key: &Key) -> usize {
        let Some(first_key) = &self.first_key else {
            return 0;
        };

        if key < first_key {
            return 0;
        }

        let mut current = first_key.clone();
        for (i, delta) in self.key_deltas.iter().enumerate() {
            current = delta.reconstruct(&current);
            if key < &current {
                return i + 1;
            }
        }

        1 + self.key_deltas.len()
    }

    /// Calculate memory usage
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        let mut size = self.first_key.as_ref().map_or(0, std::vec::Vec::len);
        for delta in &self.key_deltas {
            size += std::mem::size_of::<u16>() + delta.suffix.len();
        }
        size += self.children.len() * std::mem::size_of::<u64>();
        size
    }

    /// Calculate compression ratio compared to uncompressed storage
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        if self.first_key.is_none() {
            return 1.0;
        }

        let compressed_size = self.memory_usage();

        // Calculate uncompressed size
        let mut uncompressed_size = self.first_key.as_ref().unwrap().len();
        let mut current = self.first_key.clone().unwrap();
        for delta in &self.key_deltas {
            current = delta.reconstruct(&current);
            uncompressed_size += current.len();
        }
        uncompressed_size += self.children.len() * std::mem::size_of::<u64>();

        compressed_size as f64 / uncompressed_size as f64
    }
}

/// Unified node type for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BTreeNode {
    Internal(InternalNode),
    Leaf(LeafNode),
    CompressedInternal(CompressedInternalNode),
    CompressedLeaf(CompressedLeafNode),
}

impl BTreeNode {
    /// Check if this is an internal node
    #[must_use]
    pub const fn is_internal(&self) -> bool {
        matches!(self, Self::Internal(_) | Self::CompressedInternal(_))
    }

    /// Check if this is a leaf node
    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_) | Self::CompressedLeaf(_))
    }

    /// Check if this is a compressed node
    #[must_use]
    pub const fn is_compressed(&self) -> bool {
        matches!(self, Self::CompressedInternal(_) | Self::CompressedLeaf(_))
    }

    /// Convert to compressed variant if not already compressed
    #[must_use]
    pub fn compress(self) -> Self {
        match self {
            | Self::Internal(internal) => {
                Self::CompressedInternal(CompressedInternalNode::from_internal_node(&internal))
            },
            | Self::Leaf(leaf) => Self::CompressedLeaf(CompressedLeafNode::from_leaf_node(&leaf)),
            | compressed => compressed, // Already compressed
        }
    }

    /// Convert to uncompressed variant if compressed
    #[must_use]
    pub fn decompress(self) -> Self {
        match self {
            | Self::CompressedInternal(compressed) => Self::Internal(compressed.to_internal_node()),
            | Self::CompressedLeaf(compressed) => Self::Leaf(compressed.to_leaf_node()),
            | uncompressed => uncompressed, // Already uncompressed
        }
    }

    /// Calculate memory usage
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        match self {
            | Self::Internal(node) => {
                let keys_size: usize = node.keys.iter().map(std::vec::Vec::len).sum();
                keys_size + node.children.len() * std::mem::size_of::<u64>()
            },
            | Self::Leaf(node) => {
                let entries_size: usize = node
                    .entries
                    .iter()
                    .map(|(k, _)| k.len() + std::mem::size_of::<Value>())
                    .sum();
                entries_size
            },
            | Self::CompressedInternal(node) => node.memory_usage(),
            | Self::CompressedLeaf(node) => node.memory_usage(),
        }
    }

    /// Calculate compression ratio (1.0 means no compression)
    #[must_use]
    pub fn compression_ratio(&self) -> f64 {
        match self {
            | Self::Internal(_) | Self::Leaf(_) => 1.0,
            | Self::CompressedInternal(node) => node.compression_ratio(),
            | Self::CompressedLeaf(node) => node.compression_ratio(),
        }
    }

    /// Get as internal node (panics if not internal)
    ///
    /// # Panics
    /// Panics if the node is not an internal node.
    ///
    /// # Deprecated
    /// Use [`try_as_internal`](Self::try_as_internal) or [`as_internal_checked`](Self::as_internal_checked) instead
    /// for safer error handling.
    #[deprecated(
        since = "0.2.0",
        note = "Use try_as_internal() or as_internal_checked() for proper error handling"
    )]
    #[allow(clippy::expect_used)] // Deprecated method - panic is documented behavior
    #[must_use]
    pub const fn as_internal(&self) -> &InternalNode {
        self.try_as_internal()
            .expect("BTreeNode::as_internal called on a leaf node")
    }

    /// Get as mutable internal node (panics if not internal)
    ///
    /// # Panics
    /// Panics if the node is not an internal node.
    ///
    /// # Deprecated
    /// Use [`try_as_internal_mut`](Self::try_as_internal_mut) or [`as_internal_mut_checked`](Self::as_internal_mut_checked) instead
    /// for safer error handling.
    #[deprecated(
        since = "0.2.0",
        note = "Use try_as_internal_mut() or as_internal_mut_checked() for proper error handling"
    )]
    #[allow(clippy::expect_used)] // Deprecated method - panic is documented behavior
    pub const fn as_internal_mut(&mut self) -> &mut InternalNode {
        self.try_as_internal_mut()
            .expect("BTreeNode::as_internal_mut called on a leaf node")
    }

    /// Get as leaf node (panics if not leaf)
    ///
    /// # Panics
    /// Panics if the node is not a leaf node.
    ///
    /// # Deprecated
    /// Use [`try_as_leaf`](Self::try_as_leaf) or [`as_leaf_checked`](Self::as_leaf_checked) instead
    /// for safer error handling.
    #[deprecated(
        since = "0.2.0",
        note = "Use try_as_leaf() or as_leaf_checked() for proper error handling"
    )]
    #[allow(clippy::expect_used)] // Deprecated method - panic is documented behavior
    #[must_use]
    pub const fn as_leaf(&self) -> &LeafNode {
        self.try_as_leaf()
            .expect("BTreeNode::as_leaf called on an internal node")
    }

    /// Get as mutable leaf node (panics if not leaf)
    ///
    /// # Panics
    /// Panics if the node is not a leaf node.
    ///
    /// # Deprecated
    /// Use [`try_as_leaf_mut`](Self::try_as_leaf_mut) or [`as_leaf_mut_checked`](Self::as_leaf_mut_checked) instead
    /// for safer error handling.
    #[deprecated(
        since = "0.2.0",
        note = "Use try_as_leaf_mut() or as_leaf_mut_checked() for proper error handling"
    )]
    #[allow(clippy::expect_used)] // Deprecated method - panic is documented behavior
    pub const fn as_leaf_mut(&mut self) -> &mut LeafNode {
        self.try_as_leaf_mut()
            .expect("BTreeNode::as_leaf_mut called on an internal node")
    }

    /// Get as internal node with Result-based error handling
    ///
    /// Returns an error if the node is not an internal node.
    /// This is the recommended alternative to `as_internal()`.
    ///
    /// # Errors
    /// Returns an error if this is a leaf node, not an internal node.
    pub fn as_internal_checked(&self) -> Result<&InternalNode> {
        self.try_as_internal()
            .ok_or_else(|| anyhow!("Expected internal node, found leaf node"))
    }

    /// Get as mutable internal node with Result-based error handling
    ///
    /// Returns an error if the node is not an internal node.
    /// This is the recommended alternative to `as_internal_mut()`.
    ///
    /// # Errors
    /// Returns an error if this is a leaf node, not an internal node.
    pub fn as_internal_mut_checked(&mut self) -> Result<&mut InternalNode> {
        self.try_as_internal_mut()
            .ok_or_else(|| anyhow!("Expected internal node, found leaf node"))
    }

    /// Get as leaf node with Result-based error handling
    ///
    /// Returns an error if the node is not a leaf node.
    /// This is the recommended alternative to `as_leaf()`.
    ///
    /// # Errors
    /// Returns an error if this is an internal node, not a leaf node.
    pub fn as_leaf_checked(&self) -> Result<&LeafNode> {
        self.try_as_leaf()
            .ok_or_else(|| anyhow!("Expected leaf node, found internal node"))
    }

    /// Get as mutable leaf node with Result-based error handling
    ///
    /// Returns an error if the node is not a leaf node.
    /// This is the recommended alternative to `as_leaf_mut()`.
    ///
    /// # Errors
    /// Returns an error if this is an internal node, not a leaf node.
    pub fn as_leaf_mut_checked(&mut self) -> Result<&mut LeafNode> {
        self.try_as_leaf_mut()
            .ok_or_else(|| anyhow!("Expected leaf node, found internal node"))
    }

    /// Try to get as internal node (returns None if not internal)
    ///
    /// This is the safe alternative to `as_internal()` that doesn't panic.
    #[must_use]
    pub const fn try_as_internal(&self) -> Option<&InternalNode> {
        match self {
            | Self::Internal(node) => Some(node),
            | Self::Leaf(_) | Self::CompressedInternal(_) | Self::CompressedLeaf(_) => None,
        }
    }

    /// Try to get as mutable internal node (returns None if not internal)
    ///
    /// This is the safe alternative to `as_internal_mut()` that doesn't panic.
    #[must_use]
    pub const fn try_as_internal_mut(&mut self) -> Option<&mut InternalNode> {
        match self {
            | Self::Internal(node) => Some(node),
            | Self::Leaf(_) | Self::CompressedInternal(_) | Self::CompressedLeaf(_) => None,
        }
    }

    /// Try to get as leaf node (returns None if not leaf)
    ///
    /// This is the safe alternative to `as_leaf()` that doesn't panic.
    #[must_use]
    pub const fn try_as_leaf(&self) -> Option<&LeafNode> {
        match self {
            | Self::Leaf(node) => Some(node),
            | Self::Internal(_) | Self::CompressedInternal(_) | Self::CompressedLeaf(_) => None,
        }
    }

    /// Try to get as mutable leaf node (returns None if not leaf)
    ///
    /// This is the safe alternative to `as_leaf_mut()` that doesn't panic.
    #[must_use]
    pub const fn try_as_leaf_mut(&mut self) -> Option<&mut LeafNode> {
        match self {
            | Self::Leaf(node) => Some(node),
            | Self::Internal(_) | Self::CompressedInternal(_) | Self::CompressedLeaf(_) => None,
        }
    }
}

// Tests extracted to tests/btree_node_tests.rs
