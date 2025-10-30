# NeuroQuantumDB Monitoring Setup

Complete monitoring stack with Prometheus, Grafana, and AlertManager for production-grade observability.

## 🚀 Quick Start

### Start the Monitoring Stack

```bash
cd docker/monitoring
docker-compose up -d
```

### Access Dashboards

- **Grafana**: http://localhost:3000
  - Username: `admin`
  - Password: `neuroquantum123`
- **Prometheus**: http://localhost:9090
- **AlertManager**: http://localhost:9093

## 📊 Available Dashboards

### 1. NeuroQuantumDB - Overview
Main dashboard providing high-level system health metrics:
- Database status and uptime
- Buffer pool hit ratio
- Active connections
- Query throughput (QPS)
- P99 query latency
- Memory and disk I/O
- Active WebSocket connections
- Slow query rate

**UID**: `neuroquantum-overview`

### 2. NeuroQuantumDB - Query Performance
Detailed query analysis and optimization insights:
- Query distribution by type
- Top 10 slowest queries
- Query time breakdown (execution, planning, lock wait, I/O)
- Index usage statistics
- Lock contention by type and resource
- Row examination efficiency
- Unused indexes detection
- Cache performance
- Query queue length

**UID**: `neuroquantum-queries`

### 3. NeuroQuantumDB - WebSocket
Real-time WebSocket monitoring:
- Active WebSocket connections
- Active channels and query streams
- Backpressure states
- Message throughput (sent/received)
- Top channels by subscribers
- Query streaming performance
- Message drop rate
- Connection health
- Message latency percentiles

**UID**: `neuroquantum-websocket`

## 🔔 Alerts

The system includes 14 pre-configured alerts:

### Critical Alerts
- **NeuroQuantumDBDown**: Service is down for > 1 minute
- **QueryQueueBuildup**: > 100 queries in queue

### Warning Alerts
- **HighQueryLatency**: P99 latency > 1 second
- **HighSlowQueryRate**: > 10 slow queries/second
- **HighLockContention**: > 5 seconds lock wait time/second
- **LowBufferPoolHitRatio**: < 90% hit ratio
- **HighMemoryUsage**: > 85% memory usage
- **ConnectionPoolSaturation**: > 80% connections used
- **HighErrorRate**: > 1 error/second
- **WebSocketConnectionDrop**: > 10 disconnections/second

### Info Alerts
- **HighWALGrowthRate**: > 100 MB/s WAL writes
- **UnusedIndexDetected**: Index not used in 24+ hours
- **QuantumOperationFailures**: Quantum operation failures detected
- **LowDiskSpace**: < 15% disk space remaining

## 📈 Metrics Exposed

### Query Metrics
```
neuroquantum_queries_total{query_type, status}
neuroquantum_query_duration_seconds_bucket{query_type}
neuroquantum_slow_queries_total
neuroquantum_query_errors_total{error_type}
neuroquantum_active_queries
neuroquantum_query_queue_length
```

### Connection Metrics
```
neuroquantum_active_connections
neuroquantum_max_connections
neuroquantum_connections_total
neuroquantum_connection_errors_total
```

### WebSocket Metrics
```
neuroquantum_websocket_connections_active
neuroquantum_websocket_messages_sent_total
neuroquantum_websocket_messages_received_total
neuroquantum_websocket_channels_total
neuroquantum_websocket_active_streams
neuroquantum_websocket_message_latency_seconds
```

### Storage Metrics
```
neuroquantum_buffer_pool_hit_ratio
neuroquantum_buffer_pool_pages{state}
neuroquantum_disk_read_bytes_total
neuroquantum_disk_write_bytes_total
neuroquantum_wal_bytes_written
neuroquantum_wal_segments_total
```

### Performance Metrics
```
neuroquantum_lock_wait_seconds_total{lock_type}
neuroquantum_index_scans_total{table_name, index_name}
neuroquantum_index_hit_ratio
neuroquantum_memory_usage_bytes
```

## 🛠️ Configuration

### Prometheus Configuration
Edit `prometheus.yml` to adjust:
- Scrape intervals (default: 15s)
- Scrape targets
- Retention period (default: 30 days)

### AlertManager Configuration
Edit `alertmanager.yml` to configure:
- Email recipients
- Slack webhooks
- PagerDuty integrations
- Alert grouping and inhibition rules

