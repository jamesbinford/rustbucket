# Start with an official Rust image to build the app
FROM rust:1.72 AS builder

# Set the working directory
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Pre-fetch dependencies
RUN cargo fetch

# Copy the entire source code
COPY . .

# Build the application in release mode
RUN cargo build --release

# Use a minimal base image for the final stage
FROM debian:buster-slim

# Copy the compiled binary from the builder stage
COPY --from=builder /home/james/builds/release/rustbucket /usr/local/bin/your-executable-name

# Specify the command to run the executable
CMD ["rustbucket"]