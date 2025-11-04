# Configuration

Complete reference for configuring NeuroQuantumDB for different environments and use cases.

## Configuration Files

NeuroQuantumDB uses TOML configuration files located in the `config/` directory:

- **`config/dev.toml`** - Development settings (debug logging, relaxed security)
- **`config/prod.toml`** - Production settings (optimized, hardened)

---

## Configuration Sections

### Server Configuration

```toml
[server]
# Network binding
host = "0.0.0.0"  # "127.0.0.1" for localhost only
port = 8080

# Worker threads (CPU cores)
worker_threads = 4  # Raspberry Pi 4 has 4 cores

# Request limits
max_request_size = 10485760  # 10 MB
request_timeout_seconds = 30

# Keep-alive
keep_alive_seconds = 75
```

**Recommendations:**
- **Development:** `host = "127.0.0.1"` for local-only access
- **Production:** `host = "0.0.0.0"` with firewall configured
- **Worker Threads:** Set to CPU core count (4 for Raspberry Pi 4)

---

### Storage Configuration

```toml
[storage]
# Data directory
data_dir = "/data/neuroquantum"

# Page size (bytes)
page_size = 8192  # 8 KB (optimal for most workloads)

# Buffer pool (pages)
buffer_pool_size = 256  # 256 pages = 2 MB

# Write-Ahead Log
wal_enabled = true
wal_sync_mode = "fsync"  # "fsync" | "fdatasync" | "none"

# Checkpointing
checkpoint_interval_seconds = 300  # 5 minutes
checkpoint_pages = 1000  # Trigger after 1000 dirty pages
```

**Buffer Pool Sizing:**
- **Raspberry Pi 2GB:** `buffer_pool_size = 256` (2 MB)
- **Raspberry Pi 4GB:** `buffer_pool_size = 1024` (8 MB)
- **Server 16GB+:** `buffer_pool_size = 8192` (64 MB)

**WAL Sync Modes:**
- `fsync` - Maximum durability (slowest)
- `fdatasync` - Good durability (faster)
- `none` - No sync (fastest, use only for dev/testing)

---

### Authentication & Security

```toml
[auth]
# JWT configuration
jwt_secret = "CHANGE-THIS-IN-PRODUCTION"
jwt_expiration_hours = 8

# API Key settings
api_key_prefix = "nq_live_"
api_key_length = 32

# Biometric authentication
eeg_auth_enabled = false
eeg_threshold = 0.85
```

**Production Security Checklist:**
1. ✅ Generate secure JWT secret: `neuroquantum-api generate-jwt-secret`
2. ✅ Never commit secrets to Git
3. ✅ Use environment variables: `JWT_SECRET=xxx neuroquantum-api`
4. ✅ Rotate API keys regularly
5. ✅ Enable IP whitelisting (see below)

```toml
[security]
# Admin endpoint IP whitelist
admin_ip_whitelist = [
    "127.0.0.1",
    "::1",
    "192.168.1.100"  # Add your admin IP
]

# Rate limiting (requires Redis)
rate_limit_enabled = true
rate_limit_requests_per_hour = 1000
rate_limit_burst = 50

# Post-quantum cryptography
pqc_enabled = true
pqc_algorithm = "ml-kem"  # "ml-kem" | "ml-dsa"
```

---

### DNA Compression

```toml
[compression]
# Enable DNA compression globally
enabled = true

# Compression threshold (bytes)
min_size_bytes = 1024  # Only compress data > 1 KB

# Reed-Solomon error correction
ecc_enabled = true
ecc_redundancy = 0.2  # 20% redundancy

# NEON SIMD acceleration (ARM64 only)
neon_enabled = true
```

**When to Use DNA Compression:**
- ✅ Large text columns (logs, documents)
- ✅ JSON/XML data
- ✅ Genomic sequences
- ❌ Already compressed data (JPEG, PNG, ZIP)
- ❌ Small records (< 1 KB)

---

### Quantum Algorithms

```toml
[quantum]
# Grover's Search
grover_enabled = true
grover_threshold_rows = 100  # Use Grover for tables > 100 rows
grover_max_iterations = 1000

# Quantum Annealing
annealing_enabled = true
annealing_temperature_start = 10.0
annealing_temperature_end = 0.01
annealing_steps = 1000

# QAOA (Quantum Approximate Optimization)
qaoa_enabled = false
qaoa_layers = 3
```

**Performance Notes:**
- Grover's algorithm provides speedup for tables with **100+ rows**
- Annealing is best for **query optimization** (join order, index selection)
- QAOA is experimental (requires significant CPU)

---

### Neuromorphic Learning

```toml
[neuromorphic]
# Enable neuromorphic indexes
enabled = true

# Learning parameters
learning_rate = 0.01
decay_rate = 0.001

# STDP (Spike-Timing-Dependent Plasticity)
stdp_enabled = true
stdp_window_ms = 20

# Hebbian learning
hebbian_enabled = true
hebbian_threshold = 0.1
```