### Grafana Settings
Environment variables in `docker-compose.yml`:
- `GF_SECURITY_ADMIN_USER`: Admin username
- `GF_SECURITY_ADMIN_PASSWORD`: Admin password
- `GF_INSTALL_PLUGINS`: Additional plugins

## 🔧 Integration with NeuroQuantumDB

### Enable Metrics Export in API

Add to your `main.rs`:

```rust
use neuroquantum_core::monitoring::MetricsExporter;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Create metrics exporter
    let metrics = Arc::new(MetricsExporter::new()?);
    
    // Add metrics endpoint to your API
    let app = Router::new()
        .merge(neuroquantum_core::monitoring::prometheus::metrics_router(metrics.clone()))
        .route("/api/query", post(query_handler))
        // ... other routes
        .with_state(AppState { metrics });
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

### Record Metrics in Query Handler

```rust
async fn query_handler(
    State(state): State<AppState>,
    Json(query): Json<QueryRequest>,
) -> Result<Json<QueryResponse>> {
    let start = Instant::now();
    state.metrics.active_queries.inc();
    
    let result = execute_query(&query).await;
    
    let duration = start.elapsed();
    state.metrics.active_queries.dec();
    state.metrics.queries_total
        .with_label_values(&[&query.query_type, if result.is_ok() { "success" } else { "error" }])
        .inc();
    state.metrics.query_duration
        .with_label_values(&[&query.query_type])
        .observe(duration.as_secs_f64());
    
    if duration.as_millis() > 100 {
        state.metrics.slow_queries_total.inc();
    }
    
    result
}
```

## 📊 Retention and Storage

### Prometheus Storage
- **Path**: `prometheus-data` volume
- **Retention**: 30 days (configurable)
- **Size**: ~500MB per day for typical workload

### Grafana Storage
- **Path**: `grafana-data` volume
- **Size**: ~10MB for dashboards and users

## 🔐 Security

### Production Recommendations

1. **Change default passwords** in `docker-compose.yml`
2. **Enable HTTPS** with reverse proxy (nginx/traefik)
3. **Restrict access** using firewall rules
4. **Enable authentication** for all services
5. **Use secrets management** for sensitive config

### Example Nginx Config

```nginx
server {
    listen 443 ssl;
    server_name grafana.neuroquantum.local;
    
    ssl_certificate /etc/ssl/certs/grafana.crt;
    ssl_certificate_key /etc/ssl/private/grafana.key;
    
    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## 🧪 Testing

### Test Metrics Endpoint

```bash
curl http://localhost:8080/metrics
```

### Test Prometheus Scraping

```bash
curl http://localhost:9090/api/v1/targets
```

### Verify Alerts

```bash
curl http://localhost:9090/api/v1/alerts
```

## 📝 Maintenance

### Backup Grafana Dashboards

```bash
# Export all dashboards
docker exec neuroquantum-grafana grafana-cli admin export-dashboard

# Or backup the data volume
docker run --rm -v neuroquantum-monitoring_grafana-data:/data -v $(pwd):/backup \
    alpine tar czf /backup/grafana-backup.tar.gz /data
```

### Clean Up Old Metrics

```bash
# Prometheus auto-cleans based on retention settings
# Force cleanup if needed:
docker exec neuroquantum-prometheus promtool tsdb cleanup /prometheus
```

### Update Dashboards

1. Edit dashboard JSON files in `dashboards/`
2. Restart Grafana: `docker-compose restart grafana`
3. Dashboards auto-reload within 10 seconds

## 🐛 Troubleshooting

### Metrics Not Appearing

1. Check NeuroQuantumDB is exposing `/metrics` endpoint
2. Verify Prometheus is scraping: http://localhost:9090/targets
3. Check Prometheus logs: `docker logs neuroquantum-prometheus`

### Dashboard Shows "No Data"

1. Verify data source connection in Grafana
2. Check metric names match in queries
3. Adjust time range (some metrics need time to accumulate)

### High Memory Usage

1. Reduce Prometheus retention period
2. Decrease scrape frequency
3. Limit number of metrics series

## 🔗 Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [AlertManager Documentation](https://prometheus.io/docs/alerting/latest/alertmanager/)
- [PromQL Tutorial](https://prometheus.io/docs/prometheus/latest/querying/basics/)

## 📄 License

Part of NeuroQuantumDB - See main LICENSE file.

