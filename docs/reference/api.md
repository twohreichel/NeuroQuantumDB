# API Reference

## Rust API Documentation

Detailed Rust documentation for all crates:

- **[neuroquantum-core](../api/neuroquantum_core/index.html)** — Core engine: DNA compression, quantum algorithms, storage
- **[neuroquantum-api](../api/neuroquantum_api/index.html)** — REST API, WebSocket, authentication  
- **[neuroquantum-qsql](../api/neuroquantum_qsql/index.html)** — QSQL parser, optimizer, executor

Or browse all crates: **[API Index](../api/index.html)**

---

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

**Note:** `/api/v1/query/explain` and `/api/v1/sql` endpoints are not implemented. Use `/api/v1/query` instead.

### DNA

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/dna/compress` | Compress data |
| `POST` | `/api/v1/dna/decompress` | Decompress data |
| `GET` | `/api/v1/dna/stats` | Compression stats |

### Quantum

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/quantum/search` | Quantum similarity search |

**Note:** `/api/v1/quantum/optimize` endpoint is not implemented.

### Neural

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/neural/train` | Train network |
| `GET` | `/api/v1/neural/train/{network_id}` | Training status |

**Note:** `/api/v1/neural/create` and `/api/v1/neural/predict` endpoints are not implemented.

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
