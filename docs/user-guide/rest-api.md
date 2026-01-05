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
    "schema": {
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
    }
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
| POST | `/api/v1/dna/compress` | Compress DNA sequences |
| POST | `/api/v1/dna/decompress` | Decompress DNA data |

#### Compress DNA Sequences

**Request:**
```json
{
  "sequences": [
    "ATCGATCGATCG",
    "GCTAGCTAGCTA"
  ],
  "algorithm": "KmerBased",
  "compression_level": 5
}
```

**Algorithm Options:**
- `KmerBased` - K-mer based compression
- `NeuralNetwork` - Neural network compression
- `QuantumInspired` - Quantum-inspired compression
- `Hybrid` - Hybrid approach

**Response:**
```json
{
  "success": true,
  "data": {
    "compressed_sequences": [
      {
        "original_length": 12,
        "compressed_data": "base64_encoded_data",
        "compression_ratio": 2.5,
        "checksum": "abc123"
      }
    ],
    "compression_stats": {
      "total_input_size": 24,
      "total_compressed_size": 10,
      "average_compression_ratio": 2.4,
      "compression_time_ms": 15.2
    }
  }
}
```

#### Decompress DNA Data

**Request:**
```json
{
  "compressed_data": [
    "base64_encoded_data1",
    "base64_encoded_data2"
  ]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "decompressed_sequences": [
      {
        "decompressed_data": "ATCGATCGATCG",
        "original_checksum": "abc123",
        "checksum_valid": true
      }
    ],
    "decompression_stats": {
      "total_compressed_size": 10,
      "total_decompressed_size": 24,
      "decompression_time_ms": 8.5
    }
  }
}
```

### Quantum Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/quantum/search` | Quantum similarity search |

**Note:** `/api/v1/quantum/optimize` endpoint is not implemented.

#### Quantum Search

**Request:**
```json
{
  "table_name": "users",
  "query_vector": [0.1, 0.5, 0.8, 0.3],
  "similarity_threshold": 0.7,
  "max_results": 10,
  "entanglement_boost": 1.2
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "results": [
      {
        "record": {
          "id": 1,
          "name": "Alice",
          "features": [0.15, 0.52, 0.79, 0.28]
        },
        "similarity_score": 0.95,
        "quantum_probability": 0.88,
        "entanglement_strength": 0.72
      }
    ],
    "quantum_stats": {
      "coherence_time_used_ms": 2.5,
      "superposition_states": 16,
      "measurement_collapses": 4,
      "entanglement_operations": 8
    }
  }
}
```

### Neural Networks

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/neural/train` | Train neural network |
| GET | `/api/v1/neural/train/{network_id}` | Get training status |

**Note:** `/api/v1/neural/predict` endpoint is not implemented.

#### Train Neural Network

**Request:**
```json
{
  "network_name": "user_classifier",
  "training_data": [
    {
      "input": [0.1, 0.5, 0.8],
      "target": [1.0, 0.0],
      "weight": 1.0
    }
  ],
  "config": {
    "layers": [
      {
        "layer_type": "Dense",
        "size": 64,
        "activation": "ReLU",
        "dropout": 0.2
      }
    ],
    "learning_rate": 0.001,
    "epochs": 100,
    "batch_size": 32,
    "optimizer": "Adam",
    "loss_function": "MeanSquaredError"
  },
  "validation_split": 0.2
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "network_id": "abc123",
    "training_started": true,
    "estimated_time_ms": 5000
  }
}
```

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
