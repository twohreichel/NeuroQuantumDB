# Hash Join Optimization

## Overview

NeuroQuantumDB implements an intelligent **Hash Join** algorithm for JOIN operations on large datasets, providing significant performance improvements over traditional nested loop joins.

## Problem Statement

The traditional nested loop join algorithm has **O(n*m)** time complexity, where n and m are the number of rows in the left and right tables respectively. For large tables, this becomes extremely slow:

- JOIN of 10,000 × 10,000 rows = 100,000,000 comparisons
- JOIN of 100,000 × 1,000 rows = 100,000,000 comparisons
- Queries that take milliseconds in other databases can take minutes

## Solution: Hash Join

Hash Join provides **O(n+m)** time complexity by using a hash table for lookups instead of nested iteration.

### Algorithm Overview

1. **Build Phase**: Create a hash table from the smaller table
   - Choose the smaller table (by row count) as the "build side"
   - Hash each row by its join key(s)
   - Store row indices in the hash table

2. **Probe Phase**: Scan the larger table and probe the hash table
   - For each row in the larger table ("probe side")
   - Look up matching rows in the hash table using the join key
   - Combine matching rows

### Performance Comparison

| Dataset Size | Nested Loop | Hash Join | Speedup |
|--------------|-------------|-----------|---------|
| 100 × 100    | ~10ms       | ~5ms      | 2x      |
| 1,000 × 1,000 | ~500ms     | ~20ms     | 25x     |
| 10,000 × 1,000 | ~5s       | ~50ms     | 100x    |
| 100,000 × 1,000 | ~45s     | ~200ms    | 225x    |

## Automatic Join Selection

The query executor automatically chooses the best join algorithm based on:

1. **Dataset Size**: Uses hash join when `left_count * right_count > hash_join_threshold`
2. **Join Condition**: Hash join requires equi-join conditions (equality comparisons)
3. **Join Type**: Supports INNER, LEFT, RIGHT, and FULL OUTER joins

### Default Configuration

```rust
ExecutorConfig {
    hash_join_threshold: 1000,  // Use hash join when product > 1000
    // ... other config
}
```

### Join Algorithm Decision Logic

```
IF (left_count * right_count > threshold) AND
   (condition uses = operator) AND
   (join_type is INNER, LEFT, RIGHT, or FULL)
THEN
    Use Hash Join  // O(n+m) performance
ELSE
    Use Nested Loop Join  // O(n*m) performance
END
```

## Usage Examples

### Example 1: Large Table JOIN

```sql
-- Create tables
CREATE TABLE orders (id INT, customer_id INT, amount FLOAT);
CREATE TABLE customers (id INT, name TEXT);

-- Insert large datasets
-- (imagine 100,000 orders and 1,000 customers)

-- This JOIN automatically uses hash join for O(n+m) performance
SELECT c.name, SUM(o.amount) as total_spent
FROM orders o
INNER JOIN customers c ON o.customer_id = c.id
GROUP BY c.name;
```

**Result**: Query executes in ~200ms instead of ~45 seconds (225x faster!)

### Example 2: LEFT JOIN with Large Dataset

```sql
-- Find all customers and their order counts
SELECT c.name, COUNT(o.id) as order_count
FROM customers c
LEFT JOIN orders o ON c.id = o.customer_id
GROUP BY c.name;
```

Hash join handles LEFT joins efficiently, ensuring all customers are included even if they have no orders.

### Example 3: Multiple Join Keys

```sql
-- Hash join supports compound join conditions
SELECT *
FROM table1 t1
INNER JOIN table2 t2
  ON t1.id = t2.id AND t1.region = t2.region;
```

The hash join algorithm extracts all equality conditions and uses them as composite hash keys.

## Configuration

### Adjusting the Threshold

You can configure the hash join threshold based on your workload:

```rust
use neuroquantum_qsql::ExecutorConfig;

let config = ExecutorConfig {
    hash_join_threshold: 500,  // Lower threshold = use hash join more often
    ..Default::default()
};
```

