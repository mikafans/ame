# Contributing to ame

Thanks for considering a contribution. ame is a small project; the bar is "works, has tests, follows the conventions below."

## Setup

The toolchain (rust, bun, uv, python, sqlx-cli) comes from the Nix devShell.
Enter it with `direnv allow` (auto-loads via `.envrc`) or `nix develop`; then:

```bash
make init-env      # web deps + playwright browsers (toolchain is from nix)
make db-up         # start Postgres in Docker/Podman
make dev           # API on :28080, frontend on :23000
make db-seed       # seed demo users, quizzes, questions
```

Demo credentials after seeding: `ada@example.com / password123` (primary user), `mira@example.com / password123` (second user).

Copy `.env.example` to `.env` if you need to override defaults.

## Quality gates

| Command | When |
|---|---|
| `make check` | Before every commit (fmt-check + lint + unit tests). Pre-commit hook enforces this — install with `make hooks-install`. |
| `make test-db` | When touching DB-backed code. Requires `make db-up`. |
| `make validate` | Before opening a PR (check + e2e). |
| `make openapi` | After changing Rust route handlers or DTOs. Regenerates `api/openapi.yaml` and the web TypeScript schema. |

CI runs `make check` + `cargo audit` on every PR (see `.github/workflows/ci.yml`). The DB-backed and e2e suites need a Postgres service and are not run in CI — run `make validate` locally before opening a PR.

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
