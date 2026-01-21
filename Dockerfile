# Multi-stage Dockerfile that uses pre-built binaries
FROM alpine:latest

# Install CA certificates for HTTPS and create non-root user
RUN apk --no-cache add ca-certificates tzdata && \
    addgroup -g 1000 p1exporter && \
    adduser -D -u 1000 -G p1exporter p1exporter

# Set working directory
WORKDIR /app

# Copy the pre-built binary for the target architecture
# The build system will place the correct binary here based on the platform
ARG TARGETARCH
COPY binaries/${TARGETARCH}/p1-prometheus-exporter-rs /app/p1-prometheus-exporter-rs

# Ensure binary is executable
RUN chmod +x /app/p1-prometheus-exporter-rs

# Switch to non-root user
USER p1exporter

# Expose the metrics port
EXPOSE 9292

# Set default environment variables
ENV RUST_LOG=info

# Run the exporter
ENTRYPOINT ["/app/p1-prometheus-exporter-rs"]
CMD ["--serial-port", "/dev/ttyUSB0", "--port", "9292"]