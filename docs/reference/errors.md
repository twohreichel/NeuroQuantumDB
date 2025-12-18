# Error Codes

## Error Response Format

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable message"
  }
}
```

## Error Categories

### Authentication Errors (AUTH_*)

| Code | Description |
|------|-------------|
| `AUTH_INVALID_TOKEN` | JWT token is invalid |
| `AUTH_EXPIRED_TOKEN` | JWT token has expired |
| `AUTH_INVALID_KEY` | API key is invalid |
| `AUTH_EXPIRED_KEY` | API key has expired |
| `AUTH_INSUFFICIENT_PERMISSIONS` | Missing required permission |

### Query Errors (QUERY_*)

| Code | Description |
|------|-------------|
| `QUERY_SYNTAX_ERROR` | QSQL syntax error |
| `QUERY_TABLE_NOT_FOUND` | Table does not exist |
| `QUERY_COLUMN_NOT_FOUND` | Column does not exist |
| `QUERY_TYPE_MISMATCH` | Data type mismatch |
| `QUERY_TIMEOUT` | Query execution timeout |

### Storage Errors (STORAGE_*)

| Code | Description |
|------|-------------|
| `STORAGE_DISK_FULL` | No disk space |
| `STORAGE_CORRUPTED` | Data corruption detected |
| `STORAGE_WAL_ERROR` | WAL write failed |
| `STORAGE_LOCK_TIMEOUT` | Could not acquire lock |

### Transaction Errors (TXN_*)

| Code | Description |
|------|-------------|
| `TXN_CONFLICT` | Transaction conflict |
| `TXN_DEADLOCK` | Deadlock detected |
| `TXN_TIMEOUT` | Transaction timeout |
| `TXN_ABORTED` | Transaction aborted |

### Rate Limit Errors (RATE_*)

| Code | Description |
|------|-------------|
| `RATE_LIMIT_EXCEEDED` | Too many requests |

## Core Error Types

```rust
pub enum NeuroQuantumError {
    Storage(StorageError),
    Query(QueryError),
    Transaction(TransactionError),
    Compression(CompressionError),
    Quantum(QuantumError),
    Neural(NeuralError),
    Config(ConfigError),
}
```
