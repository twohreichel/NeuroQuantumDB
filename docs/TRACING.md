# Distributed Tracing with OpenTelemetry

NeuroQuantumDB includes built-in support for distributed tracing using OpenTelemetry, allowing you to track requests across the system, debug slow queries, and understand request flow in cluster mode.

## Features

- ✅ OpenTelemetry SDK integration
- ✅ Multiple exporters (Jaeger, OTLP, Console)
- ✅ Automatic HTTP request tracing
- ✅ Query execution tracing
- ✅ Storage operation tracing
- ✅ Transaction lifecycle tracing
- ✅ Configurable sampling rates
- ✅ Trace context propagation

## Quick Start

### 1. Enable Tracing in Configuration

Add the following to your configuration file (e.g., `config/dev.toml`):

```toml
[tracing]
enabled = true
exporter = "jaeger"  # or "otlp", "console"
endpoint = "http://localhost:14268/api/traces"
sampling_rate = 1.0  # 100% for development, 0.1 for production (10%)
service_name = "neuroquantumdb"
trace_level = "detailed"  # "minimal", "detailed", or "debug"
```

### 2. Start Jaeger (Development)

Using Docker Compose:

```bash
cd docker/monitoring
docker-compose up -d jaeger
```

This starts Jaeger with all ports exposed:
- **UI**: http://localhost:16686
- **Collector (HTTP)**: http://localhost:14268
- **Collector (gRPC)**: http://localhost:14250
- **OTLP (gRPC)**: http://localhost:4317
- **OTLP (HTTP)**: http://localhost:4318

### 3. Start NeuroQuantumDB

```bash
cargo run --bin neuroquantum-api
```

### 4. View Traces

Open the Jaeger UI at http://localhost:16686 and select the `neuroquantumdb` service to view traces.

## Configuration Options

### Tracing Configuration

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `enabled` | bool | Enable/disable distributed tracing | `false` |
| `exporter` | string | Exporter type: "jaeger", "otlp", "zipkin", "console" | `"console"` |
| `endpoint` | string | Backend endpoint URL | `"http://localhost:14268/api/traces"` |
| `sampling_rate` | float | Sampling rate (0.0-1.0, where 1.0 = 100%) | `0.1` |
| `service_name` | string | Service name for trace identification | `"neuroquantumdb"` |
| `trace_level` | string | Detail level: "minimal", "detailed", "debug" | `"detailed"` |
| `resource_attributes` | map | Additional resource attributes | `null` |

### Trace Levels

- **minimal**: Only HTTP endpoints are traced
- **detailed**: Includes query execution and storage operations (recommended for production)
- **debug**: Includes all internal operations (development/debugging only)

### Sampling Strategies

Configure sampling rate based on your environment:

- **Development**: `1.0` (100%) - Trace every request for debugging
- **Staging**: `0.5` (50%) - Trace half of requests for testing
- **Production**: `0.1` (10%) - Trace 10% of requests to reduce overhead

## Exporters

### Jaeger Exporter

Best for local development and testing.

```toml
[tracing]
enabled = true
exporter = "jaeger"
endpoint = "http://localhost:14268/api/traces"
```

### OTLP Exporter

Best for production with OpenTelemetry Collector.

```toml
[tracing]
enabled = true
exporter = "otlp"
endpoint = "http://localhost:4317"  # gRPC endpoint
```

### Console Exporter

Best for development/debugging - prints traces to stdout.

```toml
[tracing]
enabled = true
exporter = "console"
```

## What Gets Traced?

### HTTP API Endpoints

Every HTTP request is automatically traced with:
- Request method and URI
- Response status code
- Request duration
- Trace context propagation (via headers)

### Query Execution

SQL queries are traced with:
- Statement type (SELECT, INSERT, UPDATE, DELETE)
- Table names accessed
- Execution time
- Row counts

### Storage Operations

Storage engine operations are traced with:
- Operation type (insert, select, update, delete)
- Table name
- Row counts
- Index usage

