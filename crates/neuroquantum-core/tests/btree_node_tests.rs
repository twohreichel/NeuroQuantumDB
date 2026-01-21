//! B+ Tree Node Tests
//!
//! Tests for InternalNode, LeafNode, and CompressedNode types.

use neuroquantum_core::storage::btree::{
    BTreeNode, CompressedInternalNode, CompressedLeafNode, InternalNode, LeafNode,
};

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

// Tests for compressed nodes
#[test]
fn test_compressed_leaf_node_basic() {
    let mut compressed = CompressedLeafNode::new(5);

    // Insert keys with common prefix
    compressed
        .insert(b"user_profile_12345".to_vec(), 100)
        .unwrap();
    compressed
        .insert(b"user_profile_12346".to_vec(), 200)
        .unwrap();
    compressed
        .insert(b"user_profile_12347".to_vec(), 300)
        .unwrap();

    assert_eq!(compressed.entries.len(), 3);
    assert!(!compressed.prefix.is_empty());
    assert_eq!(compressed.prefix, b"user_profile_1234");

    // Test search
    assert_eq!(
        compressed.search(&b"user_profile_12345".to_vec()),
        Some(100)
    );
    assert_eq!(
        compressed.search(&b"user_profile_12346".to_vec()),
        Some(200)
    );
    assert_eq!(
        compressed.search(&b"user_profile_12347".to_vec()),
        Some(300)
    );
    assert_eq!(compressed.search(&b"user_profile_12348".to_vec()), None);
}

#[test]
fn test_compressed_leaf_node_conversion() {
    let mut leaf = LeafNode::new(5);
    leaf.insert(b"user_profile_12345".to_vec(), 100).unwrap();
    leaf.insert(b"user_profile_12346".to_vec(), 200).unwrap();
    leaf.insert(b"user_profile_12347".to_vec(), 300).unwrap();

    // Convert to compressed
    let compressed = CompressedLeafNode::from_leaf_node(&leaf);

    // Verify compression
    assert_eq!(compressed.entries.len(), 3);
    assert!(!compressed.prefix.is_empty());

    // Convert back and verify
    let uncompressed = compressed.to_leaf_node();
    assert_eq!(uncompressed.entries.len(), 3);
    assert_eq!(uncompressed.entries, leaf.entries);
}

#[test]
fn test_compressed_leaf_node_delete() {
    let mut compressed = CompressedLeafNode::new(5);
    compressed
        .insert(b"user_profile_12345".to_vec(), 100)
        .unwrap();
    compressed
        .insert(b"user_profile_12346".to_vec(), 200)
        .unwrap();

    assert!(compressed.delete(&b"user_profile_12345".to_vec()));
    assert_eq!(compressed.entries.len(), 1);
    assert!(!compressed.delete(&b"user_profile_12348".to_vec()));
}

#[test]
fn test_compressed_leaf_node_memory_savings() {
    let mut leaf = LeafNode::new(5);
    leaf.insert(b"user_profile_12345".to_vec(), 100).unwrap();
    leaf.insert(b"user_profile_12346".to_vec(), 200).unwrap();
    leaf.insert(b"user_profile_12347".to_vec(), 300).unwrap();
    leaf.insert(b"user_profile_12348".to_vec(), 400).unwrap();

    let compressed = CompressedLeafNode::from_leaf_node(&leaf);

    // Compressed should use less memory
    let uncompressed_node = BTreeNode::Leaf(leaf);
    let compressed_node = BTreeNode::CompressedLeaf(compressed);

    let uncompressed_size = uncompressed_node.memory_usage();
    let compressed_size = compressed_node.memory_usage();

    assert!(
        compressed_size < uncompressed_size,
        "Compressed size {compressed_size} should be less than uncompressed size {uncompressed_size}"
    );

    // Should achieve significant compression ratio
    let ratio = compressed_node.compression_ratio();
    assert!(ratio < 0.7, "Compression ratio {ratio} should be < 0.7");
}

