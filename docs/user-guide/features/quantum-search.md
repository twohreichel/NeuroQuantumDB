# Quantum Search

Grover's algorithm for quadratic speedup in unstructured search.

## How It Works

```
Classical Search: O(N) operations
Quantum Search:   O(√N) operations

For N = 1,000,000:
  Classical: 1,000,000 comparisons
  Quantum:   ~1,000 comparisons
```

## Usage

### API

```bash
curl -X POST http://localhost:8080/api/v1/quantum/search \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "table": "users",
    "condition": {"age": {"$gt": 30}},
    "iterations": 100
  }'
```

### QSQL

```sql
-- Basic quantum search
QUANTUM SEARCH users WHERE age > 30;

-- With iteration limit
QUANTUM SEARCH products 
  WHERE price < 100 
  WITH ITERATIONS 50;
```

## QUBO Optimization

Solve quadratic optimization problems:

```sql
OPTIMIZE QUBO
  MINIMIZE 3*x1 + 2*x2 - x1*x2
  SUBJECT TO x1 + x2 <= 1;
```

## Configuration

```toml
[quantum]
# Minimum search space for quantum advantage
min_search_space = 4

# Default Grover iterations
default_iterations = 100

# Enable parallel tempering
parallel_tempering = true
```

## When to Use

| Scenario | Recommendation |
|----------|----------------|
| Small dataset (< 1000) | Use classical search |
| Large dataset (> 10000) | Use quantum search |
| Optimization problems | Use QUBO |
