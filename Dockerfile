# Multi-stage Dockerfile for log_pipelines Rust application

# Stage 1: Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application (both binaries)
RUN cargo build --release

# Stage 2: Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (Redis client for health checks, SSL libs)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries from builder
COPY --from=builder /app/target/release/log_pipelines /app/log_pipelines
COPY --from=builder /app/target/release/drainer /app/drainer

# Copy example config (user can override with volume mount)
COPY config.toml.example /app/config.toml.example

# Create logs directory
RUN mkdir -p /app/logs

# Expose default port (can be overridden via config)
EXPOSE 3000

# Default to running the main API server
# To run drainer instead: docker run <image> /app/drainer
CMD ["/app/log_pipelines"]

