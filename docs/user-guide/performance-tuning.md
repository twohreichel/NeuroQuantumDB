# Performance Tuning Guide

This comprehensive guide covers performance optimization techniques for NeuroQuantumDB, from basic configuration to advanced troubleshooting.

## Table of Contents

1. [Configuration](#1-configuration)
2. [Query Optimization](#2-query-optimization)
3. [Hardware Recommendations](#3-hardware-recommendations)
4. [Monitoring](#4-monitoring)
5. [Troubleshooting](#5-troubleshooting)
6. [Benchmarking](#6-benchmarking)

---

## 1. Configuration

### 1.1 Memory Settings

#### Buffer Pool Size

The buffer pool caches frequently accessed data pages in memory, significantly improving read performance.

```toml
# config/prod.toml
[storage]
buffer_pool_size_mb = 256  # Default: 256MB

# Recommendation based on available RAM:
# - 1GB RAM:  buffer_pool_size_mb = 128
# - 4GB RAM:  buffer_pool_size_mb = 512
# - 8GB RAM:  buffer_pool_size_mb = 1024
# - 16GB RAM: buffer_pool_size_mb = 2048
```

**Rule of Thumb:** Allocate 25-40% of available RAM to the buffer pool. Leave sufficient memory for the OS and other processes.

**Monitoring Buffer Pool Efficiency:**
```bash
# Check buffer pool hit rate via Prometheus
curl http://localhost:8080/metrics | grep nqdb_buffer_pool_hits
```

**Target:** Buffer pool hit rate > 95% indicates good cache efficiency.

#### Memory Pool Size

Controls the memory pool for query execution and temporary data structures.

```toml
[performance]
memory_pool_size_mb = 256  # Per-query memory limit

# Adjust based on query complexity:
# - Simple queries: 64-128 MB
# - Complex aggregations: 256-512 MB
# - Large joins: 512-1024 MB
```

#### Garbage Collection Threshold

```toml
[performance]
gc_threshold_mb = 50  # Trigger GC when unused memory exceeds threshold
```

Lower values trade increased GC frequency for lower memory footprint. Increase for workloads with many temporary objects.

### 1.2 Connection Pool Settings

#### Server Configuration

```toml
[server]
workers = 8              # Number of worker threads (match CPU cores)
max_connections = 10000  # Maximum concurrent connections

# Conservative sizing:
# workers = CPU cores (or CPU cores - 1)
# max_connections ≈ workers × 1000-2000
```

**Connection Memory Overhead:** Each connection consumes ~200KB. For 10,000 connections, reserve ~2GB RAM.

#### Database Connection Pool

```toml
[database]
max_connections = 500           # Maximum pool size
connection_timeout = 10          # Seconds to wait for connection
query_timeout = 30               # Maximum query execution time (seconds)
```

**Sizing Guidelines:**
- **Low concurrency** (< 100 active queries): `max_connections = 50-100`
- **Medium concurrency** (100-500 active queries): `max_connections = 200-500`
- **High concurrency** (> 500 active queries): `max_connections = 500-1000`

⚠️ **Warning:** Very high connection counts can degrade performance due to context switching. Consider connection pooling at the application layer.

### 1.3 WAL (Write-Ahead Logging) Configuration

WAL ensures ACID compliance and crash recovery but impacts write performance.

```toml
[storage]
wal_enabled = true                    # Enable WAL (required for durability)
wal_path = "/var/lib/neuroquantumdb/wal"
```

#### WAL Tuning Parameters

While NeuroQuantumDB manages WAL automatically, understand these concepts:

**Checkpoint Frequency:**
- More frequent checkpoints = slower writes, faster recovery
- Less frequent checkpoints = faster writes, slower recovery

**WAL Sync Modes** (if configurable in future versions):
- `fsync`: Maximum durability, slowest (each commit syncs to disk)
- `async`: Best performance, risk of data loss on crash
- `group_commit`: Batches commits for balanced performance/durability

**Best Practices:**
1. Store WAL on fast SSD storage
2. Use separate disk from main data files if possible
3. Monitor WAL size with `du -h /var/lib/neuroquantumdb/wal`
4. Archive old WAL files after successful checkpoints

### 1.4 Index Settings

Indexes accelerate queries but slow down inserts and consume storage.

#### Creating Indexes

```sql
-- Create B+Tree index (default)
CREATE INDEX idx_users_email ON users(email);

-- Create unique index
CREATE UNIQUE INDEX idx_users_id ON users(id);

-- Composite index for multi-column queries
CREATE INDEX idx_orders_user_date ON orders(user_id, order_date);
```

#### Index Strategy

**When to Index:**
- ✅ Columns frequently used in `WHERE` clauses
- ✅ Foreign key columns for joins
- ✅ Columns used in `ORDER BY` and `GROUP BY`
- ✅ Primary keys (automatically indexed)

**When NOT to Index:**
- ❌ Small tables (< 1000 rows) — sequential scan is faster
- ❌ Columns with low cardinality (e.g., boolean flags)
- ❌ Frequently updated columns — index maintenance overhead
- ❌ Columns in write-heavy tables

#### Index Maintenance

```sql
-- View indexes on a table
SHOW INDEXES FROM users;

-- Drop unused index
DROP INDEX idx_users_email;
```

**Monitoring:** Track index usage and remove unused indexes to save storage and improve write performance.

---

## 2. Query Optimization

### 2.1 Using EXPLAIN

EXPLAIN shows the query execution plan, helping identify performance bottlenecks.

```sql
EXPLAIN SELECT * FROM users WHERE email = 'john@example.com';
```

**Output Interpretation:**
```
Query Plan:
├─ Index Scan on idx_users_email
│  ├─ Estimated rows: 1
│  ├─ Cost: 0.42
│  └─ Index: B+Tree
└─ Execution time: 1.2ms
```

**Key Metrics:**
- **Scan Type:** `Index Scan` > `Sequential Scan` for large tables
- **Estimated Rows:** Fewer rows = faster query
- **Cost:** Lower is better (arbitrary units)

### 2.2 Index Strategies

#### Choose the Right Index Order

For composite indexes, order matters:

```sql
-- ✅ GOOD: Index matches query order
CREATE INDEX idx_orders_user_date ON orders(user_id, order_date);
SELECT * FROM orders WHERE user_id = 123 AND order_date > '2025-01-01';

-- ❌ POOR: Index doesn't match query
SELECT * FROM orders WHERE order_date > '2025-01-01' AND user_id = 123;
-- Consider: CREATE INDEX idx_orders_date_user ON orders(order_date, user_id);
```

**Rule:** Put the most selective column first (column with highest cardinality).

#### Covering Indexes

Include all columns used in a query to avoid table lookups:

```sql
-- Query uses id, email, created_at
CREATE INDEX idx_users_covering ON users(email, id, created_at);

SELECT id, created_at FROM users WHERE email = 'john@example.com';
-- ✅ Can be satisfied entirely from index
```

### 2.3 Query Rewriting

#### Avoid SELECT *

```sql
-- ❌ BAD: Retrieves unnecessary data
SELECT * FROM users WHERE id = 1;

-- ✅ GOOD: Only fetch needed columns
SELECT id, email, name FROM users WHERE id = 1;
```

#### Use Efficient Joins

```sql
-- ❌ AVOID: Cartesian product
SELECT * FROM users, orders WHERE users.id = orders.user_id;

-- ✅ PREFER: Explicit JOIN
SELECT u.*, o.* FROM users u
INNER JOIN orders o ON u.id = o.user_id;
```

#### Limit Result Sets

```sql
-- Always use LIMIT for large result sets
SELECT * FROM logs ORDER BY timestamp DESC LIMIT 100;
```

#### Optimize Subqueries

```sql
-- ❌ SLOW: Correlated subquery
SELECT * FROM users u WHERE u.id IN (
  SELECT user_id FROM orders WHERE total > 100
);

-- ✅ FAST: Use JOIN instead
SELECT DISTINCT u.* FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE o.total > 100;
```

### 2.4 Batch Operations

Group multiple operations into single transactions for better performance.

#### Batch Inserts

```sql
-- ❌ SLOW: Individual inserts
INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');
INSERT INTO users (name, email) VALUES ('Bob', 'bob@example.com');
INSERT INTO users (name, email) VALUES ('Charlie', 'charlie@example.com');

-- ✅ FAST: Batch insert
BEGIN TRANSACTION;
INSERT INTO users (name, email) VALUES 
  ('Alice', 'alice@example.com'),
  ('Bob', 'bob@example.com'),
  ('Charlie', 'charlie@example.com');
COMMIT;
```

**Performance Gain:** 10-100x faster for large batches.

#### Batch Updates

```sql
BEGIN TRANSACTION;
UPDATE users SET status = 'active' WHERE id IN (1, 2, 3, 4, 5);
UPDATE users SET last_login = NOW() WHERE id IN (1, 2, 3, 4, 5);
COMMIT;
```

### 2.5 Caching Strategies

#### Application-Level Caching

```rust
// Example: Redis cache
use redis::Client;

let client = Client::open("redis://127.0.0.1/")?;
let mut con = client.get_connection()?;

// Check cache first
let cached: Option<String> = con.get("user:123").ok();
if let Some(data) = cached {
    return Ok(data);
}

// Cache miss - query database
let result = query_database("SELECT * FROM users WHERE id = 123")?;
con.set_ex("user:123", &result, 300)?; // TTL: 5 minutes
```

#### Query Result Caching

NeuroQuantumDB includes built-in caching:

```toml
[database.dna_config]
cache_size_mb = 64  # DNA compression cache
```

**Effective for:**
- Frequently accessed bioinformatics data
- Repetitive read queries
- Cold start scenarios

---

## 3. Hardware Recommendations

### 3.1 CPU Requirements

#### Minimum Specifications

| Workload | Cores | Architecture | Notes |
|----------|-------|--------------|-------|
| **Development** | 2 cores | x86_64 or ARM64 | Sufficient for testing |
| **Small Production** | 4 cores | x86_64 or ARM64 | Up to 100 req/s |
| **Medium Production** | 8 cores | x86_64 or ARM64 | Up to 500 req/s |
| **Large Production** | 16+ cores | x86_64 or ARM64 | Up to 2000+ req/s |

#### ARM64 Optimization

NeuroQuantumDB is optimized for ARM64 (Apple Silicon, Raspberry Pi 4):

```toml
[performance]
arm64_neon_enabled = true  # Enable NEON SIMD instructions
```

**Performance Gains with NEON:**
- DNA Compression: 4.27x faster
- Matrix operations: 3-5x faster
- Vector computations: 2-4x faster

**Recommended ARM64 Hardware:**
- **Edge Deployment:** Raspberry Pi 4 (4GB+ RAM)
- **Desktop/Server:** Apple M1/M2/M3, AWS Graviton 3/4
- **High Performance:** Ampere Altra, NVIDIA Grace

### 3.2 RAM Sizing

#### Memory Requirements by Workload

| Workload | Minimum RAM | Recommended RAM | Buffer Pool | Notes |
|----------|-------------|-----------------|-------------|-------|
| **Development** | 1 GB | 2 GB | 128 MB | Single user testing |
| **Small DB** (< 1GB data) | 2 GB | 4 GB | 512 MB | Up to 50 connections |
| **Medium DB** (1-10GB data) | 8 GB | 16 GB | 2048 MB | Up to 500 connections |
| **Large DB** (10-100GB data) | 32 GB | 64 GB | 8192 MB | Up to 2000 connections |
| **Very Large DB** (> 100GB) | 128 GB+ | 256 GB+ | 32768+ MB | Enterprise scale |

**Calculation Formula:**
```
Total RAM = Buffer Pool + Connection Memory + OS + App Overhead
         ≈ Buffer Pool + (max_connections × 0.2 MB) + 2 GB + 1 GB
```

**Example:** For 500 connections with 2GB buffer pool:
```
Total = 2048 MB + (500 × 0.2 MB) + 2048 MB + 1024 MB ≈ 5.2 GB
Recommended: 8 GB RAM
```

### 3.3 Storage: SSD vs HDD

#### Storage Performance Comparison

| Operation | HDD | SATA SSD | NVMe SSD | Recommended |
|-----------|-----|----------|----------|-------------|
| **Sequential Read** | 120 MB/s | 550 MB/s | 3500 MB/s | NVMe > SATA SSD |
| **Random Read** | 1 MB/s | 95 MB/s | 500 MB/s | NVMe > SATA SSD |
| **Random Write** | 1 MB/s | 90 MB/s | 450 MB/s | NVMe > SATA SSD |
| **Latency** | 10-20 ms | 0.1-0.2 ms | 0.01-0.02 ms | NVMe > SATA SSD |

⚠️ **Critical:** NeuroQuantumDB requires SSD for production. HDD is only acceptable for development/testing.

#### Storage Layout

**Single Disk Setup:**
```
/var/lib/neuroquantumdb/
├── data/           # Main database files
└── wal/            # Write-ahead log
```

**Multi-Disk Setup (Optimal):**
```
Disk 1 (NVMe): /var/lib/neuroquantumdb/data/   # Main data
Disk 2 (SSD):  /var/lib/neuroquantumdb/wal/    # WAL for write performance
```

**Benefits:**
- Parallel I/O for reads and writes
- WAL writes don't block data reads
- Improved crash recovery performance

#### Storage Capacity Planning

```
Required Storage = Raw Data Size × Compression Ratio × Safety Factor

Example (bioinformatics data):
- Raw data: 100 GB
- DNA compression ratio: 4:1
- After compression: 25 GB
- Safety factor: 3× (indexes, WAL, temp)
- Total needed: 75 GB
```

**Recommendations:**
- **Development:** 50-100 GB SSD
- **Small Production:** 250-500 GB NVMe SSD
- **Medium Production:** 1-2 TB NVMe SSD
- **Large Production:** 4+ TB NVMe SSD (RAID 10)

### 3.4 Network for Cluster Deployments

⚠️ **Note:** Cluster mode is currently in Beta. For production, use single-node configuration.

For future cluster deployments:

#### Network Requirements

| Metric | Minimum | Recommended | Optimal |
|--------|---------|-------------|---------|
| **Bandwidth** | 1 Gbps | 10 Gbps | 25-100 Gbps |
| **Latency** | < 10 ms | < 1 ms | < 0.5 ms |
| **Topology** | Star | Full Mesh | RDMA Network |

#### Network Configuration

```toml
# config/cluster.toml (EXPERIMENTAL)
[cluster]
enabled = false  # Keep disabled for production
node_id = 1
bind_addr = "0.0.0.0:9000"

peers = [
    "10.0.1.2:9000",  # Node 2
    "10.0.1.3:9000"   # Node 3
]
```

**Best Practices:**
- Use dedicated network for inter-node communication
- Enable TCP keepalive
- Use load balancers with health checks
- Monitor network saturation

---

## 4. Monitoring

### 4.1 Prometheus Metrics Interpretation

NeuroQuantumDB exposes metrics at `http://localhost:8080/metrics`.

#### Key Performance Metrics

**Query Performance:**
```promql
# Query rate (queries per second)
rate(nqdb_queries_total[5m])

# Query latency (p50, p95, p99)
histogram_quantile(0.50, rate(nqdb_query_duration_seconds_bucket[5m]))
histogram_quantile(0.95, rate(nqdb_query_duration_seconds_bucket[5m]))
histogram_quantile(0.99, rate(nqdb_query_duration_seconds_bucket[5m]))

# Slow queries (> 1 second)
rate(nqdb_query_duration_seconds_bucket{le="1.0"}[5m])
```

**Resource Utilization:**
```promql
# Buffer pool hit rate
rate(nqdb_buffer_pool_hits[5m]) / rate(nqdb_buffer_pool_accesses[5m])

# Active connections
nqdb_connections_active

# Memory usage (if exposed)
process_resident_memory_bytes
```

**Feature-Specific Metrics:**
```promql
# DNA compression operations
rate(nqdb_dna_compressions_total[5m])

# Quantum search operations
rate(nqdb_quantum_searches_total[5m])

# Transaction rate
rate(nqdb_transactions_total[5m])
```

#### Target Thresholds

| Metric | Target | Warning | Critical |
|--------|--------|---------|----------|
| **Query Latency (p95)** | < 100 ms | 100-500 ms | > 500 ms |
| **Buffer Pool Hit Rate** | > 95% | 90-95% | < 90% |
| **Active Connections** | < 80% max | 80-90% max | > 90% max |
| **CPU Usage** | < 70% | 70-85% | > 85% |
| **Disk I/O Wait** | < 10% | 10-25% | > 25% |

### 4.2 Grafana Dashboards Setup

#### 1. Add Prometheus Data Source

```bash
# In Grafana UI:
Configuration → Data Sources → Add data source → Prometheus
URL: http://localhost:9090
```

#### 2. Import Dashboard

NeuroQuantumDB includes pre-built dashboards:

```bash
# Dashboard files located at:
docker/monitoring/dashboards/neuroquantumdb-overview.json
docker/monitoring/dashboards/neuroquantumdb-performance.json
```

**Import Steps:**
1. Grafana UI → Dashboards → Import
2. Upload JSON file or paste JSON
3. Select Prometheus data source
4. Click "Import"

#### 3. Key Dashboard Panels

**Overview Dashboard:**
- Query throughput (QPS)
- Active connections
- Resource usage (CPU, RAM, Disk)
- Error rate

**Performance Dashboard:**
- Query latency percentiles (p50, p95, p99)
- Buffer pool efficiency
- Index usage
- WAL write rate
- DNA compression ratio

### 4.3 Alerting Configuration

#### Prometheus Alerting Rules

Create `/etc/prometheus/alerts/neuroquantumdb.yml`:

```yaml
groups:
  - name: neuroquantumdb
    interval: 30s
    rules:
      # High query latency
      - alert: HighQueryLatency
        expr: histogram_quantile(0.95, rate(nqdb_query_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High query latency detected"
          description: "P95 latency is {{ $value }}s (threshold: 1s)"

      # Low buffer pool hit rate
      - alert: LowBufferPoolHitRate
        expr: rate(nqdb_buffer_pool_hits[5m]) / rate(nqdb_buffer_pool_accesses[5m]) < 0.90
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Buffer pool hit rate below 90%"
          description: "Consider increasing buffer_pool_size_mb"

      # Connection pool exhaustion
      - alert: ConnectionPoolNearLimit
        expr: nqdb_connections_active / nqdb_connections_max > 0.85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Connection pool usage above 85%"
          description: "{{ $value }}% of connections in use"

      # Database down
      - alert: DatabaseDown
        expr: up{job="neuroquantumdb"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "NeuroQuantumDB is down"
          description: "Instance {{ $labels.instance }} is unreachable"

      # High error rate
      - alert: HighErrorRate
        expr: rate(nqdb_errors_total[5m]) > 10
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "{{ $value }} errors/sec"
```

#### Alertmanager Configuration

Configure `/etc/alertmanager/alertmanager.yml`:

```yaml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'cluster']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'team-notifications'

receivers:
  - name: 'team-notifications'
    slack_configs:
      - api_url: 'YOUR_SLACK_WEBHOOK_URL'
        channel: '#database-alerts'
        title: 'NeuroQuantumDB Alert'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
    
    email_configs:
      - to: 'dba-team@example.com'
        from: 'alerts@example.com'
        smarthost: 'smtp.example.com:587'
        auth_username: 'alerts@example.com'
        auth_password: 'password'
```

### 4.4 Performance Regression Detection

#### Continuous Monitoring

Track performance trends over time:

```promql
# Compare current week vs last week
avg_over_time(
  histogram_quantile(0.95, rate(nqdb_query_duration_seconds_bucket[5m]))[7d:1h]
)

# Month-over-month query throughput
avg_over_time(rate(nqdb_queries_total[5m])[30d:1h])
```

#### Benchmark Regression Tests

Run automated benchmarks after deployments:

```bash
# Run benchmark suite
cargo bench --features benchmarks

# Compare with baseline
# Results stored in: target/criterion/
criterion-compare baseline current
```

**Set up CI/CD integration:**
```yaml
# .github/workflows/benchmark.yml
- name: Run Benchmarks
  run: cargo bench --features benchmarks -- --save-baseline main

- name: Compare Benchmarks
  run: cargo bench --features benchmarks -- --baseline main
```

---

## 5. Troubleshooting

### 5.1 Identifying Slow Queries

#### Enable Query Logging

```toml
[logging]
level = "debug"
structured_logging = true
```

```bash
# Watch for slow queries in logs
tail -f /var/log/neuroquantumdb/api.log | grep "Query executed" | grep -E "[0-9]{4,}ms"
```

#### Analyzing Query Plans

```sql
-- Get execution plan
EXPLAIN SELECT u.name, COUNT(o.id) as order_count
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.name;

-- Look for:
-- ❌ "Sequential Scan" on large tables
-- ❌ High "Cost" estimates
-- ❌ Missing indexes
```

#### Common Slow Query Patterns

**1. Missing Index:**
```sql
-- SLOW: Sequential scan
SELECT * FROM users WHERE email = 'john@example.com';

-- FIX: Add index
CREATE INDEX idx_users_email ON users(email);
```

**2. Non-Sargable Queries:**
```sql
-- ❌ SLOW: Function on indexed column prevents index usage
SELECT * FROM users WHERE UPPER(email) = 'JOHN@EXAMPLE.COM';

-- ✅ FAST: Use case-insensitive index or normalize data
SELECT * FROM users WHERE email = LOWER('john@example.com');
```

**3. Large Result Sets:**
```sql
-- ❌ SLOW: Returns millions of rows
SELECT * FROM logs;

-- ✅ FAST: Use pagination
SELECT * FROM logs ORDER BY timestamp DESC LIMIT 1000 OFFSET 0;
```

**4. Inefficient Joins:**
```sql
-- ❌ SLOW: Cartesian product
SELECT * FROM users, orders;

-- ✅ FAST: Proper JOIN with index
SELECT u.*, o.* FROM users u
INNER JOIN orders o ON u.id = o.user_id;
-- Ensure index exists: CREATE INDEX idx_orders_user_id ON orders(user_id);
```

### 5.2 Memory Issues

#### Symptoms

- Server crashes with OOM errors
- Swap usage increases significantly
- Query performance degrades over time
- `malloc` failures in logs

#### Diagnosis

```bash
# Check memory usage
free -h

# Monitor NeuroQuantumDB process
ps aux | grep neuroquantum-api

# Check buffer pool size
curl http://localhost:8080/metrics | grep buffer_pool
```

#### Solutions

**1. Reduce Buffer Pool Size:**
```toml
[storage]
buffer_pool_size_mb = 128  # Reduce from 256
```

**2. Limit Query Memory:**
```toml
[performance]
memory_pool_size_mb = 128  # Reduce per-query limit
```

**3. Reduce Connection Count:**
```toml
[server]
max_connections = 5000  # Reduce from 10000

[database]
max_connections = 250  # Reduce from 500
```

**4. Enable DNA Compression:**
```toml
[compression]
dna_enabled = true
compression_level = 6  # Higher = better compression, slower
```

**5. Increase System Limits:**
```bash
# Edit /etc/security/limits.conf
neuroquantum soft memlock unlimited
neuroquantum hard memlock unlimited

# Edit /etc/sysctl.conf
vm.overcommit_memory = 2
vm.overcommit_ratio = 90
```

### 5.3 Lock Contention

#### Symptoms

- Queries waiting for locks
- Timeout errors
- Decreased throughput under high concurrency

#### Diagnosis

```sql
-- Check for blocked queries (if implemented)
SHOW LOCKS;

-- Check long-running transactions
SHOW TRANSACTIONS;
```

```bash
# Monitor lock wait metrics
curl http://localhost:8080/metrics | grep lock_wait
```

#### Solutions

**1. Optimize Transaction Scope:**
```sql
-- ❌ BAD: Long-running transaction
BEGIN TRANSACTION;
SELECT * FROM large_table;  -- Holds locks for a long time
-- ... extensive processing ...
UPDATE users SET status = 'processed';
COMMIT;

-- ✅ GOOD: Minimize lock duration
SELECT * FROM large_table;  -- Query outside transaction
-- ... extensive processing ...
BEGIN TRANSACTION;
UPDATE users SET status = 'processed';
COMMIT;  -- Locks held briefly
```

**2. Use Batch Updates:**
```sql
-- Process in smaller batches to reduce lock time
BEGIN TRANSACTION;
UPDATE users SET status = 'active' WHERE id BETWEEN 1 AND 1000;
COMMIT;

BEGIN TRANSACTION;
UPDATE users SET status = 'active' WHERE id BETWEEN 1001 AND 2000;
COMMIT;
```

**3. Read Committed Isolation:**
```sql
-- Use lower isolation level if full serialization isn't needed
SET TRANSACTION ISOLATION LEVEL READ COMMITTED;
```

**4. Partition Hot Tables:**
```sql
-- Split frequently updated table into partitions
-- (Future feature)
CREATE TABLE orders_2025_01 PARTITION OF orders
  FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
```

### 5.4 Disk I/O Problems

#### Symptoms

- High disk I/O wait (`iowait` in `top`)
- Slow write performance
- WAL checkpoint delays
- Queries timing out

#### Diagnosis

```bash
# Monitor disk I/O
iostat -x 1

# Check disk usage
df -h /var/lib/neuroquantumdb

# Monitor I/O wait
top  # Look for high %wa (wait)

# Specific process I/O
iotop -p $(pgrep neuroquantum-api)
```

#### Solutions

**1. Upgrade to SSD/NVMe:**
- HDD → SATA SSD: 10-50x performance improvement
- SATA SSD → NVMe: 3-7x performance improvement

**2. Separate WAL and Data Disks:**
```toml
[storage]
data_path = "/mnt/nvme0/neuroquantumdb"  # Fast NVMe
wal_path = "/mnt/ssd0/neuroquantumdb/wal"  # Separate SSD
```

**3. Increase Buffer Pool:**
```toml
[storage]
buffer_pool_size_mb = 2048  # More caching = less disk I/O
```

**4. Optimize WAL Configuration:**
```bash
# Ensure WAL files aren't on slow storage
ls -lh /var/lib/neuroquantumdb/wal/

# Archive old WAL files
neuroquantum-api wal-archive --compress --target /backup/wal/
```

**5. Check Disk Health:**
```bash
# SMART status
smartctl -a /dev/sda

# File system errors
dmesg | grep -i error
```

**6. Disable Unnecessary Services:**
```bash
# Stop disk-intensive background services
systemctl stop updatedb.timer  # mlocate indexing
systemctl stop fstrim.timer     # If on HDD
```

---

## 6. Benchmarking

### 6.1 Running Benchmarks

NeuroQuantumDB uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for performance benchmarking.

#### Prerequisites

```bash
# Install Rust nightly (for certain benchmark features)
rustup install nightly

# Clone repository
git clone https://github.com/twoh-me/NeuroQuantumDB.git
cd NeuroQuantumDB
```

#### Running All Benchmarks

```bash
# Full benchmark suite (takes 30-60 minutes)
cargo bench --features benchmarks

# Results saved to: target/criterion/
# HTML reports: target/criterion/*/report/index.html
```

#### Running Specific Benchmarks

```bash
# B+Tree index benchmarks
cargo bench --features benchmarks -p neuroquantum-core --bench btree_benchmark

# DNA compression benchmarks
cargo bench --features benchmarks -p neuroquantum-core --bench dna_compression

# Quantum algorithm benchmarks
cargo bench --features benchmarks -p neuroquantum-core --bench grover_search
cargo bench --features benchmarks -p neuroquantum-core --bench quantum_annealing

# NEON SIMD optimization benchmarks
cargo bench --features benchmarks -p neuroquantum-core --bench neon_optimization

# Storage benchmarks
cargo bench --features benchmarks -p neuroquantum-core --bench page_storage_benchmark
```

#### Quick Benchmarks (Reduced Samples)

```bash
# Run with fewer samples for faster results
cargo bench --features benchmarks -- --sample-size 10
```

### 6.2 Interpreting Results

#### Understanding Criterion Output

```
B+Tree Sequential Insert/100
                        time:   [1.71 ms 1.73 ms 1.75 ms]
                        change: [-5.99% -5.53% -5.01%] (p = 0.00 < 0.05)
                        Performance has improved.
```

**Key Fields:**
- **time**: `[lower_bound median upper_bound]` — 95% confidence interval
- **change**: Comparison with previous run (if exists)
- **Performance has improved/regressed**: Statistical significance (p < 0.05)

#### Performance Baselines

See [Performance Benchmarks](../reference/benchmarks.md) for detailed baseline metrics.

**Key Performance Indicators:**

| Component | Metric | Baseline (M2 Pro) |
|-----------|--------|-------------------|
| **B+Tree Insert** | 1K elements | 21.15 ms (47.29 Kelem/s) |
| **DNA Compression** | 8 KB data | 21.74 ms (368 KiB/s) |
| **DNA Decompression** | 64 KB data | 9.36 ms (6.68 MiB/s) |
| **NEON SIMD Speedup** | DNA encoding | 4.27x faster than scalar |
| **Grover's Search** | 64 elements | 1.43 µs |
| **Matrix Multiply** | 32×32 | 6.73 µs |
| **Page Allocation** | 1K pages | 13.10 ms (76.34 Kelem/s) |

### 6.3 Comparison with Other Databases

#### Methodology

**Fair Comparison Requirements:**
1. Same hardware (CPU, RAM, disk)
2. Same dataset size and characteristics
3. Same query workload (read/write ratio)
4. Same isolation level (ACID guarantees)
5. Warm cache state

#### Sample Benchmark Workload

```bash
# sysbench-style benchmark
# 1M rows, 8 threads, 60 seconds

# NeuroQuantumDB
sysbench --test=oltp --oltp-table-size=1000000 \
  --num-threads=8 --max-time=60 \
  --db-driver=neuroquantum run

# Compare with PostgreSQL, MySQL, SQLite
```

#### Interpreting Comparisons

**What to Compare:**
- ✅ Throughput (QPS) for similar query types
- ✅ Latency percentiles (p95, p99) under load
- ✅ Memory footprint for similar datasets
- ✅ Storage efficiency (compression ratios)

**What NOT to Compare:**
- ❌ Different workloads (OLTP vs OLAP)
- ❌ Different data types (time-series vs relational)
- ❌ Different features (e.g., NeuroQuantumDB's DNA compression vs standard compression)

#### NeuroQuantumDB Strengths

- **DNA Compression:** 4:1 compression ratio for bioinformatics data
- **ARM64 Optimization:** NEON SIMD provides 4.27x speedup
- **Quantum Algorithms:** Efficient for specific search patterns (Grover's algorithm)
- **Edge Computing:** Low power consumption (< 1.5W) on Raspberry Pi 4
- **Neuromorphic Features:** Adaptive learning and pattern recognition

#### Trade-offs

- **Maturity:** PostgreSQL/MySQL have decades of optimization
- **Ecosystem:** Fewer third-party tools and integrations
- **Use Case:** Optimized for edge computing and specialized workloads

### 6.4 Custom Benchmarking

#### Application-Specific Benchmarks

Create benchmarks that match your actual workload:

```rust
// benches/custom_workload.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use neuroquantum_core::Database;

fn my_workload_benchmark(c: &mut Criterion) {
    let db = Database::new("test_db").unwrap();
    
    c.bench_function("my_workload", |b| {
        b.iter(|| {
            // Your typical query pattern
            db.execute(black_box("SELECT * FROM users WHERE active = true"))
        });
    });
}

criterion_group!(benches, my_workload_benchmark);
criterion_main!(benches);
```

```bash
# Run custom benchmark
cargo bench --bench custom_workload
```

#### Load Testing

Use tools like `wrk` or `k6` for HTTP API load testing:

```bash
# Install wrk
sudo apt-get install wrk

# Load test REST API
wrk -t8 -c100 -d30s --latency \
  -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/api/v1/query

# Example output:
# Latency Distribution
#   50%   12.34ms
#   75%   23.45ms
#   90%   34.56ms
#   99%   89.12ms
```

---

## Best Practices Summary

### Configuration Checklist

- ✅ Set `buffer_pool_size_mb` to 25-40% of RAM
- ✅ Configure `workers` to match CPU cores
- ✅ Enable WAL for durability (`wal_enabled = true`)
- ✅ Tune `max_connections` based on expected load
- ✅ Enable DNA compression for bioinformatics data
- ✅ Enable NEON SIMD on ARM64 (`arm64_neon_enabled = true`)

### Query Optimization Checklist

- ✅ Use `EXPLAIN` to analyze query plans
- ✅ Create indexes on frequently queried columns
- ✅ Avoid `SELECT *` — fetch only needed columns
- ✅ Use batch operations for bulk inserts/updates
- ✅ Implement application-level caching
- ✅ Limit result sets with `LIMIT` clauses

### Hardware Checklist

- ✅ Use SSD or NVMe storage (never HDD for production)
- ✅ Separate WAL and data on different disks if possible
- ✅ Provision adequate RAM (8GB+ for production)
- ✅ Match worker threads to CPU cores

### Monitoring Checklist

- ✅ Set up Prometheus metrics scraping
- ✅ Configure Grafana dashboards
- ✅ Enable alerting for critical metrics
- ✅ Monitor buffer pool hit rate (target > 95%)
- ✅ Track query latency percentiles (p95, p99)
- ✅ Set up log aggregation

### Maintenance Checklist

- ✅ Regular benchmark regression tests
- ✅ Archive old WAL files
- ✅ Monitor disk space usage
- ✅ Review slow query logs weekly
- ✅ Update indexes based on query patterns
- ✅ Keep NeuroQuantumDB updated

---

## Getting Help

If you encounter performance issues not covered in this guide:

1. **Search Documentation:** Check [troubleshooting guide](troubleshooting.md)
2. **GitHub Discussions:** Ask in [community discussions](https://github.com/twoh-me/NeuroQuantumDB/discussions)
3. **Open Issue:** Report performance bugs at [GitHub Issues](https://github.com/twoh-me/NeuroQuantumDB/issues)
4. **Share Metrics:** Include Prometheus metrics and EXPLAIN output when asking for help

---

## See Also

- [Configuration Guide](configuration.md) — Detailed configuration reference
- [Monitoring Guide](monitoring.md) — Prometheus and Grafana setup
- [Troubleshooting Guide](troubleshooting.md) — Common issues and solutions
- [Performance Benchmarks](../reference/benchmarks.md) — Baseline performance metrics
- [Architecture](../developer-guide/architecture.md) — Understanding internal components
