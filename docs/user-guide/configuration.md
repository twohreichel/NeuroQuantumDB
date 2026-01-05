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

## Cluster Configuration (Beta)

⚠️ **WARNING: Cluster mode is currently in Beta/Preview and NOT recommended for production use.**

The cluster module is under active development. For production deployments, use single-node configuration.

### Missing Features

The following features are not yet implemented:

- **gRPC Network Transport**: Inter-node communication is incomplete
- **Full Raft Consensus**: Leader election and log replication are partial
- **Service Discovery**: DNS/Consul/etcd integration not available
- **Complete Replication**: Data synchronization has known limitations

### Cluster Configuration (Experimental)

If you want to test the cluster functionality in a development environment:

```toml
# config/cluster.toml (EXPERIMENTAL - DO NOT USE IN PRODUCTION)

[cluster]
enabled = false  # Keep disabled for production
node_id = 1
bind_addr = "0.0.0.0:9000"

# Peer nodes (if cluster enabled)
peers = [
    "node2:9000",
    "node3:9000"
]

[cluster.discovery]
# Service discovery (not yet implemented)
# method = "dns"  # or "consul", "etcd"
# endpoint = "neuroquantumdb.service.consul"
```

### Deployment Recommendations

| Deployment Scenario | Configuration | Status |
|---------------------|---------------|--------|
| **Development/Testing** | Single-node | ✅ Fully Supported |
| **Production** | Single-node | ✅ **Recommended** |
| **High Availability (Future)** | Multi-node cluster | ⚠️ Beta - Not Production Ready |

### Roadmap

Full cluster support with Raft consensus, gRPC transport, and service discovery is planned for **2026**. See the [Future Vision](../concept/06-future-vision.md) documentation for detailed roadmap.

## Next Steps

→ [Getting Started](getting-started.md)
