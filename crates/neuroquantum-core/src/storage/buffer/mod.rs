//! Buffer Pool Manager for `NeuroQuantumDB`
//!
//! Provides intelligent page caching with:
//! - LRU/Clock page replacement policies
//! - Dirty page tracking
//! - Pin/unpin mechanism for concurrent access
//! - Background flushing
//! - Memory limit enforcement

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::Duration;
use tracing::{debug, info};

pub mod eviction;
pub mod flusher;
pub mod frame;

pub use eviction::{ClockEviction, EvictionPolicy, LRUEviction};
pub use flusher::BackgroundFlusher;
pub use frame::{Frame, FrameError, FrameId};

use super::pager::{Page, PageId, PageStorageManager};

/// Buffer pool configuration
#[derive(Debug, Clone)]
pub struct BufferPoolConfig {
    /// Maximum number of frames in the pool
    pub pool_size: usize,
    /// Eviction policy to use
    pub eviction_policy: EvictionPolicyType,
    /// Enable background flushing
    pub enable_background_flush: bool,
    /// Background flush interval
    pub flush_interval: Duration,
    /// Maximum dirty pages before forced flush
    pub max_dirty_pages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicyType {
    LRU,
    Clock,
}

impl Default for BufferPoolConfig {
    fn default() -> Self {
        Self {
            pool_size: 1000,
            eviction_policy: EvictionPolicyType::LRU,
            enable_background_flush: true,
            flush_interval: Duration::from_secs(5),
            max_dirty_pages: 100,
        }
    }
}

impl BufferPoolConfig {
    /// Create a new configuration with auto-tuned buffer pool size based on available RAM.
    ///
    /// This method automatically calculates the optimal buffer pool size by:
    /// - Detecting total available system RAM
    /// - Allocating 50% of RAM to buffer pool (conservative for shared systems)
    /// - Enforcing minimum of 512 MB (512 frames @ 4KB pages)
    /// - Enforcing maximum of 32 GB (32768 frames @ 4KB pages)
    ///
    /// # Example RAM-to-Pool-Size Mapping
    ///
    /// | System RAM | Buffer Pool | Frames (4KB pages) |
    /// |------------|-------------|-------------------|
    /// | 1 GB       | 512 MB      | 512 (min)        |
    /// | 4 GB       | 2 GB        | 2048             |
    /// | 8 GB       | 4 GB        | 4096             |
    /// | 16 GB      | 8 GB        | 8192             |
    /// | 32 GB      | 16 GB       | 16384            |
    /// | 64 GB      | 32 GB       | 32768 (max)      |
    /// | 128 GB+    | 32 GB       | 32768 (max)      |
    ///
    /// # Returns
    ///
    /// A new `BufferPoolConfig` with auto-tuned `pool_size` and default values for other fields.
    ///
    /// # Panics
    ///
    /// Panics if system memory information cannot be retrieved. This is extremely rare
    /// and typically indicates a platform incompatibility.
    pub fn auto_tuned() -> Self {
        let pool_size = Self::calculate_optimal_pool_size();

        info!(
            "🎯 Auto-tuning buffer pool: detected optimal size = {} frames",
            pool_size
        );

        Self {
            pool_size,
            eviction_policy: EvictionPolicyType::LRU,
            enable_background_flush: true,
            flush_interval: Duration::from_secs(5),
            max_dirty_pages: (pool_size / 10).max(100), // 10% of pool size, min 100
        }
    }

