# Quick Start

This guide will get you up and running with NeuroQuantumDB in under 5 minutes.

## Prerequisites

- ✅ NeuroQuantumDB installed ([Installation Guide](./installation.md))
- ✅ Server running on `http://localhost:8080`
- ✅ Admin API key generated

---

## Step 1: Initialize the Database

```bash
# Initialize with admin credentials
neuroquantum-api init

# Save the output:
# ✅ Admin API Key: nq_live_abc123xyz789...
```

**Store this key securely!** You'll need it for all API requests.

---

## Step 2: Verify Server is Running

```bash
# Health check
curl http://localhost:8080/health

# Expected response:
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 42
}
```

---

## Step 3: First QSQL Query

### 3.1 Create a Table

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer nq_live_abc123xyz789..." \
  -H "Content-Type: application/json" \
  -d '{
    "query": "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100), email VARCHAR(255), age INT)"
  }'
```

**Response:**
```json
{
  "success": true,
  "rows_affected": 0,
  "execution_time_ms": 12.5
}
```

### 3.2 Insert Data

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer nq_live_abc123xyz789..." \
  -H "Content-Type: application/json" \
  -d '{
    "query": "INSERT INTO users VALUES (1, \"Alice\", \"alice@example.com\", 30)"
  }'
```

**Batch Insert:**
```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer nq_live_abc123xyz789..." \
  -H "Content-Type: application/json" \
  -d '{
    "query": "INSERT INTO users VALUES 
      (2, \"Bob\", \"bob@example.com\", 25),
      (3, \"Charlie\", \"charlie@example.com\", 35),
      (4, \"Diana\", \"diana@example.com\", 28)"
  }'
```

### 3.3 Query Data

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer nq_live_abc123xyz789..." \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT * FROM users WHERE age > 25"
  }'
```

**Response:**
```json
{
  "success": true,
  "columns": ["id", "name", "email", "age"],
  "rows": [
    [1, "Alice", "alice@example.com", 30],
    [3, "Charlie", "charlie@example.com", 35],
    [4, "Diana", "diana@example.com", 28]
  ],
  "rows_returned": 3,
  "execution_time_ms": 8.2
}
```

---

## Step 4: Advanced QSQL Features

### 4.1 Quantum-Optimized Search (Grover's Algorithm)

```sql
-- Use Grover's algorithm for search optimization
SELECT * FROM users 
WHERE name = 'Alice'
USING QUANTUM GROVER;
```

**Benefits:**
- Quadratic speedup for unindexed searches
- Automatic oracle function generation
- Optimal iteration count calculation

### 4.2 Neuromorphic Index

```sql
-- Create adaptive neuromorphic index
CREATE NEUROMORPHIC INDEX ON users(name)
WITH LEARNING_RATE 0.01;
```

**Features:**
- Hebbian learning for access pattern adaptation
- Synaptic weight updates based on query frequency
- Automatic index optimization

### 4.3 DNA Compression

```sql
-- Enable DNA compression for large text columns
ALTER TABLE users 
ADD COLUMN notes TEXT 
WITH COMPRESSION DNA;
```

**Compression Ratios:**
- Plain text: ~500:1
- JSON data: ~200:1
- Binary data: ~100:1

### 4.4 Natural Language Queries

```sql
-- Natural language query (translated to QSQL)
EXPLAIN "Find all users older than 25"
```

**Supported Patterns:**
- "Find all X where Y"
- "Show me X ordered by Y"
- "Count X grouped by Y"
- "Delete X where Y"

---

## Step 5: WebSocket Real-Time Queries

### 5.1 Connect via WebSocket

```javascript
// JavaScript example
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  console.log('Connected to NeuroQuantumDB');
  
  // Authenticate
  ws.send(JSON.stringify({
    type: 'auth',
    token: 'nq_live_abc123xyz789...'
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};
```

### 5.2 Subscribe to Table Changes

```javascript
// Subscribe to INSERT/UPDATE/DELETE events
ws.send(JSON.stringify({
  type: 'subscribe',
  table: 'users',
  events: ['insert', 'update', 'delete']
}));

// Receive real-time updates:
// {
//   "type": "table_update",
//   "table": "users",
//   "event": "insert",
//   "row": [5, "Eve", "eve@example.com", 27]
// }
```

### 5.3 Stream Query Results

```javascript
// Execute query with streaming results
ws.send(JSON.stringify({
  type: 'query_stream',
  query: 'SELECT * FROM users'
}));

// Receive results in chunks:
// { "type": "query_chunk", "rows": [...] }
// { "type": "query_complete", "total_rows": 100 }
```

---

## Step 6: Monitoring & Metrics

### 6.1 Prometheus Metrics

```bash
curl http://localhost:8080/metrics
```

**Key Metrics:**
- `neuroquantum_queries_total` - Total queries executed
- `neuroquantum_query_duration_seconds` - Query latency histogram
- `neuroquantum_cache_hit_rate` - Buffer pool cache hit rate
- `neuroquantum_compression_ratio` - DNA compression effectiveness
- `neuroquantum_grover_iterations` - Quantum search iterations

### 6.2 Database Statistics

```bash
curl http://localhost:8080/api/v1/stats \
  -H "Authorization: Bearer nq_live_abc123xyz789..."
```

**Response:**
```json
{
  "tables": 1,
  "total_rows": 4,
  "total_size_bytes": 8192,
  "cache_hit_rate": 0.85,
  "queries_per_second": 42.5,
  "avg_query_time_ms": 12.3,
  "compression_ratio": 456.2,
  "quantum_searches": 128,
  "neuromorphic_updates": 1024
}
```

---

## Step 7: Backup & Recovery

### 7.1 Create Backup

```bash
curl -X POST http://localhost:8080/api/v1/backup \
  -H "Authorization: Bearer nq_live_abc123xyz789..." \
  -H "Content-Type: application/json" \
  -d '{
    "type": "full",
    "destination": "/backups/neuroquantum-backup-2025-11-04.tar.gz"
  }'
```

### 7.2 Restore Backup

```bash
curl -X POST http://localhost:8080/api/v1/restore \
  -H "Authorization: Bearer nq_live_abc123xyz789..." \
  -H "Content-Type: application/json" \
  -d '{
    "source": "/backups/neuroquantum-backup-2025-11-04.tar.gz",
    "verify_checksum": true
  }'
