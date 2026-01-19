//! Migration progress tracking
//!
//! Tracks progress of long-running migrations.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::storage::migration::MigrationId;

/// Progress information for a migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationProgress {
    /// Migration identifier
    pub migration_id: MigrationId,
    /// Total items to process
    pub total_items: u64,
    /// Items processed so far
    pub processed_items: u64,
    /// Percentage complete (0-100)
    pub percentage: f64,
    /// Estimated time remaining in seconds
    pub estimated_remaining_secs: Option<u64>,
    /// Current operation description
    pub current_operation: String,
    /// Can the migration be paused?
    pub pausable: bool,
    /// Is the migration currently paused?
    pub paused: bool,
}

impl MigrationProgress {
    /// Create new progress tracker for a migration
    #[must_use]
    pub fn new(migration_id: MigrationId, total_items: u64) -> Self {
        Self {
            migration_id,
            total_items,
            processed_items: 0,
            percentage: 0.0,
            estimated_remaining_secs: None,
            current_operation: String::from("Starting..."),
            pausable: true,
            paused: false,
        }
    }

    /// Update progress
    pub fn update(&mut self, processed: u64, operation: String) {
        self.processed_items = processed;
        self.percentage = if self.total_items > 0 {
            (processed as f64 / self.total_items as f64) * 100.0
        } else {
            0.0
        };
        self.current_operation = operation;
    }
}

/// Progress tracker for migrations
pub struct ProgressTracker {
    /// Current progress state
    progress: Arc<RwLock<Option<MigrationProgress>>>,
    /// Pause flag
    pause_flag: Arc<AtomicBool>,
    /// Cancel flag
    cancel_flag: Arc<AtomicBool>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            progress: Arc::new(RwLock::new(None)),
            pause_flag: Arc::new(AtomicBool::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start tracking a new migration
    pub async fn start(&self, migration_id: MigrationId, total_items: u64) {
        let mut progress = self.progress.write().await;
        *progress = Some(MigrationProgress::new(migration_id, total_items));
        self.pause_flag.store(false, Ordering::SeqCst);
        self.cancel_flag.store(false, Ordering::SeqCst);
    }

    /// Update progress
    pub async fn update(&self, processed: u64, operation: String) {
        let mut progress = self.progress.write().await;
        if let Some(ref mut p) = *progress {
            p.update(processed, operation);
        }
    }

    /// Get current progress
    pub async fn get_progress(&self) -> Option<MigrationProgress> {
        let progress = self.progress.read().await;
        progress.clone()
    }

    /// Pause the migration
    pub fn pause(&self) {
        self.pause_flag.store(true, Ordering::SeqCst);
    }

    /// Resume the migration
    pub fn resume(&self) {
        self.pause_flag.store(false, Ordering::SeqCst);
    }

    /// Cancel the migration
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Check if paused
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.pause_flag.load(Ordering::SeqCst)
    }

    /// Check if cancelled
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    /// Wait while paused
    pub async fn wait_if_paused(&self) {
        while self.is_paused() && !self.is_cancelled() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Complete the migration
    pub async fn complete(&self) {
        let mut progress = self.progress.write().await;
        if let Some(ref mut p) = *progress {
            p.processed_items = p.total_items;
            p.percentage = 100.0;
            p.current_operation = "Completed".to_string();
        }
    }

    /// Clear progress tracking
    pub async fn clear(&self) {
        let mut progress = self.progress.write().await;
        *progress = None;
        self.pause_flag.store(false, Ordering::SeqCst);
        self.cancel_flag.store(false, Ordering::SeqCst);
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_tracking() {
        let tracker = ProgressTracker::new();

        // Start tracking
        tracker.start("001".to_string(), 100).await;

        // Update progress
        tracker.update(50, "Processing...".to_string()).await;

        let progress = tracker.get_progress().await.unwrap();
        assert_eq!(progress.processed_items, 50);
        assert_eq!(progress.percentage, 50.0);

        // Complete
        tracker.complete().await;
        let progress = tracker.get_progress().await.unwrap();
        assert_eq!(progress.percentage, 100.0);
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let tracker = ProgressTracker::new();

        assert!(!tracker.is_paused());

        tracker.pause();
        assert!(tracker.is_paused());

        tracker.resume();
        assert!(!tracker.is_paused());
    }

    #[tokio::test]
    async fn test_cancel() {
        let tracker = ProgressTracker::new();

        assert!(!tracker.is_cancelled());

        tracker.cancel();
        assert!(tracker.is_cancelled());
    }
}
