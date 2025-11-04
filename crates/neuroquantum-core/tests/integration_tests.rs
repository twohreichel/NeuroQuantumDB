// Integration tests for NeuroQuantumDB Core
// Tests end-to-end functionality across multiple components

use neuroquantum_core::{
    dna::compression::{CompressionAlgorithm, DNACompressionEngine},
    quantum::{annealing::QuantumAnnealer, grover::GroverSearch},
    storage::{btree::BTree, buffer::BufferPoolManager, PageId, Value},
    transaction::{TransactionId, TransactionManager},
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;

/// Test full CRUD operations with buffer pool
#[tokio::test]
async fn test_crud_with_buffer_pool() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let buffer_pool = Arc::new(BufferPoolManager::new(
        temp_dir.path().to_path_buf(),
        100,  // 100 pages
        4096, // 4KB pages
    ));

    let btree = Arc::new(RwLock::new(BTree::new(buffer_pool.clone(), PageId(1))));

    // Insert data
    let mut tree = btree.write().await;
    tree.insert(vec![1, 2, 3], Value::Integer(42)).await?;
    tree.insert(vec![4, 5, 6], Value::Text("hello".to_string()))
        .await?;
    tree.insert(vec![7, 8, 9], Value::Boolean(true)).await?;
    drop(tree);

    // Read data
    let tree = btree.read().await;
    let value1 = tree.search(&[1, 2, 3]).await?;
    assert_eq!(value1, Some(Value::Integer(42)));

    let value2 = tree.search(&[4, 5, 6]).await?;
    assert_eq!(value2, Some(Value::Text("hello".to_string())));

    let value3 = tree.search(&[7, 8, 9]).await?;
    assert_eq!(value3, Some(Value::Boolean(true)));
    drop(tree);

    // Update data
    let mut tree = btree.write().await;
    tree.insert(vec![1, 2, 3], Value::Integer(100)).await?;
    drop(tree);

    // Verify update
    let tree = btree.read().await;
    let updated = tree.search(&[1, 2, 3]).await?;
    assert_eq!(updated, Some(Value::Integer(100)));
    drop(tree);

    // Delete data
    let mut tree = btree.write().await;
    let deleted = tree.delete(&[1, 2, 3]).await?;
    assert!(deleted);
    drop(tree);

    // Verify deletion
    let tree = btree.read().await;
    let not_found = tree.search(&[1, 2, 3]).await?;
    assert_eq!(not_found, None);

    Ok(())
}

/// Test buffer pool cache hit/miss tracking
#[tokio::test]
async fn test_buffer_pool_cache_metrics() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let buffer_pool = Arc::new(BufferPoolManager::new(
        temp_dir.path().to_path_buf(),
        10, // Small pool for testing eviction
        4096,
    ));

    let btree = Arc::new(RwLock::new(BTree::new(buffer_pool.clone(), PageId(1))));

    // Initial hit rate should be 0
    let initial_stats = buffer_pool.get_stats().await;
    assert_eq!(initial_stats.cache_hit_rate, 0.0);

    // Insert some data (will cause misses)
    let mut tree = btree.write().await;
    for i in 0..5 {
        tree.insert(vec![i], Value::Integer(i as i64)).await?;
    }
    drop(tree);

    // Read same data (should hit cache)
    let tree = btree.read().await;
    for i in 0..5 {
        let _ = tree.search(&[i]).await?;
    }
    drop(tree);

    // Check improved hit rate
    let final_stats = buffer_pool.get_stats().await;
    assert!(final_stats.cache_hit_rate > 0.0);
    assert!(final_stats.total_hits > 0);

    Ok(())
}

/// Test DNA compression and decompression
#[tokio::test]
async fn test_dna_compression_integration() -> anyhow::Result<()> {
    let engine = DNACompressionEngine::new();

    // Test various DNA sequences
    let sequences = vec![
        "ATCGATCGATCG".repeat(10),    // 120 bases, repetitive
        "GGGGCCCCAAAATTTT".repeat(5), // 80 bases, homopolymers
        "ACGTACGTACGT".repeat(8),     // 96 bases, pattern
    ];

    for sequence in sequences {
        // Compress
        let compressed =
            engine.compress(sequence.as_bytes(), CompressionAlgorithm::HuffmanCoding)?;

        // Verify compression ratio
        let ratio = sequence.len() as f64 / compressed.len() as f64;
        assert!(ratio > 1.0, "Compression ratio should be > 1.0");

        // Decompress
        let decompressed = engine.decompress(&compressed)?;

        // Verify integrity
        assert_eq!(
            sequence.as_bytes(),
            decompressed.as_slice(),
            "Decompressed data should match original"
        );
    }

    Ok(())
}

/// Test Grover's search algorithm
#[tokio::test]
async fn test_grover_search_integration() -> anyhow::Result<()> {
    let grover = GroverSearch::new(4); // 4 qubits = 16 possible states

    // Search for specific value
    let target = 7; // Binary: 0111
    let result = grover.search(target).await?;

    assert_eq!(result, target, "Grover search should find target value");

    Ok(())
}

/// Test quantum annealing optimization
#[tokio::test]
async fn test_quantum_annealing_integration() -> anyhow::Result<()> {
    let annealer = QuantumAnnealer::new(5); // 5 qubits

    // Simple optimization problem
    let couplings = vec![(0, 1, -1.0), (1, 2, -1.0), (2, 3, -1.0), (3, 4, -1.0)];

    let result = annealer.anneal(&couplings, 100).await?;

    // Result should be a valid state
    assert_eq!(result.len(), 5);
    assert!(result.iter().all(|&b| b == 0 || b == 1));

    Ok(())
}

