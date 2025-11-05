# 🧠 NeuroQuantumDB - Das intelligente Datenbank-Wunder

<div align="center">

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/neuroquantumdb/neuroquantumdb)
[![License](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)
[![ARM64](https://img.shields.io/badge/platform-ARM64-orange)](https://www.raspberrypi.org)
[![Docker](https://img.shields.io/badge/docker-ready-2496ED)](https://hub.docker.com)

*Revolutionary database architecture combining neuromorphic computing, quantum-inspired algorithms, and DNA-storage principles for ultra-efficient edge computing applications on Raspberry Pi 4*

</div>

---

## ⚡ Quick Start für Entwickler

### 🚀 Automatisches Setup (Empfohlen)

Nach dem Klonen des Repositories führen Sie einfach aus:

```bash
# Repository klonen
git clone https://github.com/neuroquantumdb/neuroquantumdb.git
cd neuroquantumdb

# Automatisches Development Setup
./scripts/setup-dev.sh
```

Das Setup-Script installiert automatisch:
- ✅ Alle erforderlichen Rust-Tools (cargo-audit, cargo-deny, cargo-machete)
- ✅ Pre-commit Hooks für Code-Qualität
- ✅ Git-Konfiguration für optimalen Workflow
- ✅ Post-merge Hooks für Dependency-Updates
- ✅ Commit-Message Validation

---

## 🔒 Security & Production Setup

### Initial Setup (Required)

NeuroQuantumDB uses secure initialization instead of default credentials:

```bash
# Initialize the database with your first admin key
neuroquantum-api init

# Or non-interactive with custom settings
neuroquantum-api init --name admin --expiry-hours 8760 --output .env --yes

# Generate a secure JWT secret for production
neuroquantum-api generate-jwt-secret --output config/jwt-secret.txt
```

### Security Features

- ✅ **No Default Credentials** - Requires explicit initialization
- ✅ **JWT Authentication** - Secure token-based authentication
- ✅ **API Key Management** - Granular permission control
- ✅ **Rate Limiting** - Protection against abuse (5 key generations/hour per IP)
- ✅ **IP Whitelisting** - Admin endpoints protected by IP whitelist
- ✅ **Post-Quantum Crypto** - ML-KEM & ML-DSA ready
- ✅ **Biometric Auth** - EEG-based authentication support

### Production Configuration

Edit `config/prod.toml`:

```toml
[auth]
jwt_secret = "YOUR-GENERATED-SECRET-HERE"
jwt_expiration_hours = 8

[security]
admin_ip_whitelist = [
    "127.0.0.1",
    "::1",
    "YOUR-ADMIN-IP-HERE"
]
```

📖 **Full Documentation:** See [SECURITY_HARDENING.md](./SECURITY_HARDENING.md) for complete security guide.

---

## 🧪 API Testing mit Postman

Die komplette API kann lokal mit Postman getestet werden:

### 📥 Import & Setup (2 Minuten)

1. **Importiere die Postman Collection:**
   - Öffne Postman
   - Klicke auf "Import"
   - Ziehe die Dateien aus `postman/` in das Import-Fenster:
     - `NeuroQuantumDB.postman_collection.json`
     - `NeuroQuantumDB.postman_environment.json`

2. **Environment aktivieren:**
   - Wähle oben rechts "NeuroQuantumDB Local"

3. **Starte den Server:**
   ```bash
   cargo run --bin neuroquantum-api
   ```

4. **Teste die API:**
   - Health Check → Login → Create Table → Insert Data
   - **Der Token wird automatisch gespeichert!** ✨

### 🎯 Verfügbare Endpunkte

Die Postman Collection enthält fertige Requests für:

- ✅ **Authentication** - Login, Token Refresh, API Key Management
- ✅ **CRUD Operations** - Create, Read, Update, Delete mit SQL
- ✅ **Neural Networks** - Training und Status-Abfrage
- ✅ **Quantum Search** - Grover's Algorithmus Suche
- ✅ **DNA Compression** - DNA-Sequenz Kompression
- ✅ **Biometric Auth** - EEG-basierte Authentifizierung
- ✅ **Monitoring** - Prometheus Metriken & Performance Stats

📖 **Detaillierte Anleitung:** Siehe [postman/README.md](./postman/README.md)

---

Have a look at the [Wiki](https://twoh-me.github.io/NeuroQuantumDB/) for more information.