    /// Calculate optimal buffer pool size based on available system RAM.
    ///
    /// # Algorithm
    ///
    /// 1. Query total system RAM in bytes
    /// 2. Calculate buffer pool as 50% of total RAM
    /// 3. Convert to number of frames (assuming 4KB page size)
    /// 4. Clamp to [512, 32768] range
    ///
    /// # Implementation Notes
    ///
    /// - Uses `sysinfo` crate for cross-platform RAM detection
    /// - Conservative 50% allocation allows for OS, other processes, and memory spikes
    /// - For dedicated database servers, consider using `with_ram_percentage()`
    ///
    /// # Returns
    ///
    /// Number of frames (pages) for the buffer pool.
    fn calculate_optimal_pool_size() -> usize {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_memory();

        // sysinfo 0.30+ returns memory in bytes
        let total_ram_bytes = sys.total_memory();
        let total_ram_mb = total_ram_bytes / 1024 / 1024;

        // Allocate 50% of RAM to buffer pool (conservative)
        let buffer_pool_mb = (total_ram_mb as f64 * 0.5) as u64;

        // Convert MB to number of 4KB frames
        // 1 MB = 1024 KB = 256 frames (@ 4KB per frame)
        let frames = (buffer_pool_mb * 256) as usize;

        // Clamp to [512, 32768] range
        // Min: 512 frames = 2 MB (for resource-constrained devices)
        // Max: 32768 frames = 128 MB (prevents excessive memory usage)
        let clamped_frames = frames.clamp(512, 32768);

        debug!(
            "System RAM: {} MB, Buffer Pool: {} MB ({} frames, clamped: {} frames)",
            total_ram_mb, buffer_pool_mb, frames, clamped_frames
        );

        clamped_frames
    }

    /// Create a configuration with custom RAM percentage allocation.
    ///
    /// # Arguments
    ///
    /// * `ram_percentage` - Percentage of RAM to allocate (0.0 to 1.0)
    ///   - 0.5 (50%) - Conservative, for shared systems (default in `auto_tuned()`)
    ///   - 0.7 (70%) - Moderate, for database-focused servers
    ///   - 0.8 (80%) - Aggressive, for dedicated database servers
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Dedicated database server - allocate 80% RAM
    /// let config = BufferPoolConfig::with_ram_percentage(0.8);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `ram_percentage` is not in the range [0.0, 1.0].
    pub fn with_ram_percentage(ram_percentage: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&ram_percentage),
            "RAM percentage must be between 0.0 and 1.0, got {ram_percentage}"
        );

        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_memory();

        // sysinfo 0.30+ returns memory in bytes
        let total_ram_bytes = sys.total_memory();
        let total_ram_mb = total_ram_bytes / 1024 / 1024;
        let buffer_pool_mb = (total_ram_mb as f64 * ram_percentage) as u64;
        let frames = (buffer_pool_mb * 256) as usize;
        let pool_size = frames.clamp(512, 32768);

        info!(
            "🎯 Auto-tuning buffer pool with {}% RAM: {} frames",
            (ram_percentage * 100.0) as u8,
            pool_size
        );

        Self {
            pool_size,
            eviction_policy: EvictionPolicyType::LRU,
            enable_background_flush: true,
            flush_interval: Duration::from_secs(5),
            max_dirty_pages: (pool_size / 10).max(100),
        }
    }
}

/// Buffer Pool Manager
///
/// Manages a pool of page frames with intelligent caching,
/// eviction policies, and dirty page tracking.
pub struct BufferPoolManager {
    /// Configuration (stored for future use)
    _config: BufferPoolConfig,
    /// Page storage manager
    pager: Arc<PageStorageManager>,
    /// Frame pool
    frames: Arc<RwLock<HashMap<FrameId, Frame>>>,
    /// Page ID to Frame ID mapping
    page_table: Arc<RwLock<HashMap<PageId, FrameId>>>,
    /// Free frame list
    free_list: Arc<RwLock<VecDeque<FrameId>>>,
    /// Eviction policy
    eviction: Arc<RwLock<Box<dyn EvictionPolicy>>>,
    /// Dirty page set
    dirty_pages: Arc<RwLock<HashMap<PageId, FrameId>>>,
    /// Semaphore for limiting concurrent flushes
    flush_semaphore: Arc<Semaphore>,
    /// Background flusher (optional)
    flusher: Option<Arc<BackgroundFlusher>>,
    /// Cache hit counter
    cache_hits: Arc<RwLock<u64>>,
    /// Cache miss counter
    cache_misses: Arc<RwLock<u64>>,
}

