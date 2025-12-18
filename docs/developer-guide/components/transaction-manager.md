# Transaction Manager

## ACID Properties

| Property | Implementation |
|----------|----------------|
| **Atomicity** | WAL rollback |
| **Consistency** | Constraint validation |
| **Isolation** | MVCC + 2PL |
| **Durability** | WAL + fsync |

## Isolation Levels

```rust
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}
```

## Transaction Lifecycle

```
BEGIN → [Operations] → COMMIT
           ↓
        [Error]
           ↓
        ROLLBACK
```

## Lock Manager

Two-Phase Locking (2PL):

```rust
pub enum LockType {
    Shared,      // Read lock
    Exclusive,   // Write lock
}

pub struct LockManager {
    pub fn acquire(&self, txn_id: TxnId, resource: ResourceId, lock_type: LockType);
    pub fn release(&self, txn_id: TxnId, resource: ResourceId);
    pub fn release_all(&self, txn_id: TxnId);
}
```

## Usage

```rust
let txn_mgr = TransactionManager::new(log_mgr, lock_mgr);

// Start transaction
let txn = txn_mgr.begin(IsolationLevel::Serializable)?;

// Operations
storage.put(txn.id(), key, value)?;

// Commit or rollback
txn_mgr.commit(txn.id())?;
// or: txn_mgr.rollback(txn.id())?;
```

## Recovery

On startup:

1. **Analysis** — Scan WAL for active transactions
2. **Redo** — Replay committed changes
3. **Undo** — Rollback uncommitted transactions
