# ame

Quiz and exam platform. Rust (Axum) backend, Next.js 16 / React 19 frontend, Postgres.

**Stack**: Rust · Axum 0.8 · sqlx · Next.js 16 · Bun · TypeScript · Tailwind 3 · Postgres 18

## Quick start

```bash
mise install                            # rust + bun + uv runtimes
make db-up                              # start Postgres in Docker (requires Colima)
make db-migrate                         # run pending migrations
make dev-env                            # API on :8080, frontend on :3000
uv run --script scripts/seed.py         # seed demo users, quizzes, questions
```

Credentials after seeding: `learner@example.com / password123`, `instructor@example.com / password123`

## Common tasks

```bash
make check          # fmt-check + lint + tests (pre-commit gate)
make validate       # check + e2e (pre-PR gate)
make db-reset       # wipe DB data, recreate, migrate (use when migration checksums conflict)
make openapi        # regenerate api/openapi.yaml + web/src/api/generated/schema.d.ts
```

## E2E tests

```bash
TOKEN=$(curl -s -X POST http://localhost:8080/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"learner@example.com","password":"password123"}' | jq -r .token)

E2E_API_TOKEN=$TOKEN E2E_BASE_URL=http://localhost:3000 \
  bunx @playwright/test test --project=chromium
```

For interactive visual audits use `bunx @playwright/cli` — see `AGENTS.md` for the auth cookie pattern.

## Local database

Postgres runs in Docker. On macOS, [Colima](https://github.com/abiosoft/colima) is recommended.

```bash
colima start
make db-up
# ... work ...
make db-down
colima stop
```

## Layout

- `api/` — Rust backend
- `web/` — Next.js frontend
- `db/` — docker-compose + sqlx migrations
- `design/source/src/*.jsx` — pixel-faithful UI design source of truth
- `docs/specs/` — design specs (`2026-05-20-harus-platform-design.md` is canonical)
- `docs/plans/` — implementation plans
- `.tmp/` — gitignored scratch space for screenshots and playwright artifacts

See `AGENTS.md` for full working conventions and `AGENTS.md` → toolchain details.