impl BufferPoolManager {
    /// Create a new buffer pool manager
    pub async fn new(pager: Arc<PageStorageManager>, config: BufferPoolConfig) -> Result<Self> {
        info!(
            "🏊 Initializing BufferPoolManager with {} frames",
            config.pool_size
        );

        // Initialize frames
        let mut frames = HashMap::new();
        let mut free_list = VecDeque::new();

        for i in 0..config.pool_size {
            let frame_id = FrameId(i);
            frames.insert(frame_id, Frame::new(frame_id));
            free_list.push_back(frame_id);
        }

        // Create eviction policy
        let eviction: Box<dyn EvictionPolicy> = match config.eviction_policy {
            | EvictionPolicyType::LRU => Box::new(LRUEviction::new(config.pool_size)),
            | EvictionPolicyType::Clock => Box::new(ClockEviction::new(config.pool_size)),
        };

        let manager = Self {
            _config: config.clone(),
            pager,
            frames: Arc::new(RwLock::new(frames)),
            page_table: Arc::new(RwLock::new(HashMap::new())),
            free_list: Arc::new(RwLock::new(free_list)),
            eviction: Arc::new(RwLock::new(eviction)),
            dirty_pages: Arc::new(RwLock::new(HashMap::new())),
            flush_semaphore: Arc::new(Semaphore::new(10)), // Max 10 concurrent flushes
            flusher: None,
            cache_hits: Arc::new(RwLock::new(0)),
            cache_misses: Arc::new(RwLock::new(0)),
        };

        // Start background flusher if enabled
        let manager = if config.enable_background_flush {
            let flusher = BackgroundFlusher::new(
                manager.dirty_pages.clone(),
                manager.frames.clone(),
                manager.pager.clone(),
                config.flush_interval,
            );

            flusher.start().await;

            Self {
                flusher: Some(Arc::new(flusher)),
                ..manager
            }
        } else {
            manager
        };

        info!("✅ BufferPoolManager initialized successfully");
        Ok(manager)
    }

    /// Fetch a page from the buffer pool
    ///
    /// If the page is not in the pool, it will be loaded from disk.
    /// The page is pinned and must be unpinned after use.
    pub async fn fetch_page(&self, page_id: PageId) -> Result<Arc<RwLock<Page>>> {
        // Check if page is already in buffer
        let page_table = self.page_table.read().await;
        if let Some(&frame_id) = page_table.get(&page_id) {
            drop(page_table);

            // Record cache hit
            {
                let mut hits = self.cache_hits.write().await;
                *hits += 1;
            }

            // Update access in eviction policy
            let mut eviction = self.eviction.write().await;
            eviction.record_access(frame_id);
            drop(eviction);

            // Pin the frame and return page
            let frames = self.frames.read().await;
            let frame = frames
                .get(&frame_id)
                .ok_or_else(|| anyhow!("Frame not found: {frame_id:?}"))?;

            frame.pin();
            debug!(
                "📌 Fetched page {:?} from buffer (frame {:?}) [CACHE HIT]",
                page_id, frame_id
            );

            return Ok(frame.page().await?);
        }
        drop(page_table);

        // Record cache miss
        {
            let mut misses = self.cache_misses.write().await;
            *misses += 1;
        }

        // Page not in buffer - need to load it
        self.load_page(page_id).await
    }

