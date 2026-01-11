# NeuroQuantumDB Monitoring Stack

This directory contains a complete monitoring and observability stack for NeuroQuantumDB including metrics, logging, and distributed tracing.

## Services Included

- **Prometheus** - Metrics collection and storage (port 9090)
- **Grafana** - Metrics visualization and dashboards (port 3000)
- **AlertManager** - Alert routing and management (port 9093)
- **Node Exporter** - System metrics (port 9100)
- **cAdvisor** - Container metrics (port 8080)
- **Jaeger** - Distributed tracing (UI: port 16686)

## Quick Start

Start all monitoring services:

```bash
docker-compose up -d
```

Stop all services:

```bash
docker-compose down
```

Remove all data volumes:

```bash
docker-compose down -v
```

## Accessing Services

### Grafana
- URL: http://localhost:3000
- Default credentials:
  - Username: `admin`
  - Password: `neuroquantum123`

### Prometheus
- URL: http://localhost:9090

### Jaeger UI
- URL: http://localhost:16686

### AlertManager
- URL: http://localhost:9093

## Jaeger Distributed Tracing

Jaeger is configured with all receivers enabled:

- **Jaeger Collector (HTTP)**: http://localhost:14268
- **Jaeger Collector (gRPC)**: http://localhost:14250
- **OTLP gRPC Receiver**: http://localhost:4317
- **OTLP HTTP Receiver**: http://localhost:4318
- **Zipkin Compatible**: http://localhost:9411

### Using with NeuroQuantumDB

Configure NeuroQuantumDB to send traces to Jaeger:

```toml
[tracing]
enabled = true
exporter = "jaeger"
endpoint = "http://localhost:4317"
sampling_rate = 1.0
service_name = "neuroquantumdb-dev"
```

Or use OTLP:

```toml
[tracing]
enabled = true
exporter = "otlp"
endpoint = "http://localhost:4317"
sampling_rate = 1.0
service_name = "neuroquantumdb-dev"
```

## Configuration Files

- `prometheus.yml` - Prometheus scrape configuration
- `alertmanager.yml` - Alert routing configuration
- `datasources.yml` - Grafana datasource configuration
- `dashboard-config.yml` - Grafana dashboard provisioning
- `dashboards/` - Pre-built Grafana dashboards
- `rules/` - Prometheus alerting rules

## Data Persistence

All data is persisted in Docker volumes:

- `prometheus-data` - Prometheus time-series data
- `grafana-data` - Grafana dashboards and settings
- `alertmanager-data` - AlertManager state
- `jaeger-data` - Jaeger trace storage (Badger DB)

## Customization

### Adding Custom Dashboards

Place JSON dashboard files in `dashboards/` directory and restart Grafana.

### Configuring Alerts

Edit `rules/*.yml` files and restart Prometheus:

```bash
docker-compose restart prometheus
```

### Jaeger Storage

By default, Jaeger uses embedded Badger storage. For production, consider:

- Elasticsearch backend
- Cassandra backend
- Configure retention policies

## Troubleshooting

### Prometheus Can't Scrape NeuroQuantumDB

Ensure NeuroQuantumDB is running and accessible at the configured endpoint in `prometheus.yml`.

### Grafana Dashboards Not Loading

Check that datasources are correctly configured in `datasources.yml`.

### Jaeger Not Receiving Traces

1. Verify Jaeger is running: `docker ps | grep jaeger`
2. Check NeuroQuantumDB tracing configuration
3. Verify network connectivity to Jaeger

## Resource Requirements

Minimum recommended resources:

- CPU: 2 cores
- RAM: 4GB
- Disk: 20GB (for time-series data and traces)

## Production Deployment

For production:

1. **Use external storage** for Prometheus and Jaeger
2. **Configure retention policies** to manage disk usage
3. **Set up authentication** for Grafana and other services
4. **Configure TLS** for secure communication
5. **Set up backup** for Grafana dashboards and Prometheus data
6. **Use proper resource limits** in docker-compose.yml

## Further Reading

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [NeuroQuantumDB Tracing Guide](../../docs/TRACING.md)
