# AGENTS.md — Briefing for AI Assistants

This file is the canonical, machine-readable rundown of how to work in this repo.

## Stack
- Backend: Rust (edition 2021), Axum 0.8, Tokio, sqlx (added in Plan 2), Postgres 18.
- Frontend: Next.js 15 App Router, React 19, TypeScript, Tailwind 3. Runtime + package manager is **bun** (no node, no npm). One deployment; `/admin/*` is role-gated.
- Infra: Postgres in Docker (`db/docker-compose.yml`). Local dev only — production runs in Kubernetes.
- Toolchain: `mise` for language runtimes (rust, bun); `nix develop` for everything else (psql, gh, sqlfluff, …).

## Commands
- `make check` — fmt-check + lint + test. Required before every commit (pre-commit hook enforces).
- `make validate` — `make check` + e2e. Required before opening a PR.
- `make test-db` — DB-backed API integration tests. Requires `make db-up`; `make check` skips these unless `AME_RUN_DB_TESTS=1`.
- `make db-up` / `make db-down` — start / stop Postgres.
- `cd api && mise exec -- cargo run` — boot the API on `:8080`.
- `cd web && mise exec -- bun run dev` — boot the frontend on `:3000`.

## Conventions
- Conventional Commits: `feat:`, `fix:`, `chore:`, `docs:`, `deploy:`. No emojis.
- 2-space indent for config (YAML, TOML, JSON, MD); 4-space for Rust/Python; tabs for Makefile (enforced by .editorconfig).
- Stage specific files: `git add path/to/file`, never `git add -A` or `git add .`.
- Always ask before committing or pushing.

## Layout
See `docs/specs/2026-05-19-question-exam-platform-design.md` for the full architecture spec.
See `docs/plans/` for implementation plans.

## Module boundaries (api/src/)
`engine/` and `assess/` may depend on `bank/` and `domain/`. Reverse is forbidden. `bank/` does not know that attempts exist.
