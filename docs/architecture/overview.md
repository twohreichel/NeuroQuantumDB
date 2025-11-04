# Architecture Overview

NeuroQuantumDB combines three cutting-edge technologies into a unified database system.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   Client Applications                        │
│  (REST, WebSocket, CLI)                                     │
└────────────────┬────────────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────────────┐
│                  API Layer (Actix-Web)                       │
│  • Authentication (JWT, API Keys, EEG Biometric)            │
│  • Rate Limiting (Redis-backed)                             │
│  • Request Validation                                        │
│  • OpenAPI/Swagger Documentation                            │
└────────────────┬────────────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────────────┐
│                  QSQL Query Engine                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Parser  │→ │ Planner  │→ │Optimizer │→ │ Executor │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│  • SQL-compatible syntax                                    │
│  • Quantum/Neuromorphic extensions                          │
│  • Natural language preprocessing                           │
└────┬───────────────────┬───────────────────┬────────────────┘
     │                   │                   │
┌────▼──────────┐ ┌─────▼────────────┐ ┌───▼─────────────────┐
│   Quantum     │ │  Neuromorphic    │ │  DNA Compression    │
│   Processor   │ │  Learning Engine │ │  Engine             │
│               │ │                  │ │                     │
│ • Grover's    │ │ • STDP           │ │ • Quaternary Code   │
│   Search      │ │ • Hebbian        │ │ • Reed-Solomon ECC  │
│ • QAOA        │ │   Learning       │ │ • NEON SIMD         │
│ • Annealing   │ │ • Adaptive Index │ │ • 999:1 Ratio       │
│ • TFIM        │ │ • Pattern Learn  │ │ • Auto-detect       │
└───────────────┘ └──────────────────┘ └─────────────────────┘
                         │
        ┌────────────────▼────────────────────────────────┐
        │           Storage Engine                        │
        │  ┌─────────┐  ┌──────┐  ┌────────────┐         │
        │  │ B+ Tree │  │ WAL  │  │ Buffer Pool│         │
        │  └─────────┘  └──────┘  └────────────┘         │
        │  • Page Management                             │
        │  • Transaction Manager (MVCC)                  │
        │  • Recovery Manager (ARIES)                    │
        │  • Backup/Restore                              │
        └────────────────┬───────────────────────────────┘
                         │
        ┌────────────────▼───────────────────────────────┐
        │          Persistent Storage                    │
        │  (Disk/SSD/SD Card)                           │
        └────────────────────────────────────────────────┘
```

## Core Components

### 1. API Layer
- **REST API:** 17 endpoints with OpenAPI documentation
- **WebSocket:** Real-time query streaming and pub/sub
- **Authentication:** Multi-factor (JWT, API Keys, Biometric)
- **Rate Limiting:** Per-IP and per-user limits

### 2. QSQL Query Engine
- **Parser:** SQL-compatible with extensions
- **Planner:** Query execution plan generation
- **Optimizer:** Cost-based optimization + quantum/neuromorphic hints
- **Executor:** Multi-threaded execution

### 3. Quantum Processor
- **Grover's Search:** O(√N) search complexity
- **Quantum Annealing:** Optimization problems (QUBO, TFIM)
- **QAOA:** Approximate optimization

### 4. Neuromorphic Learning
- **STDP:** Spike-Timing-Dependent Plasticity
- **Hebbian Learning:** "Neurons that fire together, wire together"
- **Adaptive Indexes:** Self-optimizing based on access patterns

### 5. DNA Compression
- **Quaternary Encoding:** A, T, G, C (4 states per symbol)
- **Reed-Solomon ECC:** Error correction for noisy storage
- **NEON SIMD:** ARM64 hardware acceleration

### 6. Storage Engine
- **B+ Tree:** Clustered index with efficient range scans
- **WAL:** Write-Ahead Logging for crash recovery
- **Buffer Pool:** LRU cache for hot pages
- **MVCC:** Multi-Version Concurrency Control
- **ARIES Recovery:** Analysis, Redo, Undo phases

## Data Flow

### Query Execution

1. **Client Request** → API Layer
2. **Authentication** → Verify JWT/API Key
3. **Parse** → QSQL → AST (Abstract Syntax Tree)
4. **Plan** → Generate execution plan
5. **Optimize** → Apply quantum/neuromorphic optimizations
6. **Execute** → Fetch data from storage
7. **Compress** → DNA compression (if enabled)
8. **Response** → JSON/Binary result

### Write Path

1. **Client Write** → INSERT/UPDATE/DELETE
2. **Transaction Begin** → Acquire locks (MVCC)
3. **Write to WAL** → Durability guarantee
4. **Update B+ Tree** → In-memory modification
5. **Update Buffer Pool** → Mark page dirty
6. **Checkpoint** → Flush dirty pages to disk
7. **Transaction Commit** → Release locks

### Read Path

1. **Client Read** → SELECT query
2. **Check Buffer Pool** → Cache hit? Return immediately
3. **Read from Disk** → Cache miss? Load page
4. **Decompress** → DNA decompression (if compressed)
5. **Apply MVCC** → Filter by transaction visibility
6. **Return Result** → JSON/Binary response

## Technology Stack

- **Language:** Rust (Edition 2021)
- **Async Runtime:** Tokio
- **Web Framework:** Actix-Web
- **Serialization:** Bincode, Serde
- **Crypto:** Post-Quantum (ML-KEM, ML-DSA), Argon2, AES-GCM
- **Monitoring:** Prometheus
- **Testing:** 328+ tests (80%+ coverage)

## Performance Characteristics

### Raspberry Pi 4 (ARM64)
- **Startup Time:** < 5 seconds
- **Memory Usage:** < 100 MB
- **Power Consumption:** < 2W idle, < 3W peak
- **Query Latency:** ~10ms (cached), ~50ms (disk)
- **Throughput:** ~500 queries/second

### Server (x86_64, 16GB RAM)
- **Startup Time:** < 2 seconds
- **Memory Usage:** < 500 MB
- **Query Latency:** ~2ms (cached), ~10ms (disk)
- **Throughput:** ~10,000 queries/second

## Next Sections

- [DNA Compression](./dna-compression.md) - Deep dive into compression
- [Quantum Algorithms](./quantum-algorithms.md) - Grover, QAOA, Annealing
- [Neuromorphic Learning](./neuromorphic-learning.md) - STDP, Hebbian
- [Storage Engine](./storage-engine.md) - B+ Tree, WAL, MVCC
- [Transaction Management](./transaction-management.md) - ACID guarantees

