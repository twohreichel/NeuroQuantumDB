# NeuroQuantumDB - Quick Reference Card

**Version:** 0.1.0 | **Platform:** ARM64/x86_64 | **License:** MIT

---

## 🚀 Quick Start

```bash
# Install
curl -L https://github.com/neuroquantumdb/neuroquantumdb/releases/latest/download/neuroquantum-api -o neuroquantum-api
chmod +x neuroquantum-api

# Initialize
./neuroquantum-api init

# Start server
./neuroquantum-api
```

---

## 🔑 Authentication

```bash
# Login and get token
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"api_key": "nq_live_abc123..."}'
  
# Use token in requests
curl -H "Authorization: Bearer <token>" http://localhost:8080/...
```

---

## 📊 CRUD Operations

### Create Table
```bash
curl -X POST http://localhost:8080/tables/create \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "sensors",
    "columns": [
      {"name": "id", "data_type": "Integer", "primary_key": true},
      {"name": "temp", "data_type": "Float"}
    ]
  }'
```

### Insert Data
```bash
curl -X POST http://localhost:8080/tables/sensors/insert \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"rows": [{"id": 1, "temp": 25.5}]}'
```

### Query Data
```bash
curl -X POST http://localhost:8080/query/sql \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"query": "SELECT * FROM sensors WHERE temp > 20"}'
```

---

## 🧬 QSQL Extensions

### Neuromorphic Pattern Matching
```sql
SELECT * FROM sensors 
WHERE temperature NEUROMATCH pattern_id 
WITH SYNAPTIC_WEIGHT 0.8;
```

### Quantum-Accelerated Join
```sql
SELECT a.*, b.* FROM sensors a 
QUANTUM_JOIN readings b ON a.id = b.sensor_id
WITH GROVER_ITERATIONS 5;
```

### Natural Language Query
```bash
curl -X POST http://localhost:8080/query/natural \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"question": "Show all sensors in Berlin"}'
```

---

## ⚛️ Advanced Features

### DNA Compression
```bash
curl -X POST http://localhost:8080/compress/dna \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"data": "base64-data", "compression_level": 6}'
```

### Quantum Search
```bash
curl -X POST http://localhost:8080/quantum/search \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"table": "sensors", "pattern": {"location": "Berlin"}}'
```

### Train Neural Network
```bash
curl -X POST http://localhost:8080/neural/train \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"table": "sensors", "target_column": "temp", "epochs": 100}'
```

---

## ⚙️ Configuration

**File:** `config/prod.toml`

```toml
[server]
host = "127.0.0.1"
port = 8080

[jwt]
secret = "your-secret-key-min-32-chars"
expiration_hours = 1

[rate_limit]
requests_per_hour = 10000

[security]
admin_ip_whitelist = ["127.0.0.1"]
```

---

## 🏥 Health & Monitoring

```bash
# Health check
curl http://localhost:8080/health

# Metrics (Prometheus)
curl http://localhost:8080/metrics

# Performance stats
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/stats
```

---

## 🛠️ CLI Commands

```bash
# Initialize database
./neuroquantum-api init

# Generate JWT secret
./neuroquantum-api generate-jwt-secret

# Start server
./neuroquantum-api

# Custom config
./neuroquantum-api --config config/prod.toml

# Health check
./neuroquantum-api health-check
```

---

## 🔧 Development

```bash
# Build
make dev              # Development build
make build-release    # Production build

# Test
make test            # Run all tests
cargo test --lib     # Unit tests only

# Documentation
make docs            # Generate all docs
make docs-serve      # Serve at http://localhost:8000

# Code quality
make lint            # Run all linters
make format          # Format code
```

---

## 📚 Data Types

| Type | Description | Example |
|------|-------------|---------|
| Integer | 64-bit signed | `42` |
| Float | 64-bit float | `3.14` |
| Text | UTF-8 string | `"Hello"` |
| Boolean | True/false | `true` |
| Timestamp | Date/time | `"2025-11-17T12:00:00Z"` |
| Blob | Binary data | Base64 encoded |

---

## 🚨 Error Codes

| Code | Meaning |
|------|---------|
| `AUTH_REQUIRED` | Missing authentication |
| `INVALID_TOKEN` | Token expired/invalid |
| `RATE_LIMIT_EXCEEDED` | Too many requests |
| `INVALID_QUERY` | SQL syntax error |
| `TABLE_NOT_FOUND` | Table doesn't exist |

---

## 📈 Performance Tips

1. **Use indexes**: `CREATE INDEX idx_name ON table(column)`
2. **Enable quantum**: Add `WITH QUANTUM_PARALLEL` to queries
3. **Check query plans**: Use `EXPLAIN SELECT ...`
4. **Tune buffer pool**: Set in config (40-60% of RAM)
5. **Use DNA compression**: For large text/binary data

---

## 🔒 Security Checklist

- [ ] Changed default JWT secret
- [ ] Configured admin IP whitelist
- [ ] Enabled HTTPS (reverse proxy)
- [ ] Set rate limits
- [ ] Regular API key rotation
- [ ] Monitoring enabled
- [ ] Backup configured

---

## 📖 Full Documentation

- **User Guide**: [docs/user_guide.md](./docs/user_guide.md)
- **Developer Guide**: [docs/developer_guide.md](./docs/developer_guide.md)
- **Documentation Index**: [docs/README.md](./docs/README.md)
- **API Docs**: http://localhost:8080/api-docs/

---

## 🆘 Getting Help

- **GitHub Issues**: https://github.com/neuroquantumdb/neuroquantumdb/issues
- **Documentation**: https://neuroquantumdb.org/docs
- **Email**: support@neuroquantumdb.org

---

**NeuroQuantumDB** - Neuromorphic • Quantum • DNA Compression

*Made with 🧠 by the NeuroQuantumDB Team*

