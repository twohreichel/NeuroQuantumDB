# ISSUE-002: DNA Compression Float-Bug

**Priorität:** 🔴 KRITISCH  
**Aufwand:** 8-16 Stunden  
**Status:** ⬜ Offen  
**Sprint:** 3 (Kritische Bugs)

---

## Problembeschreibung

Es existiert ein bekannter Bug in der DNA-Kompression, der dazu führt, dass Zeilen mit Float-Werten beim Zurücklesen von der Festplatte nicht dekomprimiert werden können.

## Betroffene Dateien

- `crates/neuroquantum-qsql/tests/multi_row_insert_tests.rs` (Zeile ~166)
- `crates/neuroquantum-core/src/dna/` (Kompressionslogik)

## Aktueller Code

```rust
/// NOTE: This test is currently ignored due to a bug in DNA compression
/// that causes certain rows to fail decompression when reading back from disk.
/// The issue appears to be related to how Float values are serialized/compressed.
#[ignore = "DNA compression bug causes row decompression failures - needs investigation"]
```

## Impact

- Float-Werte können nach Speicherung korrupt sein
- Datenintegrität nicht gewährleistet
- Potenzieller Datenverlust in Production

---

## Lösungsschritte

### Schritt 1: Ignorierten Test finden
```bash
grep -n "ignore.*DNA\|DNA.*ignore" crates/neuroquantum-qsql/tests/multi_row_insert_tests.rs
```

### Schritt 2: Float-Handling in DNA-Kompression analysieren
```bash
grep -rn "f32\|f64\|Float" crates/neuroquantum-core/src/dna/
```

### Schritt 3: Implementation
1. Finde den ignorierten Test und verstehe das Problem
2. Suche die DNA-Kompressionslogik für numerische Typen
3. Untersuche Float-Serialisierung (IEEE 754)
4. Füge Debug-Logging hinzu
5. Implementiere korrekten Fix
6. Aktiviere den Test wieder (entferne `#[ignore]`)

---

## Validierung

```bash
cargo test -p neuroquantum-qsql multi_row_insert -- --nocapture
```

## Akzeptanzkriterium

- [ ] Float-Serialisierung korrekt (IEEE 754 kompatibel)
- [ ] Test `multi_row_insert_tests` bestanden (ohne `#[ignore]`)
- [ ] Keine Datenkorruption bei Round-Trip
