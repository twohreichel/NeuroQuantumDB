# Monitoring

## Prometheus Metrics

Endpoint: `GET /metrics`

### Available Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `nqdb_queries_total` | Counter | Total queries executed |
| `nqdb_query_duration_seconds` | Histogram | Query latency |
| `nqdb_connections_active` | Gauge | Active connections |
| `nqdb_buffer_pool_hits` | Counter | Buffer cache hits |
| `nqdb_dna_compressions_total` | Counter | DNA compressions |
| `nqdb_quantum_searches_total` | Counter | Quantum searches |

### Example Scrape Config

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'neuroquantumdb'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: /metrics
```

## Health Check

```bash
curl http://localhost:8080/health
```

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "storage": {
    "status": "ok",
    "used_bytes": 1073741824
  }
}
```

## Logging

Configure via `RUST_LOG`:

```bash
# Levels: error, warn, info, debug, trace
RUST_LOG=info,neuroquantum=debug ./neuroquantum-api
```

### Log Output

```
2024-12-11T10:00:00Z INFO  neuroquantum_api: Server started on 0.0.0.0:8080
2024-12-11T10:00:01Z DEBUG neuroquantum_core: Buffer pool initialized (256MB)
2024-12-11T10:00:05Z INFO  neuroquantum_api: Query executed in 12ms
```

## Grafana Dashboard

Import dashboard from `docker/monitoring/dashboards/`:

- Query performance
- Resource usage
- Error rates
- Neural network training progress
