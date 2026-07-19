# Deploying ame

This directory documents the supported deploy shapes. The repo ships container
artifacts but **not** opinionated infra-as-code — bring your own k8s manifests,
Terraform, Helm chart, etc.

For a complete single-host installation, see [`self-hosting.md`](self-hosting.md).

## Artifacts

- `api/Dockerfile` — multi-stage Rust build, ~50 MB Debian-slim runtime.
- `web/Dockerfile` — multi-stage Next.js standalone build, runs on Node 22.
- `docker-compose.prod.yml` (repo root) — Postgres + Valkey + API + web +
  containerized Caddy with healthchecks and embedded API migrations.

## Smoke deploy (single host)

```bash
cp .env.example .env
# Set at minimum: POSTGRES_PASSWORD, AME_HOSTNAME, NEXT_PUBLIC_API_URL,
# AME_CORS_ORIGINS
docker compose -f docker-compose.prod.yml up --build -d
docker compose -f docker-compose.prod.yml logs -f caddy
```

Or via the Makefile shortcuts:

```bash
make docker-build   # build both images
make docker-up      # bring the stack up (-d)
make docker-down
```

The API bootstraps the current clean baseline on an empty Postgres database,
waits for healthy Postgres and Valkey, then serves the web application and API.
This rework does not provide an upgrade path for the retired schema; preserve
old data separately and start the new deployment with a fresh database.

## Production checklist

Before pointing real users at a deployment:

- [ ] `POSTGRES_PASSWORD` is rotated from the example value.
- [ ] Set `AME_HOSTNAME` to the public DNS name so the Caddy container can
      terminate TLS. The API speaks plain HTTP inside the Compose network.
- [ ] `AME_CORS_ORIGINS` is set to your frontend origin(s), comma-separated. It
      defaults to `http://localhost:3000`; setting it to `*` re-enables permissive
      CORS (credentials are then disallowed per the CORS spec).
- [ ] Confirm the API's configured rate limits for `/public/v1/auth/login`,
      `/public/v1/auth/register`, and `/public/v1/onboarding/start`.
- [ ] Configure log shipping — the API logs structured tracing to stdout.
- [ ] Back up `pgdata` volume on a schedule.
- [ ] Set `NEXT_PUBLIC_API_URL` to the **public** URL the browser will hit,
      not the in-cluster service name (it's baked into the client bundle at
      build time).

See [`../SECURITY.md`](../SECURITY.md) for the current threat model.

## Kubernetes

The [`k3s/`](k3s/) and [`k8s/`](k8s/) directories contain the reference image
build and Kustomize manifests. They are intentionally environment-specific;
review the overlays and secrets before applying them to another cluster.

The reference Kubernetes web pod includes a small Caddy sidecar for the
machine-readable public documents. It copies the same `docs/public` content
packaged in the web image into a shared volume and serves:

```text
GET /public/llms.txt
GET /public/skill.json
GET /public/openapi.yaml
```

The `/public/v1/*` prefix remains API-backed and is routed to `ame-api`.
There is no API or Next.js runtime handler for the static documents.

## CI image publishing

Not wired yet. When it lands, images will be pushed to `ghcr.io/haru/ame-api`
and `ghcr.io/haru/ame-web` on tagged releases.
