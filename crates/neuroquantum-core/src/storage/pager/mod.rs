//! Page Storage Manager for NeuroQuantumDB
//!
//! Provides low-level disk I/O management with:
//! - 4KB page-based storage
//! - Free page tracking
//! - Page allocation/deallocation
//! - Checksum validation
//! - Async file operations

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub mod free_list;
pub mod io;
pub mod page;

pub use free_list::FreeList;
pub use io::PageIO;
pub use page::{Page, PageHeader, PageId, PageType, PAGE_SIZE};

/// Configuration for the page storage manager
#[derive(Debug, Clone)]
pub struct PagerConfig {
    /// Maximum file size in bytes (default: 10GB)
    pub max_file_size: u64,
    /// Enable page checksums for data integrity
    pub enable_checksums: bool,
    /// Sync mode: fsync after each write
    pub sync_mode: SyncMode,
    /// Enable direct I/O (bypass OS cache)
    pub direct_io: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// No sync (fastest, least safe)
    None,
    /// Sync on transaction commit
    Commit,
    /// Sync after every write (slowest, safest)
    Always,
}

impl Default for PagerConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024 * 1024, // 10GB
            enable_checksums: true,
            sync_mode: SyncMode::Commit,
            direct_io: false,
        }
    }
}

/// Page Storage Manager
///
/// Manages disk-based page storage with allocation, deallocation,
/// and efficient I/O operations.
pub struct PageStorageManager {
    /// Path to the database file
    file_path: PathBuf,
    /// Configuration
    config: PagerConfig,
    /// Page I/O handler
    io: Arc<RwLock<PageIO>>,
    /// Free page list
    free_list: Arc<RwLock<FreeList>>,
    /// Total number of pages
    total_pages: Arc<RwLock<u64>>,
    /// Page cache (simple LRU)
    page_cache: Arc<RwLock<lru::LruCache<PageId, Page>>>,
}

impl PageStorageManager {
    /// Create a new page storage manager
    pub async fn new<P: AsRef<Path>>(file_path: P, config: PagerConfig) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        info!(
            "📄 Initializing PageStorageManager at: {}",
            file_path.display()
        );

