# REST API

Base URL: `http://localhost:8080/api/v1`

## Authentication

All requests require an API key:

```bash
# API Key Header
X-API-Key: nqdb_xxxxxxxxxxxx
```

## Endpoints

### Health & Status

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/metrics` | Prometheus metrics |
| GET | `/api/v1/stats` | Database statistics |

### Table Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/tables` | Create table |
| GET | `/api/v1/tables` | List tables |
| DELETE | `/api/v1/tables/{name}` | Drop table |

#### Create Table with Auto-Increment

```bash
curl -X POST http://localhost:8080/api/v1/tables \
  -H "X-API-Key: your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "users",
    "columns": [
      {
        "name": "id",
        "data_type": "BigSerial",
        "nullable": false,
        "auto_increment": true
      },
      {
        "name": "name",
        "data_type": "Text",
        "nullable": false
      },
      {
        "name": "email",
        "data_type": "Text",
        "nullable": true
      }
    ],
    "id_strategy": "AutoIncrement"
  }'
```

**Column Data Types:**

| Type | Description |
|------|-------------|
| `BigSerial` | Auto-incrementing 64-bit integer |
| `Serial` | Auto-incrementing 32-bit integer |
| `SmallSerial` | Auto-incrementing 16-bit integer |
| `Integer` | 64-bit integer |
| `Float` | 64-bit floating point |
| `Text` | Variable-length string |
| `Boolean` | true/false |
| `Timestamp` | Date and time |
| `Binary` | Binary data |

**ID Strategy Options:**

| Strategy | Description |
|----------|-------------|
| `AutoIncrement` | Sequential integers (default, recommended) |
| `Uuid` | Random UUIDs |
| `Snowflake` | Time-based distributed IDs |

### Record Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/records` | Insert record |
| PUT | `/api/v1/records` | Update record |
| DELETE | `/api/v1/records` | Delete record |

#### Insert Record (Auto-Generated ID)

```bash
# ID is automatically generated - don't include it!
curl -X POST http://localhost:8080/api/v1/records \
  -H "X-API-Key: your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "table_name": "users",
    "record": {
      "name": "Alice",
      "email": "alice@example.com"
    }
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "inserted_id": 1,
    "rows_affected": 1
  }
}
```

The `inserted_id` field returns the auto-generated ID.

### Query Execution

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/query` | Execute QSQL query |
| POST | `/api/v1/query/stream` | Stream query results |

**Request:**
```json
{
  "query": "SELECT * FROM users WHERE id > 10",
  "params": {}
}
```

**Response:**
```json
{
  "columns": ["id", "name", "email"],
  "rows": [
    [11, "Alice", "alice@example.com"],
    [12, "Bob", "bob@example.com"]
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

### API Key Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/auth/generate-key` | Create API key |
| POST | `/api/v1/auth/revoke-key` | Revoke API key |
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
  "success": false,
  "error": {
    "code": "INVALID_QUERY",
    "message": "Syntax error near 'SELEC'"
  }
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `INVALID_QUERY` | SQL syntax error |
| `TABLE_NOT_FOUND` | Table does not exist |
| `PERMISSION_DENIED` | API key lacks permission |
| `VALIDATION_ERROR` | Invalid request data |
| `INTERNAL_ERROR` | Server error |

## Next Steps

- [Features](features.md)
- [Auto-Increment Configuration](features/auto-increment.md)
