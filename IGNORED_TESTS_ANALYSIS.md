# NeuroQuantumDB - Ignorierte und Fehlgeschlagene Tests Analyse

> **Erstellt:** 20. Januar 2026  
> **Ziel:** Detaillierte Analyse aller ignorierten Tests zur systematischen Behebung

---

## 📊 Übersichtstabelle

| ID | Status | Test | Kategorie | Grund | Priorität |
|----|--------|------|-----------|-------|-----------|
| T01 | ✅ DONE | `test_recursive_cte_employee_hierarchy` | Parser | Recursive CTE implementiert | 🔴 Hoch |
| T02 | ✅ DONE | `test_recursive_cte_generate_series` | Parser | Recursive CTE implementiert | 🔴 Hoch |
| T03 | ✅ DONE | `test_recursive_cte_graph_traversal` | Parser | Recursive CTE implementiert | 🔴 Hoch |
| T04 | ✅ DONE | `test_recursive_cte_union_semantics` | Parser | Recursive CTE implementiert | 🔴 Hoch |
| T05 | ✅ DONE | `test_recursive_cte_depth_limit` | Parser | Recursive CTE implementiert | 🔴 Hoch |
| T06 | ✅ DONE | `test_recursive_cte_with_column_list` | Parser | Recursive CTE implementiert | 🔴 Hoch |
| T07 | ✅ DONE | `test_recursive_cte_with_multiple_ctes` | Parser | Recursive CTE implementiert | 🔴 Hoch |
| T08 | ⬜ TODO | `benchmark_1m_inserts` | Performance | Benchmark überschreitet Zeitlimit (37s > 30s) | 🟠 Mittel |
| T09 | ⬜ TODO | `benchmark_point_lookup` | Performance | Lang-laufender Benchmark | 🟢 Niedrig |
| T10 | ⬜ TODO | `benchmark_range_scan` | Performance | Lang-laufender Benchmark | 🟢 Niedrig |
| T11 | ⬜ TODO | `test_read_throughput_scaling` | Load Tests | Lang-laufender Load-Test | 🟢 Niedrig |
| T12 | ⬜ TODO | `test_write_throughput_scaling` | Load Tests | Lang-laufender Load-Test | 🟢 Niedrig |
| T13 | ⬜ TODO | `test_sustained_load_stability` | Load Tests | Lang-laufender Load-Test | 🟢 Niedrig |
| T14 | ⬜ TODO | `test_load_test_summary` | Load Tests | Zusammenfassungstest | 🟢 Niedrig |
| T15 | ⬜ TODO | `test_concurrent_transactions_crash` | Chaos Engineering | Lang-laufender Test | 🟢 Niedrig |
| T16 | ⬜ TODO | `test_acid_properties_after_crash` | Chaos Engineering | Lang-laufender Test | 🟢 Niedrig |
| T17 | ⬜ TODO | `test_repeated_crash_recovery_cycles` | Chaos Engineering | Sehr lang-laufender Test | 🟢 Niedrig |
| T18 | ⬜ TODO | `test_chaos_random_node_kills` | Cluster E2E | Lang-laufender Test | 🟢 Niedrig |
| T19 | ⬜ TODO | `test_chaos_concurrent_load_with_failures` | Cluster E2E | Lang-laufender Test | 🟢 Niedrig |
| D01 | ⬜ TODO | Doc-Test: `permissions.rs` line 8 | Doc-Tests | Fehlender Import/Modul-Struktur | 🟠 Mittel |
| D02 | ⬜ TODO | Doc-Test: `lib.rs` line 113 | Doc-Tests | Async/Storage-Kontext fehlt | 🟠 Mittel |
| D03 | ⬜ TODO | Doc-Test: `concurrency.rs` lines 64,84,100,219,245 | Doc-Tests | Async/Kontext-Probleme | 🟠 Mittel |
| D04 | ⬜ TODO | Doc-Test: `quantum/mod.rs` lines 65,89,119 | Doc-Tests | Async/Kontext-Probleme | 🟠 Mittel |
| D05 | ⬜ TODO | Doc-Test: `quantum/backends/dwave.rs` line 22 | Doc-Tests | API-Token-Abhängigkeit | 🟢 Niedrig |
| D06 | ⬜ TODO | Doc-Test: `quantum/backends/ibm.rs` line 21 | Doc-Tests | API-Token-Abhängigkeit | 🟢 Niedrig |
| D07 | ⬜ TODO | Doc-Test: `quantum/backends/braket.rs` line 23 | Doc-Tests | AWS-Credentials-Abhängigkeit | 🟢 Niedrig |
| D08 | ⬜ TODO | Doc-Test: `quantum/backends/ionq.rs` line 28 | Doc-Tests | API-Key-Abhängigkeit | 🟢 Niedrig |
| D09 | ⬜ TODO | Doc-Test: `quantum/backends/mod.rs` line 43 | Doc-Tests | Fehlende Imports | 🟠 Mittel |
| D10 | ⬜ TODO | Doc-Test: `quantum/grover_hardware_backends.rs` line 34 | Doc-Tests | Async/API-Abhängigkeit | 🟢 Niedrig |
| D11 | ⬜ TODO | Doc-Test: `quantum/parallel_tempering_hardware_backends.rs` line 41 | Doc-Tests | Async/API-Abhängigkeit | 🟢 Niedrig |
| D12 | ⬜ TODO | Doc-Test: `quantum/qubo_hardware_backends.rs` line 35 | Doc-Tests | Async/API-Abhängigkeit | 🟢 Niedrig |
| D13 | ⬜ TODO | Doc-Test: `quantum/tfim_hardware_backends.rs` line 35 | Doc-Tests | Async/API-Abhängigkeit | 🟢 Niedrig |
| D14 | ⬜ TODO | Doc-Test: `quantum/tfim_unified.rs` line 9 | Doc-Tests | Fehlender Kontext | 🟠 Mittel |
| D15 | ⬜ TODO | Doc-Test: `storage.rs` line 896 (drop_table) | Doc-Tests | Async/Storage-Kontext fehlt | 🟠 Mittel |
| D16 | ⬜ TODO | Doc-Test: `storage.rs` line 1034 (alter_table) | Doc-Tests | Async/Storage-Kontext fehlt | 🟠 Mittel |
| D17 | ⬜ TODO | Doc-Test: `storage.rs` line 1279 (reset_auto_increment) | Doc-Tests | Async/Storage-Kontext fehlt | 🟠 Mittel |
| D18 | ⬜ TODO | Doc-Test: `storage.rs` line 1472 (insert_row) | Doc-Tests | Async/Storage-Kontext fehlt | 🟠 Mittel |
| D19 | ⬜ TODO | Doc-Test: `storage/buffer/mod.rs` line 238 | Doc-Tests | Async/Kontext fehlt | 🟠 Mittel |
| D20 | ⬜ TODO | Doc-Test: `storage/encryption.rs` line 151 | Doc-Tests | Async/Kontext fehlt | 🟠 Mittel |
| D21 | ⬜ TODO | Doc-Test: `storage/migration/executor.rs` line 64 | Doc-Tests | Fehlender Executor-Kontext | 🟠 Mittel |
| D22 | ⬜ TODO | Doc-Test: `storage/migration/mod.rs` line 39 | Doc-Tests | Fehlender SqlExecutor-Kontext | 🟠 Mittel |