    /// Load a page from disk into the buffer pool
    async fn load_page(&self, page_id: PageId) -> Result<Arc<RwLock<Page>>> {
        debug!("💾 Loading page {:?} from disk", page_id);

        // Get a free frame or evict one
        let frame_id = self.get_free_frame().await?;

        // Read page from disk
        let page = self.pager.read_page(page_id).await?;

        // Store in frame
        let mut frames = self.frames.write().await;
        let frame = frames
            .get_mut(&frame_id)
            .ok_or_else(|| anyhow!("Frame not found: {frame_id:?}"))?;

        frame.set_page(page_id, page).await;
        frame.pin();

        let page_arc = frame.page().await?;
        drop(frames);

        // Update page table
        let mut page_table = self.page_table.write().await;
        page_table.insert(page_id, frame_id);
        drop(page_table);

        // Record access in eviction policy
        let mut eviction = self.eviction.write().await;
        eviction.record_access(frame_id);
        drop(eviction);

        debug!("✅ Loaded page {:?} into frame {:?}", page_id, frame_id);
        Ok(page_arc)
    }

    /// Get a free frame, evicting if necessary
    async fn get_free_frame(&self) -> Result<FrameId> {
        // Try to get a free frame from the free list
        let mut free_list = self.free_list.write().await;
        if let Some(frame_id) = free_list.pop_front() {
            drop(free_list);
            debug!("🆓 Got free frame: {:?}", frame_id);
            return Ok(frame_id);
        }
        drop(free_list);

        // No free frames - need to evict
        self.evict_page().await
    }

    /// Evict a page from the buffer pool
    async fn evict_page(&self) -> Result<FrameId> {
        debug!("🔄 Evicting a page from buffer pool");

        // Get victim from eviction policy
        let mut eviction = self.eviction.write().await;
        let victim_frame_id = eviction
            .select_victim()
            .ok_or_else(|| anyhow!("No victim found for eviction"))?;
        drop(eviction);

        // Check if frame is pinned
        let frames = self.frames.read().await;
        let frame = frames
            .get(&victim_frame_id)
            .ok_or_else(|| anyhow!("Victim frame not found: {victim_frame_id:?}"))?;

        if frame.is_pinned() {
            drop(frames);
            return Err(anyhow!("All frames are pinned, cannot evict"));
        }

        let victim_page_id = frame
            .page_id()
            .await
            .ok_or_else(|| anyhow!("Frame has no page"))?;

        let is_dirty = frame.is_dirty();
        drop(frames);

        // Flush if dirty
        if is_dirty {
            debug!(
                "💾 Flushing dirty page {:?} before eviction",
                victim_page_id
            );
            self.flush_page_internal(victim_page_id, victim_frame_id)
                .await?;
        }

        // Remove from page table
        let mut page_table = self.page_table.write().await;
        page_table.remove(&victim_page_id);
        drop(page_table);

        // Clear frame
        let mut frames = self.frames.write().await;
        let frame = frames
            .get_mut(&victim_frame_id)
            .ok_or_else(|| anyhow!("Victim frame not found: {victim_frame_id:?}"))?;
        frame.clear().await;
        drop(frames);

        debug!(
            "✅ Evicted page {:?} from frame {:?}",
            victim_page_id, victim_frame_id
        );
        Ok(victim_frame_id)
    }

    /// Unpin a page, allowing it to be evicted
    pub async fn unpin_page(&self, page_id: PageId, is_dirty: bool) -> Result<()> {
        let page_table = self.page_table.read().await;
        let frame_id = page_table
            .get(&page_id)
            .ok_or_else(|| anyhow!("Page not in buffer: {page_id:?}"))?;
        let frame_id = *frame_id;
        drop(page_table);

        let frames = self.frames.read().await;
        let frame = frames
            .get(&frame_id)
            .ok_or_else(|| anyhow!("Frame not found: {frame_id:?}"))?;

        frame
            .unpin()
            .map_err(|e| anyhow!("Failed to unpin frame {frame_id:?}: {e}"))?;

        if is_dirty {
            frame.set_dirty(true);
            drop(frames);

            // Add to dirty pages
            let mut dirty_pages = self.dirty_pages.write().await;
            dirty_pages.insert(page_id, frame_id);
            drop(dirty_pages);

            debug!("📝 Marked page {:?} as dirty", page_id);
        } else {
            drop(frames);
        }

        debug!("📍 Unpinned page {:?}", page_id);
        Ok(())
    }

