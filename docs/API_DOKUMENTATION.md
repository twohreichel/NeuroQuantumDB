# 🌐 API-Dokumentation - REST-API Referenz

## 🎯 Überblick

Die NeuroQuantumDB REST-API ermöglicht es Ihnen, aus **jeder Programmiersprache** auf die Superkräfte der Datenbank zuzugreifen!

### 🚀 Was Sie erreichen können:
- 🧠 **Neuromorphe Abfragen** über HTTP
- ⚛️ **Quantum-beschleunigte Suchen** 
- 🧬 **DNA-Kompression** per API
- 📊 **Real-time Monitoring** 
- 🔐 **Quantensichere Authentifizierung**

### 📡 API-Basis-URL:
```
http://localhost:8080/api/v1/
```

## 🔐 Authentifizierung

### API-Key anfordern:
```bash
# 🔑 Neuen API-Key generieren
curl -X POST http://localhost:8080/api/v1/auth/generate-key \
  -H "Content-Type: application/json" \
  -d '{"name": "mein-projekt", "permissions": ["read", "write"]}'

# Antwort:
{
  "api_key": "nqdb_1234567890abcdef",
  "expires_at": "2025-09-13T10:00:00Z",
  "permissions": ["read", "write"]
}
```

### API-Key verwenden:
```bash
# 🛡️ Bei jeder Anfrage im Header
curl -H "X-API-Key: nqdb_1234567890abcdef" \
     -H "Content-Type: application/json" \
     http://localhost:8080/api/v1/health
```

## 🧠 Neuromorphic Endpoints

### POST /neuromorphic/query
**Intelligente Abfragen mit automatischem Lernen**

```bash
# 🧠 Neuromorphe QSQL-Abfrage
curl -X POST http://localhost:8080/api/v1/neuromorphic/query \
  -H "X-API-Key: your-key-here" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "NEUROMATCH users WHERE city = \"Berlin\" WITH SYNAPTIC_WEIGHT 0.8",
    "learning_enabled": true,
    "plasticity_threshold": 0.5
  }'

# 📊 Antwort:
{
  "status": "success",
  "execution_time_us": 0.7,
  "results": [
    {"id": 1, "name": "Alice", "city": "Berlin"},
    {"id": 2, "name": "Bob", "city": "Berlin"}
  ],
  "neuromorphic_stats": {
    "synaptic_strength": 0.83,
    "pathway_optimized": true,
    "learning_events": 2
  }
}
```

### GET /neuromorphic/network-status
**Zustand des neuronalen Netzwerks**

```bash
curl http://localhost:8080/api/v1/neuromorphic/network-status \
  -H "X-API-Key: your-key-here"

# 📈 Antwort:
{
  "active_synapses": 2847392,
  "learning_rate": 0.012,
  "plasticity_events_per_second": 1205,
  "memory_efficiency": 94.7,
  "strongest_pathways": [
    {"path": "users->orders", "strength": 0.94},
    {"path": "products->categories", "strength": 0.87}
  ]
}
```

### POST /neuromorphic/train
**Manuelles Training des Netzwerks**

```bash
# 🎓 Netzwerk mit spezifischen Patterns trainieren
curl -X POST http://localhost:8080/api/v1/neuromorphic/train \
  -H "X-API-Key: your-key-here" \
  -H "Content-Type: application/json" \
  -d '{
    "training_data": [
      {"pattern": ["user_login", "search_products", "purchase"], "weight": 0.9},
      {"pattern": ["user_login", "browse_categories", "add_to_cart"], "weight": 0.7}
    ],
    "learning_rate": 0.02,
    "epochs": 100
  }'
```

## ⚛️ Quantum Endpoints

### POST /quantum/search
**Quantum-beschleunigte Suche**

```bash
# ⚛️ Grover's Algorithm für Datenbanksuche
curl -X POST http://localhost:8080/api/v1/quantum/search \
  -H "X-API-Key: your-key-here" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "QUANTUM_SELECT * FROM products WHERE category = \"electronics\"",
    "grover_iterations": 15,
    "amplitude_amplification": true,
    "parallel_processing": true
  }'

# 🚀 Antwort:
{
  "status": "success",
  "execution_time_us": 0.3,
  "quantum_speedup": 15247,
  "results": [...],
  "quantum_stats": {
    "coherence_time_us": 847,
    "error_rate": 0.0001,
    "iterations_used": 12,
    "optimal_iterations": 14
  }
}
```

