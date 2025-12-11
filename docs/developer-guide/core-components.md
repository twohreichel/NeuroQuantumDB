# Core Components

Overview of the main components in `neuroquantum-core`.

## Component Map

| Component | Module | Purpose |
|-----------|--------|---------|
| [Storage Engine](components/storage-engine.md) | `storage/` | Data persistence |
| [Transaction Manager](components/transaction-manager.md) | `transaction.rs` | ACID compliance |
| [Quantum Processor](components/quantum-processor.md) | `quantum/` | Quantum algorithms |
| [Synaptic Network](components/synaptic-network.md) | `synaptic.rs` | Neural computing |

## Interactions

```
         ┌─────────────────┐
         │ Transaction Mgr │
         └────────┬────────┘
                  │ coordinates
    ┌─────────────┼─────────────┐
    ↓             ↓             ↓
┌───────┐   ┌─────────┐   ┌─────────┐
│Storage│←──│ Buffer  │──→│   WAL   │
│Engine │   │  Pool   │   │         │
└───────┘   └─────────┘   └─────────┘
    ↑
    │ indexes
    ↓
┌───────┐
│B+Tree │
└───────┘
```

## Key Traits

```rust
/// Storage abstraction
pub trait StorageBackend {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
}

/// Compressor abstraction
pub trait Compressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;
}
```
