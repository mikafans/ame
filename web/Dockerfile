# syntax=docker/dockerfile:1.7
# Multi-stage build for the ame web frontend (Next.js standalone output).
# Build context must be the repo root so api/openapi.yaml is reachable for the schema codegen step.
#
#   docker build -f web/Dockerfile -t ame-web:dev .

ARG BUN_VERSION=1.2
ARG NODE_VERSION=22

# ── deps ───────────────────────────────────────────────────────────────────
FROM oven/bun:${BUN_VERSION} AS deps
WORKDIR /build
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile

# ── builder ────────────────────────────────────────────────────────────────
FROM oven/bun:${BUN_VERSION} AS builder
WORKDIR /build
ENV NEXT_TELEMETRY_DISABLED=1

COPY --from=deps /build/node_modules ./node_modules
COPY web/ ./
COPY docs/public ./docs/public
# Generated OpenAPI types live in src/api/generated/schema.d.ts; if missing,
# rebuild from the spec that ships at the repo root.
COPY api/openapi.yaml /tmp/openapi.yaml
RUN if [ ! -f src/api/generated/schema.d.ts ]; then \
      bunx openapi-typescript /tmp/openapi.yaml -o src/api/generated/schema.d.ts; \
    fi
# Embedded reference client (src/generated/clientSource.ts) is committed; if
# missing, rebuild from agents/client.py at the repo root.
COPY agents/client.py /tmp/client.py
RUN if [ ! -f src/generated/clientSource.ts ]; then \
      bun scripts/gen-client-source.mjs /tmp/client.py; \
    fi

# NEXT_PUBLIC_API_URL is baked into the client bundle at build time.
ARG NEXT_PUBLIC_API_URL=http://localhost:28080
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}

RUN bun run build

# ── runtime ────────────────────────────────────────────────────────────────
FROM node:${NODE_VERSION}-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000
ENV HOSTNAME=0.0.0.0

# The node base image already ships a non-root `node` user at UID 1000.
# Next.js "standalone" output: a self-contained server.js plus minimal deps.
COPY --from=builder --chown=node:node /build/.next/standalone ./
COPY --from=builder --chown=node:node /build/.next/static ./.next/static
COPY --from=builder --chown=node:node /build/public ./public
COPY --from=builder --chown=node:node /build/docs/public ./docs/public

USER node
EXPOSE 3000
CMD ["node", "server.js"]
