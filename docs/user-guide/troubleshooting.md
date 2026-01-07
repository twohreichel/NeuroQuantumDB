# Troubleshooting

This comprehensive guide helps you diagnose and resolve common issues with NeuroQuantumDB.

## 1. Common Errors and Solutions

### Connection Refused

**Symptom:** `Connection refused` or `Failed to connect to localhost:8080`

**Causes:**
- Server is not running
- Wrong host/port configuration
- Firewall blocking connection

**Solutions:**
```bash
# Check if server is running
ps aux | grep neuroquantum-api

# Check port availability
netstat -tuln | grep 8080
# or
lsof -i :8080

# Start server if not running
./neuroquantum-api --config config/prod.toml

# Verify server is listening
curl http://localhost:8080/health
```

---

### Server Won't Start (Address Already in Use)

**Symptom:** `Address already in use` error during startup

**Solutions:**
```bash
# Find process using port
lsof -i :8080
netstat -tulnp | grep 8080

# Kill the process
kill -9 <PID>

# Or change port in config
# config/prod.toml
[server]
port = 8081
```

---

### Out of Memory

**Symptom:** Server crashes, `OOM` errors, or extreme slowness

**Diagnosis:**
```bash
# Check memory usage
free -h
top -o %MEM

# Check NeuroQuantumDB memory
ps aux | grep neuroquantum-api

# Check buffer pool metrics
curl http://localhost:8080/metrics | grep buffer_pool
```

**Solutions:**
```toml
# config/prod.toml - Reduce buffer pool size
[storage]
buffer_pool_size_mb = 128  # Default: 256

# Enable DNA compression to reduce memory
[compression]
dna_enabled = true
compression_level = 6

# Limit concurrent connections
[server]
max_connections = 50  # Default: 100
```

**Additional steps:**
```bash
# Set memory limit with systemd
# /etc/systemd/system/neuroquantumdb.service
[Service]
MemoryLimit=512M

# Or use Docker limits
docker run -m 512m neuroquantumdb/neuroquantum-api
```

---

### Transaction Deadlock

**Symptom:** `TXN_DEADLOCK` error, transactions hanging

**Diagnosis:**
```bash
# Check for long-running transactions
curl http://localhost:8080/api/v1/admin/transactions

# Enable deadlock logging
RUST_LOG=neuroquantum_core::transaction=debug ./neuroquantum-api
```

**Solutions:**
```sql
-- Always acquire locks in same order
BEGIN TRANSACTION;
SELECT * FROM table_a WHERE id = 1 FOR UPDATE;
SELECT * FROM table_b WHERE id = 2 FOR UPDATE;
COMMIT;

-- Set transaction timeout
SET transaction_timeout = '30s';
```

```toml
# config/prod.toml
[transaction]
deadlock_detection_interval_ms = 100
max_transaction_duration_secs = 30
```

---

### Query Timeout

**Symptom:** `QUERY_TIMEOUT` error, queries never complete

**Diagnosis:**
```sql
-- Check query execution plan
EXPLAIN SELECT * FROM large_table WHERE complex_condition;

-- Enable query logging
SET log_queries = true;
```

**Solutions:**
```toml
# Increase query timeout
[query]
timeout_seconds = 60  # Default: 30

# Increase statement timeout
[query]
statement_timeout_seconds = 120
```

```sql
-- Create missing indexes
CREATE INDEX idx_user_email ON users(email);
CREATE INDEX idx_order_date ON orders(created_at);

-- Optimize query
-- Before: Full table scan
SELECT * FROM users WHERE LOWER(email) = 'user@example.com';

-- After: Index usage
SELECT * FROM users WHERE email = 'user@example.com';
```

---

### Authentication Failed

**Symptom:** `401 Unauthorized` or `AUTH_INVALID_TOKEN`

**Diagnosis:**
```bash
# Verify token format
echo $TOKEN

# Should be: Bearer eyJhbGc...

# Test authentication endpoint
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/auth/verify

# Check JWT secret configuration
grep jwt_secret config/prod.toml
```

**Solutions:**
```bash
# Generate new JWT secret
neuroquantum-api generate-jwt-secret

# Set in configuration
export NQDB_JWT_SECRET="your-generated-secret"

# Verify token expiration
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/auth/info

# Generate new token
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'
```

---

### Permission Denied

**Symptom:** `AUTH_INSUFFICIENT_PERMISSIONS` or `403 Forbidden`

**Solutions:**
```bash
# Check user permissions
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/auth/permissions

# Grant required permissions (as admin)
curl -X POST http://localhost:8080/api/v1/admin/users/123/permissions \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"permissions": ["read", "write", "admin"]}'
```

---

### Disk Full

**Symptom:** `STORAGE_DISK_FULL` error, write operations fail

**Diagnosis:**
```bash
# Check disk usage
df -h /var/lib/neuroquantumdb

# Check data directory size
du -sh /var/lib/neuroquantumdb/*

# Check WAL size
du -sh /var/lib/neuroquantumdb/wal
```

**Solutions:**
```bash
# Archive old WAL files
neuroquantum-api wal-archive --before 2024-01-01

# Enable WAL compression
# config/prod.toml
[storage]
wal_compression = true

# Clean up old backups
rm -rf /var/lib/neuroquantumdb/backups/old-*

# Vacuum database
neuroquantum-api vacuum --full
```

---

### Data Corruption

**Symptom:** `STORAGE_CORRUPTED` error, checksum failures

**Diagnosis:**
```bash
# Run integrity check
neuroquantum-api check --verbose

# Check specific table
neuroquantum-api check --table users

# Review logs for corruption patterns
grep -i corrupt /var/log/neuroquantumdb/app.log
```

**Solutions:**
```bash
# Restore from backup (if available)
neuroquantum-api restore --from /backups/latest.nqdb

# Attempt repair (may lose data)
neuroquantum-api repair --table users --force

# Rebuild indexes
neuroquantum-api reindex --all
```

---

### WAL Write Failed

**Symptom:** `STORAGE_WAL_ERROR`, transactions fail to commit

**Diagnosis:**
```bash
# Check WAL directory permissions
ls -la /var/lib/neuroquantumdb/wal

# Check disk I/O
iostat -x 1 10

# Verify WAL configuration
grep wal config/prod.toml
```

