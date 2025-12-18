# Storage Engine

## Overview

The storage engine provides persistent, crash-safe data storage.

## Components

```
Storage Engine
├── Buffer Pool      # In-memory page cache
├── B+Tree Index     # Ordered key-value storage
├── WAL              # Write-ahead logging
├── Pager            # Page management
└── Backup           # S3/Local backup
```

## Buffer Pool

LRU cache for database pages:

```rust
pub struct BufferPool {
    frames: Vec<Frame>,
    page_table: HashMap<PageId, FrameId>,
    replacer: LRUReplacer,
}

impl BufferPool {
    pub async fn fetch_page(&self, page_id: PageId) -> Result<Arc<RwLock<Page>>>;
    pub async fn flush_page(&self, page_id: PageId) -> Result<()>;
    pub async fn new_page(&self) -> Result<PageId>;
}
```

## B+Tree Index

Persistent ordered index:

```rust
pub struct BPlusTree {
    root: PageId,
    order: usize,
}

impl BPlusTree {
    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<()>;
    pub fn search(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    pub fn range(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}
```

## WAL (Write-Ahead Logging)

ARIES-style recovery:

| Record Type | Purpose |
|-------------|---------|
| `BEGIN` | Transaction start |
| `UPDATE` | Data modification |
| `COMMIT` | Transaction commit |
| `ABORT` | Transaction rollback |
| `CHECKPOINT` | Recovery point |

## Configuration

```toml
[storage]
buffer_pool_size_mb = 256
page_size = 4096
wal_enabled = true
wal_sync_mode = "fsync"  # fsync, async, none
```