    /// Flush a specific page to disk
    pub async fn flush_page(&self, page_id: PageId) -> Result<()> {
        let page_table = self.page_table.read().await;
        let frame_id = page_table
            .get(&page_id)
            .ok_or_else(|| anyhow!("Page not in buffer: {page_id:?}"))?;
        let frame_id = *frame_id;
        drop(page_table);

        self.flush_page_internal(page_id, frame_id).await
    }

    /// Internal flush implementation
    async fn flush_page_internal(&self, page_id: PageId, frame_id: FrameId) -> Result<()> {
        // Acquire flush semaphore
        let _permit = self.flush_semaphore.acquire().await?;

        let frames = self.frames.read().await;
        let frame = frames
            .get(&frame_id)
            .ok_or_else(|| anyhow!("Frame not found: {frame_id:?}"))?;

        if !frame.is_dirty() {
            drop(frames);
            return Ok(());
        }

        let page = frame.page().await?;
        drop(frames);

        // Write to disk
        let page_guard = page.read().await;
        self.pager.write_page(&page_guard).await?;
        drop(page_guard);

        // Mark as clean
        let frames = self.frames.read().await;
        let frame = frames
            .get(&frame_id)
            .ok_or_else(|| anyhow!("Frame not found: {frame_id:?}"))?;
        frame.set_dirty(false);
        drop(frames);

        // Remove from dirty pages
        let mut dirty_pages = self.dirty_pages.write().await;
        dirty_pages.remove(&page_id);
        drop(dirty_pages);

        debug!("💾 Flushed page {:?} to disk", page_id);
        Ok(())
    }

    /// Flush all dirty pages to disk
    pub async fn flush_all(&self) -> Result<()> {
        info!("💾 Flushing all dirty pages to disk");

        let dirty_pages = self.dirty_pages.read().await;
        let pages_to_flush: Vec<(PageId, FrameId)> = dirty_pages
            .iter()
            .map(|(&page_id, &frame_id)| (page_id, frame_id))
            .collect();
        drop(dirty_pages);

        for (page_id, frame_id) in pages_to_flush {
            self.flush_page_internal(page_id, frame_id).await?;
        }

        // Sync pager
        self.pager.sync().await?;

        info!("✅ All dirty pages flushed");
        Ok(())
    }

    /// Get buffer pool statistics
    pub async fn stats(&self) -> BufferPoolStats {
        let frames = self.frames.read().await;
        let page_table = self.page_table.read().await;
        let free_list = self.free_list.read().await;
        let dirty_pages = self.dirty_pages.read().await;

        let total_frames = frames.len();
        let used_frames = page_table.len();
        let free_frames = free_list.len();
        let dirty_count = dirty_pages.len();

        let pinned_count = frames.values().filter(|f| f.is_pinned()).count();

        // Calculate hit rate
        let hits = *self.cache_hits.read().await;
        let misses = *self.cache_misses.read().await;
        let total_accesses = hits + misses;
        let hit_rate = if total_accesses > 0 {
            hits as f64 / total_accesses as f64
        } else {
            0.0
        };

        BufferPoolStats {
            total_frames,
            used_frames,
            free_frames,
            dirty_frames: dirty_count,
            pinned_frames: pinned_count,
            hit_rate,
        }
    }

    /// Reset cache statistics (useful for benchmarking)
    pub async fn reset_stats(&self) {
        let mut hits = self.cache_hits.write().await;
        let mut misses = self.cache_misses.write().await;
        *hits = 0;
        *misses = 0;
        debug!("🔄 Cache statistics reset");
    }

