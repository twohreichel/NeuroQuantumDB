# Building & Testing

## Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Development tools
cargo install cargo-audit cargo-deny cargo-machete
```

## Build Commands

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build specific crate
cargo build -p neuroquantum-core
```

## Testing

```bash
# All tests
cargo test --all

# Specific crate
cargo test -p neuroquantum-core

# With output
cargo test -- --nocapture

# Single test
cargo test test_dna_compression
```

## Code Quality

```bash
# Linting
cargo clippy --all-targets --all-features

# Format check
cargo fmt --all -- --check

# Format fix
cargo fmt --all

# Security audit
cargo audit

# Dependency check
cargo deny check
```

## Makefile Targets

```bash
make build       # Build release
make test        # Run all tests
make lint        # Run clippy
make lint-fix    # Fix lint issues
make docs        # Generate docs
make clean       # Clean build artifacts
```

## Benchmarks

```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench dna_compression
```

Located in `target/criterion/` after running.

## CI/CD

GitHub Actions workflow:

```yaml
jobs:
  test:
    - cargo fmt --check
    - cargo clippy
    - cargo test
    - cargo audit
```
