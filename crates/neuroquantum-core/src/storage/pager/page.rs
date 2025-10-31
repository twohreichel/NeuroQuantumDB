//! Page structure and management
//!
//! Defines the page format:
//! - Page Header (64 bytes): metadata
//! - Page Data (4032 bytes): actual data
//! - Total: 4096 bytes (4KB)

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Page size in bytes (4KB standard)
pub const PAGE_SIZE: usize = 4096;

/// Page header size in bytes
pub const PAGE_HEADER_SIZE: usize = 64;

/// Available data size per page
pub const PAGE_DATA_SIZE: usize = PAGE_SIZE - PAGE_HEADER_SIZE;

/// Page identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageId(pub u64);

impl fmt::Display for PageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page({})", self.0)
    }
}

/// Page type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PageType {
    /// Free page list metadata
    FreePage = 0,
    /// Data page
    Data = 1,
    /// B+ Tree internal node
    BTreeInternal = 2,
    /// B+ Tree leaf node
    BTreeLeaf = 3,
    /// Overflow page for large data
    Overflow = 4,
    /// Write-Ahead Log page
    WAL = 5,
}

impl TryFrom<u8> for PageType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(PageType::FreePage),
            1 => Ok(PageType::Data),
            2 => Ok(PageType::BTreeInternal),
            3 => Ok(PageType::BTreeLeaf),
            4 => Ok(PageType::Overflow),
            5 => Ok(PageType::WAL),
            _ => Err(anyhow!("Invalid page type: {}", value)),
        }
    }
}

/// Page header (64 bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageHeader {
    /// Magic number for validation (0xDEADBEEF)
    pub magic: u32,
    /// Page type
    pub page_type: PageType,
    /// Page ID
    pub page_id: PageId,
    /// LSN (Log Sequence Number)
    pub lsn: u64,
    /// Checksum for data integrity
    pub checksum: u32,
    /// Free space in this page
    pub free_space: u16,
    /// Number of slots/records in this page
    pub slot_count: u16,
    /// Next page ID (for linked pages)
    pub next_page: Option<PageId>,
    /// Previous page ID (for doubly-linked pages)
    pub prev_page: Option<PageId>,
    /// Reserved for future use
    reserved: [u8; 16],
}

const MAGIC_NUMBER: u32 = 0xDEADBEEF;

impl PageHeader {
    /// Create a new page header
    pub fn new(page_id: PageId, page_type: PageType) -> Self {
        Self {
            magic: MAGIC_NUMBER,
            page_type,
            page_id,
            lsn: 0,
            checksum: 0,
            free_space: PAGE_DATA_SIZE as u16,
            slot_count: 0,
            next_page: None,
            prev_page: None,
            reserved: [0; 16],
        }
    }

    /// Validate magic number
    pub fn is_valid(&self) -> bool {
        self.magic == MAGIC_NUMBER
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Result<[u8; PAGE_HEADER_SIZE]> {
        let mut buf = [0u8; PAGE_HEADER_SIZE];

        // Magic
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());

        // Page type
        buf[4] = self.page_type as u8;

        // Page ID
        buf[8..16].copy_from_slice(&self.page_id.0.to_le_bytes());

        // LSN
        buf[16..24].copy_from_slice(&self.lsn.to_le_bytes());

        // Checksum
        buf[24..28].copy_from_slice(&self.checksum.to_le_bytes());

        // Free space
        buf[28..30].copy_from_slice(&self.free_space.to_le_bytes());

        // Slot count
        buf[30..32].copy_from_slice(&self.slot_count.to_le_bytes());

        // Next page (use u64::MAX as sentinel for None)
        let next = self.next_page.map_or(u64::MAX, |p| p.0);
        buf[32..40].copy_from_slice(&next.to_le_bytes());

        // Prev page (use u64::MAX as sentinel for None)
        let prev = self.prev_page.map_or(u64::MAX, |p| p.0);
        buf[40..48].copy_from_slice(&prev.to_le_bytes());

        // Reserved
        buf[48..64].copy_from_slice(&self.reserved);

        Ok(buf)
    }

