# Public documentation

This directory is the canonical source for documents exposed to unauthenticated
users and external clients.

- [`self-hosting.md`](self-hosting.md) — single-host Docker or Podman deployment.
- [`deploy.md`](deploy.md) — supported deployment artifacts and production checklist.
- [`k3s.md`](k3s.md) — k3s image build and release workflow.
- [`llms.txt`](llms.txt) — compact agent discovery entrypoint, served at `/public/llms.txt`.
- [`skill.json`](skill.json) — generated machine-readable agent manifest, served at `/public/skill.json`.
- [`learning-contract.json`](learning-contract.json) — versioned learning stories, evidence rules, and fixture-simulation boundary, served at `/public/learning-contract.json`.
- [`learning-principles.md`](learning-principles.md) — learner-readable explanation of the learning contract, served at `/public/learning-principles.md`.

For a local, browser-visible three-round learner journey, run
`HARU_SIM_PASSWORD='choose-your-own-password' make local-haru-simulation`.
The command creates only disposable learner-origin data and prints the journey
URL plus a non-secret login email; it never stores or prints the password.

Build and runtime projections must consume these files; do not create parallel
copies under `api/`, `deploy/`, or `web/`. The Compose Caddy container and the
reference Kubernetes Caddy sidecar serve the three machine-readable documents
directly from `docs/public`; the API has no runtime handlers for these
documents.

Regenerate generated public documents with `make public-docs`.
