# ame

Quiz and exam platform. Rust (Axum) backend, Next.js (App Router) frontend, Postgres.

**Stack**: Rust 2024 · Axum 0.8 · sqlx · Next.js 16 · React 19 · Bun · TypeScript · MUI 7 · Postgres 18

**License**: [GPL-3.0](./LICENSE)

## Quick start

```bash
make init-env       # mise install + sqlx-cli + web deps + Playwright
make db-up          # start Postgres in Docker or Podman
make db-migrate     # apply pending migrations
make dev            # API on :28080, frontend on :23000
make db-seed        # seed demo users, quizzes, questions (requires API running)
```

Demo credentials after seeding: `learner@example.com / password123`, `instructor@example.com / password123`, `admin@example.com / password123` (admin).

### Admin users

Registration only ever grants the `user` role — there is no API path to self-register
as an admin (`POST /v1/auth/register` rejects `role: admin`). Admins are granted
**directly in the database**:

```bash
make db-admin                                  # create/grant admin@example.com (default)
make db-admin ADMIN_EMAIL=you@example.com      # promote your own account (password untouched)
```

`db-seed` depends on `db-admin`, so the demo admin exists before seeding runs (the
seed needs an admin to upgrade the instructor to premium, which in turn unlocks the
agent-creation quota used by `make db-bulk`). After being promoted, log out and back
in — token scopes are fixed at login.

Copy `.env.example` to `.env` if you need to override defaults.

## Common tasks

```bash
make check          # fmt-check + lint + unit tests (pre-commit gate)
make ci             # check + DB-backed tests + db-reset + e2e (full gate)
make test-db        # DB-backed integration tests (requires `make db-up`)
make db-reset       # wipe DB data, recreate, migrate (fixes migration checksum conflicts)
make openapi        # regenerate api/openapi.yaml + web TypeScript schema
```

Install the pre-commit hook with `make hooks-install`.

## E2E tests

```bash
make e2e            # auto-starts API + seeds + runs Playwright suite
```

For interactive visual audits use `bunx @playwright/cli` — see `AGENTS.md` for the auth cookie pattern.

## Deploy

`docker-compose.prod.yml` is a production-shaped stack (Postgres + API + web) for
smoke deploys, demos, and CI integration testing — not a substitute for the k8s
manifests. The API runs its migrations on boot, so no separate migration step is
needed.

```bash
cp .env.example .env   # set POSTGRES_PASSWORD (and NEXT_PUBLIC_API_URL for the web bundle)
make docker-build      # build api + web images
make docker-up         # start the stack (-d)
make docker-logs       # tail logs
make docker-down       # stop the stack
```

## Local database

Postgres runs in Docker or Podman (auto-detected by the Makefile). On macOS, [Colima](https://github.com/abiosoft/colima) or Docker Desktop works.

```bash
make db-up
# ... work ...
make db-down
```

## Layout

- `api/` — Rust backend (Axum, sqlx).
- `web/` — Next.js frontend (App Router, MUI).
- `db/` — `docker-compose.yml` + sqlx migrations.
- `design/source/src/*.jsx` — pixel-faithful UI design source of truth.
- `docs/specs/` — design specs (`2026-05-20-harus-platform-design.md` is canonical).
- `docs/plans/` — implementation plans.
- `.tmp/` — gitignored scratch space for screenshots and Playwright artifacts.

## Contributing

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for setup, conventions, and the test workflow. Security issues: see [`SECURITY.md`](./SECURITY.md).

`AGENTS.md` is the full working agreement (originally written for AI assistants but useful for humans too).
