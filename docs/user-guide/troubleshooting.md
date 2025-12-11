# Troubleshooting

## Common Issues

### Server Won't Start

**Symptom:** `Address already in use`

```bash
# Find process using port
lsof -i :8080

# Kill process
kill -9 <PID>
```

---

### Authentication Failed

**Symptom:** `401 Unauthorized`

**Check:**
1. API key is valid and not expired
2. JWT secret matches configuration
3. Token format: `Bearer <token>`

```bash
# Verify token
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/auth/verify
```

---

### Out of Memory

**Symptom:** Server crashes or slows down

**Solutions:**
```toml
# Reduce buffer pool
[storage]
buffer_pool_size_mb = 128

# Enable compression
[compression]
dna_enabled = true
```

---

### Slow Queries

**Symptom:** Query takes > 1 second

**Debug:**
```sql
-- Check query plan
EXPLAIN SELECT * FROM large_table WHERE id = 1;

-- Create index
CREATE INDEX idx_id ON large_table(id);
```

---

### WAL Recovery Failed

**Symptom:** `WAL corruption detected`

```bash
# Backup corrupted WAL
mv data/wal data/wal.bak

# Reinitialize
neuroquantum-api init --force
```

---

## Debug Mode

Enable verbose logging:

```bash
RUST_LOG=trace ./neuroquantum-api
```

## Getting Help

1. Check [GitHub Issues](https://github.com/neuroquantumdb/neuroquantumdb/issues)
2. Search [Discussions](https://github.com/neuroquantumdb/neuroquantumdb/discussions)
3. Open new issue with:
   - NeuroQuantumDB version
   - OS and architecture
   - Steps to reproduce
   - Error logs
