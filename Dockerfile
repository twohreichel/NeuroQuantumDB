# Multi-stage build for NeuroQuantumDB
# Target: ARM64 (Raspberry Pi 4)
# Size target: < 15MB

# Build argument for target platform
ARG TARGETPLATFORM=linux/arm64

# Stage 1: Rust builder
FROM rust:latest AS rust-builder

# Install build dependencies including cross-compilation tools
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    libssl-dev:arm64 \
    && rm -rf /var/lib/apt/lists/*

# Set up comprehensive cross-compilation environment
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
ENV CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
ENV AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
ENV STRIP_aarch64_unknown_linux_gnu=aarch64-linux-gnu-strip
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH_aarch64_unknown_linux_gnu=/usr/lib/aarch64-linux-gnu/pkgconfig
ENV RUSTFLAGS="-C target-feature=+neon"

# Additional environment variables for C dependencies like zstd-sys
ENV CC=aarch64-linux-gnu-gcc
ENV CXX=aarch64-linux-gnu-g++
ENV AR=aarch64-linux-gnu-ar
ENV STRIP=aarch64-linux-gnu-strip

WORKDIR /app

# Copy workspace configuration and create production version without tests
COPY Cargo.toml ./Cargo.toml.full
COPY Cargo.lock ./Cargo.lock
COPY crates/ crates/

# Create production Cargo.toml without tests workspace member
RUN sed '/^[[:space:]]*"tests"/d' Cargo.toml.full > Cargo.toml

# Add Rust target and build with proper environment
RUN rustup target add aarch64-unknown-linux-gnu

# Build with explicit target and feature flags
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
