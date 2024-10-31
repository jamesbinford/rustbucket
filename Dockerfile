# Stage 1: Build the Rust application
FROM rust:latest AS builder

WORKDIR /app
COPY . .

# Build the project with Cargo in release mode
RUN cargo build --release

# Stage 2: Create a lightweight container with the binary
FROM debian:bookworm-slim
EXPOSE 25
EXPOSE 23
EXPOSE 21

# Copy the Rust executable from the builder stage
COPY --from=builder /app/target/release/rustbucket /usr/local/bin/rustbucket

# Set the entrypoint to the Rust executable
ENTRYPOINT ["/usr/local/bin/rustbucket"]
