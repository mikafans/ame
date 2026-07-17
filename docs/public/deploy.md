# Deploying ame

This directory documents the supported deploy shapes. The repo ships container
artifacts but **not** opinionated infra-as-code — bring your own k8s manifests,
Terraform, Helm chart, etc.

For a complete single-host installation, see [`../docs/public/self-hosting.md`](../docs/public/self-hosting.md).

## Artifacts

- `api/Dockerfile` — multi-stage Rust build, ~50 MB Debian-slim runtime.
- `web/Dockerfile` — multi-stage Next.js standalone build, runs on Node 22.
- `docker-compose.prod.yml` (repo root) — Postgres + Valkey + API + web with
  healthchecks and embedded API migrations.

## Smoke deploy (single host)

```bash
cp .env.example .env
# Set at minimum: POSTGRES_PASSWORD, NEXT_PUBLIC_API_URL
docker compose -f docker-compose.prod.yml up --build -d
docker compose -f docker-compose.prod.yml logs -f api
```

Or via the Makefile shortcuts:

```bash
make docker-build   # build both images
make docker-up      # bring the stack up (-d)
make docker-down
```

The compose stack runs the sqlx migration job once, waits for healthy postgres,
then starts the API.

## Production checklist

Before pointing real users at a deployment:

- [ ] `POSTGRES_PASSWORD` is rotated from the example value.
- [ ] Terminate TLS at a reverse proxy (Caddy, nginx, Envoy, cloudflared). The
      API speaks plain HTTP.
- [ ] `AME_CORS_ORIGINS` is set to your frontend origin(s), comma-separated. It
      defaults to `http://localhost:3000`; setting it to `*` re-enables permissive
      CORS (credentials are then disallowed per the CORS spec).
- [ ] Add a rate limiter in front of `/v1/auth/login`, `/v1/auth/register`, and
      `/v1/agents/register` (the latter is unauthenticated by design).
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

## CI image publishing

Not wired yet. When it lands, images will be pushed to `ghcr.io/haru/ame-api`
and `ghcr.io/haru/ame-web` on tagged releases.
