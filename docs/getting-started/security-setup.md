# Security Setup

Complete guide to securing NeuroQuantumDB for production deployment.

## Security Overview

NeuroQuantumDB implements multiple layers of security:

1. **Authentication** - JWT tokens + API keys + Biometric
2. **Authorization** - Role-based access control (RBAC)
3. **Encryption** - TLS + AES-GCM + Post-Quantum Cryptography
4. **Rate Limiting** - Protection against abuse
5. **IP Whitelisting** - Network-level access control
6. **Audit Logging** - Complete security event tracking

---

## Initial Setup (Critical!)

### Step 1: Initialize Admin Credentials

⚠️ **NeuroQuantumDB does NOT have default credentials.** You must initialize explicitly:

```bash
# Interactive setup (recommended)
neuroquantum-api init

# Non-interactive with custom settings
neuroquantum-api init \
  --name admin \
  --expiry-hours 8760 \
  --output .env \
  --yes
```

**Output:**
```
✅ Admin API Key created: nq_live_abc123xyz789...
⚠️  Save this key! It won't be shown again.
📝 Key saved to: .env
```

**Store this key in a secure location:**
- Password manager (1Password, Bitwarden)
- Secrets management system (HashiCorp Vault, AWS Secrets Manager)
- Encrypted configuration file

---

### Step 2: Generate JWT Secret

```bash
# Generate 512-bit secure random secret
neuroquantum-api generate-jwt-secret --output config/jwt-secret.txt

# Or specify length
neuroquantum-api generate-jwt-secret --length 1024 --output config/jwt-secret.txt
```

**Update `config/prod.toml`:**
```toml
[auth]
jwt_secret_file = "config/jwt-secret.txt"
# Or inline (not recommended):
# jwt_secret = "YOUR-GENERATED-SECRET-HERE"
```

---

### Step 3: Configure IP Whitelisting

Edit `config/prod.toml`:

```toml
[security]
# Only these IPs can access admin endpoints
admin_ip_whitelist = [
    "127.0.0.1",      # Localhost
    "::1",            # IPv6 localhost
    "192.168.1.100",  # Your admin workstation
    "10.0.0.50"       # CI/CD server
]

# Enable whitelist enforcement
enforce_ip_whitelist = true
```

**Protected Admin Endpoints:**
- `POST /api/v1/admin/keys` - Create API keys
- `DELETE /api/v1/admin/keys/:id` - Revoke API keys
- `POST /api/v1/admin/users` - Create users
- `GET /api/v1/admin/stats` - Detailed statistics

---

## Authentication Methods

### 1. API Keys

**Create API Key:**
```bash
curl -X POST http://localhost:8080/api/v1/admin/keys \
  -H "Authorization: Bearer YOUR_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "production-app",
    "permissions": ["read", "write"],
    "expires_in_hours": 8760
  }'
```

**Response:**
```json
{
  "api_key": "nq_live_xyz789abc123...",
  "key_id": "key_123",
  "expires_at": "2026-11-04T12:00:00Z"
}
```

**Use API Key:**
```bash
curl http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer nq_live_xyz789abc123..." \
  -d '{"query": "SELECT * FROM users"}'
```

**Revoke API Key:**
```bash
curl -X DELETE http://localhost:8080/api/v1/admin/keys/key_123 \
  -H "Authorization: Bearer YOUR_ADMIN_KEY"
```

---

### 2. JWT Tokens

**Login (Get JWT Token):**
```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "secure_password_here"
  }'
```

**Response:**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2025-11-04T20:00:00Z",
  "user": {
    "id": 1,
    "username": "alice",
    "role": "admin"
  }
}
```

**Use JWT Token:**
```bash
curl http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -d '{"query": "SELECT * FROM users"}'
```

**Token Expiration:**
```toml
[auth]
jwt_expiration_hours = 8  # Tokens expire after 8 hours
```

---

### 3. Biometric Authentication (EEG)

**Enable EEG Authentication:**
```toml
[auth]
eeg_auth_enabled = true
eeg_threshold = 0.85  # 85% confidence required
eeg_sample_rate = 256  # Hz
```

**Enroll EEG Pattern:**
```bash
curl -X POST http://localhost:8080/api/v1/auth/eeg/enroll \
  -H "Authorization: Bearer YOUR_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": 1,
    "eeg_data": [0.12, 0.45, -0.23, ...],
    "duration_seconds": 30
  }'