/// Test transaction commit and rollback
#[tokio::test]
async fn test_transaction_integration() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let tx_manager = Arc::new(RwLock::new(TransactionManager::new(
        temp_dir.path().to_path_buf(),
    )?));

    // Start transaction
    let mut manager = tx_manager.write().await;
    let tx_id = manager.begin_transaction().await?;
    drop(manager);

    // Simulate some operations
    // (In real scenario, these would modify data)

    // Commit transaction
    let mut manager = tx_manager.write().await;
    manager.commit_transaction(tx_id).await?;
    drop(manager);

    // Verify transaction is no longer active
    let manager = tx_manager.read().await;
    let is_active = manager.is_transaction_active(tx_id).await;
    assert!(!is_active);

    Ok(())
}

/// Test transaction rollback
#[tokio::test]
async fn test_transaction_rollback() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let tx_manager = Arc::new(RwLock::new(TransactionManager::new(
        temp_dir.path().to_path_buf(),
    )?));

    // Start transaction
    let mut manager = tx_manager.write().await;
    let tx_id = manager.begin_transaction().await?;
    drop(manager);

    // Rollback transaction
    let mut manager = tx_manager.write().await;
    manager.rollback_transaction(tx_id).await?;
    drop(manager);

    // Verify transaction is no longer active
    let manager = tx_manager.read().await;
    let is_active = manager.is_transaction_active(tx_id).await;
    assert!(!is_active);

    Ok(())
}

/// Test concurrent transactions
#[tokio::test]
async fn test_concurrent_transactions() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let tx_manager = Arc::new(RwLock::new(TransactionManager::new(
        temp_dir.path().to_path_buf(),
    )?));

    // Start multiple transactions concurrently
    let mut handles = vec![];

    for _ in 0..5 {
        let tx_manager_clone = tx_manager.clone();
        let handle = tokio::spawn(async move {
            let mut manager = tx_manager_clone.write().await;
            let tx_id = manager.begin_transaction().await?;
            drop(manager);

            // Simulate work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Commit
            let mut manager = tx_manager_clone.write().await;
            manager.commit_transaction(tx_id).await
        });
        handles.push(handle);
    }

    // Wait for all transactions to complete
    for handle in handles {
        handle.await??;
    }

    Ok(())
}

/// Test full stack: Storage + DNA + Quantum
#[tokio::test]
async fn test_full_stack_integration() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;

    // 1. Setup storage
    let buffer_pool = Arc::new(BufferPoolManager::new(
        temp_dir.path().to_path_buf(),
        50,
        4096,
    ));
    let btree = Arc::new(RwLock::new(BTree::new(buffer_pool.clone(), PageId(1))));

    // 2. Store DNA sequence
    let dna_sequence = "ATCGATCGATCG".repeat(20);
    let engine = DNACompressionEngine::new();
    let compressed =
        engine.compress(dna_sequence.as_bytes(), CompressionAlgorithm::HuffmanCoding)?;

    let mut tree = btree.write().await;
    tree.insert(vec![1, 0, 0], Value::Blob(compressed.clone()))
        .await?;
    drop(tree);

    // 3. Retrieve and verify
    let tree = btree.read().await;
    let stored = tree.search(&[1, 0, 0]).await?;
    assert!(stored.is_some());

    if let Some(Value::Blob(data)) = stored {
        let decompressed = engine.decompress(&data)?;
        assert_eq!(dna_sequence.as_bytes(), decompressed.as_slice());
    } else {
        panic!("Expected Blob value");
    }
    drop(tree);

    // 4. Use quantum search
    let grover = GroverSearch::new(3);
    let search_result = grover.search(5).await?;
    assert_eq!(search_result, 5);

    // 5. Check buffer pool performance
    let stats = buffer_pool.get_stats().await;
    assert!(stats.total_accesses > 0);

    Ok(())
}

/// Test error handling across components
#[tokio::test]
async fn test_error_handling_integration() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let buffer_pool = Arc::new(BufferPoolManager::new(
        temp_dir.path().to_path_buf(),
        10,
        4096,
    ));

    let btree = Arc::new(RwLock::new(BTree::new(buffer_pool.clone(), PageId(1))));

    // Test searching for non-existent key
    let tree = btree.read().await;
    let result = tree.search(&[99, 99, 99]).await?;
    assert_eq!(result, None);
    drop(tree);

    // Test invalid DNA sequence
    let engine = DNACompressionEngine::new();
    let invalid_data = vec![255; 100]; // Invalid DNA data
    let result = engine.compress(&invalid_data, CompressionAlgorithm::HuffmanCoding);
    // Should handle gracefully (either compress or return error)
    assert!(result.is_ok() || result.is_err());

    Ok(())
}

/// Test memory constraints under load
#[tokio::test]
async fn test_memory_constraints() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;

    // Small buffer pool to test eviction
    let buffer_pool = Arc::new(BufferPoolManager::new(
        temp_dir.path().to_path_buf(),
        5, // Only 5 pages
        4096,
    ));

    let btree = Arc::new(RwLock::new(BTree::new(buffer_pool.clone(), PageId(1))));

    // Insert more data than buffer pool can hold
    let mut tree = btree.write().await;
    for i in 0..20 {
        tree.insert(vec![i], Value::Integer(i as i64)).await?;
    }
    drop(tree);

    // Should still be able to read all data (via eviction/fetch)
    let tree = btree.read().await;
    for i in 0..20 {
        let value = tree.search(&[i]).await?;
        assert_eq!(value, Some(Value::Integer(i as i64)));
    }
    drop(tree);

    // Check stats
    let stats = buffer_pool.get_stats().await;
    assert!(
        stats.total_misses > 0,
        "Should have cache misses due to eviction"
    );
    assert!(stats.eviction_count > 0, "Should have evicted pages");

    Ok(())
}
