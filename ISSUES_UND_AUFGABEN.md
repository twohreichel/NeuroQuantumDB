# NeuroQuantumDB - Agent-Optimierte Aufgabenliste

**Erstellt:** 17. Januar 2026  
**Optimiert für:** Schrittweise automatisierte Abarbeitung  
**Workspace:** `/Users/andreasreichel/workspace/NeuroQuantumDB`

---

## 🎯 Anweisungen für Agent

1. **Bearbeite Tasks in der angegebenen Reihenfolge** (Abhängigkeiten beachtet)
2. **Führe nach jedem Task den Validierungsbefehl aus**
3. **Markiere erledigte Tasks mit `[x]`**
4. **Bei Fehlern:** Notiere den Fehler und fahre mit dem nächsten Task fort

---

## 📋 SPRINT 1: Quick Wins (< 2 Stunden gesamt)

Diese Tasks sind schnell und unabhängig voneinander lösbar.

### [ ] TASK-006: JWT-Secret Validierung (30 Min)

**Ziel:** Leerer JWT-Secret soll als unsicher erkannt werden

**Datei:** `crates/neuroquantum-api/src/config.rs`

**Schritte:**
1. Öffne die Datei und finde `INSECURE_DEFAULT_SECRETS`
2. Füge `""` (leerer String) zur Liste hinzu
3. Finde `impl Default for JwtConfig` und prüfe ob `secret: String::new()` verwendet wird

**Erwartete Änderung:**
```rust
const INSECURE_DEFAULT_SECRETS: &[&str] = &[
    "",  // ← HINZUFÜGEN
    "your-super-secret-jwt-key-change-this-in-production",
    // ... weitere
];
```

**Validierung:**
```bash
cargo test -p neuroquantum-api jwt -- --nocapture 2>&1 | head -50
```

**Akzeptanzkriterium:** Leerer JWT-Secret wird als unsicher erkannt und blockiert.

---

### [ ] TASK-013: Pre-commit Hook für SIMD anpassen (30 Min)

**Ziel:** SIMD-Dateien von unsafe-Prüfung ausnehmen

**Datei:** `hooks/pre-commit`

**Schritte:**
1. Öffne die Datei
2. Finde die Stelle, die `unsafe` prüft
3. Füge Ausnahmen für SIMD-Dateien hinzu:
   - `crates/neuroquantum-core/src/simd/neon.rs`
   - `crates/neuroquantum-core/src/dna/simd/mod.rs`
   - `crates/neuroquantum-core/src/neon_optimization.rs`

**Erwartete Änderung:**
```bash
# Beispiel für Ausnahme-Pattern
SIMD_FILES="simd/neon.rs|dna/simd/|neon_optimization.rs"
# unsafe-Check nur für Dateien die NICHT in SIMD_FILES sind
```

**Validierung:**
```bash
./hooks/pre-commit && echo "Hook OK"
```

---

### [ ] TASK-008: UPDATE ohne WHERE - Fehlermeldung verbessern (1 Std)

**Ziel:** Aussagekräftige Fehlermeldung statt generischem Fehler

**Dateien zu untersuchen:**
- `crates/neuroquantum-qsql/src/query_plan.rs` (UPDATE-Logik)
- `crates/neuroquantum-qsql/src/executor.rs` (Ausführung)

**Schritte:**
1. Suche nach UPDATE-Handling im Query-Planner
2. Finde die Stelle, wo WHERE-Klausel geprüft wird
3. Füge explizite Fehlermeldung hinzu wenn WHERE fehlt

**Erwartete Änderung:**
```rust
if where_clause.is_none() {
    return Err(QsqlError::SafetyError(
        "UPDATE without WHERE clause is disabled for safety. Use WHERE 1=1 to update all rows.".to_string()
    ));
}
```

**Validierung:**
```bash
cargo test -p neuroquantum-qsql update -- --nocapture 2>&1 | grep -i "safety\|where"
```

---

## 📋 SPRINT 2: Security & API Fixes (4-6 Stunden)

### [ ] TASK-004: EEG-AuthService Persistenz (2-4 Std)

**Ziel:** EEG-Signaturen zwischen Requests persistieren

**Datei:** `crates/neuroquantum-api/src/handlers.rs`

**Abhängigkeiten:** Keine

