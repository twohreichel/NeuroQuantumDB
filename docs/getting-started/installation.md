# Installation

This guide covers the installation of NeuroQuantumDB on various platforms, with special focus on Raspberry Pi 4 (ARM64).

## Prerequisites

### Hardware Requirements (Minimum)

#### Raspberry Pi 4 (Recommended)
- **CPU:** ARM Cortex-A72 (4 cores) @ 1.5GHz
- **RAM:** 2GB (4GB+ recommended for production)
- **Storage:** 8GB+ SD card or SSD
- **OS:** Raspberry Pi OS (64-bit) or Ubuntu 22.04 LTS ARM64

#### Desktop/Server
- **CPU:** x86_64 or ARM64
- **RAM:** 4GB+
- **Storage:** 10GB+
- **OS:** Linux, macOS, or Windows (WSL2)

### Software Prerequisites

- **Rust:** 1.75+ (Edition 2021)
- **Git:** 2.30+
- **Docker:** 24.0+ (optional, für Container-Deployment)
- **Redis:** 7.0+ (optional, für distributed rate limiting)

---

## Installation Methods

### Method 1: Docker (Recommended for Production)

The easiest way to run NeuroQuantumDB in production:

```bash
# Pull the official image
docker pull neuroquantumdb/neuroquantumdb:latest

# Run with persistent storage
docker run -d \
  --name neuroquantumdb \
  -p 8080:8080 \
  -v neuroquantum-data:/data \
  -v neuroquantum-config:/config \
  -e RUST_LOG=info \
  neuroquantumdb/neuroquantumdb:latest

# Check health
curl http://localhost:8080/health
```

**Docker Compose (Full Stack):**

```bash
# Clone repository
git clone https://github.com/neuroquantumdb/neuroquantumdb.git
cd neuroquantumdb

# Start complete stack (DB + Redis + Prometheus + Grafana)
docker-compose -f docker/production/docker-compose.yml up -d
```

---

### Method 2: From Source (Recommended for Development)

#### Step 1: Install Rust

```bash
# Install rustup (Rust installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Configure current shell
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### Step 2: Clone Repository

```bash
git clone https://github.com/neuroquantumdb/neuroquantumdb.git
cd neuroquantumdb
```

#### Step 3: Automated Dev Setup

```bash
# Run setup script (installs tools, hooks, dependencies)
./scripts/setup-dev.sh
```

The setup script installs:
- ✅ `cargo-audit` - Security vulnerability scanning
- ✅ `cargo-deny` - License and dependency checking
- ✅ `cargo-machete` - Unused dependency detection
- ✅ Git hooks (pre-commit, post-merge, commit-msg)
- ✅ Development dependencies

#### Step 4: Build & Test

```bash
# Build all crates (debug mode)
cargo build --workspace

# Run all tests
cargo test --workspace

# Build release binary (optimized)
cargo build --release --workspace

# Binary location
./target/release/neuroquantum-api
```

---

### Method 3: Pre-built Binaries

Download pre-built binaries from [GitHub Releases](https://github.com/neuroquantumdb/neuroquantumdb/releases):

```bash
# Download for ARM64 (Raspberry Pi)
wget https://github.com/neuroquantumdb/neuroquantumdb/releases/latest/download/neuroquantum-api-arm64

# Make executable
chmod +x neuroquantum-api-arm64

# Run
./neuroquantum-api-arm64
```

---

## Raspberry Pi 4 - Specific Setup

### Optimize for ARM64

1. **Enable NEON SIMD (already enabled in build):**
   ```toml
   # Cargo.toml already configured for ARM64 optimization
   [profile.release]
   codegen-units = 1
   lto = true
   opt-level = 3
   ```

2. **Increase Swap (optional for 2GB models):**
   ```bash
   sudo dphys-swapfile swapoff
   sudo nano /etc/dphys-swapfile
   # Set: CONF_SWAPSIZE=2048
   sudo dphys-swapfile setup
   sudo dphys-swapfile swapon
   ```

3. **Disable Unnecessary Services:**
   ```bash
   # Free up RAM
   sudo systemctl disable bluetooth
   sudo systemctl disable cups
   ```

4. **Use SSD instead of SD Card (recommended):**
   - 10x faster I/O performance
   - Better durability for WAL writes

---

## Post-Installation Steps

### 1. Initialize Database

```bash
# First-time setup (creates admin API key)
neuroquantum-api init

# Save the generated API key securely!
# Output example:
# ✅ Admin API Key created: nq_live_abc123xyz789...
# ⚠️  Save this key! It won't be shown again.
```

### 2. Generate JWT Secret

```bash
# For production use
neuroquantum-api generate-jwt-secret --output config/jwt-secret.txt

# Update config/prod.toml with the secret
```

### 3. Configure Production Settings

Edit `config/prod.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080
worker_threads = 4  # CPU cores on Raspberry Pi 4

[storage]
data_dir = "/data/neuroquantum"
page_size = 8192
buffer_pool_size = 256  # Pages (2MB on Raspberry Pi)
wal_enabled = true

[auth]
jwt_secret = "YOUR-GENERATED-SECRET-HERE"
jwt_expiration_hours = 8
```

### 4. Start Server

```bash
# Run with production config
RUST_LOG=info neuroquantum-api --config config/prod.toml
```

### 5. Verify Installation

```bash
# Health check
curl http://localhost:8080/health

# Expected response:
# {"status":"healthy","version":"0.1.0","uptime_seconds":5}

# Metrics
curl http://localhost:8080/metrics
```

---

## Troubleshooting

### Build Fails on ARM64

**Problem:** Compilation errors related to cryptographic libraries

**Solution:**
```bash
# Install required system libraries
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake
```

### "Cannot allocate memory" Error

**Problem:** Insufficient RAM during build

**Solution:**
```bash
# Build with fewer parallel jobs
cargo build --release -j 2

# Or increase swap space (see above)
```

### Port 8080 Already in Use

**Problem:** Another service is using port 8080

**Solution:**
```bash
# Change port in config/prod.toml
[server]
port = 8081

# Or find and stop the conflicting service
sudo lsof -i :8080
```

---

## Next Steps

- ✅ [Quick Start Guide](./quick-start.md) - First queries with QSQL
- ✅ [Configuration Guide](./configuration.md) - Fine-tuning for your use case
- ✅ [Security Setup](./security-setup.md) - Production hardening

---

## Platform-Specific Notes

### macOS (Apple Silicon)

NeuroQuantumDB runs natively on M1/M2/M3 chips (ARM64):

```bash
# No special steps needed
cargo build --release
```

### Windows (WSL2)

Recommended to use WSL2 with Ubuntu 22.04:

```bash
# Inside WSL2
wsl --install -d Ubuntu-22.04
wsl
# Then follow Linux installation steps
```

### Linux (x86_64)

Standard installation works on all major distributions:
- Ubuntu 20.04+
- Debian 11+
- Fedora 36+
- Arch Linux

---

## Uninstallation

### Remove Source Build

```bash
cd neuroquantumdb
cargo clean
cd ..
rm -rf neuroquantumdb
```

### Remove Docker

```bash
docker stop neuroquantumdb
docker rm neuroquantumdb
docker rmi neuroquantumdb/neuroquantumdb:latest
docker volume rm neuroquantum-data neuroquantum-config
```

### Remove Data

```bash
# ⚠️ Warning: This deletes all database files!
rm -rf /data/neuroquantum
rm -rf ~/.neuroquantum
```

