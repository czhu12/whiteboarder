# Start from the official Rust image
FROM rust:latest as builder

# Create a new empty shell project
RUN USER=root cargo new --bin whiteboarder
WORKDIR /whiteboarder

# Copy over your manifests
COPY whiteboarder/Cargo.toml whiteboarder/Cargo.lock ./

# This build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# Copy over your source tree
COPY whiteboarder/src ./src

# Build your project
RUN touch src/main.rs
RUN cargo build --release

# Use the same Rust image for the runtime to ensure compatibility
FROM rust:latest

# Install required dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the build stage
COPY --from=builder /whiteboarder/target/release/whiteboarder /usr/local/bin/whiteboarder

# Set the startup command to run your binary
CMD ["whiteboarder"]

# Expose the port that the application runs on
EXPOSE 3000

