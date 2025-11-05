# NeuroQuantumDB Postman Collection

Diese Postman Collection ermöglicht es Ihnen, alle API-Endpunkte der NeuroQuantumDB lokal zu testen.

## 📦 Inhalt

- `NeuroQuantumDB.postman_collection.json` - Vollständige API-Collection mit allen Endpunkten
- `NeuroQuantumDB.postman_environment.json` - Environment-Konfiguration für lokales Testing
- `README.md` - Diese Anleitung

## 🚀 Schnellstart

### 1. Postman Collection importieren

1. Öffnen Sie Postman
2. Klicken Sie auf **Import** (oben links)
3. Wählen Sie **File** und importieren Sie:
   - `NeuroQuantumDB.postman_collection.json`
   - `NeuroQuantumDB.postman_environment.json`
4. Die Collection erscheint unter "Collections" und das Environment unter "Environments"

### 2. Environment aktivieren

1. Klicken Sie oben rechts auf den Environment-Dropdown
2. Wählen Sie **"NeuroQuantumDB Local"**
3. Das Environment ist nun aktiv und zeigt `http://localhost:8080` als Base URL

### 3. API Server starten

Stellen Sie sicher, dass der NeuroQuantumDB API Server läuft:

```bash
cd /Users/andreasreichel/workspace/NeuroQuantumDB
cargo run --bin neuroquantum-api
```

Der Server startet standardmäßig auf `http://localhost:8080`.

### 4. Testen der API

#### Health Check (ohne Authentifizierung)
1. Öffnen Sie die Collection **NeuroQuantumDB API**
2. Navigieren Sie zu **Health & Status** → **Health Check**
3. Klicken Sie auf **Send**
4. Sie sollten eine erfolgreiche Response mit Status "healthy" erhalten

#### Login & Token-Authentifizierung
1. Navigieren Sie zu **Authentication** → **Login**
2. Klicken Sie auf **Send**
3. **Der Access Token wird automatisch extrahiert und gespeichert!**
4. Alle nachfolgenden Requests verwenden diesen Token automatisch

## 🔑 Automatische Token-Verwaltung

Die Collection enthält Post-Response-Scripts, die automatisch:

- ✅ **Access Token** aus der Login-Response extrahieren
- ✅ **Refresh Token** speichern
- ✅ **User ID** speichern
- ✅ **API Keys** nach der Generierung speichern
- ✅ **Network IDs** und andere IDs für nachfolgende Requests bereitstellen

Sie müssen nichts manuell kopieren oder einfügen!

## 📋 API-Endpunkte Übersicht

### Health & Status
- **Health Check** - Prüft den Server-Status (keine Auth erforderlich)

### Authentication
- **Login** - Authentifizierung mit Username/Password → generiert Access Token
- **Refresh Token** - Erneuert den Access Token
- **Generate API Key** - Erstellt einen neuen API Key (Admin-Berechtigung erforderlich)
- **Revoke API Key** - Widerruft einen API Key (Admin-Berechtigung erforderlich)

### CRUD Operations
- **Execute SQL Query** - Führt beliebige SQL-Abfragen aus
- **Create Table** - Erstellt eine neue Tabelle mit Schema
- **Insert Data** - Fügt Daten in Batch ein
- **Query Data** - Fragt Daten mit Filtern ab
- **Update Data** - Aktualisiert Datensätze
- **Delete Data** - Löscht Datensätze (mit Soft-Delete und Cascade-Option)

### Neural Networks
- **Train Neural Network** - Startet das Training eines neuronalen Netzwerks
- **Get Training Status** - Ruft den Training-Status ab

### Quantum Search
- **Quantum Search** - Führt Quantum-inspirierte Suche mit Grover's Algorithmus durch

### DNA Compression
- **Compress DNA** - Komprimiert DNA-Sequenzen mit fortschrittlichen Algorithmen

### Biometric Authentication
- **EEG Enroll User** - Registriert Benutzer mit EEG-biometrischer Signatur
- **EEG Authenticate** - Authentifiziert mit EEG-Daten
- **EEG Update Signature** - Aktualisiert EEG-Signatur
- **EEG List Users** - Listet alle registrierten EEG-Benutzer

### Monitoring
- **Get Metrics** - Prometheus-kompatible Metriken
- **Get Performance Stats** - Detaillierte Performance-Statistiken

## 🔐 Authentifizierung

Die Collection unterstützt zwei Authentifizierungsmethoden:

### 1. JWT Bearer Token (empfohlen für Testing)
- Wird automatisch nach dem Login verwendet
- Wird in allen geschützten Endpunkten automatisch mitgesendet
- Token läuft nach 24 Stunden ab (kann mit Refresh Token erneuert werden)

### 2. API Key Authentication
- Kann über **Generate API Key** erstellt werden
- Benötigt Admin-Berechtigung
- Für langfristige Zugriffe geeignet