    /// Deserialize header from bytes
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < PAGE_HEADER_SIZE {
            return Err(anyhow!("Buffer too small for page header"));
        }

        let magic = u32::from_le_bytes(buf[0..4].try_into()?);
        if magic != MAGIC_NUMBER {
            return Err(anyhow!("Invalid magic number: 0x{:X}", magic));
        }

        let page_type = PageType::try_from(buf[4])?;
        let page_id = PageId(u64::from_le_bytes(buf[8..16].try_into()?));
        let lsn = u64::from_le_bytes(buf[16..24].try_into()?);
        let checksum = u32::from_le_bytes(buf[24..28].try_into()?);
        let free_space = u16::from_le_bytes(buf[28..30].try_into()?);
        let slot_count = u16::from_le_bytes(buf[30..32].try_into()?);

        let next_page_raw = u64::from_le_bytes(buf[32..40].try_into()?);
        let next_page = if next_page_raw == u64::MAX {
            None
        } else {
            Some(PageId(next_page_raw))
        };

        let prev_page_raw = u64::from_le_bytes(buf[40..48].try_into()?);
        let prev_page = if prev_page_raw == u64::MAX {
            None
        } else {
            Some(PageId(prev_page_raw))
        };

        let mut reserved = [0u8; 16];
        reserved.copy_from_slice(&buf[48..64]);

        Ok(Self {
            magic,
            page_type,
            page_id,
            lsn,
            checksum,
            free_space,
            slot_count,
            next_page,
            prev_page,
            reserved,
        })
    }
}

/// Page structure (4KB)
#[derive(Clone)]
pub struct Page {
    /// Page header
    header: PageHeader,
    /// Page data
    data: [u8; PAGE_DATA_SIZE],
}

impl Page {
    /// Create a new empty page
    pub fn new(page_id: PageId, page_type: PageType) -> Self {
        Self {
            header: PageHeader::new(page_id, page_type),
            data: [0; PAGE_DATA_SIZE],
        }
    }

    /// Get page ID
    pub fn id(&self) -> PageId {
        self.header.page_id
    }

    /// Get page header
    pub fn header(&self) -> &PageHeader {
        &self.header
    }

    /// Get mutable page header
    pub fn header_mut(&mut self) -> &mut PageHeader {
        &mut self.header
    }

    /// Get page data
    pub fn data(&self) -> &[u8; PAGE_DATA_SIZE] {
        &self.data
    }

    /// Get mutable page data
    pub fn data_mut(&mut self) -> &mut [u8; PAGE_DATA_SIZE] {
        &mut self.data
    }

    /// Write data to page at offset
    pub fn write_data(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > PAGE_DATA_SIZE {
            return Err(anyhow!(
                "Data too large: offset={}, size={}, available={}",
                offset,
                data.len(),
                PAGE_DATA_SIZE - offset
            ));
        }

        self.data[offset..offset + data.len()].copy_from_slice(data);

        // Update free space
        let used = offset + data.len();
        self.header.free_space = (PAGE_DATA_SIZE - used) as u16;

        Ok(())
    }

    /// Read data from page at offset
    pub fn read_data(&self, offset: usize, len: usize) -> Result<&[u8]> {
        if offset + len > PAGE_DATA_SIZE {
            return Err(anyhow!(
                "Read beyond page boundary: offset={}, len={}, page_size={}",
                offset,
                len,
                PAGE_DATA_SIZE
            ));
        }

        Ok(&self.data[offset..offset + len])
    }

    /// Calculate checksum for the page
    pub fn calculate_checksum(&self) -> u32 {
        // Simple CRC32 checksum
        crc32fast::hash(&self.data)
    }