**Schritte:**
1. Suche nach `EEGAuthService::new()` in handlers.rs
2. Finde `AppState` oder ähnliche Struktur für shared state
3. Füge `eeg_service: Arc<RwLock<EEGAuthService>>` zu AppState hinzu
4. Ändere Handler, um shared service zu nutzen

**Code-Suche:**
```bash
grep -n "EEGAuthService" crates/neuroquantum-api/src/handlers.rs
grep -n "AppState\|app_state\|State<" crates/neuroquantum-api/src/
```

**Erwartete Änderung:**
```rust
// In AppState oder ähnlicher Struktur:
pub struct AppState {
    // ... existing fields
    pub eeg_service: Arc<RwLock<EEGAuthService>>,
}

// In Handler:
let eeg_service = state.eeg_service.read().await;
// oder
let eeg_service = state.eeg_service.write().await;
```

**Validierung:**
```bash
cargo test -p neuroquantum-api eeg -- --nocapture 2>&1 | tail -30
```

**Akzeptanzkriterium:** EEG-Enroll gefolgt von EEG-Verify funktioniert.

---

### [ ] TASK-012: Auto-Increment Reset bei TRUNCATE (2-4 Std)

**Ziel:** TRUNCATE TABLE setzt Auto-Increment-Counter zurück

**Datei:** `crates/neuroquantum-qsql/src/query_plan.rs` (Zeile ~3756)

**Schritte:**
1. Suche nach `TODO: Reset identity/serial columns`
2. Finde die TRUNCATE-Implementierung
3. Implementiere Counter-Reset für Auto-Increment-Spalten

**Code-Suche:**
```bash
grep -n "identity\|serial\|auto_increment\|TRUNCATE" crates/neuroquantum-qsql/src/query_plan.rs | head -20
```

**Validierung:**
```bash
cargo test -p neuroquantum-qsql truncate -- --nocapture
```

---

## 📋 SPRINT 3: Kritische Bugs (8-16 Stunden)

### [ ] TASK-002: DNA Compression Float-Bug (8-16 Std)

**Ziel:** Float-Werte korrekt komprimieren/dekomprimieren

**Dateien:**
- `crates/neuroquantum-qsql/tests/multi_row_insert_tests.rs` (ignorierter Test)
- `crates/neuroquantum-core/src/dna/` (Kompressionslogik)

**Abhängigkeiten:** Keine

**Schritte:**
1. Finde den ignorierten Test und verstehe das Problem
2. Suche die DNA-Kompressionslogik für numerische Typen
3. Untersuche Float-Serialisierung (IEEE 754)
4. Füge Debug-Logging hinzu
5. Implementiere korrekten Fix
6. Aktiviere den Test wieder

**Code-Suche:**
```bash
# Finde den ignorierten Test
grep -n "ignore.*DNA\|DNA.*ignore" crates/neuroquantum-qsql/tests/multi_row_insert_tests.rs

# Finde Float-Handling in DNA-Kompression
grep -rn "f32\|f64\|Float" crates/neuroquantum-core/src/dna/
```

**Validierung:**
```bash
# Nach Fix: Test aktivieren und ausführen
cargo test -p neuroquantum-qsql multi_row_insert -- --nocapture
```

**Akzeptanzkriterium:** Test `multi_row_insert_tests` bestanden (ohne `#[ignore]`).

---

### [ ] TASK-001: Migration Executor implementieren (8-16 Std)

**Ziel:** Migrationen tatsächlich ausführen statt simulieren

**Datei:** `crates/neuroquantum-core/src/storage/migration/executor.rs` (Zeile ~215)

**Abhängigkeiten:** Keine

**Schritte:**
1. Finde die `tokio::time::sleep` Simulation
2. Analysiere wie `neuroquantum_qsql` SQL ausführt
3. Integriere den Query-Executor
4. Implementiere transaktionale Ausführung
5. Entferne die Sleep-Simulation

**Code-Suche:**
```bash
# Finde die Simulation
grep -n "sleep\|simulate\|TODO.*execute" crates/neuroquantum-core/src/storage/migration/executor.rs

# Finde Query-Executor Pattern
grep -rn "execute_query\|run_query\|QueryExecutor" crates/neuroquantum-qsql/src/
```

**Erwartete Änderung:**
```rust
// ENTFERNEN:
// tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

// HINZUFÜGEN:
let result = query_executor.execute_sql(&migration.sql).await?;
```

