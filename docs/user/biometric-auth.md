# EEG-basierte biometrische Authentifizierung

## Überblick

NeuroQuantumDB bietet eine fortschrittliche EEG-basierte (Elektroenzephalographie) biometrische Authentifizierung. Diese innovative Funktion nutzt einzigartige Gehirnwellenmuster zur Benutzeridentifikation und passt perfekt zum neuromorphen Charakter der Datenbank.

## Features

- **Einzigartige Brainwave-Signaturen**: Jeder Benutzer hat ein einzigartiges EEG-Muster
- **Signalverarbeitung**: Fortschrittliche FFT-Analyse zur Extraktion von Frequenzbändern
- **Adaptive Enrollment**: Verbesserte Genauigkeit durch mehrfache Registrierungen
- **Hohe Sicherheit**: Schwer zu fälschen im Vergleich zu traditionellen Methoden

## EEG-Frequenzbänder

Die Implementierung analysiert folgende Frequenzbänder:

- **Delta (0.5-4 Hz)**: Tiefschlaf
- **Theta (4-8 Hz)**: Schläfrigkeit, Meditation
- **Alpha (8-13 Hz)**: Entspannung, Ruhe
- **Beta (13-30 Hz)**: Aktives Denken, Fokus
- **Gamma (30-100 Hz)**: Hochlevel-Informationsverarbeitung

## API-Endpunkte

### 1. Benutzer registrieren

Registriert einen neuen Benutzer mit EEG-Daten.

**Endpoint**: `POST /api/v1/biometric/eeg/enroll`

**Berechtigungen**: Erfordert `admin` Berechtigung

**Request Body**:
```json
{
  "user_id": "john_doe",
  "sampling_rate": 256.0,
  "raw_eeg_data": [0.1, 0.2, -0.1, 0.3, ...],
  "channel": "Fp1"
}
```

**Parameter**:
- `user_id`: Eindeutige Benutzer-ID (3-100 Zeichen)
- `sampling_rate`: EEG-Abtastrate in Hz (128-2048 Hz)
- `raw_eeg_data`: Rohe EEG-Samples (mind. 2 Sekunden Daten)
- `channel`: Optional - EEG-Kanal-Name

**Response**:
```json
{
  "success": true,
  "data": {
    "user_id": "john_doe",
    "enrolled": true,
    "signature_quality": 85.5,
    "enrollment_count": 1,
    "created_at": "2025-10-28T10:30:00Z"
  },
  "metadata": {
    "response_time_ms": 125.3,
    "message": "User john_doe enrolled with EEG biometric signature"
  }
}
```

### 2. Benutzer authentifizieren

Authentifiziert einen Benutzer mittels EEG-Daten.

**Endpoint**: `POST /api/v1/biometric/eeg/authenticate`

**Request Body**:
```json
{
  "user_id": "john_doe",
  "sampling_rate": 256.0,
  "raw_eeg_data": [0.1, 0.2, -0.1, 0.3, ...]
}
```

**Success Response** (Authentifizierung erfolgreich):
```json
{
  "success": true,
  "data": {
    "authenticated": true,
    "user_id": "john_doe",
    "similarity_score": 0.92,
    "threshold": 0.85,
    "timestamp": "2025-10-28T10:35:00Z"
  }
}
```

**Failure Response** (Authentifizierung fehlgeschlagen):
```json
{
  "success": false,
  "error": {
    "Unauthorized": "EEG authentication failed: signature mismatch"
  },
  "data": {
    "authenticated": false,
    "user_id": "john_doe",
    "similarity_score": 0.72,
    "threshold": 0.85,
    "timestamp": "2025-10-28T10:35:00Z"
  }
}
```

### 3. Signatur aktualisieren

Verbessert die Benutzersignatur mit zusätzlichen EEG-Samples.

**Endpoint**: `POST /api/v1/biometric/eeg/update`

**Berechtigungen**: Erfordert `admin` Berechtigung