**Solutions:**
```bash
# Fix permissions
chown -R neuroquantum:neuroquantum /var/lib/neuroquantumdb/wal
chmod 700 /var/lib/neuroquantumdb/wal

# Increase WAL buffer
# config/prod.toml
[storage]
wal_buffer_size_mb = 16  # Default: 8

# Sync mode for reliability
[storage]
wal_sync_mode = "fsync"  # Options: fsync, fdatasync, none
```

---

### Lock Timeout

**Symptom:** `STORAGE_LOCK_TIMEOUT`, operations hang waiting for locks

**Diagnosis:**
```bash
# Monitor lock contention
curl http://localhost:8080/api/v1/admin/locks

# Check for blocking queries
curl http://localhost:8080/api/v1/admin/blocked-queries
```

**Solutions:**
```toml
# Increase lock timeout
[storage]
lock_timeout_ms = 5000  # Default: 1000

# Reduce lock contention
[storage]
lock_granularity = "row"  # Options: table, page, row
```

---

### High CPU Usage

**Symptom:** CPU usage constantly > 80%

**Diagnosis:**
```bash
# Profile CPU usage
top -p $(pgrep neuroquantum-api)

# Get CPU metrics
curl http://localhost:8080/metrics | grep cpu

# Enable query profiling
RUST_LOG=neuroquantum_core::query=trace ./neuroquantum-api
```

**Solutions:**
```bash
# Identify slow queries
neuroquantum-api slow-queries --min-duration 1s

# Optimize queries with indexes
# Add query result caching
# config/prod.toml
[cache]
query_cache_size_mb = 64
query_cache_ttl_secs = 300
```

---

### Network Timeout

**Symptom:** Clients timeout waiting for responses

**Diagnosis:**
```bash
# Test network latency
ping -c 10 database-host

# Check network metrics
curl http://localhost:8080/metrics | grep network

# Monitor connection state
netstat -an | grep 8080
```

**Solutions:**
```toml
# Increase timeouts
[server]
request_timeout_secs = 60
keep_alive_timeout_secs = 75

# Enable TCP keepalive
[server]
tcp_keepalive_secs = 60
```

---

### Backup Failed

**Symptom:** Backup operations fail or timeout

**Diagnosis:**
```bash
# Check backup status
neuroquantum-api backup-status

# Test backup manually
neuroquantum-api backup --dest /tmp/test-backup.nqdb

# Check disk space
df -h /var/backups
```

**Solutions:**
```bash
# Increase backup timeout
neuroquantum-api backup --timeout 3600 \
  --dest /backups/$(date +%Y%m%d).nqdb

# Use incremental backups
neuroquantum-api backup --incremental \
  --base /backups/base.nqdb \
  --dest /backups/incr-$(date +%Y%m%d).nqdb

# Schedule backups during low-traffic periods
# crontab
0 2 * * * /usr/bin/neuroquantum-api backup --dest /backups/daily.nqdb
```

---

### Restore Failed

**Symptom:** Cannot restore from backup

**Diagnosis:**
```bash
# Verify backup integrity
neuroquantum-api verify-backup /backups/latest.nqdb

# Check backup format
file /backups/latest.nqdb

# Review restore logs
tail -f /var/log/neuroquantumdb/restore.log
```

**Solutions:**
```bash
# Stop server before restore
systemctl stop neuroquantumdb

# Restore with force flag
neuroquantum-api restore --force \
  --from /backups/latest.nqdb \
  --to /var/lib/neuroquantumdb

# Verify after restore
neuroquantum-api check --verbose

# Restart server
systemctl start neuroquantumdb
```

---

### Index Corruption

**Symptom:** Query results inconsistent, `INDEX_CORRUPTED` errors

**Solutions:**
```bash
# Reindex specific table
neuroquantum-api reindex --table users

# Reindex all tables
neuroquantum-api reindex --all

# Drop and recreate index
neuroquantum-api drop-index idx_user_email
neuroquantum-api create-index idx_user_email ON users(email)
```

---

### Service Won't Stop

**Symptom:** `systemctl stop` hangs, process won't terminate

**Solutions:**
```bash
# Check for stuck transactions
curl http://localhost:8080/api/v1/admin/transactions

# Force shutdown (last resort)
kill -TERM $(pgrep neuroquantum-api)

# Wait 30 seconds, then force kill
sleep 30
kill -KILL $(pgrep neuroquantum-api)

# Clean up stale PID files
rm -f /var/run/neuroquantumdb.pid
```

---

### Configuration Error

**Symptom:** `CONFIG_ERROR`, server fails to start

**Diagnosis:**
```bash
# Validate configuration
neuroquantum-api validate-config --config config/prod.toml

# Check syntax
grep -v '^#' config/prod.toml | grep -v '^$'

# Test with default config
neuroquantum-api --config /dev/null
```

**Solutions:**
```bash
# Use example configuration
cp config/prod.toml.example config/prod.toml

# Check environment variables
env | grep NQDB_

# Override with environment
export NQDB_PORT=8080
export NQDB_HOST=0.0.0.0
./neuroquantum-api
```

---

### SSL/TLS Errors

**Symptom:** Certificate errors, handshake failures

**Diagnosis:**
```bash
# Test SSL connection
openssl s_client -connect localhost:8443

# Verify certificate
openssl x509 -in /etc/neuroquantumdb/cert.pem -text -noout

# Check certificate expiration
openssl x509 -in /etc/neuroquantumdb/cert.pem -noout -enddate
```

**Solutions:**
```bash
# Generate self-signed certificate (dev only)
openssl req -x509 -newkey rsa:4096 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes

# Configure TLS
# config/prod.toml
[server.tls]
enabled = true
cert_path = "/etc/neuroquantumdb/cert.pem"
key_path = "/etc/neuroquantumdb/key.pem"

# Use Let's Encrypt (production)
certbot certonly --standalone -d db.example.com
```

---

### Migration Failed

**Symptom:** Schema migration errors

**Diagnosis:**
```bash
# Check migration status
neuroquantum-api migration-status

# List pending migrations
neuroquantum-api migration-list --pending

# Review migration logs
tail -f /var/log/neuroquantumdb/migration.log
```

