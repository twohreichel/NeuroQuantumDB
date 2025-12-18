// Integration tests for NeuroQuantumDB Core
// Tests end-to-end functionality across multiple components

use neuroquantum_core::{
    storage::btree::BTree,
    transaction::{IsolationLevel, TransactionManager},
};
use tempfile::TempDir;

/// Test full CRUD operations with BTree
#[tokio::test]
async fn test_crud_with_btree() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut btree = BTree::new(temp_dir.path()).await?;

    // Insert data
    btree.insert(vec![1, 2, 3], 42).await?;
    btree.insert(vec![4, 5, 6], 100).await?;
    btree.insert(vec![7, 8, 9], 200).await?;

    // Read data
    let value1 = btree.search(&vec![1, 2, 3]).await?;
    assert_eq!(value1, Some(42));

    let value2 = btree.search(&vec![4, 5, 6]).await?;
    assert_eq!(value2, Some(100));

    let value3 = btree.search(&vec![7, 8, 9]).await?;
    assert_eq!(value3, Some(200));

    // Update data using upsert
    btree.upsert(vec![1, 2, 3], 1000).await?;

    // Verify update
    let updated = btree.search(&vec![1, 2, 3]).await?;
    assert_eq!(updated, Some(1000));

    // Delete data
    let deleted = btree.delete(&vec![1, 2, 3]).await?;
    assert!(deleted);

    // Verify deletion
    let not_found = btree.search(&vec![1, 2, 3]).await?;
    assert_eq!(not_found, None);

    Ok(())
}

/// Test BTree with multiple operations
#[tokio::test]
async fn test_btree_multiple_operations() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let mut btree = BTree::new(temp_dir.path()).await?;

    // Insert multiple items
    for i in 0..10 {
        btree.insert(vec![i], i as u64).await?;
    }

    // Read all data
    for i in 0..10 {
        let value = btree.search(&vec![i]).await?;
        assert_eq!(value, Some(i as u64));
    }

    Ok(())
}

/// Test transaction begin functionality
#[tokio::test]
async fn test_transaction_begin() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let tx_manager = TransactionManager::new_async(temp_dir.path()).await?;

    // Start transaction
    let _tx_id = tx_manager
        .begin_transaction(IsolationLevel::ReadCommitted)
        .await?;

    // Transaction should be started successfully
    Ok(())
}

/// Test error handling across components
#[tokio::test]
async fn test_error_handling_integration() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let btree = BTree::new(temp_dir.path()).await?;

    // Test searching for non-existent key
    let result = btree.search(&vec![99, 99, 99]).await?;
    assert_eq!(result, None);

    Ok(())
}
