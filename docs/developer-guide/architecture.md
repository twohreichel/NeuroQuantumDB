# Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     REST / WebSocket API                     │
│                    (neuroquantum-api)                        │
├─────────────────────────────────────────────────────────────┤
│                      QSQL Engine                             │
│                    (neuroquantum-qsql)                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  Parser  │→ │Optimizer │→ │ Planner  │→ │ Executor │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │
├─────────────────────────────────────────────────────────────┤
│                     Core Engine                              │
│                   (neuroquantum-core)                        │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐   │
│  │  Storage  │ │    DNA    │ │  Quantum  │ │  Neural   │   │
│  │  Engine   │ │Compression│ │ Processor │ │  Network  │   │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘   │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐                  │
│  │Transaction│ │  Security │ │   SIMD    │                  │
│  │  Manager  │ │   Layer   │ │  Engine   │                  │
│  └───────────┘ └───────────┘ └───────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

```
Request → Auth → Rate Limit → Handler → QSQL → Storage → Response
                                          ↓
                              ┌───────────┴───────────┐
                              │                       │
                           B+Tree              DNA Compression
                              │                       │
                             WAL                   SIMD
```

## Key Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Zero-copy** | Memory-mapped I/O where possible |
| **Lock-free** | Atomic operations for hot paths |
| **SIMD-first** | ARM NEON / x86 AVX2 acceleration |
| **Fail-safe** | WAL for crash recovery |

## Crate Dependencies

```
neuroquantum-api
       │
       ├──→ neuroquantum-qsql
       │           │
       ├──→ neuroquantum-cluster (Beta)
       │           │
       └───────────┴──→ neuroquantum-core
```

## Cluster Architecture (Beta)

⚠️ **WARNING: The cluster module (`neuroquantum-cluster`) is currently in Beta/Preview status and is NOT production-ready.**

### Current Implementation Status

The cluster crate provides a foundation for distributed deployments but is incomplete:

```
┌─────────────────────────────────────────────────────────────┐
│              neuroquantum-cluster (Beta Module)              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ✅ Implemented:                                            │
│     • Basic node management                                 │
│     • Configuration structure                               │
│     • Consistent hashing for sharding                       │
│     • Basic cluster state tracking                          │
│                                                             │
│  ❌ Missing / Incomplete:                                   │
│     • gRPC network transport (partial)                      │
│     • Complete Raft consensus implementation                │
│     • Service discovery (DNS/Consul/etcd)                   │
│     • Full replication guarantees                           │
│     • Network partition handling                            │
│     • Distributed transaction coordination                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Planned Cluster Architecture (2026)

The complete cluster implementation is planned as part of the 2026 roadmap:

```
┌─────────────────────────────────────────────────────────────┐
│                  Future: Production Cluster                  │
│                                                             │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│   │   Node 1    │    │   Node 2    │    │   Node 3    │    │
│   │  (Leader)   │◄──►│  (Follower) │◄──►│  (Follower) │    │
│   └─────────────┘    └─────────────┘    └─────────────┘    │
│         │                  │                  │             │
│         └──────────────────┼──────────────────┘             │
│                            │                                │
│                    ┌───────▼───────┐                        │
│                    │  Raft Consensus│                       │
│                    │  + gRPC        │                       │
│                    │  + Discovery   │                       │
│                    └───────────────┘                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Production Deployment Guidance

| Feature | Single-Node | Multi-Node Cluster |
|---------|-------------|-------------------|
| **Production Use** | ✅ Recommended | ❌ Not Ready |
| **Data Durability** | ✅ WAL + Backups | ⚠️ Limited |
| **High Availability** | ❌ Single point of failure | ⚠️ Incomplete |
| **Horizontal Scaling** | ❌ Vertical only | ⚠️ Beta |
| **Operational Complexity** | ✅ Low | ⚠️ High (experimental) |

**Recommendation:** Use single-node deployment for all production workloads. The cluster module can be explored in development environments but should not be relied upon for production systems.

### Roadmap for Cluster Completion

See [Future Vision - 2026: Distributed Architecture](../concept/06-future-vision.md#mid-term-2026-distributed-architecture) for detailed plans including:

- Neural consensus mechanisms
- Synaptic sharding algorithms
- Federated learning across nodes
- Complete Raft implementation with gRPC

---

## Component Details
