# API Referenz

## REST API Endpoints

### Basis-URL
```
https://your-domain.com/api/v1
```

### Authentifizierung
Alle API-Aufrufe erfordern einen gültigen API-Schlüssel im Header:
```
Authorization: Bearer YOUR_API_KEY
```

## Datenbank-Management

### GET /databases
Listet alle verfügbaren Datenbanken auf.

**Response:**
```json
{
  "databases": [
    {
      "name": "users",
      "description": "Benutzerverwaltung",
      "created_at": "2024-01-15T10:30:00Z",
      "size": 1048576,
      "document_count": 1234
    }
  ]
}
```

### POST /databases
Erstellt eine neue Datenbank.

**Request:**
```json
{
  "name": "neue_db",
  "description": "Beschreibung der neuen Datenbank",
  "config": {
    "plasticity_enabled": true,
    "quantum_optimization": "medium"
  }
}
```

### GET /databases/{name}
Ruft Details einer spezifischen Datenbank ab.

### DELETE /databases/{name}
Löscht eine Datenbank unwiderruflich.

## Dokument-Operationen

### POST /databases/{name}/documents
Fügt ein neues Dokument hinzu.

**Request:**
```json
{
  "id": "doc_001",
  "data": {
    "field1": "value1",
    "nested": {
      "field2": 42
    }
  }
}
```

### GET /databases/{name}/documents/{id}
Ruft ein spezifisches Dokument ab.

### PUT /databases/{name}/documents/{id}
Aktualisiert ein Dokument vollständig.

### PATCH /databases/{name}/documents/{id}
Aktualisiert Teile eines Dokuments.

### DELETE /databases/{name}/documents/{id}
Löscht ein Dokument.

## Query-Operationen

### POST /databases/{name}/query
Führt eine QSQL-Abfrage aus.

**Request:**
```json
{
  "query": "SELECT * FROM documents WHERE age > 25",
  "language": "qsql",
  "options": {
    "plasticity": 0.8,
    "quantum_optimization": true
  }
}
```

### POST /databases/{name}/natural-query
Führt eine natürlichsprachliche Abfrage aus.

**Request:**
```json
{
  "query": "Finde alle Benutzer über 25 Jahre",
  "language": "de"
}
```

## Monitoring & Metriken

### GET /health
System-Gesundheitscheck.

### GET /metrics
Prometheus-kompatible Metriken.

### GET /metrics/plasticity
Neuroplastizitäts-Metriken.

### GET /metrics/quantum
Quantum-Optimierung Metriken.
