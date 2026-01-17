# ISSUE-008: Auto-Increment Reset bei TRUNCATE

**Priorität:** 🟡 MITTEL  
**Aufwand:** 2-4 Stunden  
**Status:** ⬜ Offen  
**Sprint:** 2 (Security & API)

---

## Problembeschreibung

Das Zurücksetzen von Identity/Serial-Spalten bei TRUNCATE TABLE ist nicht implementiert.

## Betroffene Dateien

- `crates/neuroquantum-qsql/src/query_plan.rs` (Zeile ~3756)

## Aktueller Code

```rust
// TODO: Reset identity/serial columns
```

## Impact

- TRUNCATE TABLE setzt Auto-Increment nicht zurück
- Mögliche ID-Lücken nach Datenbereinigungen

---

## Lösungsschritte

### Schritt 1: TODO finden
```bash
grep -n "identity\|serial\|auto_increment\|TRUNCATE" crates/neuroquantum-qsql/src/query_plan.rs | head -20
```

### Schritt 2: Implementation
1. Counter-State für Auto-Increment-Spalten persistieren
2. Reset-Logik bei TRUNCATE implementieren
3. Optional: `ALTER TABLE ... RESTART IDENTITY` unterstützen

---

## Validierung

```bash
cargo test -p neuroquantum-qsql truncate -- --nocapture
```

## Akzeptanzkriterium

- [ ] TRUNCATE setzt Auto-Increment auf 1 zurück
- [ ] Counter-State wird korrekt persistiert
- [ ] Alle TRUNCATE-Tests bestehen