```

**Authenticate with EEG:**
```bash
curl -X POST http://localhost:8080/api/v1/auth/eeg/verify \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": 1,
    "eeg_data": [0.11, 0.46, -0.24, ...]
  }'
```

---

## Rate Limiting

### Redis-Based Rate Limiting

**Install Redis:**
```bash
# Docker
docker run -d --name redis -p 6379:6379 redis:7-alpine

# Or use Docker Compose (included)
docker-compose -f docker/production/docker-compose.yml up -d redis
```

**Configure Rate Limiting:**
```toml
[rate_limiting]
enabled = true
backend = "redis"  # "redis" or "memory"
redis_url = "redis://localhost:6379"

# Global limits
requests_per_hour = 1000
burst = 50

# Per-endpoint limits
[rate_limiting.endpoints]
"/api/v1/query" = { requests_per_hour = 500, burst = 20 }
"/api/v1/admin/keys" = { requests_per_hour = 5, burst = 2 }
```

**Rate Limit Response:**
```json
{
  "error": "Rate limit exceeded",
  "retry_after_seconds": 3600,
  "limit": 1000,
  "remaining": 0
}
```

**Headers:**
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 450
X-RateLimit-Reset: 1699200000
```

---

## Encryption

### 1. TLS/HTTPS (Recommended)

Use a reverse proxy (Nginx, Caddy) for TLS termination:

**Nginx Configuration:**
```nginx
server {
    listen 443 ssl http2;
    server_name db.example.com;

    ssl_certificate /etc/letsencrypt/live/db.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/db.example.com/privkey.pem;

    # Strong TLS settings
    ssl_protocols TLSv1.3;
    ssl_prefer_server_ciphers on;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

**Caddy Configuration (Automatic HTTPS):**
```
db.example.com {
    reverse_proxy localhost:8080
}
```

---

### 2. At-Rest Encryption

**Encrypt Data Directory:**
```bash
# Using LUKS (Linux)
sudo cryptsetup luksFormat /dev/sdb1
sudo cryptsetup open /dev/sdb1 neuroquantum-encrypted
sudo mkfs.ext4 /dev/mapper/neuroquantum-encrypted
sudo mount /dev/mapper/neuroquantum-encrypted /data/neuroquantum
```

**Application-Level Encryption:**
```toml
[encryption]
# Encrypt sensitive columns
enabled = true
algorithm = "aes-256-gcm"
key_derivation = "argon2id"

# Key management
master_key_file = "/secure/master.key"
rotate_keys_days = 90
```

---

### 3. Post-Quantum Cryptography

**Enable ML-KEM (Kyber):**
```toml
[security]
pqc_enabled = true
pqc_algorithm = "ml-kem"  # ML-KEM-768 (NIST Level 3)

# Hybrid mode (classical + quantum-resistant)
pqc_hybrid_mode = true
```

**Enable ML-DSA (Dilithium) for Signatures:**
```toml
[security]
signature_algorithm = "ml-dsa"  # ML-DSA-65 (NIST Level 3)
```

---

## Access Control (RBAC)

### Define Roles

```toml
[authorization]
roles = [
    { name = "admin", permissions = ["read", "write", "admin"] },
    { name = "developer", permissions = ["read", "write"] },
    { name = "analyst", permissions = ["read"] }
]
```

### Create User with Role

```bash
curl -X POST http://localhost:8080/api/v1/admin/users \
  -H "Authorization: Bearer YOUR_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "bob",
    "password": "secure_password",
    "role": "developer",
    "email": "bob@example.com"
  }'
