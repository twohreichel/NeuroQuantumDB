# Configuration

## Configuration Files

| File | Purpose |
|------|---------|
| `config/dev.toml` | Development settings |
| `config/prod.toml` | Production settings |

## Environment Variables

```bash
# Server
NQDB_HOST=0.0.0.0
NQDB_PORT=8080

# Security
NQDB_JWT_SECRET=your-secret-key
NQDB_JWT_EXPIRATION_HOURS=8

# Storage
NQDB_DATA_PATH=/var/lib/neuroquantumdb
NQDB_WAL_PATH=/var/lib/neuroquantumdb/wal

# Logging
RUST_LOG=info,neuroquantum=debug
```

## Production Configuration

```toml
# config/prod.toml

[server]
host = "0.0.0.0"
port = 8080
workers = 4

[auth]
jwt_secret = "YOUR-GENERATED-SECRET"
jwt_expiration_hours = 8

[security]
admin_ip_whitelist = ["127.0.0.1", "::1"]
rate_limit_requests = 100
rate_limit_window_secs = 60

[storage]
data_path = "/var/lib/neuroquantumdb"
buffer_pool_size_mb = 256
wal_enabled = true

[compression]
dna_enabled = true
compression_level = 6
```

## Generate JWT Secret

```bash
# Generate secure secret
neuroquantum-api generate-jwt-secret --output config/jwt-secret.txt
```

## Next Steps

→ [Getting Started](getting-started.md)
