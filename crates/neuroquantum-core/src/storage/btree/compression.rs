//! Prefix Compression Utilities for B+ Tree Nodes
//!
//! Implements prefix compression and delta encoding techniques to reduce
//! memory usage in B+ Tree nodes by up to 30-60%.

use serde::{Deserialize, Serialize};

use super::Key;

/// Calculate the common prefix between two byte slices
///
/// # Examples
///
/// ```rust
/// # use neuroquantum_core::storage::btree::compression::common_prefix;
/// let a = b"user_profile_12345";
/// let b = b"user_profile_12346";
/// let prefix = common_prefix(a, b);
/// assert_eq!(prefix, b"user_profile_1234");
/// ```
#[must_use]
pub fn common_prefix(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter()
        .zip(b.iter())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| *x)
        .collect()
}

/// Calculate the longest common prefix among a collection of keys
///
/// # Examples
///
/// ```rust
/// # use neuroquantum_core::storage::btree::compression::longest_common_prefix;
/// let keys = vec![
///     b"user_profile_12345".to_vec(),
///     b"user_profile_12346".to_vec(),
///     b"user_profile_12347".to_vec(),
/// ];
/// let prefix = longest_common_prefix(&keys);
/// assert_eq!(prefix, b"user_profile_1234");
/// ```
#[must_use]
pub fn longest_common_prefix(keys: &[Key]) -> Vec<u8> {
    if keys.is_empty() {
        return Vec::new();
    }

    if keys.len() == 1 {
        return keys[0].clone();
    }

    // Start with the first key as the prefix
    let mut prefix = keys[0].clone();

    // Iterate through remaining keys to find the shortest common prefix
    for key in keys.iter().skip(1) {
        prefix = common_prefix(&prefix, key);
        if prefix.is_empty() {
            break;
        }
    }

    prefix
}

/// Delta encoding for a key relative to a previous key
///
/// Stores only the number of shared bytes and the differing suffix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyDelta {
    /// Number of bytes shared with the previous key
    pub shared_prefix_len: u16,
    /// Differing suffix
    pub suffix: Vec<u8>,
}

impl KeyDelta {
    /// Create a new key delta by comparing two keys
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use neuroquantum_core::storage::btree::compression::KeyDelta;
    /// let prev = b"user_profile_12345";
    /// let curr = b"user_profile_12346";
    /// let delta = KeyDelta::from_keys(prev, curr);
    /// assert_eq!(delta.shared_prefix_len, 17);
    /// assert_eq!(delta.suffix, b"6");
    /// ```
    #[must_use]
    pub fn from_keys(prev_key: &[u8], curr_key: &[u8]) -> Self {
        let prefix = common_prefix(prev_key, curr_key);
        let shared_prefix_len = prefix.len();
        let suffix = curr_key[shared_prefix_len..].to_vec();

        Self {
            shared_prefix_len: shared_prefix_len as u16,
            suffix,
        }
    }

    /// Reconstruct the full key from a previous key and this delta
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use neuroquantum_core::storage::btree::compression::KeyDelta;
    /// let prev = b"user_profile_12345";
    /// let delta = KeyDelta {
    ///     shared_prefix_len: 17,
    ///     suffix: b"6".to_vec(),
    /// };
    /// let reconstructed = delta.reconstruct(prev);
    /// assert_eq!(reconstructed, b"user_profile_12346");
    /// ```
    #[must_use]
    pub fn reconstruct(&self, prev_key: &[u8]) -> Vec<u8> {
        let mut key = prev_key[..self.shared_prefix_len as usize].to_vec();
        key.extend_from_slice(&self.suffix);
        key
    }

    /// Calculate the memory savings compared to storing the full key
    #[must_use]
    pub const fn memory_savings(&self, full_key_len: usize) -> isize {
        let delta_size = std::mem::size_of::<u16>() + self.suffix.len();
        full_key_len as isize - delta_size as isize
    }
}

/// Front coding structure for efficient key storage
///
/// Stores the first key fully and subsequent keys as (`shared_prefix_len`, suffix) pairs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontCodedKeys {
    /// The first key, stored fully
    pub first_key: Vec<u8>,
    /// Subsequent keys as (`shared_prefix_len`, suffix) pairs
    pub subsequent: Vec<(u8, Vec<u8>)>,
}

impl FrontCodedKeys {
    /// Create a new front-coded key structure from a list of keys
    ///
    /// # Panics
    /// Panics if keys is empty
    #[must_use]
    pub fn from_keys(keys: &[Key]) -> Self {
        assert!(
            !keys.is_empty(),
            "Cannot create FrontCodedKeys from empty key list"
        );

        let first_key = keys[0].clone();
        let mut subsequent = Vec::with_capacity(keys.len() - 1);

        let mut prev_key = &keys[0];
        for key in keys.iter().skip(1) {
            let prefix = common_prefix(prev_key, key);
            let shared_len = prefix.len();
            let suffix = key[shared_len..].to_vec();
            subsequent.push((shared_len as u8, suffix));
            prev_key = key;
        }

        Self {
            first_key,
            subsequent,
        }
    }

    /// Reconstruct all keys from the front-coded structure
    #[must_use]
    pub fn reconstruct_all(&self) -> Vec<Key> {
        let mut keys = Vec::with_capacity(1 + self.subsequent.len());
        keys.push(self.first_key.clone());

        let mut prev_key = self.first_key.clone();
        for (shared_len, suffix) in &self.subsequent {
            let mut key = prev_key[..*shared_len as usize].to_vec();
            key.extend_from_slice(suffix);
            prev_key = key.clone();
            keys.push(key);
        }

        keys
    }

    /// Get a specific key by index
    #[must_use]
    pub fn get(&self, index: usize) -> Option<Key> {
        if index == 0 {
            return Some(self.first_key.clone());
        }

        if index > self.subsequent.len() {
            return None;
        }

        // Reconstruct by walking from the first key
        let mut current = self.first_key.clone();
        for (shared_len, suffix) in self.subsequent.iter().take(index) {
            let mut new_key = current[..*shared_len as usize].to_vec();
            new_key.extend_from_slice(suffix);
            current = new_key;
        }

        Some(current)
    }

    /// Calculate total memory usage
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        let mut size = self.first_key.len();
        for (_, suffix) in &self.subsequent {
            size += std::mem::size_of::<u8>() + suffix.len();
        }
        size
    }

    /// Calculate compression ratio compared to storing all keys fully
    #[must_use]
    pub fn compression_ratio(&self, original_size: usize) -> f64 {
        let compressed = self.memory_usage();
        compressed as f64 / original_size as f64
    }
}
