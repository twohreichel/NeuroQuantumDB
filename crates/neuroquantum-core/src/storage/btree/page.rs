//! Page-level storage for B+ Tree nodes
//!
//! Handles serialization, deserialization, and disk I/O of B+ Tree pages

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, warn};

use super::node::{InternalNode, LeafNode};

/// Page identifier
pub type PageId = u64;

/// Page size in bytes (4KB - standard page size)
pub const PAGE_SIZE: usize = 4096;

/// Magic number to identify valid B+ Tree pages
const MAGIC_NUMBER: u32 = 0x42545245; // "BTRE" in hex

/// Page header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageHeader {
    /// Magic number for validation
    magic: u32,
    /// Page type (0 = free, 1 = internal, 2 = leaf)
    page_type: u8,
    /// Page ID
    page_id: PageId,
    /// Checksum for data integrity
    checksum: u32,
    /// Data length in bytes
    data_len: u32,
}

impl PageHeader {
    fn new(page_id: PageId, page_type: u8, data_len: u32) -> Self {
        Self {
            magic: MAGIC_NUMBER,
            page_type,
            page_id,
            checksum: 0, // Will be calculated before write
            data_len,
        }
    }

    fn is_valid(&self) -> bool {
        self.magic == MAGIC_NUMBER
    }

    fn serialized_size() -> usize {
        // magic(4) + type(1) + id(8) + checksum(4) + len(4) = 21 bytes
        21
    }
}

/// Page serialization/deserialization utility
pub struct PageSerializer;

impl PageSerializer {
    /// Serialize an internal node to bytes
    pub fn serialize_internal(node: &InternalNode) -> Result<Vec<u8>> {
        let data = bincode::serialize(node)?;
        if data.len() > PAGE_SIZE - PageHeader::serialized_size() {
            return Err(anyhow!(
                "Internal node too large: {} bytes (max: {})",
                data.len(),
                PAGE_SIZE - PageHeader::serialized_size()
            ));
        }
        Ok(data)
    }

    /// Deserialize an internal node from bytes
    pub fn deserialize_internal(data: &[u8]) -> Result<InternalNode> {
        Ok(bincode::deserialize(data)?)
    }

    /// Serialize a leaf node to bytes
    pub fn serialize_leaf(node: &LeafNode) -> Result<Vec<u8>> {
        let data = bincode::serialize(node)?;
        if data.len() > PAGE_SIZE - PageHeader::serialized_size() {
            return Err(anyhow!(
                "Leaf node too large: {} bytes (max: {})",
                data.len(),
                PAGE_SIZE - PageHeader::serialized_size()
            ));
        }
        Ok(data)
    }

    /// Deserialize a leaf node from bytes
    pub fn deserialize_leaf(data: &[u8]) -> Result<LeafNode> {
        Ok(bincode::deserialize(data)?)
    }

    /// Calculate checksum for data
    fn calculate_checksum(data: &[u8]) -> u32 {
        // Simple CRC32-like checksum
        let mut checksum = 0u32;
        for &byte in data {
            checksum = checksum.wrapping_add(byte as u32);
            checksum = checksum.wrapping_mul(31);
        }
        checksum
    }

    /// Create a complete page with header and data
    pub fn create_page(page_id: PageId, page_type: u8, data: &[u8]) -> Result<Vec<u8>> {
        let checksum = Self::calculate_checksum(data);
        let header = PageHeader {
            magic: MAGIC_NUMBER,
            page_type,
            page_id,
            checksum,
            data_len: data.len() as u32,
        };

        let mut page = Vec::with_capacity(PAGE_SIZE);

        // Write header
        page.extend_from_slice(&header.magic.to_le_bytes());
        page.push(header.page_type);
        page.extend_from_slice(&header.page_id.to_le_bytes());
        page.extend_from_slice(&header.checksum.to_le_bytes());
        page.extend_from_slice(&header.data_len.to_le_bytes());

        // Write data
        page.extend_from_slice(data);

        // Pad to PAGE_SIZE
        page.resize(PAGE_SIZE, 0);

        Ok(page)
    }

