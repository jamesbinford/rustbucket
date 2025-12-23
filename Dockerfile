# Stage 1: Build the Rust application
FROM rust:latest AS builder

# Create a non-root user for building
RUN useradd -m -u 1000 rustbuilder

WORKDIR /app

# Copy only dependency files first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Now copy the actual source code
COPY --chown=rustbuilder:rustbuilder . .

# Build the actual project
RUN cargo build --release && \
    strip target/release/rustbucket

# Stage 2: Create a lightweight runtime container
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Set working directory
WORKDIR /app

# Create logs directory
RUN mkdir -p /app/logs

# Copy the binary from builder stage
COPY --from=builder /app/target/release/rustbucket /usr/local/bin/rustbucket

# Copy config file
COPY Config.toml.example ./Config.toml

# Default ports (high ports for unprivileged container operation)
# Override with -e SSH_PORT=22 etc. when running with --privileged or as root
ENV SSH_PORT=2222
ENV FTP_PORT=2121
ENV SMTP_PORT=2525
ENV HTTP_PORT=8080

# Expose default high ports (map to any host port with -p)
EXPOSE 2222/tcp
EXPOSE 2121/tcp
EXPOSE 2525/tcp
EXPOSE 8080/tcp

# Create volume for logs (persists across container restarts)
VOLUME ["/app/logs"]

# Add healthcheck (checks if process is running)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD pgrep -x rustbucket || exit 1

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/rustbucket"]
