//! # Concurrency Model and Lock Hierarchy Documentation
//!
//! This module documents the concurrency model and lock hierarchy used throughout
//! NeuroQuantumDB. Following these guidelines is critical to prevent deadlocks
//! and ensure thread-safe operation.
//!
//! ## Lock Hierarchy Overview
//!
//! NeuroQuantumDB uses a strict lock hierarchy to prevent deadlocks. Locks must
//! always be acquired in order from **Level 1 (highest)** to **Level 6 (lowest)**.
//! Never acquire a higher-level lock while holding a lower-level lock.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                     NEUROQUANTUMDB LOCK HIERARCHY                           │
//! │                                                                             │
//! │  Level 1 (Top)     ┌─────────────────────────────────────────┐              │
//! │  Database          │  Arc<tokio::sync::RwLock<NeuroQuantumDB>>│             │
//! │                    └─────────────────────────────────────────┘              │
//! │                                      │                                      │
//! │                                      ▼                                      │
//! │  Level 2           ┌─────────────────────────────────────────┐              │
//! │  Query Engine      │  Arc<tokio::sync::Mutex<QSQLEngine>>    │              │
//! │                    └─────────────────────────────────────────┘              │
//! │                                      │                                      │
//! │                                      ▼                                      │
//! │  Level 3           ┌─────────────────────────────────────────┐              │
//! │  Storage           │  Arc<tokio::sync::RwLock<StorageEngine>>│              │
//! │                    └─────────────────────────────────────────┘              │
//! │                                      │                                      │
//! │                    ┌─────────────────┼─────────────────┐                    │
//! │                    ▼                 ▼                 ▼                    │
//! │  Level 4      ┌─────────┐     ┌───────────┐     ┌───────────┐               │
//! │  Subsystems   │   WAL   │     │   BTree   │     │ Buffer    │               │
//! │               │ Manager │     │  Indexes  │     │   Pool    │               │
//! │               └─────────┘     └───────────┘     └───────────┘               │
//! │                                      │                                      │
//! │                                      ▼                                      │
//! │  Level 5           ┌─────────────────────────────────────────┐              │
//! │  Monitoring        │  Arc<tokio::sync::RwLock<QueryMetrics>> │              │
//! │                    └─────────────────────────────────────────┘              │
//! │                                      │                                      │
//! │                                      ▼                                      │
//! │  Level 6 (Bottom)  ┌─────────────────────────────────────────┐              │
//! │  Utilities         │  std::sync::Mutex (CircuitBreaker, etc.)│              │
//! │                    └─────────────────────────────────────────┘              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Lock Types and Usage Guidelines
//!
//! ### tokio::sync::RwLock (Preferred for Async)
//!
//! Use `tokio::sync::RwLock` for all asynchronous code paths. This lock is
//! designed to work correctly with Tokio's async runtime and will not block
//! the executor thread.
//!
//! **When to use:**
//! - Any data accessed in async functions
//! - Shared state in API handlers
//! - Database and storage engine wrappers
//!
//! **Example:**
//! ```text
//! let db = Arc::new(tokio::sync::RwLock::new(NeuroQuantumDB::new()));
//!
//! // Read access (multiple readers allowed)
//! let guard = db.read().await;
//!
//! // Write access (exclusive)
//! let mut guard = db.write().await;
//! ```
//!
//! ### tokio::sync::Mutex (Exclusive Async Access)
//!
//! Use `tokio::sync::Mutex` when you need exclusive access and don't benefit
//! from read/write separation (e.g., when most operations are mutations).
//!
//! **When to use:**
//! - Query execution engines
//! - State machines with mostly write operations
//!
//! **Example:**
//! ```text
//! let engine = Arc::new(tokio::sync::Mutex::new(QSQLEngine::new()));
//! let mut guard = engine.lock().await;
//! ```
//!
//! ### std::sync::RwLock (Sync Code Only)
//!
//! Use `std::sync::RwLock` **only** in synchronous code that is never called
//! from an async context. Holding a `std::sync` lock across an `.await` point
//! will block the async executor and can cause deadlocks.
//!
//! **When to use:**
//! - Internal synchronous modules (e.g., `spiking.rs`, `synaptic.rs`)
//! - Configuration that is set once and read many times
//!
//! **Warning:**
//! ```text
//! // ❌ WRONG: Don't hold std::sync locks across .await
//! let guard = std_rwlock.read().unwrap();
//! some_async_function().await;  // DEADLOCK RISK!
//! drop(guard);
//!
//! // ✅ CORRECT: Drop before await
//! let data = {
//!     let guard = std_rwlock.read().unwrap();
//!     guard.clone()
//! };
//! some_async_function().await;
//! ```
//!
//! ### std::sync::Mutex (Short Critical Sections)
//!
//! Use `std::sync::Mutex` for very short critical sections in sync code,
//! such as updating counters or simple state machines.
//!
//! **When to use:**
//! - Circuit breaker state (see `middleware.rs`)
//! - Error correction statistics
//! - Atomic-like operations that need more complex logic
//!
//! ## Detailed Lock Inventory
//!
//! ### Level 1: Database Layer
//!
//! | Lock | Type | Location | Purpose |
//! |------|------|----------|---------|
//! | `NeuroQuantumDB` | `Arc<tokio::sync::RwLock<_>>` | `neuroquantum-api/src/lib.rs` | Main database access |
//!
//! ### Level 2: Query Engine Layer
//!
//! | Lock | Type | Location | Purpose |
//! |------|------|----------|---------|
//! | `QSQLEngine` | `Arc<tokio::sync::Mutex<_>>` | `neuroquantum-api/src/lib.rs` | Query execution |
//! | `QSQLEngine` | `Arc<tokio::sync::Mutex<_>>` | `websocket/handler.rs` | WebSocket query execution |
//!
//! ### Level 3: Storage Layer
//!
//! | Lock | Type | Location | Purpose |
//! |------|------|----------|---------|
//! | `StorageEngine` | `Arc<tokio::sync::RwLock<_>>` | `neuroquantum-qsql/src/lib.rs` | Storage access for QSQL |
//! | `StorageEngine` | `Arc<tokio::sync::RwLock<_>>` | `query_plan.rs` | Query executor storage |
//!
//! ### Level 4: Storage Subsystems
//!
//! | Lock | Type | Location | Purpose |
//! |------|------|----------|---------|
//! | `BufferPoolManager` | Internal | `storage/buffer/` | Page buffer management |
//! | `WALManager` | Internal | `storage/wal/` | Write-ahead logging |
//! | `BTree` | Internal | `storage/btree/` | Index operations |
//!
//! ### Level 5: Monitoring Layer
//!
//! | Lock | Type | Location | Purpose |
//! |------|------|----------|---------|
//! | `SlowQueryLog` | `Arc<tokio::sync::RwLock<_>>` | `monitoring/query_metrics.rs` | Slow query tracking |
//! | `IndexUsageStats` | `Arc<tokio::sync::RwLock<_>>` | `monitoring/query_metrics.rs` | Index statistics |
//! | `LockContentionTracker` | `Arc<tokio::sync::RwLock<_>>` | `monitoring/query_metrics.rs` | Lock contention stats |
//! | `QueryExecutionStats` | `Arc<tokio::sync::RwLock<_>>` | `monitoring/query_metrics.rs` | Execution statistics |
//!
//! ### Level 6: Utility Layer
//!
//! | Lock | Type | Location | Purpose |
//! |------|------|----------|---------|
//! | `CircuitBreakerState` | `Rc<std::sync::Mutex<_>>` | `middleware.rs` | Circuit breaker FSM |
//! | `ErrorCorrectionStats` | `Arc<std::sync::Mutex<_>>` | `dna/error_correction.rs` | Error correction stats |
//! | `SynapticNetwork` | `std::sync::RwLock<_>` | `synaptic.rs` | Synaptic weight updates |
//! | `SpikingNetwork` | `std::sync::RwLock<_>` | `spiking.rs` | Neural network state |
//!
//! ### WebSocket Subsystem (Independent Hierarchy)
//!
//! The WebSocket subsystem has its own lock hierarchy that is **independent**
//! of the main database hierarchy. WebSocket locks should not be held while
//! acquiring database locks, and vice versa.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │              WEBSOCKET LOCK HIERARCHY (INDEPENDENT)          │
//! │                                                               │
//! │  WS Level 1    ┌────────────────────────────────────┐         │
//! │  Connections   │  DashMap<ConnectionId, Connection> │         │
//! │                └────────────────────────────────────┘         │
//! │                                 │                             │
//! │                                 ▼                             │
//! │  WS Level 2    ┌────────────────────────────────────┐         │
//! │  PubSub        │  Arc<tokio::sync::RwLock<Channels>>│         │
//! │                └────────────────────────────────────┘         │
//! │                                 │                             │
//! │                                 ▼                             │
//! │  WS Level 3    ┌────────────────────────────────────┐         │
//! │  Flow Control  │  Arc<tokio::sync::RwLock<FlowCtrl>>│         │
//! │                └────────────────────────────────────┘         │
//! │                                 │                             │
//! │                                 ▼                             │
//! │  WS Level 4    ┌────────────────────────────────────┐         │
//! │  Shutdown      │  Arc<tokio::sync::RwLock<bool>>    │         │
//! │                └────────────────────────────────────┘         │
//! └───────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Deadlock Prevention Rules
//!
//! 1. **Always acquire locks in hierarchy order** (Level 1 → Level 6)
//!
//! 2. **Never hold locks across `.await` points** when using `std::sync` locks
//!
//! 3. **Prefer `tokio::sync` locks** in async code
//!
//! 4. **Keep critical sections short** - do expensive work outside the lock
//!
//! 5. **Use `try_lock()` with timeout** for non-critical operations
//!
//! 6. **Document lock acquisition** in complex functions
//!
//! ## Example: Correct Lock Acquisition Order
//!
//! ```text
//! async fn execute_query_with_storage(
//!     db: Arc<tokio::sync::RwLock<NeuroQuantumDB>>,
//!     engine: Arc<tokio::sync::Mutex<QSQLEngine>>,
//!     storage: Arc<tokio::sync::RwLock<StorageEngine>>,
//! ) -> Result<()> {
//!     // ✅ CORRECT: Acquire in hierarchy order (Level 1 → 2 → 3)
//!     
//!     // Level 1: Database (if needed)
//!     let db_guard = db.read().await;
//!     
//!     // Level 2: Query Engine
//!     let mut engine_guard = engine.lock().await;
//!     
//!     // Level 3: Storage
//!     let storage_guard = storage.read().await;
//!     
//!     // ... perform operations ...
//!     
//!     // Locks are dropped in reverse order automatically
//!     Ok(())
//! }
//! ```
//!
//! ## Example: Incorrect Lock Acquisition (DON'T DO THIS)
//!
//! ```text
//! async fn bad_lock_order(
//!     storage: Arc<tokio::sync::RwLock<StorageEngine>>,
//!     db: Arc<tokio::sync::RwLock<NeuroQuantumDB>>,
//! ) -> Result<()> {
//!     // ❌ WRONG: Acquiring Level 3 before Level 1
//!     let storage_guard = storage.write().await;
//!     let db_guard = db.read().await;  // POTENTIAL DEADLOCK!
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Performance Considerations
//!
//! - **Use `read()` when possible**: Multiple readers can proceed concurrently
//! - **Batch writes**: Combine multiple writes under a single write lock
//! - **Clone data out**: If you need data for a long operation, clone it and release the lock
//! - **Consider DashMap**: For high-contention maps, use `DashMap` (sharded concurrent HashMap)
//!
//! ## Testing for Deadlocks
//!
//! Run stress tests with thread sanitizer enabled:
//!
//! ```bash
//! RUSTFLAGS="-Z sanitizer=thread" cargo test --release
//! ```
//!
//! Or use the built-in stress tests:
//!
//! ```bash
//! cargo test --package neuroquantum-core stress_test -- --ignored
//! ```

