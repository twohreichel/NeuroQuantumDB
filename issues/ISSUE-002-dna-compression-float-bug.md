# ISSUE-002: DNA Compression Float-Bug

**Priorität:** 🔴 KRITISCH  
**Aufwand:** 8-16 Stunden  
**Status:** ✅ Erledigt  
**Sprint:** 3 (Kritische Bugs)  
**Abgeschlossen:** 19. Januar 2026

---

## Problembeschreibung

Es existierte ein bekannter Bug in der DNA-Kompression, der dazu führte, dass Zeilen mit Float-Werten beim Zurücklesen von der Festplatte nicht dekomprimiert werden konnten.

## Ursache

Das Problem war, dass die Dictionary-Kompression das Byte `0xFF` als Escape-Byte für Dictionary-Referenzen verwendete, aber literale `0xFF`-Bytes in den Daten nicht escaped wurden. Da Float-Werte (IEEE 754) häufig `0xFF`-Bytes enthalten, führte dies zu fehlerhafter Dekompression.

## Lösung

1. **Encoder-Fix** ([encoder.rs](../crates/neuroquantum-core/src/dna/encoder.rs)):
   - Literale `0xFF`-Bytes werden nun als `0xFF 0xFF` escaped
   - Dictionary-IDs sind auf den Bereich 256-65278 beschränkt (0xFF im High-Byte vermieden)

2. **Decoder-Fix** ([decoder.rs](../crates/neuroquantum-core/src/dna/decoder.rs)):
   - `0xFF 0xFF` wird als literales `0xFF`-Byte interpretiert
   - Reguläre Dictionary-Referenzen: `0xFF [high] [low]` (wenn high ≠ 0xFF)

3. **Serialisierungs-Konsistenz** ([storage.rs](../crates/neuroquantum-core/src/storage.rs)):
   - Alle Stellen verwenden nun konsistent `bincode::serialize` statt gemischte JSON/bincode

## Betroffene Dateien

- `crates/neuroquantum-core/src/dna/encoder.rs` - 0xFF-Escaping hinzugefügt
- `crates/neuroquantum-core/src/dna/decoder.rs` - 0xFF-Escape-Handling hinzugefügt
- `crates/neuroquantum-core/src/storage.rs` - Konsistente bincode-Serialisierung
- `crates/neuroquantum-qsql/tests/multi_row_insert_tests.rs` - Test aktiviert

---

## Validierung

```bash
cargo test -p neuroquantum-qsql multi_row_insert -- --nocapture
```

## Akzeptanzkriterium

- [ ] Float-Serialisierung korrekt (IEEE 754 kompatibel)
- [ ] Test `multi_row_insert_tests` bestanden (ohne `#[ignore]`)
- [ ] Keine Datenkorruption bei Round-Trip
