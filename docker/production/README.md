# NeuroQuantumDB Production Docker Deployment

This directory contains the production-ready Docker Compose configuration for NeuroQuantumDB.

## Quick Start

1. **Configure environment variables**:
   ```bash
   cp .env.example .env
   # Edit .env and set NEUROQUANTUM_JWT_SECRET (required)
   ```

2. **Generate secure secrets**:
   ```bash
   # JWT Secret (required)
   echo "NEUROQUANTUM_JWT_SECRET=$(openssl rand -base64 48)" >> .env
   
   # Admin API Key (recommended)
   echo "NEUROQUANTUM_ADMIN_KEY=$(openssl rand -hex 32)" >> .env
   ```

3. **Start the stack**:
   ```bash
   docker-compose up -d
   ```

4. **Verify deployment**:
   ```bash
   curl http://localhost:8080/health
   ```

## Services

| Service | Port | Description |
|---------|------|-------------|
| NeuroQuantumDB | 8080 | Main API server |
| NeuroQuantumDB Metrics | 9090 | Prometheus metrics |
| Redis | 6379 | Rate limiting & caching |
| Jaeger UI | 16686 | Distributed tracing UI |
| Prometheus | 9091 | Metrics collection |
| Grafana | 3000 | Monitoring dashboards |

## Configuration Methods

### 1. Environment Variables (Recommended for secrets)

Set in `.env` file or pass directly:

```bash
# Required
NEUROQUANTUM_JWT_SECRET=your-secure-secret-here

# Optional overrides
NEUROQUANTUM_HOST=0.0.0.0
NEUROQUANTUM_PORT=8080
NEUROQUANTUM_LOG_LEVEL=info
NEUROQUANTUM_RATE_LIMIT=10000
NEUROQUANTUM_REDIS_URL=redis://redis:6379
```

### 2. Custom Configuration File

Mount your own configuration file:

```bash
# In .env
NEUROQUANTUM_CONFIG_FILE=/path/to/your/config.toml
```

Or modify `./config/prod.toml` directly.

### 3. Environment Variable Priority

Environment variables always override config file values, in this order:
1. Environment variables (highest priority)
2. Mounted config file
3. Default values (lowest priority)

## Optional Load Balancer

The HAProxy load balancer is disabled by default. To enable:

```bash
# Start with load balancer
docker-compose --profile with-lb up -d

# Configure TLS in .env
TLS_CERT_PATH=/path/to/cert.crt
TLS_KEY_PATH=/path/to/key.key
```

## Monitoring

### Grafana Dashboards

Access Grafana at http://localhost:3000 (default: admin/neuroquantum2024)

Pre-configured dashboards:
- NeuroQuantumDB Overview
- API Performance
- Rate Limiting Status
- System Resources

### Jaeger Tracing

Access Jaeger UI at http://localhost:16686

View distributed traces for:
- API requests
- Database operations
- Query execution

### Prometheus Metrics

Metrics available at:
- http://localhost:8080/metrics (application)
- http://localhost:9091 (Prometheus UI)

## Resource Limits

Optimized for Raspberry Pi 4 / edge devices:

| Service | Memory Limit | CPU Limit |
|---------|--------------|-----------|
| NeuroQuantumDB | 100MB | 1.0 |
| Redis | 64MB | 0.5 |
| Jaeger | 128MB | 0.5 |
| Prometheus | - | - |
| Grafana | - | - |

## Troubleshooting

### Check logs
```bash
docker-compose logs -f neuroquantumdb
```

### Verify configuration
```bash
docker-compose exec neuroquantumdb cat /etc/neuroquantumdb/config.toml
```

### Check health
```bash
curl -s http://localhost:8080/health | jq
```

### Reset everything
```bash
docker-compose down -v
docker-compose up -d
```

## Security Checklist

- [ ] Set `NEUROQUANTUM_JWT_SECRET` (minimum 32 characters)
- [ ] Set `NEUROQUANTUM_ADMIN_KEY` for admin operations
- [ ] Change Grafana admin password
- [ ] Update CORS origins in config
- [ ] Configure admin IP whitelist
- [ ] Enable TLS if exposing externally
- [ ] Review rate limiting settings