**Request Body**:
```json
{
  "user_id": "john_doe",
  "sampling_rate": 256.0,
  "raw_eeg_data": [0.1, 0.2, -0.1, 0.3, ...]
}
```

**Response**:
```json
{
  "success": true,
  "data": "EEG signature updated for user john_doe",
  "metadata": {
    "response_time_ms": 98.2,
    "message": "Signature updated successfully"
  }
}
```

### 4. Registrierte Benutzer auflisten

Listet alle Benutzer mit EEG-Signaturen auf.

**Endpoint**: `GET /api/v1/biometric/eeg/users`

**Berechtigungen**: Erfordert `admin` Berechtigung

**Response**:
```json
{
  "success": true,
  "data": ["john_doe", "jane_smith", "alice_wonder"],
  "metadata": {
    "response_time_ms": 5.2,
    "message": "EEG enrolled users retrieved"
  }
}
```

## Verwendungsbeispiel (Python)

```python
import requests
import numpy as np

BASE_URL = "http://localhost:8080"
API_KEY = "nqdb_your_admin_api_key"

headers = {
    "X-API-Key": API_KEY,
    "Content-Type": "application/json"
}

# 1. EEG-Daten generieren oder von Hardware auslesen
# Beispiel: 3 Sekunden bei 256 Hz = 768 Samples
sampling_rate = 256.0
duration = 3.0
num_samples = int(sampling_rate * duration)

# Simulierte EEG-Daten (in der Praxis von echtem EEG-Gerät)
eeg_data = np.random.randn(num_samples).tolist()

# 2. Benutzer registrieren
enroll_data = {
    "user_id": "neuroscientist_alice",
    "sampling_rate": sampling_rate,
    "raw_eeg_data": eeg_data,
    "channel": "Fp1"
}

response = requests.post(
    f"{BASE_URL}/api/v1/biometric/eeg/enroll",
    json=enroll_data,
    headers=headers
)

print("Enrollment:", response.json())

# 3. Neue EEG-Daten für Authentifizierung erfassen
auth_eeg_data = np.random.randn(num_samples).tolist()

auth_data = {
    "user_id": "neuroscientist_alice",
    "sampling_rate": sampling_rate,
    "raw_eeg_data": auth_eeg_data
}

response = requests.post(
    f"{BASE_URL}/api/v1/biometric/eeg/authenticate",
    json=auth_data,
    headers=headers
)

result = response.json()
if result["data"]["authenticated"]:
    print(f"✅ Authentication successful! Score: {result['data']['similarity_score']:.2%}")
else:
    print(f"❌ Authentication failed. Score: {result['data']['similarity_score']:.2%}")
```

## Verwendungsbeispiel (Rust)

```rust
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:8080";
    let api_key = "nqdb_your_admin_api_key";
    
    // Simulierte EEG-Daten (768 Samples für 3 Sekunden bei 256 Hz)
    let eeg_data: Vec<f32> = (0..768)
        .map(|i| (i as f32 * 0.1).sin() + rand::random::<f32>() * 0.1)
        .collect();
    
    // Benutzer registrieren
    let enroll_payload = json!({
        "user_id": "rust_developer",
        "sampling_rate": 256.0,
        "raw_eeg_data": eeg_data,
        "channel": "Fp1"
    });
    
    let response = client
        .post(&format!("{}/api/v1/biometric/eeg/enroll", base_url))
        .header("X-API-Key", api_key)
        .json(&enroll_payload)
        .send()
        .await?;
    
    println!("Enrollment response: {}", response.text().await?);
    
    Ok(())
}
```

## Signalverarbeitung

Die EEG-Signalverarbeitung umfasst:

