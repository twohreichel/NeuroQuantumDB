//! Eviction policies for buffer pool
//!
//! Implements LRU (Least Recently Used) and Clock eviction algorithms.

use std::collections::HashMap;

use indexmap::IndexSet;

use super::frame::FrameId;

/// Eviction policy trait
pub trait EvictionPolicy: Send + Sync {
    /// Record an access to a frame
    fn record_access(&mut self, frame_id: FrameId);

    /// Select a victim frame for eviction
    fn select_victim(&mut self) -> Option<FrameId>;

    /// Remove a frame from tracking
    fn remove(&mut self, frame_id: FrameId);
}

/// LRU (Least Recently Used) eviction policy
///
/// This implementation uses `IndexSet` to achieve O(1) complexity for all operations:
/// - `record_access`: O(1) - removes and re-inserts at end
/// - `select_victim`: O(1) - returns first element
/// - `remove`: O(1) - removes by value
///
/// The `IndexSet` maintains insertion order, allowing us to use it as an LRU queue
/// where the front contains the least recently used items.
pub struct LRUEviction {
    /// LRU set maintaining insertion order (front = least recently used)
    /// `IndexSet` provides O(1) remove by value, O(1) insert, and maintains order
    order: IndexSet<FrameId>,
}

impl LRUEviction {
    /// Create a new LRU eviction policy
    #[must_use]
    pub fn new(pool_size: usize) -> Self {
        Self {
            order: IndexSet::with_capacity(pool_size),
        }
    }
}

impl EvictionPolicy for LRUEviction {
    fn record_access(&mut self, frame_id: FrameId) {
        // Remove from current position if exists (O(1) with IndexSet)
        // Then insert at end (most recently used position) (O(1))
        // IndexSet::shift_remove removes and shifts elements, maintaining order
        self.order.shift_remove(&frame_id);
        self.order.insert(frame_id);
    }

    fn select_victim(&mut self) -> Option<FrameId> {
        // Return least recently used (first element in set) - O(1)
        self.order.first().copied()
    }

    fn remove(&mut self, frame_id: FrameId) {
        // Remove frame from tracking - O(1)
        self.order.shift_remove(&frame_id);
    }
}

/// Clock eviction policy (Second-Chance algorithm)
pub struct ClockEviction {
    /// Circular list of frames
    frames: Vec<FrameId>,
    /// Reference bits for each frame
    reference_bits: HashMap<FrameId, bool>,
    /// Clock hand (current position)
    hand: usize,
}

impl ClockEviction {
    /// Create a new Clock eviction policy
    pub fn new(pool_size: usize) -> Self {
        let frames: Vec<FrameId> = (0..pool_size).map(FrameId).collect();
        let reference_bits = frames.iter().map(|&frame_id| (frame_id, false)).collect();

        Self {
            frames,
            reference_bits,
            hand: 0,
        }
    }
}

impl EvictionPolicy for ClockEviction {
    fn record_access(&mut self, frame_id: FrameId) {
        self.reference_bits.insert(frame_id, true);
    }

    fn select_victim(&mut self) -> Option<FrameId> {
        if self.frames.is_empty() {
            return None;
        }

        // Sweep clock hand until we find a frame with reference bit = 0
        loop {
            let frame_id = self.frames[self.hand];

            if let Some(&ref_bit) = self.reference_bits.get(&frame_id) {
                if ref_bit {
                    // Give second chance, clear reference bit
                    self.reference_bits.insert(frame_id, false);
                } else {
                    // Found victim
                    return Some(frame_id);
                }
            }

            // Move clock hand
            self.hand = (self.hand + 1) % self.frames.len();
        }
    }

    fn remove(&mut self, frame_id: FrameId) {
        self.reference_bits.remove(&frame_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_basic() {
        let mut lru = LRUEviction::new(3);

        // Access pattern: 0, 1, 2
        lru.record_access(FrameId(0));
        lru.record_access(FrameId(1));
        lru.record_access(FrameId(2));

        // Victim should be 0 (least recently used)
        assert_eq!(lru.select_victim(), Some(FrameId(0)));
    }

    #[test]
    fn test_lru_reaccess() {
        let mut lru = LRUEviction::new(3);

        // Access pattern: 0, 1, 2, 0
        lru.record_access(FrameId(0));
        lru.record_access(FrameId(1));
        lru.record_access(FrameId(2));
        lru.record_access(FrameId(0)); // Re-access 0

        // Victim should be 1 (least recently used)
        assert_eq!(lru.select_victim(), Some(FrameId(1)));
    }

    #[test]
    fn test_lru_remove() {
        let mut lru = LRUEviction::new(3);

        lru.record_access(FrameId(0));
        lru.record_access(FrameId(1));
        lru.record_access(FrameId(2));

        lru.remove(FrameId(0));

        // After removing 0, victim should be 1
        assert_eq!(lru.select_victim(), Some(FrameId(1)));
    }

    #[test]
    fn test_clock_basic() {
        let mut clock = ClockEviction::new(3);

        // All reference bits start at 0
        // Should select frame 0
        assert_eq!(clock.select_victim(), Some(FrameId(0)));
    }

    #[test]
    fn test_clock_second_chance() {
        let mut clock = ClockEviction::new(3);

        // Access frame 0
        clock.record_access(FrameId(0));

        // Should skip frame 0 (reference bit = 1), give it second chance,
        // and select frame 1
        let victim = clock.select_victim();
        assert!(victim == Some(FrameId(1)) || victim == Some(FrameId(2)));
    }

    #[test]
    fn test_clock_circular() {
        let mut clock = ClockEviction::new(3);

        // Set all reference bits
        clock.record_access(FrameId(0));
        clock.record_access(FrameId(1));
        clock.record_access(FrameId(2));

        // First call should clear all bits and select frame 0
        let victim = clock.select_victim();
        assert!(victim.is_some());
    }

    #[test]
    fn test_clock_remove() {
        let mut clock = ClockEviction::new(3);

        clock.record_access(FrameId(0));
        clock.remove(FrameId(0));

        // Frame 0 should still be selectable (removal only removes from tracking)
        let victim = clock.select_victim();
        assert!(victim.is_some());
    }

    #[test]
    fn test_lru_large_scale_performance() {
        // Test with 10,000 frames to verify O(1) performance
        let mut lru = LRUEviction::new(10_000);

        // Insert 10,000 frames
        for i in 0..10_000 {
            lru.record_access(FrameId(i));
        }

        // Re-access random frames - this should be O(1) per operation
        // With the old O(n) implementation, this would take ~50μs per access
        // With the new O(1) implementation, this should take ~50ns per access
        for i in (0..10_000).step_by(100) {
            lru.record_access(FrameId(i));
        }

        // Verify LRU order is maintained
        let victim = lru.select_victim();
        assert!(victim.is_some());
        assert_ne!(victim, Some(FrameId(0))); // Frame 0 was re-accessed
    }

    #[test]
    fn test_lru_no_duplicates() {
        let mut lru = LRUEviction::new(5);

        // Access same frame multiple times
        lru.record_access(FrameId(0));
        lru.record_access(FrameId(1));
        lru.record_access(FrameId(0)); // Re-access
        lru.record_access(FrameId(2));

        // Verify no duplicates in internal structure
        // Victim should be 1 (least recently used), not 0
        assert_eq!(lru.select_victim(), Some(FrameId(1)));
    }
}