**Solutions:**
```bash
# Rollback failed migration
neuroquantum-api migration-rollback

# Apply migrations one by one
neuroquantum-api migration-apply --version 001

# Force migration (careful!)
neuroquantum-api migration-apply --force
```

---

### Rate Limit Exceeded

**Symptom:** `RATE_LIMIT_EXCEEDED` errors, 429 responses

**Solutions:**
```toml
# Increase rate limits
[security]
rate_limit_requests = 1000  # Default: 100
rate_limit_window_secs = 60

# Whitelist trusted IPs
[security]
rate_limit_whitelist = ["10.0.0.0/8", "192.168.1.100"]
```

---

### Quantum Search Timeout

**Symptom:** Quantum search queries timeout or fail

**Diagnosis:**
```bash
# Check quantum processor status
curl http://localhost:8080/api/v1/admin/quantum/status

# Monitor quantum metrics
curl http://localhost:8080/metrics | grep quantum
```

**Solutions:**
```toml
# Adjust quantum search parameters
[quantum]
max_iterations = 1000  # Default: 500
search_timeout_ms = 5000  # Default: 2000

# Reduce search space
[quantum]
vector_dimensions = 128  # Default: 256
```

---

### DNA Compression Failed

**Symptom:** Compression errors, degraded performance

**Solutions:**
```toml
# Adjust compression settings
[compression]
dna_enabled = true
compression_level = 4  # Lower = faster, less compression

# Disable for specific tables
# In QSQL
CREATE TABLE large_data (
  id INTEGER PRIMARY KEY,
  data TEXT COMPRESSION NONE
);
```

---

## 2. Log Analysis

### Configuring Log Levels

```bash
# Environment variable (most common)
export RUST_LOG=info

# Module-specific logging
export RUST_LOG=neuroquantum_core=debug,neuroquantum_api=info

# Trace level for all modules (verbose)
export RUST_LOG=trace

# Multiple modules with different levels
export RUST_LOG=neuroquantum_core::storage=trace,neuroquantum_core::query=debug,info
```

**Configuration file:**
```toml
# config/prod.toml
[logging]
level = "info"  # Options: error, warn, info, debug, trace
format = "json"  # Options: json, plain
output = "/var/log/neuroquantumdb/app.log"

# Module-specific levels
[logging.modules]
"neuroquantum_core::storage" = "debug"
"neuroquantum_core::transaction" = "trace"
```

### Understanding Important Log Messages

**Startup Messages:**
```
INFO neuroquantum_api: Server starting version=1.0.0
INFO neuroquantum_core::storage: Buffer pool initialized size_mb=256
INFO neuroquantum_core::wal: WAL recovery completed records=1523
INFO neuroquantum_api: Server listening address=0.0.0.0:8080
```

**Error Messages:**
```
ERROR neuroquantum_core::storage: WAL write failed error="disk full"
ERROR neuroquantum_core::transaction: Deadlock detected txn_id=12345
ERROR neuroquantum_api: Authentication failed user=unknown reason="invalid token"
WARN neuroquantum_core::query: Slow query detected duration_ms=5234
```

**Performance Warnings:**
```
WARN neuroquantum_core::storage: Buffer pool hit ratio low ratio=0.45
WARN neuroquantum_core: High memory usage used_mb=1024 total_mb=1024
WARN neuroquantum_core::query: Full table scan detected table=users
```

**Transaction Messages:**
```
DEBUG neuroquantum_core::transaction: Transaction started txn_id=12345
DEBUG neuroquantum_core::transaction: Acquiring lock table=users mode=exclusive
TRACE neuroquantum_core::transaction: Lock acquired duration_ms=2
INFO neuroquantum_core::transaction: Transaction committed txn_id=12345 duration_ms=45
```

### Log Rotation Setup

**Using logrotate (Linux):**
```bash
# /etc/logrotate.d/neuroquantumdb
/var/log/neuroquantumdb/*.log {
    daily
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 neuroquantum neuroquantum
    sharedscripts
    postrotate
        systemctl reload neuroquantumdb
    endscript
}
```

**Built-in rotation:**
```toml
# config/prod.toml
[logging.rotation]
enabled = true
max_size_mb = 100
max_age_days = 30
max_backups = 10
compress = true
```

### Log Aggregation

**Filebeat configuration:**
```yaml
# filebeat.yml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/neuroquantumdb/*.log
    json.keys_under_root: true
    json.add_error_key: true
    fields:
      service: neuroquantumdb
      environment: production

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "neuroquantumdb-%{+yyyy.MM.dd}"
```

**Fluentd configuration:**
```xml
# fluent.conf
<source>
  @type tail
  path /var/log/neuroquantumdb/app.log
  pos_file /var/log/td-agent/neuroquantumdb.pos
  tag neuroquantumdb
  <parse>
    @type json
    time_key timestamp
    time_format %Y-%m-%dT%H:%M:%S.%NZ
  </parse>
</source>

<match neuroquantumdb>
  @type elasticsearch
  host elasticsearch
  port 9200
  index_name neuroquantumdb
  type_name _doc
</match>
```

**Structured logging queries:**
```bash
# Find all errors in last hour
jq 'select(.level == "ERROR" and .timestamp > "2024-01-07T10:00:00Z")' \
  /var/log/neuroquantumdb/app.log

# Count errors by type
jq -s 'group_by(.error_type) | map({type: .[0].error_type, count: length})' \
  /var/log/neuroquantumdb/app.log

# Find slow queries
jq 'select(.duration_ms > 1000) | {query: .query, duration: .duration_ms}' \
  /var/log/neuroquantumdb/app.log
```

---

## 3. Diagnostic Tools

### Health Check Endpoints

**Basic health check:**
```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "storage": {
    "status": "ok",
    "used_bytes": 1073741824,
    "available_bytes": 5368709120
  },
  "connections": {
    "active": 15,
    "max": 100
  }
}
```

**Detailed health check:**
```bash
curl http://localhost:8080/api/v1/admin/health/detailed
```