### Transaction Lifecycle

Transaction operations are traced with:
- Transaction ID
- Isolation level
- Begin, commit, rollback operations
- Savepoint operations

## Production Deployment

### With OpenTelemetry Collector

1. Deploy OpenTelemetry Collector
2. Configure NeuroQuantumDB to use OTLP exporter:

```toml
[tracing]
enabled = true
exporter = "otlp"
endpoint = "http://otel-collector:4317"
sampling_rate = 0.1  # 10% sampling
service_name = "neuroquantumdb"
trace_level = "detailed"
```

3. Configure OpenTelemetry Collector to export to your backend (Jaeger, Zipkin, etc.)

### With Jaeger Production Deployment

```toml
[tracing]
enabled = true
exporter = "jaeger"
endpoint = "http://jaeger-collector:14268/api/traces"
sampling_rate = 0.1
service_name = "neuroquantumdb"
trace_level = "detailed"

[tracing.resource_attributes]
environment = "production"
datacenter = "us-east-1"
cluster_id = "prod-cluster-01"
```

## Cluster Mode

In cluster mode, distributed tracing becomes even more valuable:

1. **Enable tracing on all nodes** with the same configuration
2. **Use consistent service names** or add node IDs as resource attributes
3. **Trace context is automatically propagated** between nodes

Example cluster configuration:

```toml
[tracing]
enabled = true
exporter = "otlp"
endpoint = "http://otel-collector:4317"
sampling_rate = 0.1
service_name = "neuroquantumdb"

[tracing.resource_attributes]
node_id = "node-1"
cluster_id = "prod-cluster"
region = "us-east-1"
```

## Troubleshooting

### Traces Not Appearing

1. **Check if tracing is enabled** in your configuration
2. **Verify exporter endpoint** is reachable
3. **Check sampling rate** - if too low, you might not see traces
4. **Check logs** for tracing initialization messages:
   ```
   📊 Initializing OpenTelemetry distributed tracing
   ✅ OpenTelemetry tracing initialized successfully
   ```

### High Overhead

If tracing is causing performance issues:

1. **Reduce sampling rate** from 1.0 to 0.1 or lower
2. **Change trace level** from "debug" to "detailed" or "minimal"
3. **Disable tracing** temporarily: `enabled = false`

### Connection Refused

If you see "connection refused" errors:

1. **Check if Jaeger/OTLP collector is running**:
   ```bash
   docker ps | grep jaeger
   ```
2. **Verify endpoint URL** in configuration
3. **Check firewall rules** if running remotely

## Environment Variables

You can override tracing configuration using environment variables:

```bash
export NEUROQUANTUM_TRACING_ENABLED=true
export NEUROQUANTUM_TRACING_EXPORTER=jaeger
export NEUROQUANTUM_TRACING_ENDPOINT=http://localhost:14268/api/traces
export NEUROQUANTUM_TRACING_SAMPLING_RATE=0.1
```

## Best Practices

1. **Use appropriate sampling rates** for each environment
2. **Add resource attributes** to identify traces by environment, region, etc.
3. **Use meaningful service names** in cluster deployments
4. **Monitor trace storage** - traces can consume significant storage
5. **Set retention policies** in your tracing backend
6. **Use trace level "detailed"** for production (good balance)
7. **Enable trace context propagation** in clients calling the API

## Examples

### View Slow Queries

1. Open Jaeger UI at http://localhost:16686
2. Select service: `neuroquantumdb`
3. Select operation: `execute`
4. Set min duration filter (e.g., `> 100ms`)
5. Click "Find Traces"

### Debug Failed Requests

1. Search for traces with tag: `error=true`
2. Examine the span details and logs
3. Follow the trace through the entire request lifecycle

### Analyze Query Performance

1. Find traces with operation: `select_rows`
2. Compare execution times across different queries
3. Identify which tables are slowest
4. Use trace data to guide optimization efforts

## Further Reading

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
