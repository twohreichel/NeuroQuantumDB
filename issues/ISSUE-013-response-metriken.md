# ISSUE-013: Response-Metriken vervollständigen

**Priorität:** 🟢 NIEDRIG  
**Aufwand:** 4-8 Stunden  
**Status:** ⬜ Offen  
**Sprint:** 6 (Nice-to-Have)

---

## Problembeschreibung

Viele API-Endpoints liefern `N/A` oder keine Werte für wichtige Metriken.

## Fehlende Metriken

| Endpoint | Fehlende Metrik |
|----------|-----------------|
| DNA Compression | Compression Ratio |
| Quantum Search | Quantum Speedup |
| Neural Network Training | Training Loss |
| Performance Stats | Memory Usage |

## Impact

- Schlechtere Observability
- Debugging und Performance-Analyse erschwert
- Unvollständige Telemetrie

---

## Lösungsschritte

### Schritt 1: Handler analysieren
```bash
grep -rn "compression_ratio\|speedup\|loss\|memory" crates/neuroquantum-api/src/handlers.rs
```

### Schritt 2: Implementation
1. Response-Strukturen um fehlende Felder erweitern
2. Metriken während Operationen berechnen und zurückgeben
3. Prometheus-Metriken für diese Werte exportieren

**Beispiel für DNA Compression:**
```rust
pub struct DnaCompressionResponse {
    pub compressed_data: Vec<u8>,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,  // ← HINZUFÜGEN
}
```

---

## Validierung

```bash
cargo test -p neuroquantum-api metrics -- --nocapture
```

## Akzeptanzkriterium

- [ ] Compression Ratio in DNA-Response
- [ ] Speedup in Quantum-Search-Response
- [ ] Loss in Neural-Training-Response
- [ ] Memory Usage in Stats-Response