**Response:**
```json
{
  "status": "healthy",
  "checks": {
    "storage": "ok",
    "wal": "ok",
    "replication": "ok",
    "cluster": "degraded"
  },
  "metrics": {
    "query_rate": 125.5,
    "avg_query_time_ms": 23.4,
    "buffer_pool_hit_ratio": 0.95,
    "active_transactions": 5
  }
}
```

**Component-specific checks:**
```bash
# Check storage
curl http://localhost:8080/api/v1/admin/health/storage

# Check WAL
curl http://localhost:8080/api/v1/admin/health/wal

# Check cluster
curl http://localhost:8080/api/v1/admin/health/cluster
```

### Interpreting Metrics

**Prometheus endpoint:**
```bash
curl http://localhost:8080/metrics
```

**Key metrics to monitor:**

**Query Performance:**
```
# Total queries
nqdb_queries_total{type="select"} 125432

# Query latency percentiles
nqdb_query_duration_seconds{quantile="0.5"} 0.012
nqdb_query_duration_seconds{quantile="0.95"} 0.145
nqdb_query_duration_seconds{quantile="0.99"} 0.523

# Slow queries
nqdb_slow_queries_total 15
```

**Storage Metrics:**
```
# Buffer pool efficiency
nqdb_buffer_pool_hits_total 1234567
nqdb_buffer_pool_misses_total 45678
# Hit ratio = hits / (hits + misses) = 0.964

# Disk usage
nqdb_storage_used_bytes 1073741824
nqdb_storage_available_bytes 5368709120

# WAL activity
nqdb_wal_writes_total 98765
nqdb_wal_sync_duration_seconds 0.003
```

**Connection Metrics:**
```
# Active connections
nqdb_connections_active 25
nqdb_connections_max 100

# Connection errors
nqdb_connection_errors_total{type="timeout"} 5
nqdb_connection_errors_total{type="refused"} 2
```

**Transaction Metrics:**
```
# Active transactions
nqdb_transactions_active 8

# Transaction outcomes
nqdb_transactions_committed_total 45678
nqdb_transactions_aborted_total 234
nqdb_transactions_deadlocked_total 12

# Transaction duration
nqdb_transaction_duration_seconds{quantile="0.95"} 0.089
```

**Resource Metrics:**
```
# Memory usage
nqdb_memory_used_bytes 536870912
nqdb_memory_buffer_pool_bytes 268435456

# CPU usage
nqdb_cpu_seconds_total 1234.56

# I/O operations
nqdb_disk_reads_total 345678
nqdb_disk_writes_total 123456
```

### Activating Debug Mode

**Method 1: Environment variable**
```bash
# Full trace logging
RUST_LOG=trace ./neuroquantum-api

# Specific modules
RUST_LOG=neuroquantum_core::storage=trace,neuroquantum_core::transaction=debug \
  ./neuroquantum-api
```

**Method 2: Configuration file**
```toml
# config/debug.toml
[logging]
level = "trace"

[debug]
enabled = true
query_logging = true
transaction_logging = true
storage_logging = true
```

**Method 3: Runtime toggle (requires admin authentication)**
```bash
# Enable debug mode
curl -X POST http://localhost:8080/api/v1/admin/debug/enable \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# Set log level at runtime
curl -X POST http://localhost:8080/api/v1/admin/logging/level \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"level": "debug"}'

# Disable debug mode
curl -X POST http://localhost:8080/api/v1/admin/debug/disable \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

### Query Profiling

**Enable query profiling:**
```sql
-- Enable for session
SET profile_queries = true;

-- Execute query
SELECT * FROM users WHERE email LIKE '%@example.com%';

-- View profile
SHOW PROFILE;
```

**Profile output:**
```
Query Plan:
  Sequential Scan on users (cost=0..1523.45 rows=234)
    Filter: email LIKE '%@example.com%'
  
Execution Stats:
  Planning Time: 2.34 ms
  Execution Time: 145.67 ms
  Rows Returned: 234
  
Buffer Usage:
  Shared Blocks Hit: 1523
  Shared Blocks Read: 45
  Shared Blocks Written: 0
```

**Command-line profiling:**
```bash
# Profile specific query
neuroquantum-api profile-query \
  --query "SELECT * FROM users WHERE email = 'test@example.com'"

# Profile slow queries
neuroquantum-api profile-slow-queries \
  --min-duration 1000  # milliseconds

# Generate query report
neuroquantum-api query-report \
  --output /tmp/query-report.html \
  --start "2024-01-01" \
  --end "2024-01-07"
```

**Real-time query monitoring:**
```bash
# Watch active queries
watch -n 1 'curl -s http://localhost:8080/api/v1/admin/queries | jq'

# Stream query log
tail -f /var/log/neuroquantumdb/queries.log | jq 'select(.duration_ms > 100)'
```

---

## 4. Recovery Procedures

### Crash Recovery

**Automatic recovery on startup:**
```bash
# Start server (recovery happens automatically)
./neuroquantum-api --config config/prod.toml

# Monitor recovery progress
tail -f /var/log/neuroquantumdb/app.log | grep recovery
```

**Manual recovery:**
```bash
# Check database consistency
neuroquantum-api check --all

# Recover from WAL
neuroquantum-api recover-wal \
  --wal-dir /var/lib/neuroquantumdb/wal \
  --data-dir /var/lib/neuroquantumdb/data

# Verify recovery
neuroquantum-api verify --verbose
```

**Recovery stages:**
```
INFO Crash recovery started
INFO Scanning WAL files found=15
INFO Replaying WAL records records=45678
INFO Rebuilding indexes tables=23
INFO Recovery completed duration_secs=12.3
INFO Database is ready
```

### Repairing Corrupted Data

**Detect corruption:**
```bash
# Run integrity check
neuroquantum-api check --verbose

# Check specific table
neuroquantum-api check --table users

# Check indexes
neuroquantum-api check-indexes --all
```

**Repair procedures:**

**Option 1: Rebuild from WAL**
```bash
# Stop server
systemctl stop neuroquantumdb

# Backup current data
cp -r /var/lib/neuroquantumdb/data /var/lib/neuroquantumdb/data.backup

# Rebuild from WAL
neuroquantum-api rebuild \
  --wal-dir /var/lib/neuroquantumdb/wal \
  --data-dir /var/lib/neuroquantumdb/data

# Verify
neuroquantum-api check --all

