# Multi-stage build Dockerfile
FROM rust:1.70-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/kaseeder

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create virtual target to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src

# Build application
RUN cargo build --release

# Runtime image
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r kaseeder && useradd -r -g kaseeder kaseeder

# Create necessary directories
RUN mkdir -p /app/data /app/logs && \
    chown -R kaseeder:kaseeder /app

# Copy binary file
COPY --from=builder /usr/src/kaseeder/target/release/kaseeder /usr/local/bin/

# Set permissions
RUN chmod +x /usr/local/bin/kaseeder

# Switch to non-root user
USER kaseeder

# Set working directory
WORKDIR /app

# Expose ports
EXPOSE 5354 3737

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3737/health || exit 1

# Default command
ENTRYPOINT ["kaseeder"]

# Default arguments
CMD ["--help"]