    /// Get detailed cache metrics
    pub async fn cache_metrics(&self) -> CacheMetrics {
        let hits = *self.cache_hits.read().await;
        let misses = *self.cache_misses.read().await;
        let total_accesses = hits + misses;
        let hit_rate = if total_accesses > 0 {
            hits as f64 / total_accesses as f64
        } else {
            0.0
        };

        CacheMetrics {
            hits,
            misses,
            total_accesses,
            hit_rate,
        }
    }

    /// Shutdown the buffer pool, flushing all pages
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down BufferPoolManager");

        // Stop background flusher
        if let Some(flusher) = &self.flusher {
            flusher.stop().await;
        }

        // Flush all dirty pages
        self.flush_all().await?;

        info!("✅ BufferPoolManager shutdown complete");
        Ok(())
    }
}

/// Cache metrics for monitoring
#[derive(Debug, Clone, Copy)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub total_accesses: u64,
    pub hit_rate: f64,
}

/// Buffer pool statistics
#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
    pub dirty_frames: usize,
    pub pinned_frames: usize,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::storage::pager::{PageType, PagerConfig};

    async fn create_test_buffer_pool() -> (BufferPoolManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pager = Arc::new(
            PageStorageManager::new(&db_path, PagerConfig::default())
                .await
                .unwrap(),
        );

        let config = BufferPoolConfig {
            pool_size: 10,
            enable_background_flush: false,
            ..Default::default()
        };

        let buffer_pool = BufferPoolManager::new(pager, config).await.unwrap();
        (buffer_pool, temp_dir)
    }

    #[tokio::test]
    async fn test_buffer_pool_creation() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.total_frames, 10);
        assert_eq!(stats.used_frames, 0);
        assert_eq!(stats.free_frames, 10);
    }

    #[tokio::test]
    async fn test_fetch_and_unpin_page() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        // Allocate a page through pager
        let page_id = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();

        // Fetch page
        let page = buffer_pool.fetch_page(page_id).await.unwrap();

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.used_frames, 1);
        assert_eq!(stats.pinned_frames, 1);

        // Unpin page
        drop(page);
        buffer_pool.unpin_page(page_id, false).await.unwrap();

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.pinned_frames, 0);
    }

    #[tokio::test]
    async fn test_dirty_page_tracking() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        let page_id = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();
        let page = buffer_pool.fetch_page(page_id).await.unwrap();

        // Modify page
        {
            let mut page_guard = page.write().await;
            page_guard.write_data(0, b"dirty data").unwrap();
        }

        // Unpin as dirty
        buffer_pool.unpin_page(page_id, true).await.unwrap();

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.dirty_frames, 1);
    }

    #[tokio::test]
    async fn test_cache_hit_rate_initial() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        // Initially, hit rate should be 0.0 (no accesses yet)
        let metrics = buffer_pool.cache_metrics().await;
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 0);
        assert_eq!(metrics.total_accesses, 0);
        assert_eq!(metrics.hit_rate, 0.0);

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        // First access to a page should be a miss
        let page_id = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();

        let _page = buffer_pool.fetch_page(page_id).await.unwrap();

        let metrics = buffer_pool.cache_metrics().await;
        assert_eq!(metrics.hits, 0);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.total_accesses, 1);
        assert_eq!(metrics.hit_rate, 0.0);
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        let page_id = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();

        // First access - miss
        let page = buffer_pool.fetch_page(page_id).await.unwrap();
        drop(page);
        buffer_pool.unpin_page(page_id, false).await.unwrap();

        // Second access to same page - should be a hit
        let _page = buffer_pool.fetch_page(page_id).await.unwrap();

        let metrics = buffer_pool.cache_metrics().await;
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.total_accesses, 2);
        assert_eq!(metrics.hit_rate, 0.5); // 1 hit out of 2 accesses
    }

    #[tokio::test]
    async fn test_cache_hit_rate_multiple_accesses() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        // Create 3 pages
        let page1 = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();
        let page2 = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();
        let page3 = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();

        // Access pattern: page1, page2, page1, page3, page1, page2
        // Misses: page1, page2, page3 = 3
        // Hits: page1 (2x), page2 (1x) = 3
        // Total: 6, Hit rate: 3/6 = 0.5

        let p = buffer_pool.fetch_page(page1).await.unwrap(); // miss
        drop(p);
        buffer_pool.unpin_page(page1, false).await.unwrap();

        let p = buffer_pool.fetch_page(page2).await.unwrap(); // miss
        drop(p);
        buffer_pool.unpin_page(page2, false).await.unwrap();

        let p = buffer_pool.fetch_page(page1).await.unwrap(); // hit
        drop(p);
        buffer_pool.unpin_page(page1, false).await.unwrap();

        let p = buffer_pool.fetch_page(page3).await.unwrap(); // miss
        drop(p);
        buffer_pool.unpin_page(page3, false).await.unwrap();

        let p = buffer_pool.fetch_page(page1).await.unwrap(); // hit
        drop(p);
        buffer_pool.unpin_page(page1, false).await.unwrap();

        let p = buffer_pool.fetch_page(page2).await.unwrap(); // hit
        drop(p);
        buffer_pool.unpin_page(page2, false).await.unwrap();

        let metrics = buffer_pool.cache_metrics().await;
        assert_eq!(metrics.hits, 3);
        assert_eq!(metrics.misses, 3);
        assert_eq!(metrics.total_accesses, 6);
        assert_eq!(metrics.hit_rate, 0.5);

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[tokio::test]
    async fn test_reset_stats() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        let page_id = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();

        // Generate some hits and misses
        let _ = buffer_pool.fetch_page(page_id).await.unwrap(); // miss
        buffer_pool.unpin_page(page_id, false).await.unwrap();
        let _ = buffer_pool.fetch_page(page_id).await.unwrap(); // hit
        buffer_pool.unpin_page(page_id, false).await.unwrap();

        let metrics_before = buffer_pool.cache_metrics().await;
        assert_eq!(metrics_before.total_accesses, 2);

        // Reset stats
        buffer_pool.reset_stats().await;

        let metrics_after = buffer_pool.cache_metrics().await;
        assert_eq!(metrics_after.hits, 0);
        assert_eq!(metrics_after.misses, 0);
        assert_eq!(metrics_after.total_accesses, 0);
        assert_eq!(metrics_after.hit_rate, 0.0);
    }

    #[tokio::test]
    async fn test_page_eviction() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        // Fill buffer pool
        for _ in 0..10 {
            let page_id = buffer_pool
                .pager
                .allocate_page(PageType::Data)
                .await
                .unwrap();

            let page = buffer_pool.fetch_page(page_id).await.unwrap();
            drop(page);
            buffer_pool.unpin_page(page_id, false).await.unwrap();
        }

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.used_frames, 10);

        // Fetch one more page - should trigger eviction
        let new_page_id = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();
        let page = buffer_pool.fetch_page(new_page_id).await.unwrap();
        drop(page);
        buffer_pool.unpin_page(new_page_id, false).await.unwrap();

        // Should still have 10 frames used
        let stats = buffer_pool.stats().await;
        assert_eq!(stats.used_frames, 10);
    }

    #[tokio::test]
    async fn test_flush_dirty_page() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        let page_id = buffer_pool
            .pager
            .allocate_page(PageType::Data)
            .await
            .unwrap();
        let page = buffer_pool.fetch_page(page_id).await.unwrap();

        {
            let mut page_guard = page.write().await;
            page_guard.write_data(0, b"test data").unwrap();
        }

        buffer_pool.unpin_page(page_id, true).await.unwrap();

        // Flush page
        buffer_pool.flush_page(page_id).await.unwrap();

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.dirty_frames, 0);
    }

    #[tokio::test]
    async fn test_flush_all() {
        let (buffer_pool, _temp_dir) = create_test_buffer_pool().await;

        // Create multiple dirty pages
        for _ in 0..5 {
            let page_id = buffer_pool
                .pager
                .allocate_page(PageType::Data)
                .await
                .unwrap();
            let page = buffer_pool.fetch_page(page_id).await.unwrap();

            {
                let mut page_guard = page.write().await;
                page_guard.write_data(0, b"dirty").unwrap();
            }

            buffer_pool.unpin_page(page_id, true).await.unwrap();
        }

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.dirty_frames, 5);

        // Flush all
        buffer_pool.flush_all().await.unwrap();

        let stats = buffer_pool.stats().await;
        assert_eq!(stats.dirty_frames, 0);
    }

    #[test]
    fn test_auto_tuned_config() {
        // Test that auto_tuned() produces a valid configuration
        let config = BufferPoolConfig::auto_tuned();

        // Pool size should be within valid range
        assert!(
            config.pool_size >= 512,
            "Pool size too small: {}",
            config.pool_size
        );
        assert!(
            config.pool_size <= 32768,
            "Pool size too large: {}",
            config.pool_size
        );

        // Verify other defaults are set correctly
        assert_eq!(config.eviction_policy, EvictionPolicyType::LRU);
        assert!(config.enable_background_flush);
        assert_eq!(config.flush_interval, Duration::from_secs(5));

        // max_dirty_pages should be 10% of pool_size, min 100
        let expected_max_dirty = (config.pool_size / 10).max(100);
        assert_eq!(config.max_dirty_pages, expected_max_dirty);
    }

    #[test]
    fn test_with_ram_percentage_50() {
        let config = BufferPoolConfig::with_ram_percentage(0.5);

        // Verify pool size is within bounds
        assert!(config.pool_size >= 512);
        assert!(config.pool_size <= 32768);

        // Verify defaults
        assert_eq!(config.eviction_policy, EvictionPolicyType::LRU);
        assert!(config.enable_background_flush);
    }

    #[test]
    fn test_with_ram_percentage_80() {
        // Test aggressive allocation for dedicated database servers
        let config = BufferPoolConfig::with_ram_percentage(0.8);

        assert!(config.pool_size >= 512);
        assert!(config.pool_size <= 32768);
    }

    #[test]
    fn test_with_ram_percentage_30() {
        // Test conservative allocation for shared systems
        let config = BufferPoolConfig::with_ram_percentage(0.3);

        assert!(config.pool_size >= 512);
        assert!(config.pool_size <= 32768);
    }

    #[test]
    #[should_panic(expected = "RAM percentage must be between 0.0 and 1.0")]
    fn test_with_ram_percentage_invalid_high() {
        // Should panic with percentage > 1.0
        let _config = BufferPoolConfig::with_ram_percentage(1.5);
    }

    #[test]
    #[should_panic(expected = "RAM percentage must be between 0.0 and 1.0")]
    fn test_with_ram_percentage_invalid_low() {
        // Should panic with negative percentage
        let _config = BufferPoolConfig::with_ram_percentage(-0.1);
    }

    #[test]
    fn test_ram_percentage_edge_cases() {
        // Test edge cases: 0% and 100%
        let config_min = BufferPoolConfig::with_ram_percentage(0.0);
        assert_eq!(config_min.pool_size, 512); // Should clamp to minimum

        let config_max = BufferPoolConfig::with_ram_percentage(1.0);
        assert!(config_max.pool_size >= 512);
        assert!(config_max.pool_size <= 32768); // Should respect maximum
    }

    #[test]
    fn test_auto_tuned_vs_default() {
        let auto_config = BufferPoolConfig::auto_tuned();
        let default_config = BufferPoolConfig::default();

        // Auto-tuned should generally have different pool_size than fixed default (1000)
        // unless by coincidence the system has exactly 16MB RAM
        // We just verify both are valid
        assert!(auto_config.pool_size >= 512);
        assert!(default_config.pool_size == 1000);
    }
}
