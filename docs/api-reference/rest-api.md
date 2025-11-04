# REST API Reference

Complete reference for all REST API endpoints.

## Base URL

```
http://localhost:8080/api/v1
```

## Authentication

All requests require authentication via Bearer token:

```bash
Authorization: Bearer YOUR_API_KEY_OR_JWT_TOKEN
```

---

## Endpoints

### Query Execution

#### `POST /api/v1/query`

Execute a QSQL query.

**Request:**
```json
{
  "query": "SELECT * FROM users WHERE age > 25"
}
```

**Response:**
```json
{
  "success": true,
  "columns": ["id", "name", "age"],
  "rows": [[1, "Alice", 30], [2, "Bob", 28]],
  "rows_returned": 2,
  "execution_time_ms": 12.5
}
```

---

### Admin Endpoints

#### `POST /api/v1/admin/keys`

Create a new API key (requires admin role).

**Request:**
```json
{
  "name": "production-app",
  "permissions": ["read", "write"],
  "expires_in_hours": 8760
}
```

**Response:**
```json
{
  "api_key": "nq_live_abc123...",
  "key_id": "key_123",
  "expires_at": "2026-11-04T12:00:00Z"
}
```

---

### Health & Monitoring

#### `GET /health`

Health check endpoint (no authentication required).

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 42
}
```

#### `GET /metrics`

Prometheus metrics endpoint (no authentication required).

**Response:**
```
# TYPE neuroquantum_queries_total counter
neuroquantum_queries_total 12345
```

---

## Error Responses

All errors follow this format:

```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": {}
}
```

**Common HTTP Status Codes:**
- `200 OK` - Success
- `400 Bad Request` - Invalid input
- `401 Unauthorized` - Missing or invalid authentication
- `403 Forbidden` - Insufficient permissions
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server error

---

## Rate Limiting

All endpoints are rate-limited. Headers indicate limits:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 450
X-RateLimit-Reset: 1699200000
```

---

## OpenAPI Specification

Full OpenAPI 3.0 specification available at:

```
http://localhost:8080/swagger-ui
http://localhost:8080/api-docs/openapi.json
```