---

## 📋 Detaillierte Task-Liste

---

### T01: `test_recursive_cte_employee_hierarchy` ✅ ERLEDIGT

**Status:** ✅ Implementiert und Test aktiviert

**Datei:** `crates/neuroquantum-qsql/tests/recursive_cte_tests.rs:45`

**Lösung implementiert:**
1. Parser erweitert um `WITH RECURSIVE` Syntax zu unterstützen
2. `is_recursive` Flag im CTE-AST korrekt gesetzt
3. Executor implementiert für rekursive CTE-Ausführung mit UNION/UNION ALL
4. Spezielle Behandlung für `level` Keyword als Spaltenname
5. IS NULL WHERE-Klausel Unterstützung hinzugefügt
6. Literal-Expression-Evaluierung (z.B. `1 as level`) implementiert

**Betroffene Dateien:**
- `crates/neuroquantum-qsql/src/parser.rs` - Parser-Erweiterung
- `crates/neuroquantum-qsql/src/query_plan.rs` - Ausführungsplan

---

### T02: `test_recursive_cte_generate_series` ✅ ERLEDIGT

**Status:** ✅ Implementiert und Test aktiviert

**Datei:** `crates/neuroquantum-qsql/tests/recursive_cte_tests.rs:150`

**Lösung implementiert:**
- Parser/Executor-Änderungen wie T01
- Numerische Iteration funktioniert korrekt
- Terminierungsbedingung wird korrekt evaluiert