        // Create parent directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create storage directory")?;
        }

        // Open or create file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file_path)
            .await
            .context("Failed to open database file")?;

        let io = Arc::new(RwLock::new(PageIO::new(file, config.clone())));

        // Load or initialize free list
        let (free_list, total_pages) = Self::load_metadata(&io).await?;

        info!(
            "📊 Loaded {} total pages, {} free pages",
            total_pages,
            free_list.free_count()
        );

        let manager = Self {
            file_path,
            config: config.clone(),
            io,
            free_list: Arc::new(RwLock::new(free_list)),
            total_pages: Arc::new(RwLock::new(total_pages)),
            page_cache: Arc::new(RwLock::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(1000).unwrap(),
            ))),
        };

        // Initialize page 0 with free list if it's a new database
        if total_pages == 1 {
            let free_list = manager.free_list.read().await;
            manager.persist_free_list(&free_list).await?;
        }

        Ok(manager)
    }

    /// Load metadata from disk or initialize new
    async fn load_metadata(io: &Arc<RwLock<PageIO>>) -> Result<(FreeList, u64)> {
        let io = io.read().await;
        let file_size = io.file_size().await?;

        if file_size == 0 {
            // New database file - reserve page 0 for metadata
            debug!("📝 Initializing new database file");
            return Ok((FreeList::new(), 1)); // Start with 1 page (page 0 reserved)
        }

        let total_pages = file_size / PAGE_SIZE as u64;

        // Try to read free list from first page
        match io.read_page(PageId(0)).await {
            Ok(page) if page.header().page_type == PageType::FreePage => {
                let free_list = FreeList::deserialize(page.data())?;
                debug!("📋 Loaded free list: {} free pages", free_list.free_count());
                Ok((free_list, total_pages))
            }
            _ => {
                // Create new free list
                warn!("⚠️ Free list not found, rebuilding...");
                Ok((FreeList::new(), total_pages.max(1)))
            }
        }
    }

    /// Allocate a new page
    pub async fn allocate_page(&self, page_type: PageType) -> Result<PageId> {
        let mut free_list = self.free_list.write().await;

        // Try to reuse a free page
        if let Some(page_id) = free_list.pop_free_page() {
            debug!("♻️ Reusing free page: {:?}", page_id);

            // Initialize the page
            let page = Page::new(page_id, page_type);
            self.write_page(&page).await?;

            return Ok(page_id);
        }

        // Allocate a new page at the end of the file
        let mut total_pages = self.total_pages.write().await;
        let page_id = PageId(*total_pages);
        *total_pages += 1;

        debug!(
            "📄 Allocating new page: {:?} (type: {:?})",
            page_id, page_type
        );

        // Check file size limit
        let new_size = *total_pages * PAGE_SIZE as u64;
        if new_size > self.config.max_file_size {
            return Err(anyhow!(
                "Database file size limit exceeded: {} bytes",
                self.config.max_file_size
            ));
        }

        // Initialize the page
        let page = Page::new(page_id, page_type);
        self.write_page(&page).await?;

        Ok(page_id)
    }

    /// Deallocate a page (add to free list)
    pub async fn deallocate_page(&self, page_id: PageId) -> Result<()> {
        debug!("🗑️ Deallocating page: {:?}", page_id);

        let mut free_list = self.free_list.write().await;
        free_list.add_free_page(page_id);

        // Remove from cache
        let mut cache = self.page_cache.write().await;
        cache.pop(&page_id);

        // Persist free list
        self.persist_free_list(&free_list).await?;

        Ok(())
    }

    /// Read a page from disk (with caching)
    pub async fn read_page(&self, page_id: PageId) -> Result<Page> {
        // Check cache first
        {
            let mut cache = self.page_cache.write().await;
            if let Some(page) = cache.get(&page_id) {
                debug!("💾 Cache hit for page: {:?}", page_id);
                return Ok(page.clone());
            }
        }

        // Read from disk
        debug!("📖 Reading page from disk: {:?}", page_id);
        let io = self.io.read().await;
        let page = io.read_page(page_id).await?;

        // Validate checksum if enabled
        if self.config.enable_checksums && !page.verify_checksum() {
            return Err(anyhow!("Checksum validation failed for page {:?}", page_id));
        }

        // Add to cache
        {
            let mut cache = self.page_cache.write().await;
            cache.put(page_id, page.clone());
        }

        Ok(page)
    }

    /// Write a page to disk
    pub async fn write_page(&self, page: &Page) -> Result<()> {
        debug!("💾 Writing page: {:?}", page.id());

        // Update checksum if enabled
        let mut page = page.clone();
        if self.config.enable_checksums {
            page.update_checksum();
        }

        // Write to disk
        let io = self.io.write().await;
        io.write_page(&page).await?;

        // Sync if configured
        if self.config.sync_mode == SyncMode::Always {
            io.sync().await?;
        }

        // Update cache
        {
            let mut cache = self.page_cache.write().await;
            cache.put(page.id(), page);
        }

        Ok(())
    }

    /// Sync all pending writes to disk
    pub async fn sync(&self) -> Result<()> {
        debug!("🔄 Syncing all writes to disk");
        let io = self.io.read().await;
        io.sync().await
    }

    /// Get total number of pages
    pub async fn total_pages(&self) -> u64 {
        *self.total_pages.read().await
    }

    /// Get number of free pages
    pub async fn free_pages(&self) -> usize {
        self.free_list.read().await.free_count()
    }

    /// Persist the free list to page 0
    async fn persist_free_list(&self, free_list: &FreeList) -> Result<()> {
        let data = free_list.serialize()?;
        let mut page = Page::new(PageId(0), PageType::FreePage);
        page.write_data(0, &data)?;

        // Update checksum if enabled
        if self.config.enable_checksums {
            page.update_checksum();
        }

        let io = self.io.write().await;
        io.write_page(&page).await?;

        Ok(())
    }

    /// Flush cache and sync to disk
    pub async fn flush(&self) -> Result<()> {
        info!("💾 Flushing page cache and syncing to disk");

        // Persist free list
        let free_list = self.free_list.read().await;
        self.persist_free_list(&free_list).await?;

        // Sync all writes
        self.sync().await?;

        Ok(())
    }

    /// Get storage statistics
    pub async fn stats(&self) -> StorageStats {
        let total = *self.total_pages.read().await;
        let free = self.free_list.read().await.free_count() as u64;
        let cache = self.page_cache.read().await.len() as u64;

        StorageStats {
            total_pages: total,
            free_pages: free,
            used_pages: total.saturating_sub(free),
            cached_pages: cache,
            file_size_bytes: total * PAGE_SIZE as u64,
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_pages: u64,
    pub free_pages: u64,
    pub used_pages: u64,
    pub cached_pages: u64,
    pub file_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_page_storage_manager() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();

        // Should have 1 page (page 0 for metadata)
        assert_eq!(manager.total_pages().await, 1);
    }

    #[tokio::test]
    async fn test_allocate_and_read_page() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();

        // Allocate a page (should be page 1, as page 0 is reserved for metadata)
        let page_id = manager.allocate_page(PageType::Data).await.unwrap();
        assert_eq!(page_id, PageId(1));

        // Read it back
        let page = manager.read_page(page_id).await.unwrap();
        assert_eq!(page.id(), page_id);
        assert_eq!(page.header().page_type, PageType::Data);
    }

    #[tokio::test]
    async fn test_write_and_read_page_data() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();

        // Allocate and write data
        let page_id = manager.allocate_page(PageType::Data).await.unwrap();
        let mut page = manager.read_page(page_id).await.unwrap();

        let test_data = b"Hello, NeuroQuantumDB!";
        page.write_data(0, test_data).unwrap();

        manager.write_page(&page).await.unwrap();

        // Read back
        let page = manager.read_page(page_id).await.unwrap();
        let read_data = &page.data()[..test_data.len()];
        assert_eq!(read_data, test_data);
    }

    #[tokio::test]
    async fn test_allocate_multiple_pages() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();

        // Allocate 100 pages (starting from page 1, as page 0 is metadata)
        for i in 0..100 {
            let page_id = manager.allocate_page(PageType::Data).await.unwrap();
            assert_eq!(page_id, PageId(i + 1));
        }

        // Total pages = 1 (metadata) + 100 (data)
        assert_eq!(manager.total_pages().await, 101);
    }

    #[tokio::test]
    async fn test_deallocate_and_reuse_page() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();

        // Allocate 3 pages
        let page1 = manager.allocate_page(PageType::Data).await.unwrap();
        let page2 = manager.allocate_page(PageType::Data).await.unwrap();
        let page3 = manager.allocate_page(PageType::Data).await.unwrap();

        // Deallocate middle page
        manager.deallocate_page(page2).await.unwrap();
        assert_eq!(manager.free_pages().await, 1);

        // Allocate new page - should reuse page2
        let page4 = manager.allocate_page(PageType::Data).await.unwrap();
        assert_eq!(page4, page2);
        assert_eq!(manager.free_pages().await, 0);
    }

    #[tokio::test]
    async fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let mut page_ids = Vec::new();

        // Create and populate database
        {
            let manager = PageStorageManager::new(&db_path, PagerConfig::default())
                .await
                .unwrap();

            for i in 0..10 {
                let page_id = manager.allocate_page(PageType::Data).await.unwrap();
                page_ids.push(page_id);
                let mut page = manager.read_page(page_id).await.unwrap();

                let data = format!("Page {}", i).into_bytes();
                page.write_data(0, &data).unwrap();

                manager.write_page(&page).await.unwrap();
            }

            manager.flush().await.unwrap();
        }

        // Reopen and verify
        {
            let manager = PageStorageManager::new(&db_path, PagerConfig::default())
                .await
                .unwrap();

            // Total pages = 1 (metadata) + 10 (data)
            assert_eq!(manager.total_pages().await, 11);

            for (i, &page_id) in page_ids.iter().enumerate() {
                let page = manager.read_page(page_id).await.unwrap();
                let expected = format!("Page {}", i).into_bytes();
                let actual = &page.data()[..expected.len()];
                assert_eq!(actual, expected.as_slice());
            }
        }
    }

    #[tokio::test]
    async fn test_checksum_validation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = PagerConfig {
            enable_checksums: true,
            ..Default::default()
        };

        let manager = PageStorageManager::new(&db_path, config).await.unwrap();

        let page_id = manager.allocate_page(PageType::Data).await.unwrap();
        let mut page = manager.read_page(page_id).await.unwrap();

        page.write_data(0, b"Test data").unwrap();
        manager.write_page(&page).await.unwrap();

        // Read back - should validate checksum
        let page = manager.read_page(page_id).await.unwrap();
        assert!(page.verify_checksum());
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = PageStorageManager::new(&db_path, PagerConfig::default())
            .await
            .unwrap();

        // Allocate 5 pages
        let page1 = manager.allocate_page(PageType::Data).await.unwrap();
        let page2 = manager.allocate_page(PageType::Data).await.unwrap();
        let page3 = manager.allocate_page(PageType::Data).await.unwrap();
        let page4 = manager.allocate_page(PageType::Data).await.unwrap();
        let page5 = manager.allocate_page(PageType::Data).await.unwrap();

        // Deallocate 2 pages
        manager.deallocate_page(page2).await.unwrap();
        manager.deallocate_page(page4).await.unwrap();

        let stats = manager.stats().await;
        // Total pages = 1 (metadata) + 5 (data) = 6
        assert_eq!(stats.total_pages, 6);
        assert_eq!(stats.free_pages, 2);
        assert_eq!(stats.used_pages, 4);
    }
}
