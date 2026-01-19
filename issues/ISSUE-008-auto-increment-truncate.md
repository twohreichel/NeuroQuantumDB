# ISSUE-008: Auto-Increment Reset bei TRUNCATE

**Priorität:** 🟡 MITTEL  
**Aufwand:** 2-4 Stunden  
**Status:** ✅ Abgeschlossen  
**Sprint:** 2 (Security & API)  
**Abgeschlossen:** 19. Januar 2026

---

## Problembeschreibung

Das Zurücksetzen von Identity/Serial-Spalten bei TRUNCATE TABLE ist nicht implementiert.

## Betroffene Dateien

- `crates/neuroquantum-qsql/src/query_plan.rs` (Zeile ~3756)
- `crates/neuroquantum-core/src/storage.rs`

## Implementierte Lösung

### Änderungen in `storage.rs`:
1. Neue Methode `reset_auto_increment(&mut self, table_name: &str)` hinzugefügt
2. Neue Methode `get_table_schema_mut(&mut self, table_name: &str)` hinzugefügt
3. `save_metadata` öffentlich gemacht für Tests

### Änderungen in `query_plan.rs`:
1. TRUNCATE TABLE ruft jetzt `reset_auto_increment` auf, wenn `restart_identity = true`

### Neue Tests in `truncate_table_tests.rs`:
1. `test_truncate_restart_identity_resets_auto_increment` - Prüft, dass Auto-Increment auf 1 zurückgesetzt wird
2. `test_truncate_continue_identity_preserves_auto_increment` - Prüft, dass Counter bei CONTINUE IDENTITY erhalten bleibt

## Impact

- ✅ TRUNCATE TABLE RESTART IDENTITY setzt Auto-Increment korrekt zurück
- ✅ TRUNCATE TABLE CONTINUE IDENTITY behält Counter bei (Standard-Verhalten)
- ✅ Counter-State wird korrekt persistiert

---

## Validierung

```bash
cargo test -p neuroquantum-qsql truncate -- --nocapture
```

## Akzeptanzkriterium

- [x] TRUNCATE setzt Auto-Increment auf 1 zurück
- [x] Counter-State wird korrekt persistiert
- [x] Alle TRUNCATE-Tests bestehen (21 Tests)