# Start server
systemctl start neuroquantumdb
```

**Option 2: Restore from backup**
```bash
# Stop server
systemctl stop neuroquantumdb

# Restore from backup
neuroquantum-api restore \
  --from /backups/latest.nqdb \
  --to /var/lib/neuroquantumdb

# Apply WAL logs since backup
neuroquantum-api apply-wal \
  --wal-dir /var/lib/neuroquantumdb/wal \
  --since-backup

# Start server
systemctl start neuroquantumdb
```

**Option 3: Repair in-place (may lose data)**
```bash
# Attempt automatic repair
neuroquantum-api repair --all --auto

# Manual repair with options
neuroquantum-api repair \
  --table users \
  --fix-checksums \
  --rebuild-indexes \
  --vacuum

# Export salvageable data
neuroquantum-api export-data \
  --table users \
  --output /tmp/users-recovered.csv \
  --skip-corrupted
```

### Restoring from Backup

**Full restore:**
```bash
# Stop database
systemctl stop neuroquantumdb

# Restore backup
neuroquantum-api restore \
  --from /backups/2024-01-07.nqdb \
  --to /var/lib/neuroquantumdb \
  --force

# Verify integrity
neuroquantum-api check --all

# Start database
systemctl start neuroquantumdb
```

**Point-in-time recovery (PITR):**
```bash
# Restore base backup
neuroquantum-api restore \
  --from /backups/base-2024-01-01.nqdb \
  --to /var/lib/neuroquantumdb

# Replay WAL to specific timestamp
neuroquantum-api replay-wal \
  --wal-dir /backups/wal \
  --until "2024-01-07T12:30:00Z" \
  --data-dir /var/lib/neuroquantumdb

# Verify and start
neuroquantum-api check --all
systemctl start neuroquantumdb
```

**Incremental restore:**
```bash
# Restore base backup
neuroquantum-api restore \
  --from /backups/base.nqdb \
  --to /var/lib/neuroquantumdb

# Apply incremental backups in order
neuroquantum-api restore \
  --from /backups/incr-2024-01-02.nqdb \
  --to /var/lib/neuroquantumdb \
  --incremental

neuroquantum-api restore \
  --from /backups/incr-2024-01-03.nqdb \
  --to /var/lib/neuroquantumdb \
  --incremental
```

**Selective restore:**
```bash
# Restore specific tables
neuroquantum-api restore \
  --from /backups/latest.nqdb \
  --tables users,orders \
  --to /var/lib/neuroquantumdb

# Restore specific database
neuroquantum-api restore \
  --from /backups/latest.nqdb \
  --database production \
  --to /var/lib/neuroquantumdb
```

### Cluster Recovery

⚠️ **Note:** Cluster features are experimental. For production, use single-node configuration.

**Recover single node:**
```bash
# Stop failed node
ssh node1 "systemctl stop neuroquantumdb"

# Sync from healthy node
neuroquantum-api sync-node \
  --from node2:9000 \
  --to node1 \
  --full

# Start node
ssh node1 "systemctl start neuroquantumdb"

# Verify cluster status
neuroquantum-api cluster-status
```

**Recover from quorum loss:**
```bash
# Force promote a node to leader (careful!)
neuroquantum-api force-leader \
  --node node2 \
  --cluster-id cluster-prod

# Restart other nodes
for node in node1 node3; do
  ssh $node "systemctl restart neuroquantumdb"
done

# Wait for cluster to stabilize
neuroquantum-api wait-for-cluster --timeout 300
```

**Rebuild cluster from backup:**
```bash
# Restore backup on all nodes
for node in node1 node2 node3; do
  ssh $node "systemctl stop neuroquantumdb"
  ssh $node "neuroquantum-api restore --from /backups/cluster.nqdb"
done

# Start bootstrap node first
ssh node1 "systemctl start neuroquantumdb"
sleep 10

# Start remaining nodes
ssh node2 "systemctl start neuroquantumdb"
ssh node3 "systemctl start neuroquantumdb"

# Verify cluster
neuroquantum-api cluster-status --wait-for-healthy
```

---

## 5. Performance Problems

### Identifying Slow Queries

**Enable slow query logging:**
```toml
# config/prod.toml
[query]
log_slow_queries = true
slow_query_threshold_ms = 1000
```

**Find slow queries:**
```bash
# View slow query log
tail -f /var/log/neuroquantumdb/slow-queries.log

# Get slow query report
neuroquantum-api slow-query-report \
  --since "24h" \
  --output /tmp/slow-queries.html

# Top 10 slowest queries
curl http://localhost:8080/api/v1/admin/slow-queries?limit=10 | jq
```

**Analyze slow query:**
```sql
-- Get execution plan
EXPLAIN ANALYZE SELECT * FROM users 
WHERE email LIKE '%@example.com%';

-- Output shows:
-- Sequential Scan on users (cost=0..1523.45) (actual time=0.123..145.678)
--   Filter: email LIKE '%@example.com%'
--   Rows Removed by Filter: 9766

-- Solution: Create index for prefix searches
CREATE INDEX idx_user_email_gin ON users USING GIN(email);
```

**Optimization strategies:**
```sql
-- 1. Add indexes
CREATE INDEX idx_user_created ON users(created_at);
CREATE INDEX idx_order_status ON orders(status);

-- 2. Use covering indexes
CREATE INDEX idx_user_email_name ON users(email, name);

-- 3. Optimize JOIN queries
-- Before: Multiple table scans
SELECT u.*, o.* FROM users u, orders o WHERE u.id = o.user_id;

-- After: Explicit JOIN with indexes
SELECT u.*, o.* FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE u.created_at > '2024-01-01';

-- 4. Limit result sets
SELECT * FROM large_table LIMIT 100;

-- 5. Use query cache
-- config/prod.toml
[cache]
query_cache_size_mb = 64
```

### Debugging High CPU/Memory Usage

**Monitor resource usage:**
```bash
# Real-time monitoring
top -p $(pgrep neuroquantum-api)

# Detailed CPU profiling
perf record -p $(pgrep neuroquantum-api) -g -- sleep 60
perf report

# Memory profiling
valgrind --tool=massif ./neuroquantum-api --config config/prod.toml

