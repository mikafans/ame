# Public documentation

This directory is the canonical source for documents exposed to unauthenticated
users and external clients.

- [`self-hosting.md`](self-hosting.md) — single-host Docker or Podman deployment.
- [`deploy.md`](deploy.md) — supported deployment artifacts and production checklist.
- [`k3s.md`](k3s.md) — k3s image build and release workflow.
- [`llms.txt`](llms.txt) — agent discovery and API usage contract, served at `/public/llms.txt`.
- [`skill.json`](skill.json) — generated machine-readable agent manifest, served at `/public/skill.json`.

Build and runtime projections must consume these files; do not create parallel
copies under `api/`, `deploy/`, or `web/`. Containerized Caddy serves the three
machine-readable documents directly from `docs/public`; the API has no runtime
handlers for these documents.

Regenerate generated public documents with `make public-docs`.
