# Multi-stage build for NeuroQuantumDB
# Target: ARM64 (Raspberry Pi 4)
# Size target: < 15MB

# Stage 1: Rust builder
FROM --platform=linux/arm64 rust:1.70-slim as rust-builder

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
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build optimized binary for ARM64
RUN rustup target add aarch64-unknown-linux-gnu
RUN cargo build --release --target aarch64-unknown-linux-gnu \
    --features "neon-optimizations,quantum-simd,dna-compression"

# Stage 2: Zig builder (for performance modules)
FROM --platform=linux/arm64 alpine:3.18 as zig-builder

# Install Zig compiler
RUN apk add --no-cache wget tar xz
RUN wget https://ziglang.org/download/0.11.0/zig-linux-aarch64-0.11.0.tar.xz
RUN tar -xf zig-linux-aarch64-0.11.0.tar.xz
RUN mv zig-linux-aarch64-0.11.0 /opt/zig

WORKDIR /app
COPY zig/ zig/ 2>/dev/null || true

# Build Zig performance modules (if they exist)
RUN if [ -d "zig" ]; then \
    /opt/zig/zig build-lib zig/neon-simd/simd_ops.zig \
        -target aarch64-linux -O ReleaseFast -dynamic || true; \
    /opt/zig/zig build-lib zig/quantum-kernels/grover.zig \
        -target aarch64-linux -O ReleaseFast -dynamic || true; \
    fi

# Stage 3: Final runtime image
FROM --platform=linux/arm64 debian:bullseye-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get autoremove -y \
    && apt-get clean

# Create non-root user for security
RUN groupadd -r neuroquantum && useradd -r -g neuroquantum neuroquantum

# Create application directory
WORKDIR /app

# Copy binary from Rust builder
COPY --from=rust-builder /app/target/aarch64-unknown-linux-gnu/release/neuroquantumdb /app/

# Copy Zig libraries if they exist
COPY --from=zig-builder /app/*.so /app/lib/ 2>/dev/null || true

# Create necessary directories
RUN mkdir -p /app/config /app/data /app/logs

# Copy configuration files
COPY config/ /app/config/ 2>/dev/null || true

# Set ownership and permissions
RUN chown -R neuroquantum:neuroquantum /app
RUN chmod +x /app/neuroquantumdb

# Switch to non-root user
USER neuroquantum

# Expose ports
EXPOSE 8080 9090

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/app/neuroquantumdb", "health-check"]

# Set environment variables for Raspberry Pi 4 optimization
ENV NEUROQUANTUM_MAX_MEMORY=100MB
ENV NEUROQUANTUM_MAX_POWER=2W
ENV NEUROQUANTUM_CPU_THREADS=4
ENV NEUROQUANTUM_NEON_OPTIMIZATIONS=true
ENV RUST_LOG=info

# Entry point
ENTRYPOINT ["/app/neuroquantumdb"]
CMD ["--config", "/app/config/prod.toml"]

# Container metadata
LABEL maintainer="NeuroQuantumDB Team <team@neuroquantumdb.org>"
LABEL version="1.0.0"
LABEL description="Ultra-efficient neuromorphic database for edge computing"
LABEL architecture="arm64"
LABEL target="raspberry-pi-4"
LABEL org.opencontainers.image.source="https://github.com/neuroquantumdb/neuroquantumdb"