---

### T03: `test_recursive_cte_graph_traversal` ✅ ERLEDIGT

**Status:** ✅ Implementiert und Test aktiviert

**Datei:** `crates/neuroquantum-qsql/tests/recursive_cte_tests.rs:208`

**Lösung implementiert:**
- Parser/Executor wie T01
- Parenthesierte Ausdrücke in SELECT-Liste (z.B. `(n + 1)`) korrekt geparst
- Alias-Handling für Spalten (z.B. `to_node as node`) korrigiert
- UNION-Semantik für Duplikat-Eliminierung implementiert

---

### T04: `test_recursive_cte_union_semantics` ✅ ERLEDIGT

**Status:** ✅ Implementiert und Test aktiviert

**Datei:** `crates/neuroquantum-qsql/tests/recursive_cte_tests.rs:235`

**Lösung implementiert:**
- UNION-Semantik mit Duplikat-Tracking implementiert
- UNION ALL-Semantik mit direktem Append ohne Prüfung

---

### T05: `test_recursive_cte_depth_limit` ✅ ERLEDIGT

**Status:** ✅ Implementiert und Test aktiviert

**Datei:** `crates/neuroquantum-qsql/tests/recursive_cte_tests.rs:305`

**Lösung implementiert:**
- `max_recursion_depth` Limit bei 100 Iterationen
- Fehlerbehandlung bei Limit-Überschreitung

---

### T06: `test_recursive_cte_with_column_list` ✅ ERLEDIGT

**Status:** ✅ Implementiert und Test aktiviert

**Datei:** `crates/neuroquantum-qsql/tests/recursive_cte_tests.rs:385`

**Lösung implementiert:**
- Parser: Spaltenlistenunterstützung in CTE-Definition
- AST: `column_list: Option<Vec<String>>` im CTE-Struct
- Executor: Spalten-Aliasing und korrektes Mapping

---

### T07: `test_recursive_cte_with_multiple_ctes` ✅ ERLEDIGT

**Status:** ✅ Implementiert und Test aktiviert

**Datei:** `crates/neuroquantum-qsql/tests/recursive_cte_tests.rs:510`

**Lösung implementiert:**
- Mehrere CTEs in einer Abfrage werden unterstützt
- CTE-Abhängigkeitsauflösung funktioniert korrekt

---

### T08: `benchmark_1m_inserts`

**Datei:** `crates/neuroquantum-core/src/storage/btree/tests.rs:245`

**Ignore-Grund:** `Long-running benchmark - run with: cargo test --release -- --ignored --nocapture`

**Problem:** Benchmark schlägt fehl - 36.97s statt <30s Zielzeit

**Beschreibung:**  
Performance-Benchmark für 1 Million B+-Tree Inserts. Aktuell ~27.000 Inserts/Sekunde, benötigt ~33.000/Sekunde für das 30s-Ziel.

**Erforderliche Optimierungen:**
1. B+-Tree Bulk-Loading optimieren
2. Page-Splitting effizienter gestalten
3. Write-Batching implementieren
4. Async I/O-Optimierung prüfen
5. Alternativ: Zeitlimit auf 40s erhöhen wenn Hardware-abhängig

**Betroffene Dateien:**
- `crates/neuroquantum-core/src/storage/btree/mod.rs`
- `crates/neuroquantum-core/src/storage/btree/node.rs`

---

### T09: `benchmark_point_lookup`

**Datei:** `crates/neuroquantum-core/src/storage/btree/tests.rs:293`

**Ignore-Grund:** `Long-running benchmark`

