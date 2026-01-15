//! B+ Tree Node Implementation
//!
//! Contains internal nodes and leaf nodes for the B+ Tree structure

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

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

/// Unified node type for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BTreeNode {
    Internal(InternalNode),
    Leaf(LeafNode),
}

impl BTreeNode {
    /// Check if this is an internal node
    #[must_use]
    pub const fn is_internal(&self) -> bool {
        matches!(self, Self::Internal(_))
    }

    /// Check if this is a leaf node
    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
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
            | Self::Leaf(_) => None,
        }
    }

    /// Try to get as mutable internal node (returns None if not internal)
    ///
    /// This is the safe alternative to `as_internal_mut()` that doesn't panic.
    #[must_use]
    pub const fn try_as_internal_mut(&mut self) -> Option<&mut InternalNode> {
        match self {
            | Self::Internal(node) => Some(node),
            | Self::Leaf(_) => None,
        }
    }

    /// Try to get as leaf node (returns None if not leaf)
    ///
    /// This is the safe alternative to `as_leaf()` that doesn't panic.
    #[must_use]
    pub const fn try_as_leaf(&self) -> Option<&LeafNode> {
        match self {
            | Self::Leaf(node) => Some(node),
            | Self::Internal(_) => None,
        }
    }

    /// Try to get as mutable leaf node (returns None if not leaf)
    ///
    /// This is the safe alternative to `as_leaf_mut()` that doesn't panic.
    #[must_use]
    pub const fn try_as_leaf_mut(&mut self) -> Option<&mut LeafNode> {
        match self {
            | Self::Leaf(node) => Some(node),
            | Self::Internal(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_node_insert() {
        let mut node = InternalNode::new(5);

        // Internal nodes need at least one child initially
        node.children.push(0);

        node.insert_child(b"key2".to_vec(), 2).unwrap();
        node.insert_child(b"key1".to_vec(), 1).unwrap();
        node.insert_child(b"key3".to_vec(), 3).unwrap();

        assert_eq!(node.keys.len(), 3);
        assert_eq!(node.children.len(), 4);
        assert_eq!(node.keys[0], b"key1");
        assert_eq!(node.keys[1], b"key2");
        assert_eq!(node.keys[2], b"key3");
    }

    #[test]
    fn test_internal_node_find_child() {
        let mut node = InternalNode::new(5);
        node.keys = vec![b"key2".to_vec(), b"key4".to_vec()];
        node.children = vec![1, 2, 3];

        assert_eq!(node.find_child_index(&b"key1".to_vec()), 0);
        assert_eq!(node.find_child_index(&b"key3".to_vec()), 1);
        assert_eq!(node.find_child_index(&b"key5".to_vec()), 2);
    }

    #[test]
    fn test_internal_node_split() {
        let mut node = InternalNode::new(5);
        node.keys = vec![
            b"key1".to_vec(),
            b"key2".to_vec(),
            b"key3".to_vec(),
            b"key4".to_vec(),
        ];
        node.children = vec![1, 2, 3, 4, 5];

        let (promoted_key, right_node) = node.split();

        // With 4 keys, mid = 2, so promoted key is keys[2] = "key3"
        assert_eq!(promoted_key, b"key3");
        assert_eq!(node.keys.len(), 2); // key1, key2
        assert_eq!(right_node.keys.len(), 1); // key4
        assert_eq!(node.children.len(), 3); // 1,2,3
        assert_eq!(right_node.children.len(), 2); // 4,5
    }

    #[test]
    fn test_leaf_node_insert() {
        let mut leaf = LeafNode::new(5);

        leaf.insert(b"key2".to_vec(), 200).unwrap();
        leaf.insert(b"key1".to_vec(), 100).unwrap();
        leaf.insert(b"key3".to_vec(), 300).unwrap();

        assert_eq!(leaf.entries.len(), 3);
        assert_eq!(leaf.entries[0].0, b"key1");
        assert_eq!(leaf.entries[1].0, b"key2");
        assert_eq!(leaf.entries[2].0, b"key3");
    }

    #[test]
    fn test_leaf_node_search() {
        let mut leaf = LeafNode::new(5);
        leaf.insert(b"key1".to_vec(), 100).unwrap();
        leaf.insert(b"key2".to_vec(), 200).unwrap();

        assert_eq!(leaf.search(&b"key1".to_vec()), Some(100));
        assert_eq!(leaf.search(&b"key2".to_vec()), Some(200));
        assert_eq!(leaf.search(&b"key3".to_vec()), None);
    }

    #[test]
    fn test_leaf_node_delete() {
        let mut leaf = LeafNode::new(5);
        leaf.insert(b"key1".to_vec(), 100).unwrap();
        leaf.insert(b"key2".to_vec(), 200).unwrap();

        assert!(leaf.delete(&b"key1".to_vec()));
        assert_eq!(leaf.entries.len(), 1);
        assert!(!leaf.delete(&b"key3".to_vec()));
    }

    #[test]
    fn test_leaf_node_split() {
        let mut leaf = LeafNode::new(5);
        leaf.insert(b"key1".to_vec(), 100).unwrap();
        leaf.insert(b"key2".to_vec(), 200).unwrap();
        leaf.insert(b"key3".to_vec(), 300).unwrap();
        leaf.insert(b"key4".to_vec(), 400).unwrap();

        let (promoted_key, right_leaf) = leaf.split();

        assert_eq!(promoted_key, b"key3");
        assert_eq!(leaf.entries.len(), 2);
        assert_eq!(right_leaf.entries.len(), 2);
        assert_eq!(leaf.entries[0].0, b"key1");
        assert_eq!(right_leaf.entries[0].0, b"key3");
    }

    #[test]
    fn test_leaf_node_duplicate_key() {
        let mut leaf = LeafNode::new(5);
        leaf.insert(b"key1".to_vec(), 100).unwrap();

        let result = leaf.insert(b"key1".to_vec(), 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_as_internal() {
        let internal = BTreeNode::Internal(InternalNode::new(5));
        let leaf = BTreeNode::Leaf(LeafNode::new(5));

        // try_as_internal on internal node should return Some
        assert!(internal.try_as_internal().is_some());

        // try_as_internal on leaf node should return None
        assert!(leaf.try_as_internal().is_none());
    }

    #[test]
    fn test_try_as_internal_mut() {
        let mut internal = BTreeNode::Internal(InternalNode::new(5));
        let mut leaf = BTreeNode::Leaf(LeafNode::new(5));

        // try_as_internal_mut on internal node should return Some
        assert!(internal.try_as_internal_mut().is_some());

        // try_as_internal_mut on leaf node should return None
        assert!(leaf.try_as_internal_mut().is_none());
    }

    #[test]
    fn test_try_as_leaf() {
        let internal = BTreeNode::Internal(InternalNode::new(5));
        let leaf = BTreeNode::Leaf(LeafNode::new(5));

        // try_as_leaf on leaf node should return Some
        assert!(leaf.try_as_leaf().is_some());

        // try_as_leaf on internal node should return None
        assert!(internal.try_as_leaf().is_none());
    }

    #[test]
    fn test_try_as_leaf_mut() {
        let mut internal = BTreeNode::Internal(InternalNode::new(5));
        let mut leaf = BTreeNode::Leaf(LeafNode::new(5));

        // try_as_leaf_mut on leaf node should return Some
        assert!(leaf.try_as_leaf_mut().is_some());

        // try_as_leaf_mut on internal node should return None
        assert!(internal.try_as_leaf_mut().is_none());
    }

    #[test]
    fn test_as_internal_checked() {
        let internal = BTreeNode::Internal(InternalNode::new(5));
        let leaf = BTreeNode::Leaf(LeafNode::new(5));

        // as_internal_checked on internal node should return Ok
        assert!(internal.as_internal_checked().is_ok());

        // as_internal_checked on leaf node should return Err
        let result = leaf.as_internal_checked();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("leaf"));
    }

    #[test]
    fn test_as_internal_mut_checked() {
        let mut internal = BTreeNode::Internal(InternalNode::new(5));
        let mut leaf = BTreeNode::Leaf(LeafNode::new(5));

        // as_internal_mut_checked on internal node should return Ok
        assert!(internal.as_internal_mut_checked().is_ok());

        // as_internal_mut_checked on leaf node should return Err
        let result = leaf.as_internal_mut_checked();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("leaf"));
    }

    #[test]
    fn test_as_leaf_checked() {
        let internal = BTreeNode::Internal(InternalNode::new(5));
        let leaf = BTreeNode::Leaf(LeafNode::new(5));

        // as_leaf_checked on leaf node should return Ok
        assert!(leaf.as_leaf_checked().is_ok());

        // as_leaf_checked on internal node should return Err
        let result = internal.as_leaf_checked();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("internal"));
    }

    #[test]
    fn test_as_leaf_mut_checked() {
        let mut internal = BTreeNode::Internal(InternalNode::new(5));
        let mut leaf = BTreeNode::Leaf(LeafNode::new(5));

        // as_leaf_mut_checked on leaf node should return Ok
        assert!(leaf.as_leaf_mut_checked().is_ok());

        // as_leaf_mut_checked on internal node should return Err
        let result = internal.as_leaf_mut_checked();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("internal"));
    }
}
