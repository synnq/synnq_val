# Use the official Rust image as a build stage
FROM rust:1.72 as builder

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the entire project into the container
COPY . .

# Build the application in release mode
RUN cargo build --release

# Use a smaller base image for the runtime
FROM debian:buster-slim

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/your-app-name .

# Expose the application's port (change 8080 to the actual port)
EXPOSE 8080

# Run the binary
CMD ["./your-app-name"]