# Get metrics
curl http://localhost:8080/metrics | grep -E "(cpu|memory)"
```

**High CPU causes and solutions:**

**1. Too many concurrent queries:**
```toml
# Limit concurrent queries
[query]
max_concurrent_queries = 50

# Use connection pooling
[server]
max_connections = 100
connection_queue_size = 50
```

**2. Missing indexes:**
```bash
# Find queries doing full table scans
neuroquantum-api find-missing-indexes

# Output suggests:
# CREATE INDEX idx_users_email ON users(email);
```

**3. Complex queries:**
```sql
-- Simplify queries, break into smaller pieces
-- Use materialized views for expensive aggregations
CREATE MATERIALIZED VIEW user_stats AS
SELECT user_id, COUNT(*) as order_count, SUM(total) as total_spent
FROM orders
GROUP BY user_id;

-- Refresh periodically instead of computing every time
REFRESH MATERIALIZED VIEW user_stats;
```

**High memory causes and solutions:**

**1. Large buffer pool:**
```toml
# Reduce buffer pool
[storage]
buffer_pool_size_mb = 128  # Adjust based on available RAM
```

**2. Memory leaks (check logs):**
```bash
# Monitor memory over time
while true; do
  ps aux | grep neuroquantum-api | awk '{print $6}'
  sleep 60
done

# If increasing, may need to restart periodically
# /etc/systemd/system/neuroquantumdb.service
[Service]
Restart=always
RuntimeMaxSec=86400  # Restart daily
```

**3. Large result sets:**
```sql
-- Use pagination
SELECT * FROM large_table LIMIT 100 OFFSET 0;

-- Use cursor for large exports
DECLARE cur CURSOR FOR SELECT * FROM large_table;
FETCH 1000 FROM cur;
```

### Disk I/O Bottlenecks

**Diagnose I/O issues:**
```bash
# Monitor disk I/O
iostat -x 1 10

# Check for high await times (> 10ms is concerning)
# Device   r/s   w/s   await   util%
# sda      123   456   45.67   95%

# Monitor NeuroQuantumDB I/O
iotop -p $(pgrep neuroquantum-api)

# Check disk metrics
curl http://localhost:8080/metrics | grep disk
```

**Solutions:**

**1. Use faster storage:**
```bash
# Move data to SSD
rsync -av /var/lib/neuroquantumdb/ /mnt/ssd/neuroquantumdb/

# Update configuration
[storage]
data_path = "/mnt/ssd/neuroquantumdb"
```

**2. Optimize WAL:**
```toml
# Increase WAL buffer
[storage]
wal_buffer_size_mb = 32

# Reduce sync frequency (less durable but faster)
[storage]
wal_sync_mode = "fdatasync"  # or "none" for testing only

# Place WAL on separate disk
[storage]
wal_path = "/mnt/fast-disk/wal"
```

**3. Increase buffer pool:**
```toml
# More buffer pool = less disk I/O
[storage]
buffer_pool_size_mb = 512  # If you have RAM available
```

**4. Enable compression:**
```toml
# Reduce I/O through compression
[compression]
dna_enabled = true
compression_level = 6
```

**5. Optimize queries:**
```sql
-- Reduce I/O by selecting only needed columns
-- Bad: SELECT *
SELECT * FROM large_table;

-- Good: SELECT specific columns
SELECT id, name, email FROM large_table;
```

### Network Latency Issues

**Diagnose network problems:**
```bash
# Test latency to database
ping -c 100 database-host

# Trace route
traceroute database-host

# Test bandwidth
iperf3 -c database-host

# Monitor network metrics
curl http://localhost:8080/metrics | grep network

# Check for packet loss
netstat -s | grep -i error
```

**Solutions:**

**1. Use connection pooling:**
```bash
# Client-side pooling reduces connection overhead
# Example with connection pool library
```

**2. Enable compression:**
```toml
# Compress network traffic
[server]
enable_compression = true
compression_level = 6
```

**3. Increase timeouts:**
```toml
# Allow for higher latency
[server]
request_timeout_secs = 60
keep_alive_timeout_secs = 75
```

**4. Batch operations:**
```sql
-- Instead of multiple single inserts
INSERT INTO users (name, email) VALUES ('User 1', 'user1@example.com');
INSERT INTO users (name, email) VALUES ('User 2', 'user2@example.com');

-- Batch insert
INSERT INTO users (name, email) VALUES 
  ('User 1', 'user1@example.com'),
  ('User 2', 'user2@example.com'),
  ('User 3', 'user3@example.com');
```

**5. Use local replica:**
```toml
# Set up read replica closer to application
[replication]
enabled = true
replica_nodes = ["local-replica:8080"]

# Route read queries to replica
[query]
read_replica_enabled = true
```

---

## 6. Cluster-Specific Issues

⚠️ **Warning:** Cluster mode is experimental. These troubleshooting steps are for development/testing only.

### Node Not Reachable

**Symptom:** Cluster health shows node as unreachable

**Diagnosis:**
```bash
# Check cluster status
neuroquantum-api cluster-status

# Ping node directly
ping node2

# Test node port
telnet node2 9000

# Check node health
curl http://node2:8080/health

# Review cluster logs
tail -f /var/log/neuroquantumdb/cluster.log
```

**Solutions:**

**1. Network connectivity:**
```bash
# Check firewall
sudo ufw status
sudo firewall-cmd --list-all

# Open cluster port
sudo ufw allow 9000/tcp
sudo firewall-cmd --add-port=9000/tcp --permanent

# Verify DNS resolution
nslookup node2
host node2
```

**2. Node is down:**
```bash
# SSH to node and check service
ssh node2 "systemctl status neuroquantumdb"

# Start if stopped
ssh node2 "systemctl start neuroquantumdb"

# Check for errors
ssh node2 "journalctl -u neuroquantumdb -n 100"
```

**3. Configuration mismatch:**
```bash
# Verify cluster configuration matches
ssh node2 "cat config/prod.toml | grep -A 10 '\[cluster\]'"

# Check node ID is unique
grep node_id config/prod.toml
ssh node2 "grep node_id config/prod.toml"
```

### Split Brain Situation

**Symptom:** Multiple nodes think they are leader

**Diagnosis:**
```bash
# Check leader on each node
for node in node1 node2 node3; do
  echo "=== $node ==="
  curl -s http://$node:8080/api/v1/admin/cluster/leader
