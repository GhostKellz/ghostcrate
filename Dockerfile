# Build stage
FROM rust:1.81 as builder

# Install trunk for building the frontend
RUN cargo install trunk
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY style ./style

# Build the application
RUN cargo build --release --bin server

# Runtime stage
FROM debian:bookworm-slim

# Install required packages and update all packages to latest versions
RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y \
    ca-certificates \
    curl \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 app

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/server ./ghostcrate

# Create data directory and set permissions
RUN mkdir -p /data /app/static && \
    chown -R app:app /data /app

# Switch to app user
USER app

# Expose port
EXPOSE 8080

# Set environment variables for v0.2.0
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:/data/ghostcrate.db
ENV GHOSTCRATE_HOST=0.0.0.0
ENV GHOSTCRATE_PORT=8080
ENV GHOSTCRATE_STORAGE_BACKEND=local
ENV GHOSTCRATE_STORAGE_LOCAL_PATH=/data
ENV GHOSTCRATE_AUTH_JWT_SECRET=your-secret-key-change-in-production
ENV GHOSTCRATE_AUTH_BCRYPT_COST=12
ENV LEPTOS_SITE_ADDR=0.0.0.0:8080

# Health check using the new health endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["./ghostcrate"]