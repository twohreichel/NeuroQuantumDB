# ISSUE-010: WASM DNA-Kompression

**Priorität:** 🟢 NIEDRIG  
**Aufwand:** 4-8 Stunden  
**Status:** ⬜ Offen  
**Sprint:** 6 (Nice-to-Have)

---

## Problembeschreibung

Die DNA-Kompression im WebAssembly-Modul ist nur ein Placeholder und gibt rohe Bytes zurück.

## Betroffene Dateien

- `crates/neuroquantum-wasm/src/lib.rs` (Zeile ~127-136)

## Aktueller Code

```rust
/// Note: This is a placeholder implementation for demonstration.
/// For production use, integrate with the full NeuroQuantumDB DNA compressor.
pub fn compress_dna(&self, sequence: &str) -> Result<Vec<u8>, JsValue> {
    // TODO: Integrate with neuroquantum_core::dna::QuantumDNACompressor
    Ok(sequence.as_bytes().to_vec())  // Keine echte Kompression!
}
```

## Impact

- Browser-Anwendungen haben keine echte DNA-Kompression
- Feature-Parität zwischen WASM und Native nicht gegeben

---

## Lösungsschritte

### Schritt 1: Placeholder finden
```bash
grep -n "compress_dna\|QuantumDNACompressor" crates/neuroquantum-wasm/src/lib.rs
```

### Schritt 2: Implementation (2 Optionen)

**Option A: Core-Integration**
```rust
use neuroquantum_core::dna::QuantumDNACompressor;

pub fn compress_dna(&self, sequence: &str) -> Result<Vec<u8>, JsValue> {
    let compressor = QuantumDNACompressor::new();
    compressor.compress(sequence)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

**Option B: WASM-spezifische K-mer-Implementierung**
- Vereinfachte Version für Browser
- Keine SIMD-Abhängigkeiten

---

## Validierung

```bash
wasm-pack test --headless --chrome crates/neuroquantum-wasm
```

## Akzeptanzkriterium

- [ ] Echte DNA-Kompression in WASM
- [ ] Keine rohen Bytes mehr zurückgeben
- [ ] Feature-Parität mit Native-Build
