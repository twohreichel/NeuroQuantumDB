# NeuroQuantumDB

> **Ultra-efficient neuromorphic database for edge computing**

NeuroQuantumDB combines three revolutionary technologies:

| Technology | Description |
|------------|-------------|
| 🧬 **DNA Compression** | 4:1 compression using quaternary encoding (A,C,G,T) |
| ⚛️ **Quantum Algorithms** | Grover's search, QUBO optimization |
| 🧠 **Neuromorphic Learning** | Hebbian learning, STDP, lateral inhibition |

## Key Features

- **Post-Quantum Cryptography** — ML-KEM-768/1024, ML-DSA (NIST FIPS 203/204)
- **ACID Transactions** — Full WAL support with crash recovery
- **REST & WebSocket API** — HTTP/2 with real-time streaming
- **Biometric Authentication** — EEG-based security
- **ARM64 Optimized** — NEON SIMD for Raspberry Pi 4

## Quick Start

```bash
# Build
cargo build --release

# Initialize
./target/release/neuroquantum-api init

# Run
./target/release/neuroquantum-api
```

API available at `http://localhost:8080`

## Documentation Structure

| Section | Audience |
|---------|----------|
| [User Guide](user-guide/installation.md) | End users, DevOps |
| [Developer Guide](developer-guide/architecture.md) | Contributors, Integrators |
| [Reference](reference/api.md) | API consumers |
