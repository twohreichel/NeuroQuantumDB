# Dependency Security Report

**Date:** December 15, 2025  
**Project:** NeuroQuantumDB  
**Created by:** Automated Security Audit

## Summary

This document records the status of dependencies after the security update. All directly controllable packages have been updated to the latest secure versions.

### ✅ Successfully Updated Packages

| Package | Old Version | New Version | Crate |
|---------|-------------|-------------|-------|
| tokio | 1.47.1 | 1.48.0 | Workspace |
| serde | 1.0.225 | 1.0.228 | Workspace |
| anyhow | 1.0.99 | 1.0.100 | Workspace |
| thiserror | 2.0.16 | 2.0.17 | Workspace |
| tracing | 0.1.41 | 0.1.43 | Workspace |
| tracing-subscriber | 0.3.20 | 0.3.22 | Workspace |
| uuid | 1.18.1 | 1.19.0 | Workspace |
| config | 0.15.16 | 0.15.19 | Workspace |
| clap | 4.5 | 4.5.53 | Workspace |
| reqwest | 0.11 | 0.12 | Workspace |
| rayon | 1.8 | 1.11 | neuroquantum-core |
| hashbrown | 0.14 | 0.16 | neuroquantum-core |
| crc32fast | 1.3 | 1.5 | neuroquantum-core |
| nalgebra | 0.32 | 0.34 | neuroquantum-core |
| num-complex | 0.4 | 0.4.6 | neuroquantum-core |
| criterion | 0.7.0 | 0.8 | neuroquantum-core |
| async-trait | 0.1 | 0.1.89 | neuroquantum-core |
| aws-config | 1.5 | 1.8 | neuroquantum-core |
| aws-sdk-s3 | 1.60 | 1.117 | neuroquantum-core |
| flate2 | 1.0 | 1.1 | neuroquantum-core |
| lz4_flex | 0.11 | 0.12 | neuroquantum-core |
| lru | 0.12 | 0.16 | neuroquantum-core |
| pqcrypto-traits | 0.3 | 0.3.5 | neuroquantum-core |
| zeroize | 1.7 | 1.8 | neuroquantum-core |
| rustfft | 6.1 | 6.4 | neuroquantum-core |
| sysinfo | 0.30 | 0.37 | neuroquantum-core |
| proptest | 1.4 | 1.9 | neuroquantum-core (dev) |
| tempfile | 3.8 | 3.23 | neuroquantum-core (dev) |
| actix-web | 4.11.0 | 4.12 | neuroquantum-api |
| validator | 0.19.0 | 0.20 | neuroquantum-api |
| once_cell | 1.20 | 1.21 | neuroquantum-api |
| sysinfo | 0.33 | 0.37 | neuroquantum-api |
| serde_yaml | 0.9 (deprecated) | serde_yaml_ng 0.10 | neuroquantum-qsql |
| regex | 1.11.2 | 1.12 | neuroquantum-qsql |
| pulldown-cmark | 0.9 | 0.13 | scripts |
| arbitrary | 1 | 1.4 | fuzz |

---

## ⚠️ Transitive Dependencies with Security Warnings

The following warnings concern **transitive dependencies** that are introduced by other packages and cannot be directly controlled.

### 1. `instant` (RUSTSEC-2024-0384)

**Status:** Unmaintained  
**Severity:** Warning (no active security vulnerability)  
**Date:** September 1, 2024  
**URL:** https://rustsec.org/advisories/RUSTSEC-2024-0384

**Dependency chain:**
```
instant 0.1.13
├── parking_lot_core 0.8.6
│   └── parking_lot 0.11.2
│       └── reed-solomon-erasure 6.0.0
│           └── neuroquantum-core
```

**Issue:**  
The `instant` crate is no longer maintained. It is used by `parking_lot 0.11`, which in turn is a dependency of `reed-solomon-erasure 6.0.0`.

**Recommended Actions:**

1. **Short-term:** The risk is low since this is an "unmaintained" warning, not an active security vulnerability.

