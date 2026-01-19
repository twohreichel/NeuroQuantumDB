# ISSUE-009: NEUROMATCH in JOINs fixen

**Priorität:** 🟡 MITTEL  
**Aufwand:** 4-8 Stunden  
**Status:** ✅ Erledigt (19. Januar 2026)  
**Sprint:** 4 (SQL-Funktionalität)

---

## Problembeschreibung

NEUROMATCH funktioniert in einfachen WHERE-Klauseln, aber nicht im JOIN-Kontext.

## Betroffene Dateien

- `crates/neuroquantum-qsql/src/query_plan.rs`
- `crates/neuroquantum-qsql/src/optimizer.rs` (falls vorhanden)

## Fehlerhafter Query

```sql
SELECT u.name, o.amount 
FROM users u 
INNER JOIN orders o ON u.id = o.user_id 
WHERE NEUROMATCH(u.name, 'Test') > 0.3
-- Fehler: Query execution failed
```

## Funktionierender Workaround

```sql
SELECT u.name, o.amount 
FROM (SELECT * FROM users WHERE NEUROMATCH(name, 'Test') > 0.3) u
INNER JOIN orders o ON u.id = o.user_id
```

## Impact

- QSQL-Funktionen nur eingeschränkt nutzbar
- Komplexe Queries mit neuromorphen Funktionen nicht möglich

---

## Lösungsschritte

### Schritt 1: NEUROMATCH-Handling finden
```bash
grep -rn "NEUROMATCH\|neuromatch" crates/neuroquantum-qsql/src/
```

### Schritt 2: JOIN-Planung analysieren
```bash
grep -rn "JOIN\|join.*plan" crates/neuroquantum-qsql/src/query_plan.rs | head -30
```

### Schritt 3: Implementation
1. Finde wie JOINs geplant/ausgeführt werden
2. Finde wie NEUROMATCH in WHERE verarbeitet wird
3. Identifiziere warum es im JOIN-Kontext fehlschlägt
4. Erweitere den Optimizer/Executor für QSQL-Funktionen in JOIN-Kontext

---

## Validierung

```bash
cargo test -p neuroquantum-qsql neuromatch.*join -- --nocapture
```

## Akzeptanzkriterium

- [x] NEUROMATCH funktioniert in JOIN WHERE-Klauseln
- [x] Kein Workaround mit Subquery nötig
- [x] Alle QSQL-JOIN-Tests bestehen

## Lösung (implementiert am 19.01.2026)

### Problem
Die `evaluate_where_expression` Funktion in `query_plan.rs` konnte nur einfache `Identifier`-Expressions auf der linken Seite eines Vergleichs verarbeiten. Funktionsaufrufe wie `NEUROMATCH(u.name, 'Test')` wurden nicht unterstützt und führten zu einem stillen Fehler.

### Änderungen
1. **NEUROMATCH als Scalar Function hinzugefügt** (`evaluate_scalar_function`):
   - Implementierung der NEUROMATCH-Funktion mit 2 Argumenten
   - Nutzt die bestehende `calculate_neuromatch_similarity` Methode

2. **evaluate_where_expression erweitert**:
   - Neue Behandlung für `Expression::FunctionCall` in BinaryOp-Vergleichen
   - Ermöglicht Ausdrücke wie `NEUROMATCH(col, 'pattern') > 0.3`

3. **Statische Hilfsfunktion hinzugefügt** (`evaluate_function_call_static`):
   - Evaluiert Funktionsaufrufe im WHERE-Kontext
   - Unterstützt NEUROMATCH, SYNAPTIC_WEIGHT, UPPER, LOWER, LENGTH

4. **Tests hinzugefügt**:
   - `test_parser_neuromatch_function_in_join_where`
   - `test_parser_neuromatch_with_left_join`
   - `test_parser_neuromatch_qualified_column`
