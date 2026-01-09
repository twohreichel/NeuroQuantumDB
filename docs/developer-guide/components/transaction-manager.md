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

## Savepoints

Savepoints allow you to create named checkpoints within a transaction, enabling partial rollback without aborting the entire transaction.

### Savepoint Operations

| Statement | Description |
|-----------|-------------|
| `SAVEPOINT name` | Creates a named savepoint at the current position |
| `ROLLBACK TO SAVEPOINT name` | Rolls back all operations since the savepoint |
| `RELEASE SAVEPOINT name` | Removes the savepoint (commits intermediate changes) |

### Savepoint Usage Example

```sql
BEGIN;
INSERT INTO users (id, name) VALUES (1, 'Alice');
SAVEPOINT sp1;
INSERT INTO users (id, name) VALUES (2, 'Bob');
-- Error occurs, rollback to savepoint
ROLLBACK TO SAVEPOINT sp1;
INSERT INTO users (id, name) VALUES (2, 'Charlie');
COMMIT;
-- Result: Alice and Charlie are committed, Bob is not
```

### Nested Savepoints

Multiple savepoints can be created within a single transaction:

```sql
BEGIN;
SAVEPOINT sp1;
-- Operations A
SAVEPOINT sp2;
-- Operations B
ROLLBACK TO SAVEPOINT sp2;  -- Undoes only Operations B
ROLLBACK TO SAVEPOINT sp1;  -- Undoes everything since sp1
COMMIT;
```

### Savepoint Implementation Details

Savepoints are implemented using WAL (Write-Ahead Log) LSN tracking:

```rust
pub struct SavepointInfo {
    transaction_id: TransactionId,
    lsn: LSN,  // Log Sequence Number at savepoint creation
}
```

When rolling back to a savepoint:
1. All undo log entries after the savepoint LSN are applied in reverse order
2. Inserted rows are deleted
3. Updated/deleted rows are restored from before-images
4. The savepoint remains active for multiple rollbacks

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
