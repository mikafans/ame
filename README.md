# ame

Quiz and exam platform. Rust (Axum) backend, Next.js (App Router) frontend, Postgres.

**Stack**: Rust 2024 · Axum 0.8 · sqlx · Next.js 16 · React 19 · Bun · TypeScript · MUI 7 · Postgres 18

**License**: [GPL-3.0](./LICENSE)

## Quick start

```bash
make init-env       # mise install + sqlx-cli + web deps + Playwright
make db-up          # start Postgres in Docker or Podman
make db-migrate     # apply pending migrations
make dev            # API on :8080, frontend on :3000
make db-seed        # seed demo users, quizzes, questions (requires API running)
```

Demo credentials after seeding: `learner@example.com / password123`, `instructor@example.com / password123`.

Copy `.env.example` to `.env` if you need to override defaults.

## Common tasks

```bash
make check          # fmt-check + lint + unit tests (pre-commit gate)
make validate       # check + e2e (pre-PR gate)
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
