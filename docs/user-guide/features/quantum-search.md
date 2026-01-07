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

Solve quadratic optimization problems with **real quantum backends**:

```sql
OPTIMIZE QUBO
  MINIMIZE 3*x1 + 2*x2 - x1*x2
  SUBJECT TO x1 + x2 <= 1
  BACKEND SQA;  -- SimulatedQuantumAnnealing
```

### Available Backends

| Backend | Description |
|---------|-------------|
| `VQE` | Variational Quantum Eigensolver |
| `QAOA` | Quantum Approximate Optimization Algorithm |
| `QA` | Quantum Annealing (D-Wave style) |
| `SQA` | Simulated Quantum Annealing (default) |
| `CLASSICAL` | Classical simulated annealing fallback |

### Advanced QUBO Example

```sql
-- Max-Cut problem with quantum optimization
OPTIMIZE QUBO
  GRAPH max_cut
  NODES 100
  EDGES FROM graph_edges
  BACKEND SQA
  TROTTER_SLICES 32
  ITERATIONS 1000;
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

# QUBO quantum backend settings
[quantum.qubo]
backend = "sqa"           # vqe, qaoa, qa, sqa, classical
trotter_slices = 32       # For SQA
qaoa_depth = 3            # For QAOA
auto_fallback = true      # Fall back to classical if quantum fails
```

## When to Use

| Scenario | Recommendation |
|----------|----------------|
| Small dataset (< 1000) | Use classical search |
| Large dataset (> 10000) | Use quantum search |
| Optimization problems | Use QUBO |
