# NeuroQuantumDB - Production Deployment Guide

## Quick Start

### Prerequisites
- Docker 20.10+ with BuildKit support
- Docker Compose 2.0+
- ARM64 platform (Raspberry Pi 4) or compatible emulation
- At least 4GB RAM, 8GB storage recommended

### Build the Image

```bash
# Build for ARM64 (Raspberry Pi 4)
cd /path/to/NeuroQuantumDB
docker build --platform linux/arm64 -t neuroquantumdb:latest .

# Or use the automated build script
./scripts/docker-build-and-test.sh
```

### Deploy Full Stack

```bash
cd docker/production
docker-compose up -d
```

This will start:
- **NeuroQuantumDB** (port 8080) - Main database API
- **Redis** (port 6379) - Rate limiting & caching
- **Prometheus** (port 9091) - Metrics collection
- **Grafana** (port 3000) - Visualization dashboard
- **Jaeger** (port 16686) - Distributed tracing
- **Vector** - Log aggregation
- **HAProxy** (port 80/443) - Load balancer

### Verify Deployment

```bash
# Check all services are running
docker-compose ps

# Check NeuroQuantumDB health
curl http://localhost:8080/health

# Check Prometheus metrics
curl http://localhost:9090/metrics

# View logs
docker-compose logs -f neuroquantumdb
```

### Access Web UIs

- **NeuroQuantumDB API**: http://localhost:8080
- **API Documentation**: http://localhost:8080/swagger
- **Grafana**: http://localhost:3000 (admin/neuroquantum2024)
- **Prometheus**: http://localhost:9091
- **Jaeger Tracing**: http://localhost:16686
- **HAProxy Stats**: http://localhost:8404/stats

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                      HAProxy LB                         │
│                    (Port 80/443)                        │
└─────────────────────┬───────────────────────────────────┘
                      │
          ┌───────────┴────────────┐
          │                        │
┌─────────▼──────────┐   ┌────────▼──────────┐
│ NeuroQuantumDB     │   │   Monitoring      │
│  - REST API        │   │  - Prometheus     │
│  - WebSocket       │   │  - Grafana        │
│  - QSQL Engine     │   │  - Jaeger         │
│  - Storage Engine  │   └───────────────────┘
└─────────┬──────────┘
          │
    ┌─────┴──────┐
    │            │
┌───▼────┐  ┌───▼─────┐
│ Redis  │  │ Vector  │
│ Cache  │  │ Logs    │
└────────┘  └─────────┘
```

---

## Configuration

### Environment Variables

Edit `docker-compose.yml` to configure:

```yaml
environment:
  - RUST_LOG=info                          # Log level: trace, debug, info, warn, error
  - NEUROQUANTUM_ENV=production            # Environment: development, production
  - REDIS_URL=redis://redis:6379           # Redis connection string
```

### Production Config

Edit `config/prod.toml` for:
- Database settings
- Security configuration
- Performance tuning
- Feature flags

See `docs/getting-started/configuration.md` for full reference.

### Resource Limits

Adjust resource limits for your hardware:

```yaml
deploy:
  resources:
    limits:
      memory: 100M    # Increase if needed
      cpus: '1.0'     # Adjust for available cores
```

---

## Redis Configuration

### Persistence

Redis is configured with AOF (Append Only File) for durability:
- `appendfsync everysec` - Sync every second (balanced performance/safety)
- Snapshots every 15 minutes (900s) if 1+ key changed
- Snapshots every 5 minutes (300s) if 10+ keys changed
- Snapshots every 1 minute (60s) if 10000+ keys changed

### Memory Management

- Max memory: 50MB
- Eviction policy: `allkeys-lru` (Least Recently Used)
- Ideal for rate limiting + session caching

### Backup Redis Data

```bash
# Create backup
docker exec neuroquantum-redis redis-cli BGSAVE

# Copy dump to host
docker cp neuroquantum-redis:/data/dump.rdb ./backup/
```

---

## Monitoring

### Prometheus Metrics

Available at `http://localhost:9090/metrics`:
- Database operations (queries/sec, latency)
- Cache hit rates
- Transaction metrics
- Quantum algorithm performance
- System resources (CPU, memory)

### Grafana Dashboards

Import dashboards from `docker/monitoring/dashboards/`:
- System overview
- Database performance
- Quantum metrics
- Neuromorphic learning stats

### Alerts

Configure alerts in `docker/monitoring/rules/`:
- High memory usage
- Slow query performance
- Cache miss rate threshold
- Error rate spikes

---

## Security

### Secrets Management

**⚠️ IMPORTANT: Change default secrets before production!**

