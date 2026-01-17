# ISSUE-011: Foreign Key Constraints

**Priorität:** 🟢 NIEDRIG  
**Aufwand:** 16-24 Stunden  
**Status:** ⬜ Offen  
**Sprint:** 6 (Nice-to-Have)

---

## Problembeschreibung

Foreign Key Constraints sind nicht implementiert.

## Betroffene Dateien

- `crates/neuroquantum-qsql/src/query_plan.rs` (Zeile ~3719)

## Aktueller Code

```rust
// TODO: When foreign key constraints are implemented, check for referencing tables
```

## Impact

- Keine referenzielle Integrität
- Orphaned Records möglich
- Datenqualität nicht garantierbar

---

## Lösungsschritte

### Schritt 1: TODO finden
```bash
grep -n "foreign.*key\|FOREIGN.*KEY\|referencing" crates/neuroquantum-qsql/src/query_plan.rs
```

### Schritt 2: Implementation
1. `FOREIGN KEY` Syntax im Parser unterstützen
2. Constraint-Speicherung in Metadaten implementieren
3. Constraint-Checking bei INSERT/UPDATE/DELETE
4. CASCADE, SET NULL, RESTRICT Aktionen implementieren
5. Constraint-Validierung beim Erstellen

---

## Validierung

```bash
cargo test -p neuroquantum-qsql foreign_key -- --nocapture
```

## Akzeptanzkriterium

- [ ] FOREIGN KEY Syntax wird geparst
- [ ] Constraints werden in Metadaten gespeichert
- [ ] INSERT mit ungültiger Referenz schlägt fehl
- [ ] CASCADE DELETE funktioniert
