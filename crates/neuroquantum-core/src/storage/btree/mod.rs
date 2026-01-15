//! B+ Tree Index Implementation
//!
//! High-performance persistent B+ Tree index structure with:
//! - Efficient range scans and point lookups
//! - Disk-backed persistence with page-level storage
//! - ACID-compliant operations
//! - Concurrent access support

use anyhow::{anyhow, Result};
use std::fmt;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

pub mod node;
pub mod page;

#[cfg(test)]
mod tests;

pub use node::{BTreeNode, InternalNode, LeafNode};
pub use page::{PageId, PageManager, PageSerializer};

/// Default B+ Tree order (max children per internal node)
pub const DEFAULT_ORDER: usize = 128;

/// Type alias for keys in the B+ Tree
pub type Key = Vec<u8>;

/// Type alias for values (row IDs in the database)
pub type Value = u64;

/// Type alias for async insert result with optional split information
type InsertFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<(Key, PageId)>>> + Send + 'a>>;

/// Type alias for async upsert result with optional split information and new key flag
type UpsertFuture<'a> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<(Option<(Key, PageId)>, bool)>> + Send + 'a>,
>;

/// Type alias for async range scan result
type RangeScanFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<(Key, Value)>>> + Send + 'a>>;

/// Configuration for B+ Tree
#[derive(Debug, Clone)]
pub struct BTreeConfig {
    /// Maximum number of children per internal node
    pub order: usize,
    /// Path to persist the B+ Tree data
    pub data_path: std::path::PathBuf,
    /// Enable write-ahead logging
    pub enable_wal: bool,
}

impl Default for BTreeConfig {
    fn default() -> Self {
        Self {
            order: DEFAULT_ORDER,
            data_path: std::path::PathBuf::from("./btree_data"),
            enable_wal: true,
        }
    }
}

/// B+ Tree Index
///
/// A persistent B+ Tree implementation optimized for database indexing.
/// All leaf nodes are linked for efficient range scans.
pub struct BTree {
    /// Root node page ID
    root_page_id: Option<PageId>,
    /// Tree configuration
    config: BTreeConfig,
    /// Page manager for disk I/O
    page_manager: PageManager,
    /// Total number of keys in the tree
    num_keys: usize,
    /// Tree height
    height: usize,
}

impl BTree {
    /// Create a new B+ Tree with default configuration
    pub async fn new<P: AsRef<Path>>(data_path: P) -> Result<Self> {
        let config = BTreeConfig {
            data_path: data_path.as_ref().to_path_buf(),
            ..Default::default()
        };
        Self::with_config(config).await
    }

    /// Create a new B+ Tree with custom configuration
    pub async fn with_config(config: BTreeConfig) -> Result<Self> {
        info!("🌳 Initializing B+ Tree at: {}", config.data_path.display());

        // Create data directory if it doesn't exist
        if !config.data_path.exists() {
            fs::create_dir_all(&config.data_path).await?;
        }

        // Initialize page manager
        let page_manager = PageManager::new(&config.data_path).await?;

        Ok(Self {
            root_page_id: None,
            config,
            page_manager,
            num_keys: 0,
            height: 0,
        })
    }

    /// Insert a key-value pair into the B+ Tree
    ///
    /// # Arguments
    /// * `key` - The key to insert
    /// * `value` - The value (row ID) to associate with the key
    ///
    /// # Returns
    /// * `Ok(())` if insertion was successful
    /// * `Err` if the key already exists or an I/O error occurred
    pub async fn insert(&mut self, key: Key, value: Value) -> Result<()> {
        debug!("🌳 Inserting key (len={}) with value={}", key.len(), value);

        // If tree is empty, create root leaf node
        if self.root_page_id.is_none() {
            let mut root_leaf = LeafNode::new(self.config.order);
            root_leaf.insert(key, value)?;

            let root_page_id = self.page_manager.allocate_page().await?;
            self.page_manager
                .write_leaf_node(root_page_id, &root_leaf)
                .await?;

            self.root_page_id = Some(root_page_id);
            self.num_keys = 1;
            self.height = 1;

            return Ok(());
        }

        // Insert into existing tree
        let root_page_id = self
            .root_page_id
            .ok_or_else(|| anyhow!("Tree has entries but no root page (invariant violation)"))?;
        let split_result = self.insert_recursive(root_page_id, key, value).await?;

        // Handle root split
        if let Some((split_key, new_page_id)) = split_result {
            let mut new_root = InternalNode::new(self.config.order);
            new_root.keys.push(split_key);
            new_root.children.push(root_page_id);
            new_root.children.push(new_page_id);

            let new_root_page_id = self.page_manager.allocate_page().await?;
            self.page_manager
                .write_internal_node(new_root_page_id, &new_root)
                .await?;

            self.root_page_id = Some(new_root_page_id);
            self.height += 1;
        }

        self.num_keys += 1;
        Ok(())
    }

