# API Reference

## REST Endpoints

### Authentication

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/auth/keys` | Create API key |
| `GET` | `/api/v1/auth/keys` | List API keys |
| `DELETE` | `/api/v1/auth/keys/{id}` | Revoke API key |
| `POST` | `/api/v1/auth/refresh` | Refresh JWT token |

### Query

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/query` | Execute QSQL |
| `POST` | `/api/v1/query/stream` | Stream results |
| `POST` | `/api/v1/query/explain` | Explain plan |

### DNA

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/dna/compress` | Compress data |
| `POST` | `/api/v1/dna/decompress` | Decompress data |
| `GET` | `/api/v1/dna/stats` | Compression stats |

### Quantum

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/quantum/search` | Grover search |
| `POST` | `/api/v1/quantum/optimize` | QUBO solver |

### Neural

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/neural/create` | Create network |
| `POST` | `/api/v1/neural/train` | Train network |
| `POST` | `/api/v1/neural/predict` | Predict |
| `GET` | `/api/v1/neural/status` | Training status |

### System

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/api/v1/stats` | Database stats |

## Response Codes

| Code | Meaning |
|------|---------|
| `200` | Success |
| `201` | Created |
| `400` | Bad request |
| `401` | Unauthorized |
| `403` | Forbidden |
| `404` | Not found |
| `429` | Rate limited |
| `500` | Internal error |

## Rust Crate Documentation

Generate with:

```bash
cargo doc --open
```