```

### Permission Checks

Permissions are automatically enforced for all API endpoints:

| Endpoint | Required Permission |
|----------|-------------------|
| `GET /api/v1/query` (SELECT) | `read` |
| `POST /api/v1/query` (INSERT/UPDATE) | `write` |
| `POST /api/v1/admin/*` | `admin` |
| `GET /api/v1/stats` | `read` |

---

## Audit Logging

### Enable Audit Logs

```toml
[audit]
enabled = true
log_file = "/var/log/neuroquantum/audit.log"
rotation_size_mb = 100

# What to log
log_authentication = true
log_queries = true
log_admin_actions = true
log_failures = true
```

### Audit Log Format

```json
{
  "timestamp": "2025-11-04T12:34:56Z",
  "event_type": "authentication_success",
  "user_id": 1,
  "username": "alice",
  "ip_address": "192.168.1.100",
  "user_agent": "curl/7.88.1",
  "details": {
    "method": "api_key",
    "key_id": "key_123"
  }
}
```

### Query Audit Logs

```bash
# View recent audit events
tail -f /var/log/neuroquantum/audit.log | jq

# Search for failed authentications
grep "authentication_failure" /var/log/neuroquantum/audit.log | jq

# Count queries per user
jq -r 'select(.event_type == "query_executed") | .username' audit.log | sort | uniq -c
```

---

## Security Best Practices

### ✅ DO

1. **Use strong JWT secrets** - Generate with `generate-jwt-secret`
2. **Rotate API keys regularly** - Every 90 days minimum
3. **Enable TLS** - Always use HTTPS in production
4. **Whitelist admin IPs** - Limit access to admin endpoints
5. **Monitor audit logs** - Set up alerts for suspicious activity
6. **Enable rate limiting** - Protect against abuse
7. **Use environment variables** - Never commit secrets to Git
8. **Regular backups** - Test restore procedures
9. **Update dependencies** - Run `cargo audit` regularly
10. **Enable firewall** - Only expose necessary ports

### ❌ DON'T

1. **Never use default credentials** - None provided by design
2. **Never expose without TLS** - Man-in-the-middle attacks possible
3. **Never log secrets** - Use structured logging with redaction
4. **Never disable authentication** - Even in "development"
5. **Never trust user input** - All queries are parameterized
6. **Never use short JWT secrets** - Minimum 256 bits
7. **Never run as root** - Use dedicated user account
8. **Never skip backups** - Data loss is unrecoverable
9. **Never ignore security advisories** - Run `cargo audit`
10. **Never reuse API keys** - One key per application

---

## Security Checklist

### Pre-Production

- [ ] JWT secret generated and configured
- [ ] Admin API key created and stored securely
- [ ] IP whitelist configured for admin endpoints
- [ ] Rate limiting enabled with Redis
- [ ] TLS/HTTPS configured (via reverse proxy)
- [ ] Audit logging enabled
- [ ] Firewall rules configured
- [ ] User roles and permissions defined
- [ ] Backup strategy implemented
- [ ] Security advisories checked (`cargo audit`)

### Post-Deployment

- [ ] Monitor audit logs for suspicious activity
- [ ] Set up alerts for failed authentications
- [ ] Regular security scans (weekly)
- [ ] API key rotation (quarterly)
- [ ] Backup testing (monthly)
- [ ] Dependency updates (monthly)
- [ ] Penetration testing (annually)

---

## Common Security Issues

### Issue 1: "Unauthorized" on Admin Endpoints

**Symptom:** 401 Unauthorized when accessing `/api/v1/admin/*`

**Causes:**
1. IP not whitelisted
2. Invalid or expired API key
3. Missing admin role

**Solution:**
```bash
# Check your IP
curl ipinfo.io/ip

# Add to whitelist in config/prod.toml
admin_ip_whitelist = ["YOUR_IP_HERE"]

# Restart server
systemctl restart neuroquantum-api
```

---

### Issue 2: Rate Limit Exceeded

**Symptom:** 429 Too Many Requests

**Causes:**
1. Too many requests from same IP
2. Aggressive client retry logic
3. DDoS attack

**Solution:**
```toml
# Increase limits for legitimate traffic
[rate_limiting]
requests_per_hour = 2000  # Increase limit
burst = 100  # Increase burst

# Or whitelist specific IPs
[rate_limiting.whitelist]
ips = ["192.168.1.100"]
```

---

### Issue 3: JWT Token Expired

**Symptom:** 401 Unauthorized with "Token expired"

**Causes:**
1. Token older than `jwt_expiration_hours`
2. Server clock skew

**Solution:**
```bash
# Request new token
curl -X POST http://localhost:8080/api/v1/auth/login \
  -d '{"username": "alice", "password": "..."}'

# Or increase expiration time
[auth]
jwt_expiration_hours = 24  # 24 hours instead of 8
```

---

## Security Resources

- 📖 [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- 📖 [NIST Post-Quantum Cryptography](https://csrc.nist.gov/projects/post-quantum-cryptography)
- 📖 [Rust Security Advisories](https://rustsec.org/)
- 📖 [JWT Best Practices](https://tools.ietf.org/html/rfc8725)

---

## Next Steps

- 🚀 [Deployment Guide](../deployment/docker.md)
- 📊 [Monitoring Setup](../deployment/monitoring.md)
- 🔧 [Operations Guide](../operations/maintenance.md)