    /// Parse a page and return header and data
    pub fn parse_page(page: &[u8]) -> Result<(PageHeader, Vec<u8>)> {
        if page.len() < PAGE_SIZE {
            return Err(anyhow!("Invalid page size: {}", page.len()));
        }

        // Read header
        let magic = u32::from_le_bytes([page[0], page[1], page[2], page[3]]);
        let page_type = page[4];
        let page_id = u64::from_le_bytes([
            page[5], page[6], page[7], page[8], page[9], page[10], page[11], page[12],
        ]);
        let checksum = u32::from_le_bytes([page[13], page[14], page[15], page[16]]);
        let data_len = u32::from_le_bytes([page[17], page[18], page[19], page[20]]);

        let header = PageHeader {
            magic,
            page_type,
            page_id,
            checksum,
            data_len,
        };

        if !header.is_valid() {
            return Err(anyhow!("Invalid page magic number: {:#x}", magic));
        }

        // Extract data
        let data_start = PageHeader::serialized_size();
        let data_end = data_start + data_len as usize;
        let data = page[data_start..data_end].to_vec();

        // Verify checksum
        let calculated_checksum = Self::calculate_checksum(&data);
        if calculated_checksum != checksum {
            warn!(
                "Checksum mismatch for page {}: expected {:#x}, got {:#x}",
                page_id, checksum, calculated_checksum
            );
        }

        Ok((header, data))
    }
}

/// Page manager for handling disk I/O
pub struct PageManager {
    /// Base directory for page files
    data_dir: PathBuf,
    /// Next available page ID
    next_page_id: AtomicU64,
    /// In-memory page cache
    page_cache: HashMap<PageId, Vec<u8>>,
    /// Cache size limit (number of pages)
    cache_limit: usize,
    /// Dirty pages that need to be flushed
    dirty_pages: HashMap<PageId, Vec<u8>>,
}

impl PageManager {
    /// Create a new page manager
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();