## 📝 Beispiel-Workflow

### Kompletter Test-Durchlauf:

1. **Health Check** - Prüfe Server-Status
2. **Login** - Authentifiziere dich (Token wird automatisch gespeichert)
3. **Generate API Key** - Erstelle einen Admin API Key (optional)
4. **Create Table** - Erstelle eine "users" Tabelle
5. **Insert Data** - Füge Test-Daten ein
6. **Query Data** - Frage die Daten ab
7. **Update Data** - Aktualisiere einen Datensatz
8. **Train Neural Network** - Starte ein neuronales Netzwerk Training
9. **Get Training Status** - Prüfe den Training-Fortschritt
10. **Quantum Search** - Führe eine Quantum-Suche durch
11. **Compress DNA** - Komprimiere DNA-Sequenzen
12. **EEG Enroll User** - Registriere einen Benutzer mit EEG
13. **EEG Authenticate** - Authentifiziere mit EEG-Daten
14. **Get Performance Stats** - Hole Performance-Metriken

## 🧪 Tests

Jeder Request enthält automatische Tests:

```javascript
pm.test("Status code is 200", function () {
    pm.response.to.have.status(200);
});

pm.test("Response has success status", function () {
    var jsonData = pm.response.json();
    pm.expect(jsonData.success).to.be.true;
});
```

Die Tests werden automatisch ausgeführt und zeigen grüne Häkchen bei Erfolg.

## 🔧 Environment-Variablen

Das Environment enthält folgende Variablen:

| Variable | Beschreibung | Beispielwert |
|----------|--------------|--------------|
| `base_url` | API Base URL | `http://localhost:8080` |
| `access_token` | JWT Access Token | Wird automatisch gesetzt |
| `refresh_token` | JWT Refresh Token | Wird automatisch gesetzt |
| `api_key` | Generierter API Key | Wird automatisch gesetzt |
| `user_id` | Benutzer ID | Wird automatisch gesetzt |
| `table_name` | Standard-Tabellenname | `users` |
| `network_id` | Neural Network ID | Wird automatisch gesetzt |
| `eeg_user_id` | EEG Benutzer ID | `john_doe_123` |

Sie können diese Variablen manuell anpassen, wenn gewünscht.

## 🌐 Andere Environments

Für andere Umgebungen (z.B. Production, Staging):

1. Duplizieren Sie das Environment
2. Ändern Sie die `base_url` entsprechend:
   - Production: `https://api.neuroquantum.com`
   - Staging: `https://staging-api.neuroquantum.com`

## 🐛 Troubleshooting

### Problem: "Could not send request" / Connection refused
**Lösung:** Stellen Sie sicher, dass der API Server läuft:
```bash
cargo run --bin neuroquantum-api
```

### Problem: 401 Unauthorized
**Lösung:** 
1. Führen Sie zuerst den **Login**-Request aus
2. Der Token wird automatisch gespeichert
3. Oder verwenden Sie **Refresh Token**, wenn der Token abgelaufen ist

### Problem: 403 Forbidden
**Lösung:** Der Endpunkt erfordert spezielle Berechtigungen (z.B. Admin)
1. Loggen Sie sich mit einem Admin-Account ein
2. Oder generieren Sie einen API Key mit den benötigten Berechtigungen

### Problem: Environment-Variablen werden nicht gesetzt
**Lösung:**
1. Prüfen Sie, ob das richtige Environment ausgewählt ist (oben rechts)
2. Schauen Sie in die **Test**-Scripts der Requests (unter "Tests"-Tab)
3. Öffnen Sie die Console (Ansicht → Show Postman Console) für Debug-Logs

## 📚 Weitere Ressourcen

- [API Dokumentation](http://localhost:8080/api-docs/) - Swagger UI (wenn Server läuft)
- [Projekt README](../README.md) - Hauptdokumentation
- [Development Guide](../docs/development/) - Entwickler-Dokumentation

## 🎯 Tipps

1. **Collection Runner**: Führen Sie die gesamte Collection automatisch aus
   - Rechtsklick auf Collection → "Run collection"
   - Nützlich für Regressionstests

2. **Code Generation**: Generieren Sie Code für verschiedene Sprachen
   - Klicken Sie auf einen Request → "Code" (rechts)
   - Unterstützt curl, Python, JavaScript, Go, etc.

3. **Environment-Switcher**: Wechseln Sie schnell zwischen Environments
   - Erstellen Sie verschiedene Environments für Dev, Staging, Production

4. **Pre-request Scripts**: Fügen Sie eigene Scripts hinzu
   - Generieren Sie dynamische Daten
   - Führen Sie Setup-Code aus

## 📞 Support

Bei Fragen oder Problemen:
- Öffnen Sie ein Issue im GitHub Repository
- Konsultieren Sie die API-Dokumentation unter `/api-docs/`

---

**Happy Testing! 🚀**