### POST /quantum/optimize
**Quantum Annealing für Optimierungsprobleme**

```bash
# 🌀 Komplexe Optimierung
curl -X POST http://localhost:8080/api/v1/quantum/optimize \
  -H "X-API-Key: your-key-here" \
  -H "Content-Type: application/json" \
  -d '{
    "problem": {
      "variables": ["index_order", "cache_strategy", "memory_layout"],
      "constraints": ["memory < 100MB", "response_time < 1μs"],
      "objective": "minimize_energy_consumption"
    },
    "annealing_steps": 1000,
    "temperature_schedule": "exponential"
  }'

# 🎯 Antwort:
{
  "status": "optimized",
  "solution": {
    "index_order": "btree_neuromorphic",
    "cache_strategy": "synaptic_lru", 
    "memory_layout": "numa_aware"
  },
  "energy_saving_percent": 23.7,
  "convergence_steps": 847
}
```

### GET /quantum/status
**Quantum-Prozessor Status**

```bash
curl http://localhost:8080/api/v1/quantum/status \
  -H "X-API-Key: your-key-here"

# ⚡ Antwort:
{
  "quantum_processors": 4,
  "active_processors": 4,
  "coherence_time_us": 847,
  "error_rate": 0.0001,
  "current_operations": 12,
  "queue_length": 3,
  "average_speedup": 15247
}
```

## 🧬 DNA Storage Endpoints

### POST /dna/compress
**Daten DNA-komprimieren**

```bash
# 📦 Extreme Kompression
curl -X POST http://localhost:8080/api/v1/dna/compress \
  -H "X-API-Key: your-key-here" \
  -H "Content-Type: application/json" \
  -d '{
    "data": "Sehr langer Datenstring der komprimiert werden soll...",
    "compression_level": 9,
    "error_correction": true,
    "biological_patterns": true
  }'

# 🧬 Antwort:
{
  "status": "compressed",
  "original_size_bytes": 1000000,
  "compressed_size_bytes": 847,
  "compression_ratio": 1180,
  "dna_sequence": "ATCGATCGTAGCTA...",
  "error_correction_codes": "REED_SOLOMON_255_223",
  "estimated_storage_density": "1.8_bits_per_nucleotide"
}
```

### POST /dna/decompress
**DNA-komprimierte Daten entpacken**

```bash
# 📤 Daten wieder entpacken
curl -X POST http://localhost:8080/api/v1/dna/decompress \
  -H "X-API-Key: your-key-here" \
  -H "Content-Type: application/json" \
  -d '{
    "dna_sequence": "ATCGATCGTAGCTA...",
    "error_correction_codes": "REED_SOLOMON_255_223",
    "verify_integrity": true
  }'

# ✅ Antwort:
{
  "status": "decompressed",
  "data": "Originaler Datenstring...",
  "integrity_verified": true,
  "errors_corrected": 0,
  "decompression_time_us": 12.7
}
```

### POST /dna/repair
**Beschädigte DNA-Daten reparieren**

```bash
# 🛠️ Automatische Datenreparatur
curl -X POST http://localhost:8080/api/v1/dna/repair \
  -H "X-API-Key: your-key-here" \
  -H "Content-Type: application/json" \
  -d '{
    "damaged_sequence": "ATCGATXGTAGCTA...",  # X = beschädigtes Nukleotid
    "repair_strategy": "biological_consensus",
    "redundancy_check": true
  }'

# 🔧 Antwort:
{
  "status": "repaired",
  "repaired_sequence": "ATCGATCGTAGCTA...",
  "errors_found": 1,
  "errors_corrected": 1,
  "confidence": 0.987,
  "repair_method": "Reed-Solomon + biological_patterns"
}
```

## 📊 Monitoring & Admin Endpoints

### GET /health
**System-Gesundheit**

```bash
curl http://localhost:8080/api/v1/health

# 💚 Antwort:
{
  "status": "healthy",
  "timestamp": "2024-09-13T10:30:00Z",
  "version": "1.0.0",
  "components": {
    "neuromorphic": {"status": "active", "load": 23.7},
    "quantum": {"status": "optimal", "coherence": 94.3},
    "dna": {"status": "compressing", "efficiency": 99.2},
    "api": {"status": "serving", "response_time_us": 0.4}
  },
  "system_metrics": {
    "memory_usage_mb": 87.3,
    "power_consumption_w": 1.8,
    "active_connections": 1247,
    "queries_per_second": 50000
  }
}
```

