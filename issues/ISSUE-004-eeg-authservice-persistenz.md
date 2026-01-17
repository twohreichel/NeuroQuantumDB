# ISSUE-004: EEG-AuthService Persistenz

**Priorität:** 🟠 HOCH  
**Aufwand:** 2-4 Stunden  
**Status:** ⬜ Offen  
**Sprint:** 2 (Security & API)

---

## Problembeschreibung

Der `EEGAuthService` wird bei jedem HTTP-Request neu instanziiert. Dadurch gehen registrierte EEG-Signaturen zwischen Enroll und Verify verloren.

## Betroffene Dateien

- `crates/neuroquantum-api/src/handlers.rs` (Zeilen ~2733-2736, ~2834-2837)

## Aktueller Code

```rust
// Bei Enroll:
let eeg_service = EEGAuthService::new(); // ← NEU erstellt
eeg_service.enroll_user(...);

// Bei Verify:
let eeg_service = EEGAuthService::new(); // ← WIEDER neu erstellt (keine Persistenz!)
eeg_service.verify_user(...); // ← Findet keine Signatur
```

## Impact

- Biometrische EEG-Authentifizierung funktioniert nicht
- Feature ist in Production unbrauchbar
- 95% statt 100% Test-Erfolgsrate

---

## Lösungsschritte

### Schritt 1: EEGAuthService-Verwendung finden
```bash
grep -n "EEGAuthService" crates/neuroquantum-api/src/handlers.rs
```

### Schritt 2: AppState-Struktur finden
```bash
grep -n "AppState\|app_state\|State<" crates/neuroquantum-api/src/
```

### Schritt 3: Implementation (3 Optionen)

**LÖSUNG 1: Shared State mit Arc<RwLock<>>**
```rust
// In AppState:
pub struct AppState {
    // ... existing fields
    pub eeg_service: Arc<RwLock<EEGAuthService>>,
}

// In Handler:
let eeg_service = state.eeg_service.read().await;
// oder
let eeg_service = state.eeg_service.write().await;
```

**LÖSUNG 2: Persistenz in Datenbank**
```rust
struct EEGAuthService {
    storage: Arc<StorageEngine>, // Speichere Signaturen in DB
}
```

**LÖSUNG 3: Application State**
```rust
struct AppState {
    eeg_service: Arc<Mutex<EEGAuthService>>,
}
```

---

## Validierung

```bash
cargo test -p neuroquantum-api eeg -- --nocapture 2>&1 | tail -30
```

## Akzeptanzkriterium

- [ ] EEGAuthService wird nur einmal instanziiert
- [ ] EEG-Enroll gefolgt von EEG-Verify funktioniert
- [ ] Alle EEG-Tests bestehen
