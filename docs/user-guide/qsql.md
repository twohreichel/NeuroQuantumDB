# QSQL Query Language

QSQL extends SQL with neuromorphic and quantum-inspired operations.

## Standard SQL

```sql
-- DDL
CREATE TABLE products (id INT, name TEXT, price FLOAT);
DROP TABLE products;

-- DML
INSERT INTO products VALUES (1, 'Widget', 9.99);
UPDATE products SET price = 12.99 WHERE id = 1;
DELETE FROM products WHERE id = 1;

-- Query
SELECT * FROM products WHERE price > 10;
SELECT name, COUNT(*) FROM orders GROUP BY name;
```

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

→ [REST API](rest-api.md)