```

---

## Common QSQL Patterns

### Transactions

```sql
BEGIN TRANSACTION;

INSERT INTO users VALUES (6, 'Frank', 'frank@example.com', 40);
UPDATE users SET age = 31 WHERE id = 1;
DELETE FROM users WHERE id = 2;

COMMIT;
-- or ROLLBACK;
```

### Joins

```sql
CREATE TABLE orders (
  id INT PRIMARY KEY, 
  user_id INT, 
  amount DECIMAL(10,2)
);

SELECT u.name, o.amount
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE o.amount > 100;
```

### Aggregations

```sql
SELECT 
  age,
  COUNT(*) as count,
  AVG(age) as avg_age
FROM users
GROUP BY age
HAVING count > 1
ORDER BY avg_age DESC;
```

### Explain Query Plan

```sql
EXPLAIN ANALYZE 
SELECT * FROM users WHERE age > 25;

-- Shows:
-- - Execution plan (sequential scan vs index scan)
-- - Quantum optimization applied
-- - Estimated vs actual rows
-- - Execution time breakdown
```

---

## Performance Tips

### 1. Use Indexes Wisely

```sql
-- Create regular index for frequent lookups
CREATE INDEX idx_users_email ON users(email);

-- Create neuromorphic index for adaptive patterns
CREATE NEUROMORPHIC INDEX ON users(name);
```

### 2. Enable DNA Compression for Large Data

```sql
-- Compress large text columns
ALTER TABLE logs ADD COLUMN message TEXT WITH COMPRESSION DNA;
```

### 3. Use Quantum Search for Unindexed Queries

```sql
-- Automatic Grover's algorithm for OR conditions
SELECT * FROM users 
WHERE name = 'Alice' OR email = 'bob@example.com'
USING QUANTUM GROVER;
```

### 4. Batch Operations

```sql
-- Insert multiple rows at once
INSERT INTO users VALUES 
  (7, 'User7', 'user7@example.com', 27),
  (8, 'User8', 'user8@example.com', 28),
  (9, 'User9', 'user9@example.com', 29);
```

### 5. Monitor Cache Hit Rate

```bash
# Aim for > 80% cache hit rate
curl http://localhost:8080/metrics | grep cache_hit_rate
```

---

## Next Steps

Now that you've mastered the basics, explore:

- 📖 [Configuration Guide](./configuration.md) - Fine-tune performance
- 🔒 [Security Setup](./security-setup.md) - Production hardening
- 🧬 [DNA Compression Examples](../examples/dna-compression.md) - Advanced compression
- ⚛️ [Quantum Algorithms](../examples/grover-algorithm.md) - Deep dive into quantum features
- 🧠 [Neuromorphic Learning](../examples/neuromorphic-learning.md) - Adaptive indexing
- 🚀 [Deployment Guide](../deployment/docker.md) - Production deployment

---

## Troubleshooting

### "Unauthorized" Error

**Problem:** API requests return 401 Unauthorized

**Solution:**
```bash
# Ensure you're using the correct API key
curl -H "Authorization: Bearer YOUR_ACTUAL_KEY" http://localhost:8080/api/v1/stats
```

### "Table not found" Error

**Problem:** Query fails with table not found

**Solution:**
```sql
-- List all tables
SHOW TABLES;

-- Verify table name (case-sensitive)
SELECT * FROM users;  -- Correct
SELECT * FROM Users;  -- Wrong (if table is lowercase)
```

### Slow Query Performance

**Problem:** Queries taking > 100ms

**Solution:**
```sql
-- Analyze query plan
EXPLAIN ANALYZE SELECT * FROM users WHERE age > 25;

-- Add index if sequential scan detected
CREATE INDEX idx_users_age ON users(age);

-- Or use quantum optimization
SELECT * FROM users WHERE age > 25 USING QUANTUM GROVER;
```

---

## API Reference

For complete API documentation, see:
- [REST API Reference](../api-reference/rest-api.md)
- [QSQL Language Specification](../api-reference/qsql-language.md)
- [WebSocket API Reference](../api-reference/websocket.md)

