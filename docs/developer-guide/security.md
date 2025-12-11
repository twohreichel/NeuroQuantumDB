# Security

## Cryptography

### Post-Quantum Algorithms

| Algorithm | Standard | Purpose |
|-----------|----------|---------|
| ML-KEM-768 | NIST FIPS 203 | Key encapsulation |
| ML-KEM-1024 | NIST FIPS 203 | High-security KEM |
| ML-DSA-65 | NIST FIPS 204 | Digital signatures |
| ML-DSA-87 | NIST FIPS 204 | High-security signatures |

### Symmetric Encryption

- **AES-256-GCM** for data at rest
- **ChaCha20-Poly1305** alternative

## Authentication

### API Keys

```rust
pub struct ApiKey {
    pub id: Uuid,
    pub key_hash: String,      // bcrypt hash
    pub permissions: Vec<String>,
    pub expires_at: DateTime<Utc>,
}
```

### JWT

- Algorithm: HS256 (configurable)
- Expiration: Configurable (default 8 hours)
- Key rotation: 90 days with grace period

## Security Headers

```rust
// Applied to all responses
"Strict-Transport-Security: max-age=31536000; includeSubDomains"
"X-Content-Type-Options: nosniff"
"X-Frame-Options: DENY"
"Content-Security-Policy: default-src 'none'; ..."
"X-XSS-Protection: 1; mode=block"
```

## Rate Limiting

Token bucket algorithm:

```rust
pub struct RateLimiter {
    requests_per_window: u32,
    window_seconds: u64,
}
```

## Secret Management

All secrets use `Zeroize`:

```rust
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretKey {
    inner: Vec<u8>,
}
```

## Best Practices

1. **Never log secrets**
2. **Use constant-time comparison**
3. **Rotate keys regularly**
4. **Validate all inputs**
5. **Use prepared statements**