    /// Update checksum in header
    pub fn update_checksum(&mut self) {
        self.header.checksum = self.calculate_checksum();
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {
        self.header.checksum == self.calculate_checksum()
    }

    /// Serialize page to bytes
    pub fn to_bytes(&self) -> Result<[u8; PAGE_SIZE]> {
        let mut buf = [0u8; PAGE_SIZE];

        // Write header
        let header_bytes = self.header.to_bytes()?;
        buf[0..PAGE_HEADER_SIZE].copy_from_slice(&header_bytes);

        // Write data
        buf[PAGE_HEADER_SIZE..].copy_from_slice(&self.data);

        Ok(buf)
    }

    /// Deserialize page from bytes
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() != PAGE_SIZE {
            return Err(anyhow!(
                "Invalid page size: expected {}, got {}",
                PAGE_SIZE,
                buf.len()
            ));
        }

        let header = PageHeader::from_bytes(&buf[0..PAGE_HEADER_SIZE])?;

        let mut data = [0u8; PAGE_DATA_SIZE];
        data.copy_from_slice(&buf[PAGE_HEADER_SIZE..]);

        Ok(Self { header, data })
    }

    /// Get available free space
    pub fn free_space(&self) -> u16 {
        self.header.free_space
    }

    /// Get slot count
    pub fn slot_count(&self) -> u16 {
        self.header.slot_count
    }

    /// Set LSN
    pub fn set_lsn(&mut self, lsn: u64) {
        self.header.lsn = lsn;
    }

    /// Serialize page to `Vec<u8>` (for backup)
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let bytes = self.to_bytes()?;
        Ok(bytes.to_vec())
    }
}

impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Page")
            .field("id", &self.header.page_id)
            .field("type", &self.header.page_type)
            .field("lsn", &self.header.lsn)
            .field("free_space", &self.header.free_space)
            .field("slot_count", &self.header.slot_count)
            .field("checksum", &self.header.checksum)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_creation() {
        let page = Page::new(PageId(0), PageType::Data);
        assert_eq!(page.id(), PageId(0));
        assert_eq!(page.header().page_type, PageType::Data);
        assert_eq!(page.free_space(), PAGE_DATA_SIZE as u16);
    }

    #[test]
    fn test_page_write_read() {
        let mut page = Page::new(PageId(0), PageType::Data);
        let data = b"Hello, World!";

        page.write_data(0, data).unwrap();

        let read = page.read_data(0, data.len()).unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn test_page_serialization() {
        let mut page = Page::new(PageId(42), PageType::Data);
        page.write_data(0, b"Test data").unwrap();
        page.update_checksum();

        let bytes = page.to_bytes().unwrap();
        let deserialized = Page::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.id(), PageId(42));
        assert_eq!(deserialized.header().page_type, PageType::Data);
        assert_eq!(deserialized.read_data(0, 9).unwrap(), b"Test data");
    }

    #[test]
    fn test_checksum() {
        let mut page = Page::new(PageId(0), PageType::Data);
        page.write_data(0, b"Test").unwrap();

        page.update_checksum();
        assert!(page.verify_checksum());

        // Modify data without updating checksum
        page.data_mut()[0] = 0xFF;
        assert!(!page.verify_checksum());
    }

    #[test]
    fn test_page_header_serialization() {
        let header = PageHeader::new(PageId(100), PageType::BTreeLeaf);

        let bytes = header.to_bytes().unwrap();
        let deserialized = PageHeader::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.page_id, PageId(100));
        assert_eq!(deserialized.page_type, PageType::BTreeLeaf);
        assert!(deserialized.is_valid());
    }

    #[test]
    fn test_write_beyond_page_boundary() {
        let mut page = Page::new(PageId(0), PageType::Data);
        let large_data = vec![0u8; PAGE_DATA_SIZE + 1];

        assert!(page.write_data(0, &large_data).is_err());
    }

    #[test]
    fn test_linked_pages() {
        let mut header = PageHeader::new(PageId(1), PageType::Data);
        header.next_page = Some(PageId(2));
        header.prev_page = Some(PageId(0));

        let bytes = header.to_bytes().unwrap();
        let deserialized = PageHeader::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.next_page, Some(PageId(2)));
        assert_eq!(deserialized.prev_page, Some(PageId(0)));
    }
}
