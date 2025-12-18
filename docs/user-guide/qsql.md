# QSQL Query Language

QSQL extends SQL with neuromorphic and quantum-inspired operations.

## Standard SQL

### Data Definition (DDL)

```sql
-- Create table with auto-increment ID (recommended)
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TIMESTAMP
);

-- Alternative: Using AUTO_INCREMENT constraint
CREATE TABLE products (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    name TEXT NOT NULL,
    price FLOAT
);

-- SQL:2003 standard syntax
CREATE TABLE orders (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id INTEGER NOT NULL,
    total FLOAT
);

-- Drop table
DROP TABLE products;
```

### Auto-Increment Data Types

| Type | Range | Storage | Description |
|------|-------|---------|-------------|
| `SMALLSERIAL` | 1 to 32,767 | 2 bytes | Small auto-increment |
| `SERIAL` | 1 to 2,147,483,647 | 4 bytes | Standard auto-increment |
| `BIGSERIAL` | 1 to 9,223,372,036,854,775,807 | 8 bytes | Large auto-increment (recommended) |

### Data Manipulation (DML)

```sql
-- Insert WITHOUT specifying ID - it's auto-generated!
INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');

-- Insert multiple rows
INSERT INTO users (name, email) VALUES 
    ('Bob', 'bob@example.com'),
    ('Charlie', 'charlie@example.com');

-- Update
UPDATE users SET email = 'newemail@example.com' WHERE id = 1;

-- Delete
DELETE FROM users WHERE id = 1;
```

### Query

```sql
-- Basic select
SELECT * FROM users WHERE id > 10;

-- Aggregation
SELECT name, COUNT(*) FROM orders GROUP BY name;

-- Pagination
SELECT * FROM users ORDER BY id LIMIT 10 OFFSET 20;
```

## ID Generation Strategies

NeuroQuantumDB supports three ID generation strategies:

### 1. Auto-Increment (Default)

Best for single-instance databases with high performance requirements.

```sql
-- Using BIGSERIAL (PostgreSQL-style)
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT
);

-- Using AUTO_INCREMENT (MySQL-style)  
CREATE TABLE users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    name TEXT
);
```

**Pros:**
- Minimal storage (8 bytes)
- Excellent B+Tree performance (sequential inserts)
- Human-readable and debuggable
- Perfect for synaptic/neural ID references

**Cons:**
- Predictable (potential security concern for public APIs)
- Requires coordination in distributed systems

### 2. UUID

Best for distributed systems where IDs must be globally unique.

```sql
-- Table must use TEXT type for UUID
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    event_type TEXT
) WITH ID_STRATEGY = 'UUID';
```

**Pros:**
- Globally unique without coordination
- Unpredictable (good for security)
- Works in distributed systems

**Cons:**
- Larger storage (16 bytes)
- Poor B+Tree performance (random distribution)
- Not human-readable

### 3. Snowflake

Best for distributed systems requiring time-sortable IDs.

```sql
CREATE TABLE logs (
    id BIGINT PRIMARY KEY,
    message TEXT
) WITH ID_STRATEGY = 'SNOWFLAKE', MACHINE_ID = 1;
```

**Pros:**
- Time-sortable (roughly ordered by creation)
- Distributed generation with machine ID
- Same storage as auto-increment (8 bytes)

**Cons:**
- Requires time synchronization
- More complex implementation

## Quantum Extensions

### Quantum Search

```sql
-- Grover's algorithm search
QUANTUM SEARCH users WHERE age > 30;

-- With optimization hints
QUANTUM SEARCH products 
  WHERE category = 'electronics' 
  WITH ITERATIONS 100;
```

### QUBO Optimization

```sql
-- Quadratic optimization
OPTIMIZE QUBO 
  MINIMIZE x1 + 2*x2 - x1*x2
  SUBJECT TO x1 + x2 <= 1;
```

## Neural Operations

### Train Network

```sql
-- Train synaptic network
NEURAL TRAIN network_name 
  ON training_data 
  EPOCHS 100 
  LEARNING_RATE 0.01;
```

### Predict

```sql
-- Neural prediction
NEURAL PREDICT network_name 
  INPUT (0.5, 0.3, 0.8);
```

## DNA Compression

```sql
-- Compress data
COMPRESS TABLE large_data USING DNA;

-- Decompress
DECOMPRESS TABLE large_data;

-- Check compression ratio
SHOW COMPRESSION STATS FOR large_data;
```

## Query Optimization

```sql
-- Explain query plan
EXPLAIN SELECT * FROM users WHERE id = 1;

-- Analyze performance
ANALYZE TABLE users;
```

## Next Steps

- [REST API](rest-api.md)
- [Auto-Increment Configuration](features/auto-increment.md)
