# syntax=docker/dockerfile:1

# ---------------------------------------------------------------- frontend
FROM node:22-alpine AS frontend
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

# ----------------------------------------------------------------- backend
FROM rust:1-slim-bookworm AS backend
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./
# Build the dependency graph once so the layer is cached across source changes.
RUN mkdir -p src && printf 'fn main() {}\n' > src/main.rs && printf '' > src/lib.rs \
    && cargo build --release --locked \
    && rm -rf src
COPY backend/src ./src
COPY backend/data ./data
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

# ----------------------------------------------------------------- runtime
FROM debian:bookworm-slim AS runtime
LABEL org.opencontainers.image.title="Electrical Load List" \
      org.opencontainers.image.description="Electrical load list calculation service with a Vue interface" \
      org.opencontainers.image.licenses="MIT"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /app app

WORKDIR /app
COPY --from=backend /app/target/release/electricacl /usr/local/bin/electricacl
COPY --from=frontend /app/dist /app/static

RUN mkdir -p /app/data && chown -R app:app /app
USER app

ENV PORT=8080 \
    DATA_DIR=/app/data \
    STATIC_DIR=/app/static \
    RUST_LOG=electricacl=info,tower_http=warn

VOLUME ["/app/data"]
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/electricacl", "--healthcheck"]

ENTRYPOINT ["/usr/local/bin/electricacl"]
