# REST API

Base URL: `http://localhost:8080/api/v1`

## Authentication

All requests require an API key or JWT token:

```bash
# API Key
Authorization: Bearer nqdb_xxxxxxxxxxxx

# JWT Token
Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
```

## Endpoints

### Health & Status

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/metrics` | Prometheus metrics |
| GET | `/api/v1/stats` | Database statistics |

### Query Execution

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/query` | Execute QSQL query |
| POST | `/api/v1/query/stream` | Stream query results |

**Request:**
```json
{
  "query": "SELECT * FROM users",
  "params": {}
}
```

**Response:**
```json
{
  "columns": ["id", "name", "email"],
  "rows": [
    [1, "Alice", "alice@example.com"]
  ],
  "execution_time_ms": 12
}
```

### DNA Compression

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/dna/compress` | Compress data |
| POST | `/api/v1/dna/decompress` | Decompress data |

### Quantum Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/quantum/search` | Grover search |
| POST | `/api/v1/quantum/optimize` | QUBO optimization |

### Neural Networks

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/neural/train` | Train network |
| POST | `/api/v1/neural/predict` | Get prediction |
| GET | `/api/v1/neural/status` | Training status |

### Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/auth/keys` | Create API key |
| DELETE | `/api/v1/auth/keys/{id}` | Revoke API key |
| GET | `/api/v1/auth/keys` | List API keys |

## WebSocket

Connect to `/ws` for real-time updates:

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'query_results'
}));
```

## Error Responses

```json
{
  "error": {
    "code": "INVALID_QUERY",
    "message": "Syntax error near 'SELEC'"
  }
}
```

## Next Steps

→ [Features](features.md)