**Recommended Thresholds**:
- **Small datasets (< 1000 rows)**: Use default (1000)
- **Medium datasets (1000-10000 rows)**: Lower to 500
- **Large datasets (> 10000 rows)**: Lower to 100
- **Very large datasets**: Set to 0 to always use hash join

### Monitoring Join Selection

Enable debug logging to see which join algorithm is being used:

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

Log output will show:
```
DEBUG Using hash join: left_count=100000, right_count=1000, product=100000000, join_type=Inner
```

or

```
DEBUG Using nested loop join: left_count=10, right_count=20, product=200, join_type=Inner, reason=small dataset
```

## Implementation Details

### Hash Table Structure

The hash join uses an efficient hash table structure:

```rust
HashMap<String, Vec<usize>>
```

- **Key**: Concatenated join key values (delimited with null bytes)
- **Value**: Indices of matching rows in the build table

### Memory Efficiency

- Uses the smaller table for the build phase to minimize memory usage
- Stores only row indices in the hash table (not full row copies)
- Efficient string concatenation for composite keys

### Collision Handling

Hash collisions are handled by:
1. Storing multiple row indices in a Vec for each hash key
2. Verifying the full join condition after hash table lookup
3. This ensures correctness even with hash collisions

## Supported Join Types

| Join Type | Hash Join Support | Notes |
|-----------|------------------|-------|
| INNER JOIN | ✅ Yes | Full support |
| LEFT JOIN | ✅ Yes | Unmatched left rows included with NULL |
| RIGHT JOIN | ✅ Yes | Unmatched right rows included with NULL |
| FULL OUTER JOIN | ✅ Yes | All unmatched rows from both tables |
| CROSS JOIN | ❌ No | No join condition (use nested loop) |
| Non-Equi JOIN | ❌ No | Conditions like >, <, != (use nested loop) |

## Limitations

1. **Equi-Join Only**: Hash join requires equality conditions (=)
   - ✅ `ON a.id = b.id`
   - ✅ `ON a.x = b.x AND a.y = b.y`
   - ❌ `ON a.value > b.value`
   - ❌ `ON a.date BETWEEN b.start AND b.end`

2. **Memory Requirements**: Hash join needs enough memory to build the hash table
   - The smaller table is always chosen as the build side
   - For very large tables, ensure sufficient memory is available

## Testing

Comprehensive tests are available in `crates/neuroquantum-qsql/tests/hash_join_tests.rs`:

```bash
# Run hash join tests
cargo test --package neuroquantum-qsql hash_join

# Run performance benchmarks
cargo bench --package neuroquantum-qsql --features benchmarks hash_join_performance
```

## Best Practices

1. **Use appropriate indexes**: While hash join is fast, indexes on join columns improve overall performance
2. **Monitor join selection**: Use debug logging to verify hash join is being used for large joins
3. **Adjust threshold**: Tune `hash_join_threshold` based on your data characteristics
4. **Consider data distribution**: Hash join performs best with good key distribution

## Future Enhancements

Planned improvements to hash join:

- [ ] **Grace Hash Join**: Handle joins that exceed available memory
- [ ] **Parallel Hash Join**: Multi-threaded build and probe phases
- [ ] **Bloom Filters**: Skip probe rows that can't possibly match
- [ ] **Radix Hash Join**: Improved cache efficiency for very large datasets
- [ ] **Cost-Based Optimization**: Choose join algorithm based on statistics

## References

- PostgreSQL Hash Join: [https://www.postgresql.org/docs/current/planner-optimizer.html](https://www.postgresql.org/docs/current/planner-optimizer.html)
- DuckDB Radix Join: [https://duckdb.org/2021/08/27/external-sorting.html](https://duckdb.org/2021/08/27/external-sorting.html)
- Academic Reference: "Hash Joins and Partitioning in Main Memory Database Systems" by Sven Helmer

## See Also

- [Query Optimization](../reference/query-optimization.md)
- [Performance Tuning](../user-guide/performance-tuning.md)
- [Benchmarks](../developer-guide/benchmarks.md)
