# Schnellstart

Dieser Guide führt Sie durch die ersten Schritte mit NeuroQuantumDB nach der [Installation](./installation.md).

## Erste Schritte

### 1. Service starten

```bash
# Service starten (falls nicht automatisch gestartet)
sudo systemctl start neuroquantumdb

# Status überprüfen
curl http://localhost:8080/health
```

### 2. Erste Datenbank erstellen

```bash
# Via REST API
curl -X POST http://localhost:8080/api/v1/databases \
  -H "Content-Type: application/json" \
  -d '{"name": "meine_erste_db", "description": "Meine erste NeuroQuantumDB"}'
```

### 3. Daten einfügen

```bash
# Dokument einfügen
curl -X POST http://localhost:8080/api/v1/databases/meine_erste_db/documents \
  -H "Content-Type: application/json" \
  -d '{
    "id": "user_001",
    "name": "Max Mustermann",
    "age": 30,
    "department": "Engineering"
  }'
```

### 4. Daten abfragen

#### Standard Query
```bash
# Dokument abrufen
curl http://localhost:8080/api/v1/databases/meine_erste_db/documents/user_001
```

#### QSQL Query
```bash
# Structured Query
curl -X POST http://localhost:8080/api/v1/databases/meine_erste_db/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT * FROM documents WHERE age > 25 AND department = \"Engineering\"",
    "language": "qsql"
  }'
```

#### Natural Language Query
```bash
# Natürlichsprachliche Abfrage
curl -X POST http://localhost:8080/api/v1/databases/meine_erste_db/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Finde alle Benutzer im Engineering Department über 25 Jahre",
    "language": "natural"
  }'
```

## Grundlegende Operationen

### Datenbank-Management

```bash
# Alle Datenbanken auflisten
curl http://localhost:8080/api/v1/databases

# Datenbank-Details anzeigen
curl http://localhost:8080/api/v1/databases/meine_erste_db

# Datenbank löschen
curl -X DELETE http://localhost:8080/api/v1/databases/meine_erste_db
```

### Dokument-Operationen

```bash
# Dokument aktualisieren
curl -X PUT http://localhost:8080/api/v1/databases/meine_erste_db/documents/user_001 \
  -H "Content-Type: application/json" \
  -d '{
    "id": "user_001",
    "name": "Max Mustermann",
    "age": 31,
    "department": "Engineering",
    "updated_at": "2024-01-15T10:30:00Z"
  }'

# Dokument löschen
curl -X DELETE http://localhost:8080/api/v1/databases/meine_erste_db/documents/user_001
```

## Web Interface verwenden

NeuroQuantumDB bietet ein integriertes Web-Interface:

```
http://localhost:8080/ui
```

### Features des Web-Interface:
- 📊 **Dashboard**: Übersicht über alle Datenbanken
- 🔍 **Query Builder**: Visueller Query-Editor
- 📈 **Monitoring**: Real-time Performance Metrics
- ⚙️ **Settings**: Konfigurationsmanagement

## Beispiel-Anwendung

Hier ist ein vollständiges Beispiel für eine Benutzer-Verwaltung:

```bash
#!/bin/bash

# Datenbank für Benutzerverwaltung erstellen
curl -X POST http://localhost:8080/api/v1/databases \
  -H "Content-Type: application/json" \
  -d '{"name": "users", "description": "Benutzerverwaltung"}'

# Mehrere Benutzer einfügen
users='[
  {"id": "u001", "name": "Anna Schmidt", "role": "admin", "active": true},
  {"id": "u002", "name": "Tom Weber", "role": "user", "active": true},
  {"id": "u003", "name": "Lisa Müller", "role": "moderator", "active": false}
]'

for user in $(echo "${users}" | jq -r '.[] | @base64'); do
  userData=$(echo ${user} | base64 --decode)
  curl -X POST http://localhost:8080/api/v1/databases/users/documents \
    -H "Content-Type: application/json" \
    -d "${userData}"
done

# Aktive Benutzer abfragen
curl -X POST http://localhost:8080/api/v1/databases/users/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Zeige mir alle aktiven Benutzer",
    "language": "natural"
  }'
```

## Performance-Features

### Neuromorphic Learning
NeuroQuantumDB lernt automatisch von Ihren Abfragemustern:

```bash
# Lernstatus abfragen
curl http://localhost:8080/api/v1/databases/users/learning/status

# Plasticity-Konfiguration anzeigen
curl http://localhost:8080/api/v1/databases/users/plasticity
```

### Quantum Optimizations
Quantenoptimierte Abfragen für komplexe Berechnungen:

```bash
# Quantum Query ausführen
curl -X POST http://localhost:8080/api/v1/databases/users/quantum-query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "QUANTUM_OPTIMIZE(SELECT * FROM documents WHERE complex_calculation(data) > threshold)",
    "optimization_level": "high"
  }'
```

## Monitoring und Metrics

### System Health
```bash
# Gesundheitsstatus
curl http://localhost:8080/health

# Detaillierte Metrics
curl http://localhost:8080/metrics

# Performance-Dashboard
open http://localhost:8080/ui/metrics
```

### Logging
```bash
# Live-Logs ansehen
sudo journalctl -u neuroquantumdb -f

# Detaillierte Debug-Logs aktivieren
sudo systemctl edit neuroquantumdb
# Hinzufügen: Environment=RUST_LOG=debug
```

## Nächste Schritte

Nach diesem Schnellstart empfehlen wir:

1. **[Grundlegende Konzepte](./concepts.md)** - Verstehen Sie die Architektur
2. **[QSQL Guide](./qsql.md)** - Lernen Sie die Query-Sprache
3. **[Natural Language](./natural-language.md)** - Nutzen Sie natürlichsprachliche Abfragen
4. **[Konfiguration](./configuration.md)** - Optimieren Sie die Performance
5. **[Monitoring](./monitoring.md)** - Überwachen Sie Ihre Datenbank

## Hilfe und Support

Bei Problemen:
- 📖 [Troubleshooting Guide](./troubleshooting.md)
- 💬 [Community Discussions](https://github.com/neuroquantumdb/neuroquantumdb/discussions)
- 🐛 [Bug Reports](https://github.com/neuroquantumdb/neuroquantumdb/issues)
