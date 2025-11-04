# 🚀 NeuroQuantumDB Setup-Befehle mit `cargo run`

## Voraussetzung
Das System läuft mit `cargo run`. Um CLI-Befehle auszuführen, verwenden Sie:
```bash
cargo run -p neuroquantum-api -- [BEFEHL]
```

---

## 📝 Verfügbare Befehle

### 1. Datenbank initialisieren (Interaktiv)
```bash
cargo run -p neuroquantum-api -- init
```

### 2. Datenbank initialisieren (Nicht-interaktiv mit benutzerdefinierten Einstellungen)
```bash
cargo run -p neuroquantum-api -- init --name admin --expiry-hours 8760 --output .env --yes
```

**Parameter:**
- `--name admin`: Name des Admin-Keys
- `--expiry-hours 8760`: Gültigkeit in Stunden (8760 = 1 Jahr)
- `--output .env`: Ausgabedatei für den Admin-Key
- `--yes` oder `-y`: Überspringt Bestätigungsfragen

### 3. JWT-Secret generieren
```bash
cargo run -p neuroquantum-api -- generate-jwt-secret --output config/jwt-secret.txt
```

**Oder Ausgabe direkt im Terminal:**
```bash
cargo run -p neuroquantum-api -- generate-jwt-secret
```

### 4. Server starten (Standard)
```bash
cargo run -p neuroquantum-api
# oder explizit:
cargo run -p neuroquantum-api -- serve
```

### 5. Mit benutzerdefinierter Konfiguration
```bash
cargo run -p neuroquantum-api -- --config config/prod.toml serve
```

---

## 🔧 Vollständiges Setup-Beispiel

```bash
# 1. JWT-Secret generieren
cargo run -p neuroquantum-api -- generate-jwt-secret --output config/jwt-secret.txt

# 2. Datenbank initialisieren
cargo run -p neuroquantum-api -- init --name admin --expiry-hours 8760 --output .env --yes

# 3. Server starten
cargo run -p neuroquantum-api
```

---

## 💡 Hilfreiche Tipps

### Hilfe zu einem bestimmten Befehl anzeigen:
```bash
cargo run -p neuroquantum-api -- init --help
cargo run -p neuroquantum-api -- generate-jwt-secret --help
```

### Alle verfügbaren Befehle anzeigen:
```bash
cargo run -p neuroquantum-api -- --help
```

### Mit Release-Build (schneller):
```bash
cargo run -p neuroquantum-api --release -- init
```

---

## 📋 Schnellreferenz-Tabelle

| Aktion | Befehl |
|--------|--------|
| Init (interaktiv) | `cargo run -p neuroquantum-api -- init` |
| Init (auto) | `cargo run -p neuroquantum-api -- init --name admin --expiry-hours 8760 --output .env --yes` |
| JWT-Secret | `cargo run -p neuroquantum-api -- generate-jwt-secret --output config/jwt-secret.txt` |
| Server starten | `cargo run -p neuroquantum-api` |
| Hilfe | `cargo run -p neuroquantum-api -- --help` |

---

## 🎯 Nach dem Setup

Nach erfolgreicher Initialisierung:
1. Der Admin-Key wird in der angegebenen Datei gespeichert (z.B. `.env`)
2. Das JWT-Secret steht in `config/jwt-secret.txt`
3. Sie können den Server starten und die API verwenden

**Wichtig:** Bewahren Sie den Admin-Key und das JWT-Secret sicher auf!

