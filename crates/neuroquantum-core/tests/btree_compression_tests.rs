//! B-Tree Compression Tests
//!
//! Tests for prefix compression and front-coded key storage in B-Tree indexes.

use neuroquantum_core::storage::btree::compression::{
    common_prefix, longest_common_prefix, FrontCodedKeys, KeyDelta,
};

#[test]
fn test_common_prefix() {
    let a = b"user_profile_12345";
    let b = b"user_profile_12346";
    let prefix = common_prefix(a, b);
    assert_eq!(prefix, b"user_profile_1234");

    let a = b"completely_different";
    let b = b"nothing_in_common";
    let prefix = common_prefix(a, b);
    assert!(prefix.is_empty());

    let a = b"same";
    let b = b"same";
    let prefix = common_prefix(a, b);
    assert_eq!(prefix, b"same");
}

#[test]
fn test_longest_common_prefix() {
    let keys = vec![
        b"user_profile_12345".to_vec(),
        b"user_profile_12346".to_vec(),
        b"user_profile_12347".to_vec(),
    ];
    let prefix = longest_common_prefix(&keys);
    assert_eq!(prefix, b"user_profile_1234");

    let keys = vec![b"apple".to_vec(), b"banana".to_vec(), b"cherry".to_vec()];
    let prefix = longest_common_prefix(&keys);
    assert!(prefix.is_empty());

    let keys: Vec<Vec<u8>> = vec![];
    let prefix = longest_common_prefix(&keys);
    assert!(prefix.is_empty());

    let keys = vec![b"single".to_vec()];
    let prefix = longest_common_prefix(&keys);
    assert_eq!(prefix, b"single");
}

#[test]
fn test_key_delta() {
    let prev = b"user_profile_12345";
    let curr = b"user_profile_12346";
    let delta = KeyDelta::from_keys(prev, curr);

    assert_eq!(delta.shared_prefix_len, 17);
    assert_eq!(delta.suffix, b"6");

    let reconstructed = delta.reconstruct(prev);
    assert_eq!(reconstructed, curr);
}

#[test]
fn test_key_delta_no_common_prefix() {
    let prev = b"apple";
    let curr = b"banana";
    let delta = KeyDelta::from_keys(prev, curr);

    assert_eq!(delta.shared_prefix_len, 0);
    assert_eq!(delta.suffix, b"banana");

    let reconstructed = delta.reconstruct(prev);
    assert_eq!(reconstructed, curr);
}

#[test]
fn test_key_delta_memory_savings() {
    let prev = b"user_profile_12345";
    let curr = b"user_profile_12346";
    let delta = KeyDelta::from_keys(prev, curr);

    // Full key is 18 bytes, delta is 2 (u16) + 1 (suffix) = 3 bytes
    // Savings = 18 - 3 = 15 bytes
    let savings = delta.memory_savings(curr.len());
    assert!(savings > 0);
    assert_eq!(savings, 15);
}

#[test]
fn test_front_coded_keys() {
    let keys = vec![
        b"user_profile_12345".to_vec(),
        b"user_profile_12346".to_vec(),
        b"user_profile_12347".to_vec(),
        b"user_profile_12348".to_vec(),
    ];

    let front_coded = FrontCodedKeys::from_keys(&keys);

    assert_eq!(front_coded.first_key, b"user_profile_12345");
    assert_eq!(front_coded.subsequent.len(), 3);

    // Reconstruct all keys
    let reconstructed = front_coded.reconstruct_all();
    assert_eq!(reconstructed, keys);

    // Test individual key access
    for (i, expected_key) in keys.iter().enumerate() {
        let key = front_coded.get(i).unwrap();
        assert_eq!(&key, expected_key);
    }
}

#[test]
fn test_front_coded_compression_ratio() {
    let keys = vec![
        b"user_profile_12345".to_vec(),
        b"user_profile_12346".to_vec(),
        b"user_profile_12347".to_vec(),
        b"user_profile_12348".to_vec(),
    ];

    let original_size: usize = keys.iter().map(std::vec::Vec::len).sum();
    let front_coded = FrontCodedKeys::from_keys(&keys);

    // Should achieve significant compression
    let ratio = front_coded.compression_ratio(original_size);
    assert!(ratio < 0.6); // Should compress to less than 60% of original
}

#[test]
fn test_front_coded_memory_usage() {
    let keys = vec![
        b"user_profile_12345".to_vec(),
        b"user_profile_12346".to_vec(),
        b"user_profile_12347".to_vec(),
    ];

    let front_coded = FrontCodedKeys::from_keys(&keys);
    let usage = front_coded.memory_usage();

    // First key: 18 bytes
    // Second key: 1 (shared_len) + 1 (suffix "6") = 2 bytes
    // Third key: 1 (shared_len) + 1 (suffix "7") = 2 bytes
    // Total: 18 + 2 + 2 = 22 bytes (vs 54 bytes uncompressed)
    assert_eq!(usage, 22);
}

#[test]
#[should_panic(expected = "Cannot create FrontCodedKeys from empty key list")]
fn test_front_coded_keys_empty_panic() {
    let keys: Vec<Vec<u8>> = vec![];
    let _ = FrontCodedKeys::from_keys(&keys);
}

#[test]
fn test_front_coded_single_key() {
    let keys = vec![b"single_key".to_vec()];
    let front_coded = FrontCodedKeys::from_keys(&keys);

    assert_eq!(front_coded.first_key, b"single_key");
    assert!(front_coded.subsequent.is_empty());

    let reconstructed = front_coded.reconstruct_all();
    assert_eq!(reconstructed, keys);
}

#[test]
fn test_front_coded_get_out_of_bounds() {
    let keys = vec![b"key1".to_vec(), b"key2".to_vec()];
    let front_coded = FrontCodedKeys::from_keys(&keys);

    assert!(front_coded.get(0).is_some());
    assert!(front_coded.get(1).is_some());
    assert!(front_coded.get(2).is_none());
    assert!(front_coded.get(100).is_none());
}