### GET /metrics
**Prometheus-kompatible Metriken**

```bash
curl http://localhost:8080/api/v1/metrics

# 📈 Antwort (Prometheus Format):
# TYPE neuroquantum_queries_total counter
neuroquantum_queries_total{type="neuromorphic"} 1234567
neuroquantum_queries_total{type="quantum"} 987654
neuroquantum_queries_total{type="dna"} 456789

# TYPE neuroquantum_response_time_seconds histogram
neuroquantum_response_time_seconds_bucket{le="0.000001"} 945231
neuroquantum_response_time_seconds_bucket{le="0.000005"} 998847
neuroquantum_response_time_seconds_bucket{le="+Inf"} 1000000

# TYPE neuroquantum_compression_ratio gauge
neuroquantum_compression_ratio 1247.3
```

### GET /admin/config
**Aktuelle Konfiguration anzeigen**

```bash
curl http://localhost:8080/api/v1/admin/config \
  -H "X-API-Key: admin-key-here"

# ⚙️ Antwort:
{
  "neuromorphic": {
    "learning_rate": 0.012,
    "plasticity_threshold": 0.5,
    "max_synapses": 1000000,
    "auto_optimization": true
  },
  "quantum": {
    "processors": 4,
    "grover_iterations": 15,
    "annealing_steps": 1000,
    "error_correction": true
  },
  "dna": {
    "compression_level": 9,
    "error_correction": true,
    "cache_size_mb": 64,
    "biological_patterns": true
  }
}
```

### PUT /admin/config
**Konfiguration zur Laufzeit ändern**

```bash
# 🔧 Live-Konfiguration aktualisieren
curl -X PUT http://localhost:8080/api/v1/admin/config \
  -H "X-API-Key: admin-key-here" \
  -H "Content-Type: application/json" \
  -d '{
    "neuromorphic": {
      "learning_rate": 0.015,
      "plasticity_threshold": 0.6
    },
    "quantum": {
      "grover_iterations": 20
    }
  }'

# ✅ Antwort:
{
  "status": "updated",
  "changes_applied": [
    "neuromorphic.learning_rate: 0.012 -> 0.015",
    "neuromorphic.plasticity_threshold: 0.5 -> 0.6", 
    "quantum.grover_iterations: 15 -> 20"
  ],
  "restart_required": false
}
```

## 🔍 WebSocket Real-time API

### Echtzeitdaten über WebSocket

```javascript
// 📡 WebSocket-Verbindung für Live-Updates
const ws = new WebSocket('ws://localhost:8080/api/v1/realtime');

ws.onopen = () => {
    // 🔐 Authentifizierung
    ws.send(JSON.stringify({
        type: 'auth',
        api_key: 'your-key-here'
    }));
    
    // 📊 Metriken abonnieren
    ws.send(JSON.stringify({
        type: 'subscribe',
        channels: ['neuromorphic_learning', 'quantum_operations', 'dna_compression']
    }));
};

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    
    switch(data.type) {
        case 'neuromorphic_learning':
            console.log('🧠 Neue Synapse:', data.pathway, 'Stärke:', data.strength);
            break;
            
        case 'quantum_operation':
            console.log('⚛️ Quantum-Abfrage:', data.duration + 'μs', 'Speedup:', data.speedup);
            break;
            
        case 'dna_compression':
            console.log('🧬 Kompression:', data.ratio + ':1', 'Größe:', data.size_mb + 'MB');
            break;
    }
};
```

## 🐍 Python Client Beispiel

