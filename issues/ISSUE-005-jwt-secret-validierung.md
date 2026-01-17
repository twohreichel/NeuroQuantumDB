# ISSUE-005: JWT-Secret Validierung

**Priorität:** 🟠 HOCH  
**Aufwand:** 30 Minuten  
**Status:** ⬜ Offen  
**Sprint:** 1 (Quick Wins)

---

## Problembeschreibung

Default-Secrets werden korrekt als unsicher erkannt und blockiert. Allerdings wird ein leerer String als Secret akzeptiert.

## Betroffene Dateien

- `crates/neuroquantum-api/src/config.rs`

## Aktueller Code

```rust
impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: String::new(),  // ← Leer wird akzeptiert!
```

## Impact

- Potenzielle Sicherheitslücke
- JWT könnte mit leerem Secret signiert werden
- Schwache Authentifizierung in Production möglich

---

## Lösungsschritte

### Schritt 1: INSECURE_DEFAULT_SECRETS finden
```bash
grep -n "INSECURE_DEFAULT_SECRETS" crates/neuroquantum-api/src/config.rs
```

### Schritt 2: Implementation
Füge `""` (leerer String) zur Liste hinzu:

```rust
const INSECURE_DEFAULT_SECRETS: &[&str] = &[
    "",  // ← HINZUFÜGEN
    "your-super-secret-jwt-key-change-this-in-production",
    // ... weitere
];
```

---

## Validierung

```bash
cargo test -p neuroquantum-api jwt -- --nocapture 2>&1 | head -50
```

## Akzeptanzkriterium

- [ ] Leerer JWT-Secret wird als unsicher erkannt
- [ ] Server startet nicht mit leerem JWT-Secret
- [ ] Alle JWT-Tests bestehen