    /// Insert or update a key-value pair (upsert operation)
    ///
    /// # Arguments
    /// * `key` - The key to insert or update
    /// * `value` - The value (row ID) to associate with the key
    ///
    /// # Returns
    /// * `Ok(())` if upsert was successful
    /// * `Err` if an I/O error occurred
    pub async fn upsert(&mut self, key: Key, value: Value) -> Result<()> {
        debug!("🌳 Upserting key (len={}) with value={}", key.len(), value);

        // If tree is empty, create root leaf node
        if self.root_page_id.is_none() {
            let mut root_leaf = LeafNode::new(self.config.order);
            root_leaf.upsert(key, value)?;

            let root_page_id = self.page_manager.allocate_page().await?;
            self.page_manager
                .write_leaf_node(root_page_id, &root_leaf)
                .await?;

            self.root_page_id = Some(root_page_id);
            self.num_keys = 1;
            self.height = 1;

            return Ok(());
        }

        // Upsert into existing tree
        let root_page_id = self
            .root_page_id
            .ok_or_else(|| anyhow!("Tree has entries but no root page (invariant violation)"))?;
        let (split_result, is_new_key) = self.upsert_recursive(root_page_id, key, value).await?;

        // Handle root split
        if let Some((split_key, new_page_id)) = split_result {
            let mut new_root = InternalNode::new(self.config.order);
            new_root.keys.push(split_key);
            new_root.children.push(root_page_id);
            new_root.children.push(new_page_id);

            let new_root_page_id = self.page_manager.allocate_page().await?;
            self.page_manager
                .write_internal_node(new_root_page_id, &new_root)
                .await?;

            self.root_page_id = Some(new_root_page_id);
            self.height += 1;
        }

        // Only increment key count if it was a new key
        if is_new_key {
            self.num_keys += 1;
        }

        Ok(())
    }

    /// Search for a value by key
    ///
    /// # Arguments
    /// * `key` - The key to search for
    ///
    /// # Returns
    /// * `Some(value)` if the key exists
    /// * `None` if the key doesn't exist
    pub async fn search(&self, key: &Key) -> Result<Option<Value>> {
        if self.root_page_id.is_none() {
            return Ok(None);
        }

        let root_page_id = self
            .root_page_id
            .ok_or_else(|| anyhow!("Tree has entries but no root page (invariant violation)"))?;
        self.search_recursive(root_page_id, key).await
    }

    /// Perform a range scan
    ///
    /// # Arguments
    /// * `start_key` - Start of the range (inclusive)
    /// * `end_key` - End of the range (inclusive)
    ///
    /// # Returns
    /// * Vector of (key, value) pairs in the range
    pub async fn range_scan(&self, start_key: &Key, end_key: &Key) -> Result<Vec<(Key, Value)>> {
        if self.root_page_id.is_none() {
            return Ok(Vec::new());
        }

        let root_page_id = self
            .root_page_id
            .ok_or_else(|| anyhow!("Tree has entries but no root page (invariant violation)"))?;
        self.range_scan_recursive(root_page_id, start_key, end_key)
            .await
    }

