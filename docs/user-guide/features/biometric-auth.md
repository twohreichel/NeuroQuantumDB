# Biometric Authentication

EEG-based authentication for high-security applications.

## Overview

Uses brainwave patterns for user authentication:

```
EEG Signal → Digital Filter → Feature Extraction → Verification
```

## Supported Signals

| Band | Frequency | Use |
|------|-----------|-----|
| Delta | 0.5-4 Hz | Deep patterns |
| Theta | 4-8 Hz | Memory patterns |
| Alpha | 8-13 Hz | Relaxed state |
| Beta | 13-30 Hz | Active thinking |
| Gamma | 30-100 Hz | Cognitive processing |

## Usage

### Enroll User

```bash
curl -X POST http://localhost:8080/api/v1/biometric/enroll \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "eeg_samples": [...],
    "sampling_rate": 256
  }'
```

### Authenticate

```bash
curl -X POST http://localhost:8080/api/v1/biometric/verify \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "user123",
    "eeg_sample": [...]
  }'
```

## Security

| Feature | Status |
|---------|--------|
| Signal encryption | AES-256-GCM |
| Template storage | Hashed + salted |
| Replay protection | Timestamp validation |
| Liveness detection | Pattern analysis |

## Hardware Requirements

- EEG headset (OpenBCI, Muse, etc.)
- Minimum 8 channels
- 256 Hz sampling rate
