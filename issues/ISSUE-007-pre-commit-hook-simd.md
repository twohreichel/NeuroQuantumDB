# ISSUE-007: Pre-commit Hook für SIMD anpassen

**Priorität:** 🟡 MITTEL  
**Aufwand:** 30 Minuten  
**Status:** ⬜ Offen  
**Sprint:** 1 (Quick Wins)

---

## Problembeschreibung

Der pre-commit Hook verbietet alle `unsafe`-Blöcke, was für SIMD-Optimierungen zu restriktiv ist. Alle SIMD-Funktionen sind dokumentiert und notwendig.

## Betroffene Dateien

- `hooks/pre-commit`

**SIMD-Dateien die Ausnahmen brauchen:**
- `crates/neuroquantum-core/src/simd/neon.rs`
- `crates/neuroquantum-core/src/dna/simd/mod.rs`
- `crates/neuroquantum-core/src/neon_optimization.rs`

## Impact

- Entwickler können SIMD-Code nicht committen
- Workaround erforderlich (Hook deaktivieren)

---

## Lösungsschritte

### Schritt 1: Hook analysieren
```bash
cat hooks/pre-commit | grep -A5 -B5 "unsafe"
```

### Schritt 2: Implementation
Füge Ausnahmen für SIMD-Dateien hinzu:

```bash
# Beispiel für Ausnahme-Pattern
SIMD_FILES="simd/neon.rs|dna/simd/|neon_optimization.rs"

# unsafe-Check nur für Dateien die NICHT in SIMD_FILES sind
if echo "$file" | grep -qvE "$SIMD_FILES"; then
    # unsafe-Check durchführen
fi
```

---

## Validierung

```bash
./hooks/pre-commit && echo "Hook OK"
```

## Akzeptanzkriterium

- [ ] SIMD-Dateien von unsafe-Prüfung ausgenommen
- [ ] Hook läuft ohne Fehler durch
- [ ] Nicht-SIMD-Code wird weiterhin geprüft
