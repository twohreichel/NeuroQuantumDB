//! Background flusher for dirty pages
//!
//! Periodically flushes dirty pages to disk to prevent excessive dirty page buildup.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

use super::super::pager::{PageId, PageStorageManager};
use super::frame::{Frame, FrameId};

/// Background flusher
pub struct BackgroundFlusher {
    /// Dirty page set (shared with buffer pool)
    dirty_pages: Arc<RwLock<HashMap<PageId, FrameId>>>,
    /// Frame pool (shared with buffer pool)
    frames: Arc<RwLock<HashMap<FrameId, Frame>>>,
    /// Page storage manager
    pager: Arc<PageStorageManager>,
    /// Flush interval
    interval: Duration,
    /// Running flag
    running: Arc<AtomicBool>,
}

impl BackgroundFlusher {
    /// Create a new background flusher
    pub fn new(
        dirty_pages: Arc<RwLock<HashMap<PageId, FrameId>>>,
        frames: Arc<RwLock<HashMap<FrameId, Frame>>>,
        pager: Arc<PageStorageManager>,
        interval: Duration,
    ) -> Self {
        Self {
            dirty_pages,
            frames,
            pager,
            interval,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the background flusher
    pub async fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            warn!("Background flusher already running");
            return;
        }

        info!(
            "🚀 Starting background flusher (interval: {:?})",
            self.interval
        );

        let dirty_pages = self.dirty_pages.clone();
        let frames = self.frames.clone();
        let pager = self.pager.clone();
        let interval_duration = self.interval;
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut ticker = interval(interval_duration);

            while running.load(Ordering::SeqCst) {
                ticker.tick().await;

                // Get snapshot of dirty pages
                let dirty_snapshot = {
                    let dirty = dirty_pages.read().await;
                    dirty
                        .iter()
                        .map(|(&page_id, &frame_id)| (page_id, frame_id))
                        .collect::<Vec<_>>()
                };

                if dirty_snapshot.is_empty() {
                    continue;
                }

                debug!("🧹 Background flush: {} dirty pages", dirty_snapshot.len());

                // Flush each dirty page
                let mut flushed = 0;
                for (page_id, frame_id) in dirty_snapshot {
                    // Get frame
                    let frames_guard = frames.read().await;
                    let frame = match frames_guard.get(&frame_id) {
                        Some(f) => f,
                        None => continue,
                    };

                    if !frame.is_dirty() {
                        drop(frames_guard);
                        continue;
                    }

                    // Don't flush pinned pages
                    if frame.is_pinned() {
                        drop(frames_guard);
                        continue;
                    }

                    let page = frame.page().await;
                    drop(frames_guard);

                    // Write to disk
                    match Self::flush_page_internal(
                        &pager,
                        &page,
                        page_id,
                        frame_id,
                        &frames,
                        &dirty_pages,
                    )
                    .await
                    {
                        Ok(_) => flushed += 1,
                        Err(e) => warn!("Failed to flush page {:?}: {}", page_id, e),
                    }
                }

                if flushed > 0 {
                    debug!("✅ Background flush: flushed {} pages", flushed);
                }
            }

            info!("🛑 Background flusher stopped");
        });
    }

    /// Internal flush implementation
    async fn flush_page_internal(
        pager: &Arc<PageStorageManager>,
        page: &Arc<RwLock<crate::storage::pager::Page>>,
        page_id: PageId,
        frame_id: FrameId,
        frames: &Arc<RwLock<HashMap<FrameId, Frame>>>,
        dirty_pages: &Arc<RwLock<HashMap<PageId, FrameId>>>,
    ) -> anyhow::Result<()> {
        // Write to disk
        let page_guard = page.read().await;
        pager.write_page(&page_guard).await?;
        drop(page_guard);

        // Mark as clean
        let frames_guard = frames.read().await;
        if let Some(frame) = frames_guard.get(&frame_id) {
            frame.set_dirty(false);
        }
        drop(frames_guard);

        // Remove from dirty pages
        let mut dirty = dirty_pages.write().await;
        dirty.remove(&page_id);
        drop(dirty);

        Ok(())
    }

    /// Stop the background flusher
    pub async fn stop(&self) {
        info!("🛑 Stopping background flusher");
        self.running.store(false, Ordering::SeqCst);

        // Give it some time to finish current iteration
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    /// Check if flusher is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::buffer::frame::Frame;
    use crate::storage::pager::{PageType, PagerConfig};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_background_flusher_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pager = Arc::new(
            PageStorageManager::new(&db_path, PagerConfig::default())
                .await
                .unwrap(),
        );

        let dirty_pages = Arc::new(RwLock::new(HashMap::new()));
        let frames = Arc::new(RwLock::new(HashMap::new()));

        let flusher =
            BackgroundFlusher::new(dirty_pages, frames, pager, Duration::from_millis(100));

        assert!(!flusher.is_running());

        flusher.start().await;
        assert!(flusher.is_running());

        tokio::time::sleep(Duration::from_millis(50)).await;

        flusher.stop().await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(!flusher.is_running());
    }

    #[tokio::test]
    async fn test_background_flusher_flushes_dirty_pages() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pager = Arc::new(
            PageStorageManager::new(&db_path, PagerConfig::default())
                .await
                .unwrap(),
        );

        // Create a dirty page
        let page_id = pager.allocate_page(PageType::Data).await.unwrap();
        let mut page = pager.read_page(page_id).await.unwrap();
        page.write_data(0, b"dirty data").unwrap();
        page.update_checksum();

        let frame = Frame::new(FrameId(0));
        frame.set_page(page_id, page).await;
        frame.set_dirty(true);

        let mut frames_map = HashMap::new();
        frames_map.insert(FrameId(0), frame);
        let frames = Arc::new(RwLock::new(frames_map));

        let mut dirty_map = HashMap::new();
        dirty_map.insert(page_id, FrameId(0));
        let dirty_pages = Arc::new(RwLock::new(dirty_map));

        let flusher = BackgroundFlusher::new(
            dirty_pages.clone(),
            frames,
            pager,
            Duration::from_millis(50),
        );

        flusher.start().await;

        // Wait for flush
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Check that dirty page was flushed
        let dirty = dirty_pages.read().await;
        assert_eq!(dirty.len(), 0);

        flusher.stop().await;
    }
}
