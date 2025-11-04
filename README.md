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

Have a look at the [Wiki](https://twoh-me.github.io/NeuroQuantumDB/) for more information.