**Validierung:**
```bash
cargo test -p neuroquantum-core migration -- --nocapture
```

---

## 📋 SPRINT 4: SQL-Funktionalität (8-12 Stunden)

### [ ] TASK-005: Datum/Zeit-Funktionen implementieren (8-12 Std)

**Ziel:** Temporale SQL-Funktionen hinzufügen

**Dateien:**
- `crates/neuroquantum-qsql/src/parser.rs` (oder lexer)
- `crates/neuroquantum-qsql/src/functions.rs` (oder ähnlich)
- `Cargo.toml` (chrono dependency)

**Abhängigkeiten:** Keine

**Zu implementierende Funktionen:**
1. `CURRENT_DATE` → `chrono::Utc::now().date_naive()`
2. `CURRENT_TIME` → `chrono::Utc::now().time()`
3. `CURRENT_TIMESTAMP` / `NOW()` → `chrono::Utc::now()`
4. `DATE_ADD(date, interval)` → Datum + Interval
5. `DATE_SUB(date, interval)` → Datum - Interval
6. `EXTRACT(part FROM date)` → Jahr/Monat/Tag extrahieren

**Schritte:**
1. Füge `chrono` zu Cargo.toml hinzu (falls nicht vorhanden)
2. Erweitere Parser für neue Funktionen
3. Implementiere Execution-Logic
4. Schreibe Tests

**Code-Suche:**
```bash
# Prüfe ob chrono vorhanden
grep "chrono" crates/neuroquantum-qsql/Cargo.toml

# Finde wo Funktionen definiert sind
grep -rn "fn.*UPPER\|fn.*LOWER\|BuiltinFunction" crates/neuroquantum-qsql/src/
```

**Validierung:**
```bash
cargo test -p neuroquantum-qsql date_time -- --nocapture
```

---

### [ ] TASK-010: NEUROMATCH in JOINs fixen (4-8 Std)

**Ziel:** QSQL-Funktionen in JOIN-Kontext ermöglichen

**Dateien:**
- `crates/neuroquantum-qsql/src/query_plan.rs`
- `crates/neuroquantum-qsql/src/optimizer.rs` (falls vorhanden)

**Abhängigkeiten:** Keine

**Schritte:**
1. Finde wie JOINs geplant/ausgeführt werden
2. Finde wie NEUROMATCH in WHERE verarbeitet wird
3. Identifiziere warum es im JOIN-Kontext fehlschlägt
4. Erweitere den Optimizer/Executor

**Code-Suche:**
```bash
grep -rn "NEUROMATCH\|neuromatch" crates/neuroquantum-qsql/src/
grep -rn "JOIN\|join.*plan" crates/neuroquantum-qsql/src/query_plan.rs | head -30
```

**Test-Query:**
```sql
SELECT u.name, o.amount 
FROM users u 
INNER JOIN orders o ON u.id = o.user_id 
WHERE NEUROMATCH(u.name, 'Test') > 0.3
```

**Validierung:**
```bash
cargo test -p neuroquantum-qsql neuromatch.*join -- --nocapture
```

---

## 📋 SPRINT 5: Cluster-Stabilität (16-32 Stunden)

### [ ] TASK-003: Cluster Rollback implementieren (16-32 Std)

**Ziel:** Automatisches Rollback bei fehlgeschlagenen Upgrades

**Datei:** `crates/neuroquantum-cluster/src/upgrade.rs` (Zeile ~296-321)

**Abhängigkeiten:** Keine

**Schritte:**
1. Finde die Rollback-Placeholder-Funktion
2. Analysiere wie State vor Upgrade gespeichert werden kann
3. Implementiere Snapshot-Mechanismus
4. Implementiere Restore-Mechanismus
5. Integriere Health-Checks

**Code-Suche:**
```bash
grep -n "rollback\|Rollback\|placeholder" crates/neuroquantum-cluster/src/upgrade.rs
```

**Validierung:**
```bash
cargo test -p neuroquantum-cluster rollback -- --nocapture
```

---

### [ ] TASK-007: Anti-Entropy Repair implementieren (16-24 Std)

**Ziel:** Automatische Shard-Reparatur zwischen Replikas

**Datei:** `crates/neuroquantum-cluster/src/replication.rs` (Zeile ~366-387)