use std::fmt;

/// Lock level in the hierarchy (lower number = higher priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LockLevel {
    /// Level 1: Database layer (`NeuroQuantumDB`)
    Database = 1,
    /// Level 2: Query engine layer (`QSQLEngine`)
    QueryEngine = 2,
    /// Level 3: Storage layer (`StorageEngine`)
    Storage = 3,
    /// Level 4: Storage subsystems (WAL, `BTree`, `BufferPool`)
    StorageSubsystem = 4,
    /// Level 5: Monitoring layer (`QueryMetrics`)
    Monitoring = 5,
    /// Level 6: Utility layer (`CircuitBreaker`, stats)
    Utility = 6,
}

impl fmt::Display for LockLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            | Self::Database => write!(f, "Level 1 (Database)"),
            | Self::QueryEngine => write!(f, "Level 2 (QueryEngine)"),
            | Self::Storage => write!(f, "Level 3 (Storage)"),
            | Self::StorageSubsystem => write!(f, "Level 4 (StorageSubsystem)"),
            | Self::Monitoring => write!(f, "Level 5 (Monitoring)"),
            | Self::Utility => write!(f, "Level 6 (Utility)"),
        }
    }
}

/// WebSocket-specific lock levels (independent hierarchy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum WebSocketLockLevel {
    /// WS Level 1: Connection management
    Connections = 1,
    /// WS Level 2: Pub/Sub channels
    PubSub = 2,
    /// WS Level 3: Flow control
    FlowControl = 3,
    /// WS Level 4: Shutdown signal
    Shutdown = 4,
}

