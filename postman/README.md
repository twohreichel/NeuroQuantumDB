# NeuroQuantumDB Postman Collection v2 🔒

## ⚠️ WICHTIGE SICHERHEITS-UPDATES

**JWT-Login wurde deaktiviert!** Die API verwendet jetzt ausschließlich **API-Key-Authentifizierung** für erhöhte Sicherheit.

Diese Collection enthält alle API-Endpunkte für NeuroQuantumDB mit Beispielen für:

- 🔐 **API-Key-Authentifizierung** (Persistent & Sicher)
- 📊 CRUD-Operationen
- 🧠 Neuromorphic Learning
- ⚛️ Quantum Search (Grover's Algorithm)
- 🧬 DNA Compression
- 🔬 Biometric Authentication (EEG)
- 🌐 WebSocket Real-time Updates

## 🚀 Quick Start

### 1. Erstelle deinen ersten Admin-API-Key

```bash
# Im NeuroQuantumDB-Verzeichnis ausführen
cd /Users/andreasreichel/workspace/NeuroQuantumDB

# Admin-Key erstellen
cargo run --package neuroquantum-api --bin neuroquantum-api -- init --name "admin" --output admin-key.env

# Key wird angezeigt und in admin-key.env gespeichert
# WICHTIG: Speichere diesen Key sicher!
```

**Ausgabe:**
```
🔐 API Key: nqdb_1234567890abcdef...
```

### 2. Collection importieren

1. Öffne Postman
2. Click auf "Import"
3. Wähle **`NeuroQuantumDB.postman_collection.v2.json`** (die neue Version!)
4. Wähle **`NeuroQuantumDB.postman_environment.v2.json`**

### 3. Environment konfigurieren

1. Wähle das "NeuroQuantumDB Environment (API-Key Auth)" aus
2. Setze die Variable `api_key` auf deinen generierten API-Key:
   ```
   api_key: nqdb_1234567890abcdef...
   ```
3. Optional: Setze `base_url` wenn nicht localhost:
   ```
   base_url: http://localhost:8080
   ```

### 4. Teste die Verbindung

1. Gehe zu **Health & Status > Health Check**
2. Führe den Request aus (keine Auth erforderlich)
3. Erwartete Antwort: `200 OK` mit Status "healthy"

### 5. Teste die Authentifizierung

1. Gehe zu **CRUD Operations > Query Records**
2. Führe den Request aus (verwendet automatisch deinen API-Key)
3. Erwartete Antwort: `200 OK` mit Daten

## 🔑 API-Key-Management

### Weitere API-Keys erstellen

#### Via CLI:
```bash
# Neuen Key mit read/write Permissions erstellen
export NEUROQUANTUM_ADMIN_KEY="nqdb_your_admin_key_here"

cargo run --package neuroquantum-api --bin neuroquantum-api -- \
  key create \
  --name "developer" \
  --permissions read,write,quantum \
  --expiry-hours 720 \
  --output developer-key.env
```

#### Via Postman:
1. Gehe zu **API Key Management > Generate New API Key**
2. Passe die Permissions im Request Body an:
   ```json
   {
     "name": "developer-key",
     "permissions": ["read", "write", "quantum"],
     "expiry_hours": 720,
     "rate_limit_per_hour": 1000
   }
   ```
3. Führe den Request aus
4. Speichere den generierten Key sicher!

### API-Keys auflisten

```bash
# Via CLI
export NEUROQUANTUM_ADMIN_KEY="nqdb_your_admin_key_here"
cargo run --package neuroquantum-api --bin neuroquantum-api -- key list
```

### API-Keys widerrufen

```bash
# Via CLI
export NEUROQUANTUM_ADMIN_KEY="nqdb_your_admin_key_here"
cargo run --package neuroquantum-api --bin neuroquantum-api -- \
  key revoke \
  --key "nqdb_key_to_revoke"
```

#### Via Postman:
1. Gehe zu **API Key Management > Revoke API Key**
2. Setze den zu widerrufenden Key im Request Body
3. Führe den Request aus

## 📋 Permissions-System

| Permission | Beschreibung | Endpoints |
|-----------|-------------|-----------|
| `admin` | Voller Zugriff, kann Keys verwalten | Alle + Key-Management |
| `read` | Daten lesen | GET /query, /tables |
| `write` | Daten schreiben | POST /tables, /records |
| `quantum` | Quantum-Operationen | /quantum/* |
| `neuromorphic` | Neuromorphic-Operationen | /neuromorphic/* |
| `dna` | DNA-Compression | /dna/* |

## 🔒 Sicherheits-Features

### Persistent Storage
- ✅ API-Keys werden in SQLite gespeichert
- ✅ Keys überleben Server-Neustarts
- ✅ Bcrypt-Hashing für Key-Validierung

### Rate Limiting
- Konfigurierbar pro API-Key
- Default: 10.000 Requests/Stunde für Admin-Keys
- Anpassbar bei Key-Erstellung

### Expiration
- Keys haben Ablaufdatum
- Default: 30 Tage
- Anpassbar bei Key-Erstellung (--expiry-hours)

### Audit Trail
- Alle Key-Operationen werden geloggt
- Last-used Timestamp wird getrackt
- Usage-Counter pro Key

## ❌ Deaktivierte Endpoints

Die folgenden Endpoints wurden aus Sicherheitsgründen deaktiviert:

- ~~`POST /api/v1/auth/login`~~ → Verwendet API-Keys stattdessen
- ~~`POST /api/v1/auth/refresh`~~ → Keys haben feste Ablaufdaten

## 🆘 Troubleshooting

### "Unauthorized" Error

**Problem:** `401 Unauthorized: Authentication required`

**Lösung:**
1. Überprüfe ob `X-API-Key` Header gesetzt ist
2. Verifiziere dass der API-Key korrekt ist (beginnt mit `nqdb_`)
3. Überprüfe ob der Key noch gültig ist (nicht abgelaufen)
4. Prüfe die Permissions des Keys

### "Invalid API key" Error

**Problem:** API-Key wird nicht akzeptiert

**Lösung:**
1. Key könnte abgelaufen sein → Neuen Key erstellen
2. Key könnte widerrufen sein → Neuen Key erstellen
3. Datenbank könnte resettet sein → `neuroquantum-api init` erneut ausführen

### Server startet nicht

**Problem:** `No admin keys found!`

**Lösung:**
```bash
# Erstelle initialen Admin-Key
cargo run --package neuroquantum-api --bin neuroquantum-api -- init
```

## 📚 Weitere Ressourcen

- [API-Dokumentation](http://localhost:8080/api-docs)
- [GitHub Repository](https://github.com/neuroquantumdb/neuroquantumdb)
- [Sicherheits-Best-Practices](../docs/security-best-practices.md)

## 🔄 Migration von v1

Wenn du die alte Collection mit JWT-Login verwendest hast:

1. **Erstelle einen Admin-API-Key:**
   ```bash
   cargo run --package neuroquantum-api --bin neuroquantum-api -- init
   ```

2. **Aktualisiere deine Environment:**
   - Entferne `access_token` und `refresh_token`
   - Füge `api_key` hinzu

3. **Importiere die neue Collection v2**

4. **Teste alle Endpoints erneut**

Die alte JWT-Login-Methode funktioniert nicht mehr und gibt `501 Not Implemented` zurück.

## 📞 Support

Bei Fragen oder Problemen:
- Erstelle ein Issue auf GitHub
- Prüfe die Dokumentation unter `/docs`
- Nutze `--help` bei CLI-Befehlen

