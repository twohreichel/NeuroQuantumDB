//! Async file I/O operations for page storage
//!
//! Provides efficient async read/write operations for database pages

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tracing::debug;

use super::page::{Page, PageId, PAGE_SIZE};
use super::{PagerConfig, SyncMode};

/// Group of contiguous pages for optimized batch I/O
#[derive(Debug, Clone, Copy)]
struct PageGroup {
    /// Starting page ID
    start: PageId,
    /// Number of contiguous pages
    count: usize,
}

/// Page I/O handler
pub struct PageIO {
    /// Database file handle (wrapped in `RwLock` for interior mutability)
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
    const fn page_offset(page_id: PageId) -> u64 {
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
            .context(format!("Failed to seek to page {page_id:?}"))?;

        // Read page data
        let mut buf = vec![0u8; PAGE_SIZE];
        file.read_exact(&mut buf)
            .await
            .context(format!("Failed to read page {page_id:?}"))?;

        // Deserialize page
        Page::from_bytes(&buf).context(format!("Failed to deserialize page {page_id:?}"))
    }

    /// Write a page to disk
    pub async fn write_page(&self, page: &Page) -> Result<()> {
        let page_id = page.id();
        let offset = Self::page_offset(page_id);

        debug!("💾 Writing page {:?} at offset {}", page_id, offset);

        // Serialize page
        let buf = page
            .to_bytes()
            .context(format!("Failed to serialize page {page_id:?}"))?;

        let mut file = self.file.write().await;

        // Seek to page position
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .context(format!("Failed to seek to page {page_id:?}"))?;

        // Write page data
        file.write_all(&buf)
            .await
            .context(format!("Failed to write page {page_id:?}"))?;

        Ok(())
    }

    /// Sync file to disk
    pub async fn sync(&self) -> Result<()> {
        debug!("🔄 Syncing file to disk");

        let file = self.file.write().await;

        file.sync_all().await.context("Failed to sync file to disk")
    }

    /// Read multiple pages in batch with optimized vectored I/O
    ///
    /// This method optimizes batch reads by:
    /// 1. Sorting pages by ID for sequential access
    /// 2. Grouping contiguous pages
    /// 3. Reading contiguous ranges with fewer syscalls
    /// 4. Reordering results to match the original request order
    pub async fn read_pages_batch(&self, page_ids: &[PageId]) -> Result<Vec<Page>> {
        if page_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Fast path for single page
        if page_ids.len() == 1 {
            return Ok(vec![self.read_page(page_ids[0]).await?]);
        }

        // Create index mapping for reordering later
        let original_order: Vec<(usize, PageId)> = page_ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (i, id))
            .collect();

        // Sort by page_id for sequential access
        let mut sorted_ids: Vec<PageId> = page_ids.to_vec();
        sorted_ids.sort_unstable_by_key(|p| p.0);

        // Group contiguous pages
        let groups = Self::group_contiguous_pages(&sorted_ids);

        debug!(
            "📚 Batch reading {} pages in {} contiguous groups",
            page_ids.len(),
            groups.len()
        );

        // Read each group
        let mut sorted_pages = Vec::with_capacity(page_ids.len());
        for group in groups {
            let pages = self.read_contiguous_pages(group.start, group.count).await?;
            sorted_pages.extend(pages);
        }