done

# Review Raft state
neuroquantum-api cluster-debug --show-raft
```

**Solutions:**

**1. Force new leader election:**
```bash
# Stop all nodes
for node in node1 node2 node3; do
  ssh $node "systemctl stop neuroquantumdb"
done

# Start nodes one by one
ssh node1 "systemctl start neuroquantumdb"
sleep 30

ssh node2 "systemctl start neuroquantumdb"
sleep 30

ssh node3 "systemctl start neuroquantumdb"

# Verify cluster converged
neuroquantum-api cluster-status --wait-for-leader
```

**2. Quorum restore:**
```bash
# If you have majority of nodes healthy
neuroquantum-api force-quorum \
  --nodes node1,node2 \
  --cluster-id cluster-prod

# Remove failed node from cluster
neuroquantum-api remove-node --node node3

# Add node back after fixing
neuroquantum-api add-node \
  --node node3 \
  --addr node3:9000
```

**Prevention:**
```toml
# Ensure proper network settings
[cluster]
heartbeat_interval_ms = 100
election_timeout_ms = 1000
network_timeout_ms = 5000

# Use odd number of nodes (3, 5, 7)
# Never use 2 nodes (can't form majority)
```

### Replication Lag

**Symptom:** Replica data is behind leader

**Diagnosis:**
```bash
# Check replication lag
curl http://localhost:8080/api/v1/admin/replication/lag

# Output:
# {
#   "node2": {"lag_bytes": 104857600, "lag_seconds": 45},
#   "node3": {"lag_bytes": 52428800, "lag_seconds": 23}
# }

# Monitor replication metrics
curl http://localhost:8080/metrics | grep replication
```

**Causes and solutions:**

**1. Network issues:**
```bash
# Test bandwidth between nodes
iperf3 -c node2

# Increase replication timeout
# config/prod.toml
[replication]
sync_timeout_secs = 60
```

**2. Replica overloaded:**
```bash
# Check replica CPU/memory
ssh node2 "top -b -n 1"

# Reduce read traffic to replica
# Route reads to less-loaded replicas
```

**3. Large transactions:**
```toml
# Increase replication buffer
[replication]
buffer_size_mb = 64

# Batch replication
[replication]
batch_size = 1000
batch_timeout_ms = 100
```

**4. Slow disk on replica:**
```bash
# Check I/O on replica
ssh node2 "iostat -x 1 10"

# Move to faster storage
# Consider SSD for replicas
```

**Recovery:**
```bash
# Force full resync (last resort)
neuroquantum-api resync-replica \
  --replica node2 \
  --full

# Or restore from backup
ssh node2 "systemctl stop neuroquantumdb"
ssh node2 "neuroquantum-api restore --from /backups/latest.nqdb"
ssh node2 "systemctl start neuroquantumdb"
```

### Leader Election Problems

**Symptom:** Cluster can't elect leader, writes fail

**Diagnosis:**
```bash
# Check leader status
neuroquantum-api cluster-status | jq '.leader'

# View election logs
grep -i election /var/log/neuroquantumdb/cluster.log

# Check quorum
neuroquantum-api cluster-quorum
```

**Causes and solutions:**

**1. No quorum (majority nodes down):**
```bash
# In 3-node cluster, need 2 nodes minimum

# Check node status
for node in node1 node2 node3; do
  echo "$node: $(curl -s http://$node:8080/health | jq .status)"
done

# Bring up failed nodes
# For emergency, force quorum (data loss risk)
neuroquantum-api force-quorum --nodes node1,node2
```

**2. Network partition:**
```bash
# Nodes can't communicate with each other

# Check network between nodes
for node in node1 node2 node3; do
  echo "Testing from $node:"
  ssh $node "ping -c 3 node1"
  ssh $node "ping -c 3 node2"
  ssh $node "ping -c 3 node3"
done

# Fix network, restart cluster
```

**3. Clock skew:**
```bash
# Check time on all nodes
for node in node1 node2 node3; do
  echo "$node: $(ssh $node date)"
done

# Sync clocks with NTP
for node in node1 node2 node3; do
  ssh $node "sudo ntpdate -s time.nist.gov"
done

# Configure NTP
# /etc/ntp.conf or /etc/systemd/timesyncd.conf
```

**4. Configuration issues:**
```toml
# Ensure election timeouts are appropriate
[cluster]
election_timeout_ms = 1000  # Not too short
heartbeat_interval_ms = 100  # Should be < election_timeout / 3

# Ensure node IDs are unique
[cluster]
node_id = 1  # Must be unique per node
```

---

## 7. FAQ

### General Questions

**Q: How do I check the NeuroQuantumDB version?**
```bash
neuroquantum-api --version
# or
curl http://localhost:8080/api/v1/version
```

**Q: Where are the log files located?**

Default locations:
- Application logs: `/var/log/neuroquantumdb/app.log`
- Slow query logs: `/var/log/neuroquantumdb/slow-queries.log`
- Cluster logs: `/var/log/neuroquantumdb/cluster.log`
- Audit logs: `/var/log/neuroquantumdb/audit.log`

Or configured in `config/prod.toml`:
```toml
[logging]
output = "/custom/path/app.log"
```

**Q: How do I increase memory limits?**
```toml
# config/prod.toml
[storage]
buffer_pool_size_mb = 512  # Adjust based on available RAM
```

**Q: Can I run NeuroQuantumDB on Windows?**

NeuroQuantumDB is optimized for Linux (especially ARM64). Windows support is experimental. Use Docker or WSL2 for best results.

**Q: How do I enable TLS/SSL?**
```toml
# config/prod.toml
[server.tls]
enabled = true
cert_path = "/etc/neuroquantumdb/cert.pem"
key_path = "/etc/neuroquantumdb/key.pem"
```

### Performance Questions

**Q: Why are my queries slow?**

Common causes:
1. Missing indexes - Run `EXPLAIN` to check query plan
2. Full table scans - Create appropriate indexes
3. Large result sets - Add `LIMIT` clauses
4. Low buffer pool hit ratio - Increase `buffer_pool_size_mb`
5. Disk I/O bottleneck - Use SSD storage

**Q: How do I optimize for low memory environments (Raspberry Pi)?**
```toml
# config/low-memory.toml
[storage]
buffer_pool_size_mb = 64
page_size_kb = 4

