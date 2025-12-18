# Installation

## Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **OS** | Linux, macOS | Ubuntu 22.04+ |
| **RAM** | 2 GB | 4 GB |
| **Disk** | 1 GB | 10 GB |
| **Rust** | 1.75+ | Latest stable |

## From Source

```bash
# Clone
git clone https://github.com/neuroquantumdb/neuroquantumdb.git
cd neuroquantumdb

# Build release
cargo build --release

# Run tests
cargo test --all
```

## Docker

```bash
# Pull image
docker pull neuroquantumdb/neuroquantumdb:latest

# Run container
docker run -d \
  -p 8080:8080 \
  -v nqdb-data:/data \
  --name neuroquantumdb \
  neuroquantumdb/neuroquantumdb:latest
```

## Verify Installation

```bash
# Check version
./target/release/neuroquantum-api --version

# Health check
curl http://localhost:8080/health
```

Expected response:
```json
{"status": "healthy"}
```

## Next Steps

→ [Configuration](configuration.md)