        // Reorder pages to match original request order
        Self::reorder_pages(sorted_pages, &original_order)
    }

    /// Write multiple pages in batch with optimized vectored I/O
    ///
    /// This method optimizes batch writes by:
    /// 1. Sorting pages by ID for sequential access
    /// 2. Grouping contiguous pages
    /// 3. Writing contiguous ranges with fewer syscalls
    pub async fn write_pages_batch(&self, pages: &[Page]) -> Result<()> {
        if pages.is_empty() {
            return Ok(());
        }

        // Fast path for single page
        if pages.len() == 1 {
            self.write_page(&pages[0]).await?;
            if self.config.sync_mode == SyncMode::Always {
                self.sync().await?;
            }
            return Ok(());
        }

        // Sort by page_id for sequential access
        let mut sorted_pages = pages.to_vec();
        sorted_pages.sort_unstable_by_key(|p| p.id().0);

        // Group contiguous pages
        let mut current_group_start = 0;
        let mut i = 0;

        while i < sorted_pages.len() {
            let group_start_id = sorted_pages[current_group_start].id().0;
            let mut group_end = current_group_start;

            // Find contiguous range
            while group_end + 1 < sorted_pages.len()
                && sorted_pages[group_end + 1].id().0 == sorted_pages[group_end].id().0 + 1
            {
                group_end += 1;
            }

            // Write contiguous group
            let group_count = group_end - current_group_start + 1;
            if group_count > 1 {
                debug!(
                    "💾 Writing {} contiguous pages starting at {:?}",
                    group_count,
                    PageId(group_start_id)
                );
                self.write_contiguous_pages(&sorted_pages[current_group_start..=group_end])
                    .await?;
            } else {
                self.write_page(&sorted_pages[current_group_start]).await?;
            }

            i = group_end + 1;
            current_group_start = i;
        }

        // Sync if configured
        if self.config.sync_mode == SyncMode::Always {
            self.sync().await?;
        }

        Ok(())
    }

    /// Group contiguous page IDs for optimized batch reads
    fn group_contiguous_pages(sorted_ids: &[PageId]) -> Vec<PageGroup> {
        if sorted_ids.is_empty() {
            return Vec::new();
        }

        let mut groups = Vec::new();
        let mut current_start = sorted_ids[0];
        let mut current_count = 1;

        for i in 1..sorted_ids.len() {
            if sorted_ids[i].0 == sorted_ids[i - 1].0 + 1 {
                // Contiguous page
                current_count += 1;
            } else {
                // Gap detected, save current group
                groups.push(PageGroup {
                    start: current_start,
                    count: current_count,
                });
                current_start = sorted_ids[i];
                current_count = 1;
            }
        }

        // Save final group
        groups.push(PageGroup {
            start: current_start,
            count: current_count,
        });

        groups
    }

    /// Read contiguous pages in a single optimized operation
    async fn read_contiguous_pages(&self, start: PageId, count: usize) -> Result<Vec<Page>> {
        let total_size = count * PAGE_SIZE;
        let offset = Self::page_offset(start);

        debug!(
            "📖 Reading {} contiguous pages starting at {:?} (offset {}, {} bytes)",
            count, start, offset, total_size
        );

        // Allocate buffer for all pages
        let mut buffer = vec![0u8; total_size];

        let mut file = self.file.write().await;

        // Single seek + read for all contiguous pages
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .context(format!("Failed to seek to page {start:?}"))?;

        file.read_exact(&mut buffer).await.context(format!(
            "Failed to read {count} contiguous pages starting at {start:?}"
        ))?;

        // Split buffer into pages
        let mut pages = Vec::with_capacity(count);
        for (i, chunk) in buffer.chunks_exact(PAGE_SIZE).enumerate() {
            let page = Page::from_bytes(chunk).context(format!(
                "Failed to deserialize page {:?}",
                PageId(start.0 + i as u64)
            ))?;
            pages.push(page);
        }

        Ok(pages)
    }

    /// Write contiguous pages in a single optimized operation
    async fn write_contiguous_pages(&self, pages: &[Page]) -> Result<()> {
        if pages.is_empty() {
            return Ok(());
        }

        let start = pages[0].id();
        let count = pages.len();
        let total_size = count * PAGE_SIZE;
        let offset = Self::page_offset(start);

        debug!(
            "💾 Writing {} contiguous pages starting at {:?} (offset {}, {} bytes)",
            count, start, offset, total_size
        );

        // Serialize all pages into single buffer
        let mut buffer = Vec::with_capacity(total_size);
        for page in pages {
            let page_bytes = page
                .to_bytes()
                .context(format!("Failed to serialize page {:?}", page.id()))?;
            buffer.extend_from_slice(&page_bytes);
        }

        let mut file = self.file.write().await;

        // Single seek + write for all contiguous pages
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .context(format!("Failed to seek to page {start:?}"))?;

        file.write_all(&buffer).await.context(format!(
            "Failed to write {count} contiguous pages starting at {start:?}"
        ))?;

        Ok(())
    }

    /// Reorder pages to match the original request order
    fn reorder_pages(
        mut sorted_pages: Vec<Page>,
        original_order: &[(usize, PageId)],
    ) -> Result<Vec<Page>> {
        // Create a map from PageId to Page
        use std::collections::HashMap;
        let mut page_map: HashMap<PageId, Page> =
            sorted_pages.drain(..).map(|p| (p.id(), p)).collect();

        // Build result in original order
        let mut result = Vec::with_capacity(original_order.len());
        for &(_idx, page_id) in original_order {
            let page = page_map
                .remove(&page_id)
                .context(format!("Page {page_id:?} not found in batch read results"))?;
            result.push(page);
        }

        Ok(result)
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
    use tempfile::TempDir;
    use tokio::fs::OpenOptions;

    use super::super::page::PageType;
    use super::*;

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
            let data = format!("Page {i}").into_bytes();
            page.write_data(0, &data).unwrap();
            page.update_checksum();

            io.write_page(&page).await.unwrap();
        }

        // Read back in random order
        for i in [5, 2, 8, 0, 9, 3, 1, 7, 4, 6] {
            let page = io.read_page(PageId(i)).await.unwrap();
            let expected = format!("Page {i}").into_bytes();
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
                let data = format!("Batch page {i}").into_bytes();
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
            let expected = format!("Batch page {i}").into_bytes();
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

        // Sync to ensure data is written to disk
        io.sync().await.unwrap();

        // Size should be at least PAGE_SIZE
        assert!(io.file_size().await.unwrap() >= PAGE_SIZE as u64);
    }
}
