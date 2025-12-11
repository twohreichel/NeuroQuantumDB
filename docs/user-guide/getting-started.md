# Getting Started

## Initialize Database

```bash
# Interactive setup
neuroquantum-api init

# Non-interactive
neuroquantum-api init \
  --name admin \
  --expiry-hours 8760 \
  --output .env \
  --yes
```

## Start Server

```bash
# Development
cargo run --bin neuroquantum-api

# Production
./target/release/neuroquantum-api --config config/prod.toml
```

## First API Call

### Health Check

```bash
curl http://localhost:8080/health
```

### Create API Key

```bash
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"name": "my-app", "permissions": ["read", "write"]}'
```

Response:
```json
{
  "key": "nqdb_xxxxxxxxxxxx",
  "name": "my-app",
  "permissions": ["read", "write"],
  "expires_at": "2026-12-11T00:00:00Z"
}
```

## Basic Operations

### Create Table

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "CREATE TABLE users (id INT, name TEXT, email TEXT)"}'
```

### Insert Data

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "INSERT INTO users VALUES (1, '\''Alice'\'', '\''alice@example.com'\'')"}'
```

### Query Data

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM users WHERE id = 1"}'
```

## Next Steps

→ [QSQL Query Language](qsql.md)