**Beschreibung:**  
Benchmark für Punkt-Lookups. Ziel: <1ms p99 Latenz. Fügt 100k Keys ein und führt Lookups durch.

**Status:** Funktioniert, aber ignoriert wegen Laufzeit (~Sekunden)

**Maßnahme:** Behalten als ignorierter Benchmark, bei Bedarf manuell ausführen

---

### T10: `benchmark_range_scan`

**Datei:** `crates/neuroquantum-core/src/storage/btree/tests.rs:343`

**Ignore-Grund:** `Long-running benchmark`

**Beschreibung:**  
Benchmark für Range-Scans. Ziel: 10k Zeilen in <100ms. Testet B+-Tree Leaf-Traversierung.

**Status:** Funktioniert, aber ignoriert wegen Laufzeit

**Maßnahme:** Behalten als ignorierter Benchmark, bei Bedarf manuell ausführen

---

### T11: `test_read_throughput_scaling`

**Datei:** `crates/neuroquantum-core/tests/concurrency_load_tests.rs:254`

**Ignore-Grund:** `#[ignore]` - Time-intensive Load-Test

**Beschreibung:**  
Testet Read-Durchsatz-Skalierung mit verschiedenen Concurrency-Leveln (1, 2, 4, 8, 16 Worker).

**Status:** Designbedingt ignoriert (Load-Test)

**Maßnahme:** Behalten, in CI-Pipeline für nightly/weekly runs integrieren

---

### T12: `test_write_throughput_scaling`

**Datei:** `crates/neuroquantum-core/tests/concurrency_load_tests.rs:329`

**Ignore-Grund:** `#[ignore]` - Time-intensive Load-Test

**Beschreibung:**  
Testet Write-Durchsatz-Skalierung mit verschiedenen Concurrency-Leveln.

**Status:** Designbedingt ignoriert (Load-Test)

**Maßnahme:** Behalten, in CI-Pipeline für nightly/weekly runs integrieren

---

### T13: `test_sustained_load_stability`

**Datei:** `crates/neuroquantum-core/tests/concurrency_load_tests.rs:983`

**Ignore-Grund:** `#[ignore]` - Time-intensive Load-Test

**Beschreibung:**  
Testet System-Stabilität unter anhaltender Last über längeren Zeitraum.

**Status:** Designbedingt ignoriert (Load-Test)

**Maßnahme:** Behalten, in CI-Pipeline für nightly runs integrieren

---

### T14: `test_load_test_summary`

**Datei:** `crates/neuroquantum-core/tests/concurrency_load_tests.rs:1124`

**Ignore-Grund:** `#[ignore]` - Summary-Report

**Beschreibung:**  
Generiert einen Zusammenfassungsbericht aller Load-Tests.

**Status:** Designbedingt ignoriert

**Maßnahme:** Behalten als manueller Berichtstest

---

### T15: `test_concurrent_transactions_crash`

**Datei:** `crates/neuroquantum-core/tests/chaos_engineering_tests.rs:610`

**Ignore-Grund:** `#[ignore] // Long-running test`

**Beschreibung:**  
Chaos-Engineering-Test: Simuliert Crash während mehrerer gleichzeitiger Transaktionen. Verifiziert Recovery-Konsistenz.

**Status:** Designbedingt ignoriert (Chaos-Test)

**Maßnahme:** Behalten, in CI-Pipeline für weekly runs integrieren

---

### T16: `test_acid_properties_after_crash`

**Datei:** `crates/neuroquantum-core/tests/chaos_engineering_tests.rs:806`

**Ignore-Grund:** `#[ignore] // Long-running test`

**Beschreibung:**  
Verifiziert ACID-Eigenschaften nach simuliertem Crash. Prüft committed vs uncommitted Transaktionen.

**Status:** Designbedingt ignoriert (Chaos-Test)

**Maßnahme:** Behalten, in CI-Pipeline für weekly runs integrieren

---

### T17: `test_repeated_crash_recovery_cycles`

**Datei:** `crates/neuroquantum-core/tests/chaos_engineering_tests.rs:910`

**Ignore-Grund:** `#[ignore] // Very long-running test`

