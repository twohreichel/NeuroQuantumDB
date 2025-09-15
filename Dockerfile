# Multi-stage build for NeuroQuantumDB
# Target: ARM64 (Raspberry Pi 4)
# Size target: < 15MB

# Build argument for target platform
ARG TARGETPLATFORM=linux/arm64

# Stage 1: Rust builder
FROM rust:latest AS rust-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    gcc-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    && rm -rf /var/lib/apt/lists/*

# Set up cross-compilation environment
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
ENV CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml ./
COPY crates/ crates/

# Generate a compatible lock file and build with correct features
RUN rustup target add aarch64-unknown-linux-gnu
RUN cargo generate-lockfile
RUN cargo build --release --target aarch64-unknown-linux-gnu \
    --features neon-optimizations,neuromorphic,quantum,natural-language \
    --bin neuroquantum-api

# Stage 2: Production runtime (ultra-minimal)
FROM gcr.io/distroless/cc-debian12:latest

# Create non-root user for security
USER nonroot:nonroot

# Set production environment
ENV RUST_LOG=info
ENV NEUROQUANTUM_ENV=production
ENV NEUROQUANTUM_CONFIG=/etc/neuroquantumdb/config.toml

# Copy optimized binary
COPY --from=rust-builder --chown=nonroot:nonroot \
    /app/target/aarch64-unknown-linux-gnu/release/neuroquantum-api \
    /usr/local/bin/neuroquantumdb

# Copy production configuration
COPY --chown=nonroot:nonroot config/prod.toml /etc/neuroquantumdb/config.toml

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/neuroquantumdb", "health-check"]

# Expose ports
EXPOSE 8080 9090

# Resource limits for Raspberry Pi 4
LABEL com.neuroquantumdb.memory-limit="100MB"
LABEL com.neuroquantumdb.power-limit="2W"
LABEL com.neuroquantumdb.target-platform="arm64"

# Security labels
LABEL com.neuroquantumdb.security="quantum-resistant"
LABEL com.neuroquantumdb.encryption="kyber-dilithium"

# Production entrypoint
ENTRYPOINT ["/usr/local/bin/neuroquantumdb"]
CMD ["--config", "/etc/neuroquantumdb/config.toml"]
