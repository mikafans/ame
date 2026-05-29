# syntax=docker/dockerfile:1.7
# Multi-stage build for the ame API.
# Build context must be the repo root so `db/migrations` is reachable for the migrate stage.
#
#   docker build -f api/Dockerfile -t ame-api:dev .

ARG RUST_VERSION=1.95
ARG DEBIAN_VERSION=bookworm

# ── builder ────────────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS builder
WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Cache deps separately from sources for faster rebuilds.
COPY api/Cargo.toml api/Cargo.lock ./api/
RUN mkdir -p api/src api/src/bin api/benches \
  && echo "fn main() {}" > api/src/main.rs \
  && echo "" > api/src/lib.rs \
  && echo "fn main() {}" > api/src/bin/gen_openapi.rs \
  && echo "fn main() {}" > api/benches/hot_paths.rs \
  && cd api && cargo build --release --bin ame-api \
  && rm -rf src

COPY api/src ./api/src
# llms.txt is embedded into the binary via include_str! (served at GET /llms.txt),
# so it must be present at compile time.
COPY api/llms.txt ./api/llms.txt
# Migrations are embedded into the binary via sqlx::migrate!, so they must be
# present at compile time (resolved as ../db/migrations relative to the crate).
COPY db/migrations ./db/migrations
# Touch sources so cargo rebuilds them (the dummy lib.rs above is now stale).
RUN touch api/src/main.rs api/src/lib.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cd api && cargo build --release --bin ame-api

# ── runtime ────────────────────────────────────────────────────────────────
FROM debian:${DEBIAN_VERSION}-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --create-home --uid 1000 ame

COPY --from=builder /build/api/target/release/ame-api /usr/local/bin/ame-api
# Migrations are embedded in the binary (sqlx::migrate!) and run on boot.

USER ame
EXPOSE 8080
ENV RUST_LOG=ame_api=info,tower_http=info,sqlx=warn

ENTRYPOINT ["/usr/local/bin/ame-api"]
