//! Frame management for buffer pool
//!
//! A frame represents a slot in the buffer pool that can hold a page.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use super::super::pager::{Page, PageId};

/// Error type for frame operations
#[derive(Debug, Error)]
pub enum FrameError {
    /// Frame is empty when a page was expected
    #[error("Frame is empty: no page has been set")]
    EmptyFrame,
}

/// Frame identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(pub usize);

/// Type alias for page content stored in a frame
type PageContent = Arc<RwLock<Option<(PageId, Arc<RwLock<Page>>)>>>;

/// A frame in the buffer pool
pub struct Frame {
    /// Frame ID
    id: FrameId,
    /// Page stored in this frame (None if empty)
    page: PageContent,
    /// Pin count (number of threads using this frame)
    pin_count: Arc<AtomicUsize>,
    /// Dirty flag
    is_dirty: Arc<AtomicBool>,
}

impl Frame {
    /// Create a new empty frame
    #[must_use] 
    pub fn new(id: FrameId) -> Self {
        Self {
            id,
            page: Arc::new(RwLock::new(None)),
            pin_count: Arc::new(AtomicUsize::new(0)),
            is_dirty: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get frame ID
    #[must_use] 
    pub const fn id(&self) -> FrameId {
        self.id
    }

    /// Set page in this frame
    pub async fn set_page(&self, page_id: PageId, page: Page) {
        let page_arc = Arc::new(RwLock::new(page));
        let mut guard = self.page.write().await;
        *guard = Some((page_id, page_arc));
        self.is_dirty.store(false, Ordering::SeqCst);
    }

    /// Get page from this frame
    ///
    /// # Errors
    ///
    /// Returns `FrameError::EmptyFrame` if no page has been set in this frame.
    pub async fn page(&self) -> Result<Arc<RwLock<Page>>, FrameError> {
        let guard = self.page.read().await;
        guard
            .as_ref()
            .map(|(_, page)| page.clone())
            .ok_or(FrameError::EmptyFrame)
    }

    /// Get page ID of the page in this frame
    pub async fn page_id(&self) -> Option<PageId> {
        let guard = self.page.read().await;
        guard.as_ref().map(|(page_id, _)| *page_id)
    }

    /// Check if frame is empty
    pub async fn is_empty(&self) -> bool {
        let guard = self.page.read().await;
        guard.is_none()
    }

    /// Pin this frame (increment pin count)
    pub fn pin(&self) {
        self.pin_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Unpin this frame (decrement pin count)
    ///
    /// # Errors
    ///
    /// Returns an error if the frame was not pinned (pin count already 0)
    pub fn unpin(&self) -> Result<(), &'static str> {
        let prev = self.pin_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 0 {
            // Restore the count since we failed
            self.pin_count.fetch_add(1, Ordering::SeqCst);
            return Err("Attempted to unpin a frame that was not pinned");
        }
        Ok(())
    }

    /// Check if frame is pinned
    #[must_use] 
    pub fn is_pinned(&self) -> bool {
        self.pin_count.load(Ordering::SeqCst) > 0
    }

    /// Get pin count
    #[must_use] 
    pub fn pin_count(&self) -> usize {
        self.pin_count.load(Ordering::SeqCst)
    }

    /// Mark frame as dirty
    pub fn set_dirty(&self, dirty: bool) {
        self.is_dirty.store(dirty, Ordering::SeqCst);
    }

    /// Check if frame is dirty
    #[must_use] 
    pub fn is_dirty(&self) -> bool {
        self.is_dirty.load(Ordering::SeqCst)
    }

    /// Clear frame (remove page)
    pub async fn clear(&self) {
        let mut guard = self.page.write().await;
        *guard = None;
        self.pin_count.store(0, Ordering::SeqCst);
        self.is_dirty.store(false, Ordering::SeqCst);
    }
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Try to get page_id synchronously for debug
        let page_id = self
            .page
            .try_read()
            .ok()
            .and_then(|guard| guard.as_ref().map(|(page_id, _)| *page_id));

        f.debug_struct("Frame")
            .field("id", &self.id)
            .field("page_id", &page_id)
            .field("pin_count", &self.pin_count())
            .field("is_dirty", &self.is_dirty())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::pager::{Page as PagerPage, PageType};

    #[tokio::test]
    async fn test_frame_creation() {
        let frame = Frame::new(FrameId(0));
        assert_eq!(frame.id(), FrameId(0));
        assert!(frame.is_empty().await);
        assert!(!frame.is_pinned());
        assert!(!frame.is_dirty());
    }

    #[test]
    fn test_frame_pin_unpin() {
        let frame = Frame::new(FrameId(0));

        assert_eq!(frame.pin_count(), 0);

        frame.pin();
        assert_eq!(frame.pin_count(), 1);
        assert!(frame.is_pinned());

        frame.pin();
        assert_eq!(frame.pin_count(), 2);

        assert!(frame.unpin().is_ok());
        assert_eq!(frame.pin_count(), 1);

        assert!(frame.unpin().is_ok());
        assert_eq!(frame.pin_count(), 0);
        assert!(!frame.is_pinned());
    }

    #[test]
    fn test_frame_unpin_error() {
        let frame = Frame::new(FrameId(0));

        // Trying to unpin when not pinned should return error
        let result = frame.unpin();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Attempted to unpin a frame that was not pinned"
        );

        // Pin count should remain 0
        assert_eq!(frame.pin_count(), 0);
    }

    #[tokio::test]
    async fn test_frame_set_page() {
        let frame = Frame::new(FrameId(0));
        let page = PagerPage::new(PageId(42), PageType::Data);

        frame.set_page(PageId(42), page).await;

        assert!(!frame.is_empty().await);
        assert_eq!(frame.page_id().await, Some(PageId(42)));
    }

    #[test]
    fn test_frame_dirty_flag() {
        let frame = Frame::new(FrameId(0));

        assert!(!frame.is_dirty());

        frame.set_dirty(true);
        assert!(frame.is_dirty());

        frame.set_dirty(false);
        assert!(!frame.is_dirty());
    }

    #[tokio::test]
    async fn test_frame_clear() {
        let frame = Frame::new(FrameId(0));
        let page = PagerPage::new(PageId(42), PageType::Data);

        frame.set_page(PageId(42), page).await;
        frame.pin();
        frame.set_dirty(true);

        frame.clear().await;

        assert!(frame.is_empty().await);
        assert_eq!(frame.pin_count(), 0);
        assert!(!frame.is_dirty());
    }
}
