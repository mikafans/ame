# syntax=docker/dockerfile:1.7
# Multi-stage build for the ame API.
# Build context must be the repo root so `db/migrations` is reachable for the migrate stage.
#
#   docker build -f api/Dockerfile -t ame-api:dev .

ARG RUST_VERSION=1.97.1
ARG DEBIAN_VERSION=bookworm

# ── builder ────────────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS builder
WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# api depends on path crates under crates/, so the workspace root (not just
# api/) must be present for cargo to resolve.
COPY Cargo.toml Cargo.lock ./
COPY api/Cargo.toml ./api/
COPY api/src ./api/src
COPY crates ./crates
# Migrations are embedded into the binary via sqlx::migrate!, so they must be
# present at compile time (resolved as ../db/migrations relative to the crate).
COPY db/migrations ./db/migrations
# Public agent contracts are embedded into the API so the runtime image serves
# the exact documents it was built and tested with.
COPY docs/public ./docs/public
COPY sdk/python ./sdk/python
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release -p ame-api --bin ame-api

# ── runtime ────────────────────────────────────────────────────────────────
FROM debian:${DEBIAN_VERSION}-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --create-home --uid 1000 ame

COPY --from=builder /build/target/release/ame-api /usr/local/bin/ame-api
# Migrations are embedded in the binary (sqlx::migrate!) and run on boot.

USER ame
EXPOSE 8080
ENV RUST_LOG=ame_api=info,tower_http=info,sqlx=warn

ENTRYPOINT ["/usr/local/bin/ame-api"]
