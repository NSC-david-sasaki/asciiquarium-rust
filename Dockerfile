# syntax=docker/dockerfile:1

#################################
# Stage 1: Build Rust binary
#################################
FROM rust:slim-trixie AS builder

# Optional argument to specify Rust target
ARG RUST_TARGET

# Install strip from binutils
RUN apt-get update && apt-get install -y --no-install-recommends binutils curl && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy manifest and assets first to cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY assets ./assets

# Copy source code
COPY src ./src

# Determine target if not provided and build release binary
RUN if [ -z "$RUST_TARGET" ]; then \
        case "$(uname -m)" in \
            x86_64) export RUST_TARGET=x86_64-unknown-linux-gnu ;; \
            aarch64) export RUST_TARGET=aarch64-unknown-linux-gnu ;; \
            *) echo "Unsupported architecture"; exit 1 ;; \
        esac; \
    fi && \
    echo "Building for target: $RUST_TARGET" && \
    cargo build --release --features web --target $RUST_TARGET && \
    strip target/$RUST_TARGET/release/web_server && \
    cp target/$RUST_TARGET/release/web_server target/release/web_server

#################################
# Stage 2: Minimal runtime image
#################################
FROM gcr.io/distroless/cc AS runtime

# Create non-root user
USER 1000:1000
WORKDIR /home/appuser

# Copy the binary
COPY --from=builder /app/target/release/web_server /usr/local/bin/web_server

EXPOSE 3000

# Healthcheck can use wget/curl from builder stage or skip in distroless
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
  CMD ["web_server", "--healthcheck"]

ENTRYPOINT ["/usr/local/bin/web_server"]