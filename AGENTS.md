# AGENTS.md — Briefing for AI Assistants

This file is the canonical, machine-readable rundown of how to work in this repo.

## Stack
- Backend: Rust (edition 2021), Axum 0.8, Tokio, sqlx (added in Plan 2), Postgres 18.
- Frontend: Next.js 16 App Router, React 19, TypeScript, MUI 7 (Emotion). Runtime + package manager is **bun** (no node, no npm). One deployment; role gating is enforced per-route in the API.
- Infra: Postgres in Docker (`db/docker-compose.yml`). Local dev only — production runs in Kubernetes.
- Toolchain: `mise` for language runtimes (rust, bun, uv). SQL client: `uvx pgcli postgres://postgres:postgres@localhost:5432/ame`; migrations: `sqlx migrate run` (sqlx-cli via cargo); sqlfluff via `uvx sqlfluff`.

## Commands
- `make init-env` — one-time setup on a fresh checkout: `mise install` (rust, bun, uv runtimes) + `sqlx-cli` via cargo + `bun install` for web deps + Playwright browsers. Run before `make dev-env`.
- `make dev-env` — start full local stack: DB up → migrate → API on `:8080` → frontend on `:3000`.
- `make db-seed` — seed demo data (requires API running).
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
See `docs/specs/2026-05-20-harus-platform-design.md` for the current architecture spec. It replaces `2026-05-19-question-exam-platform-design.md` (kept on disk as historical reference only).
See `docs/plans/` for implementation plans, including `docs/plans/2026-05-20-design-gap-analysis.md` for the latest design ↔ spec gap audit.

## Data model constraints

- **MC options**: bare strings (`Vec<String>` in `McPayload`) — never `{text:"..."}` objects. Both the grading engine and frontend `McOptionsEditor` expect this format.
- **Scheduled exams**: not supported. No `opens_at`/`closes_at` in `tb_exams` or the API.
- **Auth**: email + password only (`POST /v1/auth/register`, `/v1/auth/login`). SSO/SAML not implemented.
- **Quiz publish**: `PATCH /v1/quizzes/{id}` with `status: active` auto-promotes any draft questions to live.
- **Code questions**: fall back to `pending_manual` grading unless `payload.exemplar` is set; exemplar match is exact (whitespace-trimmed).

## Module boundaries (api/src/)
`engine/` and `assess/` may depend on `bank/` and `domain/`. Reverse is forbidden. `bank/` does not know that attempts exist.

## E2E / Visual audit tooling

### Running the test suite
```bash
# Full Playwright suite (non-interactive)
E2E_API_TOKEN=<learner-token> E2E_BASE_URL=http://localhost:3000 \
  bunx @playwright/test test --project=chromium
```

### Interactive browser audit (@playwright/cli)
```bash
# Package is @playwright/cli — NOT playwright-cli (that 404s on npm)
# Run from .tmp/ so snapshots land there, not in web/
cd .tmp
bunx @playwright/cli open http://localhost:3000

# Auth pattern — cookies don't survive goto; must re-set on each new page:
bunx @playwright/cli cookie-set ame_token "<token>" --domain=localhost
bunx @playwright/cli reload
sleep 3   # useAuth is async; screenshot too early catches Loading state
bunx @playwright/cli screenshot --filename=page.png

# Common commands
bunx @playwright/cli goto http://localhost:3000/exams
bunx @playwright/cli snapshot          # accessibility tree with refs
bunx @playwright/cli click e42
bunx @playwright/cli eval "document.cookie"
bunx @playwright/cli console
bunx @playwright/cli close
```

### Getting a learner token
```bash
curl -s -X POST http://localhost:8080/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"learner@example.com","password":"password123"}' | jq -r .token
```

### jq tip (fish shell)
Multi-line variable + pipe breaks jq. Use a temp file:
```bash
curl -s <url> -H "..." -o /tmp/out.json && jq '.field' /tmp/out.json
```

### Temp files
Use `.tmp/` at repo root (gitignored). Clean with `rm -rf .tmp/*.png .tmp/.playwright-cli` after a session.

## Shell discipline

- **cwd is always the repo root** — never prepend `cd api &&` or `cd web &&`. Use `make <target>` for everything; it sets the right cwd internally.
- **Search**: `rg` not `grep -r`. **Find**: `fd` not `find`. If `rg` returns a path, `Read` the file — never pipe into `xargs rg`.
- **Long output**: redirect to a file (`make check > /tmp/check.log 2>&1`) and `Read /tmp/check.log`. rtk truncates tool output; grepping truncated output silently misses lines.
- **Migration checksum mismatch** (sqlfluff reformats a migration after sqlx recorded its checksum): fix with `make db-reset && make dev`, not manual `_sqlx_migrations` surgery.
- **JS runtime**: `bun` not `node`. Playwright scripts: `bun .claude/scripts/<name>.js`.
- **Verifier scripts**: write to `.claude/scripts/` so they can be reused across sessions.

## Agent operations
`agents/` at the project root contains skill markdowns for agent-driven workflows:
- `agents/generate-questions/SKILL.md` — fetch weakest tags, generate targeted questions.
- `agents/analyze-performance/SKILL.md` — read tag stats and recent attempts.
- `agents/adaptive-generation/SKILL.md` — full adaptive loop with pool-insufficient retry.

Agent discovery starts at `GET /v1/agents/mcp.json` (public). Add `?strict=1` for strict MCP consumers.
Register a new agent key: `POST /v1/agents/register` (unauthenticated; returns `apiKey` + discovery URLs).
