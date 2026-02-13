# Synapse Rust - Production Dockerfile
# Optimized for ARM64 architecture

FROM gcr.io/distroless/cc-debian12:latest

WORKDIR /app

# Copy pre-built binary
COPY bin/synapse-rust /usr/local/bin/synapse-rust

# Expose Matrix ports
EXPOSE 8008 8448

# Environment variables
ENV RUST_LOG=info \
    RUST_BACKTRACE=1 \
    SYNAPSE_CONFIG_PATH=/app/config/homeserver.yaml

# Default command
CMD ["/usr/local/bin/synapse-rust"]

# Labels
LABEL org.opencontainers.image.title="Synapse Rust" \
      org.opencontainers.image.description="Matrix homeserver implementation in Rust" \
      org.opencontainers.image.version="2.0" \
      org.opencontainers.image.architecture="arm64"
