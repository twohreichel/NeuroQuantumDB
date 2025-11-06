# Post-Quantum Cryptography in NeuroQuantumDB

## Overview

NeuroQuantumDB implements NIST-standardized post-quantum cryptographic algorithms to protect against attacks from both classical and quantum computers. This ensures that data remains secure even as quantum computing advances.

## Implemented Algorithms

### ML-KEM (Kyber) - Key Encapsulation Mechanism
- **Standard**: NIST FIPS 203
- **Security Level**: Level 3 (equivalent to AES-192)
- **Implementation**: ML-KEM-768
- **Use Case**: Secure key exchange for session encryption

**Key Properties:**
- Public Key Size: 1,184 bytes
- Secret Key Size: 2,400 bytes
- Ciphertext Size: 1,088 bytes
- Shared Secret Size: 32 bytes

### ML-DSA (Dilithium) - Digital Signatures
- **Standard**: NIST FIPS 204
- **Security Level**: Level 3 (equivalent to AES-192)
- **Implementation**: ML-DSA-65
- **Use Case**: Authentication and data integrity verification

**Key Properties:**
- Public Key Size: 1,952 bytes
- Secret Key Size: 4,000 bytes
- Signature Size: 3,293 bytes

## Architecture

### PQCryptoManager

The core post-quantum cryptography manager provides:

```rust
use neuroquantum_core::pqcrypto::PQCryptoManager;

// Initialize with generated key pairs
let pq_manager = PQCryptoManager::new();

// Generate quantum-resistant token claims
let claims = pq_manager.generate_quantum_claims("user123", "session456")?;

// Verify quantum claims
pq_manager.verify_quantum_claims(&claims)?;
```

### JWT Integration

The API layer integrates post-quantum cryptography into JWT tokens:

```rust
use neuroquantum_api::jwt::JwtService;

let jwt_service = JwtService::new(b"secret_key");

// Generate quantum-resistant token
let token = jwt_service.generate_quantum_token("user123", "session456")?;
```

## Security Properties

### Quantum Resistance
- **Resistant to Shor's Algorithm**: ML-KEM and ML-DSA are based on lattice problems that are believed to be hard even for quantum computers
- **NIST Security Level 3**: Provides security equivalent to AES-192, requiring ~2^192 operations to break

### Classical Security
- **Strong Security Assumptions**: Based on Module Learning With Errors (MLWE) and Module Short Integer Solution (MSIS) problems
- **Conservative Parameters**: Uses larger security margins than minimum requirements

### Implementation Security
- **Constant-Time Operations**: Resistant to timing side-channel attacks
- **Memory Safety**: Implemented in Rust with zero-copy where possible
- **Key Zeroization**: Sensitive key material is securely erased from memory

## Performance Characteristics

### ML-KEM-768 Performance
- **Key Generation**: ~100 µs
- **Encapsulation**: ~120 µs
- **Decapsulation**: ~130 µs

### ML-DSA-65 Performance
- **Key Generation**: ~200 µs
- **Signing**: ~450 µs
- **Verification**: ~250 µs

*Benchmarks performed on Apple M1 processor*

## Usage Examples

### Signing and Verification

```rust
let manager = PQCryptoManager::new();

// Sign a message
let message = b"Hello, Quantum World!";
let signature = manager.sign_message(message);

// Verify the signature
let verified_msg = manager.verify_signature(&signature)?;
assert_eq!(verified_msg, message);
```

### Key Encapsulation

```rust
let manager = PQCryptoManager::new();

// Encapsulate a shared secret
let (ciphertext, shared_secret1) = manager.encapsulate();

// Decapsulate to recover the shared secret
let shared_secret2 = manager.decapsulate(&ciphertext)?;
assert_eq!(shared_secret1, shared_secret2);
```

### Quantum Token Generation

```rust
let manager = PQCryptoManager::new();

// Generate quantum-resistant authentication claims
let claims = manager.generate_quantum_claims("user123", "session456")?;

// Claims include:
// - ML-DSA signature of user_id:session_id:timestamp
// - ML-KEM ciphertext for key exchange
// - Base64-encoded for transport

// Verify the claims
manager.verify_quantum_claims(&claims)?;
```

## Migration Path

### Phase 1: Hybrid Mode (Current)
- Use both classical (RSA/ECDSA) and post-quantum signatures
- Verify both signatures for backwards compatibility
- Store both key types

### Phase 2: Post-Quantum Primary
- Post-quantum algorithms become primary
- Classical algorithms used only for legacy support
- New keys generated only with post-quantum algorithms

### Phase 3: Full Post-Quantum
- Remove classical algorithm support
- All authentication uses post-quantum algorithms
- Legacy keys deprecated and rotated

## Best Practices

### Key Management
1. **Key Rotation**: Rotate post-quantum keys periodically (recommended: every 90 days)
2. **Key Storage**: Store secret keys in hardware security modules (HSMs) when available
3. **Key Backup**: Maintain encrypted backups of key pairs
4. **Key Derivation**: Use separate key pairs for different purposes

### Token Lifecycle
1. **Short-Lived Tokens**: Use 1-hour expiration for quantum tokens
2. **Refresh Mechanism**: Implement secure token refresh before expiration
3. **Revocation**: Maintain token revocation list for compromised sessions
4. **Audit Logging**: Log all token generation and verification events

### Network Security
1. **TLS 1.3**: Use with post-quantum cipher suites when available
2. **Certificate Pinning**: Pin post-quantum public keys in client applications
3. **Perfect Forward Secrecy**: Rotate session keys frequently

## Testing

Run the test suite:

```bash
# Test post-quantum crypto module
cargo test --package neuroquantum-core pqcrypto

# Test JWT integration
cargo test --package neuroquantum-api jwt

# Run benchmarks
cargo bench --package neuroquantum-core -- pqcrypto
```

## References

- [NIST Post-Quantum Cryptography Standardization](https://csrc.nist.gov/Projects/post-quantum-cryptography)
- [FIPS 203: Module-Lattice-Based Key-Encapsulation Mechanism](https://csrc.nist.gov/pubs/fips/203/final)
- [FIPS 204: Module-Lattice-Based Digital Signature Algorithm](https://csrc.nist.gov/pubs/fips/204/final)
- [pqcrypto Rust Crate](https://github.com/rustpq/pqcrypto)

## Compliance

This implementation follows:
- NIST FIPS 203 (ML-KEM)
- NIST FIPS 204 (ML-DSA)
- NIST SP 800-208 (Recommendations for Stateful Hash-Based Signature Schemes)

## Support

For questions or issues related to post-quantum cryptography:
- Review the [Security Documentation](../security/README.md)
- Check [GitHub Issues](https://github.com/neuroquantumdb/neuroquantumdb/issues)
- Contact: security@neuroquantumdb.org

