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
| 6 | ⬜ | [ISSUE-001](issues/ISSUE-001-migration-executor.md) | Migration Executor implementieren | 8-16 Std |

### Sprint 4: SQL-Funktionalität (4-8 Stunden)

| # | Status | Issue | Beschreibung | Aufwand |
|---|--------|-------|--------------|---------|
| 7 | ⬜ | [ISSUE-009](issues/ISSUE-009-neuromatch-joins.md) | NEUROMATCH in JOINs fixen | 4-8 Std |

### Sprint 5: Cluster-Stabilität (32-56 Stunden)

| # | Status | Issue | Beschreibung | Aufwand |
|---|--------|-------|--------------|---------|
| 8 | ⬜ | [ISSUE-003](issues/ISSUE-003-cluster-rollback.md) | Cluster Rollback implementieren | 16-32 Std |
| 9 | ⬜ | [ISSUE-006](issues/ISSUE-006-anti-entropy-repair.md) | Anti-Entropy Repair | 16-24 Std |

### Sprint 6: Nice-to-Have (Optional)

| # | Status | Issue | Beschreibung | Aufwand |
|---|--------|-------|--------------|---------|
| 10 | ⬜ | [ISSUE-010](issues/ISSUE-010-wasm-dna-kompression.md) | WASM DNA-Kompression | 4-8 Std |
| 11 | ⬜ | [ISSUE-011](issues/ISSUE-011-foreign-key-constraints.md) | Foreign Key Constraints | 16-24 Std |
| 12 | ⬜ | [ISSUE-012](issues/ISSUE-012-query-plan-cache-eviction.md) | Query Plan Cache Eviction | 4-6 Std |
| 13 | ⬜ | [ISSUE-013](issues/ISSUE-013-response-metriken.md) | Response-Metriken vervollständigen | 4-8 Std |

---

## ✅ Bereits erledigt (GitHub Issues geschlossen)

Diese Tasks waren ursprünglich geplant, wurden aber bereits durch geschlossene GitHub Issues abgedeckt:

| Issue | Beschreibung | GitHub Issue |
|-------|--------------|--------------|
| ~~UPDATE ohne WHERE~~ | Fehlermeldung verbessern | [#128](https://github.com/issues/128) CLOSED |
| ~~Datum/Zeit-Funktionen~~ | CURRENT_DATE, NOW(), etc. | [#202](https://github.com/issues/202) CLOSED |
| ~~Rekursive CTEs~~ | WITH RECURSIVE | [#127](https://github.com/issues/127), [#201](https://github.com/issues/201) CLOSED |
| ~~Correlated Subqueries~~ | Subqueries mit Korrelation | [#192](https://github.com/issues/192) CLOSED |
| ~~Dead Code bereinigen~~ | Cluster-Module | [#314](https://github.com/issues/314) CLOSED |

---

## 📊 Fortschritt

| Sprint | Tasks | Erledigt | Status |
|--------|-------|----------|--------|
| Sprint 1: Quick Wins | 2 | 2 | ✅ |
| Sprint 2: Security | 2 | 2 | ✅ |
| Sprint 3: Bugs | 2 | 1 | 🔄 |
| Sprint 4: SQL | 1 | 0 | ⬜ |
| Sprint 5: Cluster | 2 | 0 | ⬜ |
| Sprint 6: Optional | 4 | 0 | ⬜ |
| **Gesamt** | **13** | **5** | **38%** |

---

## 🔗 Offene GitHub Issues (Cluster-Features)

Diese existieren bereits als GitHub Issues und sollten dort bearbeitet werden:

| GitHub # | Titel |
|----------|-------|
| #288 | Multi-Node Cluster-Core Storage Engine Integration |
| #289 | Distributed Query Routing for Multi-Node QSQL |
| #290 | Distributed Transactions with 2PC |
| #291 | Read Replica Support |
| #292 | Cluster-wide Backup |
| #293 | Multi-Datacenter Geo-Replication |
| #294 | Cluster-aware Connection Pooling |
| #295 | Distributed Lock Manager |
| #296 | Automatic Node Failure Detection |
| #297 | Cluster Monitoring Dashboard |
| #298 | REST API Cluster Integration |
| #299 | Cluster-wide Schema Synchronization |
| #308 | Connection Pooling im Cluster |
| #311 | Doppelte Serialisierung |
| #74 | rkyv vulnerability |
| #75 | paste dependency unmaintained |

---

## 🔧 Nützliche Befehle

```bash
# Alle Tests
cargo test --all

# Spezifisches Crate
cargo test -p neuroquantum-core
cargo test -p neuroquantum-qsql
cargo test -p neuroquantum-api
cargo test -p neuroquantum-cluster

# Build
cargo build --release

# Lints
cargo clippy --all-targets
cargo fmt --check
```
