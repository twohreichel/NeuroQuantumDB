//! Free page list management
//! 
//! Tracks which pages are free and can be reused

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use super::page::PageId;

/// Free list for tracking available pages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeList {
    /// Queue of free page IDs (FIFO for better locality)
    free_pages: VecDeque<PageId>,
    /// Total number of free pages
    count: usize,
}

impl FreeList {
    /// Create a new empty free list
    pub fn new() -> Self {
        Self {
            free_pages: VecDeque::new(),
            count: 0,
        }
    }

    /// Add a page to the free list
    pub fn add_free_page(&mut self, page_id: PageId) {
        self.free_pages.push_back(page_id);
        self.count += 1;
    }

    /// Pop a free page from the list
    pub fn pop_free_page(&mut self) -> Option<PageId> {
        if let Some(page_id) = self.free_pages.pop_front() {
            self.count -= 1;
            Some(page_id)
        } else {
            None
        }
    }

    /// Get the number of free pages
    pub fn free_count(&self) -> usize {
        self.count
    }

    /// Check if the free list is empty
    pub fn is_empty(&self) -> bool {
        self.free_pages.is_empty()
    }

    /// Serialize free list to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| anyhow!("Failed to serialize free list: {}", e))
    }

    /// Deserialize free list from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data)
            .map_err(|e| anyhow!("Failed to deserialize free list: {}", e))
    }

    /// Get all free page IDs (for debugging)
    pub fn get_free_pages(&self) -> Vec<PageId> {
        self.free_pages.iter().copied().collect()
    }

    /// Clear all free pages
    pub fn clear(&mut self) {
        self.free_pages.clear();
        self.count = 0;
    }

    /// Reserve capacity for expected free pages
    pub fn reserve(&mut self, additional: usize) {
        self.free_pages.reserve(additional);
    }
}

impl Default for FreeList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_list_basic() {
        let mut free_list = FreeList::new();
        assert!(free_list.is_empty());
        assert_eq!(free_list.free_count(), 0);

        free_list.add_free_page(PageId(1));
        free_list.add_free_page(PageId(2));
        free_list.add_free_page(PageId(3));

        assert_eq!(free_list.free_count(), 3);
        assert!(!free_list.is_empty());
    }

    #[test]
    fn test_free_list_pop() {
        let mut free_list = FreeList::new();
        
        free_list.add_free_page(PageId(5));
        free_list.add_free_page(PageId(10));
        free_list.add_free_page(PageId(15));

        // FIFO order
        assert_eq!(free_list.pop_free_page(), Some(PageId(5)));
        assert_eq!(free_list.pop_free_page(), Some(PageId(10)));
        assert_eq!(free_list.pop_free_page(), Some(PageId(15)));
        assert_eq!(free_list.pop_free_page(), None);
    }

    #[test]
    fn test_free_list_serialization() {
        let mut free_list = FreeList::new();
        free_list.add_free_page(PageId(1));
        free_list.add_free_page(PageId(2));
        free_list.add_free_page(PageId(3));

        let serialized = free_list.serialize().unwrap();
        let deserialized = FreeList::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.free_count(), 3);
        assert_eq!(deserialized.get_free_pages(), vec![PageId(1), PageId(2), PageId(3)]);
    }

    #[test]
    fn test_free_list_clear() {
        let mut free_list = FreeList::new();
        free_list.add_free_page(PageId(1));
        free_list.add_free_page(PageId(2));

        assert_eq!(free_list.free_count(), 2);

        free_list.clear();
        assert_eq!(free_list.free_count(), 0);
        assert!(free_list.is_empty());
    }

    #[test]
    fn test_free_list_large() {
        let mut free_list = FreeList::new();
        
        // Add 1000 pages
        for i in 0..1000 {
            free_list.add_free_page(PageId(i));
        }

        assert_eq!(free_list.free_count(), 1000);

        // Pop all pages
        for i in 0..1000 {
            assert_eq!(free_list.pop_free_page(), Some(PageId(i)));
        }

        assert!(free_list.is_empty());
    }
}

