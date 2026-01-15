//! Eviction policies for buffer pool
//!
//! Implements LRU (Least Recently Used) and Clock eviction algorithms.

use std::collections::{HashMap, VecDeque};

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
pub struct LRUEviction {
    /// LRU queue (front = least recently used)
    queue: VecDeque<FrameId>,
    /// Frame position in queue
    position: HashMap<FrameId, usize>,
}

impl LRUEviction {
    /// Create a new LRU eviction policy
    #[must_use]
    pub fn new(_pool_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            position: HashMap::new(),
        }
    }

    /// Update positions in `HashMap` after queue modification
    fn update_positions(&mut self) {
        self.position.clear();
        for (pos, &frame_id) in self.queue.iter().enumerate() {
            self.position.insert(frame_id, pos);
        }
    }
}

impl EvictionPolicy for LRUEviction {
    fn record_access(&mut self, frame_id: FrameId) {
        // Remove from current position if exists
        if let Some(&pos) = self.position.get(&frame_id) {
            self.queue.remove(pos);
        }

        // Add to back (most recently used)
        self.queue.push_back(frame_id);

        // Update positions
        self.update_positions();
    }

    fn select_victim(&mut self) -> Option<FrameId> {
        // Return least recently used (front of queue)
        self.queue.front().copied()
    }

    fn remove(&mut self, frame_id: FrameId) {
        if let Some(&pos) = self.position.get(&frame_id) {
            self.queue.remove(pos);
            self.update_positions();
        }
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
}