#[test]
fn test_compressed_internal_node_basic() {
    let mut internal = InternalNode::new(5);
    internal.children.push(0);
    internal.insert_child(b"key1".to_vec(), 1).unwrap();
    internal.insert_child(b"key2".to_vec(), 2).unwrap();
    internal.insert_child(b"key3".to_vec(), 3).unwrap();

    let compressed = CompressedInternalNode::from_internal_node(&internal);

    assert_eq!(compressed.first_key, Some(b"key1".to_vec()));
    assert_eq!(compressed.key_deltas.len(), 2);
    assert_eq!(compressed.children.len(), 4);

    // Verify reconstruction
    let reconstructed = compressed.to_internal_node();
    assert_eq!(reconstructed.keys.len(), 3);
    assert_eq!(reconstructed.keys, internal.keys);
}

#[test]
fn test_compressed_internal_node_find_child() {
    let mut internal = InternalNode::new(5);
    internal.children.push(0);
    internal.insert_child(b"key2".to_vec(), 1).unwrap();
    internal.insert_child(b"key4".to_vec(), 2).unwrap();

    let compressed = CompressedInternalNode::from_internal_node(&internal);

    assert_eq!(compressed.find_child_index(&b"key1".to_vec()), 0);
    assert_eq!(compressed.find_child_index(&b"key3".to_vec()), 1);
    assert_eq!(compressed.find_child_index(&b"key5".to_vec()), 2);
}

#[test]
fn test_compressed_internal_node_memory_savings() {
    let mut internal = InternalNode::new(5);
    internal.children.push(0);
    internal
        .insert_child(b"user_profile_12345".to_vec(), 1)
        .unwrap();
    internal
        .insert_child(b"user_profile_12346".to_vec(), 2)
        .unwrap();
    internal
        .insert_child(b"user_profile_12347".to_vec(), 3)
        .unwrap();

    let compressed = CompressedInternalNode::from_internal_node(&internal);

    let uncompressed_node = BTreeNode::Internal(internal);
    let compressed_node = BTreeNode::CompressedInternal(compressed);

    let uncompressed_size = uncompressed_node.memory_usage();
    let compressed_size = compressed_node.memory_usage();

    assert!(
        compressed_size < uncompressed_size,
        "Compressed size {compressed_size} should be less than uncompressed size {uncompressed_size}"
    );
}

#[test]
fn test_btree_node_compress_decompress() {
    let mut leaf = LeafNode::new(5);
    leaf.insert(b"user_profile_12345".to_vec(), 100).unwrap();
    leaf.insert(b"user_profile_12346".to_vec(), 200).unwrap();

    let node = BTreeNode::Leaf(leaf.clone());

    // Compress
    let compressed = node.compress();
    assert!(compressed.is_compressed());
    assert!(compressed.is_leaf());

    // Decompress
    let decompressed = compressed.decompress();
    assert!(!decompressed.is_compressed());

    // Verify data integrity
    if let BTreeNode::Leaf(decompressed_leaf) = decompressed {
        assert_eq!(decompressed_leaf.entries, leaf.entries);
    } else {
        panic!("Expected leaf node");
    }
}

#[test]
fn test_btree_node_is_compressed() {
    let internal = BTreeNode::Internal(InternalNode::new(5));
    let leaf = BTreeNode::Leaf(LeafNode::new(5));
    let compressed_internal = BTreeNode::CompressedInternal(CompressedInternalNode::new(5));
    let compressed_leaf = BTreeNode::CompressedLeaf(CompressedLeafNode::new(5));

    assert!(!internal.is_compressed());
    assert!(!leaf.is_compressed());
    assert!(compressed_internal.is_compressed());
    assert!(compressed_leaf.is_compressed());

    assert!(internal.is_internal());
    assert!(leaf.is_leaf());
    assert!(compressed_internal.is_internal());
    assert!(compressed_leaf.is_leaf());
}