1. **Rauschfilterung**: Entfernung von Netzstörungen (50/60 Hz)
2. **Bandpass-Filterung**: Fokussierung auf relevante Frequenzbereiche
3. **FFT-Analyse**: Transformation in den Frequenzbereich
4. **Feature-Extraktion**: 
   - Leistung in jedem Frequenzband
   - Statistische Merkmale (Mittelwert, Standardabweichung)
   - Verhältnisse zwischen Bändern (Alpha/Beta, Theta/Alpha)
5. **Normalisierung**: Standardisierung der Features
6. **Ähnlichkeitsberechnung**: Kosinus-Ähnlichkeit zwischen Signaturen

## Sicherheitsüberlegungen

### Vorteile
- ✅ Einzigartige, schwer zu fälschende biometrische Signatur
- ✅ Kontinuierliche Authentifizierung möglich
- ✅ Nicht-invasiv und benutzerfreundlich
- ✅ Passt perfekt zu neuromorphen Datenbanken

### Einschränkungen
- ⚠️ Erfordert EEG-Hardware (z.B. OpenBCI, NeuroSky, Emotiv)
- ⚠️ Signalqualität kann durch Bewegung beeinträchtigt werden
- ⚠️ Mentaler Zustand kann Variabilität verursachen
- ⚠️ Längere Aufnahmezeit als traditionelle Methoden

### Best Practices

1. **Mehrfache Enrollment-Sessions**: Verbessert die Genauigkeit
2. **Konsistente Bedingungen**: Gleiche Umgebung und mentaler Zustand
3. **Signalqualitätsprüfung**: Mindestens 60% Signalqualität erforderlich
4. **Regelmäßige Updates**: Signatur periodisch aktualisieren
5. **Fallback-Authentifizierung**: Zusätzliche Auth-Methode bereitstellen

## Hardware-Empfehlungen

### Professionelle Geräte
- **OpenBCI Cyton** (8 Kanäle, Open Source)
- **Emotiv EPOC X** (14 Kanäle, kommerziell)
- **g.tec Unicorn** (8 Kanäle, wissenschaftlich)

### Consumer-Geräte
- **NeuroSky MindWave** (1 Kanal, günstig)
- **Muse 2** (4 Kanäle, Meditation)

### Mindestanforderungen
- Abtastrate: 128-256 Hz (höher ist besser)
- Kanäle: Mindestens 1-2 (mehr verbessert Genauigkeit)
- Auflösung: 12-24 Bit ADC

## Performance

Typische Verarbeitungszeiten:
- Enrollment: 100-200ms
- Authentifizierung: 80-150ms
- Signatur-Update: 90-180ms

Speicherbedarf:
- Pro Benutzer: ~5-10 KB
- Rohdaten (3s @ 256Hz): ~3 KB

## Fehlerbehandlung

| Fehlercode | Beschreibung | Lösung |
|------------|--------------|--------|
| 400 | Invalid EEG data | Daten prüfen, mind. 2 Sekunden |
| 400 | Poor signal quality | Elektroden-Kontakt verbessern |
| 401 | Authentication failed | Schwellwert anpassen oder neu registrieren |
| 403 | Admin permission required | Berechtigungen prüfen |
| 500 | Failed to initialize EEG service | Abtastrate prüfen (128-2048 Hz) |

## Zukünftige Erweiterungen

- 🔮 Multi-Kanal-Unterstützung für höhere Genauigkeit
- 🔮 Echtzeit-Streaming-Authentifizierung
- 🔮 Emotions- und Kognitionszustands-Erkennung
- 🔮 Adaptive Schwellwertanpassung
- 🔮 Quantum-sichere EEG-Verschlüsselung

## Referenzen

- [OpenBCI Documentation](https://docs.openbci.com/)
- [EEG Signal Processing](https://mne.tools/)
- [Brain-Computer Interfaces](https://www.bcisociety.org/)

## Support

Bei Fragen oder Problemen:
- GitHub Issues: [NeuroQuantumDB Issues](https://github.com/neuroquantumdb/neuroquantumdb/issues)
- Dokumentation: [API Reference](../api/rest.md)

