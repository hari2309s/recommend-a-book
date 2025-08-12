# Multi-stage build for Rust API
FROM rust:1.75 as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    cmake \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY config ./config

# Build the application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/recommend-a-book-api ./
COPY --from=builder /app/config ./config

# Create necessary directories
RUN mkdir -p /tmp/huggingface_cache /app/data && \
    chown -R appuser:appuser /app /tmp/huggingface_cache

# Switch to non-root user
USER appuser

# Set environment variables
ENV RUST_LOG=info
ENV HUGGINGFACE_HUB_CACHE=/tmp/huggingface_cache
ENV RUN_MODE=production

# Expose port
EXPOSE 10000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:10000/api/health || exit 1

# Run the application
CMD ["./recommend-a-book-api"]
