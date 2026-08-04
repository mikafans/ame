# Public documentation

This directory is the canonical source for documents exposed to unauthenticated users and external clients.

- [`self-hosting.md`](self-hosting.md) — single-host Docker or Podman deployment, the only supported deploy shape. The repo ships container artifacts but no opinionated infra-as-code (k8s manifests, Terraform, Helm) — bring your own if you need one.
- [`llms.txt`](llms.txt) — compact agent discovery entrypoint, served at `/public/llms.txt`.
- [`skill.json`](skill.json) — generated machine-readable agent manifest, served at `/public/skill.json`.
- [`learning-contract.json`](learning-contract.json) — versioned learning stories, evidence rules, and fixture-simulation boundary, served at `/public/learning-contract.json`.
- [`../sdk/python/ame.py`](../sdk/python/ame.py) — reusable Python SDK, served at `/public/sdk/python/ame.py`.
- [`learning-principles.md`](learning-principles.md) — learner-readable explanation of the learning contract, served at `/public/learning-principles.md`.

For a local, browser-visible three-round learner journey, run `HARU_SIM_PASSWORD='choose-your-own-password' make local-haru-simulation`. The command creates only disposable learner-origin data and prints the journey URL plus a non-secret login email; it never stores or prints the password.

Build and runtime projections must consume these files; do not create parallel copies under `api/`, `deploy/`, or `web/`. The API image embeds these documents and serves them at runtime. A front proxy should use one `/public/*` rule to route both the documents and SDK to the API, with `/api/` and `/public/v1/` also routed to the API.

Regenerate generated public documents with `make public-docs`.
