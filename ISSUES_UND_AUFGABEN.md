# NeuroQuantumDB - Aufgabenliste

**Stand:** 19. Januar 2026  
**Issue-Ordner:** `issues/`

---

## 📋 Taskliste (optimierte Reihenfolge)

### Sprint 1: Quick Wins (< 2 Stunden)

| # | Status | Issue | Beschreibung | Aufwand |
|---|--------|-------|--------------|---------|
| 1 | ✅ | [ISSUE-005](issues/ISSUE-005-jwt-secret-validierung.md) | JWT-Secret Validierung | 30 Min |
| 2 | ✅ | [ISSUE-007](issues/ISSUE-007-pre-commit-hook-simd.md) | Pre-commit Hook für SIMD | 30 Min |

### Sprint 2: Security & API (4-6 Stunden)

| # | Status | Issue | Beschreibung | Aufwand |
|---|--------|-------|--------------|---------|
| 3 | ✅ | [ISSUE-004](issues/ISSUE-004-eeg-authservice-persistenz.md) | EEG-AuthService Persistenz | 2-4 Std |
| 4 | ✅ | [ISSUE-008](issues/ISSUE-008-auto-increment-truncate.md) | Auto-Increment Reset bei TRUNCATE | 2-4 Std |

### Sprint 3: Kritische Bugs (8-16 Stunden)

| # | Status | Issue | Beschreibung | Aufwand |
|---|--------|-------|--------------|---------|
| 5 | ✅ | [ISSUE-002](issues/ISSUE-002-dna-compression-float-bug.md) | DNA Compression Float-Bug | 8-16 Std |
| 6 | ✅ | [ISSUE-001](issues/ISSUE-001-migration-executor.md) | Migration Executor implementieren | 8-16 Std |

### Sprint 4: SQL-Funktionalität (4-8 Stunden)

| # | Status | Issue | Beschreibung | Aufwand |
|---|--------|-------|--------------|---------|
| 7 | ✅ | [ISSUE-009](issues/ISSUE-009-neuromatch-joins.md) | NEUROMATCH in JOINs fixen | 4-8 Std |

### Sprint 6: Nice-to-Have (Optional)

| # | Status | Issue | Beschreibung | Aufwand |
|---|--------|-------|--------------|---------|
| 10 | ✅ | [ISSUE-010](issues/ISSUE-010-wasm-dna-kompression.md) | WASM DNA-Kompression | 4-8 Std |
| 11 | ✅ | [ISSUE-011](issues/ISSUE-011-foreign-key-constraints.md) | Foreign Key Constraints | 16-24 Std |
| 12 | ✅ | [ISSUE-012](issues/ISSUE-012-query-plan-cache-eviction.md) | Query Plan Cache Eviction | 4-6 Std |
| 13 | ⬜ | [ISSUE-013](issues/ISSUE-013-response-metriken.md) | Response-Metriken vervollständigen | 4-8 Std |

---

## 📊 Fortschritt

| Sprint | Tasks | Erledigt | Status |
|--------|-------|----------|--------|
| Sprint 1: Quick Wins | 2 | 2 | ✅ |
| Sprint 2: Security | 2 | 2 | ✅ |
| Sprint 3: Bugs | 2 | 2 | ✅ |
| Sprint 4: SQL | 1 | 1 | ✅ |
| Sprint 5: Optional | 4 | 3 | 🔄 |
| **Gesamt** | **13** | **10** | **77%** |

---

## 🔧 Nützliche Befehle

```bash
# Alle Tests
cargo test --all

# Spezifisches Crate
cargo test -p neuroquantum-core
cargo test -p neuroquantum-qsql
cargo test -p neuroquantum-api

# Build
cargo build --release

# Lints
cargo clippy --all-targets
cargo fmt --check
```