impl fmt::Display for WebSocketLockLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            | Self::Connections => write!(f, "WS Level 1 (Connections)"),
            | Self::PubSub => write!(f, "WS Level 2 (PubSub)"),
            | Self::FlowControl => write!(f, "WS Level 3 (FlowControl)"),
            | Self::Shutdown => write!(f, "WS Level 4 (Shutdown)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_level_ordering() {
        // Verify that lock levels are properly ordered
        assert!(LockLevel::Database < LockLevel::QueryEngine);
        assert!(LockLevel::QueryEngine < LockLevel::Storage);
        assert!(LockLevel::Storage < LockLevel::StorageSubsystem);
        assert!(LockLevel::StorageSubsystem < LockLevel::Monitoring);
        assert!(LockLevel::Monitoring < LockLevel::Utility);
    }

    #[test]
    fn test_websocket_lock_level_ordering() {
        // Verify WebSocket lock levels are properly ordered
        assert!(WebSocketLockLevel::Connections < WebSocketLockLevel::PubSub);
        assert!(WebSocketLockLevel::PubSub < WebSocketLockLevel::FlowControl);
        assert!(WebSocketLockLevel::FlowControl < WebSocketLockLevel::Shutdown);
    }

    #[test]
    fn test_lock_level_display() {
        assert_eq!(format!("{}", LockLevel::Database), "Level 1 (Database)");
        assert_eq!(
            format!("{}", LockLevel::QueryEngine),
            "Level 2 (QueryEngine)"
        );
        assert_eq!(format!("{}", LockLevel::Storage), "Level 3 (Storage)");
    }
}
