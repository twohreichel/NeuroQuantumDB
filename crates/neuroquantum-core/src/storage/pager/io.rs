//! Async file I/O operations for page storage
//!
//! Provides efficient async read/write operations for database pages

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use super::page::{Page, PageId, PAGE_SIZE};
use super::{PagerConfig, SyncMode};

/// Page I/O handler
pub struct PageIO {
    /// Database file handle (wrapped in RwLock for interior mutability)
    file: Arc<RwLock<File>>,
    /// Configuration
    config: PagerConfig,
}

impl PageIO {
    /// Create a new page I/O handler
    pub fn new(file: File, config: PagerConfig) -> Self {
        Self {
            file: Arc::new(RwLock::new(file)),
            config,
        }
    }

    /// Get file size in bytes
    pub async fn file_size(&self) -> Result<u64> {
        let file = self.file.read().await;
        file.metadata()
            .await
            .context("Failed to get file metadata")
            .map(|m| m.len())
    }

    /// Calculate file offset for a page
    fn page_offset(page_id: PageId) -> u64 {
        page_id.0 * PAGE_SIZE as u64
    }

    /// Read a page from disk
    pub async fn read_page(&self, page_id: PageId) -> Result<Page> {
        let offset = Self::page_offset(page_id);

        debug!("📖 Reading page {:?} at offset {}", page_id, offset);

        let mut file = self.file.write().await;

        // Seek to page position
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .context(format!("Failed to seek to page {:?}", page_id))?;

        // Read page data
        let mut buf = vec![0u8; PAGE_SIZE];
        file.read_exact(&mut buf)
            .await
            .context(format!("Failed to read page {:?}", page_id))?;

        // Deserialize page
        Page::from_bytes(&buf).context(format!("Failed to deserialize page {:?}", page_id))
    }

    /// Write a page to disk
    pub async fn write_page(&self, page: &Page) -> Result<()> {
        let page_id = page.id();
        let offset = Self::page_offset(page_id);

        debug!("💾 Writing page {:?} at offset {}", page_id, offset);

        // Serialize page
        let buf = page
            .to_bytes()
            .context(format!("Failed to serialize page {:?}", page_id))?;

        let mut file = self.file.write().await;

        // Seek to page position
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .context(format!("Failed to seek to page {:?}", page_id))?;

        // Write page data
        file.write_all(&buf)
            .await
            .context(format!("Failed to write page {:?}", page_id))?;

        Ok(())
    }

    /// Sync file to disk
    pub async fn sync(&self) -> Result<()> {
        debug!("🔄 Syncing file to disk");

        let file = self.file.write().await;

        file.sync_all().await.context("Failed to sync file to disk")
    }

    /// Read multiple pages in batch (optimization)
    pub async fn read_pages_batch(&self, page_ids: &[PageId]) -> Result<Vec<Page>> {
        let mut pages = Vec::with_capacity(page_ids.len());

        for &page_id in page_ids {
            match self.read_page(page_id).await {
                Ok(page) => pages.push(page),
                Err(e) => {
                    warn!("⚠️ Failed to read page {:?}: {}", page_id, e);
                    return Err(e);
                }
            }
        }

        Ok(pages)
    }

    /// Write multiple pages in batch (optimization)
    pub async fn write_pages_batch(&self, pages: &[Page]) -> Result<()> {
        for page in pages {
            self.write_page(page).await?;
        }

        // Sync if configured
        if self.config.sync_mode == SyncMode::Always {
            self.sync().await?;
        }

        Ok(())
    }

    /// Truncate file to specified size
    pub async fn truncate(&self, size: u64) -> Result<()> {
        debug!("✂️ Truncating file to {} bytes", size);

        let file = self.file.write().await;

        file.set_len(size).await.context("Failed to truncate file")
    }

    /// Pre-allocate space for pages (optimization)
    pub async fn preallocate(&self, num_pages: u64) -> Result<()> {
        let size = num_pages * PAGE_SIZE as u64;

        debug!(
            "📦 Pre-allocating space for {} pages ({} bytes)",
            num_pages, size
        );

        self.truncate(size).await
    }
}

#[cfg(test)]
mod tests {
    use super::super::page::PageType;
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::OpenOptions;

    #[tokio::test]
    async fn test_page_io_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.db");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .await
            .unwrap();

        let io = PageIO::new(file, PagerConfig::default());

        // Create and write page
        let mut page = Page::new(PageId(0), PageType::Data);
        page.write_data(0, b"Test data").unwrap();
        page.update_checksum();

        io.write_page(&page).await.unwrap();

        // Read back
        let read_page = io.read_page(PageId(0)).await.unwrap();
        assert_eq!(read_page.id(), PageId(0));
        assert_eq!(read_page.read_data(0, 9).unwrap(), b"Test data");
    }

    #[tokio::test]
    async fn test_page_io_multiple_pages() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.db");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .await
            .unwrap();

        let io = PageIO::new(file, PagerConfig::default());

        // Write 10 pages
        for i in 0..10 {
            let mut page = Page::new(PageId(i), PageType::Data);
            let data = format!("Page {}", i).into_bytes();
            page.write_data(0, &data).unwrap();
            page.update_checksum();

            io.write_page(&page).await.unwrap();
        }

        // Read back in random order
        for i in [5, 2, 8, 0, 9, 3, 1, 7, 4, 6] {
            let page = io.read_page(PageId(i)).await.unwrap();
            let expected = format!("Page {}", i).into_bytes();
            let actual = page.read_data(0, expected.len()).unwrap();
            assert_eq!(actual, expected.as_slice());
        }
    }

    #[tokio::test]
    async fn test_page_io_batch() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.db");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .await
            .unwrap();

        let io = PageIO::new(file, PagerConfig::default());

        // Create pages
        let pages: Vec<Page> = (0..5)
            .map(|i| {
                let mut page = Page::new(PageId(i), PageType::Data);
                let data = format!("Batch page {}", i).into_bytes();
                page.write_data(0, &data).unwrap();
                page.update_checksum();
                page
            })
            .collect();

        // Write batch
        io.write_pages_batch(&pages).await.unwrap();

        // Read batch
        let page_ids: Vec<PageId> = (0..5).map(PageId).collect();
        let read_pages = io.read_pages_batch(&page_ids).await.unwrap();

        assert_eq!(read_pages.len(), 5);
        for (i, page) in read_pages.iter().enumerate() {
            let expected = format!("Batch page {}", i).into_bytes();
            let actual = page.read_data(0, expected.len()).unwrap();
            assert_eq!(actual, expected.as_slice());
        }
    }

    #[tokio::test]
    async fn test_page_io_sync() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.db");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .await
            .unwrap();

        let io = PageIO::new(file, PagerConfig::default());

        let mut page = Page::new(PageId(0), PageType::Data);
        page.write_data(0, b"Sync test").unwrap();

        io.write_page(&page).await.unwrap();
        io.sync().await.unwrap();

        // File should be synced to disk
        let metadata = tokio::fs::metadata(&file_path).await.unwrap();
        assert!(metadata.len() >= PAGE_SIZE as u64);
    }

    #[tokio::test]
    async fn test_page_io_file_size() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.db");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .await
            .unwrap();

        let io = PageIO::new(file, PagerConfig::default());

        // Initial size should be 0
        assert_eq!(io.file_size().await.unwrap(), 0);

        // Write a page
        let page = Page::new(PageId(0), PageType::Data);
        io.write_page(&page).await.unwrap();

        // Size should be at least PAGE_SIZE
        assert!(io.file_size().await.unwrap() >= PAGE_SIZE as u64);
    }
}
