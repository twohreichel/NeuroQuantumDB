# ISSUE-009: NEUROMATCH in JOINs fixen

**Priorität:** 🟡 MITTEL  
**Aufwand:** 4-8 Stunden  
**Status:** ⬜ Offen  
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

- [ ] NEUROMATCH funktioniert in JOIN WHERE-Klauseln
- [ ] Kein Workaround mit Subquery nötig
- [ ] Alle QSQL-JOIN-Tests bestehen
