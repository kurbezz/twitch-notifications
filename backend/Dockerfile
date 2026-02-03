# Build stage
FROM rust:latest-bookworm as builder

WORKDIR /app

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy the actual source code
COPY src ./src
COPY migrations ./migrations

# Touch main.rs to ensure it gets rebuilt
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 appuser

# Create data directory
RUN mkdir -p /data && chown appuser:appuser /data

# Copy the binary from builder
COPY --from=builder /app/target/release/twitch-notifications /usr/local/bin/twitch-notifications

# Copy migrations for runtime
COPY --from=builder /app/migrations /app/migrations

# Switch to non-root user
USER appuser

# Set environment variables
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8080
ENV DATABASE_URL=sqlite:///data/app.db

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the binary
CMD ["twitch-notifications"]