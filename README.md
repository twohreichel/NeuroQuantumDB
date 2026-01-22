# 🧠 NeuroQuantumDB - The Intelligent Database Wonder

<div align="center">

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/neuroquantumdb/neuroquantumdb)
[![License](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)
[![ARM64](https://img.shields.io/badge/platform-ARM64-orange)](https://www.raspberrypi.org)
[![Docker](https://img.shields.io/badge/docker-ready-2496ED)](https://hub.docker.com)

*Revolutionary database architecture combining neuromorphic computing, quantum-inspired algorithms, and DNA-storage principles for ultra-efficient edge computing applications on Raspberry Pi 4*

</div>

---

## 💖 Support This Project

If you find this extension helpful, please consider supporting its development! Your sponsorship helps maintain and improve this project.

[![Sponsor on GitHub](https://img.shields.io/badge/Sponsor-%E2%9D%A4-red?logo=github)](https://github.com/sponsors/twohreichel)

Every contribution, no matter the size, is greatly appreciated and helps ensure the continued development of this extension. Thank you for your support! 🙏

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

---

### Libraries
In the following folders, you can find the latest libraries for interacting with the database in a programming language of your choice. These have also been stored as tags and can be installed via the most common providers (as example packagist, GitHub, Maven, PiPy):

* connecting-libraries/php/

---

## ⚠️ Cluster Mode (Beta)

**The cluster mode is currently in development and should not be used in production environments.**

The multi-node cluster functionality is available as a **Beta/Preview feature** for testing and development purposes. The following features are still missing or incomplete:

- ❌ **gRPC Network Transport** - Inter-node communication not fully implemented
- ❌ **Complete Raft Implementation** - Consensus protocol is partial
- ❌ **Service Discovery** - DNS/Consul/etcd integration not yet available
- ❌ **Full Replication** - Data replication has limitations

### 🎯 Deployment Recommendations

| Deployment Type | Status | Use Case |
|-----------------|--------|----------|
| **Single-Node** | ✅ Production-Ready | Recommended for all production workloads |
| **Multi-Node Cluster** | ⚠️ Beta/Preview | Development and testing only |

**For production environments, we strongly recommend single-node deployments until the cluster module reaches stable release.**

### 📅 Cluster Roadmap

The full cluster implementation is planned for 2026 as part of our distributed architecture milestone. See [Future Vision](./docs/concept/06-future-vision.md#mid-term-2026-distributed-architecture) for details on the roadmap.

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

### 🧠 Complete Feature Guides

**Comprehensive guides explaining Quantum Search, Neural Endpoints, and DNA Compression in detail:**

- **[🇩🇪 Feature Guide (Deutsch)](./docs/user-guide/NEUROQUANTUM_FEATURES_GUIDE.md)** - Detaillierte Erklärungen aller Features
- **[🇬🇧 Feature Guide (English)](./docs/user-guide/NEUROQUANTUM_FEATURES_GUIDE_EN.md)** - Detailed explanations of all features

*These guides explain complex concepts in simple terms that anyone can understand!*

### 🌐 Additional Resources

- **API Documentation**: Run `make docs-api` and open `target/doc/index.html`
- **Interactive API Docs**: Start the server and visit `http://localhost:8080/api-docs/`
- **Generate All Docs**: Run `make docs` to generate complete documentation
- **Serve Docs Locally**: Run `make docs-serve` to browse at `http://localhost:8000`

---

Have a look at the [Wiki](https://twoh-me.github.io/NeuroQuantumDB/) for more information.

---

## 🚀 Release Process & Contributing

This repository uses a **monorepo structure** with automated releases via [release-please](https://github.com/googleapis/release-please).

### Conventional Commits

All commits must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. This enables automatic versioning and changelog generation.

**Commit Format:**
```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:**
| Type | Description | Version Bump |
|------|-------------|--------------|
| `feat` | New feature | Minor |
| `fix` | Bug fix | Patch |
| `feat!` or `BREAKING CHANGE:` | Breaking change | Major (v1.0.0) |
| `perf` | Performance improvement | Patch |
| `docs` | Documentation only | None |
| `refactor` | Code refactoring | None |
| `test` | Adding tests | None |
| `ci` | CI/CD changes | None |
| `chore` | Maintenance | None |

### Scope Mapping

Scopes route commits to the correct package for versioning:

| Scope | Target Package | Example |
|-------|---------------|---------|
| `api`, `core`, `cluster`, `qsql`, `wasm` | NeuroQuantumDB (main) | `feat(api): add batch endpoint` |
| `php` | PHP-Driver | `fix(php): connection timeout` |
| (no scope) | NeuroQuantumDB (main) | `feat: improve query parser` |

### Release Tags

| Component | Tag Format | Example |
|-----------|------------|---------|
| NeuroQuantumDB | `v{version}` | `v1.0.0`, `v1.1.0` |
| PHP-Driver | `php-driver/v{version}` | `php-driver/v1.0.0` |

### Release Artifacts

When a NeuroQuantumDB release is created:
- **Multi-platform binaries**: Linux x86_64, macOS arm64, macOS x86_64, Windows x86_64
- **WASM package**: npm tarball for browser integration
- **crates.io**: All 5 crates published in dependency order

### Setup Prerequisites (for Maintainers)

To enable automatic publishing, add the following repository secret:

| Secret | Purpose | How to Obtain |
|--------|---------|---------------|
| `CARGO_REGISTRY_TOKEN` | Publish crates to crates.io | Generate at https://crates.io/settings/tokens |

**Setup Steps:**
1. Go to Repository → Settings → Secrets and variables → Actions
2. Add `CARGO_REGISTRY_TOKEN` as a repository secret
3. Ensure GitHub Actions has write permissions (Settings → Actions → General → Workflow permissions → Read and write permissions)

### Adding New Connecting Libraries

When adding a new library (e.g., Python, Node.js):

1. **Add to `release-please-config.json`:**
   ```json
   "connecting-libraries/python": {
     "release-type": "python",
     "component": "python-driver",
     "include-component-in-tag": true,
     "bump-minor-pre-major": false
   }
   ```

2. **Add to `.release-please-manifest.json`:**
   ```json
   "connecting-libraries/python": "0.1.0"
   ```

3. **Add scope mapping** (document in this README under Scope Mapping)

4. **Create `connecting-libraries/<library>/CHANGELOG.md`**

5. **Add publish job** to `.github/workflows/release.yml` if needed (e.g., PyPI, npm)