[compression]
dna_enabled = true
compression_level = 9

[cache]
query_cache_size_mb = 16
```

**Q: What's the recommended buffer pool size?**

Rule of thumb: 25-40% of available RAM
- 1GB RAM → 256-384 MB
- 2GB RAM → 512-768 MB
- 4GB RAM → 1-1.5 GB

**Q: How do I monitor performance?**
```bash
# Prometheus metrics
curl http://localhost:8080/metrics

# Health check
curl http://localhost:8080/health

# Query stats
curl http://localhost:8080/api/v1/admin/query-stats
```

### Backup and Recovery Questions

**Q: How often should I backup?**

Recommended schedule:
- Full backup: Daily during low-traffic hours
- Incremental backup: Every 6 hours
- WAL archiving: Continuous

```bash
# crontab example
0 2 * * * neuroquantum-api backup --dest /backups/daily-$(date +\%Y\%m\%d).nqdb
0 */6 * * * neuroquantum-api backup --incremental --dest /backups/incr-$(date +\%Y\%m\%d-\%H).nqdb
```

**Q: How do I perform point-in-time recovery?**
```bash
# Restore base backup
neuroquantum-api restore --from /backups/base.nqdb

# Replay WAL to specific time
neuroquantum-api replay-wal \
  --until "2024-01-07T12:30:00Z" \
  --wal-dir /backups/wal
```

**Q: Can I backup while the database is running?**

Yes, NeuroQuantumDB supports online backups:
```bash
neuroquantum-api backup --online --dest /backups/live.nqdb
```

### Cluster Questions

**Q: Is cluster mode production-ready?**

⚠️ No, cluster mode is currently in beta. For production use, deploy single-node instances with external replication/backup.

**Q: How many nodes should I have in a cluster?**

- **Development:** 1 node
- **Testing:** 3 nodes (minimum for HA)
- **Production (when stable):** 5 or 7 nodes

Always use odd numbers (3, 5, 7) to ensure quorum.

**Q: What happens if I lose the leader node?**

In a healthy cluster with quorum, a new leader is elected automatically within 1-3 seconds. Writes are briefly paused during election.

**Q: How do I add a node to an existing cluster?**
```bash
# On new node, configure cluster settings
# config/prod.toml
[cluster]
enabled = true
node_id = 4
bind_addr = "0.0.0.0:9000"
peers = ["node1:9000", "node2:9000", "node3:9000"]

# Add node to cluster
neuroquantum-api add-node \
  --node node4 \
  --addr node4:9000 \
  --cluster-id cluster-prod

# Wait for sync
neuroquantum-api cluster-status
```

### Security Questions

**Q: How do I change the JWT secret?**
```bash
# Generate new secret
neuroquantum-api generate-jwt-secret

# Update config
export NQDB_JWT_SECRET="new-secret-here"

# Restart server
systemctl restart neuroquantumdb

# Note: This invalidates all existing tokens
```

**Q: How do I enable audit logging?**
```toml
# config/prod.toml
[audit]
enabled = true
log_file = "/var/log/neuroquantumdb/audit.log"
log_level = "info"
include_queries = true
include_auth = true
```

**Q: How do I restrict access by IP address?**
```toml
# config/prod.toml
[security]
admin_ip_whitelist = ["127.0.0.1", "192.168.1.0/24", "10.0.0.100"]
rate_limit_enabled = true
rate_limit_requests = 100
rate_limit_window_secs = 60
```

### Troubleshooting Questions

**Q: Server starts but immediately crashes**

Check:
1. Disk space: `df -h`
2. Memory: `free -h`
3. Permissions: `ls -la /var/lib/neuroquantumdb`
4. Logs: `tail -n 100 /var/log/neuroquantumdb/app.log`
5. Configuration: `neuroquantum-api validate-config`

**Q: Cannot connect via REST API**

Check:
1. Server is running: `systemctl status neuroquantumdb`
2. Firewall: `sudo ufw status`
3. Port is open: `netstat -tuln | grep 8080`
4. Bind address: Check `[server] host` in config
5. TLS configuration if enabled

**Q: Queries return wrong results**

Potential causes:
1. Data corruption - Run `neuroquantum-api check --all`
2. Index corruption - Run `neuroquantum-api reindex --all`
3. Cache issue - Restart server or clear cache
4. Race condition in transaction - Review transaction isolation levels

**Q: How do I report a bug?**

1. Check [GitHub Issues](https://github.com/neuroquantumdb/neuroquantumdb/issues)
2. Gather information:
   ```bash
   # Version
   neuroquantum-api --version
   
   # System info
   uname -a
   
   # Configuration (redact secrets!)
   cat config/prod.toml
   
   # Recent logs
   tail -n 100 /var/log/neuroquantumdb/app.log
   
   # Metrics
   curl http://localhost:8080/metrics > metrics.txt
   ```
3. Create detailed issue with:
   - Description of problem
   - Steps to reproduce
   - Expected vs actual behavior
   - Version and system info
   - Relevant logs

---

## Getting Help

If you cannot resolve your issue using this guide:

1. **Search Documentation:**
   - [User Guide](../user-guide/)
   - [API Reference](../reference/api.md)
   - [Developer Guide](../developer-guide/)

2. **Community Support:**
   - [GitHub Discussions](https://github.com/neuroquantumdb/neuroquantumdb/discussions)
   - [GitHub Issues](https://github.com/neuroquantumdb/neuroquantumdb/issues)

3. **When Opening an Issue:**
   - NeuroQuantumDB version: `neuroquantum-api --version`
   - Operating system and architecture: `uname -a`
   - Configuration (redact secrets)
   - Steps to reproduce the problem
   - Complete error logs
   - Output of: `curl http://localhost:8080/health`

4. **Emergency Support:**
   - For critical production issues, include `[URGENT]` in issue title
   - Provide full diagnostic dump:
     ```bash
     neuroquantum-api diagnostic-dump --output /tmp/diagnostic.tar.gz
     ```
