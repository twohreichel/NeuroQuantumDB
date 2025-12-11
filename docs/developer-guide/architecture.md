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
       └───────────┴──→ neuroquantum-core
```
