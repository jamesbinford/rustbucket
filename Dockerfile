# Use the official Rust image as a base
FROM rust:latest

# Set the working directory
WORKDIR /usr/src/app

# Copy the current directory contents into the container
COPY . .

# Build the Rust application
RUN cargo build --release

# Expose the necessary ports
EXPOSE 25 23 21

# Run the binary
CMD ["/home/james/builds/release/rustbucket"]