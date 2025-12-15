# 🧠 NeuroQuantumDB - The Intelligent Database Wonder

<div align="center">

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/neuroquantumdb/neuroquantumdb)
[![License](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)
[![ARM64](https://img.shields.io/badge/platform-ARM64-orange)](https://www.raspberrypi.org)
[![Docker](https://img.shields.io/badge/docker-ready-2496ED)](https://hub.docker.com)

*Revolutionary database architecture combining neuromorphic computing, quantum-inspired algorithms, and DNA-storage principles for ultra-efficient edge computing applications on Raspberry Pi 4*

</div>

---

## ⚡ Quick Start for Developers

### 🚀 Automated Setup (Recommended)

After cloning the repository, simply run:

```bash
# Clone repository
git clone https://github.com/neuroquantumdb/neuroquantumdb.git
cd neuroquantumdb
```

The setup script automatically installs:
- ✅ All required Rust tools (cargo-audit, cargo-deny, cargo-machete)
- ✅ Pre-commit hooks for code quality
- ✅ Git configuration for optimal workflow
- ✅ Post-merge hooks for dependency updates
- ✅ Commit message validation

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

## 🧪 API Testing with Postman

The complete API can be tested locally with Postman:

### 📥 Import & Setup (2 Minutes)

1. **Import the Postman Collection:**
   - Open Postman
   - Click on "Import"
   - Drag the files from `postman/` into the Import window:
     - `NeuroQuantumDB.postman_collection.json`
     - `NeuroQuantumDB.postman_environment.json`

2. **Activate Environment:**
   - Select "NeuroQuantumDB Local" in the top right

3. **Start the Server:**
   ```bash
   cargo run --bin neuroquantum-api
   ```

4. **Test the API:**
   - Health Check → Login → Create Table → Insert Data
   - **The token is automatically saved!** ✨

### 🎯 Available Endpoints

The Postman Collection contains ready-made requests for:

- ✅ **Authentication** - Login, Token Refresh, API Key Management
- ✅ **CRUD Operations** - Create, Read, Update, Delete with SQL
- ✅ **Neural Networks** - Training and status queries
- ✅ **Quantum Search** - Grover's algorithm search
- ✅ **DNA Compression** - DNA sequence compression
- ✅ **Biometric Auth** - EEG-based authentication
- ✅ **Monitoring** - Prometheus metrics & Performance Stats

📖 **Detailed Guide:** See [postman/README.md](./postman/README.md)

---

## 📚 Documentation

Comprehensive documentation is available for developers and users:

### 📖 Documentation Index

**[Complete Documentation](./docs/README.md)** - Overview and navigation to all documentation resources

### 🧠 Concept & Vision

**[Project Conception](./concept/README.md)** - The origin story and design philosophy:
- How a small idea evolved into NeuroQuantumDB over three years
- Neuroscience foundations — the brain as architectural blueprint
- Core principles: Self-learning, DNA encoding, quantum-inspired algorithms
- Technical evolution and milestone timeline
- Future vision and roadmap

### 🔧 For Developers

**[Developer Guide](./docs/developer_guide.md)** - Complete technical reference including:
- System architecture and design principles
- Core component internals (Storage Engine, DNA Compression, Quantum Processor)
- API reference and implementation details
- Development setup and build process
- Testing, benchmarking, and performance optimization
- Security architecture and best practices
- Contributing guidelines

### 👥 For Users

**[User Guide](./docs/user_guide.md)** - Practical guide for using NeuroQuantumDB:
- Quick start and installation instructions
- Configuration and deployment
- Using the REST API with examples
- QSQL query language reference
- Advanced features (DNA compression, quantum search, neural networks)
- Monitoring and maintenance
- Troubleshooting and FAQ

### 🌐 Additional Resources

- **API Documentation**: Run `make docs-api` and open `target/doc/index.html`
- **Interactive API Docs**: Start the server and visit `http://localhost:8080/api-docs/`
- **Generate All Docs**: Run `make docs` to generate complete documentation
- **Serve Docs Locally**: Run `make docs-serve` to browse at `http://localhost:8000`

---

Have a look at the [Wiki](https://twoh-me.github.io/NeuroQuantumDB/) for more information.