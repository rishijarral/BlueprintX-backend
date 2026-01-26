# Build stage
FROM rust:1.88-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a dummy project to cache dependencies
RUN cargo new --bin blueprintx-backend
WORKDIR /app/blueprintx-backend

# Copy manifests (Cargo.lock is optional)
COPY Cargo.toml ./
COPY Cargo.lock* ./

# Build dependencies only (this layer will be cached)
RUN cargo build --release

# Copy actual source code (this invalidates the cache)
COPY src ./src

# Touch source to ensure rebuild, then build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/blueprintx-backend/target/release/blueprintx-backend /app/blueprintx-backend

# Change ownership
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Run the application
CMD ["/app/blueprintx-backend"]