```python
# 🐍 Python-Client für NeuroQuantumDB
import requests
import json

class NeuroQuantumClient:
    def __init__(self, base_url, api_key):
        self.base_url = base_url
        self.headers = {
            'X-API-Key': api_key,
            'Content-Type': 'application/json'
        }
    
    def neuromorphic_query(self, query, learning=True):
        """🧠 Neuromorphe Abfrage ausführen"""
        response = requests.post(
            f"{self.base_url}/neuromorphic/query",
            headers=self.headers,
            json={
                'query': query,
                'learning_enabled': learning,
                'plasticity_threshold': 0.5
            }
        )
        return response.json()
    
    def quantum_search(self, query, iterations=15):
        """⚛️ Quantum-beschleunigte Suche"""
        response = requests.post(
            f"{self.base_url}/quantum/search",
            headers=self.headers,
            json={
                'query': query,
                'grover_iterations': iterations,
                'amplitude_amplification': True
            }
        )
        return response.json()
    
    def dna_compress(self, data, level=9):
        """🧬 Daten DNA-komprimieren"""
        response = requests.post(
            f"{self.base_url}/dna/compress",
            headers=self.headers,
            json={
                'data': data,
                'compression_level': level,
                'error_correction': True
            }
        )
        return response.json()

# 🚀 Verwendung:
client = NeuroQuantumClient('http://localhost:8080/api/v1', 'your-api-key')

# 🧠 Neuromorphe Abfrage
result = client.neuromorphic_query(
    'NEUROMATCH users WHERE city = "Berlin" WITH SYNAPTIC_WEIGHT 0.8'
)
print(f"Gefunden: {len(result['results'])} Benutzer in {result['execution_time_us']}μs")

# ⚛️ Quantum-Suche
quantum_result = client.quantum_search(
    'QUANTUM_SELECT * FROM products WHERE price < 100'
)
print(f"Quantum-Speedup: {quantum_result['quantum_speedup']}x")

# 🧬 DNA-Kompression
compress_result = client.dna_compress("Sehr langer Text..." * 1000)
print(f"Kompression: {compress_result['compression_ratio']}:1")
```

## 🔒 Sicherheit

### Rate Limiting
```bash
# ⚡ Automatische Begrenzung bei zu vielen Anfragen
# Antwort bei Überschreitung:
{
  "error": "rate_limit_exceeded",
  "limit": 1000,
  "window_seconds": 60,
  "retry_after": 30
}
```

### Quantum-resistente Verschlüsselung
- 🛡️ **Kyber-768** für Schlüsselaustausch
- 🔐 **Dilithium-3** für digitale Signaturen  
- 🌐 **TLS 1.3** mit Post-Quantum Ciphers

### CORS-Konfiguration
```bash
# 🌐 Erlaubte Origins konfigurieren
curl -X PUT http://localhost:8080/api/v1/admin/cors \
  -H "X-API-Key: admin-key" \
  -d '{
    "allowed_origins": ["https://myapp.com", "https://dashboard.mycompany.com"],
    "allowed_methods": ["GET", "POST", "PUT"],
    "allowed_headers": ["X-API-Key", "Content-Type"]
  }'
```

## 🚨 Fehlerbehandlung

### HTTP Status Codes
- `200` ✅ Erfolgreich
- `400` ❌ Ungültige Anfrage
- `401` 🔐 Authentifizierung erforderlich
- `403` 🚫 Nicht berechtigt
- `429` ⚡ Rate Limit erreicht
- `500` 💥 Server-Fehler
- `503` 🔧 Service nicht verfügbar

### Fehler-Format
```json
{
  "error": {
    "code": "NEUROMORPHIC_LEARNING_FAILED",
    "message": "Synaptic pathway could not be strengthened",
    "details": {
      "pathway": "users->orders",
      "current_strength": 0.23,
      "required_threshold": 0.5
    },
    "suggestions": [
      "Increase plasticity_threshold",
      "Provide more training data",
      "Check learning_rate configuration"
    ]
  },
  "request_id": "req_123456789",
  "timestamp": "2024-09-13T10:30:00Z"
}
```

## 📚 Code-Generierung

### OpenAPI Schema
```bash
# 📄 OpenAPI-Spezifikation herunterladen
curl http://localhost:8080/api/v1/openapi.json > neuroquantum-api.json

# 🔧 Client-Code generieren
npx @openapitools/openapi-generator-cli generate \
  -i neuroquantum-api.json \
  -g python \
  -o ./generated-client
```

---

## 🎉 Jetzt sind Sie API-Profi!

Sie können jetzt:
- ✅ Alle NeuroQuantumDB Features per REST-API nutzen
- ✅ In jeder Programmiersprache integrieren
- ✅ Real-time Monitoring implementieren
- ✅ Sicherheit und Performance optimieren

### Nächste Schritte:
1. 🚀 **[Production Deployment](PRODUCTION_DEPLOYMENT.md)** - Live schalten
2. ❓ **[FAQ](FAQ.md)** - Häufige API-Fragen
3. 🔧 **[Troubleshooting](TROUBLESHOOTING.md)** - Probleme lösen

---

> **💡 Pro-Tipp:** Nutzen Sie die WebSocket-API für Echtzeitanwendungen und die REST-API für Standard-Integrationen!

> **🚀 Performance:** Kombinieren Sie mehrere API-Calls mit Batch-Requests für maximale Effizienz!