```bash
# Generate secure JWT secret
./neuroquantum-api generate-jwt-secret

# Initialize admin API key
./neuroquantum-api init --admin-email admin@example.com
```

Update `config/prod.toml`:
```toml
[security]
jwt_secret = "your-generated-secret-here"
```

### TLS/HTTPS

Configure TLS in HAProxy:
1. Place certificates in `config/certs/`
2. Update `config/haproxy.cfg`
3. Restart: `docker-compose restart loadbalancer`

### Firewall Rules

Recommended firewall setup:
```bash
# Allow only necessary ports
ufw allow 80/tcp    # HTTP
ufw allow 443/tcp   # HTTPS
ufw deny 6379/tcp   # Redis (internal only)
ufw deny 9090/tcp   # Prometheus (internal only)
```

---

## Backup & Recovery

### Automated Backups

```bash
# Full backup
docker exec neuroquantumdb-main /usr/local/bin/neuroquantumdb backup \
  --type full \
  --path /var/lib/neuroquantumdb/backups

# Incremental backup
docker exec neuroquantumdb-main /usr/local/bin/neuroquantumdb backup \
  --type incremental
```

### Restore from Backup

```bash
docker exec neuroquantumdb-main /usr/local/bin/neuroquantumdb restore \
  --backup-path /var/lib/neuroquantumdb/backups/backup_TIMESTAMP
```

### Volume Backup

```bash
# Backup Docker volumes
docker run --rm \
  -v neuroquantum_data:/data \
  -v $(pwd)/backup:/backup \
  alpine tar czf /backup/neuroquantum_data.tar.gz -C /data .
```

---

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker-compose logs neuroquantumdb

# Check resource usage
docker stats

# Verify health check
docker exec neuroquantumdb-main /usr/local/bin/neuroquantumdb health-check
```

### High Memory Usage

```bash
# Check cache size
curl http://localhost:9090/stats

# Clear Redis cache
docker exec neuroquantum-redis redis-cli FLUSHDB

# Restart with lower limits
docker-compose down
# Edit resource limits in docker-compose.yml
docker-compose up -d
```

### Redis Connection Failed

```bash
# Check Redis health
docker exec neuroquantum-redis redis-cli ping

# Restart Redis
docker-compose restart redis

# Check network connectivity
docker exec neuroquantumdb-main ping redis
```

### Slow Performance

1. Check resource limits: `docker stats`
2. Verify NEON optimizations are enabled (ARM64 SIMD)
3. Tune buffer pool size in `config/prod.toml`
4. Check Redis cache hit rate: `curl http://localhost:9090/stats`
5. Review Prometheus metrics for bottlenecks

---

## Upgrading

### Zero-Downtime Upgrade

```bash
# Pull new image
docker pull neuroquantumdb:latest

# Rolling update
docker-compose up -d --no-deps --build neuroquantumdb

# Verify
curl http://localhost:8080/health
```

### With Downtime

```bash
# Backup first!
docker exec neuroquantumdb-main /usr/local/bin/neuroquantumdb backup --type full

# Stop services
docker-compose down

# Update image
docker pull neuroquantumdb:latest

# Start services
docker-compose up -d
```

---

## Performance Tuning

### Raspberry Pi 4 Optimization

**CPU Governor:**
```bash
# Set performance mode
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

**Memory:**
```bash
# Increase swap (if needed)
sudo dphys-swapfile swapoff
sudo sed -i 's/CONF_SWAPSIZE=100/CONF_SWAPSIZE=2048/' /etc/dphys-swapfile
sudo dphys-swapfile setup
sudo dphys-swapfile swapon
```

**Docker:**
```bash
# Optimize Docker daemon
cat > /etc/docker/daemon.json <<EOF
{
  "storage-driver": "overlay2",
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  }
}
EOF
sudo systemctl restart docker
```

---

## Production Checklist

- [ ] Changed default JWT secret
- [ ] Created admin API key (not default)
- [ ] Configured TLS/HTTPS
- [ ] Set up automated backups
- [ ] Configured firewall rules
- [ ] Set up monitoring alerts
- [ ] Tested disaster recovery
- [ ] Documented custom configurations
- [ ] Load tested with expected traffic
- [ ] Security audit completed
- [ ] Updated all passwords

---

## Support

- **Documentation**: See `docs/` folder or http://your-domain/docs
- **API Reference**: http://localhost:8080/swagger
- **Issues**: https://github.com/your-org/neuroquantumdb/issues
- **Examples**: See `examples/` folder

---

## License

See LICENSE file in root directory.

