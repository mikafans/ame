# Contributing to ame

Thanks for considering a contribution. ame is a small project; the bar is "works, has tests, follows the conventions below."

## Setup

The toolchain (rust, bun, uv, python, sqlx-cli) comes from mise.
Run `mise install` once; tools then activate automatically on `cd` into the
repo, or explicitly via `mise x -- <command>` in non-interactive shells. Then:

```bash
make init-env      # web deps + playwright browsers (toolchain is from mise)
make dev           # Postgres, Valkey, API, and web, all behind Caddy at :28800
```

`make dev` is the one local stack and auto-seeds demo accounts on first boot.
Demo credentials: `haru@example.com / password123` (learner), `admin@example.com / password123` (admin).

Copy `.env.example` to `.env` if you need to override defaults.

## Quality gates

| Command | When |
|---|---|
| `make check` | Before every commit (fmt-check + lint + unit tests). Pre-commit hook enforces this — install with `make hooks-install`. |
| `make test-db` | When touching DB-backed code. Requires `make db-up` (a separate bare Postgres, unrelated to `make dev`'s own database). |
| `make ci` | Before opening a PR (check + DB-backed tests + e2e, resetting the DB between them). |
| `make openapi` | After changing Rust route handlers or DTOs. Regenerates `api/openapi.yaml` and the web TypeScript schema. |

CI runs `make check` plus an OpenAPI/schema drift check (`bun run api:check`) on
every PR (see `.github/workflows/ci.yml`). The DB-backed and e2e suites need a
live stack and are not run in CI — run `make ci` locally before opening a PR.

## Conventions

- **Conventional commits**: `feat:`, `fix:`, `chore:`, `docs:`, `deploy:`. No emojis.
- **One logical change per PR**. Bundle refactors only when they're load-bearing for the feature.
- **Stage specific files** — never `git add -A`.
- **2-space indent** for config (YAML, TOML, JSON, MD); 4-space for Rust/Python; tabs for Makefile (`.editorconfig` enforces).
- **Ask before pushing** if you're working with an AI assistant; we don't auto-push.

## Code organization

- Backend: `api/src/` — module boundaries enforced. `engine/` and `assess/` may depend on `bank/` and `domain/`. Reverse is forbidden. `bank/` does not know that attempts exist.
- Frontend: `web/app/` (App Router pages) + `web/src/` (components, hooks, generated API client).
- Schema: `db/migrations/` (sqlx). Additive after the squash baseline; never edit a migration that has been applied. Use `make db-reset` to wipe and re-apply locally.
- Specs: `docs/plans/2026-07-19-agent-first-learning-rework.md` is canonical for the current 0.3 rework (see `docs/ROADMAP.md`).

See `CLAUDE.md` for the full working agreement, shell discipline notes, and tool-by-tool guidance (useful for humans too).

## Reporting issues

- Bugs: please include reproduction steps, expected vs actual, and the relevant log slice from `.tmp/ame-api.log` or browser console.
- Security issues: see `SECURITY.md` — do not file public issues.

## Licensing

ame is GPL-3.0. By submitting a PR you agree to license your contribution under the same terms.