**Abhängigkeiten:** Keine

**Schritte:**
1. Finde die `repair_shard` Funktion
2. Implementiere Merkle-Tree-Vergleich
3. Implementiere Divergenz-Erkennung
4. Implementiere Data-Streaming
5. Implementiere Conflict Resolution

**Code-Suche:**
```bash
grep -n "repair_shard\|RepairResult\|merkle" crates/neuroquantum-cluster/src/replication.rs
```

**Validierung:**
```bash
cargo test -p neuroquantum-cluster repair -- --nocapture
```

---

## 📋 SPRINT 6: Nice-to-Have (Optional)

### [ ] TASK-009: WASM DNA-Kompression (4-8 Std)

**Datei:** `crates/neuroquantum-wasm/src/lib.rs` (Zeile ~127-136)

**Schritte:**
1. Finde `compress_dna` Funktion
2. Importiere `QuantumDNACompressor` aus neuroquantum_core
3. Integriere echte Kompression

---

### [ ] TASK-011: Foreign Key Constraints (16-24 Std)

**Datei:** `crates/neuroquantum-qsql/src/query_plan.rs` (Zeile ~3719)

**Schritte:**
1. Parser für FOREIGN KEY Syntax erweitern
2. Constraint-Speicherung implementieren
3. Validation bei INSERT/UPDATE/DELETE
4. CASCADE/SET NULL/RESTRICT Aktionen

---

### [ ] TASK-014: Query Plan Cache Eviction (4-6 Std)

**Datei:** `crates/neuroquantum-qsql/src/lib.rs` (Zeile ~70-77)

**Schritte:**
1. Memory-Limit-Konfiguration hinzufügen
2. LRU-Eviction basierend auf `last_accessed` implementieren

---

### [ ] TASK-015: Response-Metriken vervollständigen (4-8 Std)

**Dateien:** Handler-Dateien in `crates/neuroquantum-api/src/`

**Fehlende Metriken:**
- Compression Ratio (DNA)
- Quantum Speedup
- Training Loss
- Memory Usage

---

### [ ] TASK-016: Correlated Subqueries (8-16 Std)

**Datei:** `crates/neuroquantum-qsql/src/`

**Test-Query:**
```sql
SELECT * FROM users u WHERE age > (SELECT AVG(age) FROM users)
```

---

### [ ] TASK-017: Rekursive CTEs (24-40 Std)

**Datei:** `crates/neuroquantum-qsql/src/`

**Test-Query:**
```sql
WITH RECURSIVE cte AS (SELECT 1 AS n UNION ALL SELECT n+1 FROM cte WHERE n < 5) 
SELECT * FROM cte
```

---

### [ ] TASK-018: Dead Code bereinigen (2-4 Std)

**Dateien:**
- `crates/neuroquantum-cluster/src/replication.rs` (Zeile 346)
- `crates/neuroquantum-cluster/src/network.rs` (Zeile 204-216)

---

### [ ] TASK-019: Cluster TLS-Prüfung (4-8 Std)

**Datei:** `crates/neuroquantum-cluster/src/network.rs`

---

## 📊 Fortschritts-Tracker

| Sprint | Tasks | Erledigt | Status |
|--------|-------|----------|--------|
| Sprint 1: Quick Wins | 3 | 0 | ⬜ Ausstehend |
| Sprint 2: Security & API | 2 | 0 | ⬜ Ausstehend |
| Sprint 3: Kritische Bugs | 2 | 0 | ⬜ Ausstehend |
| Sprint 4: SQL-Funktionalität | 2 | 0 | ⬜ Ausstehend |
| Sprint 5: Cluster | 2 | 0 | ⬜ Ausstehend |
| Sprint 6: Optional | 8 | 0 | ⬜ Backlog |

**Gesamt:** 0/19 Tasks erledigt

---

## 🔧 Nützliche Befehle

```bash
# Alle Tests ausführen
cargo test --all

# Spezifisches Crate testen
cargo test -p neuroquantum-core
cargo test -p neuroquantum-qsql
cargo test -p neuroquantum-api
cargo test -p neuroquantum-cluster

# Build prüfen
cargo build --release

# Clippy Lints
cargo clippy --all-targets

# Formatierung prüfen
cargo fmt --check
```

---

*Optimiert für Agent-Abarbeitung am 17. Januar 2026*