**Beschreibung:**  
10 wiederholte Crash-Recovery-Zyklen um Langzeit-Stabilität zu testen.

**Status:** Designbedingt ignoriert (sehr lang-laufend)

**Maßnahme:** Behalten, in CI-Pipeline für monthly runs integrieren

---

### T18: `test_chaos_random_node_kills`

**Datei:** `crates/neuroquantum-core/tests/cluster_e2e_tests.rs:1019`

**Ignore-Grund:** `#[ignore] // Long-running test`

**Beschreibung:**  
Cluster Chaos-Test: Zufälliges Beenden von Nodes während Operationen. Prüft Quorum-Erhaltung.

**Status:** Designbedingt ignoriert (Cluster-Test)

**Maßnahme:** Behalten, in CI-Pipeline für weekly runs integrieren

---

### T19: `test_chaos_concurrent_load_with_failures`

**Datei:** `crates/neuroquantum-core/tests/cluster_e2e_tests.rs:1162`

**Ignore-Grund:** `#[ignore] // Long-running test`

**Beschreibung:**  
Cluster unter Last mit periodischen Failures. Testet Resilienz unter realem Workload.

**Status:** Designbedingt ignoriert (Cluster-Test)

**Maßnahme:** Behalten, in CI-Pipeline für weekly runs integrieren

---

### D01: Doc-Test `permissions.rs` line 8

**Datei:** `crates/neuroquantum-api/src/permissions.rs:8`

**Ignore-Grund:** `rust,ignore` - Doc-Beispiel kompiliert nicht standalone

**Beschreibung:**  
Das Beispiel verwendet `use neuroquantum_api::permissions::*` was im Doc-Test-Kontext nicht funktioniert.

**Lösung:**  
Doc-Test mit `no_run` markieren oder vollständige Imports hinzufügen:
```rust
/// ```rust,no_run
/// use neuroquantum_api::permissions::{Permission, ADMIN, READ, WRITE};
/// ```
```

---

### D02-D22: Doc-Tests (Storage, Quantum, Concurrency)

**Gemeinsamer Ignore-Grund:** `rust,ignore` - Async-Kontext oder externe Abhängigkeiten

**Beschreibung:**  
Diese Doc-Tests verwenden:
- `async` Funktionen ohne Runtime
- `StorageEngine` ohne Dateisystem-Setup
- Quantum-Backends ohne API-Keys
- Concurrency-Primitives ohne vollständigen Kontext

**Lösung für alle:**
1. **Für Storage/Async:** `no_run` statt `ignore` verwenden
2. **Für Quantum-APIs:** `no_run` mit Hinweis auf erforderliche Credentials
3. **Für Concurrency:** Vollständigen, kompilierbaren Beispielcode bereitstellen

**Beispiel-Transformation:**
```rust
// VORHER:
/// ```rust,ignore
/// let storage = StorageEngine::new(path).await?;
/// ```

// NACHHER:
/// ```rust,no_run
/// # async fn example() -> anyhow::Result<()> {
/// use neuroquantum_core::storage::StorageEngine;
/// let storage = StorageEngine::new("./data").await?;
/// # Ok(())
/// # }
/// ```
```

---

## 🔧 Priorisierte Aktionsplan

### Phase 1: Kritische Features (Prio 🔴)
- [ ] T01-T07: Recursive CTE Parser-Implementation

### Phase 2: Performance-Fixes (Prio 🟠)  
- [ ] T08: B+-Tree Insert-Performance optimieren
- [ ] D01-D22: Doc-Tests auf `no_run` umstellen

### Phase 3: Wartung (Prio 🟢)
- [ ] CI-Pipeline für ignorierte Tests konfigurieren
- [ ] T09-T19: In nightly/weekly CI-Jobs integrieren

---

## 📈 Statistiken

- **Gesamt ignorierte Unit-Tests:** 19
- **Gesamt ignorierte Doc-Tests:** 31
- **Fehlgeschlagene Tests bei `--ignored`:** 1 (benchmark_1m_inserts)
- **Feature-blockierend (Parser):** 7
- **Performance-relevant:** 3
- **Designbedingt ignoriert (Load/Chaos):** 9
