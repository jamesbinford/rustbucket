# Stage 1: Build the Rust application
FROM rust:latest AS builder

WORKDIR /app
COPY . .

# Build the project with Cargo in release mode
RUN cargo build --release

# Stage 2: Create a lightweight container with the binary
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
EXPOSE 25
EXPOSE 23
EXPOSE 21

# Copy the Rust executable from the builder stage
COPY --from=builder /app/target/release/rustbucket /usr/local/bin/rustbucket
COPY Config.toml.example ./Config.toml

# Set the entrypoint to the Rust executable
ENTRYPOINT ["/usr/local/bin/rustbucket"]
