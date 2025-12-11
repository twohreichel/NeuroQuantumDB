# Project Structure

```
neuroquantumdb/
├── crates/
│   ├── neuroquantum-core/     # Core engine
│   │   └── src/
│   │       ├── dna/           # DNA compression
│   │       │   └── simd/      # SIMD implementations
│   │       ├── quantum/       # Quantum algorithms
│   │       ├── storage/       # Storage engine
│   │       │   ├── btree/     # B+Tree index
│   │       │   ├── buffer/    # Buffer pool
│   │       │   ├── wal/       # Write-ahead log
│   │       │   └── backup/    # Backup system
│   │       ├── synaptic.rs    # Neural network
│   │       ├── learning.rs    # Hebbian learning
│   │       └── plasticity.rs  # Plasticity matrix
│   │
│   ├── neuroquantum-qsql/     # Query engine
│   │   └── src/
│   │       ├── parser.rs      # QSQL parser
│   │       ├── optimizer.rs   # Query optimizer
│   │       ├── executor.rs    # Execution engine
│   │       └── query_plan.rs  # Plan generation
│   │
│   └── neuroquantum-api/      # REST API
│       └── src/
│           ├── handlers.rs    # HTTP handlers
│           ├── websocket/     # WebSocket support
│           ├── middleware.rs  # Auth, rate limit
│           └── biometric_auth.rs
│
├── config/                    # Configuration files
├── docker/                    # Docker setup
├── docs/                      # This documentation
└── scripts/                   # Build scripts
```

## Module Responsibilities

| Module | Responsibility |
|--------|----------------|
| `dna` | Quaternary encoding, compression |
| `quantum` | Grover, QUBO, TFIM, parallel tempering |
| `storage` | Persistence, indexing, WAL |
| `synaptic` | Neural network topology |
| `learning` | Hebbian rules, STDP |
| `pqcrypto` | ML-KEM, ML-DSA |