    /// Delete a key from the tree
    ///
    /// # Arguments
    /// * `key` - The key to delete
    ///
    /// # Returns
    /// * `Ok(true)` if the key was deleted
    /// * `Ok(false)` if the key didn't exist
    pub async fn delete(&mut self, key: &Key) -> Result<bool> {
        if self.root_page_id.is_none() {
            return Ok(false);
        }

        let root_page_id = self
            .root_page_id
            .ok_or_else(|| anyhow!("Tree has entries but no root page (invariant violation)"))?;
        let deleted = self.delete_recursive(root_page_id, key).await?;

        if deleted {
            self.num_keys -= 1;
        }

        Ok(deleted)
    }

    /// Get the number of keys in the tree
    pub const fn len(&self) -> usize {
        self.num_keys
    }

    /// Check if the tree is empty
    pub const fn is_empty(&self) -> bool {
        self.num_keys == 0
    }

    /// Get the height of the tree
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Flush all pending changes to disk
    pub async fn flush(&mut self) -> Result<()> {
        self.page_manager.flush().await
    }

    // === PRIVATE RECURSIVE OPERATIONS ===

    /// Recursive insert helper
    fn insert_recursive(&mut self, page_id: PageId, key: Key, value: Value) -> InsertFuture<'_> {
        Box::pin(async move {
            // Try to read as internal node first
            if let Ok(mut internal_node) = self.page_manager.read_internal_node(page_id).await {
                // Find child to insert into
                let child_index = internal_node.find_child_index(&key);
                let child_page_id = internal_node.children[child_index];

                // Recursively insert into child
                let split_result = self.insert_recursive(child_page_id, key, value).await?;

                // Handle child split
                if let Some((split_key, new_child_page_id)) = split_result {
                    internal_node.insert_child(split_key, new_child_page_id)?;

                    // Check if internal node needs to split
                    if internal_node.is_full() {
                        let (split_key, new_node) = internal_node.split();
                        let new_page_id = self.page_manager.allocate_page().await?;

                        self.page_manager
                            .write_internal_node(page_id, &internal_node)
                            .await?;
                        self.page_manager
                            .write_internal_node(new_page_id, &new_node)
                            .await?;

                        return Ok(Some((split_key, new_page_id)));
                    }
                    self.page_manager
                        .write_internal_node(page_id, &internal_node)
                        .await?;
                }

                Ok(None)
            } else {
                // Must be a leaf node
                let mut leaf_node = self.page_manager.read_leaf_node(page_id).await?;

                // Insert into leaf
                leaf_node.insert(key, value)?;

                // Check if leaf needs to split
                if leaf_node.is_full() {
                    let (split_key, mut new_leaf) = leaf_node.split();
                    let new_page_id = self.page_manager.allocate_page().await?;

                    // Update sibling pointers
                    new_leaf.next_leaf = leaf_node.next_leaf;
                    leaf_node.next_leaf = Some(new_page_id);

                    self.page_manager
                        .write_leaf_node(page_id, &leaf_node)
                        .await?;
                    self.page_manager
                        .write_leaf_node(new_page_id, &new_leaf)
                        .await?;

                    Ok(Some((split_key, new_page_id)))
                } else {
                    self.page_manager
                        .write_leaf_node(page_id, &leaf_node)
                        .await?;
                    Ok(None)
                }
            }
        })
    }

    /// Recursive upsert helper
    /// Returns: (optional split info, whether key was new)
    fn upsert_recursive(&mut self, page_id: PageId, key: Key, value: Value) -> UpsertFuture<'_> {
        Box::pin(async move {
            // Try to read as internal node first
            if let Ok(mut internal_node) = self.page_manager.read_internal_node(page_id).await {
                // Find child to upsert into
                let child_index = internal_node.find_child_index(&key);
                let child_page_id = internal_node.children[child_index];

                // Recursively upsert into child
                let (split_result, is_new_key) =
                    self.upsert_recursive(child_page_id, key, value).await?;

                // Handle child split
                if let Some((split_key, new_child_page_id)) = split_result {
                    internal_node.insert_child(split_key, new_child_page_id)?;

                    // Check if internal node needs to split
                    if internal_node.is_full() {
                        let (split_key, new_node) = internal_node.split();
                        let new_page_id = self.page_manager.allocate_page().await?;

                        self.page_manager
                            .write_internal_node(page_id, &internal_node)
                            .await?;
                        self.page_manager
                            .write_internal_node(new_page_id, &new_node)
                            .await?;

                        return Ok((Some((split_key, new_page_id)), is_new_key));
                    }
                    self.page_manager
                        .write_internal_node(page_id, &internal_node)
                        .await?;
                }

                Ok((None, is_new_key))
            } else {
                // Must be a leaf node
                let mut leaf_node = self.page_manager.read_leaf_node(page_id).await?;

                // Upsert into leaf
                let is_new_key = leaf_node.upsert(key, value)?;

                // Check if leaf needs to split
                if leaf_node.is_full() {
                    let (split_key, mut new_leaf) = leaf_node.split();
                    let new_page_id = self.page_manager.allocate_page().await?;

                    // Update sibling pointers
                    new_leaf.next_leaf = leaf_node.next_leaf;
                    leaf_node.next_leaf = Some(new_page_id);

                    self.page_manager
                        .write_leaf_node(page_id, &leaf_node)
                        .await?;
                    self.page_manager
                        .write_leaf_node(new_page_id, &new_leaf)
                        .await?;

                    Ok((Some((split_key, new_page_id)), is_new_key))
                } else {
                    self.page_manager
                        .write_leaf_node(page_id, &leaf_node)
                        .await?;
                    Ok((None, is_new_key))
                }
            }
        })
    }

    /// Recursive search helper
    fn search_recursive<'a>(
        &'a self,
        page_id: PageId,
        key: &'a Key,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Value>>> + Send + 'a>>
    {
        Box::pin(async move {
            // Try to read as internal node first
            if let Ok(internal_node) = self.page_manager.read_internal_node(page_id).await {
                let child_index = internal_node.find_child_index(key);
                let child_page_id = internal_node.children[child_index];
                return self.search_recursive(child_page_id, key).await;
            }

            // Must be a leaf node
            let leaf_node = self.page_manager.read_leaf_node(page_id).await?;
            Ok(leaf_node.search(key))
        })
    }

    /// Recursive range scan helper
    fn range_scan_recursive<'a>(
        &'a self,
        page_id: PageId,
        start_key: &'a Key,
        end_key: &'a Key,
    ) -> RangeScanFuture<'a> {
        Box::pin(async move {
            // Try to read as internal node first
            if let Ok(internal_node) = self.page_manager.read_internal_node(page_id).await {
                let child_index = internal_node.find_child_index(start_key);
                let child_page_id = internal_node.children[child_index];
                return self
                    .range_scan_recursive(child_page_id, start_key, end_key)
                    .await;
            }

            // Must be a leaf node - collect results from this leaf and siblings
            let mut results = Vec::new();
            let mut current_page_id = Some(page_id);

            while let Some(page_id) = current_page_id {
                let leaf_node = self.page_manager.read_leaf_node(page_id).await?;

                for (key, value) in &leaf_node.entries {
                    if key >= start_key && key <= end_key {
                        results.push((key.clone(), *value));
                    } else if key > end_key {
                        return Ok(results);
                    }
                }

                current_page_id = leaf_node.next_leaf;
            }

            Ok(results)
        })
    }

    /// Recursive delete helper
    fn delete_recursive<'a>(
        &'a mut self,
        page_id: PageId,
        key: &'a Key,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send + 'a>> {
        Box::pin(async move {
            // Try to read as internal node first
            if let Ok(internal_node) = self.page_manager.read_internal_node(page_id).await {
                let child_index = internal_node.find_child_index(key);
                let child_page_id = internal_node.children[child_index];
                return self.delete_recursive(child_page_id, key).await;
            }

            // Must be a leaf node
            let mut leaf_node = self.page_manager.read_leaf_node(page_id).await?;
            let deleted = leaf_node.delete(key);

            if deleted {
                self.page_manager
                    .write_leaf_node(page_id, &leaf_node)
                    .await?;
            }

            Ok(deleted)
        })
    }
}

impl fmt::Debug for BTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BTree")
            .field("root_page_id", &self.root_page_id)
            .field("num_keys", &self.num_keys)
            .field("height", &self.height)
            .field("order", &self.config.order)
            .finish()
    }
}
