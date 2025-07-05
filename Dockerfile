# Build stage
FROM rust:1.75 as builder

# Install trunk for building the frontend
RUN cargo install trunk
RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

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
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 app

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/server ./ghostcrate

# Create data directory
RUN mkdir -p /data && chown app:app /data

# Switch to app user
USER app

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:/data/ghostcrate.db
ENV LEPTOS_SITE_ADDR=0.0.0.0:8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/ || exit 1

# Run the application
CMD ["./ghostcrate"]