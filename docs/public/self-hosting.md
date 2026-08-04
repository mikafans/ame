# Self-hosting AME

This is the operator guide for running AME on one host with Docker or Podman Compose. The recommended deployment includes Caddy in the Compose stack, so the browser, API, and public machine-readable documents share one origin. The same learner API serves the web app and agents; `llms.txt` is the machine-facing entry guide for that shared contract.

## Production architecture

```text
Internet -> containerized Caddy :80/:443
                    ├-> web:3000
                    └-> api:8080 -> postgres:5432
                                  \-> valkey:6379
```

Use one public origin for the browser and API. The API image embeds the canonical documents and Python SDK. Caddy should use one `/public/*` handler for all public API, contract, and SDK paths, plus `/api/*` for authenticated routes. Keep `/healthz`, `/readyz`, and `/metrics` as root operational endpoints; all other paths go to Next.js.

## Requirements

- Docker Compose v2 or a working Podman machine with Compose support
- DNS pointing your hostname at the server
- Persistent backup storage for Postgres

The Compose Caddy container terminates TLS when `AME_HOSTNAME` is a real DNS name. A system-level Caddy or nginx is not required.

## Configure and start a production deployment

```bash
cp .env.example .env
```

Set real values in `.env`:

```dotenv
POSTGRES_PASSWORD=<long-random-password>
AME_CORS_ORIGINS=https://ame.example.com
NEXT_PUBLIC_API_URL=https://ame.example.com
AME_HOSTNAME=ame.example.com
```

`NEXT_PUBLIC_API_URL` is compiled into the browser bundle, so set it before building. Do not use internal service names (`api`, `postgres`, `valkey`) in the browser configuration.

```bash
docker compose -f docker-compose.prod.yml up --build -d
docker compose -f docker-compose.prod.yml ps
curl -fsS https://ame.example.com/healthz
curl -fsS https://ame.example.com/readyz
```

The API runs embedded SQLx migrations during startup. There is no separate migration container. Do not run `down -v` during normal upgrades.

For Podman, first verify the machine:

```bash
podman machine ls
podman info
```

If it is listed as running but `podman info` cannot connect, repair it before starting AME:

```bash
podman machine stop
podman machine start
```

The Compose stack publishes only Caddy. API and web remain private services on the Compose network, so the proxy cannot accidentally be bypassed. For a deployment that already has a separately managed proxy, the optional [`Caddyfile.example`](../../deploy/Caddyfile.example) remains available, but that system-level arrangement is not the recommended path.

## First administrator and seed data

Registration never grants the administrator role. Promote the first account in the database, then log in again so its token receives the new role:

```bash
make db-admin ADMIN_EMAIL=you@example.com ADMIN_PASSWORD='<initial-password>'
```

For demo data only:

```bash
make db-seed
```

Do not use repository demo passwords on an internet-facing instance.

## Agent discovery

After the public origin is live, AI clients can discover AME at:

```text
https://ame.example.com/public/llms.txt
https://ame.example.com/public/skill.json
https://ame.example.com/public/openapi.yaml
```

`llms.txt` is an agent discovery contract, not a self-hosting mechanism. These routes advertise the shared learner API. An agent can start onboarding with an email identifier and receive the learner bearer token; no separate integration identity is needed.

## Upgrade and backup checklist

1. Back up Postgres with `make db-backup BACKUP_FILE=...`.
2. Pull the new revision and verify the release's clean baseline before using it with a new empty database.
3. Rebuild with `docker compose -f docker-compose.prod.yml up --build -d`.
4. Check `/readyz`, logs, and the public discovery routes.
5. Keep the previous image available until smoke checks pass.

Never commit `.env`, database dumps, agent keys, or proxy certificates.

### Restore a portable learner journey

Journey manifests do not create accounts. Restore or configure the learner in your identity service first, sign in as that same owner, then submit the `ame.journey-history.v1` artifact to `POST /api/v1/learning/imports`. A foreign `ownerId` is rejected rather than merged. Keep the import receipt and original manifest; use database backups for whole-instance disaster recovery, not as a substitute for the portable artifact.