2. **Medium-term:** Check if `reed-solomon-erasure` releases an update that uses `parking_lot 0.12+`:
   ```bash
   cargo tree -i parking_lot
   ```

3. **Long-term - Alternative Libraries:**
   - **`reed-solomon-simd`** (https://crates.io/crates/reed-solomon-simd) - Modern, SIMD-optimized alternative
   - **`reed-solomon-16`** - Optimized for 16-bit Galois fields
   - **`reed-solomon-novelpoly`** - Optimized for very large datasets

**Code Adjustment When Switching to `reed-solomon-simd`:**
```rust
// Old (reed-solomon-erasure):
use reed_solomon_erasure::galois_8::ReedSolomon;

// New (reed-solomon-simd):
use reed_solomon_simd::ReedSolomonEncoder;
use reed_solomon_simd::ReedSolomonDecoder;
```

---

### 2. `paste` (RUSTSEC-2024-0436)

**Status:** Unmaintained  
**Severity:** Warning (no active security vulnerability)  
**Date:** October 7, 2024  
**URL:** https://rustsec.org/advisories/RUSTSEC-2024-0436

**Dependency chains:**
```
paste 1.0.15
├── simba 0.9.1
│   └── nalgebra 0.34.1
│       └── neuroquantum-core
└── pqcrypto-mldsa 0.1.2
    └── neuroquantum-core
```

**Issue:**  
The `paste` crate (macro for token concatenation) is no longer actively maintained. It is used by:
- `simba` (abstraction for numeric types, dependency of `nalgebra`)
- `pqcrypto-mldsa` (Post-Quantum Cryptography)

**Recommended Actions:**

1. **Short-term:** The risk is minimal since `paste` is a procedural macro that is only used at compile time and has no runtime impact.

2. **Medium-term:** Wait for updates from:
   - `nalgebra` or `simba`
   - `pqcrypto-mldsa`

3. **Long-term - Alternatives for `nalgebra`:**
   - **`ndarray`** (https://crates.io/crates/ndarray) - For N-dimensional arrays, does not use `paste`
   - **`faer`** (https://crates.io/crates/faer) - Modern linear algebra library
   - **`glam`** (https://crates.io/crates/glam) - For 3D math, lightweight

4. **Long-term - Alternatives for `pqcrypto-mldsa`:**
   - **`ml-dsa`** (https://crates.io/crates/ml-dsa) - RustCrypto implementation (once stable)
   - **`pqcrypto3`** - More modern version of the pqcrypto suite

**Note on Post-Quantum Cryptography:**
The `pqcrypto` crates are based on the liboqs C library. A fully Rust-written alternative would be preferable. The RustCrypto group is working on `ml-dsa` as a pure-Rust alternative.

---

## 📊 Audit Status

```
Crate dependencies scanned:    580
Critical vulnerabilities:      0
High vulnerabilities:          0
Medium vulnerabilities:        0
Warnings (unmaintained):       2
```

---

## 🔄 Regular Maintenance

Recommended regular security checks:

```bash
# Weekly: Run security audit
cargo audit

# Monthly: Check for outdated packages
cargo outdated -R

# As needed: Update dependencies
cargo update
```

---

## 📝 Changelog

### 2025-12-15

- Updated all direct dependencies to latest versions
- Replaced `serde_yaml` (deprecated) with `serde_yaml_ng`
- Updated `reqwest` from 0.11 to 0.12 with `rustls-tls` feature
- Updated AWS SDK to latest versions (aws-config 1.8.12, aws-sdk-s3 1.117.0)
- Verified all cryptography libraries (ml-kem, sha3, argon2, aes-gcm, etc.)
- Created documentation for non-fixable transitive dependencies

---

## 🔒 Security Notes

1. **No critical vulnerabilities:** The project contains no known critical or high security vulnerabilities.

2. **Transitive dependencies:** The remaining warnings concern "unmaintained" packages, not active security vulnerabilities.

3. **Post-Quantum Cryptography:** The PQC libraries used (ml-kem, pqcrypto-mldsa) are implemented according to current NIST standards.

4. **TLS:** The project uses `rustls` instead of OpenSSL for TLS connections.
