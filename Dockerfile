# Multi-stage build for NeuroQuantumDB
# Target: ARM64 (Raspberry Pi 4)
# Size target: < 15MB

# Build argument for target platform
ARG TARGETPLATFORM=linux/arm64

# Stage 1: Rust builder
FROM rust:latest AS rust-builder

# Configure multiarch support and ARM64 repositories
RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
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

# Add Rust target first (cacheable layer)
RUN rustup target add aarch64-unknown-linux-gnu

# Copy only dependency files first for better caching
COPY Cargo.toml ./Cargo.toml.full
COPY Cargo.lock ./Cargo.lock
COPY crates/neuroquantum-core/Cargo.toml crates/neuroquantum-core/Cargo.toml
COPY crates/neuroquantum-qsql/Cargo.toml crates/neuroquantum-qsql/Cargo.toml
COPY crates/neuroquantum-api/Cargo.toml crates/neuroquantum-api/Cargo.toml
COPY neuroquantum_data /neuroquantum_data

# Create production Cargo.toml without tests workspace member
RUN sed '/^[[:space:]]*"tests"/d' Cargo.toml.full > Cargo.toml

# Create dummy source files to cache dependencies
RUN mkdir -p crates/neuroquantum-core/src && \
    mkdir -p crates/neuroquantum-qsql/src && \
    mkdir -p crates/neuroquantum-api/src && \
    echo "fn main() {}" > crates/neuroquantum-api/src/main.rs && \
    echo "pub fn dummy() {}" > crates/neuroquantum-core/src/lib.rs && \
    echo "pub fn dummy() {}" > crates/neuroquantum-qsql/src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release --target aarch64-unknown-linux-gnu \
    --features neon-optimizations,neuromorphic,quantum,natural-language \
    --bin neuroquantum-api || true

# Now copy actual source code
COPY crates/ crates/

# Build final binary with all optimizations
RUN cargo build --release --target aarch64-unknown-linux-gnu \
    --features neon-optimizations,neuromorphic,quantum,natural-language \
    --bin neuroquantum-api && \
    aarch64-linux-gnu-strip target/aarch64-unknown-linux-gnu/release/neuroquantum-api

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
