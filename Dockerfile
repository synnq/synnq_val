# Use the official Rust image as a base with the latest version
FROM rust:1.72 as builder

# Install the required dependencies for RocksDB
RUN apt-get update && apt-get install -y \
    libclang-dev \
    clang \
    llvm-dev \
    librocksdb-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new directory for the project
WORKDIR /usr/src/app

# Copy the current directory contents into the container
COPY . .

# Build the application in release mode
RUN cargo build --release && ls -l /usr/src/app/target/release/

# Use a minimal base image to run the application
FROM debian:buster-slim

# Install RocksDB runtime dependencies
RUN apt-get update && apt-get install -y librocksdb-dev && rm -rf /var/lib/apt/lists/*

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/app/target/release/synnq_val /usr/local/bin/synnq_val

# Expose the application port
EXPOSE 8000

# Set the default command to run the application
CMD ["/usr/local/bin/synnq_val"]