**Use Cases:**
- Adaptive indexes for changing access patterns
- Query optimization based on historical patterns
- Automatic schema tuning

---

### Monitoring & Logging

```toml
[monitoring]
# Prometheus metrics
prometheus_enabled = true
prometheus_port = 9090

# Health check
health_check_enabled = true
health_check_interval_seconds = 10
```

```toml
[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log format: "json" | "pretty"
format = "json"

# Log output: "stdout" | "stderr" | file path
output = "stdout"

# Rotation (for file output)
rotation_size_mb = 100
rotation_keep_files = 10
```

**Production Logging:**
```toml
[logging]
level = "info"  # or "warn" for less verbosity
format = "json"  # for log aggregation
output = "/var/log/neuroquantum/app.log"
```

**Development Logging:**
```toml
[logging]
level = "debug"  # or "trace" for deep debugging
format = "pretty"  # human-readable
output = "stdout"
```

---

### Backup Configuration

```toml
[backup]
# Backup directory
backup_dir = "/backups/neuroquantum"

# Automatic backups
auto_backup_enabled = true
auto_backup_interval_hours = 24  # Daily backups

# Backup retention
retention_days = 30  # Keep backups for 30 days

# Compression
backup_compression = "gzip"  # "gzip" | "zstd" | "none"

# Checksum verification
verify_checksums = true
```

---

## Environment Variables

Override configuration via environment variables:

```bash
# Server
export NEUROQUANTUM_HOST="0.0.0.0"
export NEUROQUANTUM_PORT="8080"

# Storage
export NEUROQUANTUM_DATA_DIR="/data/neuroquantum"
export NEUROQUANTUM_BUFFER_POOL_SIZE="1024"

# Security
export NEUROQUANTUM_JWT_SECRET="your-secret-here"
export RUST_LOG="info"

# Start server
neuroquantum-api
```

**Environment Variable Precedence:**
1. Environment variables (highest)
2. Config file specified via `--config`
3. Default config (`config/prod.toml` or `config/dev.toml`)

---

## Performance Tuning

### Raspberry Pi 4 (2GB RAM)

```toml
[server]
worker_threads = 4

[storage]
buffer_pool_size = 256  # 2 MB
page_size = 8192
checkpoint_interval_seconds = 600  # 10 minutes

[compression]
enabled = true
min_size_bytes = 512  # Aggressive compression

[quantum]
grover_threshold_rows = 50  # Lower threshold for benefit
```

### Raspberry Pi 4 (4GB RAM)

```toml
[server]
worker_threads = 4

[storage]
buffer_pool_size = 1024  # 8 MB
page_size = 8192

[compression]
enabled = true
min_size_bytes = 1024
```

### Server (16GB+ RAM)

```toml
[server]
worker_threads = 8  # or more

[storage]
buffer_pool_size = 8192  # 64 MB
page_size = 16384  # 16 KB pages

[compression]
enabled = false  # Less critical with abundant RAM
```

---

## Configuration Examples

### Example 1: High Security

```toml
[server]
host = "0.0.0.0"
port = 8443  # Use reverse proxy with TLS

[auth]
jwt_expiration_hours = 2  # Short-lived tokens
api_key_length = 64  # Longer keys

[security]
admin_ip_whitelist = ["192.168.1.100"]
rate_limit_enabled = true
rate_limit_requests_per_hour = 100  # Strict limits
pqc_enabled = true

[logging]
level = "warn"  # Don't log sensitive data
```

### Example 2: High Performance

```toml
[storage]
buffer_pool_size = 16384  # 128 MB
wal_sync_mode = "fdatasync"  # Faster than fsync
checkpoint_interval_seconds = 900  # Less frequent

[compression]
enabled = false  # Trade space for speed

[quantum]
grover_enabled = true
grover_threshold_rows = 50

[neuromorphic]
enabled = true
learning_rate = 0.05  # Faster adaptation
```

### Example 3: Low Power (IoT)

```toml
[server]
worker_threads = 2  # Fewer threads
request_timeout_seconds = 60  # Tolerate slower responses

[storage]
buffer_pool_size = 128  # 1 MB
checkpoint_interval_seconds = 1800  # Reduce writes

[compression]
enabled = true
min_size_bytes = 256  # Compress everything

[quantum]
grover_enabled = false  # Save CPU cycles
annealing_enabled = false

[neuromorphic]
enabled = false  # Save memory
```

---

## Validation

Validate your configuration:

```bash
# Check syntax
neuroquantum-api --config config/prod.toml --validate

# Dry-run startup
neuroquantum-api --config config/prod.toml --dry-run
```

---

## Next Steps

- 🔒 [Security Setup Guide](./security-setup.md)
- 📊 [Monitoring Setup](../deployment/monitoring.md)
- 🚀 [Performance Tuning](../operations/performance-tuning.md)