        // Ensure directory exists
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).await?;
        }

        // Load metadata to get next page ID
        let metadata_path = data_dir.join("metadata.json");
        let next_page_id = if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path).await?;
            let metadata: PageManagerMetadata = serde_json::from_str(&content)?;
            metadata.next_page_id
        } else {
            1
        };

        Ok(Self {
            data_dir,
            next_page_id: AtomicU64::new(next_page_id),
            page_cache: HashMap::new(),
            cache_limit: 1000, // Cache up to 1000 pages (~4MB)
            dirty_pages: HashMap::new(),
        })
    }

    /// Allocate a new page ID
    pub async fn allocate_page(&mut self) -> Result<PageId> {
        let page_id = self.next_page_id.fetch_add(1, Ordering::SeqCst);
        self.save_metadata().await?;
        Ok(page_id)
    }

    /// Write an internal node to a page
    pub async fn write_internal_node(
        &mut self,
        page_id: PageId,
        node: &InternalNode,
    ) -> Result<()> {
        debug!("📝 Writing internal node to page {}", page_id);

        let data = PageSerializer::serialize_internal(node)?;
        let page = PageSerializer::create_page(page_id, 1, &data)?;

        // Add to dirty pages
        self.dirty_pages.insert(page_id, page.clone());

        // Update cache
        if self.page_cache.len() < self.cache_limit {
            self.page_cache.insert(page_id, page);
        }

        Ok(())
    }

    /// Write a leaf node to a page
    pub async fn write_leaf_node(&mut self, page_id: PageId, node: &LeafNode) -> Result<()> {
        debug!("📝 Writing leaf node to page {}", page_id);

        let data = PageSerializer::serialize_leaf(node)?;
        let page = PageSerializer::create_page(page_id, 2, &data)?;

        // Add to dirty pages
        self.dirty_pages.insert(page_id, page.clone());

        // Update cache
        if self.page_cache.len() < self.cache_limit {
            self.page_cache.insert(page_id, page);
        }

        Ok(())
    }

    /// Read an internal node from a page
    pub async fn read_internal_node(&self, page_id: PageId) -> Result<InternalNode> {
        debug!("📖 Reading internal node from page {}", page_id);

        let page = self.read_page(page_id).await?;
        let (header, data) = PageSerializer::parse_page(&page)?;

        if header.page_type != 1 {
            return Err(anyhow!(
                "Page {} is not an internal node (type: {})",
                page_id,
                header.page_type
            ));
        }

        PageSerializer::deserialize_internal(&data)
    }

    /// Read a leaf node from a page
    pub async fn read_leaf_node(&self, page_id: PageId) -> Result<LeafNode> {
        debug!("📖 Reading leaf node from page {}", page_id);

        let page = self.read_page(page_id).await?;
        let (header, data) = PageSerializer::parse_page(&page)?;

        if header.page_type != 2 {
            return Err(anyhow!(
                "Page {} is not a leaf node (type: {})",
                page_id,
                header.page_type
            ));
        }

        PageSerializer::deserialize_leaf(&data)
    }

    /// Read a page from disk or cache
    async fn read_page(&self, page_id: PageId) -> Result<Vec<u8>> {
        // Check dirty pages first
        if let Some(page) = self.dirty_pages.get(&page_id) {
            return Ok(page.clone());
        }

        // Check cache
        if let Some(page) = self.page_cache.get(&page_id) {
            return Ok(page.clone());
        }

        // Read from disk
        let page_path = self.page_path(page_id);
        if !page_path.exists() {
            return Err(anyhow!("Page {} not found", page_id));
        }

        let mut file = File::open(&page_path).await?;
        let mut page = vec![0u8; PAGE_SIZE];
        file.read_exact(&mut page).await?;

        Ok(page)
    }

    /// Flush all dirty pages to disk
    pub async fn flush(&mut self) -> Result<()> {
        debug!("💾 Flushing {} dirty pages to disk", self.dirty_pages.len());

        // Collect dirty pages to avoid borrow checker issues
        let dirty_pages: Vec<(PageId, Vec<u8>)> = self.dirty_pages.drain().collect();

        for (page_id, page) in dirty_pages {
            let page_path = self.page_path(page_id);

            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&page_path)
                .await?;

            file.write_all(&page).await?;
            file.sync_all().await?;
        }

        self.save_metadata().await?;

        Ok(())
    }

    /// Get the file path for a page
    fn page_path(&self, page_id: PageId) -> PathBuf {
        self.data_dir.join(format!("page_{:010}.dat", page_id))
    }

    /// Save page manager metadata
    async fn save_metadata(&self) -> Result<()> {
        let metadata = PageManagerMetadata {
            next_page_id: self.next_page_id.load(Ordering::SeqCst),
        };

        let metadata_path = self.data_dir.join("metadata.json");
        let content = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, content).await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PageManagerMetadata {
    next_page_id: PageId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_page_header_serialization() {
        let header = PageHeader::new(42, 1, 256);
        assert!(header.is_valid());
        assert_eq!(header.page_id, 42);
        assert_eq!(header.page_type, 1);
    }

    #[test]
    fn test_page_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let page = PageSerializer::create_page(123, 2, &data).unwrap();

        assert_eq!(page.len(), PAGE_SIZE);

        let (header, parsed_data) = PageSerializer::parse_page(&page).unwrap();
        assert_eq!(header.page_id, 123);
        assert_eq!(header.page_type, 2);
        assert_eq!(parsed_data, data);
    }

    #[test]
    fn test_checksum_validation() {
        let data = vec![1, 2, 3, 4, 5];
        let checksum = PageSerializer::calculate_checksum(&data);
        assert!(checksum > 0);

        // Different data should have different checksum
        let data2 = vec![1, 2, 3, 4, 6];
        let checksum2 = PageSerializer::calculate_checksum(&data2);
        assert_ne!(checksum, checksum2);
    }

    #[tokio::test]
    async fn test_page_manager_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mut pm = PageManager::new(temp_dir.path()).await.unwrap();

        let page_id = pm.allocate_page().await.unwrap();
        assert_eq!(page_id, 1);

        let page_id2 = pm.allocate_page().await.unwrap();
        assert_eq!(page_id2, 2);
    }

    #[tokio::test]
    async fn test_write_and_read_leaf_node() {
        let temp_dir = TempDir::new().unwrap();
        let mut pm = PageManager::new(temp_dir.path()).await.unwrap();

        let page_id = pm.allocate_page().await.unwrap();

        // Create and write a leaf node
        let mut leaf = LeafNode::new(128);
        leaf.insert(b"key1".to_vec(), 100).unwrap();
        leaf.insert(b"key2".to_vec(), 200).unwrap();

        pm.write_leaf_node(page_id, &leaf).await.unwrap();
        pm.flush().await.unwrap();

        // Read it back
        let read_leaf = pm.read_leaf_node(page_id).await.unwrap();
        assert_eq!(read_leaf.entries.len(), 2);
        assert_eq!(read_leaf.entries[0].1, 100);
        assert_eq!(read_leaf.entries[1].1, 200);
    }

    #[tokio::test]
    async fn test_write_and_read_internal_node() {
        let temp_dir = TempDir::new().unwrap();
        let mut pm = PageManager::new(temp_dir.path()).await.unwrap();

        let page_id = pm.allocate_page().await.unwrap();

        // Create and write an internal node
        let mut internal = InternalNode::new(128);
        internal.keys.push(b"key1".to_vec());
        internal.keys.push(b"key2".to_vec());
        internal.children.push(1);
        internal.children.push(2);
        internal.children.push(3);

        pm.write_internal_node(page_id, &internal).await.unwrap();
        pm.flush().await.unwrap();

        // Read it back
        let read_internal = pm.read_internal_node(page_id).await.unwrap();
        assert_eq!(read_internal.keys.len(), 2);
        assert_eq!(read_internal.children.len(), 3);
    }
}
