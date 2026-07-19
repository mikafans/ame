# Self-hosting AME

This is the operator guide for running AME on one Linux host with Docker or
Podman Compose and a TLS reverse proxy. It is separate from the agent guide:
`llms.txt` describes the public HTTP agent interface for AI clients; it is not a
deployment manifest or an installation guide.

## Architecture

```text
Internet -> Caddy/nginx -> web:3000
                       \-> api:8080 -> postgres:5432
                                     \-> valkey:6379
```

Use one public origin for the browser and API. Caddy sends `/v1/*`, `/healthz`,
and `/readyz` to the API; all other paths go to Next.js. The canonical public
documents (`/llms.txt`, `/skill.json`, and `/openapi.yaml`) are served by the
frontend from `docs/public`. The API also exposes compatibility routes for
clients connecting directly to its port.

## Requirements

- Docker Compose v2 or a working Podman machine with Compose support
- DNS pointing your hostname at the server
- Caddy, nginx, or another TLS-capable reverse proxy
- Persistent backup storage for Postgres

## Configure and start

```bash
cp .env.example .env
```

Set real values in `.env`:

```dotenv
POSTGRES_PASSWORD=<long-random-password>
AME_CORS_ORIGINS=https://ame.example.com
NEXT_PUBLIC_API_URL=https://ame.example.com
```

`NEXT_PUBLIC_API_URL` is compiled into the browser bundle, so set it before
building. Do not use internal service names (`api`, `postgres`, `valkey`) in the
browser configuration.

```bash
docker compose -f docker-compose.prod.yml up --build -d
docker compose -f docker-compose.prod.yml ps
curl -fsS http://127.0.0.1:8080/healthz
curl -fsS http://127.0.0.1:8080/readyz
```

The API runs embedded SQLx migrations during startup. There is no separate
migration container. Do not run `down -v` during normal upgrades.

For Podman, first verify the machine:

```bash
podman machine ls
podman info
```

If it is listed as running but `podman info` cannot connect, repair it before
starting AME:

```bash
podman machine stop
podman machine start
```

## Reverse proxy

The repository includes [`Caddyfile.example`](../../deploy/Caddyfile.example). Replace the
hostname and load it into Caddy:

```bash
caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

Compose binds API and web ports to loopback by default. Keep the proxy and
containers on the same host, or change the bind address deliberately and add a
firewall rule.

## First administrator and seed data

Registration never grants the administrator role. Promote the first account in
the database, then log in again so its token receives the new role:

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
https://ame.example.com/llms.txt
https://ame.example.com/skill.json
https://ame.example.com/openapi.yaml
```

`llms.txt` is an agent discovery contract, not a self-hosting mechanism. These
routes advertise the agent surface but do not grant access. An owner must create
an agent key through the authenticated application flow; keys are bearer
credentials and must be stored as secrets.

## Upgrade and backup checklist

1. Back up Postgres with `make db-backup BACKUP_FILE=...`.
2. Pull the new revision and review migration changes.
3. Rebuild with `docker compose -f docker-compose.prod.yml up --build -d`.
4. Check `/readyz`, logs, and the public discovery routes.
5. Keep the previous image available until smoke checks pass.

Never commit `.env`, database dumps, agent keys, or proxy certificates.
